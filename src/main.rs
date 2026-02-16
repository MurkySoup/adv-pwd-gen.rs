//! Advanced password generator CLI.
//!
//! CLI contract:
//! - User-selectable password length
//! - User-selectable number of passwords
//! - User-selectable retry bound for dead-end handling
//!
//! Exit codes:
//! 0 = success
//! 1 = invalid input
//! 2 = generation failure

mod password;

use password::Generator;
use std::env;
use std::num::NonZeroUsize;
use std::process::ExitCode;

/// Parsed command-line configuration.
#[derive(Debug)]
struct Config {
    length: NonZeroUsize,
    count: NonZeroUsize,
    max_retries: NonZeroUsize,
}

impl Config {
    fn parse() -> Result<Self, String> {
        let mut length: Option<NonZeroUsize> = None;
        let mut count: Option<NonZeroUsize> = None;
        let mut retries: Option<NonZeroUsize> = None;

        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-l" | "--length" => {
                    length = Some(parse_nz(args.next(), "length")?);
                }
                "-n" | "--count" => {
                    count = Some(parse_nz(args.next(), "count")?);
                }
                "--max-retries" => {
                    retries = Some(parse_nz(args.next(), "max-retries")?);
                }
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    return Err(format!("Unknown argument: {arg}"));
                }
            }
        }

        let length = length.ok_or("Missing required option: --length")?;
        let count = count.unwrap_or_else(|| nz(1));
        let max_retries = retries.unwrap_or_else(|| nz(256));

        if length.get() < 16 {
            return Err("Password length must be >= 16".into());
        }

        Ok(Self {
            length,
            count,
            max_retries,
        })
    }
}

fn main() -> ExitCode {
    let config = match Config::parse() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: {e}");
            print_help();
            return ExitCode::from(1);
        }
    };

    let generator = Generator::new(config.max_retries.get());

    for _ in 0..config.count.get() {
        match generator.generate(config.length.get()) {
            Ok(pw) => println!("{pw}"),
            Err(err) => {
                eprintln!("Generation failed: {err:?}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

/// Parse a required positive integer argument.
fn parse_nz(value: Option<String>, name: &str) -> Result<NonZeroUsize, String> {
    let raw = value.ok_or_else(|| format!("Missing value for {name}"))?;
    let parsed: usize = raw
        .parse()
        .map_err(|_| format!("Invalid numeric value for {name}: {raw}"))?;
    NonZeroUsize::new(parsed).ok_or_else(|| format!("{name} must be > 0"))
}

/// Create a NonZeroUsize safely (internal use only).
fn nz(v: usize) -> NonZeroUsize {
    NonZeroUsize::new(v).expect("non-zero constant")
}

fn print_help() {
    println!(
        "\
Advanced Password Generator

USAGE:
    adv-pwd-gen --length <N> [OPTIONS]

REQUIRED:
    -l, --length <N>        Password length (>= 16)

OPTIONS:
    -n, --count <N>         Number of passwords to generate (default: 1)
        --max-retries <N>   Retry bound for dead-end recovery (default: 256)
    -h, --help              Show this help message
"
    );
}

// end of source
