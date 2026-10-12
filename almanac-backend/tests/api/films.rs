use almanac::domain::{Item, ItemKind};
use serde_json::json;

use crate::items::spawn_admin_app;

#[tokio::test]
async fn create_film_returns_201_with_its_details() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload = json!({
		"kind": "film",
		"title": "Perfect Days",
		"body": "<p>Quietly wonderful.</p>",
		"occurred_at": "2026-10-04T12:00:00Z",
		"data": {
			"year": 2023,
			"director": "Wim Wenders",
			"rating": 4.5,
			"liked": true,
			"poster_url": "https://example.com/perfect-days.jpg",
		},
	});

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());

	let film: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(film.kind, ItemKind::Film);
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
			"poster_url": "https://example.com/perfect-days.jpg",
		})
	);

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_accepts_no_details() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload = json!({ "kind": "film", "title": "Paterson", "body": "" });

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());

	let film: Item = response.json().await.expect("Failed to parse item.");
	assert_eq!(film.data, json!({ "rewatch": false, "liked": false }));

	app.cleanup().await;
}

#[tokio::test]
async fn create_film_returns_400_for_invalid_details() {
	// Arrange
	let app = spawn_admin_app().await;
	let test_cases = [
		(json!({ "rating": 0 }), "a rating below half a star"),
		(json!({ "rating": 5.5 }), "a rating above five stars"),
		(json!({ "rating": 3.3 }), "a rating that isn't a half star"),
		(json!({ "year": 1500 }), "an implausible year"),
		(
			json!({ "poster_url": "javascript:alert(1)" }),
			"a non-http poster URL",
		),
		(json!({ "runtime": 120 }), "an unknown field"),
		(json!("not an object"), "data that isn't an object"),
	];

	for (data, description) in test_cases {
		// Act
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
