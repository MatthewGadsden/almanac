import type { Item } from '../api'

/** Marks items only the admin can see. */
export function PrivateBadge({ item }: { item: Item }) {
	return item.visibility === 'private' ? <span className="status-badge status-badge-private">Private</span> : null
}
