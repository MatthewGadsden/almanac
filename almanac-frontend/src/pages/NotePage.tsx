import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { errorMessageOf } from '../api'
import { useAuth } from '../auth/AuthContext'
import { LoadState } from '../components/LoadState'
import { NoteBody } from '../components/NoteBody'
import { useEntries } from '../entries/EntriesContext'
import { byTitle, editNotePath, formatDate, isTended, notePath, topicPath } from '../notes'
import { NotFoundPage } from './NotFoundPage'

export function NotePage() {
	const { id } = useParams()
	const navigate = useNavigate()
	const { entries, status, deleteEntry } = useEntries()
	const { admin } = useAuth()
	const [deleting, setDeleting] = useState(false)
	const [error, setError] = useState<string | null>(null)

	if (status !== 'ready') {
		return <LoadState />
	}

	const entry = entries.find((candidate) => candidate.id === id)
	if (!entry) {
		return <NotFoundPage />
	}

	const siblings = entries.filter((candidate) => candidate.topic === entry.topic).sort(byTitle)

	async function handleDelete() {
		if (!entry || !window.confirm(`Delete "${entry.title}"? This can't be undone.`)) {
			return
		}

		setDeleting(true)
		setError(null)
		try {
			await deleteEntry(entry.id)
			navigate('/')
		} catch (e) {
			setError(errorMessageOf(e))
			setDeleting(false)
		}
	}

	return (
		<article className="note">
			<header className="note-header">
				<nav className="breadcrumbs" aria-label="Breadcrumbs">
					<Link to="/">Notes</Link>
					<span aria-hidden="true">/</span>
					<Link to={topicPath(entry.topic)}>{entry.topic}</Link>
					<span aria-hidden="true">/</span>
					<span>{entry.title}</span>
				</nav>
				<h1 className="page-title">{entry.title}</h1>
			</header>

			<div className="note-main">
				{entry.body.trim() ? (
					<NoteBody body={entry.body} />
				) : (
					<p className="notice">This note is still a seed — it has no body yet.</p>
				)}
			</div>

			<aside className="note-aside">
				<dl className="note-meta">
					<div>
						<dt>Planted</dt>
						<dd>{formatDate(entry.created_at)}</dd>
					</div>
					<div>
						<dt>Tended</dt>
						<dd>{isTended(entry) ? formatDate(entry.updated_at) : '—'}</dd>
					</div>
					<div>
						<dt>Topic</dt>
						<dd>
							<Link to={topicPath(entry.topic)}>{entry.topic}</Link>
						</dd>
					</div>
				</dl>

				{admin && (
					<div className="note-actions">
						<Link className="pill-button" to={editNotePath(entry.id)}>
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

				<section className="where-next">
					<h2 className="aside-heading">Where to next?</h2>
					<ul className="where-next-list">
						{siblings.map((sibling) => (
							<li key={sibling.id}>
								{sibling.id === entry.id ? (
									<span className="where-next-current">
										{sibling.title}
										<span className="status-badge">You are here</span>
									</span>
								) : (
									<Link to={notePath(sibling.id)}>{sibling.title}</Link>
								)}
							</li>
						))}
					</ul>
				</section>
			</aside>
		</article>
	)
}
