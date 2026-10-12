import type { FilmItem } from '../api'
import { stars } from '../items'

/** The film's poster, or a typographic stand-in when it has no poster URL. */
export function FilmPoster({ film, size = 'medium' }: { film: FilmItem; size?: 'small' | 'medium' | 'large' }) {
	if (film.data.poster_url) {
		return (
			<img
				className={`poster poster-${size}`}
				src={film.data.poster_url}
				alt={`Poster for ${film.title}`}
				loading="lazy"
				referrerPolicy="no-referrer"
			/>
		)
	}

	return (
		<div className={`poster poster-${size} poster-placeholder`} aria-hidden="true">
			<span>{film.title}</span>
		</div>
	)
}

export function Rating({ rating }: { rating?: number }) {
	if (!rating) {
		return null
	}

	return (
		<span className="rating" role="img" aria-label={`${rating} out of 5 stars`}>
			{stars(rating)}
		</span>
	)
}

/** Liked / rewatch markers, as on a Letterboxd diary. */
export function FilmFlags({ film }: { film: FilmItem }) {
	return (
		<>
			{film.data.liked && (
				<span className="film-flag film-flag-liked" title="Liked" aria-label="Liked">
					♥
				</span>
			)}
			{film.data.rewatch && (
				<span className="film-flag" title="Rewatch" aria-label="Rewatch">
					↻
				</span>
			)}
		</>
	)
}
