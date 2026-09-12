//! Raw, `unsafe` FFI declarations mirroring
//! `native/occt_bridge/include/aicad_occt_bridge.h` exactly.
//!
//! Nothing in this module is exported from the crate (`mod ffi;` in
//! `lib.rs`, not `pub mod ffi;`). Every public API in this crate goes
//! through `lib.rs`'s safe wrapper types instead. Status codes are
//! received as plain `c_int` here, not as a `#[repr(i32)]` Rust enum --
//! see `lib.rs`'s `status_result` for why (matching an unenumerated
//! `#[repr]` enum value from C is undefined behavior; matching an
//! unenumerated plain integer is not).

#![allow(non_camel_case_types)]

use std::os::raw::c_int;

/// Opaque native kernel context (`aicad_occt_context_t` in the header).
/// The zero-sized-array field is the standard stable-Rust idiom for an
/// opaque FFI type: it has an indeterminate size/alignment from Rust's
/// point of view and can only ever be used behind a pointer.
#[repr(C)]
pub struct aicad_occt_context_t {
    _private: [u8; 0],
}

/// Mirrors `aicad_shape_handle_t` field-for-field: `uint64_t context_id;
/// uint32_t slot; uint32_t generation;`. `#[repr(C)]` on an identical
/// field sequence gives an identical layout to the C struct (8-byte
/// aligned, 16 bytes total, no padding).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct aicad_shape_handle_t {
    pub context_id: u64,
    pub slot: u32,
    pub generation: u32,
}

unsafe extern "C" {
    pub fn aicad_occt_context_create(out_context: *mut *mut aicad_occt_context_t) -> c_int;
    pub fn aicad_occt_context_destroy(context: *mut aicad_occt_context_t) -> c_int;
    pub fn aicad_occt_release_shape(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_create_box(
        context: *mut aicad_occt_context_t,
        dx: f64,
        dy: f64,
        dz: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_is_valid(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_is_valid: *mut c_int,
    ) -> c_int;
    pub fn aicad_occt_shape_volume(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_volume: *mut f64,
    ) -> c_int;
    pub fn aicad_occt_create_cylinder(
        context: *mut aicad_occt_context_t,
        radius: f64,
        height: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_transform_shape(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        matrix: *const f64, // [f64; 12], row-major 3x4
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_make_line_edge(
        context: *mut aicad_occt_context_t,
        p0: *const f64, // [f64; 3]
        p1: *const f64, // [f64; 3]
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_make_circle_wire(
        context: *mut aicad_occt_context_t,
        center: *const f64, // [f64; 3]
        normal: *const f64, // [f64; 3]
        radius: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_make_arc_edge(
        context: *mut aicad_occt_context_t,
        p_start: *const f64, // [f64; 3]
        p_mid: *const f64,   // [f64; 3]
        p_end: *const f64,   // [f64; 3]
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_make_wire_from_edges(
        context: *mut aicad_occt_context_t,
        edges: *const aicad_shape_handle_t,
        edge_count: usize,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_make_face_from_wire(
        context: *mut aicad_occt_context_t,
        wire_handle: aicad_shape_handle_t,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_extrude(
        context: *mut aicad_occt_context_t,
        face_handle: aicad_shape_handle_t,
        direction: *const f64, // [f64; 3]
        distance: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_revolve(
        context: *mut aicad_occt_context_t,
        face_handle: aicad_shape_handle_t,
        axis_origin: *const f64,    // [f64; 3]
        axis_direction: *const f64, // [f64; 3]
        angle_radians: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_sweep(
        context: *mut aicad_occt_context_t,
        profile_face_handle: aicad_shape_handle_t,
        spine_wire_handle: aicad_shape_handle_t,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_loft(
        context: *mut aicad_occt_context_t,
        sections: *const aicad_shape_handle_t,
        section_count: usize,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_boolean_union(
        context: *mut aicad_occt_context_t,
        a: aicad_shape_handle_t,
        b: aicad_shape_handle_t,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_boolean_cut(
        context: *mut aicad_occt_context_t,
        a: aicad_shape_handle_t,
        b: aicad_shape_handle_t,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_boolean_intersect(
        context: *mut aicad_occt_context_t,
        a: aicad_shape_handle_t,
        b: aicad_shape_handle_t,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_edge_count(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_count: *mut usize,
    ) -> c_int;
    pub fn aicad_occt_shape_get_edge(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        index: usize,
        out_edge_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_fillet(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        edges: *const aicad_shape_handle_t,
        edge_count: usize,
        radius: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_chamfer(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        edges: *const aicad_shape_handle_t,
        edge_count: usize,
        distance: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_face_count(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_count: *mut usize,
    ) -> c_int;
    pub fn aicad_occt_shape_get_face(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        index: usize,
        out_face_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shell(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        faces_to_remove: *const aicad_shape_handle_t,
        face_count: usize,
        thickness: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_offset(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        distance: f64,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_area(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_area: *mut f64,
    ) -> c_int;
    pub fn aicad_occt_shape_bounding_box(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_min: *mut f64, // [f64; 3]
        out_max: *mut f64, // [f64; 3]
    ) -> c_int;
    pub fn aicad_occt_shape_vertex_count(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_count: *mut usize,
    ) -> c_int;
    pub fn aicad_occt_shape_get_vertex(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        index: usize,
        out_vertex_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_edge_vertices(
        context: *mut aicad_occt_context_t,
        edge_handle: aicad_shape_handle_t,
        out_v0: *mut aicad_shape_handle_t,
        out_v1: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_edge_adjacent_face_count(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        edge_index: usize,
        out_count: *mut usize,
    ) -> c_int;
    pub fn aicad_occt_shape_edge_adjacent_face_get(
        context: *mut aicad_occt_context_t,
        shape_handle: aicad_shape_handle_t,
        edge_index: usize,
        adjacent_index: usize,
        out_face_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
    pub fn aicad_occt_shape_length(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_length: *mut f64,
    ) -> c_int;
    pub fn aicad_occt_shape_center_of_mass(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_center: *mut f64, // [f64; 3]
    ) -> c_int;
    pub fn aicad_occt_shape_validate(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_report: *mut aicad_validation_report_t,
    ) -> c_int;
    pub fn aicad_occt_tessellate(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        linear_deflection: f64,
        angular_deflection: f64,
        out_counts: *mut aicad_tessellation_counts_t,
    ) -> c_int;
    pub fn aicad_occt_tessellation_get(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        out_vertices: *mut f64,
        out_normals: *mut f64,
    ) -> c_int;
    pub fn aicad_occt_export_step(
        context: *mut aicad_occt_context_t,
        handle: aicad_shape_handle_t,
        file_path: *const std::os::raw::c_char,
    ) -> c_int;
    pub fn aicad_occt_import_step(
        context: *mut aicad_occt_context_t,
        file_path: *const std::os::raw::c_char,
        out_handle: *mut aicad_shape_handle_t,
    ) -> c_int;
}

/// Mirrors `aicad_tessellation_counts_t` field-for-field: `size_t
/// triangle_count;`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct aicad_tessellation_counts_t {
    pub triangle_count: usize,
}

/// Mirrors `aicad_validation_report_t` field-for-field: `int is_valid;
/// size_t invalid_vertex_count; size_t invalid_edge_count; size_t
/// invalid_wire_count; size_t invalid_face_count;`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct aicad_validation_report_t {
    pub is_valid: c_int,
    pub invalid_vertex_count: usize,
    pub invalid_edge_count: usize,
    pub invalid_wire_count: usize,
    pub invalid_face_count: usize,
}
