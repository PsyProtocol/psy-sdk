use psy_common::jwt::{decrypt_jwt_token, generate_jwt_token};

pub async fn run(args: super::GenerateTokenArgs) -> anyhow::Result<()> {
    let token = generate_jwt_token(&args.private_key, args.realm_id)?;
    assert_eq!(decrypt_jwt_token(&args.private_key, &token)?.realm_id, args.realm_id);
    tracing::info!("Generated JWT token: {} for realm {}", token, args.realm_id);
    Ok(())
}
