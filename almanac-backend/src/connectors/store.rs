use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ImportedItem, Provider, SyncError, SyncSummary};
use crate::domain::Visibility;

/// A configured connection to a provider, as returned by the API.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Connection {
	pub provider: Provider,
	pub settings: serde_json::Value,
	pub import_visibility: Visibility,
	pub last_synced_at: Option<DateTime<Utc>>,
	pub last_sync_summary: Option<SyncSummary>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

struct ConnectionRow {
	provider: String,
	settings: serde_json::Value,
	import_visibility: String,
	last_synced_at: Option<DateTime<Utc>>,
	last_sync_summary: Option<serde_json::Value>,
	last_error: Option<String>,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}

impl TryFrom<ConnectionRow> for Connection {
	type Error = sqlx::Error;

	fn try_from(row: ConnectionRow) -> Result<Self, Self::Error> {
		let invalid = |what: String| sqlx::Error::Decode(what.into());

		Ok(Connection {
			provider: Provider::parse(&row.provider)
				.ok_or_else(|| invalid(format!("Unknown provider {:?}", row.provider)))?,
			settings: row.settings,
			import_visibility: Visibility::parse(&row.import_visibility).ok_or_else(|| {
				invalid(format!("Unknown visibility {:?}", row.import_visibility))
			})?,
			last_synced_at: row.last_synced_at,
			last_sync_summary: row
				.last_sync_summary
				.map(serde_json::from_value)
				.transpose()
				.map_err(|e| invalid(format!("Invalid sync summary: {e}")))?,
			last_error: row.last_error,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
	}
}

pub async fn list_connections(pool: &PgPool) -> Result<Vec<Connection>, sqlx::Error> {
	sqlx::query_as!(
		ConnectionRow,
		r#"
		SELECT provider, settings, import_visibility, last_synced_at, last_sync_summary,
			last_error, created_at, updated_at
		FROM connections
		ORDER BY provider
		"#
	)
	.fetch_all(pool)
	.await?
	.into_iter()
	.map(Connection::try_from)
	.collect()
}

pub async fn get_connection(
	pool: &PgPool,
	provider: Provider,
) -> Result<Option<Connection>, sqlx::Error> {
	sqlx::query_as!(
		ConnectionRow,
		r#"
		SELECT provider, settings, import_visibility, last_synced_at, last_sync_summary,
			last_error, created_at, updated_at
		FROM connections
		WHERE provider = $1
		"#,
		provider.as_str()
	)
	.fetch_optional(pool)
	.await?
	.map(Connection::try_from)
	.transpose()
}

/// Creates the connection, or replaces its settings. Sync history is kept on update.
pub async fn save_connection(
	pool: &PgPool,
	provider: Provider,
	settings: serde_json::Value,
	import_visibility: Visibility,
) -> Result<Connection, sqlx::Error> {
	let row = sqlx::query_as!(
		ConnectionRow,
		r#"
		INSERT INTO connections (provider, settings, import_visibility, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $4)
		ON CONFLICT (provider) DO UPDATE
		SET settings = EXCLUDED.settings,
			import_visibility = EXCLUDED.import_visibility,
			updated_at = EXCLUDED.updated_at
		RETURNING provider, settings, import_visibility, last_synced_at, last_sync_summary,
			last_error, created_at, updated_at
		"#,
		provider.as_str(),
		settings,
		import_visibility.as_str(),
		Utc::now()
	)
	.fetch_one(pool)
	.await?;

	Connection::try_from(row)
}

/// Removes the connection. Items it imported are kept. Returns false if there was none.
pub async fn delete_connection(pool: &PgPool, provider: Provider) -> Result<bool, sqlx::Error> {
	let result = sqlx::query!(
		"DELETE FROM connections WHERE provider = $1",
		provider.as_str()
	)
	.execute(pool)
	.await?;

	Ok(result.rows_affected() > 0)
}

/// Stores the outcome of a sync: the summary on success, the reason on failure.
pub async fn record_sync(
	pool: &PgPool,
	provider: Provider,
	result: &Result<SyncSummary, SyncError>,
) -> Result<(), sqlx::Error> {
	match result {
		Ok(summary) => {
			let summary = serde_json::to_value(summary).expect("SyncSummary always serialises");
			sqlx::query!(
				r#"
				UPDATE connections
				SET last_synced_at = $2, last_sync_summary = $3, last_error = NULL
				WHERE provider = $1
				"#,
				provider.as_str(),
				Utc::now(),
				summary
			)
			.execute(pool)
			.await?;
		}
		Err(error) => {
			sqlx::query!(
				"UPDATE connections SET last_error = $2 WHERE provider = $1",
				provider.as_str(),
				error.to_string()
			)
			.execute(pool)
			.await?;
		}
	}

	Ok(())
}

/// Saves imported items by (source, external_id).
///
/// New items are inserted with the given visibility. For items that already exist, only the
/// provider's facts are refreshed: `data` is merged (so fields set locally but not sent by the
/// provider survive) and `occurred_at` is replaced. Title, body, topic and visibility are never
/// touched after the first import, so anything tended in Almanac stays as it is. `updated_at`
/// isn't bumped either, so a sync doesn't make an item look "Tended".
pub async fn upsert_items(
	pool: &PgPool,
	source: &str,
	visibility: Visibility,
	items: Vec<ImportedItem>,
) -> Result<SyncSummary, sqlx::Error> {
	let mut summary = SyncSummary {
		fetched: items.len(),
		..SyncSummary::default()
	};
	let now = Utc::now();
	let mut transaction = pool.begin().await?;

	for item in items {
		// No row back means the item existed and nothing changed (the WHERE skipped the update)
		let row = sqlx::query!(
			r#"
			INSERT INTO items (id, kind, title, topic, body, data, occurred_at, visibility, source,
				external_id, created_at, updated_at)
			VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, $8, $9, $10, $10)
			ON CONFLICT (source, external_id) DO UPDATE
			SET data = items.data || EXCLUDED.data,
				occurred_at = EXCLUDED.occurred_at
			WHERE items.data IS DISTINCT FROM items.data || EXCLUDED.data
				OR items.occurred_at IS DISTINCT FROM EXCLUDED.occurred_at
			RETURNING (xmax = 0) AS "inserted!"
			"#,
			Uuid::new_v4(),
			item.kind.as_str(),
			item.title,
			item.body,
			item.data,
			item.occurred_at,
			visibility.as_str(),
			source,
			item.external_id,
			now
		)
		.fetch_optional(&mut *transaction)
		.await?;

		match row {
			Some(row) if row.inserted => summary.created += 1,
			Some(_) => summary.updated += 1,
			None => summary.unchanged += 1,
		}
	}

	transaction.commit().await?;
	Ok(summary)
}
