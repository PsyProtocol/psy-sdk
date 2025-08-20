use anyhow::Result;
use rpassword::read_password;
use std::path::PathBuf;
use std::io::{self, Write};
use std::process::exit;
use crate::wallet::secp_wallet::Wallet;

const WALLET_FILENAME: &str = "worker_wallet.json";

// Get default wallet path
pub fn default_wallet_path() -> PathBuf {
    let mut path = dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir());
    path.push("psy-miner");
    std::fs::create_dir_all(&path).ok();
    path.push(WALLET_FILENAME);
    path
}

/// Load wallet in non-interactive mode
pub fn load_wallet_auto(
    path: Option<PathBuf>,
    password: Option<&str>
) -> Result<Wallet> {
    let path = path.unwrap_or_else(default_wallet_path);

    if !path.exists() {
        println!("Creating new wallet at {:?}", path);
        let wallet = Wallet::new()?;
        wallet.save(&path, password)?;
        println!("✅ New wallet created: {}", wallet.id());
        return Ok(wallet);
    }

    println!("Loading wallet from {:?}", path);
    let wallet = Wallet::load(&path, password)?;
    println!("✅ Wallet loaded: {}", wallet.id());
    Ok(wallet)
}

/// Interactive wallet UI
pub fn wallet_interactive() -> Result<Wallet> {
    print_banner();

    let path = default_wallet_path();
    println!("📁 Wallet location: {}", path.display());

    if path.exists() {
        println!("🔐 Found existing wallet");
        load_existing_wallet(&path)
    } else {
        println!("🆕 No wallet found");
        create_new_wallet(&path)
    }
}


fn load_existing_wallet(path: &PathBuf) -> Result<Wallet> {
    print!("Enter password: ");
    io::stdout().flush()?;
    let password = read_password()?;

    match Wallet::load(path, Some(&password)) {
        Ok(wallet) => {
            println!("✅ Wallet unlocked!");
            println!("Worker ID: {}", wallet.id());
            Ok(wallet)
        }
        Err(e) => {
            eprintln!("❌ Failed to unlock: {}", e);
            std::process::exit(1);
        }
    }
}

fn create_new_wallet(path: &PathBuf) -> Result<Wallet> {
    print!("Enter password (or press Enter for none): ");
    io::stdout().flush()?;
    let password = read_password()?;
    let password = if password.is_empty() { None } else { Some(password.as_str()) };

    let wallet = Wallet::new()?;
    wallet.save(path, password)?;

    println!("\n✅ New wallet created!");
    wallet.display();
    println!("\n⚠️  Save this information securely!");

    Ok(wallet)
}

fn print_banner() {
    println!(r#"
 ____              __  __ _
|  _ \ ___ _   _  |  \/  (_)_ __   ___ _ __
| |_) / __| | | | | |\/| | | '_ \ / _ \ '__|
|  __/\__ \ |_| | | |  | | | | | |  __/ |
|_|   |___/\__, | |_|  |_|_|_| |_|\___|_|
           |___/
    "#);
}