use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{ActivityData, FilmData};

/// What an item is. Each kind has its own shape for `Item::data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
	Note,
	Film,
	Activity,
}

impl ItemKind {
	pub fn as_str(self) -> &'static str {
		match self {
			ItemKind::Note => "note",
			ItemKind::Film => "film",
			ItemKind::Activity => "activity",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"note" => Some(ItemKind::Note),
			"film" => Some(ItemKind::Film),
			"activity" => Some(ItemKind::Activity),
			_ => None,
		}
	}
}

/// Private items are only returned to the logged-in admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
	Public,
	Private,
}

impl Visibility {
	pub fn as_str(self) -> &'static str {
		match self {
			Visibility::Public => "public",
			Visibility::Private => "private",
		}
	}

	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"public" => Some(Visibility::Public),
			"private" => Some(Visibility::Private),
			_ => None,
		}
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Item {
	pub id: Uuid,
	pub kind: ItemKind,
	pub title: String,
	pub topic: Option<String>,
	/// The admin's own writing, as HTML.
	pub body: String,
	/// Kind-specific fields: `FilmData` for films, `ActivityData` for activities, `{}` for notes.
	pub data: serde_json::Value,
	pub occurred_at: DateTime<Utc>,
	pub visibility: Visibility,
	/// `manual` for items made in the app; a connector name for imported ones.
	pub source: String,
	pub external_id: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Request body used to create or replace an item.
#[derive(Debug, serde::Deserialize)]
pub struct ItemPayload {
	pub kind: ItemKind,
	pub title: String,
	#[serde(default)]
	pub topic: Option<String>,
	pub body: String,
	#[serde(default)]
	pub data: Option<serde_json::Value>,
	/// Defaults to now on create; left unchanged on update when omitted.
	#[serde(default)]
	pub occurred_at: Option<DateTime<Utc>>,
	/// Defaults to public on create; left unchanged on update when omitted.
	#[serde(default)]
	pub visibility: Option<Visibility>,
}

/// An `ItemPayload` that passed validation, with blank topics dropped and `data` normalised.
#[derive(Debug)]
pub struct ValidItem {
	pub kind: ItemKind,
	pub title: String,
	pub topic: Option<String>,
	pub body: String,
	pub data: serde_json::Value,
	pub occurred_at: Option<DateTime<Utc>>,
	pub visibility: Option<Visibility>,
}

impl ItemPayload {
	pub fn validate(self) -> Result<ValidItem, String> {
		if self.title.trim().is_empty() {
			return Err("title must not be empty".into());
		}

		let topic = self.topic.filter(|topic| !topic.trim().is_empty());
		let data = self.data.unwrap_or(serde_json::Value::Null);

		let data = match self.kind {
			ItemKind::Note => {
				if topic.is_none() {
					return Err("topic must not be empty".into());
				}
				serde_json::json!({})
			}
			ItemKind::Film => FilmData::parse(data)?.to_value(),
			ItemKind::Activity => ActivityData::parse(data)?.to_value(),
		};

		Ok(ValidItem {
			kind: self.kind,
			title: self.title,
			topic,
			body: self.body,
			data,
			occurred_at: self.occurred_at,
			visibility: self.visibility,
		})
	}
}
