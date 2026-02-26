//! Runtime dungeon state and initialization.
//!
//! Ports C's `dungeon.c` — specifically `init_dungeons()` which reads the
//! parsed `dungeon.def` topology and builds the runtime dungeon graph:
//! dungeon descriptors, special level placement, and branch connections.

use nethack_rng::NhRng;
use nethack_types::dungeon::{
    BranchDef, BranchDirection, BranchType, DungeonDef, DungeonFlags, DungeonTopology,
};
use serde::Serialize;
use thiserror::Error;

/// Maximum number of dungeons, matching C's `MAXDUNGEON`.
pub const MAX_DUNGEON: usize = 16;
/// Maximum levels per dungeon, matching C's `MAXLEVEL`.
pub const MAX_LEVEL: usize = 32;

#[derive(Debug, Error)]
pub enum DungeonError {
    #[error("too many dungeons: {0} exceeds maximum {MAX_DUNGEON}")]
    TooManyDungeons(usize),
    #[error("dungeon {name:?} not found for branch target")]
    DungeonNotFound { name: String },
    #[error("level {name:?} not found for chain reference")]
    LevelNotFound { name: String },
    #[error("could not place all special levels in dungeon {dungeon:?}")]
    PlacementFailed { dungeon: String },
}

/// A level reference: dungeon number + level within that dungeon.
/// Matches C's `d_level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct DLevel {
    /// Dungeon index (0-based).
    pub dnum: u8,
    /// Level within the dungeon (1-based).
    pub dlevel: u8,
}

/// Runtime branch type matching C's `BR_*` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuntimeBranchType {
    /// Regular staircase connection.
    Stair,
    /// No stair from end1 → end2.
    NoEnd1,
    /// No stair from end2 → end1.
    NoEnd2,
    /// Magic portal connection.
    Portal,
}

/// A branch connecting two dungeon levels.
/// Matches C's `struct branch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Branch {
    /// Unique branch identifier.
    pub id: usize,
    /// Connection type.
    pub typ: RuntimeBranchType,
    /// Primary endpoint.
    pub end1: DLevel,
    /// Secondary endpoint.
    pub end2: DLevel,
    /// Does end1 connect upward?
    pub end1_up: bool,
}

/// A placed special level in the dungeon.
/// Matches C's `struct s_level`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecialLevel {
    /// Location in the dungeon graph.
    pub dlevel: DLevel,
    /// Prototype file name (e.g., "castle").
    pub proto: String,
    /// Bones file character.
    pub boneid: String,
    /// Number of randomly interchangeable level variants.
    pub rndlevs: u8,
    /// Level descriptor flags.
    pub flags: DungeonFlags,
}

/// Runtime dungeon descriptor, matching C's `struct dungeon`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dungeon {
    /// Display name (e.g., "The Dungeons of Doom").
    pub name: String,
    /// Prototype file prefix.
    pub proto: String,
    /// Bones file character.
    pub boneid: String,
    /// Level descriptor flags.
    pub flags: DungeonFlags,
    /// Entry level within this dungeon (1-based).
    pub entry_lev: u8,
    /// Total number of levels.
    pub num_dunlevs: u8,
    /// Deepest level reached by the player.
    pub dunlev_ureached: u8,
    /// Starting ledger index (for save file bookkeeping).
    pub ledger_start: i32,
    /// Logical depth of this dungeon's level 1.
    pub depth_start: i32,
}

/// Fast lookup for well-known special levels.
/// Matches C's `dgn_topology` / `dungeon_topology`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WellKnownLevels {
    pub oracle_level: Option<DLevel>,
    pub bigroom_level: Option<DLevel>,
    pub rogue_level: Option<DLevel>,
    pub medusa_level: Option<DLevel>,
    pub stronghold_level: Option<DLevel>,
    pub valley_level: Option<DLevel>,
    pub wiz1_level: Option<DLevel>,
    pub wiz2_level: Option<DLevel>,
    pub wiz3_level: Option<DLevel>,
    pub juiblex_level: Option<DLevel>,
    pub orcus_level: Option<DLevel>,
    pub baalzebub_level: Option<DLevel>,
    pub asmodeus_level: Option<DLevel>,
    pub portal_level: Option<DLevel>,
    pub sanctum_level: Option<DLevel>,
    pub earth_level: Option<DLevel>,
    pub water_level: Option<DLevel>,
    pub fire_level: Option<DLevel>,
    pub air_level: Option<DLevel>,
    pub astral_level: Option<DLevel>,
    pub qstart_level: Option<DLevel>,
    pub qlocate_level: Option<DLevel>,
    pub nemesis_level: Option<DLevel>,
    pub knox_level: Option<DLevel>,
    pub mineend_level: Option<DLevel>,
    pub sokoend_level: Option<DLevel>,
    pub tower_dnum: Option<u8>,
    pub sokoban_dnum: Option<u8>,
    pub mines_dnum: Option<u8>,
    pub quest_dnum: Option<u8>,
}

/// The complete runtime dungeon state.
#[derive(Debug, Clone, Serialize)]
pub struct DungeonState {
    /// All dungeons.
    pub dungeons: Vec<Dungeon>,
    /// All branches connecting dungeons.
    pub branches: Vec<Branch>,
    /// All placed special levels (sorted by dnum, dlevel).
    pub special_levels: Vec<SpecialLevel>,
    /// Fast lookup for well-known levels.
    pub well_known: WellKnownLevels,
    /// Passtune: the 5-character tune for the Castle drawbridge.
    pub tune: String,
}

impl DungeonState {
    /// Compute the logical depth of a dungeon level.
    /// Matches C's `depth(&d_level)`.
    pub fn depth(&self, lev: &DLevel) -> i32 {
        self.dungeons[lev.dnum as usize].depth_start + lev.dlevel as i32 - 1
    }

    /// Total number of levels in the dungeon containing `lev`.
    pub fn dunlevs_in_dungeon(&self, lev: &DLevel) -> u8 {
        self.dungeons[lev.dnum as usize].num_dunlevs
    }

    /// Look up a dungeon by name.
    pub fn dname_to_dnum(&self, name: &str) -> Option<u8> {
        self.dungeons
            .iter()
            .position(|d| d.name == name)
            .map(|i| i as u8)
    }

    /// Find a special level by prototype name.
    pub fn find_special_level(&self, proto: &str) -> Option<&SpecialLevel> {
        self.special_levels.iter().find(|sl| sl.proto == proto)
    }
}

/// Initialize the dungeon graph from a parsed `DungeonTopology`.
///
/// This is the Rust equivalent of C's `init_dungeons()` in `dungeon.c`.
/// It takes the parsed dungeon.def data and produces the runtime dungeon state
/// with randomized level counts and special level placements.
pub fn init_dungeons(
    topology: &DungeonTopology,
    rng: &mut NhRng,
) -> Result<DungeonState, DungeonError> {
    if topology.dungeons.len() > MAX_DUNGEON {
        return Err(DungeonError::TooManyDungeons(topology.dungeons.len()));
    }

    let mut state = DungeonState {
        dungeons: Vec::with_capacity(topology.dungeons.len()),
        branches: Vec::new(),
        special_levels: Vec::new(),
        well_known: WellKnownLevels::default(),
        tune: String::new(),
    };

    // Process each dungeon definition
    for (i, def) in topology.dungeons.iter().enumerate() {
        let dungeon = build_dungeon(i, def, &state, rng)?;
        state.dungeons.push(dungeon);

        // Create branch from parent dungeon (skip the root dungeon)
        if i > 0 {
            create_entry_branch(i, def, &mut state, &topology.dungeons, rng)?;
        }

        // Place special levels
        place_special_levels(i, def, &mut state, rng)?;

        // Process branch definitions within this dungeon
        for branch_def in &def.branches {
            // Branches to other dungeons are created when those dungeons
            // are processed (via create_entry_branch), but branches within
            // the same dungeon need explicit handling.
            // The branch_def.name references the *target* dungeon, which
            // may not exist yet. We defer these — they'll be picked up
            // when the target dungeon is created.
            let _ = branch_def; // processed via create_entry_branch
        }
    }

    // Generate the Castle passtune: 5 random letters A–G
    state.tune = (0..5).map(|_| (b'A' + rng.rn2(7) as u8) as char).collect();

    // Populate well-known level lookups
    populate_well_known_levels(&mut state);

    Ok(state)
}

/// Build a single runtime dungeon descriptor from its definition.
fn build_dungeon(
    index: usize,
    def: &DungeonDef,
    state: &DungeonState,
    rng: &mut NhRng,
) -> Result<Dungeon, DungeonError> {
    // Randomize level count
    let num_dunlevs = if def.rand > 0 {
        // rn1(rand, base) = rn2(rand) + base
        (rng.rn2(def.rand as i32) + def.base as i32) as u8
    } else {
        def.base as u8
    };
    let num_dunlevs = num_dunlevs.min(MAX_LEVEL as u8);

    // Compute ledger start (cumulative level count of previous dungeons)
    let ledger_start = if index == 0 {
        0
    } else {
        let prev = &state.dungeons[index - 1];
        prev.ledger_start + prev.num_dunlevs as i32
    };

    // Compute entry level
    let entry_lev = if def.entry < 0 {
        // Negative: count from bottom (-1 = last level)
        let e = num_dunlevs as i16 + def.entry + 1;
        e.max(1) as u8
    } else if def.entry > 0 {
        (def.entry as u8).min(num_dunlevs)
    } else {
        1 // default: top level
    };

    // depth_start is set later for non-root dungeons (after branch is created)
    let depth_start = if index == 0 { 1 } else { 0 };

    Ok(Dungeon {
        name: def.name.clone(),
        proto: def.protofile.clone().unwrap_or_default(),
        boneid: def.boneschar.clone(),
        flags: def.flags,
        entry_lev,
        num_dunlevs,
        dunlev_ureached: if index == 0 { 1 } else { 0 },
        ledger_start,
        depth_start,
    })
}

/// Create the branch that connects a child dungeon to its parent.
///
/// The parent and connection level are determined by looking at which
/// dungeon's branch definitions reference this dungeon.
fn create_entry_branch(
    child_dnum: usize,
    child_def: &DungeonDef,
    state: &mut DungeonState,
    all_defs: &[DungeonDef],
    rng: &mut NhRng,
) -> Result<(), DungeonError> {
    // Find which parent dungeon has a branch pointing to us
    let mut parent_dnum = None;
    let mut branch_def = None;

    for (pi, pdef) in all_defs.iter().enumerate() {
        if pi >= child_dnum {
            break; // only look at already-processed dungeons
        }
        for bdef in &pdef.branches {
            if bdef.name == child_def.name {
                parent_dnum = Some(pi);
                branch_def = Some(bdef);
                break;
            }
        }
        if parent_dnum.is_some() {
            break;
        }
    }

    let parent_dnum = parent_dnum.ok_or_else(|| DungeonError::DungeonNotFound {
        name: child_def.name.clone(),
    })?;
    let bdef = branch_def.expect("branch_def set when parent_dnum is set");

    // Determine parent connection level
    let parent_dlevel = resolve_level_offset(parent_dnum, bdef, state, all_defs, rng);

    // Determine child connection level (its entry point)
    let child_dlevel = state.dungeons[child_dnum].entry_lev;

    // Determine branch type
    let (typ, end1_up) = convert_branch_type(bdef);

    let branch_id = state.branches.len();
    let branch = Branch {
        id: branch_id,
        typ,
        end1: DLevel {
            dnum: parent_dnum as u8,
            dlevel: parent_dlevel,
        },
        end2: DLevel {
            dnum: child_dnum as u8,
            dlevel: child_dlevel,
        },
        end1_up,
    };

    // Compute child's depth_start from the branch connection
    let parent_depth = state.depth(&branch.end1);
    let depth_offset = if typ == RuntimeBranchType::Portal {
        0
    } else if end1_up {
        1 // going down from parent
    } else {
        -1 // going up from parent
    };
    state.dungeons[child_dnum].depth_start =
        parent_depth + depth_offset - (child_dlevel as i32 - 1);

    // Insert branch (sorted by end1 position)
    let insert_pos = state
        .branches
        .iter()
        .position(|b| {
            (b.end1.dnum, b.end1.dlevel, b.end2.dnum, b.end2.dlevel)
                > (
                    branch.end1.dnum,
                    branch.end1.dlevel,
                    branch.end2.dnum,
                    branch.end2.dlevel,
                )
        })
        .unwrap_or(state.branches.len());
    state.branches.insert(insert_pos, branch);

    Ok(())
}

/// Resolve the level offset for a branch definition within its parent dungeon.
fn resolve_level_offset(
    parent_dnum: usize,
    bdef: &BranchDef,
    state: &DungeonState,
    all_defs: &[DungeonDef],
    rng: &mut NhRng,
) -> u8 {
    let parent = &state.dungeons[parent_dnum];

    if let Some(ref chain_name) = bdef.chain {
        // Chained: relative to a special level in the same dungeon
        if let Some(sl) = state
            .special_levels
            .iter()
            .find(|sl| sl.proto == *chain_name && sl.dlevel.dnum == parent_dnum as u8)
        {
            let base = sl.dlevel.dlevel as i16 + bdef.offset_base;
            let rand_offset = if bdef.offset_rand > 0 {
                rng.rn2(bdef.offset_rand as i32) as i16
            } else {
                0
            };
            let result = (base + rand_offset).max(1).min(parent.num_dunlevs as i16);
            return result as u8;
        }
        // Chain target not found — fall through to absolute positioning
        log::warn!(
            "chain target {:?} not found in dungeon {:?}, using absolute offset",
            chain_name,
            all_defs[parent_dnum].name
        );
    }

    // Absolute positioning
    let base = bdef.offset_base;
    let rand_offset = if bdef.offset_rand < 0 {
        // rand=-1 means "base to end of dungeon"
        let range = parent.num_dunlevs as i16 - base + 1;
        if range > 0 {
            rng.rn2(range as i32) as i16
        } else {
            0
        }
    } else if bdef.offset_rand > 0 {
        rng.rn2(bdef.offset_rand as i32) as i16
    } else {
        0
    };

    (base + rand_offset).max(1).min(parent.num_dunlevs as i16) as u8
}

/// Convert a parsed branch type and direction into runtime types.
fn convert_branch_type(bdef: &BranchDef) -> (RuntimeBranchType, bool) {
    let end1_up = match bdef.direction {
        Some(BranchDirection::Up) => true,
        Some(BranchDirection::Down) | None => false,
    };

    let typ = match bdef.branch_type {
        BranchType::Stair => RuntimeBranchType::Stair,
        BranchType::NoUp => {
            if end1_up {
                RuntimeBranchType::NoEnd1
            } else {
                RuntimeBranchType::NoEnd2
            }
        }
        BranchType::NoDown => {
            if end1_up {
                RuntimeBranchType::NoEnd2
            } else {
                RuntimeBranchType::NoEnd1
            }
        }
        BranchType::Portal => RuntimeBranchType::Portal,
    };

    (typ, end1_up)
}

/// Place special levels within a dungeon.
///
/// Levels with chain references are placed relative to their chain target.
/// Unchained levels are placed at absolute offsets. If multiple levels compete
/// for the same slot, the placement algorithm tries alternatives.
fn place_special_levels(
    dnum: usize,
    def: &DungeonDef,
    state: &mut DungeonState,
    rng: &mut NhRng,
) -> Result<(), DungeonError> {
    let num_dunlevs = state.dungeons[dnum].num_dunlevs;

    // Track which levels in this dungeon are already occupied by specials
    let mut occupied = vec![false; num_dunlevs as usize + 1]; // 1-indexed

    // First pass: place levels that have chain references
    // (they depend on already-placed levels)
    let mut deferred = Vec::new();

    for level_def in &def.levels {
        // Apply chance roll
        if level_def.chance < 100 && rng.rn2(100) >= level_def.chance as i32 {
            continue;
        }

        if level_def.chain.is_some() {
            deferred.push(level_def);
        } else if let Some(dlevel) = pick_level_slot(
            dnum,
            level_def.offset_base,
            level_def.offset_rand,
            num_dunlevs,
            &occupied,
            rng,
        ) {
            occupied[dlevel as usize] = true;
            state.special_levels.push(SpecialLevel {
                dlevel: DLevel {
                    dnum: dnum as u8,
                    dlevel,
                },
                proto: level_def.name.clone(),
                boneid: level_def.boneschar.clone(),
                rndlevs: level_def.rndlevs,
                flags: level_def.flags,
            });
        }
    }

    // Second pass: place chained levels
    for level_def in deferred {
        let chain_name = level_def.chain.as_ref().expect("deferred have chains");

        let chain_dlevel = state
            .special_levels
            .iter()
            .find(|sl| sl.proto == *chain_name && sl.dlevel.dnum == dnum as u8)
            .map(|sl| sl.dlevel.dlevel);

        if let Some(chain_lvl) = chain_dlevel {
            let base = chain_lvl as i16 + level_def.offset_base;
            if let Some(dlevel) = pick_level_slot(
                dnum,
                base,
                level_def.offset_rand,
                num_dunlevs,
                &occupied,
                rng,
            ) {
                occupied[dlevel as usize] = true;
                state.special_levels.push(SpecialLevel {
                    dlevel: DLevel {
                        dnum: dnum as u8,
                        dlevel,
                    },
                    proto: level_def.name.clone(),
                    boneid: level_def.boneschar.clone(),
                    rndlevs: level_def.rndlevs,
                    flags: level_def.flags,
                });
            }
        } else {
            log::warn!(
                "chain target {:?} not found for level {:?}",
                chain_name,
                level_def.name
            );
        }
    }

    // Sort special levels by (dnum, dlevel) to maintain ordered chain
    state
        .special_levels
        .sort_by_key(|sl| (sl.dlevel.dnum, sl.dlevel.dlevel));

    Ok(())
}

/// Pick an unoccupied level slot for a special level.
///
/// `base` and `rand` come from the level definition's `(base, rand)` pair.
/// Negative `base` means "from the bottom" (e.g., -1 = last level, -5 = 5th from end).
/// `rand` of 0 means exact placement; positive means a range `[base, base+rand)`;
/// negative (-1) means "base through end of dungeon".
fn pick_level_slot(
    _dnum: usize,
    base: i16,
    rand: i16,
    num_dunlevs: u8,
    occupied: &[bool],
    rng: &mut NhRng,
) -> Option<u8> {
    // Resolve negative base: -1 = last level, -2 = second-to-last, etc.
    let resolved_base = if base < 0 {
        let lvl = num_dunlevs as i16 + base + 1;
        lvl.max(1)
    } else {
        base.max(1)
    };

    let lo = (resolved_base as u8).min(num_dunlevs);
    let hi = if rand < 0 {
        // rand=-1 means "base to end"
        num_dunlevs
    } else if rand > 0 {
        ((resolved_base + rand - 1) as u8).min(num_dunlevs)
    } else {
        lo
    };

    // Collect all unoccupied slots in range
    let candidates: Vec<u8> = (lo..=hi).filter(|&l| !occupied[l as usize]).collect();

    if candidates.is_empty() {
        return None;
    }

    let idx = rng.rn2(candidates.len() as i32) as usize;
    Some(candidates[idx])
}

/// Map well-known level names to their `DLevel` locations.
fn populate_well_known_levels(state: &mut DungeonState) {
    for sl in &state.special_levels {
        match sl.proto.as_str() {
            "oracle" => state.well_known.oracle_level = Some(sl.dlevel),
            "bigrm" => state.well_known.bigroom_level = Some(sl.dlevel),
            "rogue" => state.well_known.rogue_level = Some(sl.dlevel),
            "medusa" => state.well_known.medusa_level = Some(sl.dlevel),
            "castle" => state.well_known.stronghold_level = Some(sl.dlevel),
            "valley" => state.well_known.valley_level = Some(sl.dlevel),
            "wizard1" => state.well_known.wiz1_level = Some(sl.dlevel),
            "wizard2" => state.well_known.wiz2_level = Some(sl.dlevel),
            "wizard3" => state.well_known.wiz3_level = Some(sl.dlevel),
            "juiblex" => state.well_known.juiblex_level = Some(sl.dlevel),
            "orcus" => state.well_known.orcus_level = Some(sl.dlevel),
            "baalz" => state.well_known.baalzebub_level = Some(sl.dlevel),
            "asmodeus" => state.well_known.asmodeus_level = Some(sl.dlevel),
            "sanctum" => state.well_known.sanctum_level = Some(sl.dlevel),
            "earth" => state.well_known.earth_level = Some(sl.dlevel),
            "water" => state.well_known.water_level = Some(sl.dlevel),
            "fire" => state.well_known.fire_level = Some(sl.dlevel),
            "air" => state.well_known.air_level = Some(sl.dlevel),
            "astral" => state.well_known.astral_level = Some(sl.dlevel),
            "knox" => {
                state.well_known.knox_level = Some(sl.dlevel);
                state.well_known.portal_level = Some(sl.dlevel);
            }
            _ => {
                if sl.proto.ends_with("-strt") {
                    state.well_known.qstart_level = Some(sl.dlevel);
                } else if sl.proto.ends_with("-loca") {
                    state.well_known.qlocate_level = Some(sl.dlevel);
                } else if sl.proto.ends_with("-goal") {
                    state.well_known.nemesis_level = Some(sl.dlevel);
                }
            }
        }
    }

    // Look up dungeon indices by name
    let tower_dnum = state
        .dungeons
        .iter()
        .position(|d| d.name == "Vlad's Tower")
        .map(|i| i as u8);
    let sokoban_dnum = state
        .dungeons
        .iter()
        .position(|d| d.name == "Sokoban")
        .map(|i| i as u8);
    let mines_dnum = state
        .dungeons
        .iter()
        .position(|d| d.name == "The Gnomish Mines")
        .map(|i| i as u8);
    let quest_dnum = state
        .dungeons
        .iter()
        .position(|d| d.name == "The Quest")
        .map(|i| i as u8);

    state.well_known.tower_dnum = tower_dnum;
    state.well_known.sokoban_dnum = sokoban_dnum;
    state.well_known.mines_dnum = mines_dnum;
    state.well_known.quest_dnum = quest_dnum;

    // Mine-end: deepest special level in mines
    if let Some(dnum) = mines_dnum {
        state.well_known.mineend_level = state
            .special_levels
            .iter()
            .filter(|sl| sl.dlevel.dnum == dnum && sl.proto.starts_with("minend"))
            .max_by_key(|sl| sl.dlevel.dlevel)
            .map(|sl| sl.dlevel);
    }
    // Sokoban-end: deepest sokoban level
    if let Some(dnum) = sokoban_dnum {
        state.well_known.sokoend_level = state
            .special_levels
            .iter()
            .filter(|sl| sl.dlevel.dnum == dnum && sl.proto.starts_with("soko"))
            .max_by_key(|sl| sl.dlevel.dlevel)
            .map(|sl| sl.dlevel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nethack_data::dungeon_parser;

    /// Load the actual dungeon.def and initialize dungeons.
    fn init_test_dungeons(seed: u64) -> DungeonState {
        let dat_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../nethack/dat/dungeon.def");
        let input = std::fs::read_to_string(&dat_dir)
            .unwrap_or_else(|_| panic!("read {}", dat_dir.display()));
        let topology = dungeon_parser::parse_dungeon_def(&input).expect("parse dungeon.def");
        let mut rng = NhRng::new(seed);
        init_dungeons(&topology, &mut rng).expect("init_dungeons")
    }

    #[test]
    fn has_eight_dungeons() {
        let state = init_test_dungeons(42);
        assert_eq!(state.dungeons.len(), 8);
    }

    #[test]
    fn dungeon_names() {
        let state = init_test_dungeons(42);
        let names: Vec<&str> = state.dungeons.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "The Dungeons of Doom",
                "Gehennom",
                "The Gnomish Mines",
                "The Quest",
                "Sokoban",
                "Fort Ludios",
                "Vlad's Tower",
                "The Elemental Planes",
            ]
        );
    }

    #[test]
    fn doom_is_root() {
        let state = init_test_dungeons(42);
        let doom = &state.dungeons[0];
        assert_eq!(doom.depth_start, 1);
        assert_eq!(doom.ledger_start, 0);
        assert_eq!(doom.dunlev_ureached, 1);
    }

    #[test]
    fn gehennom_is_hellish() {
        let state = init_test_dungeons(42);
        let geh = &state.dungeons[1];
        assert!(geh.flags.hellish);
        assert!(geh.flags.maze_like);
    }

    #[test]
    fn dungeons_have_positive_level_counts() {
        let state = init_test_dungeons(42);
        for d in &state.dungeons {
            assert!(d.num_dunlevs > 0, "{} has 0 levels", d.name);
        }
    }

    #[test]
    fn branches_connect_valid_dungeons() {
        let state = init_test_dungeons(42);
        for br in &state.branches {
            assert!(
                (br.end1.dnum as usize) < state.dungeons.len(),
                "branch end1 dnum out of range"
            );
            assert!(
                (br.end2.dnum as usize) < state.dungeons.len(),
                "branch end2 dnum out of range"
            );
            assert!(
                br.end1.dlevel >= 1
                    && br.end1.dlevel <= state.dungeons[br.end1.dnum as usize].num_dunlevs,
                "branch end1 dlevel out of range: {} not in 1..={}",
                br.end1.dlevel,
                state.dungeons[br.end1.dnum as usize].num_dunlevs
            );
            assert!(
                br.end2.dlevel >= 1
                    && br.end2.dlevel <= state.dungeons[br.end2.dnum as usize].num_dunlevs,
                "branch end2 dlevel out of range: {} not in 1..={}",
                br.end2.dlevel,
                state.dungeons[br.end2.dnum as usize].num_dunlevs
            );
        }
    }

    #[test]
    fn special_levels_are_placed() {
        let state = init_test_dungeons(42);
        // Should have various special levels
        assert!(
            !state.special_levels.is_empty(),
            "no special levels were placed"
        );

        // Castle must exist
        assert!(
            state.find_special_level("castle").is_some(),
            "castle level not found"
        );

        // Valley of the Dead must exist (entrance to Gehennom)
        assert!(
            state.find_special_level("valley").is_some(),
            "valley level not found"
        );
    }

    #[test]
    fn well_known_levels_populated() {
        let state = init_test_dungeons(42);
        let wk = &state.well_known;
        assert!(wk.stronghold_level.is_some(), "no castle/stronghold level");
        assert!(wk.valley_level.is_some(), "no valley level");
        assert!(wk.mines_dnum.is_some(), "no mines dungeon");
        assert!(wk.sokoban_dnum.is_some(), "no sokoban dungeon");
        assert!(wk.tower_dnum.is_some(), "no tower dungeon");
        assert!(wk.quest_dnum.is_some(), "no quest dungeon");
    }

    #[test]
    fn castle_is_deepest_doom_level() {
        let state = init_test_dungeons(42);
        let castle = state
            .find_special_level("castle")
            .expect("castle must exist");
        let doom = &state.dungeons[0];
        assert_eq!(
            castle.dlevel.dnum, 0,
            "castle should be in Dungeons of Doom"
        );
        assert_eq!(
            castle.dlevel.dlevel, doom.num_dunlevs,
            "castle should be the deepest level in Doom"
        );
    }

    #[test]
    fn passtune_is_five_letters() {
        let state = init_test_dungeons(42);
        assert_eq!(state.tune.len(), 5);
        assert!(state.tune.chars().all(|c| ('A'..='G').contains(&c)));
    }

    #[test]
    fn depth_increases_through_doom() {
        let state = init_test_dungeons(42);
        let doom = &state.dungeons[0];
        for dlevel in 1..=doom.num_dunlevs {
            let d = state.depth(&DLevel { dnum: 0, dlevel });
            assert_eq!(d, dlevel as i32, "depth should equal dlevel for dungeon 0");
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let s1 = init_test_dungeons(12345);
        let s2 = init_test_dungeons(12345);
        // Same seed → same dungeon counts
        assert_eq!(s1.dungeons.len(), s2.dungeons.len());
        for (d1, d2) in s1.dungeons.iter().zip(s2.dungeons.iter()) {
            assert_eq!(d1.num_dunlevs, d2.num_dunlevs);
            assert_eq!(d1.depth_start, d2.depth_start);
        }
        assert_eq!(s1.tune, s2.tune);
    }

    #[test]
    fn different_seeds_vary() {
        // Run many seeds and check that level counts vary
        let mut doom_levels = std::collections::HashSet::new();
        for seed in 0..100 {
            let state = init_test_dungeons(seed);
            doom_levels.insert(state.dungeons[0].num_dunlevs);
        }
        // Doom has base=25, rand=5, so levels range from 25-29
        assert!(
            doom_levels.len() > 1,
            "Doom level count should vary across seeds"
        );
    }

    #[test]
    fn elemental_planes_is_last_dungeon() {
        let state = init_test_dungeons(42);
        let last = state.dungeons.last().expect("at least one dungeon");
        assert_eq!(last.name, "The Elemental Planes");
        // Elemental planes has exactly 6 levels (earth, air, fire, water, astral + dummy)
        assert_eq!(last.num_dunlevs, 6);
    }

    #[test]
    fn fort_ludios_is_portal_branch() {
        let state = init_test_dungeons(42);
        let ludios_dnum = state
            .dname_to_dnum("Fort Ludios")
            .expect("Fort Ludios should exist");
        let branch = state
            .branches
            .iter()
            .find(|b| b.end2.dnum == ludios_dnum)
            .expect("branch to Fort Ludios");
        assert_eq!(branch.typ, RuntimeBranchType::Portal);
    }
}
