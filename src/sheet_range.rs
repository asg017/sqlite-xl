use crate::parser::{
    column_name_to_idx, parse_cell_reference, parse_range_reference, CellReference,
    RangeReference,
};

/// A parsed sheet-qualified reference like `Sheet1!A1:B10` or `Sheet1!A1`.
#[derive(Debug, PartialEq)]
pub struct SheetReference {
    pub sheet: Option<String>,
    pub target: SheetTarget,
}

/// A single endpoint in a range that may omit column, row, or neither.
#[derive(Debug, PartialEq, Clone)]
pub struct Bound {
    /// 0-based column index, None if unbounded (e.g. `1:10`)
    pub col: Option<u32>,
    /// 0-based row index, None if unbounded (e.g. `A:B`)
    pub row: Option<u32>,
}

/// A range where either side can be partial.
///
/// Examples:
/// - `A:B`   → col-only bounds, all rows
/// - `1:10`  → row-only bounds, all columns
/// - `A1:D`  → full start, col-only end (all rows from A1 down through col D)
/// - `A:D5`  → col-only start, full end
#[derive(Debug, PartialEq)]
pub struct OpenRange {
    pub start: Bound,
    pub end: Bound,
}

#[derive(Debug, PartialEq)]
pub enum SheetTarget {
    Cell(CellReference),
    Range(RangeReference),
    OpenRange(OpenRange),
}

#[derive(Debug, PartialEq)]
pub enum ParseSheetReferenceError {
    Empty,
    InvalidReference(String),
}

impl std::fmt::Display for ParseSheetReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty input"),
            Self::InvalidReference(msg) => write!(f, "invalid reference: {msg}"),
        }
    }
}

/// Parse a sheet-qualified reference.
///
/// Supported formats:
/// - `A1`            → cell reference, no sheet
/// - `A1:B10`        → bounded range
/// - `A:B`           → column-only range (all rows)
/// - `1:10`          → row-only range (all columns)
/// - `A1:D`          → mixed: full start, col-only end
/// - `A:D5`          → mixed: col-only start, full end
/// - `Sheet1!A1`     → cell reference with sheet
/// - `Sheet1!A1:B10` → range with sheet
/// - `'My Sheet'!A:B`→ quoted sheet name
pub fn parse_sheet_reference(input: &str) -> Result<SheetReference, ParseSheetReferenceError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseSheetReferenceError::Empty);
    }

    let (sheet, ref_part) = split_sheet_and_ref(input);

    let target = parse_target(ref_part)?;

    Ok(SheetReference { sheet, target })
}

/// Split `Sheet1!A1:B10` into `(Some("Sheet1"), "A1:B10")`.
/// Split `'My Sheet'!A1` into `(Some("My Sheet"), "A1")`.
/// Split `A1:B10` into `(None, "A1:B10")`.
fn split_sheet_and_ref(input: &str) -> (Option<String>, &str) {
    if input.starts_with('\'') {
        // Quoted sheet name: find closing quote then '!'
        if let Some(end_quote) = input[1..].find('\'') {
            let sheet_name = &input[1..=end_quote];
            let rest = &input[end_quote + 2..]; // skip closing quote
            if let Some(ref_part) = rest.strip_prefix('!') {
                return (Some(sheet_name.to_owned()), ref_part);
            }
        }
        // Malformed quote — treat entire input as reference
        (None, input)
    } else if let Some(bang_pos) = input.find('!') {
        let sheet_name = &input[..bang_pos];
        let ref_part = &input[bang_pos + 1..];
        (Some(sheet_name.to_owned()), ref_part)
    } else {
        (None, input)
    }
}

/// Parse a single bound like `A`, `1`, `A1`, `D`, `D5`, or `*` (wildcard).
fn parse_bound(s: &str) -> Result<Bound, ParseSheetReferenceError> {
    if s.is_empty() {
        return Err(ParseSheetReferenceError::InvalidReference(
            "empty bound".to_owned(),
        ));
    }

    // Wildcard — fully unbounded
    if s == "*" {
        return Ok(Bound {
            col: None,
            row: None,
        });
    }

    // Strip trailing * for partial wildcard (e.g. `D*` = col D, all rows)
    let (s, row_wildcard) = if s.ends_with('*') {
        (&s[..s.len() - 1], true)
    } else {
        (s, false)
    };

    if s.is_empty() {
        return Err(ParseSheetReferenceError::InvalidReference("empty bound".to_owned()));
    }

    let has_alpha = s.chars().any(|c| c.is_alphabetic());
    let has_digit = s.chars().any(|c| c.is_ascii_digit());

    match (has_alpha, has_digit) {
        (true, true) => {
            // Full cell like `A1`, `D5`, `AB10`
            let split = s
                .find(|c: char| c.is_ascii_digit())
                .ok_or_else(|| ParseSheetReferenceError::InvalidReference(s.to_owned()))?;
            let col_part = &s[..split];
            let row_part = &s[split..];
            let col = column_name_to_idx(col_part)
                .map_err(|_| ParseSheetReferenceError::InvalidReference(s.to_owned()))?;
            let row: u32 = row_part
                .parse()
                .map_err(|_| ParseSheetReferenceError::InvalidReference(s.to_owned()))?;
            Ok(Bound {
                col: Some(col),
                row: if row_wildcard { None } else { Some(row - 1) },
            })
        }
        (true, false) => {
            // Column-only like `A`, `D`, `ZZ`
            let col = column_name_to_idx(s)
                .map_err(|_| ParseSheetReferenceError::InvalidReference(s.to_owned()))?;
            Ok(Bound {
                col: Some(col),
                row: None,
            })
        }
        (false, true) => {
            // Row-only like `1`, `10`, `9999`
            let row: u32 = s
                .parse()
                .map_err(|_| ParseSheetReferenceError::InvalidReference(s.to_owned()))?;
            Ok(Bound {
                col: None,
                row: Some(row - 1),
            })
        }
        (false, false) => Err(ParseSheetReferenceError::InvalidReference(s.to_owned())),
    }
}

/// Determine if a range has any partial bounds (col-only, row-only, or wildcard).
fn has_partial_bound(left: &str, right: &str) -> bool {
    if left.contains('*') || right.contains('*') {
        return true;
    }
    let left_alpha = left.chars().any(|c| c.is_alphabetic());
    let left_digit = left.chars().any(|c| c.is_ascii_digit());
    let right_alpha = right.chars().any(|c| c.is_alphabetic());
    let right_digit = right.chars().any(|c| c.is_ascii_digit());

    // If either side is missing alpha or digit, it's partial
    !(left_alpha && left_digit && right_alpha && right_digit)
}

/// Try to parse as range first (contains ':'), then as cell.
fn parse_target(input: &str) -> Result<SheetTarget, ParseSheetReferenceError> {
    // Strip optional $ signs (absolute references)
    let cleaned: String = input.chars().filter(|c| *c != '$').collect();

    if let Some((left, right)) = cleaned.split_once(':') {
        if has_partial_bound(left, right) {
            // At least one side is unbounded — use OpenRange
            let start = parse_bound(left)?;
            let end = parse_bound(right)?;
            Ok(SheetTarget::OpenRange(OpenRange { start, end }))
        } else {
            // Both sides are full cells — use the existing bounded parser
            parse_range_reference(&cleaned)
                .map(SheetTarget::Range)
                .map_err(|e| ParseSheetReferenceError::InvalidReference(e.to_string()))
        }
    } else {
        parse_cell_reference(&cleaned)
            .map(SheetTarget::Cell)
            .map_err(|e| ParseSheetReferenceError::InvalidReference(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // helper to avoid repetition
    fn sr(sheet: Option<&str>, target: SheetTarget) -> SheetReference {
        SheetReference {
            sheet: sheet.map(|s| s.to_owned()),
            target,
        }
    }

    fn cell(col: u32, row: u32) -> SheetTarget {
        SheetTarget::Cell(CellReference {
            location: (col, row),
        })
    }

    fn range(c1: u32, r1: u32, c2: u32, r2: u32) -> SheetTarget {
        SheetTarget::Range(RangeReference {
            start: (c1, r1),
            end: (c2, r2),
        })
    }

    fn open(start: Bound, end: Bound) -> SheetTarget {
        SheetTarget::OpenRange(OpenRange { start, end })
    }

    fn col(c: u32) -> Bound {
        Bound {
            col: Some(c),
            row: None,
        }
    }

    fn row(r: u32) -> Bound {
        Bound {
            col: None,
            row: Some(r),
        }
    }

    fn full(c: u32, r: u32) -> Bound {
        Bound {
            col: Some(c),
            row: Some(r),
        }
    }

    fn wildcard() -> Bound {
        Bound {
            col: None,
            row: None,
        }
    }

    // ═══════════════════════════════════════════
    //  Cell reference, no sheet
    // ═══════════════════════════════════════════

    #[test]
    fn bare_cell() {
        assert_eq!(parse_sheet_reference("A1"), Ok(sr(None, cell(0, 0))));
    }

    #[test]
    fn bare_cell_multi_col() {
        assert_eq!(parse_sheet_reference("AB10"), Ok(sr(None, cell(27, 9))));
    }

    // ═══════════════════════════════════════════
    //  Bounded range, no sheet
    // ═══════════════════════════════════════════

    #[test]
    fn bare_range() {
        assert_eq!(
            parse_sheet_reference("A1:B5"),
            Ok(sr(None, range(0, 0, 1, 4)))
        );
    }

    #[test]
    fn bare_range_large() {
        assert_eq!(
            parse_sheet_reference("A1:ZZ100"),
            Ok(sr(None, range(0, 0, 701, 99)))
        );
    }

    // ═══════════════════════════════════════════
    //  Cell reference with sheet
    // ═══════════════════════════════════════════

    #[test]
    fn sheet_cell() {
        assert_eq!(
            parse_sheet_reference("Sheet1!A1"),
            Ok(sr(Some("Sheet1"), cell(0, 0)))
        );
    }

    #[test]
    fn sheet_cell_complex_name() {
        assert_eq!(
            parse_sheet_reference("MyData2024!C5"),
            Ok(sr(Some("MyData2024"), cell(2, 4)))
        );
    }

    // ═══════════════════════════════════════════
    //  Bounded range with sheet
    // ═══════════════════════════════════════════

    #[test]
    fn sheet_range() {
        assert_eq!(
            parse_sheet_reference("Sheet2!A1:B100"),
            Ok(sr(Some("Sheet2"), range(0, 0, 1, 99)))
        );
    }

    #[test]
    fn sheet_range_large() {
        assert_eq!(
            parse_sheet_reference("Data!A1:ZZ9999"),
            Ok(sr(Some("Data"), range(0, 0, 701, 9998)))
        );
    }

    // ═══════════════════════════════════════════
    //  Quoted sheet names
    // ═══════════════════════════════════════════

    #[test]
    fn quoted_sheet_cell() {
        assert_eq!(
            parse_sheet_reference("'My Sheet'!A1"),
            Ok(sr(Some("My Sheet"), cell(0, 0)))
        );
    }

    #[test]
    fn quoted_sheet_range() {
        assert_eq!(
            parse_sheet_reference("'Table A'!A1:F50"),
            Ok(sr(Some("Table A"), range(0, 0, 5, 49)))
        );
    }

    #[test]
    fn quoted_sheet_with_numbers() {
        assert_eq!(
            parse_sheet_reference("'Sheet 2024'!B3"),
            Ok(sr(Some("Sheet 2024"), cell(1, 2)))
        );
    }

    // ═══════════════════════════════════════════
    //  Absolute references ($)
    // ═══════════════════════════════════════════

    #[test]
    fn absolute_cell() {
        assert_eq!(parse_sheet_reference("$A$1"), Ok(sr(None, cell(0, 0))));
    }

    #[test]
    fn absolute_range() {
        assert_eq!(
            parse_sheet_reference("$A$1:$B$5"),
            Ok(sr(None, range(0, 0, 1, 4)))
        );
    }

    #[test]
    fn sheet_absolute_range() {
        assert_eq!(
            parse_sheet_reference("Sheet1!$A$1:$B$5"),
            Ok(sr(Some("Sheet1"), range(0, 0, 1, 4)))
        );
    }

    // ═══════════════════════════════════════════
    //  Column-only ranges (A:B)
    // ═══════════════════════════════════════════

    #[test]
    fn col_range_ab() {
        assert_eq!(
            parse_sheet_reference("A:B"),
            Ok(sr(None, open(col(0), col(1))))
        );
    }

    #[test]
    fn col_range_single() {
        assert_eq!(
            parse_sheet_reference("A:A"),
            Ok(sr(None, open(col(0), col(0))))
        );
    }

    #[test]
    fn col_range_wide() {
        assert_eq!(
            parse_sheet_reference("A:ZZ"),
            Ok(sr(None, open(col(0), col(701))))
        );
    }

    #[test]
    fn sheet_col_range() {
        assert_eq!(
            parse_sheet_reference("Sheet1!A:D"),
            Ok(sr(Some("Sheet1"), open(col(0), col(3))))
        );
    }

    #[test]
    fn quoted_sheet_col_range() {
        assert_eq!(
            parse_sheet_reference("'My Data'!C:F"),
            Ok(sr(Some("My Data"), open(col(2), col(5))))
        );
    }

    // ═══════════════════════════════════════════
    //  Row-only ranges (1:10)
    // ═══════════════════════════════════════════

    #[test]
    fn row_range_1_10() {
        assert_eq!(
            parse_sheet_reference("1:10"),
            Ok(sr(None, open(row(0), row(9))))
        );
    }

    #[test]
    fn row_range_single() {
        assert_eq!(
            parse_sheet_reference("5:5"),
            Ok(sr(None, open(row(4), row(4))))
        );
    }

    #[test]
    fn row_range_large() {
        assert_eq!(
            parse_sheet_reference("1:99999"),
            Ok(sr(None, open(row(0), row(99998))))
        );
    }

    #[test]
    fn sheet_row_range() {
        assert_eq!(
            parse_sheet_reference("Sheet2!1:50"),
            Ok(sr(Some("Sheet2"), open(row(0), row(49))))
        );
    }

    // ═══════════════════════════════════════════
    //  Mixed ranges (A1:D, A:D5, 1:D5, A1:5)
    // ═══════════════════════════════════════════

    #[test]
    fn full_start_col_end() {
        // A1:D → start is full (A,0), end is col-only (D)
        assert_eq!(
            parse_sheet_reference("A1:D"),
            Ok(sr(None, open(full(0, 0), col(3))))
        );
    }

    #[test]
    fn col_start_full_end() {
        // A:D5 → start is col-only (A), end is full (D,4)
        assert_eq!(
            parse_sheet_reference("A:D5"),
            Ok(sr(None, open(col(0), full(3, 4))))
        );
    }

    #[test]
    fn row_start_full_end() {
        // 1:D5 → start is row-only (0), end is full (D,4)
        assert_eq!(
            parse_sheet_reference("1:D5"),
            Ok(sr(None, open(row(0), full(3, 4))))
        );
    }

    #[test]
    fn full_start_row_end() {
        // A1:5 → start is full (A,0), end is row-only (4)
        assert_eq!(
            parse_sheet_reference("A1:5"),
            Ok(sr(None, open(full(0, 0), row(4))))
        );
    }

    #[test]
    fn sheet_mixed_range() {
        assert_eq!(
            parse_sheet_reference("Sheet1!A1:D"),
            Ok(sr(Some("Sheet1"), open(full(0, 0), col(3))))
        );
    }

    #[test]
    fn sheet_mixed_range_absolute() {
        assert_eq!(
            parse_sheet_reference("Sheet1!$A$1:$D"),
            Ok(sr(Some("Sheet1"), open(full(0, 0), col(3))))
        );
    }

    #[test]
    fn col_wildcard_end() {
        // A1:D* → from A1 to column D, all rows
        assert_eq!(
            parse_sheet_reference("A1:D*"),
            Ok(sr(None, open(full(0, 0), col(3))))
        );
    }

    #[test]
    fn col_wildcard_end_with_row() {
        // A1:W* → from A1 to column W, all rows
        assert_eq!(
            parse_sheet_reference("A1:W*"),
            Ok(sr(None, open(full(0, 0), col(22))))
        );
    }

    #[test]
    fn sheet_col_wildcard() {
        assert_eq!(
            parse_sheet_reference("'Table A'!A13:W*"),
            Ok(sr(Some("Table A"), open(full(0, 12), col(22))))
        );
    }

    // ═══════════════════════════════════════════
    //  Whitespace handling
    // ═══════════════════════════════════════════

    #[test]
    fn whitespace_trimmed() {
        assert_eq!(
            parse_sheet_reference("  A1:B5  "),
            Ok(sr(None, range(0, 0, 1, 4)))
        );
    }

    #[test]
    fn whitespace_col_range() {
        assert_eq!(
            parse_sheet_reference("  A:B  "),
            Ok(sr(None, open(col(0), col(1))))
        );
    }

    // ═══════════════════════════════════════════
    //  Wildcard ranges (B5:*)
    // ═══════════════════════════════════════════

    #[test]
    fn cell_to_wildcard() {
        // B5:* → from B5 to end of sheet
        assert_eq!(
            parse_sheet_reference("B5:*"),
            Ok(sr(None, open(full(1, 4), wildcard())))
        );
    }

    #[test]
    fn a1_to_wildcard() {
        assert_eq!(
            parse_sheet_reference("A1:*"),
            Ok(sr(None, open(full(0, 0), wildcard())))
        );
    }

    #[test]
    fn wildcard_to_cell() {
        // *:D10 → from start of sheet to D10
        assert_eq!(
            parse_sheet_reference("*:D10"),
            Ok(sr(None, open(wildcard(), full(3, 9))))
        );
    }

    #[test]
    fn wildcard_to_wildcard() {
        // *:* → entire sheet
        assert_eq!(
            parse_sheet_reference("*:*"),
            Ok(sr(None, open(wildcard(), wildcard())))
        );
    }

    #[test]
    fn col_to_wildcard() {
        // A:* → from column A to end
        assert_eq!(
            parse_sheet_reference("A:*"),
            Ok(sr(None, open(col(0), wildcard())))
        );
    }

    #[test]
    fn row_to_wildcard() {
        // 5:* → from row 5 to end
        assert_eq!(
            parse_sheet_reference("5:*"),
            Ok(sr(None, open(row(4), wildcard())))
        );
    }

    #[test]
    fn sheet_cell_to_wildcard() {
        assert_eq!(
            parse_sheet_reference("Sheet1!B5:*"),
            Ok(sr(Some("Sheet1"), open(full(1, 4), wildcard())))
        );
    }

    #[test]
    fn quoted_sheet_cell_to_wildcard() {
        assert_eq!(
            parse_sheet_reference("'Table A'!C3:*"),
            Ok(sr(Some("Table A"), open(full(2, 2), wildcard())))
        );
    }

    #[test]
    fn absolute_cell_to_wildcard() {
        assert_eq!(
            parse_sheet_reference("$B$5:*"),
            Ok(sr(None, open(full(1, 4), wildcard())))
        );
    }

    // ═══════════════════════════════════════════
    //  Error cases
    // ═══════════════════════════════════════════

    #[test]
    fn empty_input() {
        assert_eq!(
            parse_sheet_reference(""),
            Err(ParseSheetReferenceError::Empty)
        );
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(
            parse_sheet_reference("   "),
            Err(ParseSheetReferenceError::Empty)
        );
    }

    #[test]
    fn invalid_reference() {
        assert!(parse_sheet_reference("!!!").is_err());
    }

    #[test]
    fn sheet_no_ref() {
        assert!(parse_sheet_reference("Sheet1!").is_err());
    }

    #[test]
    fn colon_only() {
        assert!(parse_sheet_reference(":").is_err());
    }

    #[test]
    fn colon_left_empty() {
        assert!(parse_sheet_reference(":B").is_err());
    }
}
