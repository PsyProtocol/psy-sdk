use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use qed_core::job::id::QProvingJobDataID;
use qed_core::job::worker_queue::ProvingDispatcher;
use qed_core::job::worker_queue::ProvingWorkerListener;
use rabbitmq_stream_client::error::StreamCreateError;
use rabbitmq_stream_client::types::ByteCapacity;
use rabbitmq_stream_client::types::Message;
use rabbitmq_stream_client::types::OffsetSpecification;
use rabbitmq_stream_client::types::ResponseCode;
use rabbitmq_stream_client::Consumer;
use rabbitmq_stream_client::Environment;
use rabbitmq_stream_client::NoDedup;
use rabbitmq_stream_client::Producer;
use rsmq::PooledRsmq;
use rsmq::RedisConnectionManager;
use rsmq::RsmqConnection;
use rsmq::RsmqError;
use rsmq::RsmqMessage;
use serde::Serialize;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;

pub struct RabbitStreamQueue {
    // we use queue here because pubsub is mpmc
    environment: Environment,
    consumer: Consumer,
    producer: Producer<NoDedup>,
}

pub const Q_HIDDEN: Option<Duration> = Some(Duration::from_secs(600));
pub const Q_DELAY: Option<Duration> = None;
pub const Q_CAP: Option<i32> = Some(-1);

pub const Q_CMD: &'static str = "CMD";
pub const Q_JOB: &'static str = "JOB";
pub const Q_NOTIFICATIONS: &'static str = "NOTIFICATIONS";

#[derive(Clone, Copy, PartialEq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum QueueCmd {
    ProduceBlock = 0,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum QueueNotification {
    CoreJobCompleted = 0,
}

impl RabbitStreamQueue {
    pub async fn new(environment: Environment, stream: &str) -> Result<Self> {
        let create_response = environment
            .stream_creator()
            .max_length(ByteCapacity::GB(2))
            .create(stream)
            .await;

        if let Err(e) = create_response {
            if let StreamCreateError::Create { stream, status } = e {
                match status {
                    // we can ignore this error because the stream already exists
                    ResponseCode::StreamAlreadyExists => {}
                    err => {
                        anyhow::bail!("Error creating stream: {:?} {:?}", stream, err);
                    }
                }
            }
        }
        let consumer = environment
            .consumer()
            .offset(OffsetSpecification::First)
            .build(stream)
            .await?;

        let producer: Producer<NoDedup> = environment.producer().build(stream).await?;

        Ok(Self {
            environment,
            consumer,
            producer,
        })
    }
    pub async fn push_tasks(&mut self, tasks: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let mut messages = Vec::with_capacity(tasks.len());
        for i in 0..tasks.len() {
            let msg = Message::builder().body(tasks[i].to_fixed_bytes()).build();
            messages.push(msg);
        }
        self.producer
            .batch_send(messages, |confirmation_status| async move {
                //println!("Message confirmed with status {:?}", confirmation_status);
            })
            .await?;
        
        Ok(())
    }
    pub async fn get_next_task(&mut self) -> anyhow::Result<QProvingJobDataID> {



        while let Some(Ok(delivery)) = self.consumer.next().await {
            let data =match delivery
            .message()
            .data() {
                Some(d) => Some(QProvingJobDataID::try_from_byte_vec(d)?),
                None => None,
            };
            if data.is_some() {
                return Ok(data.unwrap());
            }
        }
        println!("bad");
        Ok(QProvingJobDataID::end_cap_proof(0,0,0))
    }
    pub async fn cleanup(self) -> anyhow::Result<()>{
        self.consumer.handle().close().await?;
        self.producer.close().await?;
        Ok(())

    }
}
