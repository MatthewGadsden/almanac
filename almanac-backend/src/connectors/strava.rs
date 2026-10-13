//! Strava: OAuth 2 (authorisation code flow) plus the athlete activities API.
//! See https://developers.strava.com/docs/authentication/ and /docs/reference/.
//!
//! Strava's API agreement (effective June 2026) only allows a user's Strava data to be shown to
//! that user, so everything imported from here is private (see `Provider::allows_public_imports`).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{Connection, ConnectorContext, Fetched, ImportedItem, Provider, SyncError};
use crate::domain::{ActivityData, ItemKind};

/// `read` for the athlete's name; `activity:read_all` for every activity, including ones set to
/// "Only You" on Strava (they're private in Almanac too).
pub const SCOPES: &str = "read,activity:read_all";

pub const NOT_CONFIGURED: &str = "Strava isn't set up: add your API application's client_id and \
	client_secret to connectors.strava (see the backend README)";

/// Activities per page; the API defaults to 30.
const PER_PAGE: usize = 100;
/// A cap per sync (10,000 activities) to stay well inside Strava's rate limits. Anything past
/// it is picked up by the next sync.
const MAX_PAGES: u32 = 100;
/// Each sync re-reads the week before the last activity it saw, so recently renamed or edited
/// activities are refreshed.
const OVERLAP_SECONDS: i64 = 7 * 24 * 60 * 60;
/// Refresh the access token if it expires within this many seconds.
const REFRESH_MARGIN_SECONDS: i64 = 10 * 60;

/// Strava's OAuth tokens, stored in `connections.credentials`. Refresh tokens rotate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StravaCredentials {
	pub access_token: String,
	pub refresh_token: String,
	/// Unix seconds.
	pub expires_at: i64,
}

/// Who authorised, stored as the connection's settings (shown in the admin).
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StravaAthlete {
	pub athlete_id: i64,
	pub name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SyncCursor {
	/// Start time (Unix seconds) of the newest activity seen so far.
	after: i64,
}

fn to_json<T: serde::Serialize>(value: &T) -> serde_json::Value {
	serde_json::to_value(value).expect("Strava types always serialise")
}

impl StravaCredentials {
	pub fn to_value(&self) -> serde_json::Value {
		to_json(self)
	}
}

impl StravaAthlete {
	pub fn to_value(&self) -> serde_json::Value {
		to_json(self)
	}
}

/// Where to send the admin to authorise Almanac on Strava.
pub fn authorize_url(context: &ConnectorContext, state: &str) -> Result<String, String> {
	let config = &context.settings.strava;
	let base = format!("{}/authorize", config.oauth_base_url.trim_end_matches('/'));

	reqwest::Url::parse_with_params(
		&base,
		&[
			("client_id", config.client_id.as_str()),
			("redirect_uri", config.redirect_uri.as_str()),
			("response_type", "code"),
			("approval_prompt", "auto"),
			("scope", SCOPES),
			("state", state),
		],
	)
	.map(String::from)
	.map_err(|e| format!("Invalid Strava oauth_base_url: {e}"))
}

/// Whether the scopes the athlete actually granted (they can untick some) cover activities.
pub fn can_read_activities(granted: &str) -> bool {
	granted
		.split([',', ' '])
		.any(|scope| scope == "activity:read" || scope == "activity:read_all")
}

#[derive(serde::Deserialize)]
struct TokenResponse {
	access_token: String,
	refresh_token: String,
	expires_at: i64,
	#[serde(default)]
	athlete: Option<TokenAthlete>,
}

#[derive(serde::Deserialize)]
struct TokenAthlete {
	id: i64,
	#[serde(default)]
	firstname: Option<String>,
	#[serde(default)]
	lastname: Option<String>,
	#[serde(default)]
	username: Option<String>,
}

/// Exchanges the code from Strava's redirect for tokens, and says who authorised.
pub async fn exchange_code(
	context: &ConnectorContext,
	code: &str,
) -> Result<(StravaCredentials, StravaAthlete), SyncError> {
	let config = &context.settings.strava;
	let response = token_request(
		context,
		&[
			("client_id", config.client_id.as_str()),
			("client_secret", config.client_secret.as_str()),
			("code", code),
			("grant_type", "authorization_code"),
		],
	)
	.await?;

	let athlete = response
		.athlete
		.ok_or_else(|| SyncError::Invalid("Strava didn't say which athlete authorised".into()))?;
	let full_name = [athlete.firstname, athlete.lastname]
		.into_iter()
		.flatten()
		.map(|part| part.trim().to_string())
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>()
		.join(" ");
	let name = if full_name.is_empty() {
		athlete
			.username
			.unwrap_or_else(|| format!("Athlete {}", athlete.id))
	} else {
		full_name
	};

	Ok((
		StravaCredentials {
			access_token: response.access_token,
			refresh_token: response.refresh_token,
			expires_at: response.expires_at,
		},
		StravaAthlete {
			athlete_id: athlete.id,
			name,
		},
	))
}

async fn token_request(
	context: &ConnectorContext,
	form: &[(&str, &str)],
) -> Result<TokenResponse, SyncError> {
	let url = format!(
		"{}/token",
		context.settings.strava.oauth_base_url.trim_end_matches('/')
	);
	let response = context
		.http
		.post(&url)
		.form(form)
		.send()
		.await
		.map_err(|e| SyncError::Fetch(format!("Couldn't reach Strava: {e}")))?;

	let status = response.status();
	if status == reqwest::StatusCode::BAD_REQUEST || status == reqwest::StatusCode::UNAUTHORIZED {
		return Err(SyncError::Fetch(
			"Strava didn't accept Almanac's authorisation; connect Strava again".into(),
		));
	}
	if !status.is_success() {
		return Err(SyncError::Fetch(format!("Strava responded with {status}")));
	}

	response
		.json()
		.await
		.map_err(|e| SyncError::Invalid(format!("Unexpected response from Strava: {e}")))
}

fn stored_credentials(connection: &Connection) -> Result<StravaCredentials, SyncError> {
	connection
		.credentials
		.clone()
		.map(serde_json::from_value)
		.transpose()
		.map_err(|e| SyncError::Invalid(format!("Stored Strava tokens are unreadable: {e}")))?
		.ok_or_else(|| SyncError::Invalid("Strava isn't authorised; connect Strava again".into()))
}

/// A usable access token. If it's about to expire it's refreshed first, and the new tokens are
/// saved straight away because Strava invalidates the old refresh token immediately.
async fn access_token(
	connection: &Connection,
	pool: &PgPool,
	context: &ConnectorContext,
) -> Result<String, SyncError> {
	let credentials = stored_credentials(connection)?;
	if credentials.expires_at > Utc::now().timestamp() + REFRESH_MARGIN_SECONDS {
		return Ok(credentials.access_token);
	}

	let config = &context.settings.strava;
	let refreshed = token_request(
		context,
		&[
			("client_id", config.client_id.as_str()),
			("client_secret", config.client_secret.as_str()),
			("grant_type", "refresh_token"),
			("refresh_token", credentials.refresh_token.as_str()),
		],
	)
	.await?;

	let fresh = StravaCredentials {
		access_token: refreshed.access_token,
		refresh_token: refreshed.refresh_token,
		expires_at: refreshed.expires_at,
	};
	super::save_credentials(pool, Provider::Strava, fresh.to_value()).await?;

	Ok(fresh.access_token)
}

/// An activity as listed by `GET /athlete/activities`. Unused fields are ignored.
#[derive(serde::Deserialize)]
struct SummaryActivity {
	id: i64,
	name: String,
	#[serde(default)]
	sport_type: Option<String>,
	/// The older, coarser activity type; used if `sport_type` is missing.
	#[serde(rename = "type", default)]
	activity_type: Option<String>,
	start_date: DateTime<Utc>,
	#[serde(default)]
	distance: Option<f64>,
	#[serde(default)]
	moving_time: Option<i64>,
	#[serde(default)]
	elapsed_time: Option<i64>,
	#[serde(default)]
	total_elevation_gain: Option<f64>,
	#[serde(default)]
	average_heartrate: Option<f64>,
	#[serde(default)]
	max_heartrate: Option<f64>,
}

impl SummaryActivity {
	fn into_imported(self) -> Result<ImportedItem, String> {
		let positive = |value: Option<f64>| value.filter(|v| *v > 0.0);
		let activity = ActivityData {
			sport: self
				.sport_type
				.or(self.activity_type)
				.unwrap_or_else(|| "Workout".into()),
			// A weights session reports 0 m; leave it out rather than showing "0.0 km"
			distance_m: positive(self.distance),
			moving_time_s: self.moving_time,
			elapsed_time_s: self.elapsed_time,
			elevation_gain_m: positive(self.total_elevation_gain),
			average_heartrate: self.average_heartrate,
			max_heartrate: self.max_heartrate,
			strava_url: Some(format!("https://www.strava.com/activities/{}", self.id)),
		};
		// Imported data has to pass the same checks as data entered by hand
		let data = ActivityData::parse(activity.to_value())?.to_value();

		let title = self.name.trim();
		Ok(ImportedItem {
			kind: ItemKind::Activity,
			external_id: self.id.to_string(),
			title: if title.is_empty() {
				"Untitled activity".into()
			} else {
				title.into()
			},
			body: String::new(),
			data,
			occurred_at: self.start_date,
		})
	}
}

/// Fetches activities since the last sync (all of them the first time).
pub(super) async fn fetch(
	connection: &Connection,
	pool: &PgPool,
	context: &ConnectorContext,
) -> Result<Fetched, SyncError> {
	if !context.settings.strava.is_configured() {
		return Err(SyncError::Invalid(NOT_CONFIGURED.into()));
	}

	let token = access_token(connection, pool, context).await?;
	let previous = connection
		.sync_cursor
		.clone()
		.and_then(|cursor| serde_json::from_value::<SyncCursor>(cursor).ok())
		.map(|cursor| cursor.after);
	let after = previous.map_or(0, |after| (after - OVERLAP_SECONDS).max(0));

	let url = format!(
		"{}/athlete/activities",
		context.settings.strava.api_base_url.trim_end_matches('/')
	);
	let mut items = Vec::new();
	let mut skipped = 0;
	let mut newest = previous;

	for page in 1..=MAX_PAGES {
		let response = context
			.http
			.get(&url)
			.bearer_auth(&token)
			.query(&[
				("after", after.to_string()),
				("page", page.to_string()),
				("per_page", PER_PAGE.to_string()),
			])
			.send()
			.await
			.map_err(|e| SyncError::Fetch(format!("Couldn't reach Strava: {e}")))?;

		match response.status() {
			reqwest::StatusCode::UNAUTHORIZED => {
				return Err(SyncError::Fetch(
					"Strava no longer accepts Almanac's authorisation; connect Strava again".into(),
				));
			}
			reqwest::StatusCode::TOO_MANY_REQUESTS => {
				return Err(SyncError::Fetch(
					"Strava's rate limit was reached; try again in 15 minutes".into(),
				));
			}
			status if !status.is_success() => {
				return Err(SyncError::Fetch(format!("Strava responded with {status}")));
			}
			_ => {}
		}

		let activities: Vec<SummaryActivity> = response
			.json()
			.await
			.map_err(|e| SyncError::Invalid(format!("Unexpected response from Strava: {e}")))?;
		let last_page = activities.len() < PER_PAGE;

		for activity in activities {
			let started = activity.start_date.timestamp();
			newest = Some(newest.map_or(started, |seen| seen.max(started)));

			match activity.into_imported() {
				Ok(item) => items.push(item),
				Err(e) => {
					eprintln!("Skipping a Strava activity: {e}");
					skipped += 1;
				}
			}
		}

		if last_page {
			break;
		}
	}

	Ok(Fetched {
		items,
		skipped,
		cursor: newest.map(|after| to_json(&SyncCursor { after })),
	})
}

/// Revokes Almanac's access at Strava (used when disconnecting). Revoking the refresh token also
/// revokes its access tokens. Strava answers 200 even if the token was already gone.
pub async fn revoke(connection: &Connection, context: &ConnectorContext) -> Result<(), SyncError> {
	let config = &context.settings.strava;
	let credentials = stored_credentials(connection)?;
	let url = format!("{}/revoke", config.oauth_base_url.trim_end_matches('/'));

	let response = context
		.http
		.post(&url)
		.basic_auth(&config.client_id, Some(&config.client_secret))
		.form(&[
			("token", credentials.refresh_token.as_str()),
			("token_type_hint", "refresh_token"),
		])
		.send()
		.await
		.map_err(|e| SyncError::Fetch(format!("Couldn't reach Strava: {e}")))?;

	if !response.status().is_success() {
		return Err(SyncError::Fetch(format!(
			"Strava responded with {}",
			response.status()
		)));
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reads_granted_scopes() {
		assert!(can_read_activities("read,activity:read_all"));
		assert!(can_read_activities("read activity:read"));
		assert!(!can_read_activities("read"));
		assert!(!can_read_activities(""));
	}
}
