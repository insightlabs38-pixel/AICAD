# Constraint architecture

Stage 3 provides a solver-independent sketch constraint representation and a bounded numerical sketch solver behind that representation.

The semantic constraint model is owned by AICAD; solver-native variable IDs/algorithm details are implementation data rather than source semantics. The sketch solver uses its own convergence/tolerance policy, which is distinct from geometry-validation/equivalence tolerances.

Assembly constraints, joints, requirements, and the broader post-100 verification model are not implemented merely because the Stage-3 sketch constraint infrastructure exists.
