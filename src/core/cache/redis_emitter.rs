use anyhow::Error;
use once_cell::sync::OnceCell;
use redis::{Client, Commands};
use serde::{de::DeserializeOwned, Serialize};
use socketio_rust_emitter::Emitter;
use std::time::Duration;
use tokio::task;

pub static REDIS_EMITTER: OnceCell<RedisEmitter> = OnceCell::new();

pub async fn setup_redis_emitter(redis_url: &str) -> Result<(), Error> {
    let redis_emitter = RedisEmitter::new(redis_url)?;
    REDIS_EMITTER
        .set(redis_emitter)
        .map_err(|_| anyhow::anyhow!("Failed to set redis emitter"))?;

    Ok(())
}

pub fn get_redis_emitter() -> &'static RedisEmitter {
    match REDIS_EMITTER.get() {
        Some(emitter) => emitter,
        None => {
            println!("Redis emitter not initialized");
            std::process::exit(1);
        }
    }
}

#[derive(Clone)]
pub struct RedisEmitter {
    pub client: Client,
    pub emitter: Emitter,
}

impl RedisEmitter {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = Client::open(redis_url)?;
        let emitter = Emitter::new(client.clone());

        Ok(Self { client, emitter })
    }

    pub fn emit_room(&self, room: &str, event: &str, data: &str) -> Result<(), String> {
        let mut retries = 0;
        let room_str = room.to_string();
        let event_str = event.to_string();
        loop {
            let result = std::panic::catch_unwind({
                let emitter = self.clone().emitter.clone();
                let room = room_str.clone();
                let event = event_str.clone();
                let data = data.to_string();
                move || {
                    emitter.to(&room).emit(vec![&event, &data]);
                }
            });

            if result.is_ok() {
                return Ok(());
            } else {
                retries += 1;
                if retries >= 3 {
                    return Err(format!(
                        "Send event {:?} to room {:?} failed after 3 retries",
                        event_str, room_str
                    ));
                }
            }
        }
    }

    pub fn publish<T: Serialize>(&self, channel: &str, message: &T) -> anyhow::Result<()> {
        let mut conn = self.client.get_connection()?;
        let serialized = serde_json::to_string(message)?;
        let _: i32 = conn.publish(channel, serialized)?;
        Ok(())
    }

    pub async fn subscribe<T>(
        &self,
        channel: &str,
        callback: impl Fn(T) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + 'static
        + Clone,
    ) -> anyhow::Result<()>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let client = self.client.clone();
        let channel = channel.to_string();

        task::spawn_blocking(move || {
            let mut conn = client.get_connection()?;
            let mut pubsub = conn.as_pubsub();
            pubsub.set_read_timeout(Some(Duration::from_secs(1)))?;
            pubsub.subscribe(&channel)?;

            let rt = tokio::runtime::Handle::current();

            loop {
                match pubsub.get_message() {
                    Ok(msg) => {
                        let payload: String = msg.get_payload()?;

                        let data: T = serde_json::from_str(&payload)
                            .map_err(|e| anyhow::anyhow!("Failed to deserialize message: {}", e))?;

                        let cb = callback.clone();
                        rt.spawn(cb(data));
                    }
                    Err(err) => {
                        if err.is_timeout() {
                            continue;
                        }
                        return Err(anyhow::anyhow!("Redis PubSub error: {}", err));
                    }
                }
            }
        })
            .await??;

        Ok(())
    }
}