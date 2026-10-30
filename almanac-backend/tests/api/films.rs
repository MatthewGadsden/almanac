use almanac::domain::{Item, ItemKind};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::helpers::spawn_app_with;
use crate::items::spawn_admin_app;

const POSTER: &str = "https://image.tmdb.test/t/p/w500/perfect-days.jpg";

/// Serves Perfect Days as TMDB film 976893.
async fn mount_perfect_days(tmdb: &MockServer, expected_calls: u64) {
	Mock::given(method("GET"))
		.and(path("/3/movie/976893"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"id": 976893,
			"title": "Perfect Days",
			"release_date": "2023-11-10",
			"poster_path": "/perfect-days.jpg",
			"credits": { "crew": [{ "name": "Wim Wenders", "job": "Director" }] }
		})))
		.expect(expected_calls)
		.mount(tmdb)
		.await;
}

#[tokio::test]
async fn create_film_takes_its_details_from_tmdb() {
	// Arrange
	let app = spawn_admin_app().await;
	mount_perfect_days(&app.tmdb, 1).await;
	let payload = json!({
		"kind": "film",
		"title": "Not the real title",
		"body": "<p>Quietly wonderful.</p>",
		"occurred_at": "2026-10-04T12:00:00Z",
		"data": {
			"tmdb_id": 976893,
			// Ignored: these come from TMDB
			"year": 1999,
			"director": "Someone",
			"poster_url": "https://example.com/other.jpg",
			"letterboxd_url": "https://letterboxd.com/someone/film/x/",
			// The admin's own
			"rating": 4.5,
			"liked": true,
		},
	});

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());

	let film: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(film.kind, ItemKind::Film);
	assert_eq!(film.title, "Perfect Days");
	assert_eq!(film.topic, None);
	assert_eq!(film.occurred_at.to_rfc3339(), "2026-10-04T12:00:00+00:00");
	assert_eq!(
		film.data,
		json!({
			"year": 2023,
			"director": "Wim Wenders",
			"rating": 4.5,
			"rewatch": false,
			"liked": true,
			"poster_url": POSTER,
			"tmdb_id": 976893,
		})
	);

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_400_without_a_tmdb_id() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload =
		json!({ "kind": "film", "title": "Paterson", "body": "", "data": { "year": 2016 } });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(400, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	assert!(
		body["error"].as_str().unwrap().contains("tmdb_id"),
		"{body}"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_400_for_a_film_tmdb_doesnt_have() {
	// Arrange
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.and(path("/3/movie/999999999"))
		.respond_with(ResponseTemplate::new(404))
		.mount(&app.tmdb)
		.await;
	let payload =
		json!({ "kind": "film", "title": "?", "body": "", "data": { "tmdb_id": 999999999 } });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_400_when_tmdb_isnt_set_up() {
	// Arrange
	let app = spawn_app_with(|config| config.connectors.tmdb.read_access_token = "".into()).await;
	app.login_admin().await;
	let payload =
		json!({ "kind": "film", "title": "?", "body": "", "data": { "tmdb_id": 976893 } });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(400, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	assert!(
		body["error"]
			.as_str()
			.unwrap()
			.contains("TMDB isn't set up")
	);

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_502_when_tmdb_fails() {
	// Arrange
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.respond_with(ResponseTemplate::new(503))
		.mount(&app.tmdb)
		.await;
	let payload =
		json!({ "kind": "film", "title": "?", "body": "", "data": { "tmdb_id": 976893 } });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(502, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_400_for_invalid_details() {
	// Arrange
	let app = spawn_admin_app().await;
	// Validation happens before TMDB is asked
	mount_perfect_days(&app.tmdb, 0).await;
	let test_cases = [
		(json!({ "rating": 0 }), "a rating below half a star"),
		(json!({ "rating": 5.5 }), "a rating above five stars"),
		(json!({ "rating": 3.3 }), "a rating that isn't a half star"),
		(json!({ "tmdb_id": -1 }), "a negative tmdb_id"),
		(json!({ "runtime": 120 }), "an unknown field"),
		(json!("not an object"), "data that isn't an object"),
	];

	for (data, description) in test_cases {
		// Act
		let mut data = data;
		if let Some(object) = data.as_object_mut() {
			object.entry("tmdb_id").or_insert(json!(976893));
		}
		let payload = json!({ "kind": "film", "title": "Paterson", "body": "", "data": data });
		let response = app.post_item(&payload).await;

		// Assert
		assert_eq!(
			400,
			response.status().as_u16(),
			"The API did not fail with 400 Bad Request for {description}."
		);
	}

	app.cleanup().await;
}

#[tokio::test]
async fn update_film_keeps_its_tmdb_details_and_changes_the_rest() {
	// Arrange: TMDB is only asked once, when the film is logged
	let app = spawn_admin_app().await;
	mount_perfect_days(&app.tmdb, 1).await;
	let created: Item = app
		.post_item(&json!({
			"kind": "film", "title": "Perfect Days", "body": "",
			"data": { "tmdb_id": 976893, "rating": 4.0 },
		}))
		.await
		.json()
		.await
		.unwrap();

	// Act
	let response = app
		.put_item(
			created.id,
			&json!({
				"kind": "film", "title": "Renamed", "topic": "Wenders", "body": "<p>Even better.</p>",
				"data": {
					"tmdb_id": 976893, "year": 1990, "director": "Me",
					"rating": 5.0, "rewatch": true,
				},
			}),
		)
		.await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	let film: Item = response.json().await.unwrap();
	assert_eq!(film.title, "Perfect Days");
	assert_eq!(film.topic.as_deref(), Some("Wenders"));
	assert_eq!(film.body, "<p>Even better.</p>");
	assert_eq!(
		film.data,
		json!({
			"year": 2023,
			"director": "Wim Wenders",
			"rating": 5.0,
			"rewatch": true,
			"liked": false,
			"poster_url": POSTER,
			"tmdb_id": 976893,
		})
	);

	app.cleanup().await;
}

#[tokio::test]
async fn update_film_with_a_different_tmdb_id_takes_that_films_details() {
	// Arrange
	let app = spawn_admin_app().await;
	mount_perfect_days(&app.tmdb, 1).await;
	Mock::given(method("GET"))
		.and(path("/3/movie/370755"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"id": 370755,
			"title": "Paterson",
			"release_date": "2016-11-17",
			"poster_path": null,
			"credits": { "crew": [{ "name": "Jim Jarmusch", "job": "Director" }] }
		})))
		.expect(1)
		.mount(&app.tmdb)
		.await;
	let created: Item = app
		.post_item(&json!({
			"kind": "film", "title": "Perfect Days", "body": "", "data": { "tmdb_id": 976893 },
		}))
		.await
		.json()
		.await
		.unwrap();

	// Act
	let response = app
		.put_item(
			created.id,
			&json!({ "kind": "film", "title": "x", "body": "", "data": { "tmdb_id": 370755 } }),
		)
		.await;

	// Assert
	let film: Item = response.json().await.unwrap();
	assert_eq!(film.title, "Paterson");
	assert_eq!(
		film.data,
		json!({
			"year": 2016,
			"director": "Jim Jarmusch",
			"rewatch": false,
			"liked": false,
			"tmdb_id": 370755,
		})
	);

	app.cleanup().await;
}

#[tokio::test]
async fn update_film_logged_before_tmdb_needs_linking() {
	// Arrange: a hand-logged film from before TMDB, with typed-in details
	let app = spawn_admin_app().await;
	mount_perfect_days(&app.tmdb, 1).await;
	let id = uuid::Uuid::new_v4();
	sqlx::query!(
		r#"INSERT INTO items (id, kind, title, body, data, occurred_at, visibility, source,
			created_at, updated_at)
		VALUES ($1, 'film', 'perfect days', '', $2, now(), 'public', 'manual', now(), now())"#,
		id,
		json!({ "year": 2023, "director": "Test", "liked": false, "rewatch": false }),
	)
	.execute(&app.db_pool)
	.await
	.unwrap();

	// Act
	let unlinked = app
		.put_item(
			id,
			&json!({ "kind": "film", "title": "perfect days", "body": "", "data": {} }),
		)
		.await;
	let linked = app
		.put_item(
			id,
			&json!({ "kind": "film", "title": "perfect days", "body": "", "data": { "tmdb_id": 976893 } }),
		)
		.await;

	// Assert
	assert_eq!(400, unlinked.status().as_u16());
	assert_eq!(200, linked.status().as_u16());
	let film: Item = linked.json().await.unwrap();
	assert_eq!(film.title, "Perfect Days");
	assert_eq!(film.data["director"], "Wim Wenders");

	app.cleanup().await;
}
