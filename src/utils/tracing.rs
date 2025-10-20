use crate::config::APP_CONFIG;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_standard_tracing(crate_name: &str) {
    let level: &String = &APP_CONFIG.log_level;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{crate_name}={level},tower_http={level},api_wallet_evm={level}").into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().event_format(tracing_subscriber::fmt::format()))
        .init();
}
