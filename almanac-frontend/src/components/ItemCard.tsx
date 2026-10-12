import { Link } from 'react-router'
import type { FilmItem, Item, NoteItem } from '../api'
import { excerpt, formatDate, isFilm } from '../items'
import { itemPath } from '../kinds'
import { FilmFlags, FilmPoster, Rating } from './Film'
import { PrivateBadge } from './PrivateBadge'

export function ItemCard({ item, showTopic = true }: { item: Item; showTopic?: boolean }) {
	return isFilm(item) ? <FilmCard film={item} /> : <NoteCard note={item} showTopic={showTopic} />
}

function NoteCard({ note, showTopic }: { note: NoteItem; showTopic: boolean }) {
	return (
		<article className="note-card">
			<h3>
				<Link to={itemPath(note)}>{note.title}</Link> <PrivateBadge item={note} />
			</h3>
			<div className="meta">
				{showTopic && note.topic && `${note.topic} · `}Planted {formatDate(note.created_at)}
			</div>
			{note.body && <p>{excerpt(note.body)}</p>}
		</article>
	)
}

/** A poster tile, as in a film diary grid. */
export function FilmCard({ film }: { film: FilmItem }) {
	return (
		<article className="film-card">
			<Link to={itemPath(film)} className="film-card-link">
				<FilmPoster film={film} />
				<span className="film-card-title">{film.title}</span>
				<span className="film-card-meta">
					<Rating rating={film.data.rating} />
					<FilmFlags film={film} />
					<PrivateBadge item={film} />
				</span>
			</Link>
		</article>
	)
}
