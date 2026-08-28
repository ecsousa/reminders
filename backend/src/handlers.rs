use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tokio_rusqlite::Connection;

use crate::{
    config::Config,
    db,
    middleware::UserInfo,
    models::{Reminder, ReminderCreateRequest, RemindersResponse, UserInfoResponse},
};

pub struct AppState {
    pub db: Connection,
    pub config: Config,
    pub web_client: Client,
}

pub async fn get_user_info(Extension(user_info): Extension<UserInfo>) -> Json<UserInfoResponse> {
    Json(UserInfoResponse {
        username: user_info.username,
        name: user_info.name,
    })
}

pub async fn create_reminder(
    State(state): State<Arc<AppState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(payload): Json<ReminderCreateRequest>,
) -> Result<Json<Reminder>, StatusCode> {
    let reminder = Reminder {
        id: None,
        username: Some(user_info.username.clone()),
        created_time: None,
        reminder_message: payload.reminder_message,
    };

    tracing::info!("User {} is creating a new reminder", user_info.username);

    match db::save_reminder(&state.db, reminder).await {
        Ok(saved) => Ok(Json(saved)),
        Err(e) => {
            tracing::error!("Failed to save reminder: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_reminders(
    State(state): State<Arc<AppState>>,
    Extension(user_info): Extension<UserInfo>,
) -> Result<Json<RemindersResponse>, StatusCode> {
    match db::find_all_by_username(&state.db, user_info.username).await {
        Ok(reminders) => Ok(Json(RemindersResponse { reminders })),
        Err(e) => {
            tracing::error!("Failed to get reminders: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_reminder(
    State(state): State<Arc<AppState>>,
    Extension(user_info): Extension<UserInfo>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("User {} is deleting reminder {}", user_info.username, id);

    match db::delete_by_id_and_username(&state.db, id, user_info.username).await {
        Ok(deleted_count) if deleted_count > 0 => Ok(StatusCode::NO_CONTENT),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to delete reminder: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn trigger_reminders(
    State(state): State<Arc<AppState>>,
    Extension(user_info): Extension<UserInfo>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Triggering all pending reminders");

    let reminders = match db::find_all(&state.db).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch reminders to trigger: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    for reminder in reminders {
        if let Some(id) = reminder.id {
            let tag = format!("{}-{}", state.config.apprise_tag_prefix, user_info.username);
            let payload = json!({
                "body": reminder.reminder_message,
                "title": "Reminder"
            });

            let mut success = false;
            for attempt in 1..=3 {
                let req = state
                    .web_client
                    .post(&state.config.apprise_url)
                    .query(&[("tag", &tag)])
                    .json(&payload)
                    .send()
                    .await;

                match req {
                    Ok(resp) if resp.status().is_success() => {
                        success = true;
                        break;
                    }
                    _ => {
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            }

            if success {
                tracing::debug!("Successfully triggered reminder {}", id);
                if let Err(e) = db::delete_by_id(&state.db, id).await {
                    tracing::error!("Failed to delete triggered reminder {}: {}", id, e);
                }
            } else {
                tracing::error!("Failed to trigger reminder {} after retries", id);
            }
        }
    }

    Ok(StatusCode::OK)
}
