import { Link, NavLink } from 'react-router'
import { useAuth } from '../auth/AuthContext'
import { useEntries } from '../entries/EntriesContext'
import { groupByTopic, notePath, topicPath } from '../notes'
import { REPO_URL, SLOTH_URL } from '../site'

export function Sidebar() {
	const { admin } = useAuth()

	return (
		<aside className="sidebar">
			<div className="sidebar-content">
				<div className="sidebar-subheader">
					<img
						className="sidebar-sloth"
						src={SLOTH_URL}
						alt="Pixel art sloth"
					/>
					<h1>
						<Link to="/">Almanac</Link>
					</h1>
				</div>
				<p className="role">A digital garden</p>
				<p className="tagline">Notes that are planted, tended and pruned over time.</p>
				<nav className="section-nav" aria-label="Pages">
					<NavLink to="/" end>
						All notes
					</NavLink>
					{admin && <NavLink to="/notes/new">Plant a note</NavLink>}
					{admin && <NavLink to="/admin">Admin</NavLink>}
				</nav>
			</div>

			<TopicIndex />

			<div className="socials">
				<a href={REPO_URL} target="_blank" rel="noreferrer" aria-label="GitHub">
					<svg className="social-icon" viewBox="0 0 16 16" aria-hidden="true">
						<path d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z" />
					</svg>
				</a>
			</div>
		</aside>
	)
}

/** Garden-style tree of every note, grouped by topic. */
function TopicIndex() {
	const { entries, status } = useEntries()
	const topics = groupByTopic(entries)

	return (
		<nav className="topic-index" aria-label="Topic index">
			<h2 className="index-heading">Index</h2>
			{status === 'loading' && <p className="index-empty">Loading…</p>}
			{status === 'error' && <p className="index-empty">Index unavailable.</p>}
			{status === 'ready' && topics.length === 0 && <p className="index-empty">Nothing planted yet.</p>}
			{topics.length > 0 && (
				<ul className="index-tree">
					{topics.map(([topic, notes]) => (
						<li key={topic}>
							<NavLink className="index-topic" to={topicPath(topic)}>
								{topic}
							</NavLink>
							<ul>
								{notes.map((note) => (
									<li key={note.id}>
										<NavLink className="index-note" to={notePath(note.id)}>
											{note.title}
										</NavLink>
									</li>
								))}
							</ul>
						</li>
					))}
				</ul>
			)}
		</nav>
	)
}
