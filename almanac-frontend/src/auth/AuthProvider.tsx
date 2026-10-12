import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import * as api from '../api'
import type { AdminUser } from '../api'
import { AuthContext, type AuthContextValue } from './AuthContext'

/*
 * Only browsers that have logged in before ask the backend who is logged in. Regular visitors
 * never call /admin/me, so nothing on the public pages points at the admin area. This flag is
 * only a hint about whether to check: the session cookie is what the backend actually trusts.
 */
const SESSION_HINT_KEY = 'almanac:admin-session'

function hasSessionHint(): boolean {
	try {
		return localStorage.getItem(SESSION_HINT_KEY) === '1'
	} catch {
		return false
	}
}

function setSessionHint(present: boolean) {
	try {
		if (present) {
			localStorage.setItem(SESSION_HINT_KEY, '1')
		} else {
			localStorage.removeItem(SESSION_HINT_KEY)
		}
	} catch {
		// Storage unavailable (private mode etc.); the admin just logs in again next visit
	}
}

export function AuthProvider({ children }: { children: ReactNode }) {
	const [admin, setAdmin] = useState<AdminUser | null>(null)
	const [status, setStatus] = useState<AuthContextValue['status']>(() =>
		hasSessionHint() ? 'checking' : 'ready',
	)

	useEffect(() => {
		if (!hasSessionHint()) {
			return
		}

		let cancelled = false
		api.currentAdmin().then(
			(current) => {
				if (!cancelled) {
					setAdmin(current)
					setSessionHint(current !== null)
					setStatus('ready')
				}
			},
			() => {
				// Backend unreachable: carry on as a visitor and check again on the next load
				if (!cancelled) {
					setStatus('ready')
				}
			},
		)

		return () => {
			cancelled = true
		}
	}, [])

	const login = useCallback(async (username: string, password: string) => {
		const loggedIn = await api.login(username, password)
		setSessionHint(true)
		setAdmin(loggedIn)
		return loggedIn
	}, [])

	const logout = useCallback(async () => {
		await api.logout()
		setSessionHint(false)
		setAdmin(null)
	}, [])

	const value = useMemo(() => ({ admin, status, login, logout }), [admin, status, login, logout])

	return <AuthContext value={value}>{children}</AuthContext>
}
