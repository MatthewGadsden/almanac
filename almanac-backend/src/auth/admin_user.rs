use actix_session::{Session, SessionExt};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{Ready, ready};
use uuid::Uuid;

use crate::error::ApiError;

const USER_ID_KEY: &str = "user_id";
const USERNAME_KEY: &str = "username";

/// The logged-in admin. Add it as a handler argument to require a session: requests without one
/// are rejected with 401 before the handler runs.
#[derive(Debug, serde::Serialize)]
pub struct AdminUser {
	pub user_id: Uuid,
	pub username: String,
}

impl AdminUser {
	/// Starts a fresh session for this user (a new cookie, so a pre-login one can't be reused).
	pub fn start_session(&self, session: &Session) -> Result<(), ApiError> {
		session.renew();
		session
			.insert(USER_ID_KEY, self.user_id)
			.and_then(|()| session.insert(USERNAME_KEY, &self.username))
			.map_err(|e| ApiError::Unexpected(format!("Failed to start session: {e}")))
	}

	fn from_session(session: &Session) -> Result<Self, ApiError> {
		let user_id = session.get::<Uuid>(USER_ID_KEY).ok().flatten();
		let username = session.get::<String>(USERNAME_KEY).ok().flatten();

		match (user_id, username) {
			(Some(user_id), Some(username)) => Ok(AdminUser { user_id, username }),
			_ => Err(ApiError::Unauthorized),
		}
	}
}

impl FromRequest for AdminUser {
	type Error = ApiError;
	type Future = Ready<Result<Self, Self::Error>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		ready(AdminUser::from_session(&req.get_session()))
	}
}
