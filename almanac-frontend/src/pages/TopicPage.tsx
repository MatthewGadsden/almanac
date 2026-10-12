import { Link, useParams } from 'react-router'
import { LoadState } from '../components/LoadState'
import { NoteCard } from '../components/NoteCard'
import { useEntries } from '../entries/EntriesContext'
import { byTitle } from '../notes'
import { NotFoundPage } from './NotFoundPage'

export function TopicPage() {
	const { topic } = useParams()
	const { entries, status } = useEntries()

	if (status !== 'ready') {
		return <LoadState />
	}

	const notes = entries.filter((entry) => entry.topic === topic).sort(byTitle)
	if (!topic || notes.length === 0) {
		return <NotFoundPage />
	}

	return (
		<section>
			<nav className="breadcrumbs" aria-label="Breadcrumbs">
				<Link to="/">Notes</Link>
				<span aria-hidden="true">/</span>
				<span>{topic}</span>
			</nav>
			<h2 className="page-title">{topic}</h2>
			<p className="notice">
				{notes.length} {notes.length === 1 ? 'note' : 'notes'} in this topic.
			</p>
			<div className="note-grid">
				{notes.map((entry) => (
					<NoteCard key={entry.id} entry={entry} showTopic={false} />
				))}
			</div>
		</section>
	)
}
