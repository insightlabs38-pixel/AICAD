//! Small dense-matrix numeric utilities shared by `crate::grounding`'s
//! structural DOF analysis (`AICAD-143`) and `crate::baseline`'s
//! iterative solver (`AICAD-144`) -- kept in one place so "how many
//! degrees of freedom remain" and "how do we take a solver step" agree on
//! the same rank/pivoting convention. Plain hand-rolled dense linear
//! algebra (no external crate), bounded by the caller's own
//! variable/residual counts -- `AGENTS.md`'s resource-budget non-negotiable.

/// Central-finite-difference Jacobian of `eval` at `x`: row `i`, column
/// `j` is `d(eval(x)[i]) / d(x[j])`. `O(2n)` evaluations of `eval`.
pub fn jacobian(eval: impl Fn(&[f64]) -> Vec<f64>, x: &[f64], step: f64) -> Vec<Vec<f64>> {
    let n = x.len();
    let base = eval(x);
    let m = base.len();
    let mut jac = vec![vec![0.0; n]; m];
    for j in 0..n {
        let mut plus = x.to_vec();
        let mut minus = x.to_vec();
        plus[j] += step;
        minus[j] -= step;
        let f_plus = eval(&plus);
        let f_minus = eval(&minus);
        for i in 0..m {
            jac[i][j] = (f_plus[i] - f_minus[i]) / (2.0 * step);
        }
    }
    jac
}

/// Numeric rank of `matrix` (rows may exceed columns) via row-echelon
/// reduction with partial pivoting: a pivot below `tolerance` in
/// magnitude is treated as zero. Deterministic (fixed pivot-selection
/// tie-break: the first row at/after the current rank achieving the
/// largest pivot magnitude).
pub fn rank(matrix: &[Vec<f64>], tolerance: f64) -> usize {
    rank_with_pivot_rows(matrix, tolerance).0
}

/// [`rank`], additionally returning the *original* (pre-reduction) row
/// indices selected as pivots, in the order they were chosen --
/// `crate::conflict`'s (`AICAD-146`) redundancy/conflict classification
/// needs to know exactly *which* declared residual rows are linearly
/// independent so it can label every other row as dependent on them.
///
/// Indexed row/column loops throughout (`#[allow(clippy::
/// needless_range_loop)]`): every loop here walks a genuine `(row, col)`
/// coordinate pair into `m`, not a single sequence an iterator adapter
/// would express more clearly.
#[allow(clippy::needless_range_loop)]
pub fn rank_with_pivot_rows(matrix: &[Vec<f64>], tolerance: f64) -> (usize, Vec<usize>) {
    let rows = matrix.len();
    if rows == 0 {
        return (0, Vec::new());
    }
    let cols = matrix[0].len();
    let mut m = matrix.to_vec();
    let mut original_row: Vec<usize> = (0..rows).collect();
    let mut rank = 0;
    let mut pivot_rows = Vec::new();
    for col in 0..cols {
        if rank == rows {
            break;
        }
        let mut pivot_row = None;
        let mut pivot_val = tolerance;
        for r in rank..rows {
            let v = m[r][col].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = Some(r);
            }
        }
        let Some(pivot_row) = pivot_row else {
            continue;
        };
        m.swap(rank, pivot_row);
        original_row.swap(rank, pivot_row);
        for r in (rank + 1)..rows {
            let factor = m[r][col] / m[rank][col];
            if factor != 0.0 {
                for c in col..cols {
                    m[r][c] -= factor * m[rank][c];
                }
            }
        }
        pivot_rows.push(original_row[rank]);
        rank += 1;
    }
    (rank, pivot_rows)
}

/// Solves the square linear system `a * x = b` via Gaussian elimination
/// with partial pivoting. Returns `None` if `a` is singular (or too
/// close to singular) at `1e-14` pivot magnitude -- callers needing a
/// robust step near singularity (`crate::baseline`) add Levenberg-
/// Marquardt damping to `a` themselves before calling this.
#[allow(clippy::needless_range_loop)]
pub fn solve_linear_system(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = b.len();
    debug_assert!(a.iter().all(|row| row.len() == n));
    let mut m = a.to_vec();
    let mut rhs = b.to_vec();
    for col in 0..n {
        let mut pivot_row = col;
        let mut pivot_val = m[col][col].abs();
        for r in (col + 1)..n {
            let v = m[r][col].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = r;
            }
        }
        if pivot_val < 1e-14 {
            return None;
        }
        m.swap(col, pivot_row);
        rhs.swap(col, pivot_row);
        for r in (col + 1)..n {
            let factor = m[r][col] / m[col][col];
            if factor != 0.0 {
                for c in col..n {
                    m[r][c] -= factor * m[col][c];
                }
                rhs[r] -= factor * rhs[col];
            }
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut sum = rhs[row];
        for c in (row + 1)..n {
            sum -= m[row][c] * x[c];
        }
        x[row] = sum / m[row][row];
    }
    Some(x)
}

/// `a^T * a` for an `m x n` dense matrix `a` -- the normal-equations
/// left-hand side both `crate::grounding`'s rank analysis callers and
/// `crate::baseline`'s Gauss-Newton step build from the same Jacobian.
pub fn gram(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let m = a.len();
    let n = if m == 0 { 0 } else { a[0].len() };
    let mut g = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0;
            for row in a.iter().take(m) {
                sum += row[i] * row[j];
            }
            g[i][j] = sum;
        }
    }
    g
}

/// `a^T * v` for an `m x n` dense matrix `a` and length-`m` vector `v`.
pub fn transpose_mul_vec(a: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    let m = a.len();
    let n = if m == 0 { 0 } else { a[0].len() };
    let mut out = vec![0.0; n];
    for (row, vi) in a.iter().zip(v.iter()).take(m).map(|(row, vi)| (row, *vi)) {
        for (j, out_j) in out.iter_mut().enumerate() {
            *out_j += row[j] * vi;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jacobian_of_a_linear_map_is_exact() {
        // f(x, y) = (2x + y, x - 3y)
        let eval = |x: &[f64]| vec![2.0 * x[0] + x[1], x[0] - 3.0 * x[1]];
        let jac = jacobian(eval, &[1.0, 1.0], 1e-6);
        assert!((jac[0][0] - 2.0).abs() < 1e-4);
        assert!((jac[0][1] - 1.0).abs() < 1e-4);
        assert!((jac[1][0] - 1.0).abs() < 1e-4);
        assert!((jac[1][1] - (-3.0)).abs() < 1e-4);
    }

    #[test]
    fn rank_of_a_full_rank_square_matrix_is_its_dimension() {
        let m = vec![vec![2.0, 0.0], vec![0.0, 3.0]];
        assert_eq!(rank(&m, 1e-9), 2);
    }

    #[test]
    fn rank_of_a_rank_deficient_matrix_is_less_than_its_dimension() {
        let m = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        assert_eq!(rank(&m, 1e-9), 1);
    }

    #[test]
    fn rank_with_pivot_rows_selects_one_row_of_a_dependent_pair_as_the_pivot() {
        let m = vec![vec![1.0, 2.0], vec![2.0, 4.0], vec![0.0, 1.0]];
        let (rank, pivots) = rank_with_pivot_rows(&m, 1e-9);
        assert_eq!(rank, 2);
        // Partial pivoting picks the larger-magnitude row of the
        // dependent pair (row 1, magnitude 2.0) over row 0 -- either
        // choice is a mathematically valid pivot, but this fixes which
        // one so the test is deterministic.
        assert_eq!(pivots, vec![1, 2]);
    }

    #[test]
    fn rank_of_an_all_zero_matrix_is_zero() {
        let m = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        assert_eq!(rank(&m, 1e-9), 0);
    }

    #[test]
    fn solve_linear_system_recovers_a_known_solution() {
        let a = vec![vec![2.0, 1.0], vec![1.0, 3.0]];
        let x_expected = [3.0, -1.0];
        let b = vec![
            a[0][0] * x_expected[0] + a[0][1] * x_expected[1],
            a[1][0] * x_expected[0] + a[1][1] * x_expected[1],
        ];
        let x = solve_linear_system(&a, &b).unwrap();
        assert!((x[0] - x_expected[0]).abs() < 1e-9);
        assert!((x[1] - x_expected[1]).abs() < 1e-9);
    }

    #[test]
    fn solve_linear_system_reports_none_for_a_singular_matrix() {
        let a = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        assert!(solve_linear_system(&a, &[1.0, 2.0]).is_none());
    }
}
