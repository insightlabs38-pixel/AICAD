//! `cad-geometry-api` — WP-05 (Geometry language API).
//!
//! `AICAD-059` populates this crate with the backend-independent Geometry
//! IR (`ir` module) — see that module's doc comment for the full design.
//! Safe language-facing high-level/low-level geometry API surfaces and the
//! lowering from typed HIR into this IR remain later tasks
//! (`AICAD-060` onward) per this crate's own `README.md`.
//!
//! `AICAD-108` (Stage 5, `project/DECISION_LOG.md#DL-5`/`DL-24`/`DL-26`
//! item "advanced geometry value/IR families") adds four new kernel-neutral
//! value families later Stage-5 batches build on — construction/kernel
//! wiring for each remains that later batch's own job (see each module's
//! own doc comment for the exact dependency):
//! - [`curve`]: [`curve::AnalyticCurve`] — analytic curve values
//!   (`AICAD-109` wired construction/evaluation for the analytic families;
//!   `AICAD-110` extended the same enum with Bezier/B-spline/NURBS;
//!   `AICAD-111` added curve-local operations — trim/offset/closest-point/
//!   interpolate; kernel dispatch was not needed for any of them — see
//!   that module's own doc comment).
//! - [`surface`]: [`surface::AnalyticSurface`] — analytic surface values
//!   (`AICAD-113` wired construction/evaluation for the analytic families;
//!   `AICAD-114`-`116` extend it with Bezier/B-spline/NURBS/trimmed
//!   surfaces and generalized derivative/offset operations).
//! - [`query_result`][]: [`query_result::QueryOutcome`]/[`query_result::
//!   QueryFailure`] — the multi-solution query result shape (`AICAD-117`/
//!   `118` instantiate it for real queries).
//! - [`operation_report`]: [`operation_report::OperationReport`] — captured
//!   created/modified/deleted/split/merged evidence for a topology-changing
//!   operation (`AICAD-119`-`124` produce these).
//! - [`adoption`][]: [`adoption::AdoptionOutcome`]/[`adoption::
//!   AdoptionEvidence`]/[`adoption::AdoptionRejection`] — D22's raw-to-safe
//!   adoption outcome (`AICAD-122`-`124` wire the actual validation).

pub mod adoption;
pub mod curve;
pub mod ir;
pub mod operation_report;
pub mod query_result;
pub mod surface;

pub use adoption::{AdoptionEvidence, AdoptionOutcome, AdoptionRejection};
pub use curve::{
    AnalyticCurve, ClosestPointResult, CurveConstructionError, CurveOperationError, CurveSample,
    interpolate,
};
pub use ir::{
    EdgeIndex, FaceIndex, GeomId, GeometryGraph, GeometryIrError, GeometryNode, GeometryNodeKind,
    GeometryOp, GeometryQuery, Quantity,
};
pub use operation_report::OperationReport;
pub use query_result::{QueryFailure, QueryOutcome};
pub use surface::{
    AnalyticSurface, Orientation, SurfaceConstructionError, SurfaceOperationError, SurfaceSample,
    SurfaceTrimError, TrimError, TrimLoop,
};
