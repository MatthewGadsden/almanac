// Client for almanac-backend. In dev, Vite proxies /api to the backend (see vite.config.ts).
const API_URL: string = import.meta.env.VITE_API_URL ?? '/api'

export type ItemKind = 'note' | 'film' | 'activity'
export type Visibility = 'public' | 'private'

export interface FilmData {
	year?: number
	director?: string
	/** Out of 5 in half stars. */
	rating?: number
	rewatch: boolean
	liked: boolean
	poster_url?: string
	/** The entry on Letterboxd, for imported films. */
	letterboxd_url?: string
	tmdb_id?: number
}

export interface ActivityData {
	/** Strava's sport type, e.g. Run, TrailRun, Ride, Swim, WeightTraining. */
	sport: string
	distance_m?: number
	moving_time_s?: number
	elapsed_time_s?: number
	elevation_gain_m?: number
	average_heartrate?: number
	max_heartrate?: number
	/** The activity on Strava, for imported activities. */
	strava_url?: string
	/** The route's shape as [x, y] points in a 100×100 box, y downwards. No coordinates. */
	route_outline?: [number, number][]
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
export type ActivityItem = ItemBase & { kind: 'activity'; data: ActivityData }
export type Item = NoteItem | FilmItem | ActivityItem

export interface ItemPayload {
	kind: ItemKind
	title: string
	topic: string | null
	body: string
	data?: Partial<FilmData> | Partial<ActivityData>
	occurred_at?: string
	visibility: Visibility
}

export type Provider = 'letterboxd' | 'strava'

export interface SyncSummary {
	fetched: number
	created: number
	updated: number
	unchanged: number
	/** Entries that aren't items, e.g. Letterboxd lists. */
	skipped: number
}

export interface Connection {
	provider: Provider
	/** Letterboxd: `username`. Strava: the athlete who authorised (`athlete_id`, `name`). */
	settings: { username?: string; athlete_id?: number; name?: string }
	import_visibility: Visibility
	last_synced_at: string | null
	last_sync_summary: SyncSummary | null
	last_error: string | null
	created_at: string
	updated_at: string
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

export function listConnections(): Promise<Connection[]> {
	return request<Connection[]>('/connections')
}

export function saveConnection(
	provider: Provider,
	settings: Connection['settings'],
	importVisibility: Visibility,
): Promise<Connection> {
	return request<Connection>(`/connections/${provider}`, {
		method: 'PUT',
		body: JSON.stringify({ settings, import_visibility: importVisibility }),
	})
}

export function deleteConnection(provider: Provider): Promise<void> {
	return request<void>(`/connections/${provider}`, { method: 'DELETE' })
}

export function syncConnection(provider: Provider): Promise<SyncSummary> {
	return request<SyncSummary>(`/connections/${provider}/sync`, { method: 'POST' })
}

/** Where to send the admin to authorise Almanac on Strava. */
export function stravaAuthorize(): Promise<{ url: string }> {
	return request<{ url: string }>('/connections/strava/authorize')
}

/** Finishes connecting Strava with what Strava put on the redirect. */
export function stravaCallback(code: string, state: string, scope: string): Promise<Connection> {
	return request<Connection>('/connections/strava/callback', {
		method: 'POST',
		body: JSON.stringify({ code, state, scope }),
	})
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
