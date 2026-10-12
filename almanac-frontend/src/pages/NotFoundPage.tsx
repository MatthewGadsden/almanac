import { Link } from 'react-router'

export function NotFoundPage() {
	return (
		<section>
			<h2 className="section-heading">Nothing grows here</h2>
			<p className="notice">This note or topic doesn't exist, or it has been pruned.</p>
			<Link className="pill-button" to="/">
				Back to the garden
			</Link>
		</section>
	)
}
