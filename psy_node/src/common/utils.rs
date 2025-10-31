use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};
use plonky2::hash::hash_types::RichField;
use psy_data::config::store_config::{PsyFelt, PsyHasher};
use psy_data::qblock::cmds::deploy_contract::{QBCDeployContract, QContractMetadata, QFunctionMetadata};

pub fn extract_contract_name(contract_path: &str) -> String {
    // Handle empty input
    if contract_path.is_empty() {
        return String::new();
    }

    // Use Path for cross-platform compatibility and proper parsing
    let path = Path::new(contract_path);

    // Get the file name from the path
    let file_name = match path.file_name() {
        Some(name) => name,
        None => return String::new(),
    };

    // Convert OsStr to string, handling non-UTF8 gracefully
    let file_name_str = match file_name.to_str() {
        Some(name) => name,
        None => {
            // Try lossy conversion for non-UTF8 names
            return file_name.to_string_lossy().into_owned();
        }
    };

    // Remove file extension(s) to get the contract name
    // Handle multiple extensions like .tar.gz
    extract_base_name(file_name_str)
}

/// Helper function to extract base name by removing extensions
fn extract_base_name(file_name: &str) -> String {
    // Find the first dot to handle multiple extensions
    match file_name.find('.') {
        Some(pos) if pos > 0 => {
            // Return everything before the first dot
            file_name[..pos].to_string()
        }
        _ => {
            // No extension found or dot at the beginning (hidden file)
            file_name.to_string()
        }
    }
}

pub fn extract_contract_metadata(
    contract: &QBCDeployContract<PsyFelt>,
) -> anyhow::Result<QContractMetadata> {
    let mut functions = Vec::new();

    for function_def in &contract.code_definition.functions {
        // Deserialize the CBOR-encoded code to get the original DPNFunctionCircuitDefinition
        let dpn_def: DPNFunctionCircuitDefinition = serde_cbor::from_slice(&function_def.code)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize function code: {}", e))?;

        functions.push(QFunctionMetadata {
            method_id: function_def.method_id,
            name: dpn_def.name,
            num_inputs: function_def.num_inputs,
            num_outputs: function_def.num_outputs,
        });
    }

    // Calculate function whitelist root
    let with_root = contract.clone().into_with_whitelist_root::<PsyHasher>()?;

    Ok(QContractMetadata {
        contract_id: None, // Only can be assigned after deployment, reserved for now
        deployer: contract.deployer.to_string(),
        state_tree_height: contract.code_definition.state_tree_height,
        function_count: contract.code_definition.functions.len(),
        function_whitelist_root: with_root.function_whitelist_root.to_string(),
        functions,
    })
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn current_datetime() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extraction() {
        assert_eq!(
            extract_contract_name("${PROJECT_DIR}/token/target/token.json"),
            "token"
        );
    }

    #[test]
    fn test_absolute_path() {
        assert_eq!(
            extract_contract_name("/home/user/contracts/my_contract.json"),
            "my_contract"
        );
    }

    #[test]
    fn test_relative_path() {
        assert_eq!(
            extract_contract_name("./contracts/test.json"),
            "test"
        );
    }

    #[test]
    fn test_no_path_just_filename() {
        assert_eq!(extract_contract_name("contract.json"), "contract");
    }

    #[test]
    fn test_multiple_extensions() {
        assert_eq!(
            extract_contract_name("contract.tar.gz"),
            "contract"
        );
    }

    #[test]
    fn test_no_extension() {
        assert_eq!(extract_contract_name("contract"), "contract");
    }

    #[test]
    fn test_unicode_names() {
        assert_eq!(extract_contract_name("合约.json"), "合约");
        assert_eq!(extract_contract_name("контракт.json"), "контракт");
        assert_eq!(extract_contract_name("عقد.json"), "عقد");
    }

    #[test]
    fn test_special_characters() {
        assert_eq!(
            extract_contract_name("my-contract_v2.json"),
            "my-contract_v2"
        );
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(extract_contract_name(""), "");
        assert_eq!(extract_contract_name(".hidden"), ".hidden");
        assert_eq!(extract_contract_name("/"), "");
        assert_eq!(extract_contract_name("./"), "");
    }

    #[test]
    fn test_windows_paths() {
        assert_eq!(
            extract_contract_name("C:\\contracts\\token.json"),
            "token"
        );
        assert_eq!(
            extract_contract_name("..\\contracts\\my_contract.json"),
            "my_contract"
        );
    }

    #[test]
    fn test_spaces_in_name() {
        assert_eq!(
            extract_contract_name("my contract.json"),
            "my contract"
        );
    }
}