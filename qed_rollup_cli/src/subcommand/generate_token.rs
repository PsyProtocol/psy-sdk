use qed_rollup_utils::{decrypt_jwt_token, generate_jwt_token};

pub async fn run(private_key: &str, realm_id: u64) -> anyhow::Result<()> {
    let token = generate_jwt_token(private_key, realm_id)?;
    assert_eq!(decrypt_jwt_token(private_key, &token)?.realm_id, realm_id);
    tracing::info!("Generated JWT token: {} for realm {}", token, realm_id,);
    Ok(())
}
