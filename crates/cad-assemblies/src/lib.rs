//! `cad-assemblies` — Stage-6 assembly semantic identity primitives
//! (`AICAD-133`, `D26`, `project/DECISION_LOG.md#DL-28`).
//!
//! `D26` requires assembly identity concepts to stay distinct rather than
//! collapsing into geometry, solver, filesystem, or BOM identifiers. This
//! crate defines that distinctness as Rust-level *types*, before any
//! assembly definition/instance/nesting IR exists (`AICAD-134` onward
//! builds that IR on top of these primitives):
//!
//! - [`definition::ComponentDefinitionId`] — component-definition identity.
//! - [`instance::LogicalInstanceId`] — logical component-instance identity.
//! - [`occurrence::OccurrencePath`] — nested instance path/occurrence
//!   identity.
//! - [`configuration::ConfigurationSlotId`] — configuration/variant
//!   identity, as the stable replacement slot `AICAD-149`/`AICAD-152`
//!   build on.
//! - [`asset::ExternalAssetId`] — external-asset identity.
//! - [`topology::OccurrenceTopologyRef`] — semantic feature/topology
//!   references inside an instance.
//! - [`bom::BomClassificationId`] — BOM/purchasing classification
//!   identity.
//!
//! Each is a distinct Rust type — never a bare integer/string alias
//! interchangeable with another domain — and each is constructed
//! deterministically from stable source-level data (declared names,
//! occurrence nesting, content bytes), never from a topology index, a
//! solver variable, a raw kernel/native handle, a filesystem path, or a
//! BOM row. See each module's own doc comment for why its particular
//! construction is deterministic and why it cannot alias another domain.
//!
//! `AICAD-134` onward builds the real assembly IR on top of these
//! primitives: [`component`] (component-definition/logical-instance IR),
//! [`frame`] (`AICAD-135`'s local/world instance poses), [`graph`]
//! (`AICAD-136`'s deterministic nested-occurrence resolution and
//! definition-level cycle diagnostics), [`reference`] (`AICAD-137`'s
//! fail-closed cross-instance semantic-reference addressing over the
//! resolved occurrence tree), [`interface`] (`AICAD-138`'s reusable
//! mechanical-interface conformance/compatibility semantics built on
//! D27's general interface shape), [`mate`] (`AICAD-139`'s solver-neutral
//! mate relation IR), and [`joint`] (`AICAD-140`'s solver-neutral joint/
//! coordinate IR) -- both built on `D28`/`DL-30`'s "AICAD-owned typed
//! semantic relations" authority.

pub mod asset;
pub mod bom;
pub mod component;
pub mod configuration;
pub mod definition;
pub mod frame;
pub mod graph;
mod hash;
pub mod instance;
pub mod interface;
pub mod joint;
pub mod mate;
pub mod occurrence;
pub mod reference;
pub mod topology;
pub mod value;

pub use asset::{AssetProvenance, ExternalAssetId};
pub use bom::BomClassificationId;
pub use component::{
    ChildInstance, ComponentDefinition, ComponentDefinitionRegistry, ParameterDeclaration,
    RegistryError,
};
pub use configuration::ConfigurationSlotId;
pub use definition::ComponentDefinitionId;
pub use frame::{LocalPose, WorldPose};
pub use graph::{AssemblyGraphError, Occurrence, expand};
pub use instance::LogicalInstanceId;
pub use interface::{
    CompatibilityError, ConformanceError, InterfaceField, InterfaceFieldType, InterfaceValue,
    MechanicalInterface, MechanicalInterfaceInstance, Side, check_compatibility, check_conformance,
};
pub use joint::{Joint, JointAxis, JointCoordinate, JointError, JointId, JointKind};
pub use mate::{Mate, MateError, MateId, MateKind};
pub use occurrence::OccurrencePath;
pub use reference::{
    AssemblyBrokenReason, AssemblyPartResolver, AssemblyReferenceResolution,
    AssemblyResolutionOutcome, resolve, resolve_with_durability,
};
pub use topology::OccurrenceTopologyRef;
pub use value::ParameterValue;
