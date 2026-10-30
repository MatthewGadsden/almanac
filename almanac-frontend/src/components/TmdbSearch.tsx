import { useEffect, useId, useState, type KeyboardEvent } from 'react'
import * as api from '../api'
import type { TmdbMovie } from '../api'
import { TMDB_NOTICE, TMDB_URL } from '../site'

/** Searches start once there are this many characters, and wait for typing to pause. */
const MIN_QUERY = 2
const DEBOUNCE_MS = 300

interface TmdbSearchProps {
	/** Called with the full film (including its director) once one is picked. */
	onPick: (movie: TmdbMovie) => void
	autoFocus?: boolean
}

/** Film form: films are picked from TMDB, which supplies their title, year, director and poster. */
export function TmdbSearch({ onPick, autoFocus }: TmdbSearchProps) {
	const listId = useId()
	const [available, setAvailable] = useState<boolean | null>(null)
	const [query, setQuery] = useState('')
	// Results and errors remember the query they're for, so stale ones are never shown
	const [results, setResults] = useState<{ query: string; movies: TmdbMovie[] } | null>(null)
	const [searchError, setSearchError] = useState<{ query: string; message: string } | null>(null)
	const [active, setActive] = useState(0)
	const [open, setOpen] = useState(true)
	const [picking, setPicking] = useState<number | null>(null)
	const [pickError, setPickError] = useState<string | null>(null)

	useEffect(() => {
		let cancelled = false
		api.tmdbStatus().then(
			({ configured }) => !cancelled && setAvailable(configured),
			() => !cancelled && setAvailable(false),
		)
		return () => {
			cancelled = true
		}
	}, [])

	const trimmed = query.trim()

	useEffect(() => {
		if (trimmed.length < MIN_QUERY) {
			return
		}

		let cancelled = false
		const timer = window.setTimeout(() => {
			api.searchTmdb(trimmed).then(
				(movies) => !cancelled && setResults({ query: trimmed, movies }),
				(e: unknown) => !cancelled && setSearchError({ query: trimmed, message: api.errorMessageOf(e) }),
			)
		}, DEBOUNCE_MS)

		return () => {
			cancelled = true
			window.clearTimeout(timer)
		}
	}, [trimmed])

	if (available === false) {
		return (
			<p className="notice notice-error">
				Logging a film needs TMDB, which supplies its title, year, director and poster: set
				connectors.tmdb.read_access_token in the backend (see its README).
			</p>
		)
	}

	const movies = trimmed.length >= MIN_QUERY && results?.query === trimmed ? results.movies : null
	const error = searchError?.query === trimmed ? searchError.message : null
	const searching = trimmed.length >= MIN_QUERY && !movies && !error
	const activeIndex = movies?.length ? Math.min(active, movies.length - 1) : -1
	const expanded = open && movies !== null && movies.length > 0

	async function pick(movie: TmdbMovie) {
		if (picking !== null) {
			return
		}
		setPicking(movie.tmdb_id)
		setPickError(null)
		try {
			// Search results don't include the director, so fetch the film itself
			onPick(await api.getTmdbMovie(movie.tmdb_id))
			setQuery('')
		} catch (e) {
			setPickError(api.errorMessageOf(e))
		}
		setPicking(null)
	}

	function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
		if (event.key === 'Escape') {
			setOpen(false)
			return
		}
		if (!movies?.length) {
			return
		}
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault()
			setOpen(true)
			const step = event.key === 'ArrowDown' ? 1 : -1
			setActive((activeIndex + step + movies.length) % movies.length)
		} else if (event.key === 'Enter' && expanded) {
			event.preventDefault()
			void pick(movies[activeIndex])
		}
	}

	return (
		<div className="note-form tmdb-search">
			<label>
				<span>Film</span>
				<input
					type="search"
					role="combobox"
					aria-expanded={expanded}
					aria-controls={listId}
					aria-autocomplete="list"
					aria-activedescendant={expanded ? `${listId}-${activeIndex}` : undefined}
					value={query}
					onChange={(e) => {
						setQuery(e.target.value)
						setActive(0)
						setOpen(true)
					}}
					onKeyDown={handleKeyDown}
					onFocus={() => setOpen(true)}
					onBlur={() => setOpen(false)}
					placeholder="Search TMDB by title, e.g. Perfect Days"
					autoFocus={autoFocus}
				/>
			</label>

			{expanded && (
				<ul className="tmdb-results" id={listId} role="listbox" aria-label="Films on TMDB">
					{movies.map((movie, index) => (
						<li
							key={movie.tmdb_id}
							id={`${listId}-${index}`}
							role="option"
							aria-selected={index === activeIndex}
							className={index === activeIndex ? 'tmdb-result tmdb-result-active' : 'tmdb-result'}
							// Keeps focus in the input, so blurring doesn't close the list before the click lands
							onMouseDown={(e) => e.preventDefault()}
							onMouseEnter={() => setActive(index)}
							onClick={() => void pick(movie)}
						>
							{movie.thumbnail_url ? (
								<img
									className="poster poster-small"
									src={movie.thumbnail_url}
									alt=""
									loading="lazy"
									referrerPolicy="no-referrer"
								/>
							) : (
								<div className="poster poster-small poster-placeholder" aria-hidden="true" />
							)}
							<span className="tmdb-result-text">
								<span className="tmdb-result-title">
									{movie.title}
									{movie.year && <span className="meta"> ({movie.year})</span>}
								</span>
								<span className="meta tmdb-result-overview">
									{[movie.original_title, movie.overview].filter(Boolean).join(' · ') || 'No synopsis'}
								</span>
							</span>
							{picking === movie.tmdb_id && <span className="meta">Loading…</span>}
						</li>
					))}
				</ul>
			)}

			{searching && <p className="field-hint">Searching…</p>}
			{movies?.length === 0 && <p className="field-hint">No films on TMDB match “{trimmed}”.</p>}
			{(error ?? pickError) && <p className="notice notice-error">{error ?? pickError}</p>}

			<p className="field-hint">
				Pick the film and its title, year, director and poster come from{' '}
				<a href={TMDB_URL} target="_blank" rel="noreferrer">
					TMDB
				</a>
				. {TMDB_NOTICE}
			</p>
		</div>
	)
}
