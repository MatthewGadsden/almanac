import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'
import { errorMessageOf } from '../api'
import { useAuth } from '../auth/AuthContext'
import { useEntries } from '../entries/EntriesContext'
import { byRecentlyUpdated, editNotePath, formatDate, notePath } from '../notes'

/** /admin: the login form, or the admin's dashboard once logged in. Nothing on the site links here. */
export function AdminPage() {
	const { admin, status } = useAuth()

	if (status === 'checking') {
		return <p className="notice">Checking your session…</p>
	}

	return admin ? <AdminDashboard username={admin.username} /> : <LoginForm />
}

function LoginForm() {
	const { login } = useAuth()
	const [username, setUsername] = useState('')
	const [password, setPassword] = useState('')
	const [submitting, setSubmitting] = useState(false)
	const [error, setError] = useState<string | null>(null)

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setSubmitting(true)
		setError(null)
		try {
			await login(username, password)
		} catch (e) {
			setError(errorMessageOf(e))
			setPassword('')
			setSubmitting(false)
		}
	}

	return (
		<section className="login">
			<h2 className="section-heading">Admin</h2>
			<p className="notice">Log in to plant, tend and prune notes.</p>
			<form className="note-form" onSubmit={handleSubmit}>
				<label>
					<span>Username</span>
					<input
						value={username}
						onChange={(e) => setUsername(e.target.value)}
						autoComplete="username"
						required
						autoFocus
					/>
				</label>
				<label>
					<span>Password</span>
					<input
						type="password"
						value={password}
						onChange={(e) => setPassword(e.target.value)}
						autoComplete="current-password"
						required
					/>
				</label>

				{error && <p className="notice notice-error">{error}</p>}

				<div className="note-actions">
					<button type="submit" className="pill-button" disabled={submitting}>
						{submitting ? 'Logging in…' : 'Log in'}
					</button>
				</div>
			</form>
		</section>
	)
}

function AdminDashboard({ username }: { username: string }) {
	const { logout } = useAuth()
	const { entries, status } = useEntries()
	const [error, setError] = useState<string | null>(null)

	async function handleLogout() {
		setError(null)
		try {
			await logout()
		} catch (e) {
			setError(errorMessageOf(e))
		}
	}

	const notes = [...entries].sort(byRecentlyUpdated)

	return (
		<section>
			<h2 className="section-heading">Admin</h2>
			<p className="notice">
				Logged in as <strong className="admin-name">{username}</strong>.
			</p>
			<div className="note-actions">
				<Link className="pill-button" to="/notes/new">
					Plant a note
				</Link>
				<button type="button" className="pill-button pill-button-quiet" onClick={handleLogout}>
					Log out
				</button>
			</div>
			{error && <p className="notice notice-error">{error}</p>}

			<h3 className="aside-heading admin-notes-heading">Notes ({notes.length})</h3>
			{status === 'ready' && notes.length === 0 && <p className="notice">Nothing planted yet.</p>}
			{notes.length > 0 && (
				<ul className="admin-notes">
					{notes.map((entry) => (
						<li key={entry.id}>
							<Link to={notePath(entry.id)}>{entry.title}</Link>
							<span className="meta">{entry.topic}</span>
							<span className="meta">Tended {formatDate(entry.updated_at)}</span>
							<Link className="admin-edit" to={editNotePath(entry.id)}>
								Tend
							</Link>
						</li>
					))}
				</ul>
			)}
		</section>
	)
}
