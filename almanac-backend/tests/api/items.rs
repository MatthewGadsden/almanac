use almanac::domain::{Item, ItemKind, Visibility};
use serde_json::json;
use uuid::Uuid;

use crate::helpers::{TestApp, spawn_app};

/// Item writes need an admin session; reads don't.
pub async fn spawn_admin_app() -> TestApp {
	let app = spawn_app().await;
	app.login_admin().await;
	app
}

async fn create_note(app: &TestApp, title: &str, body: &str) -> Item {
	let payload = json!({ "kind": "note", "title": title, "topic": "Seasons", "body": body });
	let response = app.post_item(&payload).await;
	assert_eq!(201, response.status().as_u16());

	response.json().await.expect("Failed to parse item.")
}

#[tokio::test]
async fn create_note_returns_201_for_valid_payload() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload =
		json!({ "kind": "note", "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());

	let item: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(item.kind, ItemKind::Note);
	assert_eq!(item.title, "Equinox");
	assert_eq!(item.topic.as_deref(), Some("Seasons"));
	assert_eq!(item.body, "Equal day.");
	assert_eq!(item.data, json!({}));
	assert_eq!(item.visibility, Visibility::Public);
	assert_eq!(item.source, "manual");
	assert_eq!(item.occurred_at, item.created_at);

	let saved = sqlx::query!(
		"SELECT kind, title, topic, body FROM items WHERE id = $1",
		item.id
	)
	.fetch_one(&app.db_pool)
	.await
	.expect("Failed to fetch saved item.");

	assert_eq!(saved.kind, "note");
	assert_eq!(saved.title, "Equinox");
	assert_eq!(saved.topic.as_deref(), Some("Seasons"));
	assert_eq!(saved.body, "Equal day.");

	app.cleanup().await;
}

#[tokio::test]
async fn create_note_returns_400_for_invalid_payload() {
	// Arrange
	let app = spawn_admin_app().await;
	let valid =
		json!({ "kind": "note", "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	let mut test_cases = Vec::new();
	for field in ["kind", "title", "topic", "body"] {
		let mut missing = valid.clone();
		missing.as_object_mut().unwrap().remove(field);
		test_cases.push((missing, format!("missing the {field}")));
	}
	for field in ["title", "topic"] {
		let mut blank = valid.clone();
		blank[field] = json!("   ");
		test_cases.push((blank, format!("a blank {field}")));
	}
	let mut unknown_kind = valid.clone();
	unknown_kind["kind"] = json!("podcast");
	test_cases.push((unknown_kind, "an unknown kind".to_string()));
	test_cases.push((json!({}), "empty".to_string()));

	for (invalid_body, error_message) in test_cases {
		// Act
		let response = app.post_item(&invalid_body).await;

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
async fn list_items_returns_items_newest_first() {
	// Arrange
	let app = spawn_admin_app().await;
	let older = json!({
		"kind": "note", "title": "Older", "topic": "Seasons", "body": "",
		"occurred_at": "2026-01-01T12:00:00Z",
	});
	let newer = json!({
		"kind": "note", "title": "Newer", "topic": "Seasons", "body": "",
		"occurred_at": "2026-06-01T12:00:00Z",
	});
	app.post_item(&older).await;
	app.post_item(&newer).await;

	// Act
	let response = app.get_items().await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let items: Vec<Item> = response.json().await.expect("Failed to parse items.");
	let titles: Vec<&str> = items.iter().map(|item| item.title.as_str()).collect();
	assert_eq!(titles, ["Newer", "Older"]);

	app.cleanup().await;
}

#[tokio::test]
async fn get_item_returns_the_item() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_note(&app, "Solstice", "Longest day of the year.").await;

	// Act
	let response = app.get_item(created.id).await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let item: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(item.id, created.id);
	assert_eq!(item.title, "Solstice");

	app.cleanup().await;
}

#[tokio::test]
async fn get_item_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_app().await;

	// Act
	let response = app.get_item(Uuid::new_v4()).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn update_item_replaces_the_item() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_note(&app, "Solstice", "Longest day of the year.").await;
	let payload =
		json!({ "kind": "note", "title": "Winter solstice", "topic": "Winter", "body": "Short." });

	// Act
	let response = app.put_item(created.id, &payload).await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let item: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(item.id, created.id);
	assert_eq!(item.title, "Winter solstice");
	assert_eq!(item.topic.as_deref(), Some("Winter"));
	assert_eq!(item.body, "Short.");
	assert_eq!(item.created_at, created.created_at);
	assert!(item.updated_at >= created.updated_at);
	// Omitted on update, so left as they were
	assert_eq!(item.occurred_at, created.occurred_at);
	assert_eq!(item.visibility, created.visibility);

	app.cleanup().await;
}

#[tokio::test]
async fn update_item_returns_400_for_blank_title() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_note(&app, "Solstice", "Longest day of the year.").await;
	let payload = json!({ "kind": "note", "title": "", "topic": "Seasons", "body": "No title" });

	// Act
	let response = app.put_item(created.id, &payload).await;

	// Assert
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn update_item_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload =
		json!({ "kind": "note", "title": "Equinox", "topic": "Seasons", "body": "Equal day." });

	// Act
	let response = app.put_item(Uuid::new_v4(), &payload).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn delete_item_removes_the_item() {
	// Arrange
	let app = spawn_admin_app().await;
	let created = create_note(&app, "Solstice", "Longest day of the year.").await;

	// Act
	let response = app.delete_item(created.id).await;

	// Assert
	assert_eq!(204, response.status().as_u16());
	assert_eq!(404, app.get_item(created.id).await.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn delete_item_returns_404_for_unknown_id() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let response = app.delete_item(Uuid::new_v4()).await;

	// Assert
	assert_eq!(404, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn private_items_are_hidden_from_visitors() {
	// Arrange
	let app = spawn_admin_app().await;
	let public = json!({ "kind": "note", "title": "Public", "topic": "Garden", "body": "" });
	let private = json!({
		"kind": "note", "title": "Private", "topic": "Garden", "body": "",
		"visibility": "private",
	});
	app.post_item(&public).await;
	let private: Item = app
		.post_item(&private)
		.await
		.json()
		.await
		.expect("Failed to parse item.");

	// The admin sees both
	let items: Vec<Item> = app.get_items().await.json().await.unwrap();
	assert_eq!(items.len(), 2);

	// Act
	app.post_logout().await;
	let list = app.get_items().await;
	let single = app.get_item(private.id).await;

	// Assert
	let items: Vec<Item> = list.json().await.expect("Failed to parse items.");
	let titles: Vec<&str> = items.iter().map(|item| item.title.as_str()).collect();
	assert_eq!(titles, ["Public"]);
	assert_eq!(404, single.status().as_u16());

	app.cleanup().await;
}
