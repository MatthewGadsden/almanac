import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import * as api from '../api'
import type { Item, ItemPayload } from '../api'
import { useAuth } from '../auth/AuthContext'
import { ItemsContext, type ItemsContextValue } from './ItemsContext'

interface Loaded {
	/** Who the list was loaded for: the admin's id, or 'visitor'. */
	viewer: string
	items: Item[]
	error: string | null
}

const NO_ITEMS: Item[] = []

/**
 * Loads every item the current viewer may see, once, and keeps them in memory. The admin also
 * gets private items, so the list is reloaded whenever someone logs in or out. Mutations patch
 * the list after the API call succeeds, so nothing else is refetched.
 */
export function ItemsProvider({ children }: { children: ReactNode }) {
	const { admin, status: authStatus } = useAuth()
	const viewer = authStatus === 'checking' ? null : (admin?.user_id ?? 'visitor')
	const [loaded, setLoaded] = useState<Loaded | null>(null)

	useEffect(() => {
		if (viewer === null) {
			return
		}

		let cancelled = false
		api.listItems().then(
			(items) => {
				if (!cancelled) {
					setLoaded({ viewer, items, error: null })
				}
			},
			(e: unknown) => {
				if (!cancelled) {
					setLoaded({ viewer, items: [], error: api.errorMessageOf(e) })
				}
			},
		)

		return () => {
			cancelled = true
		}
	}, [viewer])

	// A list loaded for a different viewer is stale: treat it as still loading
	const current = loaded && loaded.viewer === viewer ? loaded : null
	const status: ItemsContextValue['status'] = !current ? 'loading' : current.error ? 'error' : 'ready'
	const items = current?.items ?? NO_ITEMS
	const error = current?.error ?? null

	const patchItems = useCallback((patch: (items: Item[]) => Item[]) => {
		setLoaded((previous) => previous && { ...previous, items: patch(previous.items) })
	}, [])

	const createItem = useCallback(
		async (payload: ItemPayload) => {
			const item = await api.createItem(payload)
			patchItems((existing) => [item, ...existing])
			return item
		},
		[patchItems],
	)

	const updateItem = useCallback(
		async (id: string, payload: ItemPayload) => {
			const item = await api.updateItem(id, payload)
			patchItems((existing) => existing.map((candidate) => (candidate.id === id ? item : candidate)))
			return item
		},
		[patchItems],
	)

	const deleteItem = useCallback(
		async (id: string) => {
			await api.deleteItem(id)
			patchItems((existing) => existing.filter((candidate) => candidate.id !== id))
		},
		[patchItems],
	)

	const reload = useCallback(async () => {
		if (viewer === null) {
			return
		}
		const fresh = await api.listItems()
		setLoaded({ viewer, items: fresh, error: null })
	}, [viewer])

	const value = useMemo(
		() => ({ items, status, error, createItem, updateItem, deleteItem, reload }),
		[items, status, error, createItem, updateItem, deleteItem, reload],
	)

	return <ItemsContext value={value}>{children}</ItemsContext>
}
