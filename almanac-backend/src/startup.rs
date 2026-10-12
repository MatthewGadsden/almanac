use actix_session::SessionMiddleware;
use actix_session::config::PersistentSession;
use actix_session::storage::CookieSessionStore;
use actix_web::cookie::time::Duration;
use actix_web::cookie::{Key, SameSite};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use std::net::TcpListener;

use crate::configuration::Settings;
use crate::routes::{
	create_item, current_admin, delete_item, get_item, health_check, list_items, login, logout,
	update_item,
};

/// Session cookie settings, taken from `Settings`.
#[derive(Clone)]
pub struct SessionSettings {
	pub key: Key,
	pub secure_cookies: bool,
}

impl SessionSettings {
	pub fn from_settings(settings: &Settings) -> Self {
		assert!(
			settings.session_key.len() >= 64,
			"session_key must be at least 64 characters long."
		);

		SessionSettings {
			key: Key::from(settings.session_key.as_bytes()),
			secure_cookies: settings.secure_cookies,
		}
	}
}

pub fn run(
	listener: TcpListener,
	connection_pool: PgPool,
	session: SessionSettings,
) -> Result<Server, std::io::Error> {
	let db_pool = web::Data::new(connection_pool);

	let server = HttpServer::new(move || {
		// The whole session lives in a signed, encrypted cookie; nothing is stored server-side
		let session_middleware =
			SessionMiddleware::builder(CookieSessionStore::default(), session.key.clone())
				.cookie_name("almanac_session".to_string())
				.cookie_secure(session.secure_cookies)
				.cookie_same_site(SameSite::Strict)
				.session_lifecycle(PersistentSession::default().session_ttl(Duration::days(7)))
				.build();

		App::new()
			.wrap(session_middleware)
			.route("/health_check", web::get().to(health_check))
			.service(
				web::scope("/items")
					.route("", web::get().to(list_items))
					.route("", web::post().to(create_item))
					.route("/{id}", web::get().to(get_item))
					.route("/{id}", web::put().to(update_item))
					.route("/{id}", web::delete().to(delete_item)),
			)
			.service(
				web::scope("/admin")
					.route("/login", web::post().to(login))
					.route("/logout", web::post().to(logout))
					.route("/me", web::get().to(current_admin)),
			)
			.app_data(db_pool.clone())
	})
	.listen(listener)?
	.run();

	Ok(server)
}
