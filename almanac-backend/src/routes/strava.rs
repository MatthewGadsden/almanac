use actix_session::Session;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::connectors::{self, ConnectorContext, Provider, strava};
use crate::domain::Visibility;
use crate::error::ApiError;

/// Session key for the OAuth `state`, which ties Strava's redirect back to this admin's request.
const STATE_KEY: &str = "strava_oauth_state";

#[derive(serde::Serialize)]
struct AuthorizeResponse {
	url: String,
}

/// Step 1: where to send the admin to authorise Almanac on Strava.
pub async fn strava_authorize(
	_admin: AdminUser,
	session: Session,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	if !context.settings.strava.is_configured() {
		return Err(ApiError::Validation(strava::NOT_CONFIGURED.into()));
	}

	let state = Uuid::new_v4().to_string();
	session
		.insert(STATE_KEY, &state)
		.map_err(|e| ApiError::Unexpected(format!("Failed to store OAuth state: {e}")))?;
	let url = strava::authorize_url(context.get_ref(), &state).map_err(ApiError::Unexpected)?;

	Ok(HttpResponse::Ok().json(AuthorizeResponse { url }))
}

/// What Strava put on the redirect back to the frontend's callback page.
#[derive(serde::Deserialize)]
pub struct StravaCallback {
	code: String,
	state: String,
	/// The scopes actually granted.
	#[serde(default)]
	scope: String,
}

/// Step 2: the frontend's callback page posts Strava's code here. The session cookie is
/// `SameSite=Strict`, so it isn't sent on Strava's cross-site redirect itself; the callback page
/// makes this same-site request instead.
pub async fn strava_callback(
	_admin: AdminUser,
	payload: web::Json<StravaCallback>,
	session: Session,
	pool: web::Data<PgPool>,
	context: web::Data<ConnectorContext>,
) -> Result<HttpResponse, ApiError> {
	let expected = session.get::<String>(STATE_KEY).ok().flatten();
	session.remove(STATE_KEY);
	if expected.as_deref() != Some(payload.state.as_str()) {
		return Err(ApiError::Validation(
			"This Strava authorisation didn't start here or has already been used; try connecting again"
				.into(),
		));
	}

	if !strava::can_read_activities(&payload.scope) {
		return Err(ApiError::Validation(
			"Almanac needs permission to view your activities: connect again and leave that box ticked"
				.into(),
		));
	}

	let (credentials, athlete) = strava::exchange_code(context.get_ref(), &payload.code).await?;
	let connection = connectors::save_oauth_connection(
		pool.get_ref(),
		Provider::Strava,
		athlete.to_value(),
		credentials.to_value(),
		Visibility::Private,
	)
	.await?;

	Ok(HttpResponse::Ok().json(connection))
}
