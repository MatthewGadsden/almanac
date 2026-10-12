use almanac::domain::Entry;
use serde_json::json;
use uuid::Uuid;

use crate::helpers::{TestApp, spawn_app};

/// Entry writes need an admin session; reads don't.
async fn spawn_admin_app() -> TestApp {
	let app = spawn_app().await;
	app.login_admin().await;
	app
}

async fn create_entry(app: &TestApp, title: &str, body: &str) -> Entry {
	let payload = json!({ "title": title, "topic": "Seasons", "body": body });
	let response = app.post_entry(&payload).await;
	assert_eq!(201, response.status().as_u16());

	response.json().await.expect("Failed to parse entry.")
}

#[tokio::test]
async fn create_entry_returns_201_for_valid_payload() {
	// Arrange
	let app = spawn_admin_app().await;

	let payload = json!({ "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	// Act
	let response = app.post_entry(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());

	let entry: Entry = response.json().await.expect("Failed to parse entry.");
	assert_eq!(entry.title, "Equinox");
	assert_eq!(entry.topic, "Seasons");
	assert_eq!(entry.body, "Equal day.");

	let saved = sqlx::query!(
		"SELECT title, topic, body FROM entries WHERE id = $1",
		entry.id
	)
	.fetch_one(&app.db_pool)
	.await
	.expect("Failed to fetch saved entry.");

	assert_eq!(saved.title, "Equinox");
	assert_eq!(saved.topic, "Seasons");
	assert_eq!(saved.body, "Equal day.");

	app.cleanup().await;
}

#[tokio::test]
async fn create_entry_returns_400_for_invalid_payload() {
	// Arrange
	let app = spawn_admin_app().await;
	let valid = json!({ "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	let mut test_cases = Vec::new();
	for field in ["title", "topic", "body"] {
		let mut missing = valid.clone();
		missing.as_object_mut().unwrap().remove(field);
		test_cases.push((missing, format!("missing the {field}")));
	}
	for field in ["title", "topic"] {
		let mut blank = valid.clone();
		blank[field] = json!("   ");
		test_cases.push((blank, format!("a blank {field}")));
	}
	test_cases.push((json!({}), "empty".to_string()));

	for (invalid_body, error_message) in test_cases {
		// Act
		let response = app.post_entry(&invalid_body).await;

		// Assert
		assert_eq!(
			400,
			response.status().as_u16(),
			"The API did not fail with 400 Bad Request when the payload was {error_message}."
		);
	}

	app.cleanup().await;
}

#[tokio::test]
async fn list_entries_returns_all_entries() {
	// Arrange
	let app = spawn_admin_app().await;
	create_entry(&app, "Solstice", "Longest day of the year.").await;
	create_entry(&app, "Equinox", "Day and night are equal.").await;

	// Act
	let response = app.get_entries().await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let entries: Vec<Entry> = response.json().await.expect("Failed to parse entries.");
	assert_eq!(entries.len(), 2);

	app.cleanup().await;
}

#[tokio::test]
async fn get_entry_returns_the_entry() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_entry(&app, "Solstice", "Longest day of the year.").await;

	// Act
	let response = app.get_entry(created.id).await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let entry: Entry = response.json().await.expect("Failed to parse entry.");
	assert_eq!(entry.id, created.id);
	assert_eq!(entry.title, "Solstice");
	assert_eq!(entry.topic, "Seasons");

	app.cleanup().await;
}

#[tokio::test]
async fn get_entry_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let response = app.get_entry(Uuid::new_v4()).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn update_entry_replaces_the_entry() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_entry(&app, "Solstice", "Longest day of the year.").await;

	let payload = json!({ "title": "Winter solstice", "topic": "Winter", "body": "Short day." });

	// Act
	let response = app.put_entry(created.id, &payload).await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let entry: Entry = response.json().await.expect("Failed to parse entry.");
	assert_eq!(entry.id, created.id);
	assert_eq!(entry.title, "Winter solstice");
	assert_eq!(entry.topic, "Winter");
	assert_eq!(entry.body, "Short day.");
	assert_eq!(entry.created_at, created.created_at);
	assert!(entry.updated_at >= created.updated_at);

	app.cleanup().await;
}

#[tokio::test]
async fn update_entry_returns_400_for_blank_title() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_entry(&app, "Solstice", "Longest day of the year.").await;

	let payload = json!({ "title": "", "topic": "Seasons", "body": "No title" });

	// Act
	let response = app.put_entry(created.id, &payload).await;

	// Assert
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn update_entry_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_admin_app().await;

	let payload = json!({ "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	// Act
	let response = app.put_entry(Uuid::new_v4(), &payload).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn delete_entry_removes_the_entry() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_entry(&app, "Solstice", "Longest day of the year.").await;

	// Act
	let response = app.delete_entry(created.id).await;

	// Assert
	assert_eq!(204, response.status().as_u16());
	assert_eq!(404, app.get_entry(created.id).await.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn delete_entry_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let response = app.delete_entry(Uuid::new_v4()).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}
