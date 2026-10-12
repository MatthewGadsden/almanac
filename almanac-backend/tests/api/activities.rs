use almanac::domain::{Item, ItemKind, Visibility};
use serde_json::json;

use crate::items::spawn_admin_app;

#[tokio::test]
async fn create_activity_by_hand() {
	// Arrange
	let app = spawn_admin_app().await;
	let payload = json!({
		"kind": "activity",
		"title": "Parkrun",
		"body": "",
		"occurred_at": "2026-10-10T21:00:00Z",
		"visibility": "public",
		"data": { "sport": "Run", "distance_m": 5000, "moving_time_s": 1380, "average_heartrate": 162 },
	});

	// Act
	let response = app.post_item(&payload).await;

	// Assert
	assert_eq!(201, response.status().as_u16());
	let activity: Item = response.json().await.unwrap();
	assert_eq!(activity.kind, ItemKind::Activity);
	assert_eq!(activity.source, "manual");
	assert_eq!(
		activity.visibility,
		Visibility::Public,
		"hand-logged activities can be public"
	);
	assert_eq!(
		activity.data,
		json!({ "sport": "Run", "distance_m": 5000.0, "moving_time_s": 1380, "average_heartrate": 162.0 })
	);

	app.cleanup().await;
}

#[tokio::test]
async fn create_activity_rejects_invalid_details() {
	// Arrange
	let app = spawn_admin_app().await;
	let test_cases = [
		(json!({}), "no sport"),
		(json!({ "sport": "" }), "a blank sport"),
		(json!({ "sport": "Run; DROP" }), "a sport with punctuation"),
		(
			json!({ "sport": "Run", "distance_m": -1 }),
			"a negative distance",
		),
		(
			json!({ "sport": "Run", "moving_time_s": -5 }),
			"a negative time",
		),
		(
			json!({ "sport": "Run", "average_heartrate": 300 }),
			"an implausible heart rate",
		),
		(
			json!({ "sport": "Run", "strava_url": "javascript:alert(1)" }),
			"a non-https link",
		),
		(json!({ "sport": "Run", "pace": 5 }), "an unknown field"),
	];

	for (data, description) in test_cases {
		// Act
		let payload = json!({ "kind": "activity", "title": "Run", "body": "", "data": data });
		let response = app.post_item(&payload).await;

		// Assert
		assert_eq!(
			400,
			response.status().as_u16(),
			"Expected 400 for {description}."
		);
	}

	app.cleanup().await;
}
