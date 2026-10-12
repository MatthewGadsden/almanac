use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use sqlx::PgPool;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::error::ApiError;

/// Hashed once and verified against when the username doesn't exist, so a login attempt takes
/// about as long for an unknown user as for a wrong password (no username enumeration by timing).
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
	hash_password("almanac-dummy-password").expect("Failed to hash dummy password")
});

/// Hashes a password with Argon2id into a PHC string (algorithm, parameters and salt included).
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
	let salt = SaltString::generate(&mut OsRng);
	Ok(Argon2::default()
		.hash_password(password.as_bytes(), &salt)?
		.to_string())
}

/// Checks a password against a PHC string, using the parameters stored in the hash.
pub fn verify_password(expected_hash: &str, password: &str) -> bool {
	PasswordHash::new(expected_hash)
		.and_then(|hash| Argon2::default().verify_password(password.as_bytes(), &hash))
		.is_ok()
}

/// Returns the user's id if the username exists and the password matches.
pub async fn validate_credentials(
	username: &str,
	password: &str,
	pool: &PgPool,
) -> Result<Uuid, ApiError> {
	let user = sqlx::query!(
		"SELECT id, password_hash FROM users WHERE username = $1",
		username
	)
	.fetch_optional(pool)
	.await?;

	let (user_id, expected_hash) = match user {
		Some(row) => (Some(row.id), row.password_hash),
		None => (None, DUMMY_HASH.clone()),
	};

	// Argon2 is deliberately slow, so keep it off the async worker threads
	let password = password.to_string();
	let verified = tokio::task::spawn_blocking(move || verify_password(&expected_hash, &password))
		.await
		.map_err(|e| ApiError::Unexpected(format!("Password verification failed: {e}")))?;

	match (user_id, verified) {
		(Some(id), true) => Ok(id),
		_ => Err(ApiError::InvalidCredentials),
	}
}
