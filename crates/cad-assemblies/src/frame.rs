//! [`LocalPose`]/[`WorldPose`] — rigid instance poses (`AICAD-135`,
//! `AGENTS.md`'s "FRAMES / POSES": "Be explicit about: local frame, parent
//! frame, world frame, occurrence transform ... where relevant").
//!
//! Reuses `cad_kernel_api::Transform` unchanged for the actual
//! rigid-motion math (rotation/translation composition and inversion) —
//! the same already-audited, kernel-neutral spatial model Stage-3/5
//! modeling operations already share (see that module's own doc comment)
//! — rather than reinventing rotation composition here. What this module
//! adds on top is *typed* pose identity: [`LocalPose`] (an occurrence's
//! transform relative to its immediate parent occurrence, per
//! [`crate::component::ChildInstance::local_pose`]) and [`WorldPose`] (an
//! occurrence's transform relative to the assembly root/world frame,
//! obtained only by composing a chain of `LocalPose`s from root to leaf)
//! are distinct Rust types precisely so a local pose can never be passed
//! where a world pose is expected, or vice versa.
//!
//! # Determinism and identity durability
//!
//! Composition is ordinary deterministic floating-point arithmetic over
//! an explicit root-to-leaf chain (never a `HashMap`/`HashSet` walk), and
//! neither type appears anywhere in [`crate::instance::LogicalInstanceId`]
//! or [`crate::occurrence::OccurrencePath`] — changing an instance's pose
//! can therefore never change its logical identity or occurrence path,
//! structurally rather than by convention (`AGENTS.md`: "Pose changes
//! must not change logical instance identity").

use cad_diagnostics::json::Json;
use cad_kernel_api::Transform;

fn transform_to_json(transform: Transform) -> Json {
    Json::Array(
        transform
            .to_row_major_3x4()
            .into_iter()
            .map(Json::Float)
            .collect(),
    )
}

/// The rigid transform placing one occurrence's own local frame into its
/// *immediate parent's* frame — never composed across more than one
/// nesting level by itself; see [`LocalPose::under`] to fold it into a
/// [`WorldPose`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPose(Transform);

impl LocalPose {
    pub fn new(transform: Transform) -> LocalPose {
        LocalPose(transform)
    }

    /// The identity local pose: the occurrence's local frame coincides
    /// exactly with its parent's frame.
    pub fn identity() -> LocalPose {
        LocalPose(Transform::identity())
    }

    pub fn transform(&self) -> Transform {
        self.0
    }

    /// The pose that undoes this one (child-frame-to-parent-frame becomes
    /// parent-frame-to-child-frame).
    pub fn invert(&self) -> LocalPose {
        LocalPose(self.0.invert())
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("local_pose")),
            ("matrix".to_string(), transform_to_json(self.0)),
        ])
    }

    /// Resolves this local pose into a [`WorldPose`], given the world
    /// pose of the immediate parent occurrence it is relative to — the
    /// only way a [`WorldPose`] is ever produced, so every world pose is
    /// traceably the composition of an explicit parent chain, never an
    /// ungrounded guess (`project/DECISION_LOG.md#DL-30`'s deterministic
    /// grounding principle, which `AICAD-143` will apply at the solver
    /// boundary; this module only establishes the composition mechanics
    /// it depends on).
    pub fn under(&self, parent_world: &WorldPose) -> WorldPose {
        WorldPose(self.0.compose(&parent_world.0))
    }
}

/// The rigid transform placing one occurrence's local frame into the
/// assembly root's world frame — always the fold of a root-to-leaf chain
/// of [`LocalPose`]s via [`LocalPose::under`], never constructed from raw
/// numbers directly (there is deliberately no `WorldPose::new`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPose(Transform);

impl WorldPose {
    /// The world pose of a root-level occurrence: its own local pose,
    /// interpreted directly against the world frame (an implicit identity
    /// parent).
    pub fn root(local: &LocalPose) -> WorldPose {
        local.under(&WorldPose(Transform::identity()))
    }

    pub fn transform(&self) -> Transform {
        self.0
    }

    pub fn invert(&self) -> WorldPose {
        WorldPose(self.0.invert())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::{Axis3, Direction3, Point3, Vector3};

    fn assert_point_close(a: Point3, b: Point3, tol: f64) {
        assert!((a.x - b.x).abs() < tol);
        assert!((a.y - b.y).abs() < tol);
        assert!((a.z - b.z).abs() < tol);
    }

    #[test]
    fn identity_local_pose_at_root_is_the_identity_world_pose() {
        let world = WorldPose::root(&LocalPose::identity());
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_point_close(world.transform().apply_point(p), p, 1e-12);
    }

    #[test]
    fn nested_local_poses_compose_deterministically_into_a_world_pose() {
        // Parent translated by (10, 0, 0); child translated by (0, 5, 0)
        // relative to the parent -> child's world position is (10, 5, 0).
        let parent_local = LocalPose::new(Transform::translation(Vector3::new(10.0, 0.0, 0.0)));
        let parent_world = WorldPose::root(&parent_local);

        let child_local = LocalPose::new(Transform::translation(Vector3::new(0.0, 5.0, 0.0)));
        let child_world = child_local.under(&parent_world);

        let expected = Point3::new(10.0, 5.0, 0.0);
        assert_point_close(
            child_world.transform().apply_point(Point3::ORIGIN),
            expected,
            1e-9,
        );
    }

    #[test]
    fn rebuilding_the_same_chain_independently_reproduces_an_identical_world_pose() {
        let build = || {
            let root = LocalPose::new(Transform::rotation(
                Axis3::new(Point3::ORIGIN, Direction3::Z),
                0.4,
            ));
            let root_world = WorldPose::root(&root);
            let child = LocalPose::new(Transform::translation(Vector3::new(2.0, 0.0, 0.0)));
            child.under(&root_world)
        };
        let a = build();
        let b = build();
        assert_eq!(a, b);
    }

    #[test]
    fn world_pose_invert_undoes_the_full_chain() {
        let root_world = WorldPose::root(&LocalPose::new(Transform::translation(Vector3::new(
            1.0, 2.0, 3.0,
        ))));
        let child_world = LocalPose::new(Transform::rotation(
            Axis3::new(Point3::ORIGIN, Direction3::X),
            0.9,
        ))
        .under(&root_world);

        let p = Point3::new(-1.0, 4.0, 2.0);
        let round_tripped = child_world
            .invert()
            .transform()
            .apply_point(child_world.transform().apply_point(p));
        assert_point_close(round_tripped, p, 1e-9);
    }

    #[test]
    fn local_pose_invert_round_trips() {
        let local = LocalPose::new(Transform::rotation(
            Axis3::new(Point3::new(1.0, 0.0, 0.0), Direction3::Y),
            1.2,
        ));
        let composed = local.transform().compose(&local.invert().transform());
        let p = Point3::new(3.0, -2.0, 1.0);
        assert_point_close(composed.apply_point(p), p, 1e-9);
    }
}
