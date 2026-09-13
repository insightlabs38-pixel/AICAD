# Geometry architecture

AICAD separates the source-level Safe CAD function catalogue from the internal Geometry IR and from the native kernel adapter.

- `cad-geometry-api` owns backend-neutral geometry operation/query representation and opaque graph identities.
- `cad-geometry-runtime` dispatches/evaluates that representation against a kernel implementation.
- `cad-kernel-api` defines kernel-neutral geometric/spatial contracts.
- `cad-occt-bridge` implements those contracts using OpenCascade.

Source `Geometry` values never expose OCCT objects or native topology pointers. Some Stage-3 source operations still accept raw integer face/edge selectors; these are documented limitations rather than durable semantic identity.

See [`safe-cad-api.md`](safe-cad-api.md) for the detailed current source API.
