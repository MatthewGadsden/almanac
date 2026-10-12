import { useEffect, useState, type FormEvent } from 'react'
import * as api from '../api'
import type { Connection, SyncSummary, Visibility } from '../api'
import { formatDateTime } from '../items'
import { useItems } from '../state/ItemsContext'

/** Admin: connect services that items are imported from, and sync them. */
export function Connections() {
	const [connections, setConnections] = useState<Connection[] | null>(null)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		let cancelled = false
		api.listConnections().then(
			(loaded) => {
				if (!cancelled) {
					setConnections(loaded)
				}
			},
			(e: unknown) => {
				if (!cancelled) {
					setError(api.errorMessageOf(e))
				}
			},
		)

		return () => {
			cancelled = true
		}
	}, [])

	function replace(provider: Connection['provider'], next: Connection | null) {
		setConnections((current) => [
			...(current ?? []).filter((connection) => connection.provider !== provider),
			...(next ? [next] : []),
		])
	}

	return (
		<>
			<h3 className="aside-heading admin-notes-heading">Connections</h3>
			{error && <p className="notice notice-error">{error}</p>}
			{!connections && !error && <p className="notice">Loading…</p>}
			{connections && (
				<div className="connections">
					<LetterboxdConnection
						connection={connections.find((c) => c.provider === 'letterboxd') ?? null}
						onChange={(next) => replace('letterboxd', next)}
					/>
				</div>
			)}
		</>
	)
}

function LetterboxdConnection({
	connection,
	onChange,
}: {
	connection: Connection | null
	onChange: (next: Connection | null) => void
}) {
	const { reload } = useItems()
	const [editing, setEditing] = useState(false)
	const [busy, setBusy] = useState<'saving' | 'syncing' | 'removing' | null>(null)
	const [result, setResult] = useState<SyncSummary | null>(null)
	const [error, setError] = useState<string | null>(null)

	/** Syncs, then refreshes the connection (for its status) and the garden (for new items). */
	async function sync() {
		setBusy('syncing')
		setError(null)
		setResult(null)
		try {
			setResult(await api.syncConnection('letterboxd'))
		} catch (e) {
			setError(api.errorMessageOf(e))
		}

		const refreshed = await api.listConnections().catch(() => null)
		const latest = refreshed?.find((c) => c.provider === 'letterboxd')
		if (latest) {
			onChange(latest)
		}
		await reload().catch(() => undefined)
		setBusy(null)
	}

	async function save(username: string, importVisibility: Visibility) {
		setBusy('saving')
		setError(null)
		try {
			onChange(await api.saveConnection('letterboxd', { username }, importVisibility))
			setEditing(false)
		} catch (e) {
			setError(api.errorMessageOf(e))
			setBusy(null)
			return
		}
		// Pull the feed straight away so connecting shows results
		await sync()
	}

	async function disconnect() {
		if (!window.confirm('Disconnect Letterboxd? Films it already imported stay in the garden.')) {
			return
		}
		setBusy('removing')
		setError(null)
		try {
			await api.deleteConnection('letterboxd')
			onChange(null)
			setResult(null)
		} catch (e) {
			setError(api.errorMessageOf(e))
		}
		setBusy(null)
	}

	const showForm = !connection || editing

	return (
		<section className="connection">
			<div className="connection-header">
				<div>
					<h4 className="connection-name">Letterboxd</h4>
					<p className="connection-description">
						Imports your diary entries and reviews from your public RSS feed (the latest 50) as films.
					</p>
				</div>
				{connection && !editing && (
					<div className="note-actions connection-actions">
						<button type="button" className="pill-button" onClick={sync} disabled={busy !== null}>
							{busy === 'syncing' ? 'Syncing…' : 'Sync now'}
						</button>
						<button
							type="button"
							className="pill-button pill-button-quiet"
							onClick={() => setEditing(true)}
							disabled={busy !== null}
						>
							Edit
						</button>
						<button
							type="button"
							className="pill-button pill-button-danger"
							onClick={disconnect}
							disabled={busy !== null}
						>
							Disconnect
						</button>
					</div>
				)}
			</div>

			{connection && !editing && (
				<dl className="connection-status">
					<div>
						<dt>Account</dt>
						<dd>
							<a
								href={`https://letterboxd.com/${connection.settings.username}/`}
								target="_blank"
								rel="noreferrer"
							>
								{connection.settings.username} ↗
							</a>
						</dd>
					</div>
					<div>
						<dt>Imports are</dt>
						<dd>{connection.import_visibility === 'private' ? 'Private' : 'Public'}</dd>
					</div>
					<div>
						<dt>Last synced</dt>
						<dd>{connection.last_synced_at ? formatDateTime(connection.last_synced_at) : 'Never'}</dd>
					</div>
					{connection.last_sync_summary && (
						<div>
							<dt>Last sync</dt>
							<dd>{describeSummary(connection.last_sync_summary)}</dd>
						</div>
					)}
				</dl>
			)}

			{showForm && (
				<ConnectionForm
					initial={connection}
					busy={busy !== null}
					onSave={save}
					onCancel={connection ? () => setEditing(false) : undefined}
				/>
			)}

			{result && <p className="notice connection-result">Synced: {describeSummary(result)}.</p>}
			{(error ?? connection?.last_error) && (
				<p className="notice notice-error">{error ?? connection?.last_error}</p>
			)}
			<p className="field-hint">
				Each sync refreshes ratings, likes, rewatches, posters and watch dates from Letterboxd. Titles,
				reviews and topics you've edited here are left alone.
			</p>
		</section>
	)
}

function ConnectionForm({
	initial,
	busy,
	onSave,
	onCancel,
}: {
	initial: Connection | null
	busy: boolean
	onSave: (username: string, importVisibility: Visibility) => void
	onCancel?: () => void
}) {
	const [username, setUsername] = useState(initial?.settings.username ?? '')
	const [visibility, setVisibility] = useState<Visibility>(initial?.import_visibility ?? 'public')

	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		onSave(username.trim(), visibility)
	}

	return (
		<form className="note-form connection-form" onSubmit={handleSubmit}>
			<div className="note-form-row">
				<label>
					<span>Letterboxd username</span>
					<input
						value={username}
						onChange={(e) => setUsername(e.target.value)}
						placeholder="e.g. dave"
						pattern="[A-Za-z0-9_]+"
						title="Letters, digits and underscores, as in your profile URL"
						required
					/>
				</label>
				<label>
					<span>Imported films are</span>
					<select value={visibility} onChange={(e) => setVisibility(e.target.value as Visibility)}>
						<option value="public">Public: anyone can see them</option>
						<option value="private">Private: only you, when logged in</option>
					</select>
				</label>
			</div>
			<div className="note-actions">
				<button type="submit" className="pill-button" disabled={busy}>
					{busy ? 'Connecting…' : initial ? 'Save and sync' : 'Connect and sync'}
				</button>
				{onCancel && (
					<button type="button" className="pill-button pill-button-quiet" onClick={onCancel}>
						Cancel
					</button>
				)}
			</div>
		</form>
	)
}

function describeSummary(summary: SyncSummary): string {
	const parts = [
		`${summary.created} new`,
		`${summary.updated} updated`,
		`${summary.unchanged} unchanged`,
	]
	if (summary.skipped) {
		parts.push(`${summary.skipped} skipped (lists etc.)`)
	}
	return parts.join(', ')
}
