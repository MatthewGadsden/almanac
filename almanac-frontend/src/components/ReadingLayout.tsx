import { Link, NavLink, Outlet } from 'react-router'
import { useAuth } from '../auth/AuthContext'
import { SLOTH_URL } from '../site'

export function ReadingLayout() {
	const { admin } = useAuth()

	return (
		<div className="reading">
			<header className="reading-header">
				<Link className="brand" to="/">
					<img className="brand-sloth" src={SLOTH_URL} alt="" />
					<span>Almanac</span>
				</Link>
				<nav className="reading-nav" aria-label="Pages">
					<NavLink to="/" end>
						Index
					</NavLink>
					<NavLink to="/films" end>
						Films
					</NavLink>
					{admin && <NavLink to="/admin">Admin</NavLink>}
				</nav>
			</header>

			<div className="reading-column">
				<Outlet />
			</div>
		</div>
	)
}
