//! Password generation core with invariants, modeling, and branchless scheduling.

use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;

use thiserror::Error;

/* ============================================================
COMPILE-TIME INVARIANTS
============================================================ */

const UPPER_STR: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER_STR: &str = "abcdefghijklmnopqrstuvwxyz";
const DIGIT_STR: &str = "0123456789";
const SPECIAL_STR: &str = "~!@#$%^&*()-_=+[];:,.<>/?\\|";

const fn non_empty(s: &str) -> bool {
    !s.is_empty()
}

const _: () = {
    assert!(non_empty(UPPER_STR));
    assert!(non_empty(LOWER_STR));
    assert!(non_empty(DIGIT_STR));
    assert!(non_empty(SPECIAL_STR));
};

/* ============================================================
TYPES
============================================================ */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    Upper = 0,
    Lower = 1,
    Digit = 2,
    Special = 3,
}

impl CharClass {
    pub const ALL: [CharClass; 4] = [
        CharClass::Upper,
        CharClass::Lower,
        CharClass::Digit,
        CharClass::Special,
    ];
}

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("requested length {0} is smaller than required minimum 16")]
    LengthTooSmall(usize),

    #[error("length {0} exceeds unique character capacity {1}")]
    LengthExceedsCapacity(usize, usize),

    #[error("unable to satisfy constraints after {0} attempts")]
    RetryLimitExceeded(usize),
}

/* ============================================================
BRANCHLESS CLASS SCHEDULER
============================================================ */

const TRANSITIONS: [[CharClass; 3]; 4] = [
    [CharClass::Lower, CharClass::Digit, CharClass::Special],
    [CharClass::Upper, CharClass::Digit, CharClass::Special],
    [CharClass::Upper, CharClass::Lower, CharClass::Special],
    [CharClass::Upper, CharClass::Lower, CharClass::Digit],
];

fn next_class(prev: CharClass, rng: &mut OsRng) -> CharClass {
    let idx = rng.gen_range(0..3);
    TRANSITIONS[prev as usize][idx]
}

/* ============================================================
STATISTICAL MODEL
============================================================ */

#[derive(Debug, Default)]
struct SuccessModel {
    attempts: usize,
    successes: usize,
}

impl SuccessModel {
    fn record(&mut self, success: bool) {
        self.attempts += 1;
        if success {
            self.successes += 1;
        }
    }

    fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            return 0.5;
        }
        self.successes as f64 / self.attempts as f64
    }

    fn retry_bound(&self, length: usize) -> usize {
        let p = self.success_rate().clamp(0.01, 0.99);
        let expected = (1.0 / p).ceil() as usize;
        let scale = length.max(16);
        expected * scale
    }
}

/* ============================================================
GENERATOR
============================================================ */

#[derive(Debug)]
pub struct Generator {
    upper: Vec<char>,
    lower: Vec<char>,
    digit: Vec<char>,
    special: Vec<char>,
    capacity: usize,
    model: std::cell::RefCell<SuccessModel>,
}

impl Generator {
    pub fn new() -> Self {
        let upper: Vec<char> = UPPER_STR.chars().collect();
        let lower: Vec<char> = LOWER_STR.chars().collect();
        let digit: Vec<char> = DIGIT_STR.chars().collect();
        let special: Vec<char> = SPECIAL_STR.chars().collect();

        let capacity = upper.len() + lower.len() + digit.len() + special.len();

        Self {
            upper,
            lower,
            digit,
            special,
            capacity,
            model: Default::default(),
        }
    }

    pub fn generate_adaptive(&self, length: usize) -> Result<String, GeneratorError> {
        if length < 16 {
            return Err(GeneratorError::LengthTooSmall(length));
        }

        if length > self.capacity {
            return Err(GeneratorError::LengthExceedsCapacity(length, self.capacity));
        }

        let bound = self.model.borrow().retry_bound(length);

        for attempt in 1..=bound {
            match self.generate_once(length) {
                Some(pw) => {
                    self.model.borrow_mut().record(true);
                    return Ok(pw);
                }
                None => {
                    self.model.borrow_mut().record(false);
                    if attempt == bound {
                        return Err(GeneratorError::RetryLimitExceeded(bound));
                    }
                }
            }
        }

        unreachable!()
    }

    fn generate_once(&self, length: usize) -> Option<String> {
        let mut rng = OsRng;
        let mut used_ci: HashSet<char> = HashSet::with_capacity(length);
        let mut out = String::with_capacity(length);

        let mut class_seq = Vec::with_capacity(length);

        let mut first = CharClass::ALL.to_vec();
        first.shuffle(&mut rng);
        class_seq.extend(first);

        while class_seq.len() < length {
            let prev = *class_seq.last().unwrap();
            class_seq.push(next_class(prev, &mut rng));
        }

        for class in class_seq {
            let pool = match class {
                CharClass::Upper => &self.upper,
                CharClass::Lower => &self.lower,
                CharClass::Digit => &self.digit,
                CharClass::Special => &self.special,
            };

            let mut candidates: Vec<char> = pool
                .iter()
                .copied()
                .filter(|c| !used_ci.contains(&c.to_ascii_lowercase()))
                .collect();

            if candidates.is_empty() {
                return None;
            }

            candidates.shuffle(&mut rng);
            let ch = candidates[0];

            used_ci.insert(ch.to_ascii_lowercase());
            out.push(ch);
        }

        Some(out)
    }
}

// end of source
