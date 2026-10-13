use actix_web::{HttpResponse, web};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::connectors::Provider;
use crate::domain::{Item, ItemKind, ItemPayload, Visibility};
use crate::error::ApiError;

/// An `items` row as stored; `kind` and `visibility` are checked when converting to `Item`.
struct ItemRow {
	id: Uuid,
	kind: String,
	title: String,
	topic: Option<String>,
	body: String,
	data: serde_json::Value,
	occurred_at: DateTime<Utc>,
	visibility: String,
	source: String,
	external_id: Option<String>,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}

impl TryFrom<ItemRow> for Item {
	type Error = ApiError;

	fn try_from(row: ItemRow) -> Result<Self, Self::Error> {
		let kind = ItemKind::parse(&row.kind)
			.ok_or_else(|| ApiError::Unexpected(format!("Unknown item kind {:?}", row.kind)))?;
		let visibility = Visibility::parse(&row.visibility).ok_or_else(|| {
			ApiError::Unexpected(format!("Unknown visibility {:?}", row.visibility))
		})?;

		Ok(Item {
			id: row.id,
			kind,
			title: row.title,
			topic: row.topic,
			body: row.body,
			data: row.data,
			occurred_at: row.occurred_at,
			visibility,
			source: row.source,
			external_id: row.external_id,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})
	}
}

/// Every item a visitor may see, plus private ones for the admin. Newest first by `occurred_at`.
pub async fn list_items(
	admin: Option<AdminUser>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let rows = sqlx::query_as!(
		ItemRow,
		r#"
		SELECT id, kind, title, topic, body, data, occurred_at, visibility, source, external_id,
			created_at, updated_at
		FROM items
		WHERE visibility = 'public' OR $1
		ORDER BY occurred_at DESC
		"#,
		admin.is_some()
	)
	.fetch_all(pool.get_ref())
	.await?;

	let items = rows
		.into_iter()
		.map(Item::try_from)
		.collect::<Result<Vec<_>, _>>()?;

	Ok(HttpResponse::Ok().json(items))
}

pub async fn create_item(
	_admin: AdminUser,
	payload: web::Json<ItemPayload>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let item = payload
		.into_inner()
		.validate()
		.map_err(ApiError::Validation)?;
	let now = Utc::now();

	let row = sqlx::query_as!(
		ItemRow,
		r#"
		INSERT INTO items (id, kind, title, topic, body, data, occurred_at, visibility, source,
			created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'manual', $9, $9)
		RETURNING id, kind, title, topic, body, data, occurred_at, visibility, source,
			external_id, created_at, updated_at
		"#,
		Uuid::new_v4(),
		item.kind.as_str(),
		item.title,
		item.topic,
		item.body,
		item.data,
		item.occurred_at.unwrap_or(now),
		item.visibility.unwrap_or(Visibility::Public).as_str(),
		now
	)
	.fetch_one(pool.get_ref())
	.await?;

	Ok(HttpResponse::Created().json(Item::try_from(row)?))
}

/// Private items look exactly like missing ones (404) to visitors.
pub async fn get_item(
	admin: Option<AdminUser>,
	id: web::Path<Uuid>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let row = sqlx::query_as!(
		ItemRow,
		r#"
		SELECT id, kind, title, topic, body, data, occurred_at, visibility, source, external_id,
			created_at, updated_at
		FROM items
		WHERE id = $1 AND (visibility = 'public' OR $2)
		"#,
		id.into_inner(),
		admin.is_some()
	)
	.fetch_optional(pool.get_ref())
	.await?
	.ok_or(ApiError::NotFound)?;

	Ok(HttpResponse::Ok().json(Item::try_from(row)?))
}

pub async fn update_item(
	_admin: AdminUser,
	id: web::Path<Uuid>,
	payload: web::Json<ItemPayload>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let item = payload
		.into_inner()
		.validate()
		.map_err(ApiError::Validation)?;

	if item.visibility == Some(Visibility::Public) {
		let source = sqlx::query_scalar!("SELECT source FROM items WHERE id = $1", id)
			.fetch_optional(pool.get_ref())
			.await?;
		if source
			.as_deref()
			.and_then(Provider::parse)
			.is_some_and(|provider| !provider.allows_public_imports())
		{
			return Err(ApiError::Validation(
				"Strava's API agreement only allows your Strava data to be shown to you, so \
				activities imported from Strava stay private"
					.into(),
			));
		}
	}

	let row = sqlx::query_as!(
		ItemRow,
		r#"
		UPDATE items
		SET kind = $2, title = $3, topic = $4, body = $5, data = $6,
			occurred_at = COALESCE($7, occurred_at),
			visibility = COALESCE($8, visibility),
			updated_at = $9
		WHERE id = $1
		RETURNING id, kind, title, topic, body, data, occurred_at, visibility, source,
			external_id, created_at, updated_at
		"#,
		id,
		item.kind.as_str(),
		item.title,
		item.topic,
		item.body,
		item.data,
		item.occurred_at,
		item.visibility.map(Visibility::as_str),
		Utc::now()
	)
	.fetch_optional(pool.get_ref())
	.await?
	.ok_or(ApiError::NotFound)?;

	Ok(HttpResponse::Ok().json(Item::try_from(row)?))
}

pub async fn delete_item(
	_admin: AdminUser,
	id: web::Path<Uuid>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let result = sqlx::query!("DELETE FROM items WHERE id = $1", id.into_inner())
		.execute(pool.get_ref())
		.await?;

	if result.rows_affected() == 0 {
		return Err(ApiError::NotFound);
	}

	Ok(HttpResponse::NoContent().finish())
}
