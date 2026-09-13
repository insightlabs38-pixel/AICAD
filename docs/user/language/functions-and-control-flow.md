# Functions and control flow

Geometry construction uses the same ordinary language mechanisms as scalar computation. Current repository examples define helper functions, use local bindings and mutable scalar/geometry variables where needed, and combine `if`, `while`, `for`, and `match` with Safe CAD calls.

## Functions

```aicad
fn hole_center_x(
    margin: Length,
    span: Length,
    count: Int,
    i: Int,
) -> Length {
    if count <= 1 {
        return margin;
    }
    let spacing = span / (count - 1);
    return margin + spacing * i;
}
```

Function parameters and return values are type-checked. A modeling helper can return `Geometry` exactly as another function returns `Length` or `Int`.

## Immutable and mutable local bindings

Use `let` for an immutable local binding and `var` when explicit rebinding is required:

```aicad
var radius = 8mm;
while radius * 2 > 10mm {
    radius = radius / 2;
}
```

AICAD's geometry semantics remain functional: a modeling operation returns a new value. Explicitly rebinding a `var` changes which value the name refers to; it does not mutate a hidden kernel shape in place.

## Conditions

```aicad
let result = if enabled {
    union(base, boss)
} else {
    base
};
```

`if` can be used in statement or value-producing expression contexts when the required branches are present and type-compatible.

## Ranges and `for`

Integer ranges provide deterministic ascending iteration:

```aicad
for i in 0..HOLE_COUNT {
    let x = margin + spacing * i;
    // use x
}
```

`start..end` is half-open; `start..=end` is inclusive. The iterable expression is evaluated once. Current automatic range iteration is defined for integer ranges; AICAD does not infer a step size for dimensional ranges.

List literals are also available:

```aicad
let selected_edges = [5, 9, 12];
```

## `match`

Current examples also use pattern matching for ordinary source decisions:

```aicad
let based = match style {
    0 => plate,
    _ => union(plate, boss),
};
```

Data-carrying enum variants and destructuring are implemented by the general language machinery rather than CAD-specific special cases.

## Resource bounds

The interpreter accounts for execution resources such as call depth. Runtime-backed geometry calls participate in ordinary call accounting; they do not bypass the language runtime simply because their implementation is supplied by the engine.
