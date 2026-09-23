//! A small, independent FNV-1a 64-bit accumulator, following the exact
//! precedent and rationale `cad_feature_graph::cache`'s own `StableHasher`
//! documents ("Why a hand-rolled hasher, not `DefaultHasher`" — the
//! standard library's hasher is explicitly unspecified across releases,
//! which `project/DECISION_LOG.md#DL-12` Level 1 forbids relying on for an
//! AICAD-owned deterministic value). This module is not reused from
//! `cad-feature-graph` because that type is private to its crate and this
//! crate has no other reason to depend on it; duplicating ~15 lines is
//! cheaper than adding a cross-crate coupling for it.

pub(crate) struct StableHasher(u64);

impl StableHasher {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub(crate) fn new() -> Self {
        StableHasher(Self::OFFSET_BASIS)
    }

    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    pub(crate) fn write_u8(&mut self, tag: u8) {
        self.write_bytes(&[tag]);
    }

    pub(crate) fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_input_hashes_identically() {
        let mut a = StableHasher::new();
        a.write_bytes(b"step");
        a.write_u8(0);
        a.write_bytes(b"payload");

        let mut b = StableHasher::new();
        b.write_bytes(b"step");
        b.write_u8(0);
        b.write_bytes(b"payload");

        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn different_input_hashes_differently() {
        let mut a = StableHasher::new();
        a.write_bytes(b"step");
        a.write_u8(0);
        a.write_bytes(b"payload");

        let mut b = StableHasher::new();
        b.write_bytes(b"stl");
        b.write_u8(0);
        b.write_bytes(b"payload");

        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn separator_prevents_boundary_shifting_collisions() {
        // "ab" + "c" must not hash the same as "a" + "bc".
        let mut a = StableHasher::new();
        a.write_bytes(b"ab");
        a.write_u8(0);
        a.write_bytes(b"c");

        let mut b = StableHasher::new();
        b.write_bytes(b"a");
        b.write_u8(0);
        b.write_bytes(b"bc");

        assert_ne!(a.finish(), b.finish());
    }
}
