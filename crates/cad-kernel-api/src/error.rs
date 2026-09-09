//! Backend-independent kernel error type (AICAD-017).
//!
//! Every kernel-adapter operation returns `KernelResult<T>` instead of
//! panicking or propagating a backend-specific exception type (Stage-1
//! kernel policy #7: normalize native failures into explicit structured
//! status/error results).

use std::fmt;

/// A kernel operation failure, independent of which backend (OCCT or a
/// future alternative) produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// An argument violated a precondition the kernel adapter itself
    /// enforces (e.g. a non-positive dimension). Distinct from a
    /// backend-reported geometric failure.
    InvalidArgument(String),
    /// A handle referenced a slot that was never allocated (or was
    /// allocated in a different, incompatible way) in the presented
    /// context.
    InvalidHandle,
    /// A handle referenced a slot that has since been freed and/or
    /// reused for different geometry.
    StaleHandle,
    /// A handle was created by one kernel context and presented to a
    /// different one.
    ForeignContext,
    /// The backend itself reported a failure while performing the
    /// operation (e.g. a degenerate/impossible geometric construction).
    /// The string is diagnostic-only, never parsed for control flow.
    NativeFailure(String),
    /// A failure that does not fit any of the above; always considered
    /// a defect worth investigating (Stage-1: native crashes/hangs are
    /// first-class defects), never routine.
    Unknown(String),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KernelError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            KernelError::InvalidHandle => write!(f, "invalid kernel handle"),
            KernelError::StaleHandle => write!(f, "stale kernel handle"),
            KernelError::ForeignContext => {
                write!(f, "kernel handle belongs to a different context")
            }
            KernelError::NativeFailure(msg) => write!(f, "kernel backend failure: {msg}"),
            KernelError::Unknown(msg) => write!(f, "unknown kernel error: {msg}"),
        }
    }
}

impl std::error::Error for KernelError {}

/// Result alias used by every kernel-adapter operation.
pub type KernelResult<T> = Result<T, KernelError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_non_empty_for_every_variant() {
        let variants = [
            KernelError::InvalidArgument("dx must be positive".into()),
            KernelError::InvalidHandle,
            KernelError::StaleHandle,
            KernelError::ForeignContext,
            KernelError::NativeFailure("BRepPrimAPI_MakeBox not done".into()),
            KernelError::Unknown("unexpected status code 99".into()),
        ];
        for variant in variants {
            assert!(!variant.to_string().is_empty());
        }
    }
}
