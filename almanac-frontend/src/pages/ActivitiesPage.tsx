import { Link } from 'react-router'
import type { ActivityItem } from '../api'
import { useAuth } from '../auth/AuthContext'
import { RouteOutline } from '../components/Activity'
import { LoadState } from '../components/LoadState'
import { PrivateBadge } from '../components/PrivateBadge'
import {
	byRecentlyOccurred,
	formatDistance,
	formatDuration,
	groupByMonth,
	isActivity,
	paceOrSpeed,
	sportLabel,
} from '../items'
import { itemPath, KINDS, newItemPath } from '../kinds'
import { useItems } from '../state/ItemsContext'

/** /activities: the training log, newest first, grouped by month, with this year's totals. */
export function ActivitiesPage() {
	const { items, status } = useItems()
	const { admin } = useAuth()

	if (status !== 'ready') {
		return <LoadState />
	}

	const activities = items.filter(isActivity).sort(byRecentlyOccurred)
	const thisYear = new Date().getFullYear()
	const yearActivities = activities.filter((a) => new Date(a.occurred_at).getFullYear() === thisYear)

	return (
		<section>
			<div className="section-header">
				<h2 className="section-heading">Training log</h2>
				{admin && (
					<Link className="pill-button" to={newItemPath('activity')}>
						{KINDS.activity.create}
					</Link>
				)}
			</div>

			{activities.length === 0 ? (
				<p className="notice">No activities logged yet.</p>
			) : (
				<>
					<YearTotals year={thisYear} activities={yearActivities} />

					{groupByMonth(activities).map(([month, monthActivities]) => (
						<div className="diary-month" key={month}>
							<h3 className="aside-heading">{month}</h3>
							<ol className="diary">
								{monthActivities.map((activity) => (
									<li key={activity.id}>
										<Link className="training-row" to={itemPath(activity)}>
											<span className="diary-day">{new Date(activity.occurred_at).getDate()}</span>
											<RouteOutline outline={activity.data.route_outline} className="training-route" placeholder />
											<span className="training-sport meta">{sportLabel(activity.data.sport)}</span>
											<span className="diary-film">{activity.title}</span>
											<span className="training-figure">
												{activity.data.distance_m ? formatDistance(activity.data.distance_m) : ''}
											</span>
											<span className="training-figure">
												{activity.data.moving_time_s ? formatDuration(activity.data.moving_time_s) : ''}
											</span>
											<span className="training-figure meta">{paceOrSpeed(activity.data) ?? ''}</span>
											<span className="diary-flags">
												<PrivateBadge item={activity} />
											</span>
										</Link>
									</li>
								))}
							</ol>
						</div>
					))}
				</>
			)}
		</section>
	)
}

/** This year so far, overall and per sport (sports by distance, then by count). */
function YearTotals({ year, activities }: { year: number; activities: ActivityItem[] }) {
	if (activities.length === 0) {
		return <p className="notice diary-stats">Nothing logged in {year} yet.</p>
	}

	const total = (list: ActivityItem[], field: 'distance_m' | 'moving_time_s') =>
		list.reduce((sum, activity) => sum + (activity.data[field] ?? 0), 0)

	const bySport = new Map<string, ActivityItem[]>()
	for (const activity of activities) {
		bySport.set(activity.data.sport, [...(bySport.get(activity.data.sport) ?? []), activity])
	}
	const sports = [...bySport.entries()].sort(
		([, a], [, b]) => total(b, 'distance_m') - total(a, 'distance_m') || b.length - a.length,
	)

	const distance = total(activities, 'distance_m')
	const time = total(activities, 'moving_time_s')

	return (
		<>
			<p className="notice diary-stats">
				{year} so far: {activities.length} {activities.length === 1 ? 'activity' : 'activities'}
				{distance > 0 && ` · ${formatDistance(distance)}`}
				{time > 0 && ` · ${formatDuration(time)} moving`}
			</p>
			<ul className="training-sports">
				{sports.map(([sport, list]) => {
					const sportDistance = total(list, 'distance_m')
					return (
						<li key={sport}>
							{sportLabel(sport)} · {list.length}
							{sportDistance > 0 && ` · ${formatDistance(sportDistance)}`}
						</li>
					)
				})}
			</ul>
		</>
	)
}
