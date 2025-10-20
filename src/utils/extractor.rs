use std::net::IpAddr;

use axum::http::Request;
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Bearer};
use http::Method;
use tower_governor::{
    GovernorError,
    key_extractor::{KeyExtractor, SmartIpKeyExtractor},
};

use crate::core::jwt_auth::jwt_auth::decode_jwt;

#[derive(Debug, Clone)]
pub struct BearerOrSmartIpKeyExtractor;

#[derive(Debug, Hash, Eq, PartialEq, Clone, strum_macros::Display)]
pub enum RequestKey {
    Bearer(String, Method, String), // token, method, path
    Ip(IpAddr, Method, String),     // ip, method, path
}

impl KeyExtractor for BearerOrSmartIpKeyExtractor {
    type Key = RequestKey;

    fn extract<B>(&self, req: &Request<B>) -> Result<Self::Key, GovernorError> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        if let Some(auth) = req.headers().typed_get::<Authorization<Bearer>>() {
            let token = auth.token();
            // valid jwt
            if decode_jwt(token).is_ok() {
                return Ok(RequestKey::Bearer(token.to_string(), method, path));
            }
        }

        // Fallback to IP extraction
        let ip_extractor = SmartIpKeyExtractor;
        let ip = ip_extractor.extract(req)?;
        Ok(RequestKey::Ip(ip, method, path))
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(format!("{key}"))
    }

    fn name(&self) -> &'static str {
        "BearerOrSmartIpKeyExtractor"
    }
}
