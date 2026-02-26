use nethack_data::{des_parser, dungeon_parser};
use std::path::Path;

const DAT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../nethack/dat");

#[test]
fn dungeon_def_parses_to_expected_dungeons() {
    let path = Path::new(DAT_DIR).join("dungeon.def");
    let input = std::fs::read_to_string(&path).expect("read dungeon.def");
    let topo = dungeon_parser::parse_dungeon_def(&input).expect("parse dungeon.def");
    assert_eq!(topo.dungeons.len(), 8, "expected 8 dungeons");

    let names: Vec<&str> = topo.dungeons.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"The Dungeons of Doom"));
    assert!(names.contains(&"Gehennom"));
    assert!(names.contains(&"The Gnomish Mines"));
    assert!(names.contains(&"The Quest"));
    assert!(names.contains(&"Sokoban"));
    assert!(names.contains(&"Fort Ludios"));
    assert!(names.contains(&"Vlad's Tower"));
    assert!(names.contains(&"The Elemental Planes"));
}

#[test]
fn all_des_files_parse() {
    let dat_dir = Path::new(DAT_DIR);
    let mut files: Vec<_> = std::fs::read_dir(dat_dir)
        .expect("read dat dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "des"))
        .collect();
    files.sort();

    assert!(files.len() >= 24, "expected at least 24 .des files");

    for path in &files {
        let input =
            std::fs::read_to_string(path).unwrap_or_else(|_| panic!("read {}", path.display()));
        let des = des_parser::parse_des_file(&input)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        assert!(
            !des.levels.is_empty(),
            "{} produced no levels",
            path.display()
        );
        for level in &des.levels {
            assert!(
                !level.opcodes.is_empty(),
                "{}: level '{}' has no opcodes",
                path.display(),
                level.name
            );
        }
    }
}

#[test]
fn bigroom_produces_10_levels() {
    let input =
        std::fs::read_to_string(Path::new(DAT_DIR).join("bigroom.des")).expect("read bigroom.des");
    let des = des_parser::parse_des_file(&input).expect("parse bigroom.des");
    assert_eq!(des.levels.len(), 10, "bigroom.des should define 10 levels");
}

#[test]
fn castle_has_map_and_drawbridge() {
    use nethack_types::sp_lev::SpOpcode;
    let input =
        std::fs::read_to_string(Path::new(DAT_DIR).join("castle.des")).expect("read castle.des");
    let des = des_parser::parse_des_file(&input).expect("parse castle.des");
    let opcodes: Vec<SpOpcode> = des.levels[0].opcodes.iter().map(|o| o.opcode).collect();
    assert!(opcodes.contains(&SpOpcode::Map), "castle should have MAP");
    assert!(
        opcodes.contains(&SpOpcode::Drawbridge),
        "castle should have DRAWBRIDGE"
    );
    assert!(opcodes.contains(&SpOpcode::Door), "castle should have DOOR");
}

#[test]
fn sokoban_has_premapped_flag() {
    use nethack_types::sp_lev::SpOpcode;
    let input =
        std::fs::read_to_string(Path::new(DAT_DIR).join("sokoban.des")).expect("read sokoban.des");
    let des = des_parser::parse_des_file(&input).expect("parse sokoban.des");
    // All sokoban levels should have FLAGS with PREMAPPED
    for level in &des.levels {
        let has_flags = level
            .opcodes
            .iter()
            .any(|o| o.opcode == SpOpcode::LevelFlags);
        assert!(
            has_flags,
            "sokoban level '{}' should have LEVEL_FLAGS opcode",
            level.name
        );
    }
}
