use mongodb::Database;
use tokio::sync::OnceCell;
use wither::mongodb;

use crate::config::APP_CONFIG;

static CONNECTION: OnceCell<Database> = OnceCell::const_new();

pub async fn connection() -> &'static Database {
    CONNECTION
        .get_or_init(|| async {
            let db_uri = &APP_CONFIG.database_uri;
            let db_name = &APP_CONFIG.database_name;

            mongodb::Client::with_uri_str(db_uri)
                .await
                .expect("Failed to initialize MongoDB connection")
                .database(db_name)
        })
        .await
}
