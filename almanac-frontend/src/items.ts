import type { ActivityData, ActivityItem, FilmItem, Item, NoteItem } from './api'
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
export const isActivity = (item: Item): item is ActivityItem => item.kind === 'activity'

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

/** 10234 -> "10.2 km", 850 -> "850 m". */
export function formatDistance(metres: number): string {
	if (metres < 1000) {
		return `${Math.round(metres)} m`
	}
	const km = metres / 1000
	return `${km >= 100 ? Math.round(km) : km.toFixed(1)} km`
}

/** 3125 -> "52:05", 4500 -> "1:15:00". */
export function formatDuration(seconds: number): string {
	const total = Math.round(seconds)
	const hours = Math.floor(total / 3600)
	const minutes = Math.floor((total % 3600) / 60)
	const pad = (n: number) => String(n).padStart(2, '0')
	return hours
		? `${hours}:${pad(minutes)}:${pad(total % 60)}`
		: `${minutes}:${pad(total % 60)}`
}

/** "1:02:03" or "52:05" -> seconds; a lone number is minutes. Undefined if it doesn't parse. */
export function parseDuration(text: string): number | undefined {
	const parts = text.trim().split(':').map(Number)
	if (!text.trim() || parts.some((part) => !Number.isFinite(part) || part < 0) || parts.length > 3) {
		return undefined
	}
	if (parts.length === 1) {
		return Math.round(parts[0] * 60)
	}
	return parts.reduce((total, part) => total * 60 + part, 0)
}

const FOOT_SPORTS = new Set(['Run', 'TrailRun', 'VirtualRun', 'Walk', 'Hike'])

/** Pace for running and walking ("5:05 /km"), per 100 m for swims, speed for everything else. */
export function paceOrSpeed(activity: ActivityData): string | null {
	const { distance_m: distance, moving_time_s: time, sport } = activity
	if (!distance || !time) {
		return null
	}
	if (FOOT_SPORTS.has(sport)) {
		return `${formatDuration(time / (distance / 1000))} /km`
	}
	if (sport === 'Swim') {
		return `${formatDuration(time / (distance / 100))} /100m`
	}
	return `${((distance / 1000) / (time / 3600)).toFixed(1)} km/h`
}

/** "TrailRun" -> "Trail run", "WeightTraining" -> "Weight training". */
export function sportLabel(sport: string): string {
	const words = sport.replace(/([a-z])([A-Z])/g, '$1 $2').split(' ')
	return words.map((word, index) => (index === 0 ? word : word.toLowerCase())).join(' ')
}

const dayTimeFormat = new Intl.DateTimeFormat('en-AU', {
	weekday: 'short',
	day: 'numeric',
	month: 'short',
	year: 'numeric',
	hour: 'numeric',
	minute: '2-digit',
})

/** "Tue 13 Oct 2026, 6:30 am": when an activity started, in the viewer's time zone. */
export function formatDayTime(iso: string): string {
	return dayTimeFormat.format(new Date(iso))
}

/** The headline numbers of an activity, skipping any it doesn't have. */
export function activityStats(activity: ActivityData): [label: string, value: string][] {
	const stats: [string, string | null][] = [
		['Distance', activity.distance_m ? formatDistance(activity.distance_m) : null],
		['Moving time', activity.moving_time_s ? formatDuration(activity.moving_time_s) : null],
		[activity.sport.match(/Run|Walk|Hike|Swim/) ? 'Pace' : 'Speed', paceOrSpeed(activity)],
		['Elevation', activity.elevation_gain_m ? `${Math.round(activity.elevation_gain_m)} m` : null],
		['Avg heart rate', activity.average_heartrate ? `${Math.round(activity.average_heartrate)} bpm` : null],
	]
	return stats.filter((stat): stat is [string, string] => stat[1] !== null)
}
