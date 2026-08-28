use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

pub mod date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(d) = date {
            let s = format!("{}", d.format(FORMAT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        if let Some(s) = s {
            NaiveDateTime::parse_from_str(&s, FORMAT)
                .map(Some)
                .map_err(serde::de::Error::custom)
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub id: Option<i64>,
    pub username: Option<String>,
    #[serde(with = "date_format", default)]
    pub created_time: Option<NaiveDateTime>,
    pub reminder_message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderCreateRequest {
    pub reminder_message: String,
}

#[derive(Debug, Serialize)]
pub struct RemindersResponse {
    pub reminders: Vec<Reminder>,
}

#[derive(Debug, Serialize)]
pub struct UserInfoResponse {
    pub username: String,
    pub name: String,
}
