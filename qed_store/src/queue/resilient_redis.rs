use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub connected: bool,
    last_attempt: Option<Instant>,
    backoff_duration: Duration,
}

impl ConnectionState {
    fn new(connected: bool) -> Self {
        Self {
            connected,
            last_attempt: None,
            backoff_duration: Duration::from_millis(100),
        }
    }

    fn reset_reconnect(&mut self) {
        self.last_attempt = None;
        self.backoff_duration = Duration::from_millis(100);
        self.connected = true;
    }

    fn next_attempt(&mut self) {
        self.last_attempt = Some(Instant::now());
        self.backoff_duration = std::cmp::min(
            self.backoff_duration * 2,
            Duration::from_secs(30)
        );
    }

    fn should_wait(&self) -> Option<Duration> {
        if let Some(last_attempt) = self.last_attempt {
            let elapsed = last_attempt.elapsed();
            if elapsed < self.backoff_duration {
                Some(self.backoff_duration - elapsed)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn mark_disconnected(&mut self) {
        self.connected = false;
    }
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::new(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub connected: bool,
}

#[derive(Clone)]
pub struct ResilientRedisConnection {
    client: redis::Client,
    connection: Arc<RwLock<Option<MultiplexedConnection>>>,
    state: Arc<RwLock<ConnectionState>>,
    redis_url: String,
}

impl ResilientRedisConnection {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let connection = match client.get_multiplexed_async_connection().await {
            Ok(conn) => Some(conn),
            Err(e) => {
                tracing::warn!("Initial Redis connection failed: {}, will retry later", e);
                None
            }
        };

        let state = ConnectionState::new(connection.is_some());

        let resilient_conn = Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
            state: Arc::new(RwLock::new(state)),
            redis_url: redis_url.to_string(),
        };


        Ok(resilient_conn)
    }

    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(MultiplexedConnection) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, redis::RedisError>> + Send,
        T: Send + 'static,
    {
        let conn = self.get_or_create_connection().await?;

        match operation(conn).await {
            Ok(result) => Ok(result),
            Err(e) => {
                let is_conn_error = self.is_connection_error(&e);
                
                if is_conn_error {
                    self.mark_connection_invalid().await;
                }
                Err(e.into())
            }
        }
    }

    async fn get_or_create_connection(&self) -> Result<MultiplexedConnection> {
        {
            let read_guard = self.connection.read().await;
            if let Some(ref conn) = *read_guard {
                return Ok(conn.clone());
            }
        }

        let mut write_guard = self.connection.write().await;

        if let Some(ref conn) = *write_guard {
            return Ok(conn.clone());
        }

        let new_conn = self.create_connection().await?;
        *write_guard = Some(new_conn.clone());

        drop(write_guard);

        tracing::info!("Successfully created new Redis connection");
        Ok(new_conn)
    }

    async fn create_connection(&self) -> Result<MultiplexedConnection> {
        let mut state = self.state.write().await;

        if let Some(wait_time) = state.should_wait() {
            drop(state);
            tracing::debug!("Waiting {:?} before reconnect attempt", wait_time);
            tokio::time::sleep(wait_time).await;
            state = self.state.write().await;
        }

        state.next_attempt();
        drop(state);

        match self.client.get_multiplexed_async_connection().await {
            Ok(conn) => {
                let mut state = self.state.write().await;
                state.reset_reconnect();
                drop(state);

                tracing::info!("Redis reconnection successful");
                Ok(conn)
            }
            Err(e) => {
                tracing::error!("Redis reconnection failed: {}", e);
                Err(e.into())
            }
        }
    }

    async fn mark_connection_invalid(&self) {
        *self.connection.write().await = None;

        let mut state = self.state.write().await;
        state.mark_disconnected();
    }

    async fn set_client_name(&self) -> Result<()> {
        if let Ok(role) = std::env::var("QED_ROLE") {
            let client_name = format!("resilient-{}-{}", role, std::process::id());

            self.execute(move |mut conn| {
                let name = client_name.clone();
                async move {
                    redis::cmd("CLIENT")
                        .arg("SETNAME")
                        .arg(&name)
                        .query_async::<String>(&mut conn)
                        .await
                }
            }).await?;
        }
        Ok(())
    }


    fn is_connection_error(&self, error: &redis::RedisError) -> bool {
        matches!(error.kind(),
            redis::ErrorKind::IoError |
            redis::ErrorKind::ExtensionError |
            redis::ErrorKind::ReadOnly
        )
    }

    pub async fn get_stats(&self) -> ConnectionStats {
        let state = self.state.read().await;
        ConnectionStats {
            connected: state.connected,
        }
    }

    pub async fn health_check(&self) -> bool {
        match self.execute(|mut conn| async move {
            redis::cmd("PING").query_async::<String>(&mut conn).await
        }).await {
            Ok(pong) => pong == "PONG",
            Err(_) => false,
        }
    }

    pub async fn force_reconnect(&self) -> Result<()> {
        self.mark_connection_invalid().await;
        self.get_or_create_connection().await?;
        Ok(())
    }
}

impl ResilientRedisConnection {
    pub async fn get<K, V>(&self, key: K) -> Result<V>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::FromRedisValue + Send + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            conn.get(key_clone).await
        }).await
    }

    pub async fn set<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let value_clone = value.clone();
        self.execute(move |mut conn| async move {
            conn.set(key_clone, value_clone).await
        }).await
    }

    pub async fn lpop<K, V>(&self, key: K, count: Option<usize>) -> Result<Option<V>>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::FromRedisValue + Send + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            let count_param = count.and_then(|c| std::num::NonZeroUsize::new(c));
            conn.lpop(key_clone, count_param).await
        }).await
    }

    pub async fn blpop<K>(&self, key: K, timeout: usize) -> Result<Option<(String, Vec<u8>)>>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let result = self.execute(move |mut conn| async move {
            redis::cmd("BLPOP")
                .arg(key_clone)
                .arg(timeout)
                .query_async(&mut conn)
                .await
        }).await;
        if result.is_err() {
            tracing::error!("BLPOP failed: {:?}", result);
        }
        result
    }

    pub async fn rpush<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let value_clone = value.clone();
        let result = self.execute(move |mut conn| async move {
            conn.rpush(key_clone, value_clone).await
        }).await;
        if result.is_err() {
            tracing::error!("RPUSH failed: {:?}", result);
        }
        result
    }

    pub async fn hget<K, F, V>(&self, key: K, field: F) -> Result<V>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        F: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::FromRedisValue + Send + 'static,
    {
        let key_clone = key.clone();
        let field_clone = field.clone();
        self.execute(move |mut conn| async move {
            conn.hget(key_clone, field_clone).await
        }).await
    }

    pub async fn hset<K, F, V>(&self, key: K, field: F, value: V) -> Result<bool>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        F: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let field_clone = field.clone();
        let value_clone = value.clone();
        self.execute(move |mut conn| async move {
            conn.hset(key_clone, field_clone, value_clone).await
        }).await
    }

    pub async fn hset_nx<K, F, V>(&self, key: K, field: F, value: V) -> Result<bool>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        F: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let field_clone = field.clone();
        let value_clone = value.clone();
        self.execute(move |mut conn| async move {
            conn.hset_nx(key_clone, field_clone, value_clone).await
        }).await
    }

    pub async fn hincr<K, F>(&self, key: K, field: F, delta: i64) -> Result<i64>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        F: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let field_clone = field.clone();
        self.execute(move |mut conn| async move {
            conn.hincr(key_clone, field_clone, delta).await
        }).await
    }

    pub async fn sadd<K, V>(&self, key: K, member: V) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let member_clone = member.clone();
        self.execute(move |mut conn| async move {
            conn.sadd(key_clone, member_clone).await
        }).await
    }

    pub async fn smembers<K, V>(&self, key: K) -> Result<Vec<V>>
    where
        K: redis::ToRedisArgs + Send + Sync + 'static,
        V: redis::FromRedisValue + Send + 'static,
    {
        self.execute(move |mut conn| async move {
            conn.smembers(key).await
        }).await
    }

    pub async fn srem<K, V>(&self, key: K, members: &[V]) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let members = members.to_vec();
        self.execute(move |mut conn| async move {
            conn.srem(key_clone, members).await
        }).await
    }

    pub async fn del<K>(&self, key: K) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            conn.del(key_clone).await
        }).await
    }

    pub async fn lrange<K, V>(&self, key: K, start: isize, stop: isize) -> Result<Vec<V>>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::FromRedisValue + Send + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            conn.lrange(key_clone, start, stop).await
        }).await
    }

    pub async fn ltrim<K>(&self, key: K, start: isize, stop: isize) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            conn.ltrim(key_clone, start, stop).await
        }).await
    }

    pub async fn llen<K>(&self, key: K) -> Result<usize>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        self.execute(move |mut conn| async move {
            conn.llen(key_clone).await
        }).await
    }

    pub async fn set_ex<K, V>(&self, key: K, value: V, seconds: u64) -> Result<()>
    where
        K: redis::ToRedisArgs + Send + Sync + Clone + 'static,
        V: redis::ToRedisArgs + Send + Sync + Clone + 'static,
    {
        let key_clone = key.clone();
        let value_clone = value.clone();
        self.execute(move |mut conn| async move {
            conn.set_ex(key_clone, value_clone, seconds).await
        }).await
    }

    pub async fn execute_cmd<T>(&self, cmd: redis::Cmd) -> Result<T>
    where
        T: redis::FromRedisValue + Send + 'static,
    {
        let cmd = cmd.clone();
        self.execute(move |mut conn| async move {
            cmd.query_async(&mut conn).await
        }).await
    }

    pub async fn ping(&self) -> Result<String> {
        self.execute(|mut conn| async move {
            redis::cmd("PING").query_async::<String>(&mut conn).await
        }).await
    }

    pub async fn execute_commands(&self, commands: Vec<redis::Cmd>) -> Result<Vec<redis::Value>> {
        match commands.len() {
            0 => Ok(vec![]),
            1 => {
                let cmd = commands.into_iter().next().unwrap();
                let result = self.execute_cmd(cmd).await?;
                Ok(vec![result])
            }
            _ => {
                self.execute_pipeline(commands).await
            }
        }
    }

    async fn execute_pipeline(&self, commands: Vec<redis::Cmd>) -> Result<Vec<redis::Value>> {
        self.execute(move |mut conn| async move {
            let mut pipeline = redis::pipe();
            for cmd in commands {
                pipeline.add_command(cmd);
            }
            pipeline.query_async(&mut conn).await
        }).await
    }

    pub fn cmd_builder(&self) -> CommandBuilder {
        CommandBuilder::new()
    }
}

pub struct CommandBuilder {
    commands: Vec<redis::Cmd>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn set<K, V>(mut self, key: K, value: V) -> Self
    where
        K: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("SET");
        cmd.arg(key).arg(value);
        self.commands.push(cmd);
        self
    }

    pub fn hset<K, F, V>(mut self, key: K, field: F, value: V) -> Self
    where
        K: redis::ToRedisArgs,
        F: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("HSET");
        cmd.arg(key).arg(field).arg(value);
        self.commands.push(cmd);
        self
    }

    pub fn hset_nx<K, F, V>(mut self, key: K, field: F, value: V) -> Self
    where
        K: redis::ToRedisArgs,
        F: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("HSET");
        cmd.arg(key).arg(field).arg(value).arg("NX");
        self.commands.push(cmd);
        self
    }

    pub fn sadd<K, V>(mut self, key: K, member: V) -> Self
    where
        K: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("SADD");
        cmd.arg(key).arg(member);
        self.commands.push(cmd);
        self
    }

    pub fn del<K>(mut self, key: K) -> Self
    where
        K: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("DEL");
        cmd.arg(key);
        self.commands.push(cmd);
        self
    }

    pub fn srem<K, V>(mut self, key: K, members: &[V]) -> Self
    where
        K: redis::ToRedisArgs,
        V: redis::ToRedisArgs + Clone,
    {
        let mut cmd = redis::cmd("SREM");
        cmd.arg(key);
        for member in members {
            cmd.arg(member.clone());
        }
        self.commands.push(cmd);
        self
    }

    pub fn rpush<K, V>(mut self, key: K, value: V) -> Self
    where
        K: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("RPUSH");
        cmd.arg(key).arg(value);
        self.commands.push(cmd);
        self
    }

    pub fn hincr<K, F>(mut self, key: K, field: F, delta: i64) -> Self
    where
        K: redis::ToRedisArgs,
        F: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("HINCR");
        cmd.arg(key).arg(field).arg(delta);
        self.commands.push(cmd);
        self
    }

    pub fn ltrim<K>(mut self, key: K, start: isize, stop: isize) -> Self
    where
        K: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("LTRIM");
        cmd.arg(key).arg(start).arg(stop);
        self.commands.push(cmd);
        self
    }

    pub fn set_ex<K, V>(mut self, key: K, value: V, seconds: u64) -> Self
    where
        K: redis::ToRedisArgs,
        V: redis::ToRedisArgs,
    {
        let mut cmd = redis::cmd("SETEX");
        cmd.arg(key).arg(seconds).arg(value);
        self.commands.push(cmd);
        self
    }

    pub async fn execute(self, redis: &ResilientRedisConnection) -> Result<Vec<redis::Value>> {
        redis.execute_commands(self.commands).await
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl std::fmt::Debug for ResilientRedisConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResilientRedisConnection")
            .field("redis_url", &self.redis_url)
            .finish()
    }
}
