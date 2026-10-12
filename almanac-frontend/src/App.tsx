import './styling/index.css'
import { useEffect } from 'react'
import { Route, Routes, useLocation } from 'react-router'
import { GardenLayout } from './components/GardenLayout'
import { ReadingLayout } from './components/ReadingLayout'
import { AdminPage } from './pages/AdminPage'
import { HomePage } from './pages/HomePage'
import { NoteFormPage } from './pages/NoteFormPage'
import { NotePage } from './pages/NotePage'
import { NotFoundPage } from './pages/NotFoundPage'
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
					<Route path="/topics/:topic" element={<TopicPage />} />
				</Route>

				{/* Reading and writing: a single centred column with a slim header */}
				<Route element={<ReadingLayout />}>
					<Route path="/admin" element={<AdminPage />} />
					<Route path="/notes/new" element={<NoteFormPage />} />
					<Route path="/notes/:id" element={<NotePage />} />
					<Route path="/notes/:id/edit" element={<NoteFormPage />} />
					<Route path="*" element={<NotFoundPage />} />
				</Route>
			</Routes>
		</main>
	)
}

export default App
