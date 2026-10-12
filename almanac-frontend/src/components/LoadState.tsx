import { useItems } from '../state/ItemsContext'

/** Shown in place of a page while the item list is loading or failed to load. */
export function LoadState() {
	const { status, error } = useItems()

	if (status === 'error') {
		return (
			<section>
				<h2 className="section-heading">Couldn't load the garden</h2>
				<p className="notice notice-error">{error}</p>
				<p className="notice">
					Start the API with <code>cargo run</code> in <code>almanac-backend/</code>, then refresh.
				</p>
			</section>
		)
	}

	return <p className="notice">Loading…</p>
}
