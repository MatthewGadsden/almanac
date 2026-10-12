use almanac::configuration::{DatabaseSettings, Settings, get_configuration};
use almanac::connectors::ConnectorContext;
use almanac::startup::{SessionSettings, run};
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use chrono::Utc;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::fmt::Display;
use std::net::TcpListener;
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
	pub address: String,
	pub db_pool: PgPool,
	pub config: Settings,
	/// Keeps cookies, so logging in with it makes later requests authenticated.
	pub api_client: reqwest::Client,
	pub test_admin: TestAdmin,
	/// Stands in for letterboxd.com; mount the RSS responses a test needs on it.
	pub letterboxd: MockServer,
	/// Stands in for Strava: OAuth under /oauth, the API under /api/v3.
	pub strava: MockServer,
}

pub struct TestAdmin {
	pub username: String,
	pub password: String,
}

impl TestAdmin {
	fn generate() -> Self {
		TestAdmin {
			username: Uuid::new_v4().to_string(),
			password: Uuid::new_v4().to_string(),
		}
	}

	async fn store(&self, pool: &PgPool) {
		// Cheap Argon2 parameters keep tests fast; verification reads them from the hash
		let salt = SaltString::generate(&mut OsRng);
		let password_hash = Argon2::new(
			Algorithm::Argon2id,
			Version::V0x13,
			Params::new(15000, 2, 1, None).unwrap(),
		)
		.hash_password(self.password.as_bytes(), &salt)
		.unwrap()
		.to_string();

		sqlx::query!(
			"INSERT INTO users (id, username, password_hash, created_at) VALUES ($1, $2, $3, $4)",
			Uuid::new_v4(),
			self.username,
			password_hash,
			Utc::now()
		)
		.execute(pool)
		.await
		.expect("Failed to store test admin.");
	}
}

impl TestApp {
	pub async fn post_login(&self, body: &serde_json::Value) -> reqwest::Response {
		self.api_client
			.post(format!("{}/admin/login", self.address))
			.json(body)
			.send()
			.await
			.expect("Failed to execute request.")
	}

	/// Logs the client in as the test admin.
	pub async fn login_admin(&self) {
		let body = serde_json::json!({
			"username": self.test_admin.username,
			"password": self.test_admin.password,
		});
		let response = self.post_login(&body).await;
		assert_eq!(200, response.status().as_u16(), "Test admin login failed.");
	}

	pub async fn post_logout(&self) -> reqwest::Response {
		self.api_client
			.post(format!("{}/admin/logout", self.address))
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn get(&self, path: &str) -> reqwest::Response {
		self.api_client
			.get(format!("{}{}", self.address, path))
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn post_item(&self, body: &serde_json::Value) -> reqwest::Response {
		self.api_client
			.post(format!("{}/items", self.address))
			.json(body)
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn get_items(&self) -> reqwest::Response {
		self.get("/items").await
	}

	pub async fn get_item(&self, id: impl Display) -> reqwest::Response {
		self.get(&format!("/items/{id}")).await
	}

	pub async fn put_item(&self, id: impl Display, body: &serde_json::Value) -> reqwest::Response {
		self.api_client
			.put(format!("{}/items/{}", self.address, id))
			.json(body)
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn delete_item(&self, id: impl Display) -> reqwest::Response {
		self.api_client
			.delete(format!("{}/items/{}", self.address, id))
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn get_connections(&self) -> reqwest::Response {
		self.get("/connections").await
	}

	pub async fn put_connection(
		&self,
		provider: &str,
		body: &serde_json::Value,
	) -> reqwest::Response {
		self.api_client
			.put(format!("{}/connections/{}", self.address, provider))
			.json(body)
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn delete_connection(&self, provider: &str) -> reqwest::Response {
		self.api_client
			.delete(format!("{}/connections/{}", self.address, provider))
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn sync_connection(&self, provider: &str) -> reqwest::Response {
		self.api_client
			.post(format!("{}/connections/{}/sync", self.address, provider))
			.send()
			.await
			.expect("Failed to execute request.")
	}

	pub async fn get_strava_authorize(&self) -> reqwest::Response {
		self.get("/connections/strava/authorize").await
	}

	pub async fn post_strava_callback(&self, body: &serde_json::Value) -> reqwest::Response {
		self.api_client
			.post(format!("{}/connections/strava/callback", self.address))
			.json(body)
			.send()
			.await
			.expect("Failed to execute request.")
	}

	/// Drops the per-test database. Call at the end of every test.
	pub async fn cleanup(&self) {
		database_spindown(&self.config.database).await;
	}
}

pub async fn spawn_app() -> TestApp {
	spawn_app_with(|_| {}).await
}

/// Like `spawn_app`, with a chance to adjust the configuration first.
pub async fn spawn_app_with(configure: impl FnOnce(&mut Settings)) -> TestApp {
	let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
	let port = listener.local_addr().unwrap().port();

	let app_address = format!("http://127.0.0.1:{port}");

	let letterboxd = MockServer::start().await;
	let strava = MockServer::start().await;

	let mut configuration = get_configuration().expect("Failed to read configuration");
	configuration.database.database_name = Uuid::new_v4().to_string();
	configuration.connectors.letterboxd_base_url = letterboxd.uri();
	configuration.connectors.strava.client_id = "test-client".into();
	configuration.connectors.strava.client_secret = "test-secret".into();
	configuration.connectors.strava.oauth_base_url = format!("{}/oauth", strava.uri());
	configuration.connectors.strava.api_base_url = format!("{}/api/v3", strava.uri());
	configure(&mut configuration);

	let connection_pool = configure_database(&configuration.database).await;

	let session = SessionSettings::from_settings(&configuration);
	let connectors = ConnectorContext::new(configuration.connectors.clone());
	let server = run(listener, connection_pool.clone(), session, connectors)
		.expect("Failed to bind address");
	tokio::spawn(server);

	let test_admin = TestAdmin::generate();
	test_admin.store(&connection_pool).await;

	let api_client = reqwest::Client::builder()
		.cookie_store(true)
		.build()
		.expect("Failed to build HTTP client.");

	TestApp {
		address: app_address,
		db_pool: connection_pool,
		config: configuration,
		api_client,
		test_admin,
		letterboxd,
		strava,
	}
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
	let mut connection = PgConnection::connect(&config.connection_string_without_db())
		.await
		.expect("Failed to connect to Postgres");

	connection
		.execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
		.await
		.expect("Failed to create database.");

	let connection_pool = PgPool::connect(&config.connection_string())
		.await
		.expect("Failed to connect to Postgres.");

	sqlx::migrate!("./migrations")
		.run(&connection_pool)
		.await
		.expect("Failed to migrate the database");

	connection_pool
}

pub async fn database_spindown(config: &DatabaseSettings) {
	let mut connection = PgConnection::connect(&config.connection_string_without_db())
		.await
		.expect("Failed to connect to Postgres");

	connection
		.execute(format!(r#"DROP DATABASE "{}" WITH (FORCE);"#, config.database_name).as_str())
		.await
		.expect("Failed to drop database.");
}
