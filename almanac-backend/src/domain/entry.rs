use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
	pub id: Uuid,
	pub title: String,
	pub topic: String,
	pub body: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Request body used to create or replace an entry.
#[derive(Debug, serde::Deserialize)]
pub struct EntryPayload {
	pub title: String,
	pub topic: String,
	pub body: String,
}

impl EntryPayload {
	pub fn validate(&self) -> Result<(), String> {
		if self.title.trim().is_empty() {
			return Err("title must not be empty".into());
		}

		if self.topic.trim().is_empty() {
			return Err("topic must not be empty".into());
		}

		Ok(())
	}
}
