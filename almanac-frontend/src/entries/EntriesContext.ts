import { createContext, useContext } from 'react'
import type { Entry, EntryPayload } from '../api'

export interface EntriesContextValue {
	entries: Entry[]
	status: 'loading' | 'ready' | 'error'
	error: string | null
	createEntry: (payload: EntryPayload) => Promise<Entry>
	updateEntry: (id: string, payload: EntryPayload) => Promise<Entry>
	deleteEntry: (id: string) => Promise<void>
}

export const EntriesContext = createContext<EntriesContextValue | null>(null)

export function useEntries(): EntriesContextValue {
	const value = useContext(EntriesContext)
	if (!value) {
		throw new Error('useEntries must be used inside <EntriesProvider>')
	}

	return value
}
