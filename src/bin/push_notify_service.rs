use std::net::SocketAddr;

use push_notify_service::{app, config::APP_CONFIG, utils::tracing::init_standard_tracing};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();
    init_standard_tracing(env!("CARGO_CRATE_NAME"));

    let app = app::create_app().await.unwrap();

    let address = format!("0.0.0.0:{}", APP_CONFIG.port);

    tracing::info!("Server listening on {}", &address);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");

    Ok(())
}
