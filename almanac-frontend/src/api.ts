// Client for almanac-backend. In dev, Vite proxies /api to the backend (see vite.config.ts).
const API_URL: string = import.meta.env.VITE_API_URL ?? '/api'

export type ItemKind = 'note' | 'film'
export type Visibility = 'public' | 'private'

export interface FilmData {
	year?: number
	director?: string
	/** Out of 5 in half stars. */
	rating?: number
	rewatch: boolean
	liked: boolean
	poster_url?: string
}

interface ItemBase {
	id: string
	title: string
	topic: string | null
	/** The admin's own writing, as HTML. */
	body: string
	/** When it happened (watched, run, ...), as opposed to when it was logged. */
	occurred_at: string
	visibility: Visibility
	/** 'manual', or the connector an item was imported from. */
	source: string
	external_id: string | null
	created_at: string
	updated_at: string
}

export type NoteItem = ItemBase & { kind: 'note'; data: Record<string, never> }
export type FilmItem = ItemBase & { kind: 'film'; data: FilmData }
export type Item = NoteItem | FilmItem

export interface ItemPayload {
	kind: ItemKind
	title: string
	topic: string | null
	body: string
	data?: Partial<FilmData>
	occurred_at?: string
	visibility: Visibility
}

export interface AdminUser {
	user_id: string
	username: string
}

export class ApiError extends Error {
	status: number

	constructor(status: number, message: string) {
		super(message)
		this.name = 'ApiError'
		this.status = status
	}
}

export function errorMessageOf(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}

async function errorMessage(response: Response): Promise<string> {
	const text = await response.text()
	try {
		const body = JSON.parse(text) as { error?: string }
		if (body.error) {
			return body.error
		}
	} catch {
		// Not JSON, e.g. actix's plain-text errors for a malformed body
	}

	if (response.status >= 500) {
		return `almanac-backend responded with ${response.status}. Is it running?`
	}

	return text || `Request failed with status ${response.status}`
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	let response: Response
	try {
		response = await fetch(`${API_URL}${path}`, {
			...init,
			headers: init?.body ? { 'Content-Type': 'application/json' } : undefined,
		})
	} catch {
		throw new ApiError(0, 'Could not reach almanac-backend. Is it running?')
	}

	if (!response.ok) {
		throw new ApiError(response.status, await errorMessage(response))
	}

	if (response.status === 204) {
		return undefined as T
	}

	return (await response.json()) as T
}

export function listItems(): Promise<Item[]> {
	return request<Item[]>('/items')
}

export function createItem(payload: ItemPayload): Promise<Item> {
	return request<Item>('/items', { method: 'POST', body: JSON.stringify(payload) })
}

export function updateItem(id: string, payload: ItemPayload): Promise<Item> {
	return request<Item>(`/items/${id}`, { method: 'PUT', body: JSON.stringify(payload) })
}

export function deleteItem(id: string): Promise<void> {
	return request<void>(`/items/${id}`, { method: 'DELETE' })
}

/** The logged-in admin, or null when there is no session. */
export async function currentAdmin(): Promise<AdminUser | null> {
	try {
		return await request<AdminUser>('/admin/me')
	} catch (error) {
		if (error instanceof ApiError && error.status === 401) {
			return null
		}
		throw error
	}
}

export function login(username: string, password: string): Promise<AdminUser> {
	return request<AdminUser>('/admin/login', { method: 'POST', body: JSON.stringify({ username, password }) })
}

export function logout(): Promise<void> {
	return request<void>('/admin/logout', { method: 'POST' })
}
