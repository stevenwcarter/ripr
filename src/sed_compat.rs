use crate::RipError;
use crate::range::{Bound, RangeSpec};

/// Parse a sed `-n` expression string into `Vec<RangeSpec>`.
///
/// Handles multi-expression via `";"` (splits and parses each part).
///
/// Accepted forms:
///   `Np`     → line N only
///   `$p`     → last line only
///   `N,Mp`   → lines N through M (inclusive)
///   `N,$p`   → lines N through end
///
/// Rejects: `/pattern/p`, `!p`, `n~m`, `s///`, and anything else.
pub fn parse_sed_n(expr: &str) -> Result<Vec<RangeSpec>, RipError> {
    if expr.trim().is_empty() {
        return Err(RipError::Parse(
            "empty sed expression; use 'ripr <range> <file>' for native syntax".to_string(),
        ));
    }

    expr.split(';')
        .map(|part| parse_sed_expr(part.trim()))
        .collect()
}

fn parse_sed_expr(expr: &str) -> Result<RangeSpec, RipError> {
    if expr.is_empty() {
        return Err(RipError::Parse(
            "empty sed expression; use 'ripr <range> <file>' for native syntax".to_string(),
        ));
    }

    // All sed print expressions must end with 'p'.
    let addr = expr.strip_suffix('p').ok_or_else(|| {
        RipError::Parse(format!(
            "sed expression must end with 'p' (e.g. '5,10p'): got '{expr}'\n\
             Use 'ripr <range> <file>' for native syntax"
        ))
    })?;

    // Reject unsupported sed forms before attempting to parse as addresses.
    // These checks look at the address portion (after stripping 'p').
    if addr.contains('/') {
        return Err(RipError::Parse(format!(
            "unsupported sed expression '{expr}': /pattern/ addresses are not supported\n\
             Use 'ripr <range> <file>' for native syntax"
        )));
    }
    if addr.contains('!') {
        return Err(RipError::Parse(format!(
            "unsupported sed expression '{expr}': '!' (invert) is not supported\n\
             Use 'ripr <range> <file>' for native syntax"
        )));
    }
    if addr.contains('~') {
        return Err(RipError::Parse(format!(
            "unsupported sed expression '{expr}': 'first~step' addresses are not supported\n\
             Use 'ripr <range> <file>' for native syntax"
        )));
    }

    // Parse the address part.
    if addr == "$" {
        // $p — last line only
        return Ok(RangeSpec {
            start: Bound::LastLine,
            end: Bound::LastLine,
        });
    }

    if let Some(comma_pos) = addr.find(',') {
        // Two-part address: N,M or N,$
        let lhs = &addr[..comma_pos];
        let rhs = &addr[comma_pos + 1..];

        if lhs == "$" {
            return Err(RipError::Parse(
                "'$' cannot be used as a start address in sed mode".to_string(),
            ));
        }

        let start_n = parse_sed_line_number(lhs, expr)?;
        let start = Bound::Inclusive(start_n);

        let end = if rhs == "$" {
            Bound::LastLine
        } else {
            let end_n = parse_sed_line_number(rhs, expr)?;
            if start_n > end_n {
                return Err(RipError::Parse(format!(
                    "end line {end_n} is before start line {start_n} in sed expression '{expr}'"
                )));
            }
            Bound::Inclusive(end_n)
        };

        Ok(RangeSpec { start, end })
    } else {
        // Single address: N
        let n = parse_sed_line_number(addr, expr)?;
        Ok(RangeSpec {
            start: Bound::Inclusive(n),
            end: Bound::Inclusive(n),
        })
    }
}

fn parse_sed_line_number(s: &str, expr: &str) -> Result<u64, RipError> {
    // If it's not purely digits, reject as unsupported.
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(RipError::Parse(format!(
            "unsupported sed expression '{expr}': expected a line number, got '{s}'\n\
             Use 'ripr <range> <file>' for native syntax"
        )));
    }

    let n: u64 = s.parse().map_err(|_| {
        RipError::Parse(format!(
            "unsupported sed expression '{expr}': expected a line number, got '{s}'\n\
             Use 'ripr <range> <file>' for native syntax"
        ))
    })?;

    if n == 0 {
        return Err(RipError::Parse(
            "line numbers are 1-indexed; 0 is not a valid line number".to_string(),
        ));
    }

    Ok(n)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn inc(n: u64) -> Bound {
        Bound::Inclusive(n)
    }

    fn spec(start: Bound, end: Bound) -> RangeSpec {
        RangeSpec { start, end }
    }

    // ------------------------------------------------------------------
    // Happy-path
    // ------------------------------------------------------------------

    #[test]
    fn single_line() {
        // 5p → Inclusive(5)..Inclusive(5)
        let v = parse_sed_n("5p").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], spec(inc(5), inc(5)));
    }

    #[test]
    fn last_line_only() {
        // $p → LastLine..LastLine
        let v = parse_sed_n("$p").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], spec(Bound::LastLine, Bound::LastLine));
    }

    #[test]
    fn inclusive_range() {
        // 1,10p → Inclusive(1)..Inclusive(10)
        let v = parse_sed_n("1,10p").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], spec(inc(1), inc(10)));
    }

    #[test]
    fn range_to_last_line() {
        // 5,$p → Inclusive(5)..LastLine
        let v = parse_sed_n("5,$p").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], spec(inc(5), Bound::LastLine));
    }

    #[test]
    fn range_from_one_to_last_line() {
        // 1,$p → Inclusive(1)..LastLine
        let v = parse_sed_n("1,$p").unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], spec(inc(1), Bound::LastLine));
    }

    #[test]
    fn multi_expression() {
        // 5p;20,25p → two RangeSpecs
        let v = parse_sed_n("5p;20,25p").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], spec(inc(5), inc(5)));
        assert_eq!(v[1], spec(inc(20), inc(25)));
    }

    #[test]
    fn multi_expression_with_dollar_end() {
        // 5,$p;10p → two RangeSpecs
        let v = parse_sed_n("5,$p;10p").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], spec(inc(5), Bound::LastLine));
        assert_eq!(v[1], spec(inc(10), inc(10)));
    }

    #[test]
    fn whitespace_trimmed_around_semicolons() {
        let v = parse_sed_n("5p ; 10p").unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], spec(inc(5), inc(5)));
        assert_eq!(v[1], spec(inc(10), inc(10)));
    }

    // ------------------------------------------------------------------
    // Error cases
    // ------------------------------------------------------------------

    #[test]
    fn error_zero_line() {
        let err = parse_sed_n("0p").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(msg.contains("1-indexed") || msg.contains("0"), "{msg}");
    }

    #[test]
    fn error_dollar_as_start() {
        let err = parse_sed_n("$,5p").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(msg.contains("start"), "{msg}");
    }

    #[test]
    fn error_missing_trailing_p() {
        let err = parse_sed_n("5,10").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(msg.contains("end with 'p'"), "{msg}");
    }

    #[test]
    fn error_regex_pattern() {
        let err = parse_sed_n("/regex/p").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("not supported") || msg.contains("unsupported"),
            "{msg}"
        );
    }

    #[test]
    fn error_invert() {
        let err = parse_sed_n("5!p").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("not supported") || msg.contains("unsupported"),
            "{msg}"
        );
    }

    #[test]
    fn error_step_address() {
        let err = parse_sed_n("1~2p").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("not supported") || msg.contains("unsupported"),
            "{msg}"
        );
    }

    #[test]
    fn error_empty_expression() {
        let err = parse_sed_n("").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_empty_part_in_multi() {
        // trailing semicolon produces an empty part
        let err = parse_sed_n("5p;").unwrap_err();
        assert!(matches!(err, RipError::Parse(_)));
    }

    #[test]
    fn error_end_before_start() {
        assert!(parse_sed_n("10,5p").is_err());
    }
}
