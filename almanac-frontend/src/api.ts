// Client for almanac-backend. In dev, Vite proxies /api to the backend (see vite.config.ts).
const API_URL: string = import.meta.env.VITE_API_URL ?? '/api'

export interface Entry {
	id: string
	title: string
	topic: string
	body: string
	created_at: string
	updated_at: string
}

export interface EntryPayload {
	title: string
	topic: string
	body: string
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

export function listEntries(): Promise<Entry[]> {
	return request<Entry[]>('/entries')
}

export function createEntry(payload: EntryPayload): Promise<Entry> {
	return request<Entry>('/entries', { method: 'POST', body: JSON.stringify(payload) })
}

export function updateEntry(id: string, payload: EntryPayload): Promise<Entry> {
	return request<Entry>(`/entries/${id}`, { method: 'PUT', body: JSON.stringify(payload) })
}

export function deleteEntry(id: string): Promise<void> {
	return request<void>(`/entries/${id}`, { method: 'DELETE' })
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
