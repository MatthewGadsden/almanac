//! Letterboxd has no open API, but every profile has a public RSS feed of its 50 most recent
//! activities: diary entries (`letterboxd-watch-…`), reviews (`letterboxd-review-…`) and lists
//! (`letterboxd-list-…`). Film fields come from the `letterboxd:` and `tmdb:` namespaces; the
//! description is a poster `<p><img>` followed by the review, or "Watched on <date>." when
//! there isn't one.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

use super::{Connection, ConnectorContext, ImportedItem, SyncError};
use crate::domain::{FilmData, ItemKind};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LetterboxdSettings {
	pub username: String,
}

impl LetterboxdSettings {
	pub fn parse(settings: serde_json::Value) -> Result<Self, String> {
		let mut settings: LetterboxdSettings =
			serde_json::from_value(settings).map_err(|e| format!("invalid settings: {e}"))?;

		// Usernames are letters, digits and underscores, and case-insensitive in URLs.
		// Checking this also keeps the username from changing the feed URL's path.
		settings.username = settings.username.trim().to_lowercase();
		let valid = (1..=40).contains(&settings.username.len())
			&& settings
				.username
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '_');
		if !valid {
			return Err("username must be a Letterboxd username (letters, digits and _)".into());
		}

		Ok(settings)
	}

	pub fn to_value(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("LetterboxdSettings always serialises")
	}
}

/// One diary entry or review from the feed.
#[derive(Debug, PartialEq)]
pub struct LetterboxdEntry {
	/// e.g. `letterboxd-watch-1534807134`; stable, so it's the item's external id.
	pub guid: String,
	pub title: String,
	pub year: Option<i32>,
	pub rating: Option<f64>,
	pub liked: bool,
	pub rewatch: bool,
	pub watched_on: Option<NaiveDate>,
	pub published_at: Option<DateTime<Utc>>,
	pub url: Option<String>,
	pub tmdb_id: Option<i64>,
	pub poster_url: Option<String>,
	/// The review as HTML, or empty for a diary entry without one.
	pub review_html: String,
}

impl LetterboxdEntry {
	fn into_imported(self) -> Result<ImportedItem, String> {
		// The same midday-UTC convention as dates picked in the frontend
		let occurred_at = self
			.watched_on
			.map(|date| {
				date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap())
					.and_utc()
			})
			.or(self.published_at)
			.unwrap_or_else(Utc::now);

		let film = FilmData {
			year: self.year,
			director: None,
			rating: self.rating,
			rewatch: self.rewatch,
			liked: self.liked,
			poster_url: self.poster_url,
			letterboxd_url: self.url,
			tmdb_id: self.tmdb_id,
		};
		// Imported data has to pass the same checks as data entered by hand
		let data = FilmData::parse(film.to_value())?.to_value();

		Ok(ImportedItem {
			kind: ItemKind::Film,
			external_id: self.guid,
			title: self.title,
			body: self.review_html,
			data,
			occurred_at,
		})
	}
}

/// Fetches the user's feed and converts it. Returns the items and how many entries were skipped.
pub(super) async fn fetch(
	connection: &Connection,
	context: &ConnectorContext,
) -> Result<(Vec<ImportedItem>, usize), SyncError> {
	let settings =
		LetterboxdSettings::parse(connection.settings.clone()).map_err(SyncError::Invalid)?;
	let url = format!(
		"{}/{}/rss/",
		context.settings.letterboxd_base_url.trim_end_matches('/'),
		settings.username
	);

	let response = context
		.http
		.get(&url)
		.send()
		.await
		.map_err(|e| SyncError::Fetch(format!("Couldn't reach Letterboxd: {e}")))?;

	let status = response.status();
	if status == reqwest::StatusCode::NOT_FOUND {
		return Err(SyncError::Fetch(format!(
			"Letterboxd has no user called \"{}\"",
			settings.username
		)));
	}
	if !status.is_success() {
		return Err(SyncError::Fetch(format!(
			"Letterboxd responded with {status}"
		)));
	}

	let xml = response
		.text()
		.await
		.map_err(|e| SyncError::Fetch(format!("Couldn't read Letterboxd's response: {e}")))?;
	let (entries, mut skipped) = parse_feed(&xml).map_err(SyncError::Invalid)?;

	let mut items = Vec::with_capacity(entries.len());
	for entry in entries {
		match entry.into_imported() {
			Ok(item) => items.push(item),
			Err(e) => {
				eprintln!("Skipping a Letterboxd entry: {e}");
				skipped += 1;
			}
		}
	}

	Ok((items, skipped))
}

/// Diary entries and reviews from a Letterboxd RSS feed, plus how many other entries (lists
/// and anything unrecognised) were left out.
pub fn parse_feed(xml: &str) -> Result<(Vec<LetterboxdEntry>, usize), String> {
	let channel = rss::Channel::read_from(xml.as_bytes())
		.map_err(|e| format!("Couldn't read the Letterboxd feed: {e}"))?;

	let mut entries = Vec::new();
	let mut skipped = 0;
	for item in channel.items() {
		match parse_item(item) {
			Some(entry) => entries.push(entry),
			None => skipped += 1,
		}
	}

	Ok((entries, skipped))
}

fn parse_item(item: &rss::Item) -> Option<LetterboxdEntry> {
	let guid = item.guid()?.value().to_string();
	if !(guid.starts_with("letterboxd-watch-") || guid.starts_with("letterboxd-review-")) {
		return None;
	}

	let title = extension(item, "letterboxd", "filmTitle")?
		.trim()
		.to_string();
	if title.is_empty() {
		return None;
	}

	let (poster_url, review_html) = split_description(item.description().unwrap_or_default());

	Some(LetterboxdEntry {
		guid,
		title,
		year: extension(item, "letterboxd", "filmYear").and_then(|v| v.trim().parse().ok()),
		rating: extension(item, "letterboxd", "memberRating").and_then(|v| v.trim().parse().ok()),
		liked: extension(item, "letterboxd", "memberLike").is_some_and(is_yes),
		rewatch: extension(item, "letterboxd", "rewatch").is_some_and(is_yes),
		watched_on: extension(item, "letterboxd", "watchedDate")
			.and_then(|v| NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").ok()),
		published_at: item
			.pub_date()
			.and_then(|v| DateTime::parse_from_rfc2822(v).ok())
			.map(|date| date.with_timezone(&Utc)),
		url: item.link().map(str::to_string),
		tmdb_id: extension(item, "tmdb", "movieId").and_then(|v| v.trim().parse().ok()),
		poster_url,
		review_html,
	})
}

/// The text of a namespaced element such as `<letterboxd:filmTitle>`.
fn extension<'a>(item: &'a rss::Item, prefix: &str, name: &str) -> Option<&'a str> {
	item.extensions().get(prefix)?.get(name)?.first()?.value()
}

fn is_yes(value: &str) -> bool {
	value.trim().eq_ignore_ascii_case("yes")
}

/// Splits a description into the poster URL and the review HTML (empty when it only says
/// "Watched on …").
fn split_description(html: &str) -> (Option<String>, String) {
	let html = html.trim();

	let poster_url = html.find("<img src=\"").and_then(|start| {
		let rest = &html[start + "<img src=\"".len()..];
		rest.find('"').map(|end| rest[..end].replace("&amp;", "&"))
	});

	let body = if html.starts_with("<p><img") {
		html.find("</p>")
			.map(|end| html[end + "</p>".len()..].trim())
			.unwrap_or_default()
	} else {
		html
	};

	let only_watched_on = body.starts_with("<p>Watched on ")
		&& body.ends_with("</p>")
		&& body.matches("<p>").count() == 1;

	(
		poster_url,
		if only_watched_on {
			String::new()
		} else {
			body.to_string()
		},
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn splits_poster_from_review() {
		let (poster, body) = split_description(
			r#" <p><img src="https://a.ltrbxd.com/poster.jpg?v=1"/></p> <p>Loved it.</p><p>Twice.</p> "#,
		);
		assert_eq!(
			poster.as_deref(),
			Some("https://a.ltrbxd.com/poster.jpg?v=1")
		);
		assert_eq!(body, "<p>Loved it.</p><p>Twice.</p>");
	}

	#[test]
	fn drops_the_watched_on_line() {
		let (_, body) = split_description(
			r#"<p><img src="https://a.ltrbxd.com/p.jpg"/></p> <p>Watched on Monday October 12, 2026.</p>"#,
		);
		assert_eq!(body, "");
	}

	#[test]
	fn validates_usernames() {
		let parse =
			|username: &str| LetterboxdSettings::parse(serde_json::json!({ "username": username }));
		assert_eq!(parse(" Dave ").unwrap().username, "dave");
		assert!(parse("davidehrlich_2").is_ok());
		assert!(parse("").is_err());
		assert!(parse("../admin").is_err());
		assert!(parse("dave/rss").is_err());
	}
}
