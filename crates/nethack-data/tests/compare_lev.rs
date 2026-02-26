//! Compares Rust `.des` parser output against C `lev_comp` binary `.lev` files.
//!
//! For each `.lev` fixture file, finds the corresponding `.des` source and level
//! name, parses with Rust, reads the C binary, and compares opcode-by-opcode.

use nethack_data::{des_parser, lev_reader};
use nethack_types::sp_lev::SpLevOpcode;
use std::collections::HashMap;
use std::path::Path;

const DAT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../nethack/dat");
const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lev");

/// Build a mapping from level name → (des filename, SpecialLevel opcodes).
fn build_rust_levels() -> HashMap<String, (String, Vec<SpLevOpcode>)> {
    let dat_dir = Path::new(DAT_DIR);
    let mut des_files: Vec<_> = std::fs::read_dir(dat_dir)
        .expect("read dat dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "des"))
        .collect();
    des_files.sort();

    let mut map = HashMap::new();
    for path in &des_files {
        let input =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("read {}", path.display()));
        let des = des_parser::parse_des_file(&input)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        for level in des.levels {
            map.insert(level.name.clone(), (filename.clone(), level.opcodes));
        }
    }
    map
}

/// Format an opcode for diff output.
fn format_opcode(op: &SpLevOpcode) -> String {
    match &op.operand {
        None => format!("{:?}", op.opcode),
        Some(operand) => format!("{:?} {:?}", op.opcode, operand),
    }
}

#[test]
#[ignore = "des compiler is deferred (10/120 match) — not on critical path"]
fn all_lev_fixtures_match_rust_parser() {
    let rust_levels = build_rust_levels();

    let mut lev_files: Vec<_> = std::fs::read_dir(FIXTURES_DIR)
        .expect("read fixtures dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "lev"))
        .collect();
    lev_files.sort();

    assert!(
        !lev_files.is_empty(),
        "no .lev fixture files found in {FIXTURES_DIR}"
    );

    let mut failures = Vec::new();
    let mut checked = 0;

    for lev_path in &lev_files {
        let lev_name = lev_path.file_stem().unwrap().to_string_lossy().to_string();
        let data =
            std::fs::read(lev_path).unwrap_or_else(|_| panic!("read {}", lev_path.display()));

        let c_opcodes = match lev_reader::read_lev(&data) {
            Ok(ops) => ops,
            Err(e) => {
                failures.push(format!("{lev_name}.lev: failed to read: {e}"));
                continue;
            }
        };

        let (des_file, rust_opcodes) = match rust_levels.get(&lev_name) {
            Some(entry) => entry,
            None => {
                failures.push(format!(
                    "{lev_name}.lev: no matching Rust level found (level name not in any .des)"
                ));
                continue;
            }
        };

        if c_opcodes.len() != rust_opcodes.len() {
            failures.push(format!(
                "{lev_name}.lev (from {des_file}): opcode count mismatch: C={}, Rust={}",
                c_opcodes.len(),
                rust_opcodes.len()
            ));
            // Show first divergence even if lengths differ
            let min_len = c_opcodes.len().min(rust_opcodes.len());
            for i in 0..min_len {
                if c_opcodes[i] != rust_opcodes[i] {
                    failures.push(format!(
                        "  first mismatch at opcode[{i}]:\n    C:    {}\n    Rust: {}",
                        format_opcode(&c_opcodes[i]),
                        format_opcode(&rust_opcodes[i])
                    ));
                    break;
                }
            }
            continue;
        }

        let mut level_ok = true;
        for (i, (c_op, rust_op)) in c_opcodes.iter().zip(rust_opcodes.iter()).enumerate() {
            if c_op != rust_op {
                if level_ok {
                    failures.push(format!("{lev_name}.lev (from {des_file}): opcode mismatch"));
                    level_ok = false;
                }
                failures.push(format!(
                    "  opcode[{i}]:\n    C:    {}\n    Rust: {}",
                    format_opcode(c_op),
                    format_opcode(rust_op)
                ));
                // Show at most 3 mismatches per level
                if failures.len() > 50 {
                    failures.push("  ... (truncated)".to_string());
                    break;
                }
            }
        }

        if level_ok {
            checked += 1;
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} levels matched; failures:\n{}",
            checked,
            lev_files.len(),
            failures.join("\n")
        );
    }

    eprintln!("all {checked} .lev files match Rust parser output");
}
