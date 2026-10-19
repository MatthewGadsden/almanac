//! Connectors pull items in from other services. Each provider fetches its data and turns it
//! into `ImportedItem`s; `store::upsert_items` then saves them by (source, external_id), so
//! syncing again updates them instead of creating duplicates.

mod letterboxd;
mod route;
mod store;
pub mod strava;

pub use letterboxd::{LetterboxdEntry, LetterboxdSettings, parse_feed};
pub use store::*;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::time::Duration;

use crate::configuration::ConnectorSettings;
use crate::domain::{ItemKind, Visibility};
use crate::error::ApiError;

/// Services items can be imported from. Also used as the items' `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
	Letterboxd,
	Strava,
}

impl Provider {
	pub fn as_str(self) -> &'static str {
		match self {
			Provider::Letterboxd => "letterboxd",
			Provider::Strava => "strava",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"letterboxd" => Some(Provider::Letterboxd),
			"strava" => Some(Provider::Strava),
			_ => None,
		}
	}

	/// Checks and normalises settings entered for this provider. OAuth providers get their
	/// settings from the authorisation flow instead, so they reject this.
	pub fn validate_settings(
		self,
		settings: serde_json::Value,
	) -> Result<serde_json::Value, String> {
		match self {
			Provider::Letterboxd => LetterboxdSettings::parse(settings).map(|s| s.to_value()),
			Provider::Strava => Err("Strava is connected by authorising Almanac on Strava".into()),
		}
	}

	/// Whether imported items may be public. Strava's API agreement only lets a user's Strava
	/// data be shown to that user, so Strava imports are always private.
	pub fn allows_public_imports(self) -> bool {
		match self {
			Provider::Letterboxd => true,
			Provider::Strava => false,
		}
	}
}

/// An item fetched from a provider, ready to be upserted.
#[derive(Debug)]
pub struct ImportedItem {
	pub kind: ItemKind,
	pub external_id: String,
	pub title: String,
	pub body: String,
	pub data: serde_json::Value,
	pub occurred_at: DateTime<Utc>,
}

/// What a provider's fetch returned.
#[derive(Debug, Default)]
pub struct Fetched {
	pub items: Vec<ImportedItem>,
	/// Entries that aren't items, e.g. Letterboxd lists.
	pub skipped: usize,
	/// Saved after the items are, so a failed save means the next sync retries them.
	pub cursor: Option<serde_json::Value>,
}

/// What a sync did. Stored on the connection and returned by the sync endpoint.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncSummary {
	/// Usable entries in the provider's response.
	pub fetched: usize,
	pub created: usize,
	pub updated: usize,
	pub unchanged: usize,
	/// Entries that aren't items, e.g. Letterboxd lists.
	pub skipped: usize,
}

/// The HTTP client and settings that syncing needs, shared by the API and the scheduler.
#[derive(Clone)]
pub struct ConnectorContext {
	pub http: reqwest::Client,
	pub settings: ConnectorSettings,
}

impl ConnectorContext {
	pub fn new(settings: ConnectorSettings) -> Self {
		let http = reqwest::Client::builder()
			.user_agent(concat!(
				"almanac/",
				env!("CARGO_PKG_VERSION"),
				" (+https://github.com/MatthewGadsden/almanac)"
			))
			.timeout(Duration::from_secs(20))
			.build()
			.expect("Failed to build the HTTP client.");

		ConnectorContext { http, settings }
	}
}

#[derive(Debug)]
pub enum SyncError {
	NotConnected,
	/// The provider couldn't be reached or refused the request.
	Fetch(String),
	/// The provider's response or the connection's settings couldn't be used.
	Invalid(String),
	Database(sqlx::Error),
}

impl std::fmt::Display for SyncError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SyncError::NotConnected => write!(f, "Not connected"),
			SyncError::Fetch(message) | SyncError::Invalid(message) => write!(f, "{message}"),
			SyncError::Database(e) => write!(f, "Database error: {e}"),
		}
	}
}

impl From<sqlx::Error> for SyncError {
	fn from(e: sqlx::Error) -> Self {
		SyncError::Database(e)
	}
}

impl From<SyncError> for ApiError {
	fn from(e: SyncError) -> Self {
		match e {
			SyncError::NotConnected => ApiError::NotFound,
			SyncError::Fetch(message) | SyncError::Invalid(message) => ApiError::Upstream(message),
			SyncError::Database(e) => ApiError::Database(e),
		}
	}
}

/// Fetches from the provider, saves the items, and records the outcome on the connection.
pub async fn sync(
	provider: Provider,
	pool: &PgPool,
	context: &ConnectorContext,
) -> Result<SyncSummary, SyncError> {
	let connection = get_connection(pool, provider)
		.await?
		.ok_or(SyncError::NotConnected)?;

	let result = fetch_and_save(&connection, pool, context).await;
	record_sync(pool, provider, &result).await?;
	result
}

async fn fetch_and_save(
	connection: &Connection,
	pool: &PgPool,
	context: &ConnectorContext,
) -> Result<SyncSummary, SyncError> {
	let fetched = match connection.provider {
		Provider::Letterboxd => letterboxd::fetch(connection, context).await?,
		Provider::Strava => strava::fetch(connection, pool, context).await?,
	};

	let visibility = if connection.provider.allows_public_imports() {
		connection.import_visibility
	} else {
		Visibility::Private
	};
	let mut summary = upsert_items(
		pool,
		connection.provider.as_str(),
		visibility,
		fetched.items,
	)
	.await?;
	summary.skipped = fetched.skipped;

	if let Some(cursor) = fetched.cursor {
		save_sync_cursor(pool, connection.provider, cursor).await?;
	}

	Ok(summary)
}

/// Syncs every connection straight away, then every `sync_interval_minutes`, until the process
/// exits. Failures are logged and recorded on the connection; they don't stop the loop.
pub async fn run_scheduler(pool: PgPool, context: ConnectorContext) {
	let minutes = context.settings.sync_interval_minutes;
	if minutes == 0 {
		return;
	}

	let mut interval = tokio::time::interval(Duration::from_secs(minutes * 60));
	loop {
		interval.tick().await;

		let connections = match list_connections(&pool).await {
			Ok(connections) => connections,
			Err(e) => {
				eprintln!("Scheduled sync couldn't list connections: {e}");
				continue;
			}
		};

		for connection in connections {
			if let Err(e) = sync(connection.provider, &pool, &context).await {
				eprintln!(
					"Scheduled {} sync failed: {e}",
					connection.provider.as_str()
				);
			}
		}
	}
}
