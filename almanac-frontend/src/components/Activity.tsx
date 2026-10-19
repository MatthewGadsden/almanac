import type { ActivityData } from '../api'
import { activityStats } from '../items'

/**
 * The route's shape as a bare line, with a dot where it started. Without an outline it renders
 * nothing, or an empty box when `placeholder` is set (to keep rows aligned).
 */
export function RouteOutline({
	outline,
	className,
	placeholder = false,
}: {
	outline?: [number, number][]
	className?: string
	placeholder?: boolean
}) {
	if (!outline || outline.length < 2) {
		return placeholder ? <svg className={`route-outline ${className ?? ''}`} aria-hidden="true" /> : null
	}

	const [startX, startY] = outline[0]
	return (
		<svg className={`route-outline ${className ?? ''}`} viewBox="-6 -6 112 112" aria-hidden="true">
			<polyline points={outline.map(([x, y]) => `${x},${y}`).join(' ')} />
			{/* A zero-length round-capped line: a dot that stays the same size at any scale */}
			<line className="route-start" x1={startX} y1={startY} x2={startX} y2={startY} />
		</svg>
	)
}

/** Stats as a row of labelled figures, e.g. on an activity's page. */
export function ActivityStats({ activity, size = 'large' }: { activity: ActivityData; size?: 'small' | 'large' }) {
	const stats = activityStats(activity)
	if (stats.length === 0) {
		return null
	}

	return (
		<dl className={`activity-stats activity-stats-${size}`}>
			{stats.map(([label, value]) => (
				<div key={label}>
					<dt>{label}</dt>
					<dd>{value}</dd>
				</div>
			))}
		</dl>
	)
}
