use std::{env, fs, path::Path};

fn main() {
    let config_path = env::var("PSY_CONFIG_PATH").map(|p| Path::new(&p).to_path_buf()).unwrap_or_else(|_| {
        let mut current_dir = env::current_dir().expect("Failed to get current directory");
        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(cargo_content) = fs::read_to_string(&cargo_toml) {
                    if cargo_content.contains("[workspace]") {
                        return current_dir.join("config.json");
                    }
                }
            }
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                return Path::new("config.json").to_path_buf();
            }
        }
    });

    let network = env::var("PSY_NETWORK").unwrap_or_else(|_| "localhost".to_string());

    println!("cargo:rerun-if-changed={}", config_path.display());
    println!("cargo:rerun-if-env-changed=PSY_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=PSY_NETWORK");

    let config_content = fs::read_to_string(&config_path).unwrap_or_else(|_| panic!("Failed to read config file: {}", config_path.display()));

    let config: serde_json::Value = serde_json::from_str(&config_content).expect("Failed to parse config.json");

    let networks = config["networks"].as_object().expect("Config must have 'networks' object");

    let network_config = networks
        .get(&network)
        .unwrap_or_else(|| panic!("Network '{}' not found in config", network))
        .clone();

    let magic_str = network_config["magic"].as_str().expect("magic must be a hex string");
    let magic = if magic_str.starts_with("0x") || magic_str.starts_with("0X") {
        u64::from_str_radix(&magic_str[2..], 16).expect("Invalid hex magic value")
    } else {
        magic_str.parse::<u64>().expect("Invalid magic value")
    };

    let global_user_tree_height = network_config["global_user_tree_height"]
        .as_u64()
        .expect("global_user_tree_height must be a number") as u8;
    let realm_user_tree_height = network_config["realm_user_tree_height"]
        .as_u64()
        .expect("realm_user_tree_height must be a number") as u8;
    let users_per_realm = network_config["users_per_realm"].as_u64().expect("users_per_realm must be a number");
    let group_realm_height = network_config["group_realm_height"]
        .as_u64()
        .expect("group_realm_height must be a number") as u8;

    if global_user_tree_height < realm_user_tree_height {
        panic!(
            "global_user_tree_height ({}) must be >= realm_user_tree_height ({})",
            global_user_tree_height, realm_user_tree_height
        );
    }

    let expected_users_per_realm = 1u64 << realm_user_tree_height;
    if users_per_realm != expected_users_per_realm {
        panic!(
            "users_per_realm ({}) must equal 2^realm_user_tree_height (2^{} = {})",
            users_per_realm, realm_user_tree_height, expected_users_per_realm
        );
    }

    let coordinator_user_tree_height = global_user_tree_height - realm_user_tree_height;

    let native_currency_decimal = network_config["native_currency_decimal"]
        .as_u64()
        .expect("native_currency_decimal must be a number") as u8;
    let native_currency = network_config["native_currency"]
        .as_str()
        .expect("native_currency must be a string")
        .to_string();
    let native_currency_name = network_config["native_currency_name"]
        .as_str()
        .expect("native_currency_name must be a string")
        .to_string();

    let fees = &network_config["fees"];
    let register_user_fee = fees["register_user_fee"].as_u64().expect("register_user_fee must be a number");
    let deploy_contract_fee = fees["deploy_contract_fee"].as_u64().expect("deploy_contract_fee must be a number");
    let guta_fee = fees["guta_fee"].as_u64().expect("guta_fee must be a number");

    let coordinator_rpc_url = network_config["coordinator_configs"][0]["rpc_url"]
        .as_array()
        .expect("coordinator rpc_url must be an array")
        .first()
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let realm_rpc_urls = network_config["realm_configs"]
        .as_array()
        .expect("realm_configs must be an array")
        .iter()
        .map(|realm| {
            realm["rpc_url"]
                .as_array()
                .unwrap()
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect::<Vec<_>>();

    let genesis_constants = String::new();

    let precompile_constants = if let Some(genesis) = network_config.get("genesis") {
        generate_precompile_constants_with_method_ids(genesis).unwrap_or_default()
    } else {
        String::new()
    };

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_constants.rs");

    let content = format!(
        r#"pub const PSY_NETWORK_MAGIC: u64 = {};
pub const GLOBAL_USER_TREE_HEIGHT: u8 = {};
pub const COORDINATOR_USER_TREE_HEIGHT: u8 = {};
pub const REALM_USER_TREE_HEIGHT: u8 = {};
pub const GROUP_REALM_HEIGHT: u8 = {};
pub const USERS_PER_REALM: u64 = {};
pub const NATIVE_CURRENCY_DECIMAL: u8 = {};
pub const NATIVE_CURRENCY: &str = "{}";
pub const NATIVE_CURRENCY_NAME: &str = "{}";
pub const REGISTER_USER_FEE: u64 = {};
pub const DEPLOY_CONTRACT_FEE: u64 = {};
pub const GUTA_FEE: u64 = {};
pub const CURRENT_NETWORK: &str = "{}";
pub const COORDINATOR_RPC_URL: &str = "{}";
pub const REALM_RPC_URLS: &[&str] = &{:?};
{}
{}
"#,
        magic,
        global_user_tree_height,
        coordinator_user_tree_height,
        realm_user_tree_height,
        group_realm_height,
        users_per_realm,
        native_currency_decimal,
        native_currency,
        native_currency_name,
        register_user_fee,
        deploy_contract_fee,
        guta_fee,
        network,
        coordinator_rpc_url,
        realm_rpc_urls,
        genesis_constants,
        precompile_constants
    );

    fs::write(&dest_path, content).expect("Failed to write generated constants");

    let json_constants = serde_json::json!({
        "PSY_NETWORK_MAGIC": magic,
        "GLOBAL_USER_TREE_HEIGHT": global_user_tree_height,
        "COORDINATOR_USER_TREE_HEIGHT": coordinator_user_tree_height,
        "REALM_USER_TREE_HEIGHT": realm_user_tree_height,
        "GROUP_REALM_HEIGHT": group_realm_height,
        "USERS_PER_REALM": users_per_realm,
        "NATIVE_CURRENCY_DECIMAL": native_currency_decimal,
        "NATIVE_CURRENCY": native_currency,
        "NATIVE_CURRENCY_NAME": native_currency_name,
        "REGISTER_USER_FEE": register_user_fee,
        "DEPLOY_CONTRACT_FEE": deploy_contract_fee,
        "GUTA_FEE": guta_fee,
        "CURRENT_NETWORK": network,
        "COORDINATOR_RPC_URL": coordinator_rpc_url,
        "REALM_RPC_URLS": realm_rpc_urls
    });

    let json_path = Path::new(&out_dir).join("constants.json");
    fs::write(&json_path, serde_json::to_string_pretty(&json_constants).unwrap()).expect("Failed to write JSON constants");
}

fn generate_precompile_constants_with_method_ids(genesis: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let mut constants = String::new();

    if let Some(precompiles) = genesis.get("precompiles").and_then(|p| p.as_array()) {
        constants.push_str("\n");

        for (index, precompile) in precompiles.iter().enumerate() {
            let name = precompile
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| format!("Precompile at index {} is missing required 'name' field", index))?;
            let const_name = name.to_uppercase();
            constants.push_str(&format!("pub const {}_CONTRACT_ID: u32 = {};\n", const_name, index));
        }
        constants.push('\n');

        for precompile in precompiles {
            let contract_name = precompile
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| "Precompile is missing required 'name' field".to_string())?;
            let methods_with_ids = extract_method_ids_from_bytecode(precompile)?;

            if !methods_with_ids.is_empty() {
                constants.push_str("");
                for method in methods_with_ids {
                    if let (Some(method_name), Some(method_id)) = (
                        method.get("name").and_then(|n| n.as_str()),
                        method.get("method_id").and_then(|id| id.as_u64()),
                    ) {
                        constants.push_str(&format!(
                            "pub const {}_{}_METHOD_ID: u32 = {};\n",
                            contract_name.to_uppercase(),
                            method_name.to_uppercase(),
                            method_id as u32
                        ));
                    }
                }
                constants.push('\n');
            }
        }
    }

    Ok(constants)
}

fn extract_method_ids_from_bytecode(precompile: &serde_json::Value) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut methods = Vec::new();

    if let Some(bytecode) = precompile.get("bytecode").and_then(|b| b.as_array()) {
        for item in bytecode {
            if let Some(obj) = item.as_object() {
                if let (Some(name), Some(method_id)) = (obj.get("name").and_then(|n| n.as_str()), obj.get("method_id").and_then(|id| id.as_u64())) {
                    let method_name = name.split("::").last().unwrap_or(name);

                    methods.push(serde_json::json!({
                        "name": method_name,
                        "method_id": method_id as u32
                    }));
                }
            }
        }
    }

    Ok(methods)
}
