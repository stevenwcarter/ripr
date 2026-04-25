use crate::RipError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// The line number IS included.
    Inclusive(u64),
    /// The line number IS NOT included (exclusive / open boundary).
    Exclusive(u64),
    /// `$` — the last line of input.  Only valid in the `end` position.
    LastLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeSpec {
    pub start: Bound,
    pub end: Bound,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a `;`-separated list of range tokens into a `Vec<RangeSpec>`.
pub fn parse_ranges(input: &str) -> Result<Vec<RangeSpec>, RipError> {
    input
        .split(';')
        .map(|token| parse_one(token.trim()))
        .collect()
}

// ---------------------------------------------------------------------------
// Single-token parser
// ---------------------------------------------------------------------------

/// Parse a single range token such as `5`, `3-7`, `(3-7)`, `5-$`, etc.
fn parse_one(token: &str) -> Result<RangeSpec, RipError> {
    if token.is_empty() {
        return Err(RipError::Parse("empty range token".to_string()));
    }

    // Detect open-paren on start.
    let (start_excl, rest) = if let Some(r) = token.strip_prefix('(') {
        (true, r)
    } else {
        (false, token)
    };

    // Detect close-paren on end.
    let (end_excl, rest) = if let Some(r) = rest.strip_suffix(')') {
        // `rest` here is the pre-strip value; a lone ")" token becomes rest="" after strip,
        // but we check rest==")" before reassigning to catch this before the separator search.
        if !start_excl && rest == ")" {
            return Err(RipError::Parse(format!(
                "lone ')' without opening '(' in {:?}",
                token
            )));
        }
        (true, r)
    } else {
        (false, rest)
    };

    // Split on the separator: `-` or `,`.
    // We must be careful: a plain number has no separator, and a negative
    // number is not part of the grammar (all line numbers are positive).
    let sep_pos = find_separator(rest);

    let range = if let Some((sep_idx, _sep_char)) = sep_pos {
        // Two-part token: start <sep> end
        let lhs = &rest[..sep_idx];
        let rhs = &rest[sep_idx + 1..];

        let start = parse_start_bound(lhs, start_excl, token)?;
        let end = parse_end_bound(rhs, end_excl, token)?;

        // Validate: end must not be before start (for numeric bounds).
        if let (
            Bound::Inclusive(s) | Bound::Exclusive(s),
            Bound::Inclusive(e) | Bound::Exclusive(e),
        ) = (&start, &end)
            && e < s
        {
            return Err(RipError::Parse(format!(
                "end ({e}) is before start ({s}) in range {token:?}"
            )));
        }

        RangeSpec { start, end }
    } else {
        // Single number — no separator found.
        // A lone ')' token without a '(' was already caught above; but handle
        // the edge case where someone wrote just ")".
        if end_excl && !start_excl {
            // e.g. "5)" — close paren on a single number makes no sense
            return Err(RipError::Parse(format!(
                "lone ')' without opening '(' in {token:?}"
            )));
        }

        let n = parse_line_number(rest, token)?;
        let bound = if start_excl {
            Bound::Exclusive(n)
        } else {
            Bound::Inclusive(n)
        };
        // Single-number shorthand: start == end.
        RangeSpec {
            start: bound.clone(),
            end: bound,
        }
    };

    Ok(range)
}

/// Find the position and character of the first `-` or `,` separator.
/// Returns `None` if the string is a plain number.
fn find_separator(s: &str) -> Option<(usize, char)> {
    for (i, c) in s.char_indices() {
        if c == '-' || c == ',' {
            return Some((i, c));
        }
    }
    None
}

fn parse_start_bound(s: &str, exclusive: bool, token: &str) -> Result<Bound, RipError> {
    if s == "$" {
        return Err(RipError::Parse(format!(
            "'$' is not valid in the start position of range {token:?}"
        )));
    }
    let n = parse_line_number(s, token)?;
    Ok(if exclusive {
        Bound::Exclusive(n)
    } else {
        Bound::Inclusive(n)
    })
}

fn parse_end_bound(s: &str, exclusive: bool, token: &str) -> Result<Bound, RipError> {
    if s == "$" {
        if exclusive {
            return Err(RipError::Parse(format!(
                "'$)' is not valid — '$' cannot have an exclusive closing paren in {token:?}"
            )));
        }
        return Ok(Bound::LastLine);
    }
    let n = parse_line_number(s, token)?;
    Ok(if exclusive {
        Bound::Exclusive(n)
    } else {
        Bound::Inclusive(n)
    })
}

/// Parse a positive, non-zero line number from `s`.
fn parse_line_number(s: &str, token: &str) -> Result<u64, RipError> {
    let n: u64 = s.parse().map_err(|_| {
        RipError::Parse(format!(
            "expected a line number, got {s:?} in range {token:?}"
        ))
    })?;
    if n == 0 {
        return Err(RipError::Parse(format!(
            "line number 0 is not valid (ripr is 1-indexed) in range {token:?}"
        )));
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// contains()
// ---------------------------------------------------------------------------

impl RangeSpec {
    /// Returns `true` if the given 1-indexed `line` is within this range.
    ///
    /// `line_count` is the total number of lines, used to resolve `LastLine`.
    pub fn contains(&self, line: u64, line_count: u64) -> bool {
        let start_ok = match &self.start {
            Bound::Inclusive(s) => line >= *s,
            Bound::Exclusive(s) => line > *s,
            Bound::LastLine => {
                debug_assert!(
                    false,
                    "LastLine is invalid in start position — parser should have rejected this"
                );
                false
            }
        };
        let end_ok = match &self.end {
            Bound::Inclusive(e) => line <= *e,
            Bound::Exclusive(e) => line < *e,
            Bound::LastLine => line <= line_count,
        };
        start_ok && end_ok
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helper: parse exactly one token (no semicolons).
    // ------------------------------------------------------------------
    fn one(input: &str) -> Result<RangeSpec, RipError> {
        let v = parse_ranges(input)?;
        assert_eq!(v.len(), 1, "expected 1 range, got {}", v.len());
        Ok(v.into_iter().next().unwrap())
    }

    // Inclusive convenience constructor.
    fn inc(n: u64) -> Bound {
        Bound::Inclusive(n)
    }
    fn exc(n: u64) -> Bound {
        Bound::Exclusive(n)
    }

    // ------------------------------------------------------------------
    // Happy-path: all 14 syntax forms
    // ------------------------------------------------------------------

    #[test]
    fn single_line_number() {
        // N  →  Inclusive(N)..Inclusive(N)
        let r = one("7").unwrap();
        assert_eq!(r.start, inc(7));
        assert_eq!(r.end, inc(7));
    }

    #[test]
    fn inclusive_dash_range() {
        // N-M  →  Inclusive(N)..Inclusive(M)
        let r = one("3-9").unwrap();
        assert_eq!(r.start, inc(3));
        assert_eq!(r.end, inc(9));
    }

    #[test]
    fn inclusive_comma_range() {
        // N,M  →  Inclusive(N)..Inclusive(M)
        let r = one("3,9").unwrap();
        assert_eq!(r.start, inc(3));
        assert_eq!(r.end, inc(9));
    }

    #[test]
    fn start_exclusive_dash() {
        // (N-M  →  Exclusive(N)..Inclusive(M)
        let r = one("(3-9").unwrap();
        assert_eq!(r.start, exc(3));
        assert_eq!(r.end, inc(9));
    }

    #[test]
    fn end_exclusive_dash() {
        // N-M)  →  Inclusive(N)..Exclusive(M)
        let r = one("3-9)").unwrap();
        assert_eq!(r.start, inc(3));
        assert_eq!(r.end, exc(9));
    }

    #[test]
    fn both_exclusive_dash() {
        // (N-M)  →  Exclusive(N)..Exclusive(M)
        let r = one("(3-9)").unwrap();
        assert_eq!(r.start, exc(3));
        assert_eq!(r.end, exc(9));
    }

    #[test]
    fn start_exclusive_comma() {
        // (N,M  →  Exclusive(N)..Inclusive(M)
        let r = one("(3,9").unwrap();
        assert_eq!(r.start, exc(3));
        assert_eq!(r.end, inc(9));
    }

    #[test]
    fn end_exclusive_comma() {
        // N,M)  →  Inclusive(N)..Exclusive(M)
        let r = one("3,9)").unwrap();
        assert_eq!(r.start, inc(3));
        assert_eq!(r.end, exc(9));
    }

    #[test]
    fn both_exclusive_comma() {
        // (N,M)  →  Exclusive(N)..Exclusive(M)
        let r = one("(3,9)").unwrap();
        assert_eq!(r.start, exc(3));
        assert_eq!(r.end, exc(9));
    }

    #[test]
    fn last_line_dash() {
        // N-$  →  Inclusive(N)..LastLine
        let r = one("5-$").unwrap();
        assert_eq!(r.start, inc(5));
        assert_eq!(r.end, Bound::LastLine);
    }

    #[test]
    fn last_line_start_exclusive_dash() {
        // (N-$  →  Exclusive(N)..LastLine
        let r = one("(5-$").unwrap();
        assert_eq!(r.start, exc(5));
        assert_eq!(r.end, Bound::LastLine);
    }

    #[test]
    fn last_line_comma() {
        // N,$  →  Inclusive(N)..LastLine
        let r = one("5,$").unwrap();
        assert_eq!(r.start, inc(5));
        assert_eq!(r.end, Bound::LastLine);
    }

    #[test]
    fn last_line_start_exclusive_comma() {
        // (N,$  →  Exclusive(N)..LastLine
        let r = one("(5,$").unwrap();
        assert_eq!(r.start, exc(5));
        assert_eq!(r.end, Bound::LastLine);
    }

    // ------------------------------------------------------------------
    // Multi-range via semicolon
    // ------------------------------------------------------------------

    #[test]
    fn multi_range_semicolon() {
        // "5-10;20-25"  →  two RangeSpecs
        let v = parse_ranges("5-10;20-25").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(
            v[0],
            RangeSpec {
                start: inc(5),
                end: inc(10)
            }
        );
        assert_eq!(
            v[1],
            RangeSpec {
                start: inc(20),
                end: inc(25)
            }
        );
    }

    #[test]
    fn multi_range_three() {
        let v = parse_ranges("1;5-10;20,$").unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(
            v[0],
            RangeSpec {
                start: inc(1),
                end: inc(1)
            }
        );
        assert_eq!(
            v[1],
            RangeSpec {
                start: inc(5),
                end: inc(10)
            }
        );
        assert_eq!(
            v[2],
            RangeSpec {
                start: inc(20),
                end: Bound::LastLine
            }
        );
    }

    // ------------------------------------------------------------------
    // contains() — inclusive range
    // ------------------------------------------------------------------

    #[test]
    fn contains_inclusive_inside() {
        let r = one("3-7").unwrap();
        assert!(r.contains(3, 100));
        assert!(r.contains(5, 100));
        assert!(r.contains(7, 100));
    }

    #[test]
    fn contains_inclusive_outside() {
        let r = one("3-7").unwrap();
        assert!(!r.contains(2, 100));
        assert!(!r.contains(8, 100));
    }

    #[test]
    fn contains_inclusive_boundary() {
        let r = one("3-7").unwrap();
        // Exact boundaries ARE included.
        assert!(r.contains(3, 100));
        assert!(r.contains(7, 100));
    }

    // ------------------------------------------------------------------
    // contains() — exclusive boundaries
    // ------------------------------------------------------------------

    #[test]
    fn contains_start_exclusive() {
        // (3-7  → lines 4,5,6,7 included; line 3 not
        let r = one("(3-7").unwrap();
        assert!(!r.contains(3, 100));
        assert!(r.contains(4, 100));
        assert!(r.contains(7, 100));
    }

    #[test]
    fn contains_end_exclusive() {
        // 3-7)  → lines 3,4,5,6 included; line 7 not
        let r = one("3-7)").unwrap();
        assert!(r.contains(3, 100));
        assert!(r.contains(6, 100));
        assert!(!r.contains(7, 100));
    }

    #[test]
    fn contains_both_exclusive() {
        // (3-7)  → lines 4,5,6 included; 3 and 7 not
        let r = one("(3-7)").unwrap();
        assert!(!r.contains(3, 100));
        assert!(r.contains(4, 100));
        assert!(r.contains(6, 100));
        assert!(!r.contains(7, 100));
    }

    // ------------------------------------------------------------------
    // contains() — LastLine resolution
    // ------------------------------------------------------------------

    #[test]
    fn contains_last_line() {
        // 5-$  with 10 lines → lines 5..=10
        let r = one("5-$").unwrap();
        assert!(!r.contains(4, 10));
        assert!(r.contains(5, 10));
        assert!(r.contains(10, 10));
        assert!(!r.contains(11, 10));
    }

    #[test]
    fn contains_last_line_exclusive_start() {
        // (5-$  with 10 lines → lines 6..=10
        let r = one("(5-$").unwrap();
        assert!(!r.contains(5, 10));
        assert!(r.contains(6, 10));
        assert!(r.contains(10, 10));
    }

    // ------------------------------------------------------------------
    // Single-line contains
    // ------------------------------------------------------------------

    #[test]
    fn contains_single_line() {
        let r = one("7").unwrap();
        assert!(!r.contains(6, 100));
        assert!(r.contains(7, 100));
        assert!(!r.contains(8, 100));
    }

    // ------------------------------------------------------------------
    // Error cases
    // ------------------------------------------------------------------

    #[test]
    fn error_zero_line_number() {
        let err = one("0").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        assert!(err.to_string().contains("0"));
    }

    #[test]
    fn error_zero_in_range() {
        let err = one("0-5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_non_numeric_start() {
        let err = one("abc-5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_non_numeric_end() {
        let err = one("5-abc").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_non_numeric_single() {
        let err = one("abc").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_dollar_as_start() {
        let err = one("$-5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        assert!(err.to_string().contains("start"));
    }

    #[test]
    fn error_dollar_comma_as_start() {
        let err = one("$,5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_end_before_start() {
        let err = one("10-5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        assert!(err.to_string().contains("before start"));
    }

    #[test]
    fn error_end_before_start_comma() {
        let err = one("10,5").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_lone_close_paren() {
        let err = one("5)").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        assert!(err.to_string().contains("'('"));
    }

    #[test]
    fn error_empty_token() {
        let err = one("").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_empty_token_in_multi() {
        // A trailing semicolon produces an empty token.
        let err = parse_ranges("5-10;").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    // ------------------------------------------------------------------
    // Edge: equal start and end (N-N) should be valid
    // ------------------------------------------------------------------

    #[test]
    fn equal_start_and_end() {
        let r = one("5-5").unwrap();
        assert_eq!(r.start, inc(5));
        assert_eq!(r.end, inc(5));
        assert!(r.contains(5, 10));
    }

    // ------------------------------------------------------------------
    // Single-exclusive form (N) — always-empty range
    // ------------------------------------------------------------------

    #[test]
    fn single_exclusive_always_empty() {
        // "(N)" produces Exclusive(N)..Exclusive(N) — a valid but always-empty range.
        // Users who write this almost certainly made a mistake; document behavior.
        let specs = parse_ranges("(5)").expect("parses without error");
        assert_eq!(specs.len(), 1);
        let s = &specs[0];
        assert!(!s.contains(4, 100));
        assert!(!s.contains(5, 100)); // exclusive on both sides
        assert!(!s.contains(6, 100));
    }

    // ------------------------------------------------------------------
    // Whitespace trimming in multi-range
    // ------------------------------------------------------------------

    #[test]
    fn multi_range_with_spaces() {
        let v = parse_ranges("1-3 ; 7-9").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(
            v[0],
            RangeSpec {
                start: inc(1),
                end: inc(3)
            }
        );
        assert_eq!(
            v[1],
            RangeSpec {
                start: inc(7),
                end: inc(9)
            }
        );
    }

    #[test]
    fn error_dollar_end_exclusive() {
        // "$" in end position cannot be exclusive — $ is already "last line" inclusive
        assert!(parse_ranges("5-$)").is_err());
    }
}
