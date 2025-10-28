use std::{env, path::Path};

use anyhow::Result;
use minijinja::{context, Environment};
use psy_data::qdata::contract::{ContractConfig, PrecompileConfig, RootConfig};

fn main() -> Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let out_dir = env::var("OUT_DIR")?;
    let out_path = std::path::Path::new(&out_dir);

    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not find workspace root"))?;
    let config_path = workspace_root.join("config.json");

    println!("cargo:rerun-if-changed={}", config_path.display());
    let config_str =
        std::fs::read_to_string(&config_path).map_err(|e| anyhow::anyhow!("Failed to read config.json at {}: {}", config_path.display(), e))?;

    let root_config: RootConfig = serde_json::from_str(&config_str).map_err(|e| anyhow::anyhow!("Failed to parse config.json: {}", e))?;

    let config = PrecompileConfig {
        contracts: root_config.genesis.precompiles,
    };

    for contract in &config.contracts {
        println!("cargo:rerun-if-changed={}/", contract.path);
    }

    for contract in &config.contracts {
        let json_file = out_path.join(format!("{}.json", contract.name));
        if !json_file.exists() {
            std::fs::write(&json_file, "[]").unwrap_or_else(|e| {
                println!("cargo:warning=Failed to create initial empty JSON for {}: {}", contract.name, e);
            });
        }
    }

    for contract in &config.contracts {
        let contract_dir = workspace_root.join(&contract.path);

        if !contract_dir.exists() {
            println!("cargo:warning=Contract directory {} does not exist", contract_dir.display());
            continue;
        }

        let dargo_toml = contract_dir.join("Dargo.toml");
        if !dargo_toml.exists() {
            println!("cargo:warning=Dargo.toml not found at {}", dargo_toml.display());
            continue;
        }

        let main_qed = contract_dir.join("src/main.qed");
        if !main_qed.exists() {
            println!("cargo:warning=src/main.qed not found at {}", main_qed.display());
            continue;
        }

        use dargo::{
            cli::{with_workspace, DargoConfig},
            compile_cmd::CompileOptions,
        };

        let target_dir = contract_dir.join("target");
        let dargo_config = DargoConfig {
            program_dir: contract_dir.clone(),
            target_dir: Some(target_dir.clone()),
        };

        let compile_options = CompileOptions {
            contract_name: Some(contract.contract_name.clone()),
            method_names: contract.method_names.clone(),
            entry_path: Some(main_qed),
            debug: false,
        };

        with_workspace(compile_options, dargo_config, |opts, workspace| {
            use dargo::cli::resolve_crate_path_graph;
            let crate_path_graph = resolve_crate_path_graph(&workspace, opts.entry_path.clone());
            match psy_interpreter::interpret(opts.contract_name.clone(), opts.method_names.clone(), crate_path_graph) {
                Ok(mut interpret_result) => {
                    use dargo::cli::doc_cmd::extract_function_metadata_from_context;
                    let _function_metadata = extract_function_metadata_from_context(
                        &mut interpret_result.ctx,
                        &mut interpret_result.typechecker,
                        &interpret_result.compile_results,
                    );

                    use dargo::cli::save_build_artifact_to_file;

                    if let Err(e) = save_build_artifact_to_file(&interpret_result.compile_results, &contract.name, &target_dir) {
                        println!("cargo:warning=Failed to save build artifact to target dir for {}: {}", contract.name, e);
                    }

                    let out_dir = env::var("OUT_DIR").map_err(anyhow::Error::from)?;
                    let out_path = std::path::Path::new(&out_dir);

                    if let Err(e) = save_build_artifact_to_file(&interpret_result.compile_results, &contract.name, out_path) {
                        println!("cargo:warning=Failed to save build artifact to OUT_DIR for {}: {}", contract.name, e);
                    }
                    Ok(())
                }
                Err(e) => {
                    println!("cargo:warning=Failed to compile {} contract: {}", contract.name, e);
                    Err(e.into())
                }
            }
        })?;
    }

    generate_precompile_api(&config, out_path)?;

    generate_precompile_constants_with_method_ids(&config, out_path)?;

    Ok(())
}

fn generate_precompile_api(config: &PrecompileConfig, out_path: &Path) -> Result<()> {
    let mut env = Environment::new();

    let template = r#"use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;

// Generated at compile time - zero runtime parsing cost!
// Direct embedded contract definitions - no JSON parsing at runtime!

{% for contract in contracts -%}
// Runtime lazy loading for {{ contract.name }} contract
fn load_{{ contract.name }}_functions() -> Vec<DPNFunctionCircuitDefinition> {
    let json_data = include_str!(concat!(env!("OUT_DIR"), "/{{ contract.name }}.json"));
    serde_json::from_str(json_data).unwrap_or_else(|_| Vec::new())
}

static {{ contract.name | upper }}_FUNCTIONS_ONCE: std::sync::OnceLock<Vec<DPNFunctionCircuitDefinition>> = std::sync::OnceLock::new();

pub fn get_{{ contract.name }}_functions() -> &'static Vec<DPNFunctionCircuitDefinition> {
    {{ contract.name | upper }}_FUNCTIONS_ONCE.get_or_init(load_{{ contract.name }}_functions)
}

{% endfor -%}

/// Get precompiled contract functions by contract ID
pub fn get_precompiled_contract_functions(contract_id: u32) -> Option<&'static Vec<DPNFunctionCircuitDefinition>> {
    match contract_id {
        {%- for contract in contracts %}
        {{ contract.name | upper }}_CONTRACT_ID => Some(get_{{ contract.name }}_functions()),
        {%- endfor %}
        _ => None,
    }
}

/// Get contract ID by name
pub fn get_contract_id_by_name(contract_name: &str) -> Option<u32> {
    match contract_name {
        {%- for contract in contracts %}
        "{{ contract.name }}" => Some({{ contract.name | upper }}_CONTRACT_ID),
        {%- endfor %}
        _ => None,
    }
}

/// Get a specific function from a contract by name
pub fn get_precompiled_contract_function_by_name(
    contract_name: &str,
    method_name: &str
) -> Option<&'static DPNFunctionCircuitDefinition> {
    let contract_id = get_contract_id_by_name(contract_name)?;
    let functions = get_precompiled_contract_functions(contract_id)?;

    functions.iter().find(|f| f.name == method_name)
}

/// List all available contract names
pub fn list_available_contracts() -> Vec<String> {
    vec![
        {%- for contract in contracts %}
        "{{ contract.name }}".to_string(),
        {%- endfor %}
    ]
}

/// List all methods for a specific contract
pub fn list_contract_methods(contract_name: &str) -> Vec<String> {
    if let Some(contract_id) = get_contract_id_by_name(contract_name) {
        if let Some(functions) = get_precompiled_contract_functions(contract_id) {
            return functions.iter().map(|f| f.name.clone()).collect();
        }
    }
    Vec::new()
}
"#;

    env.add_template("api", template)?;
    let tmpl = env.get_template("api")?;

    let api_code = tmpl.render(context! {
        contracts => &config.contracts
    })?;

    let api_file = out_path.join("precompile_api.rs");
    std::fs::write(api_file, api_code)?;

    Ok(())
}

fn generate_precompile_constants_with_method_ids(config: &PrecompileConfig, out_path: &Path) -> Result<()> {
    use serde_json::Value;

    let mut contracts_with_methods = Vec::new();

    for contract in &config.contracts {
        let json_file = out_path.join(format!("{}.json", contract.name));
        let methods_with_ids = extract_method_ids_from_json(&json_file, &contract.method_names)?;

        contracts_with_methods.push(serde_json::json!({
            "name": contract.name,
            "methods": methods_with_ids
        }));
    }

    let template = r#"// This file is auto-generated by build.rs from config.json
// DO NOT EDIT MANUALLY

// Precompiled contract IDs
{% for contract in contracts -%}
pub const {{ contract.name | upper }}_CONTRACT_ID: u32 = {{ loop.index0 }};
{% endfor %}

// Method ID constants for each contract (compile-time constants)
{% for contract in contracts -%}
// {{ contract.name }} contract method IDs
{%- for method_data in contract.methods %}
pub const {{ contract.name | upper }}_{{ method_data.name | upper }}_METHOD_ID: u32 = {{ method_data.method_id }};
{% endfor %}
{% endfor -%}
"#;

    let mut env = Environment::new();
    env.add_template("constants", template)?;
    let tmpl = env.get_template("constants")?;

    let constants_code = tmpl.render(context! { contracts => contracts_with_methods })?;
    let constants_file = out_path.join("precompiled_contracts.rs");
    std::fs::write(constants_file, constants_code)?;

    Ok(())
}

fn extract_method_ids_from_json(json_file: &std::path::Path, method_names: &[String]) -> Result<Vec<serde_json::Value>> {
    use serde_json::Value;

    if !json_file.exists() {
        return Ok(Vec::new());
    }

    let json_content = std::fs::read_to_string(json_file)?;
    let functions: Vec<Value> = serde_json::from_str(&json_content).unwrap_or_default();

    let methods = method_names
        .iter()
        .filter_map(|method_name| {
            functions.iter().find_map(|func| {
                if func["name"].as_str() == Some(method_name) {
                    if let Some(method_id) = func["method_id"].as_u64() {
                        return Some(serde_json::json!({
                            "name": method_name,
                            "method_id": method_id as u32
                        }));
                    }
                }
                None
            })
        })
        .collect();

    Ok(methods)
}
