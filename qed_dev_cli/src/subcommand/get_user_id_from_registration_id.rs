use clap::Parser;
use qed_core::config::network_constants::{REALM_USER_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT, COORDINATOR_USER_TREE_HEIGHT, GROUP_REALM_HEIGHT};

#[derive(Parser)]
pub struct GetUserIdFromRegistrationIdArgs {
    #[arg(help = "Registration ID to convert")]
    pub registration_id: u64,
    
    #[arg(long, short, default_value = "4", help = "Strategy to use (1, 2, 3, or 4)")]
    pub strategy: u8,
}

fn reverse_bits_in_limit(x: u64, num_bits: u8) -> u64 {
    let dif = 64 - num_bits as u64;
    (x).reverse_bits() >> dif
}

fn get_user_id_from_registration_id_strategy1(registration_id: u64) -> u64 {
    let dif = 64 - GLOBAL_USER_TREE_HEIGHT as u64;
    (registration_id).reverse_bits() >> dif
}

fn get_user_id_from_registration_id_strategy2(registration_id: u64) -> u64 {
    // rotate realms on each index
    let new_top_bits = reverse_bits_in_limit(registration_id&((1u64<<COORDINATOR_USER_TREE_HEIGHT)-1u64), COORDINATOR_USER_TREE_HEIGHT);
    
    // sequential within realms
    let new_bottom_bits = registration_id>>COORDINATOR_USER_TREE_HEIGHT;
    
    (new_top_bits<<REALM_USER_TREE_HEIGHT)|new_bottom_bits
}

fn get_user_id_from_registration_id_strategy3(registration_id: u64) -> u64 {
    (reverse_bits_in_limit(registration_id>>10u64, GLOBAL_USER_TREE_HEIGHT-10)<<10u64) |
    (registration_id & ((1u64<<10)-1u64))
}

fn get_user_id_from_registration_id_strategy4(registration_id: u64) -> u64 {
    let realm_index = registration_id & ((1u64 << GROUP_REALM_HEIGHT) - 1);
    let user_index = (registration_id >> GROUP_REALM_HEIGHT) & ((1u64 << REALM_USER_TREE_HEIGHT) - 1);
    let group_id = (registration_id >> (GROUP_REALM_HEIGHT + REALM_USER_TREE_HEIGHT)) & ((1u64 << (COORDINATOR_USER_TREE_HEIGHT - GROUP_REALM_HEIGHT)) - 1);

    let reversed_realm_index = reverse_bits_in_limit(realm_index, GROUP_REALM_HEIGHT);
    let realm_id = (group_id << GROUP_REALM_HEIGHT) | reversed_realm_index;

    (realm_id << REALM_USER_TREE_HEIGHT) | user_index
}

pub async fn run(args: GetUserIdFromRegistrationIdArgs) -> anyhow::Result<()> {
    let registration_id = args.registration_id;
    
    let user_id = match args.strategy {
        1 => get_user_id_from_registration_id_strategy1(registration_id),
        2 => get_user_id_from_registration_id_strategy2(registration_id),
        3 => get_user_id_from_registration_id_strategy3(registration_id),
        4 => get_user_id_from_registration_id_strategy4(registration_id),
        _ => anyhow::bail!("Invalid strategy. Please use 1, 2, 3, or 4"),
    };
    
    let realm = user_id >> REALM_USER_TREE_HEIGHT;
    
    println!("Strategy: {}", args.strategy);
    println!("Registration ID: {}", registration_id);
    println!("User ID: {}", user_id);
    println!("Realm: {}", realm);
    
    Ok(())
}