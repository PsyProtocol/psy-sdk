use anyhow::Result;
use std::env;
use std::path::Path;
use std::process::Command;

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR")?;
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    
    // Get the path to the dargo binary
    let dargo_path = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .join("target/release/dargo");
    
    // If dargo doesn't exist in release, try debug
    let dargo_path = if !dargo_path.exists() {
        Path::new(&manifest_dir)
            .parent()
            .unwrap()
            .join("target/debug/dargo")
    } else {
        dargo_path
    };
    
    println!("cargo:rerun-if-changed=rewards/");
    println!("cargo:rerun-if-changed=token/");
    
    // Compile rewards contract
    let rewards_dir = Path::new(&manifest_dir).join("rewards");
    compile_contract(&dargo_path, &rewards_dir, "rewards")?;
    
    // Compile token contract  
    let token_dir = Path::new(&manifest_dir).join("token");
    compile_contract(&dargo_path, &token_dir, "token")?;
    
    Ok(())
}

fn compile_contract(dargo_path: &Path, contract_dir: &Path, contract_name: &str) -> Result<()> {
    if !contract_dir.exists() {
        println!("cargo:warning=Contract directory {} does not exist", contract_dir.display());
        return Ok(());
    }
    
    let output = Command::new(dargo_path)
        .current_dir(contract_dir)
        .arg("compile")
        .arg("--entry-path")
        .arg("src/main.qed")
        .arg("--contract-name")
        .arg("ContractRef")
        .arg("--method-names")
        .arg("claim_batch_job_rewards,simple_transfer,simple_claim,get_balance,simple_mint,simple_burn")
        .output();
    
    match output {
        Ok(output) => {
            if !output.status.success() {
                println!("cargo:warning=Failed to compile {} contract: {}", 
                    contract_name, 
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                println!("cargo:warning=Successfully compiled {} contract", contract_name);
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute dargo for {}: {}", contract_name, e);
        }
    }
    
    Ok(())
}