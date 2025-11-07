use crate::core::cache::redis_emitter::get_redis_emitter;
use crate::enums::UserFcmTokenStatus;
use crate::errors::Error;
use crate::models::user_fcm_token::UserFcmToken;
use crate::utils::models::ModelExt;
use bson::doc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

static USER_FCM_TOKENS: LazyLock<RwLock<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::with_capacity(100_000)));

pub const UPDATE_FCM_TOKEN_CHANNEL: &str = "vdax:notification:update_fcm_token";

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum UpdateFcmTokenAction {
    Add,
    Remove,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateFcmToken {
    pub user_id: String,
    pub token: String,
    pub action: UpdateFcmTokenAction,
}

pub async fn preload_user_fcm_tokens() -> Result<(), Error> {
    let query = doc! {};
    let fcm_tokens = UserFcmToken::find(query, None).await?;

    let mut map = USER_FCM_TOKENS.write().await;
    map.clear();

    for token in fcm_tokens {
        if token.status == UserFcmTokenStatus::Active.to_string() {
            map.entry(token.user_id).or_default().push(token.token);
        }
    }

    Ok(())
}

pub async fn get_user_fcm_tokens(user_id: String) -> Result<Vec<String>, Error> {
    {
        let map = USER_FCM_TOKENS.read().await;

        if let Some(tokens) = map.get(&user_id) {
            return Ok(tokens.clone());
        }
    }
    tracing::debug!(
        "FCM tokens not found in memory cache for user ID {}",
        user_id
    );

    let query = doc! {
        "userId": user_id.clone()
    };

    match UserFcmToken::find(query, None).await {
        Ok(tokens) => {
            let mut map = USER_FCM_TOKENS.write().await;
            let mut result = Vec::new();

            for token in tokens {
                if token.status == UserFcmTokenStatus::Active.to_string() {
                    let fcm_token = token.token.clone();
                    map.entry(user_id.clone())
                        .or_default()
                        .push(fcm_token.clone());
                    result.push(fcm_token);
                }
            }

            Ok(result)
        }
        Err(e) => {
            tracing::error!("Failed to fetch FCM tokens for user ID {}: {}", user_id, e);
            Err(Error::internal_err(
                "Failed to fetch user FCM tokens from database.",
            ))
        }
    }
}

pub async fn update_fcm_token_in_memory(action: UpdateFcmToken) -> anyhow::Result<()> {
    match action.action {
        UpdateFcmTokenAction::Add => {
            add_fcm_token_to_memory(action.user_id, action.token.clone()).await
        }
        UpdateFcmTokenAction::Remove => {
            remove_fcm_token_from_memory(action.user_id, action.token.clone()).await
        }
    }
}

pub async fn publish_fcm_token_update(
    user_id: String,
    token: String,
    action: UpdateFcmTokenAction,
) -> Result<(), Error> {
    let redis_emitter = get_redis_emitter();

    let update = UpdateFcmToken {
        user_id: user_id.clone(),
        token,
        action,
    };

    redis_emitter
        .publish(UPDATE_FCM_TOKEN_CHANNEL, &update)
        .map_err(|e| {
            tracing::error!(
                "Failed to publish FCM token update for user ID {}: {}",
                user_id,
                e
            );
            Error::internal_err("Failed to publish FCM token update.")
        })?;

    Ok(())
}

async fn remove_fcm_token_from_memory(user_id: String, token: String) -> anyhow::Result<()> {
    let mut map = USER_FCM_TOKENS.write().await;

    let tokens = map.get_mut(&user_id);
    if let Some(tokens) = tokens {
        tokens.retain(|t| t != &token);

        if tokens.is_empty() {
            map.remove(&user_id);
        }
    }

    Ok(())
}

async fn add_fcm_token_to_memory(user_id: String, token: String) -> anyhow::Result<()> {
    let mut map = USER_FCM_TOKENS.write().await;

    let tokens = map.entry(user_id).or_default();
    if !tokens.contains(&token) {
        tokens.push(token.clone());
    }

    Ok(())
}
