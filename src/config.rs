use clap::Parser;
use once_cell::sync::Lazy;

pub static APP_CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenvy::dotenv().ok();
    Config::parse()
});

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env, default_value_t = 8080)]
    pub port: u16,

    #[clap(long, env)]
    pub database_uri: String,

    #[clap(long, env)]
    pub database_name: String,

    #[clap(long, env)]
    pub log_level: String,

    #[clap(long, env, default_value_t = false)]
    pub swagger_enabled: bool,

    #[clap(long, env, default_value_t = 100)]
    pub rate_limit_req_per_sec: u32,

    #[clap(long, env, value_delimiter = ',')]
    pub cors_origin_whitelist: Option<Vec<String>>,

    #[clap(long, env)]
    pub jwt_secret_key: String,

    #[clap(long, env, default_value = "local")]
    pub app_env: String,

    #[clap(long, env)]
    pub redis_url: String,

    #[clap(long, env)]
    pub kafka_group_id: String,

    #[clap(long, env)]
    pub kafka_brokers: String,

    #[clap(long, env)]
    pub kafka_ssl_enabled: bool,

    #[clap(long, env)]
    pub kafka_sasl_username: String,

    #[clap(long, env)]
    pub kafka_sasl_password: String,

    #[clap(long, env)]
    pub enable_idempotence: bool,

    #[clap(long, env)]
    pub firebase_credentials_path: String,
}

use serde::Deserialize;

fn default_enable_idempotence() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub kafka_group_id: String,
    pub kafka_brokers: String,
    pub kafka_ssl_enabled: bool,
    pub kafka_sasl_username: String,
    pub kafka_sasl_password: String,
    #[serde(default = "default_enable_idempotence")]
    pub enable_idempotence: bool,
}
