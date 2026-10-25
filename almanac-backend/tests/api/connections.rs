use almanac::connectors::{Connection, SyncSummary};
use almanac::domain::{Item, ItemKind, Visibility};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::TestApp;
use crate::items::spawn_admin_app;

const FEED: &str = include_str!("fixtures/letterboxd_feed.xml");

/// Serves `feed` as testmember's RSS on the mock Letterboxd.
async fn serve_feed(app: &TestApp, feed: &str) {
	Mock::given(method("GET"))
		.and(path("/testmember/rss/"))
		.respond_with(
			ResponseTemplate::new(200)
				.insert_header("Content-Type", "application/rss+xml;charset=utf-8")
				.set_body_string(feed),
		)
		.mount(&app.letterboxd)
		.await;
}

async fn connect(app: &TestApp, body: serde_json::Value) {
	let response = app.put_connection("letterboxd", &body).await;
	assert_eq!(
		200,
		response.status().as_u16(),
		"Failed to save the connection."
	);
}

async fn films(app: &TestApp) -> Vec<Item> {
	let items: Vec<Item> = app
		.get_items()
		.await
		.json()
		.await
		.expect("Failed to parse items.");
	items
		.into_iter()
		.filter(|item| item.kind == ItemKind::Film)
		.collect()
}

#[tokio::test]
async fn sync_imports_diary_entries_and_reviews() {
	// Arrange
	let app = spawn_admin_app().await;
	serve_feed(&app, FEED).await;
	connect(&app, json!({ "settings": { "username": "TestMember" } })).await;

	// Act
	let response = app.sync_connection("letterboxd").await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	let summary: SyncSummary = response.json().await.expect("Failed to parse summary.");
	assert_eq!(
		summary,
		SyncSummary {
			fetched: 4,
			created: 4,
			updated: 0,
			unchanged: 0,
			skipped: 1
		}
	);

	let films = films(&app).await;
	let titles: Vec<&str> = films.iter().map(|film| film.title.as_str()).collect();
	assert_eq!(
		titles,
		["Perfect Days", "Paterson", "Past Lives", "Aftersun"]
	);

	let perfect_days = &films[0];
	assert_eq!(perfect_days.source, "letterboxd");
	assert_eq!(
		perfect_days.external_id.as_deref(),
		Some("letterboxd-watch-1001")
	);
	assert_eq!(perfect_days.visibility, Visibility::Public);
	assert_eq!(perfect_days.topic, None);
	assert_eq!(
		perfect_days.body, "",
		"a diary entry without a review has no body"
	);
	assert_eq!(
		perfect_days.occurred_at.to_rfc3339(),
		"2026-10-11T12:00:00+00:00"
	);
	assert_eq!(
		perfect_days.created_at, perfect_days.updated_at,
		"imports start untended"
	);
	assert_eq!(
		perfect_days.data,
		json!({
			"year": 2023,
			"rating": 4.5,
			"rewatch": false,
			"liked": true,
			"poster_url": "https://a.ltrbxd.com/resized/film-poster/perfect-days-0-600-0-900-crop.jpg?v=1",
			"letterboxd_url": "https://letterboxd.com/testmember/film/perfect-days/",
			"tmdb_id": 976893,
		})
	);

	let paterson = &films[1];
	assert_eq!(
		paterson.body,
		"<p>Even better the second time.</p><p>The poems!</p>"
	);
	assert_eq!(paterson.data["rewatch"], json!(true));

	let past_lives = &films[2];
	assert!(
		past_lives.data.get("rating").is_none(),
		"unrated entries have no rating"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn sync_records_the_outcome_on_the_connection() {
	// Arrange
	let app = spawn_admin_app().await;
	serve_feed(&app, FEED).await;
	connect(&app, json!({ "settings": { "username": "testmember" } })).await;

	// Act
	app.sync_connection("letterboxd").await;

	// Assert
	let connections: Vec<Connection> = app.get_connections().await.json().await.unwrap();
	assert_eq!(connections.len(), 1);
	let connection = &connections[0];
	assert!(connection.last_synced_at.is_some());
	assert_eq!(connection.last_error, None);
	assert_eq!(
		connection.last_sync_summary.as_ref().map(|s| s.created),
		Some(4)
	);

	app.cleanup().await;
}

#[tokio::test]
async fn syncing_again_updates_letterboxd_facts_but_keeps_local_edits() {
	// Arrange
	let app = spawn_admin_app().await;
	connect(&app, json!({ "settings": { "username": "testmember" } })).await;
	serve_feed(&app, FEED).await;
	app.sync_connection("letterboxd").await;

	// Tend Paterson locally: body and topic. The title and film details are Letterboxd's, so
	// trying to change them does nothing.
	let paterson = films(&app)
		.await
		.into_iter()
		.find(|f| f.title == "Paterson")
		.unwrap();
	let mut data = paterson.data.clone();
	data["year"] = json!(1999);
	data["poster_url"] = json!("https://example.com/other.jpg");
	let tended = json!({
		"kind": "film", "title": "Paterson (again)", "topic": "Jarmusch",
		"body": "<p>My own notes.</p>", "data": data,
	});
	assert_eq!(
		200,
		app.put_item(paterson.id, &tended).await.status().as_u16()
	);

	// Then the rating changes on Letterboxd
	app.letterboxd.reset().await;
	let changed = FEED.replace(
		"<letterboxd:memberRating>4.0</letterboxd:memberRating>",
		"<letterboxd:memberRating>4.5</letterboxd:memberRating>",
	);
	serve_feed(&app, &changed).await;

	// Act
	let response = app.sync_connection("letterboxd").await;

	// Assert
	let summary: SyncSummary = response.json().await.expect("Failed to parse summary.");
	assert_eq!(
		summary,
		SyncSummary {
			fetched: 4,
			created: 0,
			updated: 1,
			unchanged: 3,
			skipped: 1
		}
	);

	let films = films(&app).await;
	assert_eq!(films.len(), 4, "syncing again must not duplicate items");
	let paterson = films.iter().find(|f| f.id == paterson.id).unwrap();
	assert_eq!(
		paterson.data["rating"],
		json!(4.5),
		"facts follow Letterboxd"
	);
	assert_eq!(paterson.title, "Paterson", "the title is Letterboxd's");
	assert_eq!(
		paterson.data["year"],
		json!(2016),
		"so are the film's details"
	);
	assert_eq!(paterson.body, "<p>My own notes.</p>");
	assert_eq!(paterson.topic.as_deref(), Some("Jarmusch"));

	app.cleanup().await;
}

#[tokio::test]
async fn imports_use_the_connections_visibility() {
	// Arrange
	let app = spawn_admin_app().await;
	serve_feed(&app, FEED).await;
	connect(
		&app,
		json!({ "settings": { "username": "testmember" }, "import_visibility": "private" }),
	)
	.await;

	// Act
	app.sync_connection("letterboxd").await;

	// Assert
	assert!(
		films(&app)
			.await
			.iter()
			.all(|f| f.visibility == Visibility::Private)
	);
	app.post_logout().await;
	assert!(
		films(&app).await.is_empty(),
		"visitors don't see private imports"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn sync_reports_an_unknown_letterboxd_user() {
	// Arrange
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.respond_with(ResponseTemplate::new(404))
		.mount(&app.letterboxd)
		.await;
	connect(&app, json!({ "settings": { "username": "nobody_here" } })).await;

	// Act
	let response = app.sync_connection("letterboxd").await;

	// Assert
	assert_eq!(502, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	assert_eq!(
		body["error"],
		"Letterboxd has no user called \"nobody_here\""
	);

	let connections: Vec<Connection> = app.get_connections().await.json().await.unwrap();
	assert_eq!(
		connections[0].last_error.as_deref(),
		Some("Letterboxd has no user called \"nobody_here\"")
	);
	assert!(connections[0].last_synced_at.is_none());

	app.cleanup().await;
}

#[tokio::test]
async fn sync_reports_a_feed_that_isnt_rss() {
	// Arrange
	let app = spawn_admin_app().await;
	serve_feed(&app, "<html>Down for maintenance</html>").await;
	connect(&app, json!({ "settings": { "username": "testmember" } })).await;

	// Act
	let response = app.sync_connection("letterboxd").await;

	// Assert
	assert_eq!(502, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn save_connection_rejects_invalid_settings() {
	// Arrange
	let app = spawn_admin_app().await;
	let test_cases = [
		(
			json!({ "settings": { "username": "../admin" } }),
			"a username with a path in it",
		),
		(
			json!({ "settings": { "username": "" } }),
			"an empty username",
		),
		(json!({ "settings": {} }), "no username"),
		(
			json!({ "settings": { "username": "dave", "extra": 1 } }),
			"an unknown setting",
		),
	];

	for (body, description) in test_cases {
		// Act
		let response = app.put_connection("letterboxd", &body).await;

		// Assert
		assert_eq!(
			400,
			response.status().as_u16(),
			"Expected 400 for {description}."
		);
	}

	app.cleanup().await;
}

#[tokio::test]
async fn unknown_providers_and_missing_connections_are_404() {
	// Arrange
	let app = spawn_admin_app().await;
	let body = json!({ "settings": { "username": "dave" } });

	// Act + Assert
	assert_eq!(
		404,
		app.put_connection("myspace", &body).await.status().as_u16()
	);
	assert_eq!(
		404,
		app.sync_connection("letterboxd").await.status().as_u16()
	);
	assert_eq!(
		404,
		app.delete_connection("letterboxd").await.status().as_u16()
	);

	app.cleanup().await;
}

#[tokio::test]
async fn delete_connection_keeps_imported_items() {
	// Arrange
	let app = spawn_admin_app().await;
	serve_feed(&app, FEED).await;
	connect(&app, json!({ "settings": { "username": "testmember" } })).await;
	app.sync_connection("letterboxd").await;

	// Act
	let response = app.delete_connection("letterboxd").await;

	// Assert
	assert_eq!(204, response.status().as_u16());
	let connections: Vec<Connection> = app.get_connections().await.json().await.unwrap();
	assert!(connections.is_empty());
	assert_eq!(films(&app).await.len(), 4);

	app.cleanup().await;
}

#[tokio::test]
async fn connections_require_a_session() {
	// Arrange
	let app = spawn_admin_app().await;
	app.post_logout().await;
	let body = json!({ "settings": { "username": "dave" } });

	// Act + Assert
	assert_eq!(401, app.get_connections().await.status().as_u16());
	assert_eq!(
		401,
		app.put_connection("letterboxd", &body)
			.await
			.status()
			.as_u16()
	);
	assert_eq!(
		401,
		app.sync_connection("letterboxd").await.status().as_u16()
	);
	assert_eq!(
		401,
		app.delete_connection("letterboxd").await.status().as_u16()
	);

	app.cleanup().await;
}
