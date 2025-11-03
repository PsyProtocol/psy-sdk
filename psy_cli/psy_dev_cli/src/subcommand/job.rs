use anyhow::Result;
use clap::{command, Args, Subcommand};
use psy_common::job::id::QProvingJobDataID;

#[derive(Args)]
pub struct JobArgs {
    #[command(subcommand)]
    pub command: JobCommand,
}

#[derive(Subcommand)]
pub enum JobCommand {
    #[command(about = "Decode job ID from hex string")]
    Decode(DecodeArgs),
}

#[derive(Args)]
pub struct DecodeArgs {
    /// Job ID in hex format
    pub job_id: String,
}

pub async fn run(args: JobArgs) -> Result<()> {
    match args.command {
        JobCommand::Decode(decode_args) => decode_job_id(decode_args).await,
    }
}

async fn decode_job_id(args: DecodeArgs) -> Result<()> {
    let job_id_hex = args.job_id.strip_prefix("0x").unwrap_or(&args.job_id);
    let job_id_bytes = hex::decode(job_id_hex).map_err(|e| anyhow::format_err!("Failed to decode hex: {}", e))?;

    let job_id = QProvingJobDataID::try_from_byte_vec(&job_id_bytes).map_err(|e| anyhow::format_err!("Failed to parse job ID: {}", e))?;

    println!("Job ID: {}", job_id.to_hex_string());
    println!("  Topic: {:?}", job_id.topic);
    println!("  Goal ID: {}", job_id.goal_id);
    println!("  Slot ID: {}", job_id.slot_id);
    println!("  Circuit Type: {:?}", job_id.circuit_type);
    println!("  Group ID: {}", job_id.group_id);
    println!("  Sub Group ID: {}", job_id.sub_group_id);
    println!("  Task Index: {}", job_id.task_index);
    println!("  Data Type: {:?}", job_id.data_type);
    println!("  Data Index: {}", job_id.data_index);

    Ok(())
}
