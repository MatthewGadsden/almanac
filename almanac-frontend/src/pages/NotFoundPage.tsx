import { Link } from 'react-router'

export function NotFoundPage() {
	return (
		<section>
			<h2 className="section-heading">Nothing grows here</h2>
			<p className="notice">There's nothing at this address, or it has been pruned.</p>
			<Link className="pill-button" to="/">
				Back to the garden
			</Link>
		</section>
	)
}
