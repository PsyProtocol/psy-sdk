use futures::StreamExt;
use lapin::{options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions}, types::FieldTable, BasicProperties, Channel, Connection, Consumer};
use qed_core::job::id::QProvingJobDataID;



pub struct RabbitMQQueue {
    // we use queue here because pubsub is mpmc
    pub channel: Channel,
    pub consumer: Option<Consumer>,
    stream: String,
    
}


impl RabbitMQQueue {
    pub async fn new(channel: Channel, stream: &str) ->anyhow::Result<Self> {
        let _queue = channel
            .queue_declare(
                stream,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
    
        

            Ok(Self{
                channel,
                consumer: None,
                stream: stream.to_string(),
            })
    
    }
    pub async fn push_id(&mut self, id: QProvingJobDataID) -> anyhow::Result<()> {

    self.channel
    .basic_publish(
        "",
        &self.stream,
        BasicPublishOptions::default(),
        &id.to_fixed_bytes(),
        BasicProperties::default(),
    )
    .await?.await?;

    Ok(())
    }

    pub async fn push_tasks(&mut self, tasks: &[QProvingJobDataID]) -> anyhow::Result<()> {
        
        for t in tasks {
            self.push_id(*t).await?;
        }
        Ok(())
    }
    pub async fn get_next_task(&mut self) -> anyhow::Result<QProvingJobDataID> {

        if self.consumer.is_none() {
            let mut opts = BasicConsumeOptions::default();
            opts.no_ack = true;
            let consumer = self.channel
            .basic_consume(
                &self.stream,
                "tag_con",
                opts,
                FieldTable::default(),
            ).await?;
            self.consumer = Some(consumer);
        }
        
        while let Some(Ok(delivery)) = self.consumer.as_mut().unwrap().next().await {
            //println!("got task");
        /* 
            delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack send_webhook_event message");*/

            if delivery.data.len() == 24 {
                return Ok(QProvingJobDataID::try_from_byte_vec(&delivery.data)?);
            }

        }
        println!("bad");
        Ok(QProvingJobDataID::end_cap_proof(0,0,0))
    }
    pub async fn cleanup(self) -> anyhow::Result<()>{
        // self.connection.close()
        //self.producer.close().await?;
        Ok(())

    }
}