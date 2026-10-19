use almanac::connectors::{Connection, SyncSummary};
use almanac::domain::{Item, ItemKind, Visibility};
use chrono::Utc;
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, header_exists, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{TestApp, spawn_app_with};
use crate::items::spawn_admin_app;

/// The `state` from the authorise URL the API hands out.
async fn authorize_state(app: &TestApp) -> String {
	let body: serde_json::Value = app.get_strava_authorize().await.json().await.unwrap();
	let url = reqwest::Url::parse(body["url"].as_str().unwrap()).unwrap();
	url.query_pairs()
		.find(|(key, _)| key == "state")
		.map(|(_, value)| value.into_owned())
		.expect("authorise URL has a state")
}

/// Runs the whole OAuth flow against the mock Strava. Tokens expire at `expires_at`.
async fn connect_strava(app: &TestApp, expires_at: i64) -> Connection {
	Mock::given(method("POST"))
		.and(path("/oauth/token"))
		.and(body_string_contains("grant_type=authorization_code"))
		.and(body_string_contains("code=code-from-strava"))
		.and(body_string_contains("client_secret=test-secret"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"token_type": "Bearer",
			"access_token": "access-1",
			"refresh_token": "refresh-1",
			"expires_at": expires_at,
			"expires_in": 21600,
			"athlete": { "id": 4242, "firstname": "Ada", "lastname": "Lovelace" },
		})))
		.mount(&app.strava)
		.await;

	let state = authorize_state(app).await;
	let response = app
		.post_strava_callback(&json!({
			"code": "code-from-strava",
			"state": state,
			"scope": "read,activity:read_all",
		}))
		.await;
	assert_eq!(200, response.status().as_u16(), "Strava callback failed.");

	response.json().await.unwrap()
}

fn in_six_hours() -> i64 {
	Utc::now().timestamp() + 6 * 60 * 60
}

fn activities() -> serde_json::Value {
	json!([
		{
			"id": 9001, "name": "Morning Run", "sport_type": "Run", "type": "Run",
			"start_date": "2026-10-12T19:30:00Z", "start_date_local": "2026-10-13T06:30:00Z",
			"distance": 10234.5, "moving_time": 3125, "elapsed_time": 3300,
			"total_elevation_gain": 85.0, "average_heartrate": 151.2, "max_heartrate": 178,
			"map": { "summary_polyline": "_p~iF~ps|U_ulLnnqC_mqNvxq`@" }, "private": false,
		},
		{
			"id": 9002, "name": "Leg day", "sport_type": "WeightTraining", "type": "WeightTraining",
			"start_date": "2026-10-10T07:00:00Z", "distance": 0.0, "moving_time": 2700,
			"elapsed_time": 2700, "total_elevation_gain": 0, "map": { "summary_polyline": "" },
		},
	])
}

/// The outline of the Morning Run's polyline: its shape in a 100×100 box, no coordinates.
const ROUTE_OUTLINE: [[u8; 2]; 3] = [[100, 100], [88, 54], [0, 0]];

async fn serve_activities(app: &TestApp, token: &str, activities: serde_json::Value) {
	Mock::given(method("GET"))
		.and(path("/api/v3/athlete/activities"))
		.and(header("Authorization", format!("Bearer {token}").as_str()))
		.respond_with(ResponseTemplate::new(200).set_body_json(activities))
		.mount(&app.strava)
		.await;
}

async fn activity_items(app: &TestApp) -> Vec<Item> {
	let items: Vec<Item> = app.get_items().await.json().await.unwrap();
	items
		.into_iter()
		.filter(|item| item.kind == ItemKind::Activity)
		.collect()
}

#[tokio::test]
async fn authorize_returns_the_strava_url() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let response = app.get_strava_authorize().await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	let url = reqwest::Url::parse(body["url"].as_str().unwrap()).unwrap();
	assert_eq!(url.path(), "/oauth/authorize");
	let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
	assert_eq!(query["client_id"], "test-client");
	assert_eq!(
		query["redirect_uri"],
		"http://localhost:5173/admin/strava/callback"
	);
	assert_eq!(query["response_type"], "code");
	assert_eq!(query["scope"], "read,activity:read_all");
	assert!(!query["state"].is_empty());

	app.cleanup().await;
}

#[tokio::test]
async fn authorize_explains_when_strava_isnt_configured() {
	// Arrange
	let app = spawn_app_with(|config| config.connectors.strava.client_id = String::new()).await;
	app.login_admin().await;

	// Act
	let response = app.get_strava_authorize().await;

	// Assert
	assert_eq!(400, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	assert!(
		body["error"]
			.as_str()
			.unwrap()
			.contains("Strava isn't set up")
	);

	app.cleanup().await;
}

#[tokio::test]
async fn callback_connects_strava_without_exposing_tokens() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let connection = connect_strava(&app, in_six_hours()).await;

	// Assert
	assert_eq!(
		connection.settings,
		json!({ "athlete_id": 4242, "name": "Ada Lovelace" })
	);
	assert_eq!(connection.import_visibility, Visibility::Private);

	let listed = app.get_connections().await.text().await.unwrap();
	assert!(listed.contains("Ada Lovelace"));
	assert!(
		!listed.contains("access-1") && !listed.contains("refresh-1"),
		"tokens must not leak"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn callback_rejects_a_state_it_didnt_issue() {
	// Arrange
	let app = spawn_admin_app().await;
	let real_state = authorize_state(&app).await;
	let callback =
		|state: &str| json!({ "code": "c", "state": state, "scope": "activity:read_all" });

	// Act + Assert: a forged state, then a reused one
	assert_eq!(
		400,
		app.post_strava_callback(&callback("forged"))
			.await
			.status()
			.as_u16()
	);
	assert_eq!(
		400,
		app.post_strava_callback(&callback(&real_state))
			.await
			.status()
			.as_u16(),
		"a state is single-use, and the failed attempt above consumed it"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn callback_requires_permission_to_read_activities() {
	// Arrange
	let app = spawn_admin_app().await;
	let state = authorize_state(&app).await;

	// Act
	let response = app
		.post_strava_callback(&json!({ "code": "c", "state": state, "scope": "read" }))
		.await;

	// Assert
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn sync_imports_activities_as_private_items() {
	// Arrange
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	serve_activities(&app, "access-1", activities()).await;

	// Act
	let response = app.sync_connection("strava").await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	let summary: SyncSummary = response.json().await.unwrap();
	assert_eq!((summary.fetched, summary.created), (2, 2));

	let activities = activity_items(&app).await;
	let run = activities
		.iter()
		.find(|a| a.title == "Morning Run")
		.unwrap();
	assert_eq!(run.source, "strava");
	assert_eq!(run.external_id.as_deref(), Some("9001"));
	assert_eq!(run.visibility, Visibility::Private);
	assert_eq!(run.occurred_at.to_rfc3339(), "2026-10-12T19:30:00+00:00");
	assert_eq!(
		run.data,
		json!({
			"sport": "Run",
			"distance_m": 10234.5,
			"moving_time_s": 3125,
			"elapsed_time_s": 3300,
			"elevation_gain_m": 85.0,
			"average_heartrate": 151.2,
			"max_heartrate": 178.0,
			"strava_url": "https://www.strava.com/activities/9001",
			"route_outline": ROUTE_OUTLINE,
		})
	);

	let legs = activities.iter().find(|a| a.title == "Leg day").unwrap();
	assert!(
		legs.data.get("distance_m").is_none(),
		"0 m isn't a distance"
	);
	assert!(
		legs.data.get("route_outline").is_none(),
		"no GPS, no outline"
	);

	app.post_logout().await;
	assert!(
		activity_items(&app).await.is_empty(),
		"visitors never see Strava data"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn sync_continues_from_the_newest_activity() {
	// Arrange
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	serve_activities(&app, "access-1", activities()).await;
	app.sync_connection("strava").await;
	app.strava.reset().await;

	// The newest activity started at 2026-10-12T19:30:00Z; the next sync overlaps a week before it
	let newest = chrono::DateTime::parse_from_rfc3339("2026-10-12T19:30:00Z").unwrap();
	let expected_after = newest.timestamp() - 7 * 24 * 60 * 60;
	Mock::given(method("GET"))
		.and(path("/api/v3/athlete/activities"))
		.and(query_param("after", expected_after.to_string().as_str()))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
		.expect(1)
		.mount(&app.strava)
		.await;

	// Act
	let response = app.sync_connection("strava").await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	app.strava.verify().await;
	assert_eq!(
		activity_items(&app).await.len(),
		2,
		"nothing is lost or duplicated"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn an_outdated_cursor_rereads_every_activity_once() {
	// Arrange: a cursor saved before imports had route outlines
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	sqlx::query("UPDATE connections SET sync_cursor = $1 WHERE provider = 'strava'")
		.bind(json!({ "after": 1_760_000_000 }))
		.execute(&app.db_pool)
		.await
		.unwrap();
	Mock::given(method("GET"))
		.and(path("/api/v3/athlete/activities"))
		.and(query_param("after", "0"))
		.respond_with(ResponseTemplate::new(200).set_body_json(activities()))
		.expect(1)
		.mount(&app.strava)
		.await;

	// Act
	let response = app.sync_connection("strava").await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	app.strava.verify().await;
	let cursor: serde_json::Value =
		sqlx::query_scalar("SELECT sync_cursor FROM connections WHERE provider = 'strava'")
			.fetch_one(&app.db_pool)
			.await
			.unwrap();
	assert_eq!(cursor["version"], 1, "the next sync is incremental again");

	app.cleanup().await;
}

#[tokio::test]
async fn sync_refreshes_an_expiring_token_and_keeps_the_new_one() {
	// Arrange: tokens that have already expired
	let app = spawn_admin_app().await;
	connect_strava(&app, Utc::now().timestamp() - 60).await;
	Mock::given(method("POST"))
		.and(path("/oauth/token"))
		.and(body_string_contains("grant_type=refresh_token"))
		.and(body_string_contains("refresh_token=refresh-1"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"token_type": "Bearer",
			"access_token": "access-2",
			"refresh_token": "refresh-2",
			"expires_at": in_six_hours(),
			"expires_in": 21600,
		})))
		.expect(1)
		.mount(&app.strava)
		.await;
	serve_activities(&app, "access-2", activities()).await;

	// Act
	let response = app.sync_connection("strava").await;

	// Assert
	assert_eq!(200, response.status().as_u16());
	app.strava.verify().await;

	let stored =
		sqlx::query_scalar!("SELECT credentials FROM connections WHERE provider = 'strava'")
			.fetch_one(&app.db_pool)
			.await
			.unwrap()
			.unwrap();
	assert_eq!(
		stored["refresh_token"], "refresh-2",
		"Strava rotates refresh tokens"
	);

	app.cleanup().await;
}

#[tokio::test]
async fn sync_reports_revoked_access() {
	// Arrange
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	Mock::given(method("GET"))
		.and(path("/api/v3/athlete/activities"))
		.respond_with(ResponseTemplate::new(401))
		.mount(&app.strava)
		.await;

	// Act
	let response = app.sync_connection("strava").await;

	// Assert
	assert_eq!(502, response.status().as_u16());
	let connections: Vec<Connection> = app.get_connections().await.json().await.unwrap();
	assert!(
		connections[0]
			.last_error
			.as_deref()
			.unwrap()
			.contains("connect Strava again")
	);

	app.cleanup().await;
}

#[tokio::test]
async fn strava_activities_cannot_be_made_public() {
	// Arrange
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	serve_activities(&app, "access-1", activities()).await;
	app.sync_connection("strava").await;
	let run = activity_items(&app)
		.await
		.into_iter()
		.find(|a| a.title == "Morning Run")
		.unwrap();
	let edit = |visibility: &str| {
		json!({
			"kind": "activity", "title": "Morning Run", "body": "<p>Felt good.</p>",
			"data": run.data, "visibility": visibility,
		})
	};

	// Act + Assert
	assert_eq!(
		400,
		app.put_item(run.id, &edit("public"))
			.await
			.status()
			.as_u16()
	);
	assert_eq!(
		200,
		app.put_item(run.id, &edit("private"))
			.await
			.status()
			.as_u16()
	);

	app.cleanup().await;
}

#[tokio::test]
async fn strava_settings_cant_be_set_by_hand() {
	// Arrange
	let app = spawn_admin_app().await;

	// Act
	let response = app
		.put_connection(
			"strava",
			&json!({ "settings": { "athlete_id": 1, "name": "Me" } }),
		)
		.await;

	// Assert
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn disconnecting_revokes_strava_access_and_keeps_activities() {
	// Arrange
	let app = spawn_admin_app().await;
	connect_strava(&app, in_six_hours()).await;
	serve_activities(&app, "access-1", activities()).await;
	app.sync_connection("strava").await;
	Mock::given(method("POST"))
		.and(path("/oauth/revoke"))
		.and(header_exists("Authorization"))
		.and(body_string_contains("token=refresh-1"))
		.respond_with(ResponseTemplate::new(200))
		.expect(1)
		.mount(&app.strava)
		.await;

	// Act
	let response = app.delete_connection("strava").await;

	// Assert
	assert_eq!(204, response.status().as_u16());
	app.strava.verify().await;
	let connections: Vec<Connection> = app.get_connections().await.json().await.unwrap();
	assert!(connections.is_empty());
	assert_eq!(activity_items(&app).await.len(), 2);

	app.cleanup().await;
}

#[tokio::test]
async fn strava_endpoints_require_a_session() {
	// Arrange
	let app = spawn_admin_app().await;
	app.post_logout().await;

	// Act + Assert
	assert_eq!(401, app.get_strava_authorize().await.status().as_u16());
	let callback = json!({ "code": "c", "state": "s", "scope": "activity:read_all" });
	assert_eq!(
		401,
		app.post_strava_callback(&callback).await.status().as_u16()
	);

	app.cleanup().await;
}
