# adv-pwd-gen

Constraint-based password generator implemented in Rust.

---

## Description

This is a Rust-based implementation of my personal password generation tool. The passwords created this way are not merely random, but are also "anti-pattern" and designed to be as difficult as possible to "break" using existing password recovery tools.

---

## Features

### Security & Correctness
- Cryptographically secure randomness (OS CSPRNG)
- Minimum length enforcement (≥ 16)
- At least one character from each class:
  - Uppercase
  - Lowercase
  - Digit
  - Special character
- No adjacent characters from the same class
- No repeated characters (case-insensitive)
- Explicit error reporting for unsatisfiable requests

### Robust Generation Model
- Compile-time invariant validation of character sets
- Branchless class scheduling (uniform transition model)
- Statistical dead-end modeling with adaptive retry bound
- Deterministic termination guarantee (no infinite retries)
- Early rejection of impossible lengths

### Engineering Quality
- Idiomatic Rust design
- Strong typing and explicit error model
- Clippy clean with `-D warnings`
- No unsafe code
- Modular and testable architecture

---

## Installation

### From Source

```bash
git clone https://github.com/MurkySoup/adv-pwd-gen.rs
cd adv-pwd-gen
cargo build --release
````

Binary will be located at:

```
target/release/adv-pwd-gen
```

---

## Usage

### Options

| Flag               | Description                     | Default |
| ------------------ | ------------------------------- | ------- |
| `-l, --length <N>` | Password length (must be ≥ 16)  | 16      |
| `-c, --count <N>`  | Number of passwords to generate | 1       |

Generate one password (default):

```bash
adv-pwd-gen
```

Generate multiple passwords:

```bash
adv-pwd-gen --length 24 --count 5
```

---

## Exit Behavior

The program exits with an error if:

* requested length is below minimum
* requested length exceeds unique character capacity
* generation cannot succeed within the adaptive retry bound

All failures are deterministic and explicit.

---

## Constraint Model

Let:

* Uppercase Set U = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
* Lowercase Set L = "abcdefghijklmnopqrstuvwxyz"
* Digit Set D = "0123456789"
* Special Character Set S = "~!@#$%^&*()-_=+[];:,.<>/?\\|"

A generated password `P` satisfies:

1. `len(P) ≥ 16`
2. `P` contains ≥ 1 char from each of {U, L, D, S}
3. Adjacent characters are from different classes
4. No character repeats ignoring case
5. A valid completion exists at each construction step

---

A smaller character set can be defined in `password.rs` to generate passwords for better readability and reduced transcription errors:
```
const UPPER_STR: &str = "ADEFGHJKLMNPRTUW";
const LOWER_STR: &str = "abdefghijkmnpqrstuwy";
const DIGIT_STR: &str = "234679";
const SPECIAL_STR: &str = "!"#*+-./:=?@^_|";
```

This character set, however, comes with the drawback of smaller maximum lengths. Testing before wide-spread use should be conducted.

---

## Design Overview

### Branchless Class Scheduler

Class transitions are selected from precomputed tables rather than conditional logic.
This ensures uniform distribution and avoids repair logic.

### Statistical Dead-End Modeling

The generator tracks empirical success rate and derives an adaptive retry bound:

```
retry_bound ≈ expected_attempts × length
```

This produces predictable runtime without user-tuned retry limits.

### Compile-Time Invariants

Character set validity is verified at compile time, preventing configuration drift from introducing unsatisfiable constraints.

---

## Build Quality

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

The project is designed to compile cleanly under the latest stable Rust toolchain.

---

# License

This tool is released under the Apache 2.0 license. See the LICENSE file in this repo for details.

## Built With

* [Rust](https://www.rust-lang.org/)

## Author

**Rick Pelletier** - [USA Today Company](https://www.usatodayco.com/)
