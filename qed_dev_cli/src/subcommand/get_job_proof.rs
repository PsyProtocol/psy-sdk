use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use qed_core::job::id::{QProvingJobDataID, JobProof};
use serde_json::json;

use crate::subcommand::GetJobProofArgs;

pub async fn run(args: GetJobProofArgs) -> Result<()> {
    // Parse the job ID from hex string
    let job_id = parse_job_id(&args.job_id)?;
    
    println!("Fetching job proof for:");
    println!("  Checkpoint ID: {}", args.checkpoint_id);
    println!("  Job ID: {:?}", job_id);
    println!("  Coordinator URL: {}", args.coordinator_url);
    
    // Use generate_batch_proofs to get the complete proof with merkle siblings
    // This will use the saved JobsTaskGraph to generate the full proof
    let proof = generate_batch_proof(&args.coordinator_url, args.checkpoint_id, job_id).await?;
    
    // Print the result
    println!("\n=== Job Proof ===");
    println!("Job ID: {:?}", proof.job_id);
    println!("Value: {:?}", proof.value);
    println!("Root: {:?}", proof.root);
    println!("Number of siblings: {}", proof.siblings.len());
    
    for (idx, sibling) in proof.siblings.iter().enumerate() {
        println!("  Sibling[{}]:", idx);
        println!("    Hash: {:?}", sibling.hash);
        println!("    Is Left: {}", sibling.is_left);
        if let Some(parent_key) = &sibling.parent_public_key {
            println!("    Parent Public Key: {:?}", parent_key);
        }
    }
    
    // Also output as JSON for easy parsing
    println!("\n=== JSON Output ===");
    let json_output = serde_json::to_string_pretty(&proof)?;
    println!("{}", json_output);
    
    Ok(())
}

fn parse_job_id(hex_str: &str) -> Result<QProvingJobDataID> {
    // Remove 0x prefix if present
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    
    // Decode hex string to bytes
    let bytes = hex::decode(hex_str)?;
    
    // QProvingJobDataID is 24 bytes
    if bytes.len() != 24 {
        anyhow::bail!("Invalid job ID length: expected 24 bytes, got {}", bytes.len());
    }
    
    // Parse the job ID from bytes
    let job_id = QProvingJobDataID::try_from_byte_vec(&bytes)?;
    Ok(job_id)
}

async fn generate_batch_proof(
    coordinator_url: &str,
    checkpoint_id: u64,
    job_id: QProvingJobDataID,
) -> Result<JobProof> {
    let client = HttpClientBuilder::default()
        .build(coordinator_url)?;
    
    // Call generate_batch_proofs RPC method with a single job ID
    let job_ids = vec![job_id];
    let proofs: Vec<JobProof> = client
        .request("qed_generate_batch_proofs", rpc_params![checkpoint_id, job_ids])
        .await?;
    
    // We expect exactly one proof back
    if proofs.is_empty() {
        anyhow::bail!("No proof returned for job ID {:?}", job_id);
    }
    
    Ok(proofs.into_iter().next().unwrap())
}