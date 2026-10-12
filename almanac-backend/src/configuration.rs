#[derive(serde::Deserialize)]
pub struct Settings {
	pub database: DatabaseSettings,
	pub application_port: u16,
	pub session_key: String,
	pub secure_cookies: bool,
	pub connectors: ConnectorSettings,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ConnectorSettings {
	pub letterboxd_base_url: String,
	/// 0 turns automatic syncing off.
	pub sync_interval_minutes: u64,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
	pub username: String,
	pub password: String,
	pub port: u16,
	pub host: String,
	pub database_name: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
	let settings = config::Config::builder()
		.add_source(config::File::new(
			"configuration.yaml",
			config::FileFormat::Yaml,
		))
		// e.g. APP_SESSION_KEY=... or APP_DATABASE__PASSWORD=... override the file
		.add_source(
			config::Environment::with_prefix("APP")
				// Without this the prefix separator defaults to "__" as well (APP__SESSION_KEY)
				.prefix_separator("_")
				.separator("__"),
		)
		.build()?;

	settings.try_deserialize::<Settings>()
}

impl DatabaseSettings {
	pub fn connection_string(&self) -> String {
		format!(
			"postgres://{}:{}@{}:{}/{}",
			self.username, self.password, self.host, self.port, self.database_name
		)
	}

	pub fn connection_string_without_db(&self) -> String {
		format!(
			"postgres://{}:{}@{}:{}",
			self.username, self.password, self.host, self.port
		)
	}
}
