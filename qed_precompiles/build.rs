use anyhow::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = env::var("OUT_DIR")?;
    let out_path = std::path::Path::new(&out_dir);

    println!("cargo:rerun-if-changed=rewards/");
    println!("cargo:rerun-if-changed=token/");

    let contracts = [
        (
            "rewards",
            "batch_claim_pm_rewards,simple_mint,simple_burn,simple_transfer,simple_claim",
        ),
        (
            "token",
            "simple_mint,simple_burn,simple_transfer,simple_claim",
        ),
    ];

    for (contract_name, _) in &contracts {
        let json_file = out_path.join(format!("{}.json", contract_name));
        if !json_file.exists() {
            std::fs::write(&json_file, "{}").unwrap_or_else(|e| {
                println!(
                    "cargo:warning=Failed to create initial empty JSON for {}: {}",
                    contract_name, e
                );
            });
        }
    }

    for (contract_name, method_names) in contracts {
        let contract_dir = Path::new(&manifest_dir).join(contract_name);

        if !contract_dir.exists() {
            println!(
                "cargo:warning=Contract directory {} does not exist",
                contract_dir.display()
            );
            continue;
        }

        let dargo_toml = contract_dir.join("Dargo.toml");
        if !dargo_toml.exists() {
            println!(
                "cargo:warning=Dargo.toml not found at {}",
                dargo_toml.display()
            );
            continue;
        }

        let main_qed = contract_dir.join("src/main.qed");
        if !main_qed.exists() {
            println!(
                "cargo:warning=src/main.qed not found at {}",
                main_qed.display()
            );
            continue;
        }

        use dargo::cli::{with_workspace, DargoConfig};
        use dargo::compile_cmd::CompileOptions;

        let config = DargoConfig {
            program_dir: contract_dir.clone(),
            target_dir: None,
        };

        let compile_options = CompileOptions {
            contract_name: Some("ContractRef".to_string()),
            method_names: method_names
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            entry_path: Some(main_qed),
            debug: false,
        };

        let result = with_workspace(compile_options, config, |opts, workspace| {
            use dargo::cli::resolve_crate_path_graph;
            let crate_path_graph = resolve_crate_path_graph(&workspace, opts.entry_path.clone());

            match qed_interpreter::interpret(
                opts.contract_name.clone(),
                opts.method_names.clone(),
                crate_path_graph,
            ) {
                Ok(mut interpret_result) => {
                    use dargo::cli::doc_cmd::extract_function_metadata_from_context;
                    let _function_metadata = extract_function_metadata_from_context(
                        &mut interpret_result.ctx,
                        &mut interpret_result.typechecker,
                        &interpret_result.compile_results,
                    );

                    use dargo::cli::save_build_artifact_to_file;
                    let out_dir = env::var("OUT_DIR").map_err(anyhow::Error::from)?;
                    let out_path = std::path::Path::new(&out_dir);

                    if let Err(e) = save_build_artifact_to_file(
                        &interpret_result.compile_results,
                        contract_name,
                        out_path,
                    ) {
                        println!(
                            "cargo:warning=Failed to save build artifact for {}: {}",
                            contract_name, e
                        );
                    }

                    if let Err(e) = save_build_artifact_to_file(
                        &interpret_result.compile_results,
                        contract_name,
                        &workspace.target_dir,
                    ) {
                        println!(
                            "cargo:warning=Could not save {} to workspace target_dir: {}",
                            contract_name, e
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    println!(
                        "cargo:warning=Failed to compile {} contract: {}",
                        contract_name, e
                    );
                    Err(e.into())
                }
            }
        });

        if let Err(_) = result {
            let out_dir = env::var("OUT_DIR").unwrap_or_else(|_| ".".to_string());
            let out_path = std::path::Path::new(&out_dir);
            let json_file = out_path.join(format!("{}.json", contract_name));

            if let Err(write_err) = std::fs::write(&json_file, "{}") {
                println!(
                    "cargo:warning=Failed to write empty JSON file for {}: {}",
                    contract_name, write_err
                );
            }
        }
    }

    Ok(())
}
