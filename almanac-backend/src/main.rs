use almanac::configuration::get_configuration;
use almanac::connectors::{ConnectorContext, run_scheduler};
use almanac::startup::{SessionSettings, run};
use sqlx::PgPool;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
	let configuration = get_configuration().expect("Failed to read configuration.");
	let connection_pool = PgPool::connect(&configuration.database.connection_string())
		.await
		.expect("Failed to connect to Postgres.");

	let address = format!("127.0.0.1:{}", configuration.application_port);
	let listener = TcpListener::bind(address)?;
	let session = SessionSettings::from_settings(&configuration);
	let connectors = ConnectorContext::new(configuration.connectors.clone());

	// Keeps connections (Letterboxd, ...) in sync in the background while the API runs
	tokio::spawn(run_scheduler(connection_pool.clone(), connectors.clone()));

	run(listener, connection_pool, session, connectors)?.await
}
