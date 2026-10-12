import { Outlet } from 'react-router'
import { Sidebar } from './Sidebar'

export function GardenLayout() {
	return (
		<div className="garden">
			<div className="column-backdrop" aria-hidden="true" />
			<Sidebar />
			<div className="content">
				<Outlet />
			</div>
		</div>
	)
}
