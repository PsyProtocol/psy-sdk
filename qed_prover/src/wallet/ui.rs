use anyhow::Result;
use rpassword::read_password;
use std::path::PathBuf;
use std::io::{self, Write};
use std::process::exit;
use crate::wallet::secp_wallet::Wallet;

const WALLET_FILENAME: &str = "worker_wallet.json";

fn get_wallet_path() -> PathBuf {
    //todo! change the path to .config
    let mut path = std::env::temp_dir();;
    path.push("worker_wallet");
    std::fs::create_dir_all(&path).ok();
    path.push(WALLET_FILENAME);
    path
}


pub fn wallet_ui() -> Result<Wallet> {
    print_logo();

    let wallet_path = get_wallet_path();
    println!("Wallet location: {}", wallet_path.display());

    // Check if wallet exists
    let wallet = if wallet_path.exists() {
        println!("Found existing wallet.");
        print!("Enter password to unlock wallet: ");
        io::stdout().flush()?;
        let password = read_password()?;

        match Wallet::load_from_file(&wallet_path, Some(password.clone())) {
            Ok(w) => {
                println!("✅ Wallet unlocked successfully!");
                println!("Worker ID (Public Key): {}", w.get_address());
                w
            }
            Err(e) => {
                eprintln!("❌ Failed to unlock wallet: {}", e);
                println!("Exiting...");
                exit(1);
            }
        }
    } else {
        println!("No wallet found. Creating new wallet...");

        print!("Enter password for new wallet (or press Enter for no password): ");
        io::stdout().flush()?;
        let password = read_password()?;
        let password = if password.is_empty() { None } else { Some(password) };

        let wallet = Wallet::new()?;
        wallet.save_to_file(&wallet_path, password.clone())?;

        println!("\n✓ New wallet created successfully!");
        println!("Worker ID (Public Key): {}", wallet.get_address());
        println!("⚠️  Please save this information securely!");

        wallet
    };

    Ok(wallet)
}

fn print_logo() {
    //print psy
    println!(r#"
 ____              __  __ _
|  _ \ ___ _   _  |  \/  (_)_ __   ___ _ __
| |_) / __| | | | | |\/| | | '_ \ / _ \ '__|
|  __/\__ \ |_| | | |  | | | | | |  __/ |
|_|   |___/\__, | |_|  |_|_|_| |_|\___|_|
           |___/
    "#)


}