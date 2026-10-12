import { Link } from 'react-router'
import { useAuth } from '../auth/AuthContext'
import { FilmFlags, FilmPoster, Rating } from '../components/Film'
import { ItemCard } from '../components/ItemCard'
import { LoadState } from '../components/LoadState'
import { byRecentlyOccurred, byRecentlyUpdated, byTitle, excerpt, formatDate, isFilm, isNote, isTended, stars } from '../items'
import { itemPath, KINDS, newItemPath } from '../kinds'
import { useItems } from '../state/ItemsContext'
import type { FilmItem, Item } from '../api'

export function HomePage() {
	const { items, status } = useItems()
	const { admin } = useAuth()

	if (status !== 'ready') {
		return <LoadState />
	}

	if (items.length === 0) {
		return (
			<section>
				<h2 className="section-heading">Nothing planted yet</h2>
				<p className="notice">The garden is empty. Check back soon.</p>
				{admin && (
					<div className="note-actions">
						<Link className="pill-button" to={newItemPath('note')}>
							{KINDS.note.create}
						</Link>
						<Link className="pill-button" to={newItemPath('film')}>
							{KINDS.film.create}
						</Link>
					</div>
				)}
			</section>
		)
	}

	// Six fills two rows of the three-column grid beside the films column
	const latest = [...items].sort(byRecentlyUpdated).slice(0, 6)
	const recentFilms = items.filter(isFilm).sort(byRecentlyOccurred).slice(0, 4)
	const notes = items.filter(isNote).sort(byTitle)

	return (
		<div className="home-layout">
			<section id="latest" className="home-latest">
				<h2 className="section-heading">Latest updates</h2>
				<ul className="update-list">
					{latest.map((item) => (
						<li key={item.id}>
							<Link className="update-item" to={itemPath(item)}>
								<span className="update-item-meta">
									<span className="status-badge">{isTended(item) ? 'Tended' : 'Planted'}</span>
									<span className="meta">{formatDate(item.updated_at)}</span>
								</span>
								<span className="update-item-title">{item.title}</span>
								<span className="meta">{updateSubtitle(item)}</span>
							</Link>
						</li>
					))}
				</ul>
			</section>

			{recentFilms.length > 0 && (
				<aside className="home-films" aria-labelledby="recent-films-heading">
					<div className="section-header">
						<h2 className="section-heading" id="recent-films-heading">
							Recently watched
						</h2>
						<Link className="section-link" to="/films">
							Diary →
						</Link>
					</div>
					<ol className="recent-films">
						{recentFilms.map((film) => (
							<li key={film.id}>
								<RecentFilm film={film} />
							</li>
						))}
					</ol>
				</aside>
			)}

			{notes.length > 0 && (
				<section id="notes" className="home-notes">
					<h2 className="section-heading">All notes</h2>
					<div className="note-grid">
						{notes.map((note) => (
							<ItemCard key={note.id} item={note} />
						))}
					</div>
				</section>
			)}
		</div>
	)
}

/** A compact review row for the home page's films column: small cover, verdict, review excerpt. */
function RecentFilm({ film }: { film: FilmItem }) {
	return (
		<Link className="recent-film" to={itemPath(film)}>
			<FilmPoster film={film} size="small" />
			<span className="recent-film-text">
				<span className="recent-film-title">
					{film.title}
					{film.data.year && <span className="meta"> {film.data.year}</span>}
				</span>
				<span className="recent-film-meta">
					<Rating rating={film.data.rating} />
					<FilmFlags film={film} />
					<span className="meta">{formatDate(film.occurred_at)}</span>
				</span>
				{film.body && <span className="recent-film-review">{excerpt(film.body, 110)}</span>}
			</span>
		</Link>
	)
}

function updateSubtitle(item: Item): string {
	if (isFilm(item)) {
		return [KINDS.film.label, item.data.rating && stars(item.data.rating)].filter(Boolean).join(' · ')
	}

	return item.topic ?? KINDS[item.kind].label
}
