//! The tables `Text.lower` needs, taken from this toolchain's own `str::to_lowercase`, so a build lowers
//! exactly as `spec-lang test` does.

use std::fmt::Write;

fn ranges(holds: impl Fn(char) -> bool) -> String {
    let mut out = String::new();
    let mut open: Option<(u32, u32)> = None;
    for c in (0..=0x10_FFFFu32).filter_map(char::from_u32) {
        let n = c as u32;
        if holds(c) {
            open = match open {
                Some((a, b)) if b + 1 == n => Some((a, n)),
                Some((a, b)) => {
                    let _ = writeln!(out, "    {{0x{a:X}, 0x{b:X}}},");
                    Some((n, n))
                }
                None => Some((n, n)),
            };
        }
    }
    if let Some((a, b)) = open {
        let _ = writeln!(out, "    {{0x{a:X}, 0x{b:X}}},");
    }
    out
}

/// `src/rt/lower.cpp`: the template with its three tables filled in.
pub fn lower_source(template: &str) -> String {
    // A capital sigma lowers to its final form after a cased letter: what the standard library does with
    // `cΣ` and `AcΣ` says whether `c` is cased and whether it is case-ignorable.
    let final_after = |s: String| s.to_lowercase().ends_with('ς');
    let cased = |c: char| final_after(format!("{c}Σ"));
    let ignorable = |c: char| final_after(format!("A{c}Σ")) && !cased(c);
    let mut lowered = String::new();
    for c in (0..=0x10_FFFFu32).filter_map(char::from_u32) {
        let to: Vec<u32> = c.to_lowercase().map(|x| x as u32).collect();
        if to != [c as u32] && c != 'Σ' {
            let mut cells = [0u32; 3];
            for (cell, x) in cells.iter_mut().zip(&to) {
                *cell = *x;
            }
            let _ = writeln!(
                lowered,
                "    {{0x{:X}, {{0x{:X}, 0x{:X}, 0x{:X}}}}},",
                c as u32, cells[0], cells[1], cells[2]
            );
        }
    }
    template
        .replace("@LOWERED@", &lowered)
        .replace("@CASED@", &ranges(cased))
        .replace("@IGNORABLE@", &ranges(ignorable))
}
