use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenClaims {
    pub user_id: String,
    pub user_name: String,
    pub display_name: String,
    pub iap: Option<usize>,
    pub iat: usize,
    pub exp: usize,
    pub oauth2_client_id: Option<String>,
    pub oauth2_scopes: Option<String>,
}

impl fmt::Display for TokenClaims {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenClaims {{ user_id: {}, user_name: {}, display_name: {}, iap: {:?}, iat: {}, exp: {} }}",
            self.user_id, self.user_name, self.display_name, self.iap, self.iat, self.exp
        )
    }
}
