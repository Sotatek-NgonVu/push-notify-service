use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use bb8_redis::redis::AsyncCommands;
use eyre::Error;
use tokio::sync::OnceCell as AsyncOnceCell;

use crate::config::APP_CONFIG;

type ConnectionPool = Pool<RedisConnectionManager>;

static REDIS_SERVICE: AsyncOnceCell<RedisService> = AsyncOnceCell::const_new();

pub struct RedisService {
    pool: ConnectionPool,
}

impl RedisService {
    pub async fn new() -> &'static RedisService {
        REDIS_SERVICE
            .get_or_init(|| async {
                let manager = RedisConnectionManager::new(APP_CONFIG.redis_url.clone()).unwrap();
                let pool = bb8::Pool::builder().build(manager).await.unwrap();
                RedisService { pool }
            })
            .await
    }

    pub async fn get_cache<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, Error> {
        let mut conn = self.pool.get().await?;

        let value: String = conn.get(key).await?;
        let cache_value = serde_json::from_str(&value)?;

        Ok(cache_value)
    }

    pub async fn set_ex_cache<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        expiration: usize,
    ) -> eyre::Result<(), Error> {
        let mut conn = self.pool.get().await?;
        let _: () = conn
            .set_ex(key, serde_json::to_string(value)?, expiration as u64)
            .await?;
        Ok(())
    }

    async fn get_sorted_cache(
        &self,
        key: &str,
        start: isize,
        limit: isize,
    ) -> Result<Vec<String>, Error> {
        let mut conn = self.pool.get().await?;
        let values: Vec<String> = conn.zrevrange(key, start, start + limit - 1).await?;

        Ok(values)
    }

    async fn get_rank_from_sorted_cache(&self, key: &str, value_str: &str) -> Result<isize, Error> {
        let mut conn = self.pool.get().await?;
        let rank: isize = conn.zrevrank(key, value_str).await?;
        Ok(rank)
    }

    fn get_aptos_price_key(&self) -> String {
        "dextrade:aptos:price_usd".to_string()
    }

    pub async fn get_aptos_price(&self) -> Result<String, Error> {
        let key = self.get_aptos_price_key();
        let mut conn = self.pool.get().await?;
        let value: Option<String> = conn.get(&key).await?;

        if let Some(val) = value {
            Ok(val)
        } else {
            Err(eyre::Error::msg("Aptos price not found"))
        }
    }

    fn get_sui_price_key(&self) -> String {
        "dextrade:sui:price_usd".to_string()
    }

    pub async fn get_sui_price(&self) -> Result<String, Error> {
        let key = self.get_sui_price_key();
        let mut conn = self.pool.get().await?;
        let value: Option<String> = conn.get(&key).await?;

        if let Some(val) = value {
            Ok(val)
        } else {
            Err(eyre::Error::msg("Sui price not found"))
        }
    }

    fn get_key(&self, prefix: &str, pair_id: &str, network: &str) -> String {
        format!("dextrade:{prefix}:network:{network}:{pair_id}")
    }

    pub async fn get_rate_limit(&self, ip: &str) -> Result<i32, Error> {
        let key = format!("rate_limit_request_external_audit:{ip}");
        self.get_cache(&key).await
    }

    pub async fn set_rate_limit(&self, ip: &str, count: i32) -> Result<(), Error> {
        let key = format!("rate_limit_request_external_audit:{ip}");
        self.set_ex_cache(&key, &count, 60).await
    }
}
