import { useRef, useState, type FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router'
import {
	errorMessageOf,
	type ActivityData,
	type FilmData,
	type Item,
	type ItemKind,
	type ItemPayload,
	type TmdbMovie,
	type Visibility,
} from '../api'
import { useAuth } from '../auth/AuthContext'
import { LoadState } from '../components/LoadState'
import { NoteBody } from '../components/NoteBody'
import { TmdbSearch } from '../components/TmdbSearch'
import {
	formatDuration,
	fromDateInput,
	groupByTopic,
	isActivity,
	isFilm,
	parseDuration,
	stars,
	toDateInput,
} from '../items'
import { itemPath, KINDS } from '../kinds'
import { tmdbMovieUrl } from '../site'
import { useItems } from '../state/ItemsContext'
import { NotFoundPage } from './NotFoundPage'

const RATINGS = [0.5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 4.5, 5]

/** Suggestions for the sport field; any Strava sport type works. */
const SPORTS = ['Run', 'TrailRun', 'Walk', 'Hike', 'Ride', 'MountainBikeRide', 'Swim', 'WeightTraining', 'Yoga', 'Workout']

/** /notes/new, /films/new and their /:id/edit pages. Visitors get the not-found page. */
export function ItemFormPage({ kind }: { kind: ItemKind }) {
	const { id } = useParams()
	const { items, status, createItem, updateItem } = useItems()
	const { admin, status: authStatus } = useAuth()

	if (authStatus === 'checking') {
		return <p className="notice">Loading…</p>
	}

	if (!admin) {
		return <NotFoundPage />
	}

	if (status !== 'ready') {
		return <LoadState />
	}

	const topics = groupByTopic(items).map(([topic]) => topic)

	if (!id) {
		return <ItemForm kind={kind} topics={topics} onSave={createItem} />
	}

	const item = items.find((candidate) => candidate.id === id && candidate.kind === kind)
	if (!item) {
		return <NotFoundPage />
	}

	return (
		<ItemForm
			key={item.id}
			kind={kind}
			initial={item}
			topics={topics}
			onSave={(payload) => updateItem(item.id, payload)}
		/>
	)
}

const BODY_PLACEHOLDER = `<style>
  h2 { color: var(--text-emphasis); }
</style>

<p>Start writing…</p>`

interface ItemFormProps {
	kind: ItemKind
	initial?: Item
	topics: string[]
	onSave: (payload: ItemPayload) => Promise<Item>
}

function ItemForm({ kind, initial, topics, onSave }: ItemFormProps) {
	const navigate = useNavigate()
	const film: Partial<FilmData> = initial && isFilm(initial) ? initial.data : {}
	const activity: Partial<ActivityData> = initial && isActivity(initial) ? initial.data : {}
	// Strava's API agreement only allows your Strava data to be shown to you
	const lockedPrivate = initial?.source === 'strava'
	const imported = initial && initial.source !== 'manual' ? initial.source : null

	const [title, setTitle] = useState(initial?.title ?? '')
	const [topic, setTopic] = useState(initial?.topic ?? '')
	const [body, setBody] = useState(initial?.body ?? '')
	const [visibility, setVisibility] = useState<Visibility>(initial?.visibility ?? 'public')
	const [watchedOn, setWatchedOn] = useState(toDateInput(initial?.occurred_at ?? new Date().toISOString()))
	const [details, setDetails] = useState<FilmDetails | null>(
		initial && isFilm(initial)
			? {
					title: initial.title,
					year: film.year,
					director: film.director,
					poster_url: film.poster_url,
					tmdb_id: film.tmdb_id,
				}
			: null,
	)
	const [changingFilm, setChangingFilm] = useState(false)
	const ratingRef = useRef<HTMLSelectElement>(null)
	const [rating, setRating] = useState(film.rating?.toString() ?? '')
	const [liked, setLiked] = useState(film.liked ?? false)
	const [rewatch, setRewatch] = useState(film.rewatch ?? false)
	const [sport, setSport] = useState(activity.sport ?? 'Run')
	const [distanceKm, setDistanceKm] = useState(activity.distance_m ? String(activity.distance_m / 1000) : '')
	const [movingTime, setMovingTime] = useState(activity.moving_time_s ? formatDuration(activity.moving_time_s) : '')
	const [elevation, setElevation] = useState(activity.elevation_gain_m?.toString() ?? '')
	const [heartRate, setHeartRate] = useState(activity.average_heartrate?.toString() ?? '')
	const [saving, setSaving] = useState(false)
	const [error, setError] = useState<string | null>(null)

	function payload(): ItemPayload {
		const common = { kind, title: title.trim(), topic: topic.trim() || null, body, visibility }
		if (kind === 'note') {
			return common
		}

		// Keep the exact time (e.g. an activity's start) unless the date itself was changed
		const occurred_at =
			initial && watchedOn === toDateInput(initial.occurred_at) ? initial.occurred_at : fromDateInput(watchedOn)

		// Empty optional fields are sent as undefined, which JSON.stringify drops
		if (kind === 'activity') {
			return {
				...common,
				occurred_at,
				data: {
					// Keep fields the form doesn't edit, e.g. strava_url and max heart rate
					...activity,
					sport: sport.trim(),
					distance_m: distanceKm ? Math.round(Number(distanceKm) * 1000 * 10) / 10 : undefined,
					moving_time_s: parseDuration(movingTime),
					elevation_gain_m: elevation ? Number(elevation) : undefined,
					average_heartrate: heartRate ? Number(heartRate) : undefined,
				},
			}
		}

		// The backend owns the film's details (from TMDB, or Letterboxd for imports); they're sent
		// as they are so the payload matches what will be saved
		return {
			...common,
			title: details?.title ?? '',
			occurred_at,
			data: {
				...film,
				year: details?.year,
				director: details?.director,
				poster_url: details?.poster_url,
				tmdb_id: details?.tmdb_id,
				rating: rating ? Number(rating) : undefined,
				liked,
				rewatch,
			},
		}
	}

	function pickFromTmdb(movie: TmdbMovie) {
		setDetails({
			title: movie.title,
			year: movie.year,
			director: movie.director,
			poster_url: movie.poster_url,
			tmdb_id: movie.tmdb_id,
		})
		setChangingFilm(false)
		// The search goes away once a film is picked; carry on with the rating
		ratingRef.current?.focus()
	}

	async function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		setSaving(true)
		setError(null)
		try {
			const saved = await onSave(payload())
			navigate(itemPath(saved))
		} catch (e) {
			setError(errorMessageOf(e))
			setSaving(false)
		}
	}

	const heading = initial ? `Tend ${KINDS[kind].label.toLowerCase()}` : KINDS[kind].create
	// Films logged here are picked from TMDB; imported ones keep their provider's details
	const needsFilm = kind === 'film' && !imported && !details?.tmdb_id
	const showSearch = kind === 'film' && !imported && (needsFilm || changingFilm)

	const topicField = (
		<label>
			<span>{kind === 'note' ? 'Topic' : 'Topic (optional)'}</span>
			<input value={topic} onChange={(e) => setTopic(e.target.value)} list="topic-options" required={kind === 'note'} />
			<datalist id="topic-options">
				{topics.map((option) => (
					<option key={option} value={option} />
				))}
			</datalist>
		</label>
	)

	return (
		<section>
			<h2 className="section-heading">{heading}</h2>
			{imported === 'strava' && (
				<p className="notice">
					Imported from Strava. Your title, topic and notes are kept, but the figures and start time are
					refreshed from Strava on every sync, and it stays private.
				</p>
			)}
			{imported === 'letterboxd' && (
				<p className="notice">
					Imported from Letterboxd, so the film's details come from there. Your topic and review edits are
					kept, but the rating, liked, rewatch and watched date are refreshed from Letterboxd on every sync.
				</p>
			)}
			{kind === 'film' && details && (
				<PickedFilm
					details={details}
					imported={imported}
					action={
						imported || needsFilm
							? undefined
							: changingFilm
								? { label: 'Keep this film', onClick: () => setChangingFilm(false) }
								: { label: 'Change film', onClick: () => setChangingFilm(true) }
					}
				/>
			)}
			{kind === 'film' && details && needsFilm && (
				<p className="notice">
					This film was logged before films came from TMDB. Pick it below to save changes: its title, year,
					director and poster will then come from TMDB.
				</p>
			)}
			{showSearch && <TmdbSearch onPick={pickFromTmdb} autoFocus />}
			<form className="note-form" onSubmit={handleSubmit}>
				{kind !== 'film' && (
					<div className="note-form-row">
						<label>
							<span>Title</span>
							<input value={title} onChange={(e) => setTitle(e.target.value)} required autoFocus />
						</label>
						{topicField}
					</div>
				)}

				{kind === 'activity' && (
					<>
						<div className="note-form-row note-form-row-4">
							<label>
								<span>Sport</span>
								<input value={sport} onChange={(e) => setSport(e.target.value)} list="sport-options" required />
								<datalist id="sport-options">
									{SPORTS.map((option) => (
										<option key={option} value={option} />
									))}
								</datalist>
							</label>
							<label>
								<span>Date</span>
								<input type="date" value={watchedOn} onChange={(e) => setWatchedOn(e.target.value)} required />
							</label>
							<label>
								<span>Distance (km)</span>
								<input
									type="number"
									min={0}
									// Any precision: Strava's distances (e.g. 14.6775 km) must survive an edit
									step="any"
									value={distanceKm}
									onChange={(e) => setDistanceKm(e.target.value)}
								/>
							</label>
							<label>
								<span>Moving time</span>
								<input
									value={movingTime}
									onChange={(e) => setMovingTime(e.target.value)}
									placeholder="h:mm:ss"
									pattern="[0-9]+(:[0-5]?[0-9]){0,2}"
									title="Minutes, or h:mm:ss"
								/>
							</label>
						</div>
						<div className="note-form-row note-form-row-4">
							<label>
								<span>Elevation gain (m)</span>
								<input type="number" min={0} value={elevation} onChange={(e) => setElevation(e.target.value)} />
							</label>
							<label>
								<span>Avg heart rate (bpm)</span>
								<input
									type="number"
									min={20}
									max={250}
									value={heartRate}
									onChange={(e) => setHeartRate(e.target.value)}
								/>
							</label>
						</div>
					</>
				)}

				{kind === 'film' && (
					<div className="note-form-row note-form-row-4">
						<label>
							<span>Watched on</span>
							<input type="date" value={watchedOn} onChange={(e) => setWatchedOn(e.target.value)} required />
						</label>
						<label>
							<span>Rating</span>
							<select ref={ratingRef} value={rating} onChange={(e) => setRating(e.target.value)}>
								<option value="">No rating</option>
								{RATINGS.map((value) => (
									<option key={value} value={value}>
										{stars(value)} ({value})
									</option>
								))}
							</select>
						</label>
						{topicField}
						<div className="checkbox-row">
							<label className="checkbox">
								<input type="checkbox" checked={liked} onChange={(e) => setLiked(e.target.checked)} />
								<span>Liked ♥</span>
							</label>
							<label className="checkbox">
								<input type="checkbox" checked={rewatch} onChange={(e) => setRewatch(e.target.checked)} />
								<span>Rewatch ↻</span>
							</label>
						</div>
					</div>
				)}

				<div className="note-editor">
					<label>
						<span>{kind === 'film' ? 'Review (HTML)' : kind === 'activity' ? 'Notes (HTML)' : 'Body (HTML)'}</span>
						<textarea
							className="code-input"
							value={body}
							onChange={(e) => setBody(e.target.value)}
							spellCheck={false}
							placeholder={BODY_PLACEHOLDER}
						/>
						<small className="field-hint">
							Any HTML works, including <code>&lt;style&gt;</code> blocks, which only apply to this page.
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

				<label className="visibility-field">
					<span>Visibility</span>
					<select
						value={visibility}
						onChange={(e) => setVisibility(e.target.value as Visibility)}
						disabled={lockedPrivate}
					>
						<option value="public">Public: anyone can see it</option>
						<option value="private">Private: only you, when logged in</option>
					</select>
					{lockedPrivate && (
						<small className="field-hint">
							Strava's API agreement only allows your Strava data to be shown to you, so this stays private.
						</small>
					)}
				</label>

				{error && <p className="notice notice-error">{error}</p>}

				<div className="note-actions">
					<button type="submit" className="pill-button" disabled={saving || needsFilm}>
						{saving ? 'Saving…' : `Save ${KINDS[kind].label.toLowerCase()}`}
					</button>
					<Link className="pill-button pill-button-quiet" to={initial ? itemPath(initial) : '/admin'}>
						Cancel
					</Link>
				</div>
			</form>
		</section>
	)
}

/** The film itself. It comes from TMDB (or from Letterboxd, for imports) and isn't edited here. */
interface FilmDetails {
	title: string
	year?: number
	director?: string
	poster_url?: string
	tmdb_id?: number
}

function PickedFilm({
	details,
	imported,
	action,
}: {
	details: FilmDetails
	imported: string | null
	action?: { label: string; onClick: () => void }
}) {
	return (
		<div className="picked-film">
			{details.poster_url ? (
				<img className="poster picked-film-poster" src={details.poster_url} alt="" referrerPolicy="no-referrer" />
			) : (
				<div className="poster picked-film-poster poster-placeholder" aria-hidden="true">
					<span>{details.title}</span>
				</div>
			)}
			<div className="picked-film-text">
				<h3 className="picked-film-title">{details.title}</h3>
				<p className="meta">{[details.year, details.director].filter(Boolean).join(' · ') || 'No year or director'}</p>
				<p className="field-hint">
					{imported === 'letterboxd' ? (
						'From Letterboxd'
					) : details.tmdb_id ? (
						<a href={tmdbMovieUrl(details.tmdb_id)} target="_blank" rel="noreferrer">
							On TMDB ↗
						</a>
					) : (
						'Not linked to TMDB yet'
					)}
				</p>
			</div>
			{action && (
				<button type="button" className="pill-button pill-button-quiet" onClick={action.onClick}>
					{action.label}
				</button>
			)}
		</div>
	)
}
