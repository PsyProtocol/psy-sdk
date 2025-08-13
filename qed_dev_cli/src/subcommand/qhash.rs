use clap::{Args, Subcommand};
use qed_core::data::qhashout::QHashOut;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_data::config::store_config::QEDHasher;
use qed_crypto::hash::traits::hasher::FieldQHasher;
use anyhow::Result;

type F = GoldilocksField;

#[derive(Args)]
pub struct QHashArgs {
    #[command(subcommand)]
    pub command: QHashCommands,
}

#[derive(Subcommand)]
pub enum QHashCommands {
    #[command(about = "Create QHashOut from string")]
    FromString {
        #[arg(help = "String to convert to QHashOut")]
        value: String,
    },
    
    #[command(about = "Create QHashOut from 4 u64 values")]
    FromValues {
        #[arg(help = "First u64 value")]
        a: u64,
        #[arg(help = "Second u64 value")]
        b: u64,
        #[arg(help = "Third u64 value")]
        c: u64,
        #[arg(help = "Fourth u64 value")]
        d: u64,
    },
    
    #[command(about = "Hash a single QHashOut")]
    Hash {
        #[arg(help = "QHashOut string to hash")]
        value: String,
    },
    
    #[command(about = "Hash two QHashOut values together")]
    TwoToOne {
        #[arg(help = "First QHashOut string")]
        left: String,
        #[arg(help = "Second QHashOut string")]
        right: String,
    },
    
    #[command(about = "Hash many QHashOut values")]
    HashMany {
        #[arg(help = "QHashOut strings to hash", num_args = 1..)]
        values: Vec<String>,
    },
}

pub fn run(args: QHashArgs) -> Result<()> {
    match args.command {
        QHashCommands::FromString { value } => {
            let hash = QHashOut::<F>::from_string_or_panic(&value);
            println!("{}", hash.to_string());
            Ok(())
        }
        
        QHashCommands::FromValues { a, b, c, d } => {
            let hash = QHashOut::<F>::from_values(a, b, c, d);
            println!("{}", hash.to_string());
            Ok(())
        }
        
        QHashCommands::Hash { value } => {
            let input = QHashOut::<F>::from_string_or_panic(&value);
            let output = QEDHasher::q_hash_many(&[
                input.0.elements[0],
                input.0.elements[1],
                input.0.elements[2],
                input.0.elements[3],
            ]);
            println!("{}", output.to_string());
            Ok(())
        }
        
        QHashCommands::TwoToOne { left, right } => {
            let left_hash = QHashOut::<F>::from_string_or_panic(&left);
            let right_hash = QHashOut::<F>::from_string_or_panic(&right);
            
            let output = QEDHasher::q_two_to_one(left_hash, right_hash);
            println!("{}", output.to_string());
            Ok(())
        }
        
        QHashCommands::HashMany { values } => {
            if values.is_empty() {
                return Err(anyhow::format_err!("No values provided to hash"));
            }
            
            let mut inputs = Vec::new();
            for value in values {
                let hash = QHashOut::<F>::from_string_or_panic(&value);
                inputs.push(hash.0.elements[0]);
                inputs.push(hash.0.elements[1]);
                inputs.push(hash.0.elements[2]);
                inputs.push(hash.0.elements[3]);
            }
            
            let output = QEDHasher::q_hash_many(&inputs);
            println!("{}", output.to_string());
            Ok(())
        }
    }
}