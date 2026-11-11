#![allow(dead_code)]
// t12: a text-level peephole optimiser over AArch64-flavoured assembly.
// The word-boundary helpers and the LineStore are lifted verbatim from ccc
// src/backend/peephole_common.rs. The passes (store/load forwarding,
// redundant move removal, branch-to-next elimination, dead-label removal,
// copy propagation via replace_source_reg_in_instruction) are new code in
// the same style, iterated to a fixpoint with per-pass statistics.
use std::collections::{BTreeMap, HashSet};

// ======== lifted from ccc src/backend/peephole_common.rs ========
/// Returns `true` if `b` is a "word character" for register/symbol matching:
/// ASCII alphanumeric, `.`, or `_`.
#[inline]
pub(crate) fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'.' || b == b'_'
}

/// Replace every whole-word occurrence of `old` with `new` in `text`.
///
/// A word boundary is a position where the adjacent character is not an
/// identifier char.  This prevents "x1" from matching inside "x11" or
/// "main.x1.0".
pub(crate) fn replace_whole_word(text: &str, old: &str, new: &str) -> String {
    let bytes = text.as_bytes();
    let old_bytes = old.as_bytes();
    let old_len = old_bytes.len();
    let text_len = bytes.len();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < text_len {
        if i + old_len <= text_len && &bytes[i..i + old_len] == old_bytes {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after_ok = i + old_len >= text_len || !is_ident_char(bytes[i + old_len]);
            if before_ok && after_ok {
                result.push_str(new);
                i += old_len;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

/// Returns `true` if `text` contains `word` at a word boundary.
pub(crate) fn has_whole_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let word_bytes = word.as_bytes();
    let word_len = word_bytes.len();
    let text_len = bytes.len();

    let mut i = 0;
    while i + word_len <= text_len {
        if &bytes[i..i + word_len] == word_bytes {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after_ok = i + word_len >= text_len || !is_ident_char(bytes[i + word_len]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Replace a register name in the *source* operand positions of an instruction.
///
/// Given an assembly line like `"  add x0, x1, x2"`, this replaces `old_reg`
/// with `new_reg` only in the operands *after* the first comma (i.e. the
/// source operands, not the destination).  Returns `None` if no replacement
/// was made.  Preserves leading whitespace.
pub(crate) fn replace_source_reg_in_instruction(
    line: &str,
    old_reg: &str,
    new_reg: &str,
) -> Option<String> {
    let trimmed = line.trim();

    // Find the first space to separate mnemonic from operands
    let space_pos = trimmed.find(' ')?;
    let args_start = space_pos + 1;
    let args = &trimmed[args_start..];

    // Find first comma -- everything after it is source operands
    let comma_pos = args.find(',')?;
    let after_first_arg = &args[comma_pos..];

    // Only replace in the source part (after the first comma)
    let new_suffix = replace_whole_word(after_first_arg, old_reg, new_reg);
    if new_suffix == after_first_arg {
        return None;
    }

    // Build the new line
    let prefix = &trimmed[..args_start + comma_pos];
    let new_trimmed = format!("{}{}", prefix, new_suffix);

    // Preserve leading whitespace
    let leading = line.len() - line.trim_start().len();
    let leading_ws = &line[..leading];
    Some(format!("{}{}", leading_ws, new_trimmed))
}

// ── LineStore: compact assembly line storage ─────────────────────────────
//
// During peephole optimization we repeatedly access and occasionally replace
// individual assembly lines.  Storing each line as its own `String` is
// wasteful (24 bytes overhead per line on 64-bit).  `LineStore` keeps the
// original text as a single `String` and records byte-offset ranges for each
// line.  Replaced lines go into a small side buffer.  This typically reduces
// memory traffic significantly for large functions.

/// Compact (start, len) entry for one line.  8 bytes vs. 24 for String.
/// When `len == u32::MAX`, the line has been replaced and `start` is the
/// index into the `replacements` vector.
#[derive(Clone, Copy)]
pub(crate) struct LineEntry {
    start: u32,
    len: u32,
}

/// Compact storage for assembly lines that avoids per-line allocation.
pub(crate) struct LineStore {
    /// The original assembly text (kept alive for the duration of optimization).
    original: String,
    /// One entry per line.
    entries: Vec<LineEntry>,
    /// Side buffer for lines that have been replaced by optimization passes.
    replacements: Vec<String>,
}

impl LineStore {
    /// Build a `LineStore` from an assembly string.
    pub(crate) fn new(asm: String) -> Self {
        let bytes = asm.as_bytes();
        let total_len = bytes.len();
        let estimated_lines = total_len / 20 + 1;
        let mut entries = Vec::with_capacity(estimated_lines);

        let mut start = 0usize;
        let mut i = 0;
        while i < total_len {
            if bytes[i] == b'\n' {
                entries.push(LineEntry {
                    start: start as u32,
                    len: (i - start) as u32,
                });
                start = i + 1;
            }
            i += 1;
        }
        // Handle last line (no trailing newline)
        if start <= total_len {
            let remaining = total_len - start;
            if remaining > 0 || entries.is_empty() {
                entries.push(LineEntry {
                    start: start as u32,
                    len: remaining as u32,
                });
            }
        }

        LineStore {
            original: asm,
            entries,
            replacements: Vec::new(),
        }
    }

    /// Get the text of line `idx`.
    #[inline]
    pub(crate) fn get(&self, idx: usize) -> &str {
        let e = &self.entries[idx];
        if e.len == u32::MAX {
            &self.replacements[e.start as usize]
        } else {
            let start = e.start as usize;
            let end = start + e.len as usize;
            &self.original[start..end]
        }
    }

    /// Number of lines.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Replace a line with new text.
    pub(crate) fn replace(&mut self, idx: usize, new_text: String) {
        let rep_idx = self.replacements.len();
        self.replacements.push(new_text);
        self.entries[idx] = LineEntry {
            start: rep_idx as u32,
            len: u32::MAX,
        };
    }

    /// Build the final output string, skipping lines where `skip(i)` returns true.
    pub(crate) fn build_result(&self, skip: impl Fn(usize) -> bool) -> String {
        let mut result = String::with_capacity(self.original.len());
        for i in 0..self.entries.len() {
            if !skip(i) {
                result.push_str(self.get(i));
                result.push('\n');
            }
        }
        result
    }
}
// ======== end of lifted code ========

#[derive(Debug, Clone, PartialEq)]
enum LineKind<'a> {
    Empty,
    Label(&'a str),
    Directive(&'a str),
    Instr { mnemonic: &'a str, ops: Vec<&'a str> },
}

fn classify(line: &str) -> LineKind<'_> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("//") {
        return LineKind::Empty;
    }
    if let Some(l) = t.strip_suffix(':') {
        return LineKind::Label(l);
    }
    if t.starts_with('.') {
        return LineKind::Directive(t);
    }
    match t.split_once(' ') {
        Some((m, rest)) => LineKind::Instr { mnemonic: m, ops: split_operands(rest) },
        None => LineKind::Instr { mnemonic: t, ops: vec![] },
    }
}

/// Split operands on commas that are not inside brackets.
fn split_operands(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

#[derive(Default, Debug)]
struct Stats {
    by_pass: BTreeMap<&'static str, usize>,
}

impl Stats {
    fn bump(&mut self, pass: &'static str) {
        *self.by_pass.entry(pass).or_insert(0) += 1;
    }
}

/// str xN, [sp, #k] ; ldr xM, [sp, #k]  =>  keep store, ldr -> mov xM, xN (or drop if M == N)
fn store_load_forward(store: &mut LineStore, dead: &mut [bool], stats: &mut Stats) {
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        let LineKind::Instr { mnemonic: "str", ops } = classify(store.get(i)) else { continue };
        let (src, addr) = (ops[0].to_string(), ops[1].to_string());
        // next live line
        let Some(j) = (i + 1..store.len()).find(|&j| !dead[j]) else { continue };
        let LineKind::Instr { mnemonic: "ldr", ops: lops } = classify(store.get(j)) else { continue };
        if lops[1] != addr {
            continue;
        }
        let dst = lops[0].to_string();
        if dst == src {
            dead[j] = true;
            stats.bump("store_load_same");
        } else {
            store.replace(j, format!("    mov {}, {}", dst, src));
            stats.bump("store_load_mov");
        }
    }
}

fn redundant_moves(store: &mut LineStore, dead: &mut [bool], stats: &mut Stats) {
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        match classify(store.get(i)) {
            LineKind::Instr { mnemonic: "mov", ops } if ops.len() == 2 && ops[0] == ops[1] => {
                dead[i] = true;
                stats.bump("self_move");
            }
            LineKind::Instr { mnemonic: "add" | "sub", ops } if ops.len() == 3 && ops[0] == ops[1] && ops[2] == "#0" => {
                dead[i] = true;
                stats.bump("add_zero");
            }
            _ => {}
        }
    }
}

fn branch_to_next(store: &mut LineStore, dead: &mut [bool], stats: &mut Stats) {
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        let LineKind::Instr { mnemonic: "b", ops } = classify(store.get(i)) else { continue };
        let target = ops[0].to_string();
        // skip following labels: if any of them is the target, the branch is useless
        let mut j = i + 1;
        while j < store.len() {
            if dead[j] {
                j += 1;
                continue;
            }
            match classify(store.get(j)) {
                LineKind::Label(l) if l == target => {
                    dead[i] = true;
                    stats.bump("branch_to_next");
                    break;
                }
                LineKind::Label(_) | LineKind::Empty => j += 1,
                _ => break,
            }
        }
    }
}

fn dead_labels(store: &mut LineStore, dead: &mut [bool], stats: &mut Stats) {
    let mut referenced: HashSet<String> = HashSet::new();
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        if let LineKind::Instr { ops, .. } = classify(store.get(i)) {
            for o in ops {
                referenced.insert(o.to_string());
            }
        }
        if let LineKind::Directive(d) = classify(store.get(i)) {
            if let Some(rest) = d.strip_prefix(".globl ") {
                referenced.insert(rest.trim().to_string());
            }
        }
    }
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        if let LineKind::Label(l) = classify(store.get(i)) {
            if l.starts_with(".L") && !referenced.contains(l) {
                dead[i] = true;
                stats.bump("dead_label");
            }
        }
    }
}

/// mov xA, xB followed (in the same basic block) by uses of xA as a source:
/// rewrite those uses to xB until either register is redefined.
fn copy_propagate(store: &mut LineStore, dead: &mut [bool], stats: &mut Stats) {
    for i in 0..store.len() {
        if dead[i] {
            continue;
        }
        let LineKind::Instr { mnemonic: "mov", ops } = classify(store.get(i)) else { continue };
        if ops.len() != 2 || !ops[1].starts_with('x') {
            continue;
        }
        let (a, b) = (ops[0].to_string(), ops[1].to_string());
        for j in i + 1..store.len() {
            if dead[j] {
                continue;
            }
            let line = store.get(j).to_string();
            let kind = classify(&line);
            let LineKind::Instr { mnemonic, ops } = kind else { break };
            if mnemonic.starts_with('b') || mnemonic == "ret" || mnemonic.starts_with("cb") {
                break;
            }
            let defines = |r: &str| ops.first().map_or(false, |d| *d == r) && mnemonic != "str" && mnemonic != "cmp";
            if let Some(new) = replace_source_reg_in_instruction(&line, &a, &b) {
                store.replace(j, new);
                stats.bump("copy_prop");
            } else if mnemonic == "cmp" && has_whole_word(&line, &a) {
                let new = format!("    cmp {}", replace_whole_word(&line.trim()[4..], &a, &b));
                store.replace(j, new);
                stats.bump("copy_prop_cmp");
            }
            if defines(&a) || defines(&b) {
                break;
            }
        }
    }
}

const INPUT: &str = "    .text
    .globl main
main:
    stp x29, x30, [sp, #-32]!
    mov x29, sp
    mov x0, #5
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    mov x1, x0
    add x2, x1, x1
    add x2, x2, #0
    str x2, [sp, #24]
    ldr x3, [sp, #24]
    cmp x1, x3
    b.ge .L1
    b .L2
.L2:
.L5:
    mov x4, x4
    mov x5, x3
    mul x6, x5, x5
    sub x6, x6, #0
    mov x5, #1
    add x7, x5, x6
    b .L3
.L1:
    mov x7, #0
.L3:
    mov x0, x7
    ldp x29, x30, [sp], #32
    ret
.Lunused:
    // comment line
    .size main, .-main
";

fn main() {
    // helper behaviour
    println!("{}", replace_whole_word("add x1, x11, x1.0, x1", "x1", "x9"));
    println!("{}", has_whole_word("ldr x10, [x1]", "x1"));
    println!("{}", has_whole_word("ldr x10, [x12]", "x1"));
    println!("{:?}", replace_source_reg_in_instruction("    add x1, x1, x2", "x1", "x7"));
    println!("{:?}", replace_source_reg_in_instruction("    add x1, x3, x2", "x1", "x7"));
    println!("{:?}", replace_source_reg_in_instruction("ret", "x1", "x7"));
    println!("{:?}", split_operands("x0, [sp, #16], lsl #2"));
    println!("{:?}", classify("    ldp x29, x30, [sp], #32"));
    println!("{:?}", classify(".L3:"));

    let mut store = LineStore::new(INPUT.to_string());
    println!("lines: {}", store.len());
    let mut dead = vec![false; store.len()];
    let mut stats = Stats::default();
    type Pass = fn(&mut LineStore, &mut [bool], &mut Stats);
    let passes: [(&str, Pass); 5] = [
        ("store_load_forward", store_load_forward),
        ("redundant_moves", redundant_moves),
        ("copy_propagate", copy_propagate),
        ("branch_to_next", branch_to_next),
        ("dead_labels", dead_labels),
    ];
    for round in 1..=6 {
        let before: usize = stats.by_pass.values().sum();
        for (name, p) in passes.iter() {
            let b: usize = stats.by_pass.values().sum();
            p(&mut store, &mut dead, &mut stats);
            let a: usize = stats.by_pass.values().sum();
            if a > b {
                println!("round {} {:<20} +{}", round, name, a - b);
            }
        }
        let after: usize = stats.by_pass.values().sum();
        if after == before {
            println!("fixpoint at round {}", round);
            break;
        }
    }
    println!("stats: {:?}", stats.by_pass);
    let result = store.build_result(|i| dead[i]);
    print!("{}", result);
    println!("{} -> {} lines, {} replacements", store.len(), result.lines().count(), store.replacements.len());
    // a store with no trailing newline and an empty store
    let s2 = LineStore::new("a\nb".to_string());
    let s3 = LineStore::new(String::new());
    println!("{} {:?} {:?} | {} {:?}", s2.len(), s2.get(0), s2.get(1), s3.len(), s3.get(0));
}
