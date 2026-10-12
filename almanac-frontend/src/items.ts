import type { FilmItem, Item, NoteItem } from './api'
import { plainText } from './noteHtml'

const dateFormat = new Intl.DateTimeFormat('en-AU', { day: 'numeric', month: 'short', year: 'numeric' })
const monthFormat = new Intl.DateTimeFormat('en-AU', { month: 'long', year: 'numeric' })

export function formatDate(iso: string): string {
	return dateFormat.format(new Date(iso))
}

const dateTimeFormat = new Intl.DateTimeFormat('en-AU', {
	day: 'numeric',
	month: 'short',
	year: 'numeric',
	hour: 'numeric',
	minute: '2-digit',
})

export function formatDateTime(iso: string): string {
	return dateTimeFormat.format(new Date(iso))
}

export function formatMonth(iso: string): string {
	return monthFormat.format(new Date(iso))
}

/** `YYYY-MM-DD` in local time, for <input type="date">. */
export function toDateInput(iso: string): string {
	const date = new Date(iso)
	const pad = (n: number) => String(n).padStart(2, '0')
	return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

/** A date picked in a form, stored at midday UTC so it shows as the same day in any timezone. */
export function fromDateInput(date: string): string {
	return `${date}T12:00:00Z`
}

/** The backend sets both timestamps to the same instant on create, so any difference means an edit. */
export function isTended(item: Item): boolean {
	return item.created_at !== item.updated_at
}

export const isNote = (item: Item): item is NoteItem => item.kind === 'note'
export const isFilm = (item: Item): item is FilmItem => item.kind === 'film'

export function byTitle(a: Item, b: Item): number {
	return a.title.localeCompare(b.title)
}

export function byRecentlyUpdated(a: Item, b: Item): number {
	return Date.parse(b.updated_at) - Date.parse(a.updated_at)
}

export function byRecentlyOccurred(a: Item, b: Item): number {
	return Date.parse(b.occurred_at) - Date.parse(a.occurred_at)
}

/** Topics in alphabetical order, each with its items sorted by title. Items without a topic are left out. */
export function groupByTopic<T extends Item>(items: T[]): [string, T[]][] {
	const topics = new Map<string, T[]>()
	for (const item of items) {
		if (item.topic) {
			topics.set(item.topic, [...(topics.get(item.topic) ?? []), item])
		}
	}

	return [...topics.entries()]
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([topic, grouped]) => [topic, grouped.sort(byTitle)])
}

/** Consecutive runs of items that fall in the same month, e.g. for a film diary. Expects sorted input. */
export function groupByMonth<T extends Item>(items: T[]): [string, T[]][] {
	const months: [string, T[]][] = []
	for (const item of items) {
		const month = formatMonth(item.occurred_at)
		const last = months.at(-1)
		if (last && last[0] === month) {
			last[1].push(item)
		} else {
			months.push([month, [item]])
		}
	}

	return months
}

export function excerpt(body: string, length = 180): string {
	const flat = plainText(body).replace(/\s+/g, ' ').trim()
	return flat.length > length ? `${flat.slice(0, length).trimEnd()}…` : flat
}

/** Letterboxd-style stars: 3.5 -> "★★★½". */
export function stars(rating: number): string {
	return '★'.repeat(Math.floor(rating)) + (rating % 1 ? '½' : '')
}
