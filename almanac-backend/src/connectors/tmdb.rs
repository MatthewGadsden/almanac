//! The Movie Database (TMDB): looks films up while logging them by hand, so the title, year,
//! director, poster and `tmdb_id` come from one place. It isn't a provider: nothing is synced
//! from it, and it needs no connection row, only `connectors.tmdb.read_access_token`.
//! See https://developer.themoviedb.org/reference/search-movie and /reference/movie-details.
//!
//! TMDB's terms ask for the notice "This product uses the TMDB API but is not endorsed or
//! certified by TMDB." and links back to themoviedb.org; the frontend shows both.

use super::ConnectorContext;
use crate::error::ApiError;

pub const NOT_CONFIGURED: &str = "TMDB isn't set up: add your API read access token to \
	connectors.tmdb.read_access_token (see the backend README)";

/// Search results returned; TMDB sends 20 a page, but the first few are what's wanted.
const MAX_RESULTS: usize = 10;
/// Long enough for any title, short enough to keep junk out of TMDB's query string.
const MAX_QUERY_CHARS: usize = 200;
/// Poster sizes TMDB serves: w500 for film pages and tiles, w154 for the search list.
const POSTER_SIZE: &str = "w500";
const THUMBNAIL_SIZE: &str = "w154";

/// A film as Almanac's film form uses it. `director` is only filled in by `movie`, because
/// search results don't include credits.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TmdbMovie {
	pub tmdb_id: i64,
	pub title: String,
	/// Only when it differs from `title`, e.g. a film's title in its own language.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub original_title: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub year: Option<i32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub director: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub poster_url: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub thumbnail_url: Option<String>,
	#[serde(default)]
	pub overview: String,
}

#[derive(serde::Deserialize)]
struct SearchResponse {
	results: Vec<MovieResponse>,
}

/// The fields Almanac reads from a search result or `/movie/{id}`.
#[derive(serde::Deserialize)]
struct MovieResponse {
	id: i64,
	title: String,
	#[serde(default)]
	original_title: Option<String>,
	/// `YYYY-MM-DD`, or empty when unknown.
	#[serde(default)]
	release_date: Option<String>,
	#[serde(default)]
	poster_path: Option<String>,
	#[serde(default)]
	overview: Option<String>,
	/// Only on `/movie/{id}?append_to_response=credits`.
	#[serde(default)]
	credits: Option<Credits>,
}

#[derive(serde::Deserialize)]
struct Credits {
	#[serde(default)]
	crew: Vec<CrewMember>,
}

#[derive(serde::Deserialize)]
struct CrewMember {
	name: String,
	job: String,
}

impl MovieResponse {
	fn into_movie(self, image_base_url: &str) -> TmdbMovie {
		let image = |size: &str| {
			self.poster_path
				.as_deref()
				.filter(|path| path.starts_with('/'))
				.map(|path| format!("{}/{size}{path}", image_base_url.trim_end_matches('/')))
		};
		let poster_url = image(POSTER_SIZE);
		let thumbnail_url = image(THUMBNAIL_SIZE);

		// Out-of-range years would fail FilmData's checks when the film is saved
		let year = self
			.release_date
			.as_deref()
			.and_then(|date| date.get(..4))
			.and_then(|year| year.parse().ok())
			.filter(|year| (1870..=2100).contains(year));

		// Co-directed films (e.g. the Coens) list each director
		let mut directors: Vec<String> = Vec::new();
		for member in self.credits.map(|credits| credits.crew).unwrap_or_default() {
			if member.job == "Director" && !directors.contains(&member.name) {
				directors.push(member.name);
			}
		}

		TmdbMovie {
			tmdb_id: self.id,
			original_title: self
				.original_title
				.filter(|original| !original.trim().is_empty() && *original != self.title),
			title: self.title,
			year,
			director: (!directors.is_empty()).then(|| directors.join(", ")),
			poster_url,
			thumbnail_url,
			overview: self.overview.unwrap_or_default(),
		}
	}
}

/// Films matching `query`, most relevant first (TMDB's order).
pub async fn search(context: &ConnectorContext, query: &str) -> Result<Vec<TmdbMovie>, ApiError> {
	let query = query.trim();
	if query.is_empty() {
		return Err(ApiError::Validation("query must not be empty".into()));
	}
	if query.chars().count() > MAX_QUERY_CHARS {
		return Err(ApiError::Validation(format!(
			"query must be at most {MAX_QUERY_CHARS} characters"
		)));
	}

	let response: SearchResponse = get(
		context,
		"/search/movie",
		&[("query", query), ("include_adult", "false"), ("page", "1")],
	)
	.await?;

	let image_base_url = &context.settings.tmdb.image_base_url;
	Ok(response
		.results
		.into_iter()
		.take(MAX_RESULTS)
		.map(|movie| movie.into_movie(image_base_url))
		.collect())
}

/// One film with its director(s). `NotFound` when TMDB has no film with that id.
pub async fn movie(context: &ConnectorContext, tmdb_id: i64) -> Result<TmdbMovie, ApiError> {
	if tmdb_id <= 0 {
		return Err(ApiError::NotFound);
	}

	let response: MovieResponse = get(
		context,
		&format!("/movie/{tmdb_id}"),
		&[("append_to_response", "credits")],
	)
	.await?;

	Ok(response.into_movie(&context.settings.tmdb.image_base_url))
}

async fn get<T: serde::de::DeserializeOwned>(
	context: &ConnectorContext,
	path: &str,
	query: &[(&str, &str)],
) -> Result<T, ApiError> {
	let config = &context.settings.tmdb;
	if !config.is_configured() {
		return Err(ApiError::Validation(NOT_CONFIGURED.into()));
	}

	let url = format!("{}{path}", config.api_base_url.trim_end_matches('/'));
	let response = context
		.http
		.get(&url)
		.bearer_auth(config.read_access_token.trim())
		.query(query)
		.send()
		.await
		.map_err(|e| ApiError::Upstream(format!("Couldn't reach TMDB: {e}")))?;

	let status = response.status();
	match status {
		reqwest::StatusCode::NOT_FOUND => return Err(ApiError::NotFound),
		reqwest::StatusCode::UNAUTHORIZED => {
			return Err(ApiError::Upstream(
				"TMDB rejected the read access token in connectors.tmdb.read_access_token".into(),
			));
		}
		reqwest::StatusCode::TOO_MANY_REQUESTS => {
			return Err(ApiError::Upstream(
				"TMDB is rate limiting requests; try again in a few seconds".into(),
			));
		}
		_ if !status.is_success() => {
			return Err(ApiError::Upstream(format!("TMDB responded with {status}")));
		}
		_ => {}
	}

	response
		.json()
		.await
		.map_err(|e| ApiError::Upstream(format!("Couldn't read TMDB's response: {e}")))
}
