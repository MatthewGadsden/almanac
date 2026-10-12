import { Link } from 'react-router'
import type { Entry } from '../api'
import { excerpt, formatDate, notePath } from '../notes'

export function NoteCard({ entry, showTopic = true }: { entry: Entry; showTopic?: boolean }) {
	return (
		<article className="note-card">
			<h3>
				<Link to={notePath(entry.id)}>{entry.title}</Link>
			</h3>
			<div className="meta">
				{showTopic && `${entry.topic} · `}Planted {formatDate(entry.created_at)}
			</div>
			{entry.body && <p>{excerpt(entry.body)}</p>}
		</article>
	)
}
