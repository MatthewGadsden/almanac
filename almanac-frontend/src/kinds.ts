import type { Item, ItemKind } from './api'

/** Per-kind wording and URL segment. Add a kind here, in api.ts, and in the backend's ItemKind. */
export const KINDS: Record<ItemKind, { label: string; plural: string; slug: string; create: string }> = {
	note: { label: 'Note', plural: 'Notes', slug: 'notes', create: 'Plant a note' },
	film: { label: 'Film', plural: 'Films', slug: 'films', create: 'Log a film' },
	activity: { label: 'Activity', plural: 'Activities', slug: 'activities', create: 'Log an activity' },
}

export const itemPath = (item: Pick<Item, 'id' | 'kind'>) => `/${KINDS[item.kind].slug}/${item.id}`
export const editItemPath = (item: Pick<Item, 'id' | 'kind'>) => `${itemPath(item)}/edit`
export const newItemPath = (kind: ItemKind) => `/${KINDS[kind].slug}/new`
export const topicPath = (topic: string) => `/topics/${encodeURIComponent(topic)}`
