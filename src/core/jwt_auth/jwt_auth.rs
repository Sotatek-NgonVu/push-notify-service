use std::{str::FromStr, sync::LazyLock};

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use http::Method;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode};
use serde::{Deserialize, Serialize};

use crate::{
    config::APP_CONFIG, enums::EAvailableScope, errors::Error as AppError,
    models::accounts::Account,
};

use super::types::TokenClaims;
use regex::Regex;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub message: String,
}

/// JWT decode function
pub fn decode_jwt(token: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let secret_key = &APP_CONFIG.jwt_secret_key; // Use a secure secret key
    let decoding_key = DecodingKey::from_secret(secret_key.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    decode::<TokenClaims>(token, &decoding_key, &validation).map(|data| data.claims)
}

// Generate JWT token
pub fn generate_jwt(
    user_id: &str,
    user_name: &str,
    display_name: &str,
    iap: Option<usize>,
    expiration_seconds: i64,
    oauth2_client_id: Option<String>,
    oauth2_scopes: Option<String>,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::seconds(expiration_seconds)).timestamp() as usize;

    let claims = TokenClaims {
        user_id: user_id.to_string(),
        user_name: user_name.to_string(),
        display_name: display_name.to_string(),
        iap,
        iat,
        exp,
        oauth2_client_id,
        oauth2_scopes,
    };

    let secret_key = &APP_CONFIG.jwt_secret_key; // Use a secure secret key
    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key.as_ref()),
    )
}

fn convert_scopes(input: Vec<String>) -> Vec<EAvailableScope> {
    input
        .iter()
        .filter_map(|s| EAvailableScope::from_str(s.as_str()).ok())
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtAuth(pub TokenClaims);

impl<S> FromRequestParts<S> for JwtAuth
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, _state)
                .await
                .map_err(|_| AppError::unauthorized("Authorization header missing"))?;

        let token_data =
            decode_jwt(bearer.token()).map_err(|_| AppError::unauthorized("Invalid jwt token"))?;

        let user_info = Account::get_account_by_user_id(&token_data.user_id).await?;

        let _ = user_info.ok_or_else(|| {
            AppError::unauthorized("The user belonging to this token no longer exists")
        })?;

        let path = parts.uri.path().to_string();
        let method = &parts.method;

        if !verify_scope(path, method, token_data.clone()) {
            return Err(AppError::forbidden(
                "You do not have permission to access this resource",
            ));
        }

        Ok(JwtAuth(token_data))
    }
}

#[derive(Clone)]
struct RoutePattern {
    regex: Regex,
    method: String,
    scopes: Vec<EAvailableScope>,
}

static ROUTE_PATTERNS: LazyLock<Vec<RoutePattern>> = LazyLock::new(|| {
    let positions_scopes = vec![
        EAvailableScope::PositionsReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];
    let preset_write_scopes = vec![
        EAvailableScope::PresetSettingWrite,
        EAvailableScope::SettingsFullAccess,
    ];
    let transactions_scopes = vec![
        EAvailableScope::TransactionsReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];
    let holders_scopes = vec![
        EAvailableScope::HoldersReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];
    let favorites_read_scopes = vec![
        EAvailableScope::FavoritesReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];
    let favorites_write_scopes = vec![
        EAvailableScope::FavoritesWrite,
        EAvailableScope::SettingsFullAccess,
    ];
    let referrals_read_scopes = vec![
        EAvailableScope::ReferralsReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];
    let referrals_write_scopes = vec![
        EAvailableScope::ReferralsWrite,
        EAvailableScope::SettingsFullAccess,
    ];
    let token_scopes = vec![
        EAvailableScope::TokenReadOnly,
        EAvailableScope::SettingsFullAccess,
    ];

    let create_pattern = |path: &str, method: &str, scopes: &[EAvailableScope]| RoutePattern {
        regex: Regex::new(path).unwrap(),
        method: method.to_string(),
        scopes: scopes.to_vec(),
    };

    vec![
        // Positions
        create_pattern(r"^/traders/positions$", "GET", &positions_scopes),
        create_pattern(r"^/traders/positions/[^/]+$", "GET", &positions_scopes),
        create_pattern(
            r"^/traders/positions/[^/]+/hide$",
            "POST",
            &positions_scopes,
        ),
        // Preset settings
        create_pattern(
            r"^/preset-settings$",
            "GET",
            &[EAvailableScope::PresetSettingReadOnly],
        ),
        create_pattern(r"^/preset-settings$", "POST", &preset_write_scopes),
        create_pattern(r"^/preset-settings$", "PUT", &preset_write_scopes),
        // Transactions
        create_pattern(r"^/my/transactions$", "GET", &transactions_scopes),
        create_pattern(r"^/transactions/user-txs$", "GET", &transactions_scopes),
        // Holder
        create_pattern(r"^/holder/balances$", "GET", &holders_scopes),
        create_pattern(r"^/holder/chart-histories$", "GET", &holders_scopes),
        // Favourites
        create_pattern(r"^/favourites$", "GET", &favorites_read_scopes),
        create_pattern(r"^/favourites/[^/]+/check$", "GET", &favorites_read_scopes),
        create_pattern(r"^/favourites$", "POST", &favorites_write_scopes),
        create_pattern(r"^/favourites/[^/]+$", "DELETE", &favorites_write_scopes),
        // Referral
        create_pattern(r"^/accounts/referral$", "GET", &referrals_read_scopes),
        create_pattern(r"^/accounts/referral$", "POST", &referrals_write_scopes),
        create_pattern(r"^/accounts/referral$", "PUT", &referrals_write_scopes),
        // Token
        create_pattern(r"^/tokens/get-balance-of$", "GET", &token_scopes),
        create_pattern(r"^/tokens/locked$", "GET", &token_scopes),
    ]
});

fn verify_scope(path: String, method: &Method, token_data: TokenClaims) -> bool {
    if token_data.oauth2_client_id.is_none() {
        return true;
    }

    let scopes = convert_scopes(
        token_data
            .oauth2_scopes
            .as_ref()
            .unwrap()
            .split(',')
            .map(String::from)
            .collect(),
    );

    for pattern in ROUTE_PATTERNS.iter() {
        if pattern.regex.is_match(&path) && pattern.method == method.as_str() {
            return scopes.contains(&EAvailableScope::FullAccess)
                || (scopes.contains(&EAvailableScope::FullReadOnly) && *method == Method::GET)
                || pattern.scopes.iter().any(|scope| scopes.contains(scope));
        }
    }

    false
}
