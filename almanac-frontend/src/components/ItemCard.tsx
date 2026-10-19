import { Link } from 'react-router'
import type { ActivityItem, FilmItem, Item, NoteItem } from '../api'
import { excerpt, formatDate, isActivity, isFilm } from '../items'
import { itemPath } from '../kinds'
import { RouteOutline } from './Activity'
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

/** A tile like a film's, with the route's outline in place of a poster. */
function ActivityCard({ activity }: { activity: ActivityItem }) {
	return (
		<article className="film-card">
			<Link to={itemPath(activity)} className="film-card-link">
				<RouteOutline outline={activity.data.route_outline} className="activity-card-route" />
				<span className="film-card-title">{activity.title}</span>
				<span className="film-card-meta">
					<span className="meta">{formatDate(activity.occurred_at)}</span>
					<PrivateBadge item={activity} />
				</span>
			</Link>
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
