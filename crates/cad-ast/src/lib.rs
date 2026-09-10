//! `cad-ast` — the source-span model shared by `cad-lexer`, `cad-parser`,
//! and (per this crate's `README.md`) the `tree-sitter-aicad` conformance
//! test corpus, plus (starting with the parser tasks, `AICAD-041`
//! onward) the AICAD abstract syntax tree node types themselves.
//!
//! **Scope of `AICAD-039`:** only the span/position model (this file)
//! existed after that task. `AICAD-041` (expression parser and
//! precedence) added [`Ident`] and the `expr` module. `AICAD-042`
//! (declarations) added the `ty`/`item`/`stmt` modules. Each node type is
//! added by the parser task that first needs to produce it, per
//! `AGENTS.md` "No speculative future work" — see each module's own docs
//! for exactly which grammar productions it covers and which are left for
//! a later task in this batch.

mod expr;
mod ident;
mod item;
mod stmt;
mod ty;

pub use expr::{Arg, BinaryOp, Expr, ExprKind, UnaryOp};
pub use ident::Ident;
pub use item::{
    ConstDecl, EnumDecl, EnumVariant, EnumVariantKind, FieldDecl, FnDecl, Item, LetDecl, Param,
    ParamDecl, PartDecl, PartMember, StructDecl,
};
pub use stmt::{AssignStmt, Block, ExprStmt, LetStmt, Stmt, VarStmt};
pub use ty::Type;

/// A half-open byte-offset range into one source file: `[start, end)`.
///
/// Deliberately just two `u32`s (not a file id) — multi-file spans are not
/// yet needed by any Stage-2 task; when module resolution (`AICAD-044`)
/// needs to distinguish spans across files, the natural extension is a
/// separate `SourceId` carried alongside a `Span`, not a change to `Span`
/// itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Span {
        debug_assert!(start <= end, "Span::new: start {start} > end {end}");
        Span { start, end }
    }

    /// A zero-length span at `offset`, e.g. for an end-of-file token.
    pub fn empty_at(offset: u32) -> Span {
        Span {
            start: offset,
            end: offset,
        }
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn to_range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }

    /// The smallest span covering both `self` and `other`. Used when a
    /// parser (a later task) combines sub-node spans into a parent node's
    /// span.
    pub fn join(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// The exact source text this span covers, given the same source
    /// string the span was computed against. Panics (via slicing) if
    /// `source` is not that same string / the span is out of bounds or
    /// splits a UTF-8 character — both are caller bugs, not recoverable
    /// runtime conditions.
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.to_range()]
    }
}

/// A value paired with the source span it was parsed/lexed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Spanned<T> {
        Spanned { node, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            node: f(self.node),
            span: self.span,
        }
    }
}

/// A 1-based (line, column) source position, matching the
/// `cad-diagnostics`/RFC-0005 §3 `source.start`/`source.end` shape
/// exactly (`{"line": ..., "column": ...}`, both 1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumn {
    pub line: u32,
    pub column: u32,
}

/// Maps byte offsets in one source string to 1-based (line, column)
/// positions, for turning a lexer/parser [`Span`] into the line/column
/// information `cad-diagnostics::SourceSpan` needs. Columns count UTF-16
/// code units the way most editor/LSP protocols do... **no** — columns
/// here count Unicode scalar values (`char`s), a simpler and still
/// well-defined choice; if LSP integration (`crates/cad-lsp`, Stage 9)
/// later needs UTF-16 columns, that conversion belongs there, not here.
pub struct LineIndex {
    /// Byte offset of the start of each line; `line_starts[0] == 0`.
    line_starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(source: &str) -> LineIndex {
        let mut line_starts = vec![0u32];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset as u32 + 1);
            }
        }
        LineIndex { line_starts }
    }

    /// The 1-based line/column for a byte `offset`. `offset` may equal
    /// `source.len()` (end-of-file position); anything past that is
    /// clamped to the last valid position rather than panicking, since
    /// diagnostic code should never crash the compiler over a
    /// bounds-computation edge case.
    pub fn line_column(&self, source: &str, offset: u32) -> LineColumn {
        let offset = offset.min(source.len() as u32);
        // Find the last line whose start is <= offset.
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert_at) => insert_at - 1,
        };
        let line_start = self.line_starts[line_idx];
        let column = source[line_start as usize..offset as usize].chars().count() as u32 + 1;
        LineColumn {
            line: line_idx as u32 + 1,
            column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_join_covers_both_spans() {
        let a = Span::new(5, 10);
        let b = Span::new(2, 7);
        assert_eq!(a.join(b), Span::new(2, 10));
    }

    #[test]
    fn span_text_slices_source() {
        let source = "let width = 5mm;";
        let span = Span::new(4, 9);
        assert_eq!(span.text(source), "width");
    }

    #[test]
    fn line_index_first_line() {
        let source = "abc\ndef\nghi";
        let index = LineIndex::new(source);
        assert_eq!(
            index.line_column(source, 0),
            LineColumn { line: 1, column: 1 }
        );
        assert_eq!(
            index.line_column(source, 2),
            LineColumn { line: 1, column: 3 }
        );
    }

    #[test]
    fn line_index_after_newline() {
        let source = "abc\ndef\nghi";
        let index = LineIndex::new(source);
        // 'd' is at byte offset 4, the first byte of line 2.
        assert_eq!(
            index.line_column(source, 4),
            LineColumn { line: 2, column: 1 }
        );
        // 'g' is at byte offset 8, the first byte of line 3.
        assert_eq!(
            index.line_column(source, 8),
            LineColumn { line: 3, column: 1 }
        );
    }

    #[test]
    fn line_index_end_of_file() {
        let source = "abc\ndef";
        let index = LineIndex::new(source);
        assert_eq!(
            index.line_column(source, source.len() as u32),
            LineColumn { line: 2, column: 4 }
        );
    }

    #[test]
    fn line_index_handles_multibyte_characters_by_char_count() {
        // "café" is 4 chars but 5 bytes ('é' is 2 UTF-8 bytes); the
        // trailing "x" must still be reported as column 2 of line 2 (char
        // count), not some byte-offset-derived column.
        let source = "café\nx";
        let index = LineIndex::new(source);
        assert_eq!(source.len(), 7);
        assert_eq!(
            index.line_column(source, source.len() as u32),
            LineColumn { line: 2, column: 2 }
        );
    }

    #[test]
    fn line_index_empty_source() {
        let index = LineIndex::new("");
        assert_eq!(index.line_column("", 0), LineColumn { line: 1, column: 1 });
    }
}
