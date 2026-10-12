import { createContext, useContext } from 'react'
import type { AdminUser } from '../api'

export interface AuthContextValue {
	/** The logged-in admin, or null for a regular visitor. */
	admin: AdminUser | null
	/** 'checking' only while confirming a remembered session on page load. */
	status: 'checking' | 'ready'
	login: (username: string, password: string) => Promise<AdminUser>
	logout: () => Promise<void>
}

export const AuthContext = createContext<AuthContextValue | null>(null)

export function useAuth(): AuthContextValue {
	const value = useContext(AuthContext)
	if (!value) {
		throw new Error('useAuth must be used inside <AuthProvider>')
	}

	return value
}
