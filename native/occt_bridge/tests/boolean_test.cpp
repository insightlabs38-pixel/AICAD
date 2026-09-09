// AICAD-026: boolean union/cut/intersect.

#include "aicad_occt_bridge.h"

#include <cmath>
#include <cstdio>
#include <cstdlib>

namespace {

int g_failures = 0;

void Check(bool condition, const char* what) {
  if (condition) {
    std::printf("PASS: %s\n", what);
  } else {
    std::printf("FAIL: %s\n", what);
    g_failures += 1;
  }
}

bool NearlyEqual(double a, double b, double tol) { return std::fabs(a - b) <= tol; }

// Translates `shape` by (dx, dy, dz) via aicad_occt_transform_shape (a
// pure-translation rigid transform: identity linear part, translation
// column).
aicad_shape_handle_t Translate(aicad_occt_context_t* ctx,
                                aicad_shape_handle_t shape,
                                double dx,
                                double dy,
                                double dz) {
  const double matrix[12] = {1, 0, 0, dx, 0, 1, 0, dy, 0, 0, 1, dz};
  aicad_shape_handle_t out = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_transform_shape(ctx, shape, matrix, &out);
  return out;
}

}  // namespace

int main() {
  aicad_occt_context_t* ctx = nullptr;
  Check(aicad_occt_context_create(&ctx) == AICAD_OCCT_OK, "context_create succeeds");

  // --- Two 2x2x2 boxes overlapping in a 1x1x1 corner region: box A at the
  //     origin, box B translated by (1,1,1). Analytic inclusion-exclusion:
  //     union = 8+8-1=15, cut (A-B) = 8-1=7, intersect = 1. ---
  aicad_shape_handle_t box_a = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &box_a);
  aicad_shape_handle_t box_b_raw = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &box_b_raw);
  aicad_shape_handle_t box_b = Translate(ctx, box_b_raw, 1.0, 1.0, 1.0);

  aicad_shape_handle_t fused = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_union(ctx, box_a, box_b, &fused) == AICAD_OCCT_OK,
        "boolean_union succeeds for two overlapping solid boxes");
  int is_valid = 0;
  Check(aicad_occt_shape_is_valid(ctx, fused, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the union result is a valid B-rep");
  double union_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, fused, &union_volume) == AICAD_OCCT_OK &&
            NearlyEqual(union_volume, 15.0, 1e-6),
        "union volume matches inclusion-exclusion: 8+8-1=15");

  aicad_shape_handle_t cut = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_cut(ctx, box_a, box_b, &cut) == AICAD_OCCT_OK,
        "boolean_cut succeeds for two overlapping solid boxes");
  Check(aicad_occt_shape_is_valid(ctx, cut, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the cut result is a valid B-rep");
  double cut_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, cut, &cut_volume) == AICAD_OCCT_OK &&
            NearlyEqual(cut_volume, 7.0, 1e-6),
        "cut (A-B) volume matches 8-1=7");

  aicad_shape_handle_t common = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_intersect(ctx, box_a, box_b, &common) == AICAD_OCCT_OK,
        "boolean_intersect succeeds for two overlapping solid boxes");
  Check(aicad_occt_shape_is_valid(ctx, common, &is_valid) == AICAD_OCCT_OK && is_valid == 1,
        "the intersect result is a valid B-rep");
  double common_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, common, &common_volume) == AICAD_OCCT_OK &&
            NearlyEqual(common_volume, 1.0, 1e-6),
        "intersect volume matches the 1x1x1 overlap region");

  // --- disjoint (non-overlapping) boxes: union volume is exactly the sum,
  //     cut leaves A unchanged, intersect is empty (zero volume). ---
  aicad_shape_handle_t disjoint_a = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &disjoint_a);
  aicad_shape_handle_t disjoint_b_raw = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 1.0, 1.0, 1.0, &disjoint_b_raw);
  aicad_shape_handle_t disjoint_b = Translate(ctx, disjoint_b_raw, 10.0, 10.0, 10.0);
  aicad_shape_handle_t disjoint_union = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_union(ctx, disjoint_a, disjoint_b, &disjoint_union) == AICAD_OCCT_OK,
        "boolean_union succeeds for two disjoint boxes");
  double disjoint_union_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, disjoint_union, &disjoint_union_volume) == AICAD_OCCT_OK &&
            NearlyEqual(disjoint_union_volume, 2.0, 1e-9),
        "disjoint union volume is exactly the sum (1+1=2)");
  aicad_shape_handle_t disjoint_common = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_intersect(ctx, disjoint_a, disjoint_b, &disjoint_common) ==
            AICAD_OCCT_OK,
        "boolean_intersect succeeds (constructs) for two disjoint boxes");
  double disjoint_common_volume = -1.0;
  Check(aicad_occt_shape_volume(ctx, disjoint_common, &disjoint_common_volume) == AICAD_OCCT_OK &&
            NearlyEqual(disjoint_common_volume, 0.0, 1e-9),
        "disjoint intersect volume is exactly zero (empty result)");

  // --- chaining: a boolean result (a COMPOUND, not a SOLID) must itself
  //     be usable as an operand to a further boolean op -- the reason
  //     these operations do not restrict operand handle kind the way
  //     extrude/revolve/sweep restrict theirs to Face/Wire (see
  //     project/reports/AICAD-026.md). ---
  aicad_shape_handle_t third_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 3.0, 3.0, 3.0, &third_box);
  aicad_shape_handle_t third_box_far = Translate(ctx, third_box, 100.0, 100.0, 100.0);
  aicad_shape_handle_t chained = AICAD_NULL_SHAPE_HANDLE;
  Check(aicad_occt_boolean_union(ctx, fused, third_box_far, &chained) == AICAD_OCCT_OK,
        "a boolean result (COMPOUND) can be chained as an operand into a further boolean op");
  double chained_volume = 0.0;
  Check(aicad_occt_shape_volume(ctx, chained, &chained_volume) == AICAD_OCCT_OK &&
            NearlyEqual(chained_volume, 15.0 + 27.0, 1e-6),
        "chained union volume is exactly the sum of two disjoint pieces (15+27=42)");

  // --- adversarial: invalid/stale/foreign handles ---
  // AICAD_NULL_SHAPE_HANDLE's context_id is 0, which never matches a real
  // context's id (Stage-1 kernel policy: context_id 0 is never assigned)
  // -- so this is rejected as FOREIGN_CONTEXT (context_id mismatch is
  // checked before the slot table is even consulted), not
  // INVALID_HANDLE/STALE_HANDLE.
  Check(aicad_occt_boolean_union(ctx, AICAD_NULL_SHAPE_HANDLE, box_a, &fused) ==
            AICAD_OCCT_ERR_FOREIGN_CONTEXT,
        "boolean_union rejects an all-zero (never-issued) handle for the first operand");
  aicad_occt_context_t* ctx2 = nullptr;
  aicad_occt_context_create(&ctx2);
  aicad_shape_handle_t foreign_box = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx2, 1.0, 1.0, 1.0, &foreign_box);
  Check(aicad_occt_boolean_union(ctx, box_a, foreign_box, &fused) ==
            AICAD_OCCT_ERR_FOREIGN_CONTEXT,
        "boolean_union rejects a handle belonging to a different context");
  aicad_occt_context_destroy(ctx2);

  // --- AICAD-026/SESSION_HANDOFF: "epoch bump on mutation" -- the first
  //     topology-mutating operation combining two independently-owned
  //     input shapes (deferred by AICAD-016/018/019's own reports
  //     pending exactly this operation existing). Proves: (1) producing
  //     a boolean result does not silently invalidate or alias either
  //     input handle (DL-2 functional/value semantics hold for two-input
  //     operations, not just the single-input ones already proven in
  //     Batch 1B); (2) explicitly releasing one input afterward correctly
  //     bumps that slot's generation and rejects further use of the
  //     stale handle, while the boolean result and the other, unreleased
  //     input remain fully valid and correctly queryable -- releasing one
  //     shape must never disturb an unrelated shape sharing its
  //     lifecycle history, matching AICAD-018's
  //     multiple_shapes_in_one_context_have_independent_lifecycles
  //     pattern extended to a shape that was itself a boolean operand. ---
  aicad_shape_handle_t epoch_a = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &epoch_a);
  aicad_shape_handle_t epoch_b_raw = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 2.0, 2.0, 2.0, &epoch_b_raw);
  aicad_shape_handle_t epoch_b = Translate(ctx, epoch_b_raw, 1.0, 1.0, 1.0);
  aicad_shape_handle_t epoch_result = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_boolean_union(ctx, epoch_a, epoch_b, &epoch_result);

  double a_volume_before = 0.0, b_volume_before = 0.0;
  aicad_occt_shape_volume(ctx, epoch_a, &a_volume_before);
  aicad_occt_shape_volume(ctx, epoch_b, &b_volume_before);
  Check(NearlyEqual(a_volume_before, 8.0, 1e-9) && NearlyEqual(b_volume_before, 8.0, 1e-9),
        "both boolean-union inputs remain independently valid with unchanged volumes "
        "immediately after producing the union result (no silent mutation/aliasing)");

  Check(aicad_occt_release_shape(ctx, epoch_a) == AICAD_OCCT_OK,
        "explicitly releasing one boolean-union input succeeds");
  double a_volume_after_release = 0.0;
  Check(aicad_occt_shape_volume(ctx, epoch_a, &a_volume_after_release) ==
            AICAD_OCCT_ERR_STALE_HANDLE,
        "the released input's handle is now correctly rejected as stale (epoch bumped)");
  double b_volume_after_release = 0.0, result_volume_after_release = 0.0;
  Check(aicad_occt_shape_volume(ctx, epoch_b, &b_volume_after_release) == AICAD_OCCT_OK &&
            NearlyEqual(b_volume_after_release, 8.0, 1e-9),
        "the OTHER (unreleased) boolean-union input remains fully valid after its sibling's "
        "release, with its volume unchanged");
  Check(aicad_occt_shape_volume(ctx, epoch_result, &result_volume_after_release) ==
            AICAD_OCCT_OK &&
            NearlyEqual(result_volume_after_release, 15.0, 1e-6),
        "the boolean UNION RESULT remains fully valid and unchanged after one of its two "
        "original inputs is released (the result does not alias/retain the input's slot)");

  // A new shape created after the release should be free to reuse the
  // freed slot, but its handle must never equal the released handle
  // (generation bump prevents aliasing) -- the same invariant
  // `dropping_and_recreating_reuses_the_slot_without_aliasing` proves in
  // the Rust test suite, exercised here specifically via a
  // boolean-operation input's release.
  aicad_shape_handle_t reused = AICAD_NULL_SHAPE_HANDLE;
  aicad_occt_create_box(ctx, 9.0, 9.0, 9.0, &reused);
  Check(!(reused.slot == epoch_a.slot && reused.generation == epoch_a.generation),
        "a shape created after releasing a boolean-union input never aliases the released "
        "handle's identity, even if it reuses the same slot");

  Check(aicad_occt_context_destroy(ctx) == AICAD_OCCT_OK, "context_destroy succeeds");

  if (g_failures > 0) {
    std::fprintf(stderr, "boolean_test: %d check(s) FAILED\n", g_failures);
    return 1;
  }
  std::printf("boolean_test: all checks PASSED\n");
  return 0;
}
