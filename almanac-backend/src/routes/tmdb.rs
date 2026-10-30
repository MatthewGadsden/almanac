use actix_web::{HttpResponse, web};

use crate::auth::AdminUser;
use crate::connectors::{ConnectorContext, tmdb};
use crate::error::ApiError;

#[derive(serde::Serialize)]
struct TmdbStatus {
	configured: bool,
}

/// Whether TMDB search is available, so the film form knows whether to offer it.
pub async fn tmdb_status(
	_admin: AdminUser,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	Ok(HttpResponse::Ok().json(TmdbStatus {
		configured: context.settings.tmdb.is_configured(),
	}))
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
	#[serde(default)]
	query: String,
}

/// Admin only, like the film form it serves; it also keeps visitors off the API quota.
pub async fn tmdb_search(
	_admin: AdminUser,
	query: web::Query<SearchQuery>,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	let movies = tmdb::search(context.get_ref(), &query.query).await?;
	Ok(HttpResponse::Ok().json(movies))
}

pub async fn tmdb_movie(
	_admin: AdminUser,
	path: web::Path<i64>,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	let movie = tmdb::movie(context.get_ref(), path.into_inner()).await?;
	Ok(HttpResponse::Ok().json(movie))
}
