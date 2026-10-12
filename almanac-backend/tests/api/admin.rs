use serde_json::json;
use uuid::Uuid;

use crate::helpers::spawn_app;

#[tokio::test]
async fn login_returns_200_and_starts_a_session_for_valid_credentials() {
	// Arrange
	let app = spawn_app().await;
	let body = json!({
		"username": app.test_admin.username,
		"password": app.test_admin.password,
	});

	// Act
	let response = app.post_login(&body).await;

	// Assert
	assert_eq!(200, response.status().as_u16());

	let me = app.get("/admin/me").await;
	assert_eq!(200, me.status().as_u16());
	let me: serde_json::Value = me.json().await.expect("Failed to parse admin.");
	assert_eq!(me["username"], app.test_admin.username);

	app.cleanup().await;
}

#[tokio::test]
async fn login_returns_401_for_invalid_credentials() {
	// Arrange
	let app = spawn_app().await;
	let test_cases = [
		(
			app.test_admin.username.clone(),
			"wrong-password".to_string(),
			"a wrong password",
		),
		(
			Uuid::new_v4().to_string(),
			app.test_admin.password.clone(),
			"an unknown username",
		),
	];

	for (username, password, description) in test_cases {
		// Act
		let body = json!({ "username": username, "password": password });
		let response = app.post_login(&body).await;

		// Assert
		assert_eq!(
			401,
			response.status().as_u16(),
			"The API did not fail with 401 Unauthorized for {description}."
		);
		assert_eq!(401, app.get("/admin/me").await.status().as_u16());
	}

	app.cleanup().await;
}

#[tokio::test]
async fn me_returns_401_without_a_session() {
	// Arrange
	let app = spawn_app().await;

	// Act
	let response = app.get("/admin/me").await;

	// Assert
	assert_eq!(401, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn logout_ends_the_session() {
	// Arrange
	let app = spawn_app().await;
	app.login_admin().await;

	// Act
	let response = app.post_logout().await;

	// Assert
	assert_eq!(204, response.status().as_u16());
	assert_eq!(401, app.get("/admin/me").await.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn entry_writes_return_401_without_a_session() {
	// Arrange
	let app = spawn_app().await;
	let payload = json!({ "title": "Equinox", "topic": "Seasons", "body": "Equal day." });
	let id = Uuid::new_v4();

	// Act
	let created = app.post_entry(&payload).await;
	let updated = app.put_entry(id, &payload).await;
	let deleted = app.delete_entry(id).await;

	// Assert
	assert_eq!(401, created.status().as_u16(), "create");
	assert_eq!(401, updated.status().as_u16(), "update");
	assert_eq!(401, deleted.status().as_u16(), "delete");

	let count = sqlx::query_scalar!("SELECT COUNT(*) FROM entries")
		.fetch_one(&app.db_pool)
		.await
		.expect("Failed to count entries.");
	assert_eq!(count, Some(0));

	app.cleanup().await;
}

#[tokio::test]
async fn entries_can_be_read_without_a_session() {
	// Arrange
	let app = spawn_app().await;
	app.login_admin().await;
	let payload = json!({ "title": "Equinox", "topic": "Seasons", "body": "Equal day." });
	let created: serde_json::Value = app
		.post_entry(&payload)
		.await
		.json()
		.await
		.expect("Failed to parse entry.");
	app.post_logout().await;

	// Act
	let list = app.get_entries().await;
	let single = app.get_entry(created["id"].as_str().unwrap()).await;

	// Assert
	assert_eq!(200, list.status().as_u16());
	assert_eq!(200, single.status().as_u16());

	app.cleanup().await;
}
