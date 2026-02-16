# adv-pwd-gen

Constraint-based password generator implemented in Rust.

---

## Description

This is a Rust-based implementation of my personal password generation tool. The passwords created this way are not merely random, but are also "anti-pattern" and designed to be as difficult as possible to "break" using existing password recovery tools.

Here is a **concise, production-ready `README.md`** aligned with the current architecture, behavior, and constraints of your generator.

You can drop this directly into your repository root.

---

## Features

### Security

* Cryptographically secure randomness (OS entropy source)
* Case-insensitive uniqueness of all characters
* Configurable retry bound prevents infinite runtime
* No unsafe code

### Structural Guarantees

Every generated password:

* Has minimum length ≥ 16
* Contains at least one:

  * uppercase letter
  * lowercase letter
  * digit
  * special character
* Contains **no adjacent characters from the same class**
* Contains **no repeated characters** (case-insensitive)
* Is generated using bounded, deterministic algorithms

### Algorithmic Design

* Branchless class scheduling
* Statistical dead-end detection and recovery
* Compile-time invariant validation of character sets
* Strong typing and zero invalid runtime states
* Clippy clean with `-D warnings`

---

## SIMD-Accelerated Uniqueness Engine

This project includes a runtime-dispatched SIMD engine that enforces **case-insensitive character uniqueness** using a 256-bit membership bitset. The engine transparently selects the fastest implementation supported by the host CPU while preserving identical behavior and security guarantees.

### What it does

* Tracks used characters in a **256-bit set** (ASCII domain).
* Performs membership test + insert in **constant time**.
* Eliminates hash tables and dynamic allocation in the hot path.
* Reduces branch pressure and improves throughput during constrained generation.

### Runtime backend selection

At program start, the engine detects CPU capabilities and selects the best backend:

1. **AVX2 backend** (x86_64)
   Uses 256-bit vector registers for bitset load/update operations.

2. **SSE2 backend** (x86_64 fallback)
   Uses 128-bit vector registers.

3. **Scalar backend** (portable fallback)
   Uses standard bit operations with identical semantics.

No configuration is required. Selection is automatic and safe.

### Behavioral guarantees

SIMD acceleration **does not change** generator semantics:

* Same password constraints and validation rules
* Same retry and dead-end handling behavior
* Same cryptographic randomness source
* Same deterministic termination properties
* Same public API and CLI interface

Only internal performance characteristics are improved.

### Performance characteristics

Compared to scalar membership checks, SIMD backends typically provide:

* Faster uniqueness enforcement in tight loops
* Lower runtime variance under heavy constraints
* Better scaling with password length and batch size
* Reduced cache pressure and branch misprediction

The largest gains occur when generating long passwords or large batches.

### Portability

The SIMD engine uses runtime feature detection and remains fully portable:

* AVX2 and SSE2 are widely available on modern systems from Intel and AMD.
* Non-x86 platforms and older CPUs automatically fall back to the scalar backend.
* The public API remains platform-agnostic.

### Safety model

* All vector operations are encapsulated behind a safe abstraction.
* No `unsafe` code is exposed to callers.
* The scalar backend guarantees correctness on all targets.

### Build configuration

SIMD support is enabled by default.
To disable SIMD explicitly:

```bash
cargo build --no-default-features
```

This forces the portable scalar backend while keeping identical functionality.

---

## Installation

Requirements:

* Rust stable toolchain (latest recommended)

Build:

```bash
cargo build --release
```

Binary will be located at:

```
target/release/adv-pwd-gen
```

---

## Usage

```bash
adv-pwd-gen --length <N> [OPTIONS]
```

### Required

```
-l, --length <N>        Password length (must be ≥ 16)
```

### Optional

```
-n, --count <N>         Number of passwords to generate (default: 1)
--max-retries <N>       Retry bound for constraint recovery (default: 256)
-h, --help              Show help message
```

---

## Examples

Generate one 20-character password:

```bash
adv-pwd-gen --length 20
```

Generate 5 passwords of length 24:

```bash
adv-pwd-gen --length 24 --count 5
```

Increase retry bound for extreme constraints:

```bash
adv-pwd-gen --length 32 --max-retries 1024
```

---

## Exit Codes

| Code | Meaning                         |
| ---- | ------------------------------- |
| 0    | Success                         |
| 1    | Invalid CLI input               |
| 2    | Constraint satisfaction failure |

---

## Design Notes

### Why bounded retries exist

Certain combinations of constraints can produce permutation dead-ends.
The generator uses statistical detection and bounded retry attempts to ensure termination.

### Why no adjacent same-class characters

This increases structural entropy and prevents pattern clustering common in naive generators.

### Why compile-time invariant checks

Character sets are validated during compilation to prevent silent security regressions.

---

## Dependency Policy

Minimal external dependencies:

* `rand` — RNG interface
* `rand_core` — OS entropy source

No CLI frameworks or heavy utility crates are used.

---

## Development

Lint and verify:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

# License

This tool is released under the Apache 2.0 license. See the LICENSE file in this repo for details.

## Built With

* [Rust](https://www.rust-lang.org/)

## Author

**Rick Pelletier** - [USA Today Company](https://www.usatodayco.com/)
