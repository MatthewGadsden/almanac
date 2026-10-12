import { Link, useParams } from 'react-router'
import { ItemCard } from '../components/ItemCard'
import { LoadState } from '../components/LoadState'
import { byTitle } from '../items'
import { useItems } from '../state/ItemsContext'
import { NotFoundPage } from './NotFoundPage'

export function TopicPage() {
	const { topic } = useParams()
	const { items, status } = useItems()

	if (status !== 'ready') {
		return <LoadState />
	}

	const inTopic = items.filter((item) => item.topic === topic).sort(byTitle)
	if (!topic || inTopic.length === 0) {
		return <NotFoundPage />
	}

	return (
		<section>
			<nav className="breadcrumbs" aria-label="Breadcrumbs">
				<Link to="/">Home</Link>
				<span aria-hidden="true">/</span>
				<span>{topic}</span>
			</nav>
			<h2 className="page-title">{topic}</h2>
			<p className="notice">
				{inTopic.length} {inTopic.length === 1 ? 'item' : 'items'} in this topic.
			</p>
			<div className="note-grid">
				{inTopic.map((item) => (
					<ItemCard key={item.id} item={item} showTopic={false} />
				))}
			</div>
		</section>
	)
}
