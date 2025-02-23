use futures::StreamExt;
use qed_core::{job::id::QProvingJobDataID, utils::debug_timer::DebugTimer};
use rabbitmq_stream_client::{
    error::StreamCreateError,
    types::{ByteCapacity, Message, OffsetSpecification, ResponseCode},
    Environment, TlsConfiguration,
};

fn gen_jobs_ids(checkpoint_id: u64, height: usize) -> Vec<Vec<QProvingJobDataID>> {
    let mut jobs = Vec::with_capacity(height);
    for h in 0..=height {
        let level = height-h;

        let num_nodes = 1usize<<level;
        let mut level_jobs = Vec::with_capacity(num_nodes);
        if h == 0 {
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_end_cap_witness(checkpoint_id, h as u32, i as u32);
                level_jobs.push(id);
            }
        }else{
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_agg_witness(checkpoint_id, h as u32, i as u32);
                level_jobs.push(id);
            }
        }

         jobs.push(level_jobs);
    }
    jobs

}


struct RabbitStreamQueueV2 {
    env: Environment,
    stream: String,
}

impl RabbitStreamQueueV2 {
    pub async fn init(&self) -> anyhow::Result<()> {
        let create_response = self
            .env
            .stream_creator()
            .max_length(ByteCapacity::GB(2))
            .create(&self.stream)
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

        Ok(())
    }
    pub async fn send_messages(&self, ids: &[QProvingJobDataID]) -> anyhow::Result<()> {
        let producer = self.env.producer().build(&self.stream).await?;
        let mut messages = Vec::with_capacity(ids.len());
        for i in 0..ids.len() {
            let msg = Message::builder().body(ids[i].to_fixed_bytes()).build();
            messages.push(msg);
        }

        producer
            .batch_send(messages, |confirmation_status| async move {
                println!("Message confirmed with status {:?}", confirmation_status);
            })
            .await?;

        producer.close().await?;

        Ok(())
    }

    pub async fn get_next_message(&self) -> anyhow::Result<QProvingJobDataID> {
        let mut consumer = self
            .env
            .consumer()
            .offset(OffsetSpecification::First)
            .build(&self.stream)
            .await
            .unwrap();
        let mut jid = QProvingJobDataID::notify_block_complete(0);

        while let Some(Ok(delivery)) = consumer.next().await {
            let jidz = delivery
                .message()
                .data()
                .map(|data| QProvingJobDataID::try_from_byte_vec(data));
            match jidz {
                Some(x) => match x {
                    Ok(r) => {
                        jid = r;
                        break;
                    }
                    Err(_) => {}
                },
                None => {}
            }
        }

        let _ = consumer.handle().close().await;

        Ok(jid)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut timer = DebugTimer::new("rb1");
    let stream = "job_test123";
    let environment = Environment::builder()
        .host("localhost")
        .port(5552)
        .build()
        .await?;

    let _ = environment.delete_stream(stream).await;
    let messages_to_batch = 1 << 15;
    let iterations = 100;

    let q = RabbitStreamQueueV2 {
        env: environment,
        stream: stream.to_string(),
    };
    q.init().await?;

    let jobs = gen_jobs_ids(1,15);

    

    q.send_messages(&jobs[0]).await?;
    let result = q.get_next_message().await?;


    println!("result: {:?}",result);

    Ok(())
}
