import { Link } from 'react-router'
import { useAuth } from '../auth/AuthContext'
import { FilmFlags, FilmPoster, Rating } from '../components/Film'
import { LoadState } from '../components/LoadState'
import { PrivateBadge } from '../components/PrivateBadge'
import { byRecentlyOccurred, groupByMonth, isFilm, stars } from '../items'
import { itemPath, KINDS, newItemPath } from '../kinds'
import { TMDB_NOTICE, TMDB_URL } from '../site'
import { useItems } from '../state/ItemsContext'

/** /films: every film, newest first, grouped by the month it was watched. */
export function FilmsPage() {
	const { items, status } = useItems()
	const { admin } = useAuth()

	if (status !== 'ready') {
		return <LoadState />
	}

	const films = items.filter(isFilm).sort(byRecentlyOccurred)
	const thisYear = new Date().getFullYear()
	const watchedThisYear = films.filter((film) => new Date(film.occurred_at).getFullYear() === thisYear).length
	// Films logged by hand and linked to TMDB took their details and poster from it
	const usesTmdb = films.some((film) => film.source === 'manual' && film.data.tmdb_id)
	const ratings = films.flatMap((film) => (film.data.rating ? [film.data.rating] : []))
	const averageRating = ratings.length
		? Math.round((ratings.reduce((sum, rating) => sum + rating, 0) / ratings.length) * 2) / 2
		: null

	return (
		<section>
			<div className="section-header">
				<h2 className="section-heading">Film diary</h2>
				{admin && (
					<Link className="pill-button" to={newItemPath('film')}>
						{KINDS.film.create}
					</Link>
				)}
			</div>

			{films.length === 0 ? (
				<p className="notice">No films logged yet.</p>
			) : (
				<>
					<p className="notice diary-stats">
						{films.length} {films.length === 1 ? 'film' : 'films'} · {watchedThisYear} in {thisYear}
						{averageRating !== null && (
							<>
								{' '}
								· average <span className="rating">{stars(averageRating)}</span>
							</>
						)}
					</p>

					{groupByMonth(films).map(([month, monthFilms]) => (
						<div className="diary-month" key={month}>
							<h3 className="aside-heading">{month}</h3>
							<ol className="diary">
								{monthFilms.map((film) => (
									<li key={film.id}>
										<Link className="diary-row" to={itemPath(film)}>
											<span className="diary-day">{new Date(film.occurred_at).getDate()}</span>
											<FilmPoster film={film} size="small" />
											<span className="diary-title">
												<span className="diary-film">{film.title}</span>
												<span className="meta">
													{[film.data.year, film.data.director].filter(Boolean).join(' · ')}
												</span>
											</span>
											<span className="diary-rating">
												<Rating rating={film.data.rating} />
											</span>
											<span className="diary-flags">
												<FilmFlags film={film} />
												<PrivateBadge item={film} />
											</span>
										</Link>
									</li>
								))}
							</ol>
						</div>
					))}
				</>
			)}

			{usesTmdb && (
				<p className="field-hint tmdb-credit">
					Some film details and posters come from{' '}
					<a href={TMDB_URL} target="_blank" rel="noreferrer">
						TMDB
					</a>
					. {TMDB_NOTICE}
				</p>
			)}
		</section>
	)
}
