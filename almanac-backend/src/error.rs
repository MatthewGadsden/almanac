use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};

/// Error type returned by route handlers, mapped onto an HTTP response.
#[derive(Debug)]
pub enum ApiError {
	NotFound,
	Validation(String),
	/// No admin session on a request that needs one.
	Unauthorized,
	/// Login attempt with an unknown username or wrong password.
	InvalidCredentials,
	Database(sqlx::Error),
	Unexpected(String),
}

impl std::fmt::Display for ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ApiError::NotFound => write!(f, "Resource not found"),
			ApiError::Validation(message) => write!(f, "Invalid request: {message}"),
			ApiError::Unauthorized => write!(f, "You need to log in to do that"),
			ApiError::InvalidCredentials => write!(f, "Invalid username or password"),
			ApiError::Database(e) => write!(f, "Database error: {e}"),
			ApiError::Unexpected(message) => write!(f, "Unexpected error: {message}"),
		}
	}
}

impl From<sqlx::Error> for ApiError {
	fn from(e: sqlx::Error) -> Self {
		ApiError::Database(e)
	}
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match self {
			ApiError::NotFound => StatusCode::NOT_FOUND,
			ApiError::Validation(_) => StatusCode::BAD_REQUEST,
			ApiError::Unauthorized | ApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
			ApiError::Database(_) | ApiError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> HttpResponse {
		match self {
			ApiError::NotFound => HttpResponse::NotFound().finish(),
			ApiError::Validation(message) => {
				HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
			}
			ApiError::Unauthorized | ApiError::InvalidCredentials => {
				let body = serde_json::json!({ "error": self.to_string() });
				HttpResponse::Unauthorized().json(body)
			}
			ApiError::Database(_) | ApiError::Unexpected(_) => {
				// Log the details server-side, but don't leak them to the client
				eprintln!("{self}");
				HttpResponse::InternalServerError().finish()
			}
		}
	}
}
