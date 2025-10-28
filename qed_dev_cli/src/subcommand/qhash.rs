use clap::{Args, Subcommand};
use qed_core::data::qhashout::QHashOut;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_data::config::store_config::QEDHasher;
use psy_crypto::hash::traits::hasher::FieldQHasher;
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

    #[command(about = "Get zero hash for given tree height")]
    ZeroHash {
        #[arg(help = "Tree height")]
        height: u8,
    },

    #[command(about = "Apply two_to_one sequentially to space-separated hashes")]
    ManyToOne {
        #[arg(help = "Space-separated QHashOut strings to reduce with two_to_one")]
        hashes: String,
    },
}

pub fn run(args: QHashArgs) -> Result<()> {
    match args.command {
        QHashCommands::FromString { value } => {
            let hash = QHashOut::<F>::from_string_or_panic(&value);
            println!("Hash: {}", hash.to_string());
            println!("Elements: [{}, {}, {}, {}]",
                hash.0.elements[0].0,
                hash.0.elements[1].0,
                hash.0.elements[2].0,
                hash.0.elements[3].0
            );
            Ok(())
        }

        QHashCommands::FromValues { a, b, c, d } => {
            let hash = QHashOut::<F>::from_values(a, b, c, d);
            println!("Hash: {}", hash.to_string());
            println!("Elements: [{}, {}, {}, {}]",
                hash.0.elements[0].0,
                hash.0.elements[1].0,
                hash.0.elements[2].0,
                hash.0.elements[3].0
            );
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

        QHashCommands::ZeroHash { height } => {
            use psy_crypto::hash::merkle::utils::simple_merkle_tree::SimpleMerkleTree;

            let empty_tree = SimpleMerkleTree::<QEDHasher, QHashOut<F>>::new(height);
            let zero_hash = empty_tree.get_root();
            println!("Zero hash for height {}: {}", height, zero_hash.to_string());
            println!("Elements: [{}, {}, {}, {}]",
                zero_hash.0.elements[0].0,
                zero_hash.0.elements[1].0,
                zero_hash.0.elements[2].0,
                zero_hash.0.elements[3].0
            );
            Ok(())
        }

        QHashCommands::ManyToOne { hashes } => {
            let hash_strings: Vec<&str> = hashes.split_whitespace().collect();

            if hash_strings.is_empty() {
                return Err(anyhow::format_err!("No hashes provided"));
            }

            if hash_strings.len() == 1 {
                println!("{}", hash_strings[0]);
                return Ok(());
            }

            let mut result = QHashOut::<F>::from_string_or_panic(hash_strings[0]);

            for hash_str in &hash_strings[1..] {
                let next_hash = QHashOut::<F>::from_string_or_panic(hash_str);
                result = QEDHasher::q_two_to_one(result, next_hash);
            }

            println!("{}", result.to_string());
            Ok(())
        }
    }
}
