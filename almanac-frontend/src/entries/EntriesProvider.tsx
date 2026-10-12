import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import * as api from '../api'
import type { Entry, EntryPayload } from '../api'
import { EntriesContext, type EntriesContextValue } from './EntriesContext'

/**
 * Loads every entry once and keeps them in memory. The sidebar index, listings and
 * note pages all read from this list; mutations update it in place after the API call
 * succeeds, so nothing is refetched.
 */
export function EntriesProvider({ children }: { children: ReactNode }) {
	const [entries, setEntries] = useState<Entry[]>([])
	const [status, setStatus] = useState<EntriesContextValue['status']>('loading')
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		let cancelled = false

		api.listEntries().then(
			(loaded) => {
				if (!cancelled) {
					setEntries(loaded)
					setStatus('ready')
				}
			},
			(e: unknown) => {
				if (!cancelled) {
					setError(api.errorMessageOf(e))
					setStatus('error')
				}
			},
		)

		return () => {
			cancelled = true
		}
	}, [])

	const createEntry = useCallback(async (payload: EntryPayload) => {
		const entry = await api.createEntry(payload)
		setEntries((current) => [entry, ...current])
		return entry
	}, [])

	const updateEntry = useCallback(async (id: string, payload: EntryPayload) => {
		const entry = await api.updateEntry(id, payload)
		setEntries((current) => current.map((existing) => (existing.id === id ? entry : existing)))
		return entry
	}, [])

	const deleteEntry = useCallback(async (id: string) => {
		await api.deleteEntry(id)
		setEntries((current) => current.filter((existing) => existing.id !== id))
	}, [])

	const value = useMemo(
		() => ({ entries, status, error, createEntry, updateEntry, deleteEntry }),
		[entries, status, error, createEntry, updateEntry, deleteEntry],
	)

	return <EntriesContext value={value}>{children}</EntriesContext>
}
