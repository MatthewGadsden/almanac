import { Link } from 'react-router'
import type { ActivityItem, FilmItem, Item, NoteItem } from '../api'
import { excerpt, formatDate, isActivity, isFilm, sportLabel } from '../items'
import { itemPath } from '../kinds'
import { ActivityStats } from './Activity'
import { FilmFlags, FilmPoster, Rating } from './Film'
import { PrivateBadge } from './PrivateBadge'

export function ItemCard({ item, showTopic = true }: { item: Item; showTopic?: boolean }) {
	if (isFilm(item)) {
		return <FilmCard film={item} />
	}
	if (isActivity(item)) {
		return <ActivityCard activity={item} />
	}
	return <NoteCard note={item} showTopic={showTopic} />
}

function ActivityCard({ activity }: { activity: ActivityItem }) {
	return (
		<article className="note-card">
			<h3>
				<Link to={itemPath(activity)}>{activity.title}</Link> <PrivateBadge item={activity} />
			</h3>
			<div className="meta">
				{sportLabel(activity.data.sport)} · {formatDate(activity.occurred_at)}
			</div>
			<ActivityStats activity={activity.data} size="small" />
		</article>
	)
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
