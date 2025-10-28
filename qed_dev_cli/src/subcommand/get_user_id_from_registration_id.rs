use clap::Parser;
use qed_core::config::network_constants::{REALM_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, COORDINATOR_USER_TREE_HEIGHT, GROUP_REALM_HEIGHT};
use psy_crypto::common::user_id::{self, UserIdBitsStrategy1, UserIdBitsStrategy2, UserIdBitsStrategy3, UserIdBitsStrategy4, UserIdGeneratorStrategy};

#[derive(Parser)]
pub struct GetUserIdFromRegistrationIdArgs {
    #[arg(help = "Registration ID to convert")]
    pub registration_id: u64,
    
    #[arg(long, short, default_value = "4", help = "Strategy to use (1, 2, 3, or 4)")]
    pub strategy: u8,
}


pub async fn run(args: GetUserIdFromRegistrationIdArgs) -> anyhow::Result<()> {
    let registration_id = args.registration_id;
    
    let user_id = match args.strategy {
        1 => UserIdBitsStrategy1::get_user_id_from_registration_id(registration_id),
        2 => UserIdBitsStrategy2::get_user_id_from_registration_id(registration_id),
        3 => UserIdBitsStrategy3::get_user_id_from_registration_id(registration_id),
        4 => UserIdBitsStrategy4::get_user_id_from_registration_id(registration_id),
        _ => anyhow::bail!("Invalid strategy. Please use 1, 2, 3, or 4"),
    };
    
    let realm = user_id >> REALM_USER_TREE_HEIGHT;
    
    println!("Strategy: {}", args.strategy);
    println!("Registration ID: {}", registration_id);
    println!("User ID: {}", user_id);
    println!("Realm: {}", realm);
    
    // Show note about current system default
    if args.strategy == 4 {
        println!("Note: This is the currently active strategy in the system");
    }
    
    Ok(())
}