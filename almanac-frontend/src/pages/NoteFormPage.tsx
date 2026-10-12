import { useState, type FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import { errorMessageOf, type Entry, type EntryPayload } from '../api'
import { useAuth } from '../auth/AuthContext'
import { LoadState } from '../components/LoadState'
import { NoteBody } from '../components/NoteBody'
import { useEntries } from '../entries/EntriesContext'
import { groupByTopic, notePath } from '../notes'
import { NotFoundPage } from './NotFoundPage'

/** Handles both /notes/new and /notes/:id/edit. */
export function NoteFormPage() {
	const { id } = useParams()
	const { entries, status, createEntry, updateEntry } = useEntries()
	const { admin, status: authStatus } = useAuth()

	if (authStatus === 'checking') {
		return <p className="notice">Loading…</p>
	}

	if (!admin) {
		return <NotFoundPage />
	}

	if (!id) {
		return <NoteForm heading="Plant a note" topics={groupByTopic(entries).map(([topic]) => topic)} onSave={createEntry} />
	}

	if (status !== 'ready') {
		return <LoadState />
	}

	const entry = entries.find((candidate) => candidate.id === id)
	if (!entry) {
		return <NotFoundPage />
	}

	return (
		<NoteForm
			key={entry.id}
			heading="Tend a note"
			initial={entry}
			topics={groupByTopic(entries).map(([topic]) => topic)}
			onSave={(payload) => updateEntry(entry.id, payload)}
		/>
	)
}

const BODY_PLACEHOLDER = `<style>
  h2 { color: var(--text-emphasis); }
</style>

<p>Start writing…</p>`

interface NoteFormProps {
	heading: string
	initial?: Entry
	topics: string[]
	onSave: (payload: EntryPayload) => Promise<Entry>
}

function NoteForm({ heading, initial, topics, onSave }: NoteFormProps) {
	const navigate = useNavigate()
	const [title, setTitle] = useState(initial?.title ?? '')
	const [topic, setTopic] = useState(initial?.topic ?? '')
	const [body, setBody] = useState(initial?.body ?? '')
	const [saving, setSaving] = useState(false)
	const [error, setError] = useState<string | null>(null)

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setSaving(true)
		setError(null)
		try {
			const saved = await onSave({ title: title.trim(), topic: topic.trim(), body })
			navigate(notePath(saved.id))
		} catch (e) {
			setError(errorMessageOf(e))
			setSaving(false)
		}
	}

	return (
		<section>
			<h2 className="section-heading">{heading}</h2>
			<form className="note-form" onSubmit={handleSubmit}>
				<div className="note-form-row">
					<label>
						<span>Title</span>
						<input value={title} onChange={(e) => setTitle(e.target.value)} required autoFocus />
					</label>

					<label>
						<span>Topic</span>
						<input value={topic} onChange={(e) => setTopic(e.target.value)} list="topic-options" required />
						<datalist id="topic-options">
							{topics.map((option) => (
								<option key={option} value={option} />
							))}
						</datalist>
					</label>
				</div>

				<div className="note-editor">
					<label>
						<span>Body (HTML)</span>
						<textarea
							className="code-input"
							value={body}
							onChange={(e) => setBody(e.target.value)}
							spellCheck={false}
							placeholder={BODY_PLACEHOLDER}
						/>
						<small className="field-hint">
							Any HTML works, including <code>&lt;style&gt;</code> blocks, which only apply to this note.
							Scripts and event handlers are removed. Text without tags is shown as plain paragraphs.
						</small>
					</label>

					<div className="note-preview">
						<span>Preview</span>
						<div className="note-preview-panel">
							{body.trim() ? <NoteBody body={body} /> : <p className="notice">Nothing to preview yet.</p>}
						</div>
					</div>
				</div>

				{error && <p className="notice notice-error">{error}</p>}

				<div className="note-actions">
					<button type="submit" className="pill-button" disabled={saving}>
						{saving ? 'Saving…' : 'Save note'}
					</button>
					<Link className="pill-button pill-button-quiet" to={initial ? notePath(initial.id) : '/'}>
						Cancel
					</Link>
				</div>
			</form>
		</section>
	)
}
