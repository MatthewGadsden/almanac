import './styling/index.css'
import { useEffect } from 'react'
import { Route, Routes, useLocation } from 'react-router'
import { GardenLayout } from './components/GardenLayout'
import { ReadingLayout } from './components/ReadingLayout'
import { ActivitiesPage } from './pages/ActivitiesPage'
import { AdminPage } from './pages/AdminPage'
import { HomePage } from './pages/HomePage'
import { FilmsPage } from './pages/FilmsPage'
import { ItemFormPage } from './pages/ItemFormPage'
import { ItemPage } from './pages/ItemPage'
import { NotFoundPage } from './pages/NotFoundPage'
import { StravaCallbackPage } from './pages/StravaCallbackPage'
import { TopicPage } from './pages/TopicPage'

function App() {
	const { pathname } = useLocation()

	// Each route is its own page, so start it at the top rather than keeping the previous scroll
	useEffect(() => {
		window.scrollTo(0, 0)
	}, [pathname])

	return (
		<main className="page">
			<div className="page-backdrop" aria-hidden="true" />
			<Routes>
				{/* Browsing: sidebar with the topic index beside a wide content area */}
				<Route element={<GardenLayout />}>
					<Route path="/" element={<HomePage />} />
					<Route path="/films" element={<FilmsPage />} />
					<Route path="/activities" element={<ActivitiesPage />} />
					<Route path="/topics/:topic" element={<TopicPage />} />
				</Route>

				{/* Reading and writing: a single centred column with a slim header */}
				<Route element={<ReadingLayout />}>
					<Route path="/admin" element={<AdminPage />} />
					<Route path="/admin/strava/callback" element={<StravaCallbackPage />} />
					<Route path="/notes/new" element={<ItemFormPage kind="note" />} />
					<Route path="/notes/:id" element={<ItemPage kind="note" />} />
					<Route path="/notes/:id/edit" element={<ItemFormPage kind="note" />} />
					<Route path="/films/new" element={<ItemFormPage kind="film" />} />
					<Route path="/films/:id" element={<ItemPage kind="film" />} />
					<Route path="/films/:id/edit" element={<ItemFormPage kind="film" />} />
					<Route path="/activities/new" element={<ItemFormPage kind="activity" />} />
					<Route path="/activities/:id" element={<ItemPage kind="activity" />} />
					<Route path="/activities/:id/edit" element={<ItemFormPage kind="activity" />} />
					<Route path="*" element={<NotFoundPage />} />
				</Route>
			</Routes>
		</main>
	)
}

export default App
