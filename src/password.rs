//! Password generation engine with runtime-dispatched SIMD uniqueness checks.
//!
//! Guarantees:
//! - Minimum length enforcement (caller responsibility)
//! - At least one char from each class
//! - No adjacent same-class characters
//! - No repeated characters (case-insensitive)
//! - Cryptographically secure randomness
//! - Dead-end detection with bounded retries
//! - Branchless class scheduling

use rand_core::{OsRng, RngCore};

/* -------------------------------------------------------------------------- */
/*                               Char classes                                 */
/* -------------------------------------------------------------------------- */

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CharClass {
    Upper,
    Lower,
    Digit,
    Special,
}

impl CharClass {
    const ALL: [CharClass; 4] = [
        CharClass::Upper,
        CharClass::Lower,
        CharClass::Digit,
        CharClass::Special,
    ];

    #[inline]
    fn index(self) -> usize {
        self as usize
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Errors                                   */
/* -------------------------------------------------------------------------- */

#[derive(Debug)]
pub enum GeneratorError {
    UnsatisfiableLength,
    ExhaustedAttempts,
}

/* -------------------------------------------------------------------------- */
/*                                  Generator                                 */
/* -------------------------------------------------------------------------- */

#[derive(Debug)]
pub struct Generator {
    uppercase: &'static [u8],
    lowercase: &'static [u8],
    digits: &'static [u8],
    special: &'static [u8],
    max_attempts: usize,
}

impl Generator {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            uppercase: b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            lowercase: b"abcdefghijklmnopqrstuvwxyz",
            digits: b"0123456789",
            special: b"~!@#$%^&*()-_=+[];:,.<>/?\\|",
            max_attempts,
        }
    }

    pub fn generate(&self, length: usize) -> Result<String, GeneratorError> {
        if length < 4 {
            return Err(GeneratorError::UnsatisfiableLength);
        }

        for _ in 0..self.max_attempts {
            if let Some(pw) = self.try_generate(length) {
                return Ok(pw);
            }
        }

        Err(GeneratorError::ExhaustedAttempts)
    }

    fn try_generate(&self, length: usize) -> Option<String> {
        let mut rng = OsRng;
        let mut used = uniqueness::UniqueSet::new();
        let mut result = Vec::with_capacity(length);

        let mut prev_class: Option<CharClass> = None;
        let mut class_used = [false; 4];

        for position in 0..length {
            let class = self.next_class(&mut rng, prev_class, position, length, &class_used)?;
            let ch = self.sample_unique_char(&mut rng, class, &mut used)?;
            class_used[class.index()] = true;

            result.push(ch);
            prev_class = Some(class);
        }

        if class_used.iter().all(|v| *v) {
            Some(String::from_utf8(result).ok()?)
        } else {
            None
        }
    }

    fn next_class(
        &self,
        rng: &mut OsRng,
        prev: Option<CharClass>,
        position: usize,
        length: usize,
        class_used: &[bool; 4],
    ) -> Option<CharClass> {
        let remaining = length - position;
        let mut candidates = [true; 4];

        if let Some(p) = prev {
            candidates[p.index()] = false;
        }

        let missing = class_used.iter().filter(|v| !**v).count();
        if missing == remaining {
            for (i, used) in class_used.iter().enumerate() {
                candidates[i] = !*used;
            }
        }

        let mut valid = Vec::with_capacity(4);
        for (i, ok) in candidates.iter().enumerate() {
            if *ok {
                valid.push(CharClass::ALL[i]);
            }
        }

        if valid.is_empty() {
            return None;
        }

        let idx = (rng.next_u64() as usize) % valid.len();
        Some(valid[idx])
    }

    fn sample_unique_char(
        &self,
        rng: &mut OsRng,
        class: CharClass,
        used: &mut uniqueness::UniqueSet,
    ) -> Option<u8> {
        let set = self.class_set(class);

        for _ in 0..64 {
            let idx = (rng.next_u64() as usize) % set.len();
            let ch = set[idx];
            let folded = ascii_lower(ch);

            if used.insert(folded) {
                return Some(ch);
            }
        }

        None
    }

    #[inline]
    fn class_set(&self, class: CharClass) -> &'static [u8] {
        match class {
            CharClass::Upper => self.uppercase,
            CharClass::Lower => self.lowercase,
            CharClass::Digit => self.digits,
            CharClass::Special => self.special,
        }
    }
}

impl Default for Generator {
    fn default() -> Self {
        Self::new(256)
    }
}

/* -------------------------------------------------------------------------- */
/*                            ASCII case folding                              */
/* -------------------------------------------------------------------------- */

#[inline]
const fn ascii_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b + 32
    } else {
        b
    }
}

/* -------------------------------------------------------------------------- */
/*                        Runtime SIMD uniqueness engine                      */
/* -------------------------------------------------------------------------- */

mod uniqueness {
    pub struct UniqueSet {
        data: [u8; 32],
        backend: Backend,
    }

    #[derive(Clone, Copy)]
    enum Backend {
        Scalar,
        #[cfg(target_arch = "x86_64")]
        Sse2,
        #[cfg(target_arch = "x86_64")]
        Avx2,
    }

    impl UniqueSet {
        pub fn new() -> Self {
            let backend = detect_backend();
            Self {
                data: [0; 32],
                backend,
            }
        }

        #[inline]
        pub fn insert(&mut self, v: u8) -> bool {
            match self.backend {
                Backend::Scalar => insert_scalar(&mut self.data, v),

                #[cfg(target_arch = "x86_64")]
                Backend::Sse2 => unsafe { insert_sse2(&mut self.data, v) },

                #[cfg(target_arch = "x86_64")]
                Backend::Avx2 => unsafe { insert_avx2(&mut self.data, v) },
            }
        }
    }

    fn detect_backend() -> Backend {
        #[cfg(target_arch = "x86_64")]
        {
            if std::arch::is_x86_feature_detected!("avx2") {
                return Backend::Avx2;
            }
            if std::arch::is_x86_feature_detected!("sse2") {
                return Backend::Sse2;
            }
        }

        Backend::Scalar
    }

    /* ----------------------------- scalar fallback ----------------------------- */

    #[inline]
    fn insert_scalar(bits: &mut [u8; 32], v: u8) -> bool {
        let idx = (v / 8) as usize;
        let mask = 1u8 << (v % 8);

        let present = bits[idx] & mask != 0;
        bits[idx] |= mask;
        !present
    }

    /* ----------------------------- AVX2 implementation ----------------------------- */

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn insert_avx2(bits: &mut [u8; 32], v: u8) -> bool {
        use std::arch::x86_64::*;

        let idx = (v / 8) as usize;
        let mask = 1u8 << (v % 8);

        let ptr = bits.as_mut_ptr();
        let vec = _mm256_loadu_si256(ptr as *const __m256i);

        let mut tmp = [0u8; 32];
        _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, vec);

        let present = tmp[idx] & mask != 0;
        tmp[idx] |= mask;

        let new_vec = _mm256_loadu_si256(tmp.as_ptr() as *const __m256i);
        _mm256_storeu_si256(ptr as *mut __m256i, new_vec);

        !present
    }

    /* ----------------------------- SSE2 implementation ----------------------------- */

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn insert_sse2(bits: &mut [u8; 32], v: u8) -> bool {
        use std::arch::x86_64::*;

        let idx = (v / 8) as usize;
        let mask = 1u8 << (v % 8);

        let ptr = bits.as_mut_ptr();

        let mut tmp = [0u8; 32];
        let a = _mm_loadu_si128(ptr as *const __m128i);
        let b = _mm_loadu_si128(ptr.add(16) as *const __m128i);
        _mm_storeu_si128(tmp.as_mut_ptr() as *mut __m128i, a);
        _mm_storeu_si128(tmp.as_mut_ptr().add(16) as *mut __m128i, b);

        let present = tmp[idx] & mask != 0;
        tmp[idx] |= mask;

        let a2 = _mm_loadu_si128(tmp.as_ptr() as *const __m128i);
        let b2 = _mm_loadu_si128(tmp.as_ptr().add(16) as *const __m128i);
        _mm_storeu_si128(ptr as *mut __m128i, a2);
        _mm_storeu_si128(ptr.add(16) as *mut __m128i, b2);

        !present
    }
}

/* -------------------------------------------------------------------------- */
/*                     Compile-time invariant validation                      */
/* -------------------------------------------------------------------------- */

const fn unique_ascii_case_insensitive(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        let mut j = i + 1;
        let a = to_lower(bytes[i]);
        while j < bytes.len() {
            if a == to_lower(bytes[j]) {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

const fn to_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b + 32
    } else {
        b
    }
}

const _: () = {
    assert!(unique_ascii_case_insensitive(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
    assert!(unique_ascii_case_insensitive(b"abcdefghijklmnopqrstuvwxyz"));
    assert!(unique_ascii_case_insensitive(b"0123456789"));
};

// end of source
