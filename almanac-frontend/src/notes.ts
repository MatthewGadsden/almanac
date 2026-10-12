import type { Entry } from './api'
import { plainText } from './noteHtml'

export const notePath = (id: string) => `/notes/${id}`
export const editNotePath = (id: string) => `/notes/${id}/edit`
export const topicPath = (topic: string) => `/topics/${encodeURIComponent(topic)}`

const dateFormat = new Intl.DateTimeFormat('en-AU', { day: 'numeric', month: 'short', year: 'numeric' })

export function formatDate(iso: string): string {
	return dateFormat.format(new Date(iso))
}

/** The backend sets both timestamps to the same instant on create, so any difference means an edit. */
export function isTended(entry: Entry): boolean {
	return entry.created_at !== entry.updated_at
}

export function byTitle(a: Entry, b: Entry): number {
	return a.title.localeCompare(b.title)
}

export function byRecentlyUpdated(a: Entry, b: Entry): number {
	return Date.parse(b.updated_at) - Date.parse(a.updated_at)
}

/** Topics in alphabetical order, each with its notes sorted by title. */
export function groupByTopic(entries: Entry[]): [string, Entry[]][] {
	const topics = new Map<string, Entry[]>()
	for (const entry of entries) {
		topics.set(entry.topic, [...(topics.get(entry.topic) ?? []), entry])
	}

	return [...topics.entries()]
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([topic, notes]) => [topic, notes.sort(byTitle)])
}

export function excerpt(body: string, length = 180): string {
	const flat = plainText(body).replace(/\s+/g, ' ').trim()
	return flat.length > length ? `${flat.slice(0, length).trimEnd()}…` : flat
}
