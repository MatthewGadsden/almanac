import type { ActivityData } from '../api'
import { activityStats } from '../items'

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
