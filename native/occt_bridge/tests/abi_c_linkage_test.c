/* AICAD-016: proves aicad_occt_bridge.h is genuinely plain-C-consumable
 * (not just "compiles under extern \"C\" but was never actually built as
 * C"), since the eventual Rust FFI binding (AICAD-018) links against this
 * ABI the same way a C caller would — through the C header, not any
 * C++ name-mangled symbol. Deliberately minimal: the full behavioral
 * matrix is covered by tests/abi_smoke_test.cpp. */

#include <math.h>
#include <stdio.h>

#include "aicad_occt_bridge.h"

int main(void) {
  AicadKernelContext *context = aicad_kernel_context_create();
  if (context == NULL) {
    fprintf(stderr, "FAIL: context creation returned NULL\n");
    return 1;
  }

  AicadShapeHandle handle;
  handle.id = 0;
  AicadStatus status = aicad_create_box(context, 2.0, 3.0, 6.0, &handle);
  if (status != AICAD_STATUS_OK) {
    fprintf(stderr, "FAIL: aicad_create_box status=%d error=%s\n",
            (int)status, aicad_kernel_context_last_error(context));
    aicad_kernel_context_destroy(context);
    return 1;
  }

  double diagonal = 0.0;
  status = aicad_shape_bbox_diagonal(context, handle, &diagonal);
  if (status != AICAD_STATUS_OK) {
    fprintf(stderr, "FAIL: aicad_shape_bbox_diagonal status=%d\n", (int)status);
    aicad_kernel_context_destroy(context);
    return 1;
  }

  /* 2x3x6 box: diagonal = sqrt(4+9+36) = 7 exactly. */
  const double expected = 7.0;
  if (fabs(diagonal - expected) > 1e-9) {
    fprintf(stderr, "FAIL: diagonal=%f expected=%f\n", diagonal, expected);
    aicad_kernel_context_destroy(context);
    return 1;
  }

  status = aicad_shape_release(context, handle);
  if (status != AICAD_STATUS_OK) {
    fprintf(stderr, "FAIL: aicad_shape_release status=%d\n", (int)status);
    aicad_kernel_context_destroy(context);
    return 1;
  }

  aicad_kernel_context_destroy(context);
  printf("C_LINKAGE_TEST_RESULT=PASS\n");
  return 0;
}
