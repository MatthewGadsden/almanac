import { createContext, useContext } from 'react'
import type { Item, ItemPayload } from '../api'

export interface ItemsContextValue {
	items: Item[]
	status: 'loading' | 'ready' | 'error'
	error: string | null
	createItem: (payload: ItemPayload) => Promise<Item>
	updateItem: (id: string, payload: ItemPayload) => Promise<Item>
	deleteItem: (id: string) => Promise<void>
	/** Refetches the list in the background, e.g. after a connector sync imported items. */
	reload: () => Promise<void>
}

export const ItemsContext = createContext<ItemsContextValue | null>(null)

export function useItems(): ItemsContextValue {
	const value = useContext(ItemsContext)
	if (!value) {
		throw new Error('useItems must be used inside <ItemsProvider>')
	}

	return value
}
