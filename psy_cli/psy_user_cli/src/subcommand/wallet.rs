use std::{
    io::{self, Write},
    path::Path,
};

use anyhow::Result;
use psy_rust_sdk::wallet::secp_wallet::Wallet;
use rpassword::read_password;
use tracing::info;

use super::args::{WalletArgs, WalletCommands};

pub fn run(args: WalletArgs) -> Result<()> {
    match args.command {
        WalletCommands::Create { output, password } => {
            let wallet = Wallet::new()?;

            if let Some(path) = output {
                let password = match password {
                    Some(p) => p,
                    None => {
                        print!("Enter password for wallet: ");
                        io::stdout().flush()?;
                        read_password()?
                    }
                };

                wallet.save(Path::new(&path), Some(&password))?;
                println!("✅ Wallet created and saved to: {}", path);
            } else {
                println!("✅ Wallet created:");
            }

            println!("ETH Address: {}", wallet.address());
            println!("Public Key: {}", wallet.public_key_hash());
            // println!("Private Key: {}", wallet.private_key_hex());

            Ok(())
        }
        WalletCommands::Load {
            private_key,
            keystore_path,
            password,
        } => {
            let wallet = Wallet::load(private_key.as_deref(), keystore_path.as_ref().map(|p| Path::new(p)), password.as_deref())?;

            println!("✅ Wallet loaded:");
            println!("ETH Address: {}", wallet.address());
            println!("Public Key: {}", wallet.public_key_hash());

            Ok(())
        }
        WalletCommands::List { keystore_dir } => {
            let dir = keystore_dir.as_ref().map(|p| Path::new(p));
            let accounts = Wallet::list_accounts(dir)?;

            if accounts.is_empty() {
                println!("No wallets found");
            } else {
                println!("Found {} wallet(s):", accounts.len());
                for account in accounts {
                    println!("  {}", account);
                }
            }

            Ok(())
        }
    }
}
