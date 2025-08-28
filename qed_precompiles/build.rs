use anyhow::Result;
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    if env::var("DARGO_STD_PATH").is_err() {
        let std_path = env::var("CARGO_MANIFEST_DIR").unwrap() + "/../qed_compiler/qed-std/std.qed";
        let canonical_std_path = std::path::Path::new(&std_path)
            .canonicalize()
            .expect("Failed to canonicalize DARGO_STD_PATH");
        env::set_var("DARGO_STD_PATH", &canonical_std_path);
    }

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

        use dargo::compile_cmd::CompileOptions;
        use qed_package::{
            files::{find_file_manifest_root, get_package_manifest},
            resolve_workspace_from_toml,
        };

        let package_dir = match find_file_manifest_root(&contract_dir) {
            Ok(dir) => dir,
            Err(e) => {
                println!(
                    "cargo:warning=Failed to find manifest root for {}: {}",
                    contract_name, e
                );
                continue;
            }
        };

        let toml_path = match get_package_manifest(&package_dir) {
            Ok(path) => path,
            Err(e) => {
                println!(
                    "cargo:warning=Failed to get package manifest for {}: {}",
                    contract_name, e
                );
                continue;
            }
        };

        let workspace = match resolve_workspace_from_toml(&toml_path) {
            Ok(ws) => ws,
            Err(e) => {
                println!(
                    "cargo:warning=Failed to resolve workspace for {}: {}",
                    contract_name, e
                );
                continue;
            }
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

        use dargo::cli::resolve_crate_path_graph;
        let crate_path_graph =
            resolve_crate_path_graph(&workspace, compile_options.entry_path.clone());

        match qed_interpreter::interpret(
            compile_options.contract_name.clone(),
            compile_options.method_names.clone(),
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
                let out_dir = env::var("OUT_DIR")?;
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
            }
            Err(e) => {
                println!(
                    "cargo:warning=Failed to compile {} contract: {}",
                    contract_name, e
                );

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
    }

    Ok(())
}
