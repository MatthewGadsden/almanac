/// Outlines are scaled into a box this size (0..=ROUTE_OUTLINE_SIZE on both axes).
pub const ROUTE_OUTLINE_SIZE: u8 = 100;
/// More points than an outline needs; imported ones have at most 80.
const MAX_OUTLINE_POINTS: usize = 200;

/// The `data` of an activity item: a run, ride, swim, workout, ...
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivityData {
	/// Strava's sport type, e.g. `Run`, `TrailRun`, `Ride`, `Swim`, `WeightTraining`.
	pub sport: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub distance_m: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub moving_time_s: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub elapsed_time_s: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub elevation_gain_m: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub average_heartrate: Option<f64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub max_heartrate: Option<f64>,
	/// The activity on Strava, for imported activities.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub strava_url: Option<String>,
	/// The route's shape as `[x, y]` points in a `ROUTE_OUTLINE_SIZE` box, y downwards (as SVG).
	/// Just the shape: no coordinates are stored.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub route_outline: Option<Vec<[u8; 2]>>,
}

impl ActivityData {
	/// Parses and validates activity data from a request or an import.
	pub fn parse(data: serde_json::Value) -> Result<Self, String> {
		let mut activity: ActivityData =
			serde_json::from_value(data).map_err(|e| format!("invalid activity data: {e}"))?;

		activity.sport = activity.sport.trim().to_string();
		let valid_sport = (1..=40).contains(&activity.sport.len())
			&& activity
				.sport
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == ' ');
		if !valid_sport {
			return Err("sport must be a sport type such as Run or Ride".into());
		}

		let negative = |value: Option<f64>| value.is_some_and(|v| v < 0.0 || !v.is_finite());
		if negative(activity.distance_m) || negative(activity.elevation_gain_m) {
			return Err("distance and elevation can't be negative".into());
		}
		if activity.moving_time_s.is_some_and(|t| t < 0)
			|| activity.elapsed_time_s.is_some_and(|t| t < 0)
		{
			return Err("times can't be negative".into());
		}

		let implausible_heartrate =
			|value: Option<f64>| value.is_some_and(|bpm| !(20.0..=250.0).contains(&bpm));
		if implausible_heartrate(activity.average_heartrate)
			|| implausible_heartrate(activity.max_heartrate)
		{
			return Err("heart rate must be between 20 and 250 bpm".into());
		}

		activity.strava_url = activity.strava_url.filter(|url| !url.trim().is_empty());
		if activity
			.strava_url
			.as_deref()
			.is_some_and(|url| !url.starts_with("https://"))
		{
			return Err("strava_url must be an https URL".into());
		}

		if let Some(outline) = &activity.route_outline {
			let valid_outline = (2..=MAX_OUTLINE_POINTS).contains(&outline.len())
				&& outline.iter().flatten().all(|v| *v <= ROUTE_OUTLINE_SIZE);
			if !valid_outline {
				return Err(format!(
					"route_outline must be 2 to {MAX_OUTLINE_POINTS} points between 0 and {ROUTE_OUTLINE_SIZE}"
				));
			}
		}

		Ok(activity)
	}

	pub fn to_value(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("ActivityData always serialises")
	}
}
