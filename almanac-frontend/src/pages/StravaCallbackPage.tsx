import { useEffect, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import * as api from '../api'
import { useAuth } from '../auth/AuthContext'
import { useItems } from '../state/ItemsContext'
import { NotFoundPage } from './NotFoundPage'

/**
 * /admin/strava/callback: where Strava sends the admin back after authorising. This page posts
 * the code to the API (a same-site request, so the SameSite=Strict session cookie goes with it),
 * runs a first sync, then returns to /admin.
 */
export function StravaCallbackPage() {
	const { admin, status } = useAuth()

	if (status === 'checking') {
		return <p className="notice">Checking your session…</p>
	}
	if (!admin) {
		return <NotFoundPage />
	}

	return <ConnectStrava />
}

function ConnectStrava() {
	const [params] = useSearchParams()
	const navigate = useNavigate()
	const { reload } = useItems()
	const [step, setStep] = useState('Connecting to Strava…')
	const [error, setError] = useState<string | null>(null)

	const code = params.get('code')
	const state = params.get('state')
	const scope = params.get('scope') ?? ''
	const denied = params.get('error')

	useEffect(() => {
		if (denied || !code || !state) {
			return
		}

		// Strava codes are single-use, so a StrictMode double-run must not post twice
		let started = false
		const timer = window.setTimeout(async () => {
			started = true
			try {
				await api.stravaCallback(code, state, scope)
				setStep('Connected. Importing your activities…')
				await api.syncConnection('strava').catch(() => undefined)
				await reload().catch(() => undefined)
				navigate('/admin', { replace: true })
			} catch (e) {
				setError(api.errorMessageOf(e))
			}
		}, 0)

		return () => {
			if (!started) {
				window.clearTimeout(timer)
			}
		}
	}, [code, state, scope, denied, navigate, reload])

	if (denied || !code || !state) {
		return (
			<section className="login">
				<h2 className="section-heading">Strava</h2>
				<p className="notice notice-error">
					{denied === 'access_denied'
						? 'Strava access was declined, so nothing was connected.'
						: "Strava didn't send back an authorisation code."}
				</p>
				<Link className="pill-button" to="/admin">
					Back to admin
				</Link>
			</section>
		)
	}

	return (
		<section className="login">
			<h2 className="section-heading">Strava</h2>
			{error ? (
				<>
					<p className="notice notice-error">{error}</p>
					<Link className="pill-button" to="/admin">
						Back to admin
					</Link>
				</>
			) : (
				<p className="notice">{step}</p>
			)}
		</section>
	)
}
