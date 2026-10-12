use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::domain::{Entry, EntryPayload};
use crate::error::ApiError;

pub async fn list_entries(pool: web::Data<PgPool>) -> Result<HttpResponse, ApiError> {
	let entries = sqlx::query_as!(
		Entry,
		r#"
		SELECT id, title, topic, body, created_at, updated_at
		FROM entries
		ORDER BY created_at DESC
		"#
	)
	.fetch_all(pool.get_ref())
	.await?;

	Ok(HttpResponse::Ok().json(entries))
}

pub async fn create_entry(
	_admin: AdminUser,
	payload: web::Json<EntryPayload>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	payload.validate().map_err(ApiError::Validation)?;

	let entry = sqlx::query_as!(
		Entry,
		r#"
		INSERT INTO entries (id, title, topic, body, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $5)
		RETURNING id, title, topic, body, created_at, updated_at
		"#,
		Uuid::new_v4(),
		payload.title,
		payload.topic,
		payload.body,
		Utc::now()
	)
	.fetch_one(pool.get_ref())
	.await?;

	Ok(HttpResponse::Created().json(entry))
}

pub async fn get_entry(
	id: web::Path<Uuid>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let entry = sqlx::query_as!(
		Entry,
		r#"
		SELECT id, title, topic, body, created_at, updated_at
		FROM entries
		WHERE id = $1
		"#,
		id.into_inner()
	)
	.fetch_optional(pool.get_ref())
	.await?
	.ok_or(ApiError::NotFound)?;

	Ok(HttpResponse::Ok().json(entry))
}

pub async fn update_entry(
	_admin: AdminUser,
	id: web::Path<Uuid>,
	payload: web::Json<EntryPayload>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	payload.validate().map_err(ApiError::Validation)?;

	let entry = sqlx::query_as!(
		Entry,
		r#"
		UPDATE entries
		SET title = $2, topic = $3, body = $4, updated_at = $5
		WHERE id = $1
		RETURNING id, title, topic, body, created_at, updated_at
		"#,
		id.into_inner(),
		payload.title,
		payload.topic,
		payload.body,
		Utc::now()
	)
	.fetch_optional(pool.get_ref())
	.await?
	.ok_or(ApiError::NotFound)?;

	Ok(HttpResponse::Ok().json(entry))
}

pub async fn delete_entry(
	_admin: AdminUser,
	id: web::Path<Uuid>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let result = sqlx::query!("DELETE FROM entries WHERE id = $1", id.into_inner())
		.execute(pool.get_ref())
		.await?;

	if result.rows_affected() == 0 {
		return Err(ApiError::NotFound);
	}

	Ok(HttpResponse::NoContent().finish())
}
