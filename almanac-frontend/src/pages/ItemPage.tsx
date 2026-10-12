import { useState, type ReactNode } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { errorMessageOf, type FilmItem, type Item, type ItemKind, type NoteItem } from '../api'
import { useAuth } from '../auth/AuthContext'
import { FilmFlags, FilmPoster, Rating } from '../components/Film'
import { LoadState } from '../components/LoadState'
import { NoteBody } from '../components/NoteBody'
import { PrivateBadge } from '../components/PrivateBadge'
import { byRecentlyOccurred, byTitle, formatDate, isFilm, isTended } from '../items'
import { editItemPath, itemPath, topicPath } from '../kinds'
import { useItems } from '../state/ItemsContext'
import { NotFoundPage } from './NotFoundPage'

/** /notes/:id and /films/:id. The body column is shared; the header and aside vary by kind. */
export function ItemPage({ kind }: { kind: ItemKind }) {
	const { id } = useParams()
	const navigate = useNavigate()
	const { items, status, deleteItem } = useItems()
	const { admin } = useAuth()
	const [deleting, setDeleting] = useState(false)
	const [error, setError] = useState<string | null>(null)

	if (status !== 'ready') {
		return <LoadState />
	}

	const item = items.find((candidate) => candidate.id === id && candidate.kind === kind)
	if (!item) {
		return <NotFoundPage />
	}

	async function handleDelete() {
		if (!item || !window.confirm(`Delete "${item.title}"? This can't be undone.`)) {
			return
		}

		setDeleting(true)
		setError(null)
		try {
			await deleteItem(item.id)
			navigate(item.kind === 'film' ? '/films' : '/')
		} catch (e) {
			setError(errorMessageOf(e))
			setDeleting(false)
		}
	}

	return (
		<article className={`note note-${item.kind}`}>
			{isFilm(item) ? <FilmHeader film={item} /> : <NoteHeader note={item} />}

			<div className="note-main">
				{item.body.trim() ? (
					<NoteBody body={item.body} />
				) : (
					<p className="notice">
						{isFilm(item) ? 'No review written yet.' : 'This note is still a seed — it has no body yet.'}
					</p>
				)}
			</div>

			<aside className="note-aside">
				<dl className="note-meta">
					{isFilm(item) ? (
						<>
							<MetaRow label="Watched">{formatDate(item.occurred_at)}</MetaRow>
							<MetaRow label="Logged">{formatDate(item.created_at)}</MetaRow>
						</>
					) : (
						<MetaRow label="Planted">{formatDate(item.created_at)}</MetaRow>
					)}
					<MetaRow label="Tended">{isTended(item) ? formatDate(item.updated_at) : '—'}</MetaRow>
					{item.topic && (
						<MetaRow label="Topic">
							<Link to={topicPath(item.topic)}>{item.topic}</Link>
						</MetaRow>
					)}
					{item.visibility === 'private' && (
						<MetaRow label="Visibility">
							<PrivateBadge item={item} />
						</MetaRow>
					)}
				</dl>

				{admin && (
					<div className="note-actions">
						<Link className="pill-button" to={editItemPath(item)}>
							Tend (edit)
						</Link>
						<button
							type="button"
							className="pill-button pill-button-danger"
							onClick={handleDelete}
							disabled={deleting}
						>
							{deleting ? 'Pruning…' : 'Prune (delete)'}
						</button>
					</div>
				)}
				{error && <p className="notice notice-error">{error}</p>}

				<WhereNext item={item} items={items} />
			</aside>
		</article>
	)
}

function MetaRow({ label, children }: { label: string; children: ReactNode }) {
	return (
		<div>
			<dt>{label}</dt>
			<dd>{children}</dd>
		</div>
	)
}

function NoteHeader({ note }: { note: NoteItem }) {
	return (
		<header className="note-header">
			<nav className="breadcrumbs" aria-label="Breadcrumbs">
				<Link to="/">Notes</Link>
				{note.topic && (
					<>
						<span aria-hidden="true">/</span>
						<Link to={topicPath(note.topic)}>{note.topic}</Link>
					</>
				)}
				<span aria-hidden="true">/</span>
				<span>{note.title}</span>
			</nav>
			<h1 className="page-title">{note.title}</h1>
		</header>
	)
}

function FilmHeader({ film }: { film: FilmItem }) {
	const details = [film.data.year, film.data.director && `Directed by ${film.data.director}`].filter(Boolean)

	return (
		<header className="note-header film-header">
			<FilmPoster film={film} size="large" />
			<div className="film-header-text">
				<nav className="breadcrumbs" aria-label="Breadcrumbs">
					<Link to="/films">Film diary</Link>
					<span aria-hidden="true">/</span>
					<span>{film.title}</span>
				</nav>
				<h1 className="page-title">{film.title}</h1>
				{details.length > 0 && <p className="film-details">{details.join(' · ')}</p>}
				<p className="film-verdict">
					<Rating rating={film.data.rating} />
					<FilmFlags film={film} />
				</p>
			</div>
		</header>
	)
}

/**
 * Notes: the other notes in the same topic. Films: the films watched just before and after,
 * in diary order. Either way the current item is marked "You are here".
 */
function WhereNext({ item, items }: { item: Item; items: Item[] }) {
	let neighbours: Item[]
	if (isFilm(item)) {
		const films = items.filter(isFilm).sort(byRecentlyOccurred)
		const index = films.findIndex((film) => film.id === item.id)
		neighbours = films.slice(Math.max(0, index - 3), index + 4)
	} else {
		neighbours = item.topic
			? items.filter((candidate) => candidate.kind === 'note' && candidate.topic === item.topic).sort(byTitle)
			: [item]
	}

	return (
		<section className="where-next">
			<h2 className="aside-heading">{isFilm(item) ? 'In the diary' : 'Where to next?'}</h2>
			<ul className="where-next-list">
				{neighbours.map((neighbour) => (
					<li key={neighbour.id}>
						{neighbour.id === item.id ? (
							<span className="where-next-current">
								{neighbour.title}
								<span className="status-badge">You are here</span>
							</span>
						) : (
							<Link to={itemPath(neighbour)}>{neighbour.title}</Link>
						)}
					</li>
				))}
			</ul>
		</section>
	)
}
