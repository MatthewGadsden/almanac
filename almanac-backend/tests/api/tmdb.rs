use almanac::connectors::tmdb::TmdbMovie;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{spawn_app, spawn_app_with};
use crate::items::spawn_admin_app;

const IMAGES: &str = "https://image.tmdb.test/t/p";

#[tokio::test]
async fn tmdb_routes_require_an_admin() {
	let app = spawn_app().await;

	for route in ["/tmdb", "/tmdb/search?query=heat", "/tmdb/movies/949"] {
		let response = app.get(route).await;
		assert_eq!(401, response.status().as_u16(), "{route}");
	}

	app.cleanup().await;
}

#[tokio::test]
async fn tmdb_status_says_whether_a_token_is_set() {
	let app = spawn_admin_app().await;
	let body: serde_json::Value = app.get("/tmdb").await.json().await.unwrap();
	assert_eq!(json!({ "configured": true }), body);
	app.cleanup().await;

	let app = spawn_app_with(|config| config.connectors.tmdb.read_access_token = "".into()).await;
	app.login_admin().await;
	let body: serde_json::Value = app.get("/tmdb").await.json().await.unwrap();
	assert_eq!(json!({ "configured": false }), body);
	app.cleanup().await;
}

#[tokio::test]
async fn search_returns_films_from_tmdb() {
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.and(path("/3/search/movie"))
		.and(header("authorization", "Bearer test-tmdb-token"))
		.and(query_param("query", "perfect days"))
		.and(query_param("include_adult", "false"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"page": 1,
			"results": [
				{
					"id": 976893,
					"title": "Perfect Days",
					"original_title": "PERFECT DAYS",
					"release_date": "2023-11-10",
					"poster_path": "/mjEk5Wwx6TYVqw3zuxmdS5XBjzq.jpg",
					"overview": "Hirayama cleans toilets in Tokyo.",
					"popularity": 30.1
				},
				{
					"id": 1,
					"title": "Perfect Days",
					"original_title": "Perfect Days",
					"release_date": "",
					"poster_path": null,
					"overview": ""
				}
			],
			"total_pages": 1,
			"total_results": 2
		})))
		.expect(1)
		.mount(&app.tmdb)
		.await;

	// Surrounding spaces are trimmed before searching
	let response = app.get("/tmdb/search?query=%20perfect%20days%20").await;

	assert_eq!(200, response.status().as_u16());
	let movies: Vec<TmdbMovie> = response.json().await.unwrap();
	assert_eq!(
		vec![
			TmdbMovie {
				tmdb_id: 976893,
				title: "Perfect Days".into(),
				original_title: Some("PERFECT DAYS".into()),
				year: Some(2023),
				director: None,
				poster_url: Some(format!("{IMAGES}/w500/mjEk5Wwx6TYVqw3zuxmdS5XBjzq.jpg")),
				thumbnail_url: Some(format!("{IMAGES}/w154/mjEk5Wwx6TYVqw3zuxmdS5XBjzq.jpg")),
				overview: "Hirayama cleans toilets in Tokyo.".into(),
			},
			TmdbMovie {
				tmdb_id: 1,
				title: "Perfect Days".into(),
				original_title: None,
				year: None,
				director: None,
				poster_url: None,
				thumbnail_url: None,
				overview: "".into(),
			},
		],
		movies
	);

	app.cleanup().await;
}

#[tokio::test]
async fn search_keeps_the_first_ten_results() {
	let app = spawn_admin_app().await;
	let results: Vec<_> = (1..=20)
		.map(|id| json!({ "id": id, "title": format!("Film {id}") }))
		.collect();
	Mock::given(method("GET"))
		.and(path("/3/search/movie"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({ "results": results })))
		.mount(&app.tmdb)
		.await;

	let movies: Vec<TmdbMovie> = app
		.get("/tmdb/search?query=film")
		.await
		.json()
		.await
		.unwrap();

	let ids: Vec<i64> = movies.iter().map(|movie| movie.tmdb_id).collect();
	assert_eq!((1..=10).collect::<Vec<_>>(), ids);

	app.cleanup().await;
}

#[tokio::test]
async fn search_rejects_an_empty_query_without_calling_tmdb() {
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.respond_with(ResponseTemplate::new(200))
		.expect(0)
		.mount(&app.tmdb)
		.await;

	for route in [
		"/tmdb/search",
		"/tmdb/search?query=",
		"/tmdb/search?query=%20%20",
	] {
		let response = app.get(route).await;
		assert_eq!(400, response.status().as_u16(), "{route}");
	}
	let long = "a".repeat(201);
	let response = app.get(&format!("/tmdb/search?query={long}")).await;
	assert_eq!(400, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn movie_includes_its_directors() {
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.and(path("/3/movie/6977"))
		.and(header("authorization", "Bearer test-tmdb-token"))
		.and(query_param("append_to_response", "credits"))
		.respond_with(ResponseTemplate::new(200).set_body_json(json!({
			"id": 6977,
			"title": "No Country for Old Men",
			"original_title": "No Country for Old Men",
			"release_date": "2007-11-08",
			"poster_path": "/bj1v6YKF8yHqA489VFfnQvOJpnc.jpg",
			"overview": "Llewelyn Moss stumbles upon dead bodies.",
			"runtime": 122,
			"credits": {
				"cast": [{ "name": "Javier Bardem", "character": "Anton Chigurh" }],
				"crew": [
					{ "name": "Joel Coen", "job": "Director", "department": "Directing" },
					{ "name": "Joel Coen", "job": "Screenplay", "department": "Writing" },
					{ "name": "Ethan Coen", "job": "Director", "department": "Directing" },
					{ "name": "Roger Deakins", "job": "Director of Photography", "department": "Camera" }
				]
			}
		})))
		.mount(&app.tmdb)
		.await;

	let response = app.get("/tmdb/movies/6977").await;

	assert_eq!(200, response.status().as_u16());
	let movie: TmdbMovie = response.json().await.unwrap();
	assert_eq!(movie.tmdb_id, 6977);
	assert_eq!(movie.title, "No Country for Old Men");
	assert_eq!(movie.original_title, None);
	assert_eq!(movie.year, Some(2007));
	assert_eq!(movie.director.as_deref(), Some("Joel Coen, Ethan Coen"));
	assert_eq!(
		movie.poster_url,
		Some(format!("{IMAGES}/w500/bj1v6YKF8yHqA489VFfnQvOJpnc.jpg"))
	);

	app.cleanup().await;
}

#[tokio::test]
async fn unknown_movie_is_a_404() {
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.and(path("/3/movie/999999999"))
		.respond_with(ResponseTemplate::new(404).set_body_json(json!({
			"success": false,
			"status_code": 34,
			"status_message": "The resource you requested could not be found."
		})))
		.mount(&app.tmdb)
		.await;

	assert_eq!(
		404,
		app.get("/tmdb/movies/999999999").await.status().as_u16()
	);
	// So are ids that can't exist
	assert_eq!(404, app.get("/tmdb/movies/0").await.status().as_u16());
	assert_eq!(
		404,
		app.get("/tmdb/movies/not-a-number").await.status().as_u16()
	);

	app.cleanup().await;
}

#[tokio::test]
async fn tmdb_failures_are_502s_with_a_reason() {
	let app = spawn_admin_app().await;
	Mock::given(method("GET"))
		.and(path("/3/search/movie"))
		.respond_with(ResponseTemplate::new(401).set_body_json(json!({
			"success": false,
			"status_code": 7,
			"status_message": "Invalid API key: You must be granted a valid key."
		})))
		.mount(&app.tmdb)
		.await;
	Mock::given(method("GET"))
		.and(path("/3/movie/1"))
		.respond_with(ResponseTemplate::new(503))
		.mount(&app.tmdb)
		.await;

	let response = app.get("/tmdb/search?query=heat").await;
	assert_eq!(502, response.status().as_u16());
	let body: serde_json::Value = response.json().await.unwrap();
	assert!(
		body["error"]
			.as_str()
			.unwrap()
			.contains("read access token"),
		"{body}"
	);

	let response = app.get("/tmdb/movies/1").await;
	assert_eq!(502, response.status().as_u16());

	app.cleanup().await;
}

#[tokio::test]
async fn search_without_a_token_is_a_400() {
	let app = spawn_app_with(|config| config.connectors.tmdb.read_access_token = " ".into()).await;
	app.login_admin().await;

	let response = app.get("/tmdb/search?query=heat").await;

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
