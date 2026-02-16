//! Constraint-based password generator CLI.
//!
//! Rules enforced:
//! - Length >= 16 (configurable)
//! - At least one char from each class: Upper, Lower, Digit, Special
//! - No adjacent characters from the same class
//! - No repeated characters (case-insensitive)
//!
//! Usage:
//!   adv-pwd-gen --length 20 --count 5

use clap::Parser;
mod password;
use password::{Generator};

#[derive(Debug, Parser)]
#[command(name = "adv-pwd-gen", version, about)]
struct Cli {
    /// Password length (>= 16)
    #[arg(short, long, default_value_t = 16)]
    length: usize,

    /// Number of passwords to generate
    #[arg(short, long, default_value_t = 1)]
    count: usize,
}

fn main() {
    let cli = Cli::parse();

    let gen = Generator::new();

    for _ in 0..cli.count {
        match gen.generate_adaptive(cli.length) {
            Ok(pw) => println!("{pw}"),
            Err(e) => {
                eprintln!("Generation failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

// end of source
