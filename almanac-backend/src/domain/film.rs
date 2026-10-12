/// The `data` of a film item: what was watched and how it landed.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilmData {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub year: Option<i32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub director: Option<String>,
	/// Out of 5, in half stars (0.5 to 5.0), as on Letterboxd.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rating: Option<f64>,
	#[serde(default)]
	pub rewatch: bool,
	#[serde(default)]
	pub liked: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub poster_url: Option<String>,
	/// The diary entry or review on Letterboxd, for imported films.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub letterboxd_url: Option<String>,
	/// The Movie Database id, which Letterboxd includes; useful for matching films later.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub tmdb_id: Option<i64>,
}

impl FilmData {
	/// Parses and validates film data from a request; `null` means no extra details.
	pub fn parse(data: serde_json::Value) -> Result<Self, String> {
		let data = if data.is_null() {
			serde_json::json!({})
		} else {
			data
		};
		let mut film: FilmData =
			serde_json::from_value(data).map_err(|e| format!("invalid film data: {e}"))?;

		film.director = film.director.filter(|director| !director.trim().is_empty());
		film.poster_url = film.poster_url.filter(|url| !url.trim().is_empty());
		film.letterboxd_url = film.letterboxd_url.filter(|url| !url.trim().is_empty());

		if film.year.is_some_and(|year| !(1870..=2100).contains(&year)) {
			return Err("year must be between 1870 and 2100".into());
		}

		let is_half_star = |rating: f64| (rating * 2.0).fract().abs() < f64::EPSILON;
		if film
			.rating
			.is_some_and(|rating| !(0.5..=5.0).contains(&rating) || !is_half_star(rating))
		{
			return Err("rating must be between 0.5 and 5 in half stars".into());
		}

		let is_web_url = |url: &str| url.starts_with("https://") || url.starts_with("http://");
		if film
			.poster_url
			.as_deref()
			.is_some_and(|url| !is_web_url(url))
		{
			return Err("poster_url must be an http(s) URL".into());
		}
		if film
			.letterboxd_url
			.as_deref()
			.is_some_and(|url| !is_web_url(url))
		{
			return Err("letterboxd_url must be an http(s) URL".into());
		}

		if film.tmdb_id.is_some_and(|id| id <= 0) {
			return Err("tmdb_id must be positive".into());
		}

		Ok(film)
	}

	pub fn to_value(&self) -> serde_json::Value {
		serde_json::to_value(self).expect("FilmData always serialises")
	}
}
