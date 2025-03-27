use std::num::NonZeroU8;

use serde::{Deserialize, Serialize};

/// A hash function outputting a fixed-length digest of `N` bytes.
///
/// The hash function must produce strong digests with a low probability of
/// collision. Use of a cryptographic hash function is not required, but may be
/// preferred for security/compliance reasons.
///
/// Use of a faster hash function results in faster tree operations. Use of
/// 64bit hash values (`N <= 8`) and smaller is not recommended due to the
/// higher probability of collisions.
///
/// # Determinism & Portability
///
/// Implementations are required to be deterministic across all peers which
/// compare tree values. Notably the standard library [`Hash`] derive does not
/// produce portable hashes across differing platforms, endian-ness or Rust
/// compiler versions.
///
/// # Default Implementation
///
/// The default [`Hasher`] implementation ([`SipHasher`]) outputs 128-bit/16
/// byte digests which are strong, but not of cryptographic quality.
///
/// [`SipHasher`] uses the standard library [`Hash`] trait, which may produce
/// non-portable hashes as described above (and in the [`Hash`] documentation).
///
/// Users may choose to initialise the [`SipHasher`] with seed keys if untrusted
/// key/value user input is used in a tree in order to prevent chosen-hash
/// collision attacks.
///
/// The provided [`SipHasher`] implementation is not portable across platforms /
/// Rust versions due to limitations of the [`Hash`] trait.
///
/// [`SipHasher`]: super::siphash::SipHasher
pub trait Hasher<const N: usize, T> {
    /// Hash `T`, producing a unique, deterministic digest of `N` bytes length.
    fn hash(&self, value: &T) -> Digest<N>;
}

/// A variable bit length digest, output from a [`Hasher`] implementation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Digest<const N: usize>([u8; N]);

impl<const N: usize> Digest<N> {
    /// Wrap an opaque byte array in a [`Digest`] for type safety.
    pub const fn new(digest: [u8; N]) -> Self {
        Self(digest)
    }

    /// Return a reference to a fixed size digest byte array.
    pub const fn as_bytes(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> Serialize for Digest<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de, const N: usize> Deserialize<'de> for Digest<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let bytes: [u8; N] = bytes.try_into().map_err(|_| serde::de::Error::custom("invalid length"))?;
        Ok(Self(bytes))
    }
}

impl<const N: usize> AsRef<[u8]> for Digest<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// TODO(dom:doc): update this

/// Extract the number of leading 0's when expressed as base 16 digits, defining
/// the tree level the hash should reside at.
pub(crate) fn level<const N: usize>(v: &Digest<N>, base: NonZeroU8) -> u8 {
    let mut out = 0;
    for v in v.0.into_iter().map(|v| base_count_zero(v, base)) {
        match v {
            2 => out += 2,
            1 => return out + 1,
            0 => return out,
            _ => unreachable!(),
        }
    }
    out
}

const fn base_count_zero(v: u8, base: NonZeroU8) -> u8 {
    if v == 0 {
        return 2;
    }

    if v % base.get() == 0 {
        return 1;
    }

    0
}
