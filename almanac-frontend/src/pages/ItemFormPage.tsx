import { useState, type FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { errorMessageOf, type FilmData, type Item, type ItemKind, type ItemPayload, type Visibility } from '../api'
import { useAuth } from '../auth/AuthContext'
import { LoadState } from '../components/LoadState'
import { NoteBody } from '../components/NoteBody'
import { fromDateInput, groupByTopic, isFilm, stars, toDateInput } from '../items'
import { itemPath, KINDS } from '../kinds'
import { useItems } from '../state/ItemsContext'
import { NotFoundPage } from './NotFoundPage'

const RATINGS = [0.5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 4.5, 5]

/** /notes/new, /films/new and their /:id/edit pages. Visitors get the not-found page. */
export function ItemFormPage({ kind }: { kind: ItemKind }) {
	const { id } = useParams()
	const { items, status, createItem, updateItem } = useItems()
	const { admin, status: authStatus } = useAuth()

	if (authStatus === 'checking') {
		return <p className="notice">Loading…</p>
	}

	if (!admin) {
		return <NotFoundPage />
	}

	if (status !== 'ready') {
		return <LoadState />
	}

	const topics = groupByTopic(items).map(([topic]) => topic)

	if (!id) {
		return <ItemForm kind={kind} topics={topics} onSave={createItem} />
	}

	const item = items.find((candidate) => candidate.id === id && candidate.kind === kind)
	if (!item) {
		return <NotFoundPage />
	}

	return (
		<ItemForm
			key={item.id}
			kind={kind}
			initial={item}
			topics={topics}
			onSave={(payload) => updateItem(item.id, payload)}
		/>
	)
}

const BODY_PLACEHOLDER = `<style>
  h2 { color: var(--text-emphasis); }
</style>

<p>Start writing…</p>`

interface ItemFormProps {
	kind: ItemKind
	initial?: Item
	topics: string[]
	onSave: (payload: ItemPayload) => Promise<Item>
}

function ItemForm({ kind, initial, topics, onSave }: ItemFormProps) {
	const navigate = useNavigate()
	const film: Partial<FilmData> = initial && isFilm(initial) ? initial.data : {}

	const [title, setTitle] = useState(initial?.title ?? '')
	const [topic, setTopic] = useState(initial?.topic ?? '')
	const [body, setBody] = useState(initial?.body ?? '')
	const [visibility, setVisibility] = useState<Visibility>(initial?.visibility ?? 'public')
	const [watchedOn, setWatchedOn] = useState(toDateInput(initial?.occurred_at ?? new Date().toISOString()))
	const [year, setYear] = useState(film.year?.toString() ?? '')
	const [director, setDirector] = useState(film.director ?? '')
	const [rating, setRating] = useState(film.rating?.toString() ?? '')
	const [liked, setLiked] = useState(film.liked ?? false)
	const [rewatch, setRewatch] = useState(film.rewatch ?? false)
	const [posterUrl, setPosterUrl] = useState(film.poster_url ?? '')
	const [saving, setSaving] = useState(false)
	const [error, setError] = useState<string | null>(null)

	function payload(): ItemPayload {
		const common = { kind, title: title.trim(), topic: topic.trim() || null, body, visibility }
		if (kind !== 'film') {
			return common
		}

		// Empty optional fields are sent as undefined, which JSON.stringify drops
		return {
			...common,
			occurred_at: fromDateInput(watchedOn),
			data: {
				// Keep fields the form doesn't edit, e.g. letterboxd_url and tmdb_id on imported films
				...film,
				year: year ? Number(year) : undefined,
				director: director.trim() || undefined,
				rating: rating ? Number(rating) : undefined,
				liked,
				rewatch,
				poster_url: posterUrl.trim() || undefined,
			},
		}
	}

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setSaving(true)
		setError(null)
		try {
			const saved = await onSave(payload())
			navigate(itemPath(saved))
		} catch (e) {
			setError(errorMessageOf(e))
			setSaving(false)
		}
	}

	const heading = initial ? `Tend ${KINDS[kind].label.toLowerCase()}` : KINDS[kind].create
	const imported = initial && initial.source !== 'manual' ? initial.source : null

	return (
		<section>
			<h2 className="section-heading">{heading}</h2>
			{imported === 'letterboxd' && (
				<p className="notice">
					Imported from Letterboxd. Your title, topic and review edits are kept, but the rating, liked,
					rewatch, year, poster and watched date are refreshed from Letterboxd on every sync.
				</p>
			)}
			<form className="note-form" onSubmit={handleSubmit}>
				<div className="note-form-row">
					<label>
						<span>Title</span>
						<input value={title} onChange={(e) => setTitle(e.target.value)} required autoFocus />
					</label>

					<label>
						<span>{kind === 'note' ? 'Topic' : 'Topic (optional)'}</span>
						<input
							value={topic}
							onChange={(e) => setTopic(e.target.value)}
							list="topic-options"
							required={kind === 'note'}
						/>
						<datalist id="topic-options">
							{topics.map((option) => (
								<option key={option} value={option} />
							))}
						</datalist>
					</label>
				</div>

				{kind === 'film' && (
					<>
						<div className="note-form-row note-form-row-4">
							<label>
								<span>Watched on</span>
								<input type="date" value={watchedOn} onChange={(e) => setWatchedOn(e.target.value)} required />
							</label>
							<label>
								<span>Year</span>
								<input
									type="number"
									min={1870}
									max={2100}
									value={year}
									onChange={(e) => setYear(e.target.value)}
								/>
							</label>
							<label>
								<span>Director</span>
								<input value={director} onChange={(e) => setDirector(e.target.value)} />
							</label>
							<label>
								<span>Rating</span>
								<select value={rating} onChange={(e) => setRating(e.target.value)}>
									<option value="">No rating</option>
									{RATINGS.map((value) => (
										<option key={value} value={value}>
											{stars(value)} ({value})
										</option>
									))}
								</select>
							</label>
						</div>

						<div className="note-form-row">
							<label>
								<span>Poster URL</span>
								<input
									type="url"
									value={posterUrl}
									onChange={(e) => setPosterUrl(e.target.value)}
									placeholder="https://…"
								/>
							</label>
							<div className="checkbox-row">
								<label className="checkbox">
									<input type="checkbox" checked={liked} onChange={(e) => setLiked(e.target.checked)} />
									<span>Liked ♥</span>
								</label>
								<label className="checkbox">
									<input type="checkbox" checked={rewatch} onChange={(e) => setRewatch(e.target.checked)} />
									<span>Rewatch ↻</span>
								</label>
							</div>
						</div>
					</>
				)}

				<div className="note-editor">
					<label>
						<span>{kind === 'film' ? 'Review (HTML)' : 'Body (HTML)'}</span>
						<textarea
							className="code-input"
							value={body}
							onChange={(e) => setBody(e.target.value)}
							spellCheck={false}
							placeholder={BODY_PLACEHOLDER}
						/>
						<small className="field-hint">
							Any HTML works, including <code>&lt;style&gt;</code> blocks, which only apply to this page.
							Scripts and event handlers are removed. Text without tags is shown as plain paragraphs.
						</small>
					</label>

					<div className="note-preview">
						<span>Preview</span>
						<div className="note-preview-panel">
							{body.trim() ? <NoteBody body={body} /> : <p className="notice">Nothing to preview yet.</p>}
						</div>
					</div>
				</div>

				<label className="visibility-field">
					<span>Visibility</span>
					<select value={visibility} onChange={(e) => setVisibility(e.target.value as Visibility)}>
						<option value="public">Public: anyone can see it</option>
						<option value="private">Private: only you, when logged in</option>
					</select>
				</label>

				{error && <p className="notice notice-error">{error}</p>}

				<div className="note-actions">
					<button type="submit" className="pill-button" disabled={saving}>
						{saving ? 'Saving…' : `Save ${KINDS[kind].label.toLowerCase()}`}
					</button>
					<Link className="pill-button pill-button-quiet" to={initial ? itemPath(initial) : '/admin'}>
						Cancel
					</Link>
				</div>
			</form>
		</section>
	)
}
