//! Parser for NetHack's `dungeon.def` format.
//!
//! The format is line-oriented: each line starts with a keyword followed by
//! a colon and arguments. `DUNGEON` starts a new dungeon block; subsequent
//! keywords modify the current dungeon or its most-recent level.

use nethack_types::dungeon::{
    BranchDef, BranchDirection, BranchType, DungeonAlignment, DungeonDef, DungeonFlags,
    DungeonTopology, LevelDef,
};

#[derive(Debug, thiserror::Error)]
pub enum DungeonParseError {
    #[error("line {line}: {msg}")]
    Parse { line: usize, msg: String },
}

/// Parse a `dungeon.def` file into a `DungeonTopology`.
pub fn parse_dungeon_def(input: &str) -> Result<DungeonTopology, DungeonParseError> {
    let mut dungeons = Vec::new();
    let mut current: Option<DungeonDef> = None;

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let (keyword, rest) = split_keyword(line).ok_or_else(|| DungeonParseError::Parse {
            line: line_num + 1,
            msg: format!("expected keyword, got: {line}"),
        })?;

        match keyword {
            "DUNGEON" => {
                if let Some(d) = current.take() {
                    dungeons.push(d);
                }
                current = Some(parse_dungeon_line(rest, line_num)?);
            }
            "DESCRIPTION" => {
                let d = current_mut(&mut current, line_num)?;
                apply_description(&mut d.flags, rest.trim(), line_num)?;
            }
            "ALIGNMENT" => {
                let d = current_mut(&mut current, line_num)?;
                d.flags.align = parse_alignment(rest.trim(), line_num)?;
            }
            "ENTRY" => {
                let d = current_mut(&mut current, line_num)?;
                d.entry = parse_i16(rest.trim(), line_num)?;
            }
            "PROTOFILE" => {
                let d = current_mut(&mut current, line_num)?;
                d.protofile = Some(unquote(rest.trim(), line_num)?);
            }
            "LEVEL" => {
                let d = current_mut(&mut current, line_num)?;
                d.levels
                    .push(parse_level_line(rest, line_num, false, false)?);
            }
            "RNDLEVEL" => {
                let d = current_mut(&mut current, line_num)?;
                d.levels
                    .push(parse_level_line(rest, line_num, true, false)?);
            }
            "CHAINLEVEL" => {
                let d = current_mut(&mut current, line_num)?;
                d.levels
                    .push(parse_level_line(rest, line_num, false, true)?);
            }
            "LEVELDESC" => {
                let d = current_mut(&mut current, line_num)?;
                let lev = d
                    .levels
                    .last_mut()
                    .ok_or_else(|| DungeonParseError::Parse {
                        line: line_num + 1,
                        msg: "LEVELDESC before any LEVEL".into(),
                    })?;
                apply_description(&mut lev.flags, rest.trim(), line_num)?;
            }
            "LEVALIGN" => {
                let d = current_mut(&mut current, line_num)?;
                let lev = d
                    .levels
                    .last_mut()
                    .ok_or_else(|| DungeonParseError::Parse {
                        line: line_num + 1,
                        msg: "LEVALIGN before any LEVEL".into(),
                    })?;
                lev.flags.align = parse_alignment(rest.trim(), line_num)?;
            }
            "BRANCH" => {
                let d = current_mut(&mut current, line_num)?;
                d.branches.push(parse_branch_line(rest, line_num, false)?);
            }
            "CHAINBRANCH" => {
                let d = current_mut(&mut current, line_num)?;
                d.branches.push(parse_branch_line(rest, line_num, true)?);
            }
            _ => {
                return Err(DungeonParseError::Parse {
                    line: line_num + 1,
                    msg: format!("unknown keyword: {keyword}"),
                });
            }
        }
    }

    if let Some(d) = current {
        dungeons.push(d);
    }

    Ok(DungeonTopology { dungeons })
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(pos) => &line[..pos],
        None => line,
    }
}

fn split_keyword(line: &str) -> Option<(&str, &str)> {
    let colon_pos = line.find(':')?;
    let keyword = line[..colon_pos].trim();
    let rest = &line[colon_pos + 1..];
    Some((keyword, rest))
}

fn current_mut(
    current: &mut Option<DungeonDef>,
    line_num: usize,
) -> Result<&mut DungeonDef, DungeonParseError> {
    current.as_mut().ok_or_else(|| DungeonParseError::Parse {
        line: line_num + 1,
        msg: "keyword before any DUNGEON".into(),
    })
}

/// Parse: `"name" "boneschar" (base, rand)`
fn parse_dungeon_line(rest: &str, line_num: usize) -> Result<DungeonDef, DungeonParseError> {
    let tokens = tokenize(rest);
    // Expect: "name" "bones" ( base , rand )
    if tokens.len() < 7 {
        return Err(parse_err(
            line_num,
            "DUNGEON requires: \"name\" \"bones\" (base, rand)",
        ));
    }
    let name = unquote(&tokens[0], line_num)?;
    let boneschar = unquote(&tokens[1], line_num)?;
    // tokens[2] = "(", tokens[3] = base, tokens[4] = ",", tokens[5] = rand, tokens[6] = ")"
    let base = parse_i16(&tokens[3], line_num)?;
    let rand = parse_i16(&tokens[5], line_num)?;

    Ok(DungeonDef {
        name,
        boneschar,
        base,
        rand,
        flags: DungeonFlags::default(),
        entry: 0,
        protofile: None,
        levels: Vec::new(),
        branches: Vec::new(),
    })
}

/// Parse level lines in three variants:
/// LEVEL:     `"name" "bones" @ (base, rand) [chance]`
/// RNDLEVEL:  `"name" "bones" @ (base, rand) count [chance]`
/// CHAINLEVEL: `"name" "bones" "chain" + (base, rand)`
fn parse_level_line(
    rest: &str,
    line_num: usize,
    is_rnd: bool,
    is_chain: bool,
) -> Result<LevelDef, DungeonParseError> {
    let tokens = tokenize(rest);
    let name = unquote(&tokens[0], line_num)?;
    let boneschar = unquote(&tokens[1], line_num)?;

    if is_chain {
        // "name" "bones" "chain" + ( base , rand )
        let chain = unquote(&tokens[2], line_num)?;
        // tokens[3] = "+", tokens[4] = "(", tokens[5] = base, tokens[6] = ",", tokens[7] = rand, tokens[8] = ")"
        let base = parse_i16(&tokens[5], line_num)?;
        let rand = parse_i16(&tokens[7], line_num)?;
        Ok(LevelDef {
            name,
            boneschar,
            chain: Some(chain),
            offset_base: base,
            offset_rand: rand,
            rndlevs: 0,
            chance: 100,
            flags: DungeonFlags::default(),
        })
    } else {
        // "name" "bones" @ ( base , rand ) [count] [chance]
        // tokens[2] = "@", tokens[3] = "(", tokens[4] = base, tokens[5] = ",", tokens[6] = rand, tokens[7] = ")"
        let base = parse_i16(&tokens[4], line_num)?;
        let rand = parse_i16(&tokens[6], line_num)?;

        let mut rndlevs: u8 = 0;
        let mut chance: u8 = 100;
        let extra = &tokens[8..];

        if is_rnd {
            // RNDLEVEL has count as next token, then optional chance
            if !extra.is_empty() {
                rndlevs = parse_i16(&extra[0], line_num)? as u8;
            }
            if extra.len() > 1 {
                chance = parse_i16(&extra[1], line_num)? as u8;
            }
        } else {
            // LEVEL may have an optional chance
            if !extra.is_empty() {
                chance = parse_i16(&extra[0], line_num)? as u8;
            }
        }

        Ok(LevelDef {
            name,
            boneschar,
            chain: None,
            offset_base: base,
            offset_rand: rand,
            rndlevs,
            chance,
            flags: DungeonFlags::default(),
        })
    }
}

/// Parse branch lines:
/// BRANCH:      `"name" @ (base, rand) [type] [direction]`
/// CHAINBRANCH: `"name" "chain" + (base, rand) [type] [direction]`
fn parse_branch_line(
    rest: &str,
    line_num: usize,
    is_chain: bool,
) -> Result<BranchDef, DungeonParseError> {
    let tokens = tokenize(rest);
    let name = unquote(&tokens[0], line_num)?;

    let (chain, offset_start) = if is_chain {
        let chain = unquote(&tokens[1], line_num)?;
        // tokens[2] = "+", tokens[3] = "(", ...
        (Some(chain), 4)
    } else {
        // tokens[1] = "@", tokens[2] = "(", ...
        (None, 3)
    };

    let base = parse_i16(&tokens[offset_start], line_num)?;
    let rand = parse_i16(&tokens[offset_start + 2], line_num)?;
    let extra_start = offset_start + 4; // past ")"

    let mut branch_type = BranchType::Stair;
    let mut direction = None;

    for token in tokens.iter().skip(extra_start) {
        match token.as_str() {
            "portal" => branch_type = BranchType::Portal,
            "no_up" => branch_type = BranchType::NoUp,
            "no_down" => branch_type = BranchType::NoDown,
            "up" => direction = Some(BranchDirection::Up),
            "down" => direction = Some(BranchDirection::Down),
            _ => {
                return Err(parse_err(
                    line_num,
                    &format!("unknown branch modifier: {token}"),
                ));
            }
        }
    }

    Ok(BranchDef {
        name,
        chain,
        offset_base: base,
        offset_rand: rand,
        branch_type,
        direction,
    })
}

fn apply_description(
    flags: &mut DungeonFlags,
    desc: &str,
    line_num: usize,
) -> Result<(), DungeonParseError> {
    match desc {
        "town" => flags.town = true,
        "hellish" => flags.hellish = true,
        "mazelike" => flags.maze_like = true,
        "roguelike" => flags.rogue_like = true,
        _ => {
            return Err(parse_err(line_num, &format!("unknown description: {desc}")));
        }
    }
    Ok(())
}

fn parse_alignment(s: &str, line_num: usize) -> Result<DungeonAlignment, DungeonParseError> {
    match s {
        "unaligned" => Ok(DungeonAlignment::Unaligned),
        "lawful" => Ok(DungeonAlignment::Lawful),
        "neutral" => Ok(DungeonAlignment::Neutral),
        "chaotic" => Ok(DungeonAlignment::Chaotic),
        "noalign" => Ok(DungeonAlignment::Noalign),
        _ => Err(parse_err(line_num, &format!("unknown alignment: {s}"))),
    }
}

fn parse_i16(s: &str, line_num: usize) -> Result<i16, DungeonParseError> {
    s.parse::<i16>()
        .map_err(|_| parse_err(line_num, &format!("expected integer, got: {s}")))
}

fn unquote(s: &str, line_num: usize) -> Result<String, DungeonParseError> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        Ok(s[1..s.len() - 1].to_string())
    } else {
        Err(parse_err(
            line_num,
            &format!("expected quoted string, got: {s}"),
        ))
    }
}

fn parse_err(line_num: usize, msg: &str) -> DungeonParseError {
    DungeonParseError::Parse {
        line: line_num + 1,
        msg: msg.into(),
    }
}

/// Simple tokenizer that respects quoted strings and treats punctuation as
/// separate tokens.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '"' {
            let mut s = String::new();
            s.push(chars.next().expect("peeked"));
            while let Some(&c) = chars.peek() {
                s.push(chars.next().expect("peeked"));
                if c == '"' {
                    break;
                }
            }
            tokens.push(s);
        } else if "(),+@".contains(ch) {
            tokens.push(chars.next().expect("peeked").to_string());
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || "(),+@\"".contains(c) {
                    break;
                }
                s.push(chars.next().expect("peeked"));
            }
            tokens.push(s);
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_dungeon_def() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../nethack/dat/dungeon.def"
        ))
        .expect("dungeon.def should exist")
    }

    #[test]
    fn parse_actual_dungeon_def() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        assert_eq!(topo.dungeons.len(), 8);
    }

    #[test]
    fn doom_dungeon() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        let doom = &topo.dungeons[0];
        assert_eq!(doom.name, "The Dungeons of Doom");
        assert_eq!(doom.boneschar, "D");
        assert_eq!(doom.base, 25);
        assert_eq!(doom.rand, 5);
        assert_eq!(doom.flags.align, DungeonAlignment::Unaligned);
    }

    #[test]
    fn gehennom_flags() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        let geh = &topo.dungeons[1];
        assert_eq!(geh.name, "Gehennom");
        assert!(geh.flags.hellish);
        assert!(geh.flags.maze_like);
        assert_eq!(geh.flags.align, DungeonAlignment::Noalign);
    }

    #[test]
    fn sokoban_entry_and_levels() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        let sok = topo
            .dungeons
            .iter()
            .find(|d| d.name == "Sokoban")
            .expect("Sokoban dungeon");
        assert_eq!(sok.entry, -1);
        assert_eq!(sok.flags.align, DungeonAlignment::Neutral);
        assert_eq!(sok.levels.len(), 4);
        for lev in &sok.levels {
            assert!(lev.rndlevs > 0, "Sokoban levels should be RNDLEVEL");
        }
    }

    #[test]
    fn elemental_planes() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        let planes = topo
            .dungeons
            .iter()
            .find(|d| d.name == "The Elemental Planes")
            .expect("Elemental Planes");
        assert_eq!(planes.entry, -2);
        assert_eq!(planes.levels.len(), 6);
    }

    #[test]
    fn vlads_tower_protofile() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        let vlad = topo
            .dungeons
            .iter()
            .find(|d| d.name == "Vlad's Tower")
            .expect("Vlad's Tower");
        assert_eq!(vlad.protofile.as_deref(), Some("tower"));
    }

    #[test]
    fn chain_references_valid() {
        let input = load_dungeon_def();
        let topo = parse_dungeon_def(&input).expect("parse dungeon.def");
        for dungeon in &topo.dungeons {
            for level in &dungeon.levels {
                if let Some(chain) = &level.chain {
                    let found = dungeon.levels.iter().any(|l| l.name == *chain);
                    assert!(
                        found,
                        "CHAINLEVEL '{}' references unknown level '{chain}'",
                        level.name
                    );
                }
            }
            for branch in &dungeon.branches {
                if let Some(chain) = &branch.chain {
                    let found = dungeon.levels.iter().any(|l| l.name == *chain);
                    assert!(
                        found,
                        "CHAINBRANCH '{}' references unknown level '{chain}'",
                        branch.name
                    );
                }
            }
        }
    }

    #[test]
    fn empty_input() {
        let topo = parse_dungeon_def("").expect("empty input");
        assert!(topo.dungeons.is_empty());
    }

    #[test]
    fn unknown_keyword() {
        let result = parse_dungeon_def("DUNGEON: \"Test\" \"T\" (1, 0)\nFOOBAR: baz\n");
        assert!(result.is_err());
    }
}
