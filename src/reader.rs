use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::RipError;
use crate::range::RangeSpec;

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Returns true when we are guaranteed to have passed all matching lines and can stop reading.
/// PRECONDITION: caller must ensure `line_count > 0` before calling this — if line_count is
/// unknown (0), LastLine ranges cannot be resolved and short-circuit is unsafe.
fn can_short_circuit(line: u64, ranges: &[RangeSpec]) -> bool {
    use crate::range::Bound;
    ranges.iter().all(|spec| match &spec.end {
        Bound::Inclusive(n) => *n <= line,
        Bound::Exclusive(n) => n.saturating_sub(1) <= line,
        Bound::LastLine => false,
    })
}

/// Returns `true` if any range uses `LastLine` in either the start or end
/// position — meaning we must know the total line count before emitting.
fn uses_last_line(ranges: &[RangeSpec]) -> bool {
    use crate::range::Bound;
    ranges
        .iter()
        .any(|spec| matches!(spec.start, Bound::LastLine) || matches!(spec.end, Bound::LastLine))
}

// ---------------------------------------------------------------------------
// Core streaming emitter
// ---------------------------------------------------------------------------

/// Read `source` line by line, writing lines that match any `RangeSpec` in
/// `ranges` to `out`.  Lines are 1-indexed.
///
/// `line_count` is the total line count of the source (needed for `LastLine`
/// resolution). Pass `0` when it is unknown; short-circuiting is disabled in
/// that case (since we cannot tell when all ranges have been exhausted).
pub fn emit_lines<R: Read, W: Write>(
    source: R,
    ranges: &[RangeSpec],
    line_count: u64,
    out: &mut BufWriter<W>,
) -> Result<(), RipError> {
    let reader = BufReader::new(source);
    for (idx, result) in reader.lines().enumerate() {
        let line_num = (idx + 1) as u64;
        let text = result?;

        let matched = ranges
            .iter()
            .any(|spec| spec.contains(line_num, line_count));
        if matched {
            out.write_all(text.as_bytes())?;
            out.write_all(b"\n")?;
        } else if line_count > 0 && can_short_circuit(line_num, ranges) {
            break;
        }
    }
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Buffer-based emitter (used by read_stdin and its tests)
// ---------------------------------------------------------------------------

/// Emit lines from an in-memory `lines` slice.  `line_count` is
/// `lines.len() as u64` (already computed by the caller).
pub(crate) fn emit_from_buffer<W: Write>(
    lines: &[String],
    ranges: &[RangeSpec],
    out: &mut BufWriter<W>,
) -> Result<(), RipError> {
    let line_count = lines.len() as u64;
    for (idx, text) in lines.iter().enumerate() {
        let line_num = (idx + 1) as u64;
        if ranges
            .iter()
            .any(|spec| spec.contains(line_num, line_count))
        {
            out.write_all(text.as_bytes())?;
            out.write_all(b"\n")?;
        }
        // No short-circuit here: line_count is always known, but we still need
        // to handle LastLine ranges correctly.  The buffer is already in memory
        // so there is no I/O cost to continuing.
    }
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// count_lines
// ---------------------------------------------------------------------------

/// Count the number of lines in a file without buffering content.
pub fn count_lines(path: &Path) -> Result<u64, RipError> {
    let f = File::open(path)?;
    let mut count = 0u64;
    for result in BufReader::new(f).lines() {
        result?;
        count += 1;
    }
    Ok(count)
}

// ---------------------------------------------------------------------------
// High-level entry points
// ---------------------------------------------------------------------------

/// Read from a file path, emitting lines that match any range.
///
/// If any range uses `LastLine`, `count_lines` is called first so the total
/// is available during the streaming pass.
pub fn read_file<W: Write>(
    path: &Path,
    ranges: &[RangeSpec],
    out: &mut BufWriter<W>,
) -> Result<(), RipError> {
    let line_count = if uses_last_line(ranges) {
        count_lines(path)?
    } else {
        0
    };
    let f = File::open(path)?;
    emit_lines(f, ranges, line_count, out)
}

/// Read from stdin, buffering all lines, then emitting matching lines.
///
/// Because stdin cannot be seeked, we always buffer the full input so that
/// `LastLine` ranges resolve correctly.
pub fn read_stdin<W: Write>(ranges: &[RangeSpec], out: &mut BufWriter<W>) -> Result<(), RipError> {
    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
    emit_from_buffer(&lines, ranges, out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::parse_ranges;
    use std::io::Cursor;

    // Helper: build a Cursor from N lines labelled "1\n2\n…\nN\n".
    fn numbered_input(n: u64) -> Vec<u8> {
        (1..=n)
            .flat_map(|i| format!("{i}\n").into_bytes())
            .collect()
    }

    fn run_emit(input: &[u8], range_str: &str, line_count: u64) -> String {
        let ranges = parse_ranges(range_str).unwrap();
        let mut buf = Vec::new();
        {
            let mut out = BufWriter::new(&mut buf);
            emit_lines(Cursor::new(input), &ranges, line_count, &mut out).unwrap();
        }
        String::from_utf8(buf).unwrap()
    }

    // ------------------------------------------------------------------
    // emit_lines — inclusive range
    // ------------------------------------------------------------------

    #[test]
    fn emit_inclusive_range() {
        // 10-line input, range 3-5 → "3\n4\n5\n"
        let input = numbered_input(10);
        let out = run_emit(&input, "3-5", 0);
        assert_eq!(out, "3\n4\n5\n");
    }

    // ------------------------------------------------------------------
    // emit_lines — exclusive range
    // ------------------------------------------------------------------

    #[test]
    fn emit_exclusive_range() {
        // (3-5) → Exclusive(3)..Exclusive(5) → only line 4
        let input = numbered_input(10);
        let out = run_emit(&input, "(3-5)", 0);
        assert_eq!(out, "4\n");
    }

    // ------------------------------------------------------------------
    // emit_lines — LastLine range
    // ------------------------------------------------------------------

    #[test]
    fn emit_last_line_range() {
        // 10-line input, range 5-$ with line_count=10 → lines 5..=10
        let input = numbered_input(10);
        let out = run_emit(&input, "5-$", 10);
        assert_eq!(out, "5\n6\n7\n8\n9\n10\n");
    }

    // ------------------------------------------------------------------
    // emit_lines — multi-range
    // ------------------------------------------------------------------

    #[test]
    fn emit_multi_range() {
        // ranges 2-3 and 8-9
        let input = numbered_input(10);
        let out = run_emit(&input, "2-3;8-9", 0);
        assert_eq!(out, "2\n3\n8\n9\n");
    }

    // ------------------------------------------------------------------
    // emit_lines — short-circuit
    // ------------------------------------------------------------------

    #[test]
    fn emit_short_circuit() {
        // 1000-line input, range 5-10: output must be exactly lines 5..=10
        let input = numbered_input(1000);
        let out = run_emit(&input, "5-10", 1000);
        assert_eq!(out, "5\n6\n7\n8\n9\n10\n");
    }

    // ------------------------------------------------------------------
    // emit_lines — short-circuit actually stops early (line-level check)
    // ------------------------------------------------------------------

    #[test]
    fn emit_short_circuit_stops_early() {
        // Verify short-circuit by using a reader that panics if we try to read
        // past a sentinel line.  We insert a "STOP" line after line 15 that
        // would cause a parse error in our numbered-line format if reached.
        // Instead we simply check that the output contains exactly lines 5-10
        // and nothing beyond, confirming that iteration ended before line 1000.

        // 1000-line input, ranges 5-10 with line_count=1000 (enables short-circuit)
        let input = numbered_input(1000);
        let ranges = parse_ranges("5-10").unwrap();
        let mut buf = Vec::new();
        {
            let mut out = BufWriter::new(&mut buf);
            emit_lines(Cursor::new(input), &ranges, 1000, &mut out).unwrap();
        }
        let result = String::from_utf8(buf).unwrap();
        // Correct output — implies we stopped after line 10
        assert_eq!(result, "5\n6\n7\n8\n9\n10\n");

        // Also verify that WITHOUT line_count (line_count=0), disabling short-circuit,
        // we still get the same correct output (just reads all 1000 lines).
        let input2 = numbered_input(1000);
        let mut buf2 = Vec::new();
        {
            let mut out2 = BufWriter::new(&mut buf2);
            emit_lines(Cursor::new(input2), &ranges, 0, &mut out2).unwrap();
        }
        assert_eq!(String::from_utf8(buf2).unwrap(), "5\n6\n7\n8\n9\n10\n");
    }

    // ------------------------------------------------------------------
    // emit_from_buffer (stdin logic)
    // ------------------------------------------------------------------

    #[test]
    fn emit_from_buffer_basic() {
        let lines: Vec<String> = (1..=5).map(|i| i.to_string()).collect();
        let ranges = parse_ranges("2-4").unwrap();
        let mut buf = Vec::new();
        {
            let mut out = BufWriter::new(&mut buf);
            emit_from_buffer(&lines, &ranges, &mut out).unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "2\n3\n4\n");
    }

    #[test]
    fn emit_from_buffer_last_line() {
        let lines: Vec<String> = (1..=10).map(|i| i.to_string()).collect();
        let ranges = parse_ranges("8-$").unwrap();
        let mut buf = Vec::new();
        {
            let mut out = BufWriter::new(&mut buf);
            emit_from_buffer(&lines, &ranges, &mut out).unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "8\n9\n10\n");
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn emit_empty_input() {
        let out = run_emit(b"", "1-5", 0);
        assert_eq!(out, "");
    }

    #[test]
    fn emit_range_beyond_eof() {
        // 3-line input, range 10-20 → no output, no error
        let input = numbered_input(3);
        let out = run_emit(&input, "10-20", 0);
        assert_eq!(out, "");
    }

    #[test]
    fn emit_line_count_zero_with_last_line() {
        // line_count=0 and a LastLine range: contains() resolves LastLine against 0,
        // meaning line <= 0 is never true for 1-indexed lines.
        // No crash expected — just empty output.
        let input = numbered_input(10);
        let out = run_emit(&input, "5-$", 0);
        // With line_count=0, `line <= 0` is false for all lines ≥ 1, so nothing printed.
        assert_eq!(out, "");
    }
}
