import { Link } from 'react-router'
import { LoadState } from '../components/LoadState'
import { NoteCard } from '../components/NoteCard'
import { useAuth } from '../auth/AuthContext'
import { useEntries } from '../entries/EntriesContext'
import { byRecentlyUpdated, byTitle, formatDate, isTended, notePath } from '../notes'

export function HomePage() {
	const { entries, status } = useEntries()
	const { admin } = useAuth()

	if (status !== 'ready') {
		return <LoadState />
	}

	if (entries.length === 0) {
		return (
			<section>
				<h2 className="section-heading">Nothing planted yet</h2>
				<p className="notice">The garden is empty. Check back soon.</p>
				{admin && (
					<Link className="pill-button" to="/notes/new">
						Plant a note
					</Link>
				)}
			</section>
		)
	}

	const latest = [...entries].sort(byRecentlyUpdated).slice(0, 5)
	const notes = [...entries].sort(byTitle)

	return (
		<>
			<section id="latest">
				<h2 className="section-heading">Latest updates</h2>
				<ul className="update-list">
					{latest.map((entry) => (
						<li key={entry.id}>
							<Link className="update-item" to={notePath(entry.id)}>
								<span className="update-item-meta">
									<span className="status-badge">{isTended(entry) ? 'Tended' : 'Planted'}</span>
									<span className="meta">{formatDate(entry.updated_at)}</span>
								</span>
								<span className="update-item-title">{entry.title}</span>
								<span className="meta">{entry.topic}</span>
							</Link>
						</li>
					))}
				</ul>
			</section>

			<section id="notes">
				<h2 className="section-heading">All notes</h2>
				<div className="note-grid">
					{notes.map((entry) => (
						<NoteCard key={entry.id} entry={entry} />
					))}
				</div>
			</section>
		</>
	)
}
