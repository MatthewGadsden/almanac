use actix_web::{HttpResponse, web};
use sqlx::PgPool;

use crate::auth::AdminUser;
use crate::connectors::{self, ConnectorContext, Provider};
use crate::domain::Visibility;
use crate::error::ApiError;

#[derive(serde::Deserialize)]
pub struct ConnectionPayload {
	settings: serde_json::Value,
	/// Defaults to public.
	#[serde(default)]
	import_visibility: Option<Visibility>,
}

fn provider(path: &str) -> Result<Provider, ApiError> {
	Provider::parse(path).ok_or(ApiError::NotFound)
}

pub async fn list_connections(
	_admin: AdminUser,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let connections = connectors::list_connections(pool.get_ref()).await?;
	Ok(HttpResponse::Ok().json(connections))
}

/// Connects a provider, or updates its settings. Doesn't sync; call the sync endpoint for that.
pub async fn save_connection(
	_admin: AdminUser,
	path: web::Path<String>,
	payload: web::Json<ConnectionPayload>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let provider = provider(&path)?;
	let payload = payload.into_inner();
	let settings = provider
		.validate_settings(payload.settings)
		.map_err(ApiError::Validation)?;

	let connection = connectors::save_connection(
		pool.get_ref(),
		provider,
		settings,
		payload.import_visibility.unwrap_or(Visibility::Public),
	)
	.await?;

	Ok(HttpResponse::Ok().json(connection))
}

/// Disconnects a provider. Items it imported stay in the garden. For OAuth providers the tokens
/// are revoked first, on a best-effort basis: a failure there shouldn't stop disconnecting.
pub async fn delete_connection(
	_admin: AdminUser,
	path: web::Path<String>,
	pool: web::Data<PgPool>,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	let provider = provider(&path)?;
	let connection = connectors::get_connection(pool.get_ref(), provider)
		.await?
		.ok_or(ApiError::NotFound)?;

	if provider == Provider::Strava
		&& let Err(e) = connectors::strava::revoke(&connection, context.get_ref()).await
	{
		eprintln!("Couldn't revoke Strava access while disconnecting: {e}");
	}

	connectors::delete_connection(pool.get_ref(), provider).await?;
	Ok(HttpResponse::NoContent().finish())
}

/// Syncs now. 502 when the provider fails; the reason is also stored on the connection.
pub async fn sync_connection(
	_admin: AdminUser,
	path: web::Path<String>,
	pool: web::Data<PgPool>,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	let provider = provider(&path)?;
	let summary = connectors::sync(provider, pool.get_ref(), context.get_ref()).await?;
	Ok(HttpResponse::Ok().json(summary))
}
