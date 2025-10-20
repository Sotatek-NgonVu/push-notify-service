use axum::extract::FromRef;
use mongodb::Database;

use crate::database;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
}

impl AppState {
    pub async fn init() -> eyre::Result<Self> {
        let database = database::connection().await.clone();

        Ok(Self { database })
    }
}

impl FromRef<AppState> for Database {
    fn from_ref(app_state: &AppState) -> Database {
        app_state.database.clone()
    }
}
