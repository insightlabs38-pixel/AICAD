//! Kernel-neutral topology-operation evidence (`AICAD-108`) — a snapshot,
//! serializable-shaped record of what a topology-changing operation
//! (trim, sew, heal, split, an advanced surface/curve-derived construction,
//! ...) did to its own input entities, generic over whatever kernel-neutral
//! entity identity type a caller supplies (a `GeomId`, a `cad_kernel_api`
//! handle, ...).
//!
//! # Relationship to the existing Stage-4 boolean/fillet/chamfer lineage
//!
//! `cad_occt_bridge::Lineage<'ctx>` (`AICAD-086`) already answers
//! "was this input deleted / what did it generate / what did it become" for
//! `union`/`cut`/`intersect`/`fillet`/`chamfer` — but only as a **live
//! query** against a native handle that only exists while the operation's
//! own `'ctx`-scoped result is still alive, and only for those five
//! specific operations. [`OperationReport`] is the value-level counterpart
//! future Stage-5 topology operations (`AICAD-119`-`121` construction/
//! healing/inspection, `AICAD-122`-`124` raw edit/adoption) need: an owned,
//! `'ctx`-independent **snapshot** any of those operations can build once
//! and hand to a caller (Geometry IR, `cad-cli`'s own lineage capture, a
//! future diagnostic) without holding a live kernel context open — the
//! `cad-occt-bridge`-side "how do I get this evidence out of OCCT" work
//! remains each of those later tasks' own job; this module only fixes the
//! **shape** the evidence takes once captured, per this task's own
//! "operation-report concepts needed by later tasks" acceptance line.
//!
//! Every field is optional evidence (an empty `Vec` where an operation
//! genuinely produced none), never a forced single classification — an
//! entity may legitimately appear in more than one report from the same
//! operation only if the operation's own semantics require it (this module
//! does not enforce mutual exclusion; that is a per-operation invariant its
//! own implementation is responsible for).

/// One topology-changing operation's own captured evidence about what
/// happened to its input entities — see module doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationReport<T> {
    /// Entities that exist in the result with no corresponding input (pure
    /// new construction — e.g. a new face where a hole broke through an
    /// existing one).
    pub created: Vec<T>,
    /// `(input, result)` pairs: an input entity carried forward as a
    /// geometrically changed but not newly created counterpart (e.g. a face
    /// re-trimmed by a boolean operation).
    pub modified: Vec<(T, T)>,
    /// Input entities with no surviving counterpart in the result at all.
    pub deleted: Vec<T>,
    /// `(input, results)` pairs: one input entity that became more than one
    /// result entity (e.g. a face divided by a healing/trim operation).
    pub split: Vec<(T, Vec<T>)>,
    /// `(inputs, result)` pairs: more than one input entity that collapsed
    /// into one result entity (e.g. sewing coincident faces together).
    pub merged: Vec<(Vec<T>, T)>,
}

impl<T> OperationReport<T> {
    /// An empty report — every field starts empty; a caller fills in
    /// exactly the evidence its own operation actually produced.
    pub fn empty() -> OperationReport<T> {
        OperationReport {
            created: Vec::new(),
            modified: Vec::new(),
            deleted: Vec::new(),
            split: Vec::new(),
            merged: Vec::new(),
        }
    }

    /// Whether this report records no evidence at all (every field empty)
    /// — a legitimate outcome for an operation that touched nothing (e.g.
    /// a no-op heal pass), distinct from an operation that was never run.
    pub fn is_empty(&self) -> bool {
        self.created.is_empty()
            && self.modified.is_empty()
            && self.deleted.is_empty()
            && self.split.is_empty()
            && self.merged.is_empty()
    }
}

impl<T> Default for OperationReport<T> {
    fn default() -> OperationReport<T> {
        OperationReport::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_report_is_empty() {
        let report: OperationReport<u32> = OperationReport::empty();
        assert!(report.is_empty());
        assert_eq!(report, OperationReport::default());
    }

    #[test]
    fn a_report_with_any_evidence_is_not_empty() {
        let mut report: OperationReport<u32> = OperationReport::empty();
        report.created.push(7);
        assert!(!report.is_empty());
    }

    #[test]
    fn split_and_merge_carry_the_full_input_result_relationship() {
        let mut report: OperationReport<&str> = OperationReport::empty();
        report.split.push(("face_a", vec!["face_a_1", "face_a_2"]));
        report.merged.push((vec!["face_b", "face_c"], "face_bc"));
        assert_eq!(report.split[0].1.len(), 2);
        assert_eq!(report.merged[0].0.len(), 2);
    }

    #[test]
    fn two_reports_with_identical_evidence_compare_equal() {
        let mut a: OperationReport<u32> = OperationReport::empty();
        a.modified.push((1, 2));
        let mut b: OperationReport<u32> = OperationReport::empty();
        b.modified.push((1, 2));
        assert_eq!(a, b);
    }
}
