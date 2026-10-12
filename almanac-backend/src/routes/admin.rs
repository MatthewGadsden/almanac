use actix_session::Session;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

use crate::auth::{AdminUser, validate_credentials};
use crate::error::ApiError;

#[derive(serde::Deserialize)]
pub struct Credentials {
	username: String,
	password: String,
}

pub async fn login(
	credentials: web::Json<Credentials>,
	pool: web::Data<PgPool>,
	session: Session,
) -> Result<HttpResponse, ApiError> {
	let Credentials { username, password } = credentials.into_inner();
	let user_id = validate_credentials(&username, &password, pool.get_ref()).await?;

	let admin = AdminUser { user_id, username };
	admin.start_session(&session)?;

	Ok(HttpResponse::Ok().json(admin))
}

pub async fn logout(session: Session) -> HttpResponse {
	session.purge();
	HttpResponse::NoContent().finish()
}

/// Who is logged in. 401 when nobody is, which the frontend uses to decide what to show.
pub async fn current_admin(admin: AdminUser) -> HttpResponse {
	HttpResponse::Ok().json(admin)
}
