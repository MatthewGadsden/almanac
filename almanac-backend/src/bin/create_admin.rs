//! Creates an admin account, or resets the password of an existing one.
//!
//! Run from almanac-backend/ (it reads configuration.yaml):
//!     cargo run --bin create_admin -- <username>

use almanac::auth::hash_password;
use almanac::configuration::get_configuration;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

const MIN_PASSWORD_LENGTH: usize = 12;

#[tokio::main]
async fn main() {
	let Some(username) = std::env::args().nth(1) else {
		eprintln!("Usage: cargo run --bin create_admin -- <username>");
		std::process::exit(2);
	};

	let password = rpassword::prompt_password("Password: ").expect("Failed to read password");
	if password.chars().count() < MIN_PASSWORD_LENGTH {
		eprintln!("Password must be at least {MIN_PASSWORD_LENGTH} characters.");
		std::process::exit(1);
	}
	let confirmation =
		rpassword::prompt_password("Confirm password: ").expect("Failed to read password");
	if password != confirmation {
		eprintln!("Passwords don't match.");
		std::process::exit(1);
	}

	let configuration = get_configuration().expect("Failed to read configuration.");
	let pool = PgPool::connect(&configuration.database.connection_string())
		.await
		.expect("Failed to connect to Postgres.");
	let password_hash = hash_password(&password).expect("Failed to hash password");

	sqlx::query!(
		r#"
		INSERT INTO users (id, username, password_hash, created_at)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (username) DO UPDATE SET password_hash = EXCLUDED.password_hash
		"#,
		Uuid::new_v4(),
		username,
		password_hash,
		Utc::now()
	)
	.execute(&pool)
	.await
	.expect("Failed to save admin user");

	println!("Admin user '{username}' is ready. Log in at /admin.");
}
