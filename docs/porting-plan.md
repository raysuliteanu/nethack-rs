# NetHack Rust Port — Multi-Phase Porting Plan

## Design Decisions

- **State management**: Central `GameState` struct passed by reference (not ECS, not globals)
- **Display**: Terminal only using crossterm + ratatui
- **Data files**: Parse original binary formats (.lev, dungeon.pdf) first, to use C-compiled assets during development
- **Save format**: Abstract behind a trait; bincode/MessagePack first, C-compatible format as a future option

## Workspace Structure

```
nethack-rs/
├── Cargo.toml              (workspace root)
├── crates/
│   ├── nethack-types/      enums, structs, constants
│   ├── nethack-data/       static data tables, file format parsers
│   ├── nethack-rng/        ISAAC64 RNG (dual-stream: Core + Display)
│   ├── nethack-game/       game logic (Phases 3–6)
│   └── nethack-tui/        terminal display (Phase 5)
├── src/                    binary crate (main entry point)
├── tools/
│   ├── dgn-comp/           dungeon compiler binary
│   └── lev-comp/           level compiler binary
├── nethack/                C source (git submodule)
└── docs/
```

## Phase Dependencies

```
Phase 1 (Types, Data, RNG)
    └──> Phase 2 (Compilers, Parsers)
            └──> Phase 3 (Map, Level Gen, GameState)
                    ├──> Phase 4 (Monsters, Objects, Player)
                    │       └──> Phase 6 (Core Game Systems)
                    │               └──> Phase 7 (Save/Restore, Polish)
                    └──> Phase 5 (TUI Display)  ← can start in parallel with Phase 4
```

## Estimated Scope Per Phase

| Phase | Approx. C Lines | Key Challenge |
|-------|-----------------|---------------|
| 1 | ~15,000 (headers + static data) | Faithfully transcribing 380+ monster defs, 500+ object defs |
| 2 | ~8,000 (compilers + parsers) | Reverse-engineering binary formats from C code |
| 3 | ~12,000 (level generation) | Complex procedural generation with many special cases |
| 4 | ~10,000 (entity creation) | Object identification and randomization system |
| 5 | ~8,000 (display) | Vision algorithm, glyph system, ratatui layout |
| 6 | ~120,000 (core gameplay) | Sheer volume; deeply interconnected systems |
| 7 | ~19,000 (save/restore + misc) | Serialization of complex graph of objects |

---

## Phase 1: Project Scaffolding, Core Types, Static Data, and RNG

**Goal:** Establish the Cargo workspace, define all fundamental types mirroring
the C headers, port the static data tables (monsters, objects, roles), and
implement the dual-stream ISAAC64 RNG.

This phase produces no runnable game but creates the type foundation that
everything else depends on.

### What Gets Built

- Cargo workspace with crate layout above
- All fundamental enums and structs mirroring the C headers
- Static data tables ported from `monst.c`, `objects.c`, `role.c`
- Dual-stream ISAAC64 RNG (Core for gameplay, Display for visuals)

### Key Types

```rust
// nethack-types

// From permonst.h, monattk.h
pub enum AttackType { None, Claw, Bite, Kick, Butt, Touch, Sting, /* ... */ Weapon, Magic }
pub enum DamageType { Physical, MagicMissile, Fire, Cold, Sleep, /* ... */ }
pub struct Attack { pub attack_type: AttackType, pub damage_type: DamageType, pub dice_num: u8, pub dice_sides: u8 }
pub struct MonsterType { pub name: &'static str, pub symbol: char, pub level: i8, pub speed: i8, pub ac: i8, /* ... */ }

// From objclass.h, obj.h
pub enum ObjectClass { Weapon, Armor, Ring, Amulet, Tool, Food, Potion, Scroll, /* ... */ }
pub enum Material { Liquid, Wax, Veggy, Flesh, Paper, /* ... */ Mineral }
pub struct ObjectType { pub name: &'static str, pub class: ObjectClass, pub material: Material, /* ... */ }

// From you.h
pub enum RoleKind { Archeologist, Barbarian, Caveman, /* ... */ Wizard }
pub enum RaceKind { Human, Elf, Dwarf, Gnome, Orc }

// From rm.h
pub enum LocationType { Stone, VWall, HWall, TLCorner, /* ... */ Cloud }

// From flag.h, context.h
pub struct GameFlags { /* persistent saved-game flags */ }
pub struct GameContext { /* per-turn transient state */ }
```

```rust
// nethack-rng

pub enum RngStream { Core, Display }

pub struct NhRng {
    core: Isaac64Ctx,
    display: Isaac64Ctx,
}

impl NhRng {
    pub fn new(seed: u64) -> Self;
    pub fn rn2(&mut self, stream: RngStream, x: i32) -> i32;  // [0, x)
    pub fn rnd(&mut self, x: i32) -> i32;                      // [1, x]
    pub fn d(&mut self, n: i32, x: i32) -> i32;                // sum of n dice of x sides
    pub fn rnl(&mut self, x: i32, luck: i32) -> i32;           // luck-adjusted
    pub fn rne(&mut self, x: i32, ulevel: i32) -> i32;         // experience-scaled
}
```

The ISAAC64 implementation should be ported directly from the C code (rather
than using `rand_isaac`) so that output can be verified to match the C version
exactly for reproducible testing.

### C Files Ported

- All `include/` headers (type definitions only)
- `src/monst.c` — static `mons[]` array (380+ monster definitions)
- `src/objects.c` — static `objects[]` array (500+ object definitions)
- `src/role.c` — role/race/gender/alignment tables
- `src/isaac64.c`, `include/isaac64.h` — ISAAC64 RNG engine
- `src/rnd.c` — RNG wrapper functions

### Crates

- `thiserror` — custom error types
- `strum` — enum derives (`EnumIter`, `Display`, `EnumString`)
- `color-eyre` / `anyhow` — error propagation with `.context()`
- `log`, `env_logger` — logging
- `serde` — derive `Serialize`/`Deserialize` on all types from day one (used in Phase 7)
- `bitflags` — flag fields

### Testable at End of Phase

- Unit tests for all enum conversions (round-trip from C integer values)
- Spot-checks: specific monsters have expected stats, correct total count
- Spot-checks: specific objects have expected class/weight/cost, correct total count
- RNG: seed with known values, verify output matches C version
- Property tests: `rn2(x)` always in `[0, x)`, `rnd(x)` always in `[1, x]`

---

## Phase 2: Dungeon Compiler, Level Compiler, and Binary Parsers

**Goal:** Parse the binary formats produced by the C compilers, then port the
text parsers to replace the C yacc/lex compilers entirely.

### Sub-phase 2a: Binary Format Parsers (priority)

Parse the binary output of the C compilers so the Rust game can load
C-compiled assets during development.

```rust
// nethack-data

pub struct DungeonTopology {
    pub dungeons: Vec<DungeonDef>,
    pub levels: Vec<SpecialLevelDef>,
    pub branches: Vec<BranchDef>,
}

pub fn parse_dungeon_binary(data: &[u8]) -> Result<DungeonTopology>;

pub struct SpecialLevel {
    pub opcodes: Vec<Opcode>,
}

pub enum Opcode {
    Message(String),
    Monster(MonsterPlacement),
    Object(ObjectPlacement),
    Room(RoomDef),
    // all SPO_* opcodes from sp_lev.h
}

pub fn parse_level_binary(data: &[u8]) -> Result<SpecialLevel>;
```

### Sub-phase 2b: Text Format Parsers

Replace the C yacc/lex compilers with Rust parsers for `dungeon.def` and `.des`
files.

```rust
pub fn parse_dungeon_def(input: &str) -> Result<DungeonTopology>;
pub fn parse_des_file(input: &str) -> Result<SpecialLevel>;
```

### C Files Ported

- `util/dgn_comp.l`, `util/dgn_comp.y`, `util/dgn_main.c` — dungeon compiler
- `util/lev_comp.l`, `util/lev_comp.y`, `util/lev_main.c` — level compiler
- `include/dgn_file.h` — `tmpdungeon`, `tmplevel`, `tmpbranch` structs
- `include/sp_lev.h` — all `SPO_*` opcodes, level data structs
- `src/sp_lev.c` — special level binary format (reading side)
- `src/dlb.c`, `util/dlb_main.c` — data library format (optional, low priority)

### Crates

- `winnow` — binary and text format parsing (zero-copy, efficient)
- `pest` — text format parsing for `.def` and `.des` (grammar-based, optional)
- `clap` — CLI for the compiler tools

### Testable at End of Phase

- Parse C-compiled `dungeon.pdf`, verify topology matches `dungeon.def` expectations
- Parse all C-compiled `.lev` files without error
- Text parsers: parse `dungeon.def`, produce binary output identical to C `dgn_comp`
- Text parsers: parse all 40+ `.des` files without error
- Fuzz tests on binary parsers with random input

---

## Phase 3: Map, Level Generation, and the GameState Struct

**Goal:** Define the central `GameState` struct, implement level generation,
produce the first visually verifiable output (ASCII map dump to stdout).

### Key Types

```rust
pub struct GameState {
    pub player: Player,
    pub dungeon: DungeonState,
    pub current_level: Level,
    pub flags: GameFlags,
    pub context: GameContext,
    pub rng: NhRng,
    pub turn: u64,
    pub monster_moves: u64,
}

pub const COLNO: usize = 80;
pub const ROWNO: usize = 21;

pub struct Level {
    pub locations: [[MapCell; ROWNO]; COLNO],
    pub objects: LevelObjects,
    pub monsters: LevelMonsters,
    pub flags: LevelFlags,
    pub rooms: Vec<Room>,
}

pub struct MapCell {
    pub typ: LocationType,
    pub glyph: Glyph,
    pub seen: u8,
    pub flags: u8,
    pub horizontal: bool,
    pub lit: bool,
    pub was_lit: bool,
    pub room_id: Option<u8>,
}

// Replace C linked lists with Vec + spatial index
pub struct LevelMonsters {
    monsters: Vec<Monster>,
    grid: [[Option<usize>; ROWNO]; COLNO],
}

pub struct LevelObjects {
    objects: Vec<Object>,
    floor: [[Vec<usize>; ROWNO]; COLNO],
}
```

The C code uses linked lists extensively (`fmon`, `fobj`, `nmon`, `nobj`). In
Rust these become `Vec` with a 2D grid for O(1) spatial lookup — avoiding
unsafe linked-list patterns while preserving the access patterns the C code
relies on.

### C Files Ported

- `src/dungeon.c` — `init_dungeons()`, dungeon topology setup
- `src/mklev.c` — `mklev()`, `makecorridors()`, `makerooms()`
- `src/mkroom.c` — special room creation (shops, temples, etc.)
- `src/mkmaze.c` — maze generation
- `src/mkmap.c` — cave-like level generation
- `src/sp_lev.c` — special level bytecode interpreter
- `src/rect.c` — rectangle splitting for room placement
- `src/extralev.c` — extra level utilities

### Testable at End of Phase

- Generate a level for each dungeon depth without panics
- Verify room count and corridor connectivity
- Special levels (castle, oracle, mines) match expected layout shapes
- Statistical tests: generate 1000 levels, check room count distributions
- Load C-compiled `.lev` files, execute bytecode, dump ASCII map

---

## Phase 4: Monsters, Objects, and the Player

**Goal:** Runtime entity creation with enough behavior for a non-interactive
simulation. Monsters can be spawned and placed, objects can be created with
proper randomization, the player can be initialized with starting inventory.

### Key Types

```rust
pub struct Monster {
    pub id: u32,
    pub typ: MonsterTypeId,
    pub pos: Position,
    pub hp: i32,
    pub hp_max: i32,
    pub level: u8,
    pub movement: i16,
    pub status: MonsterStatus,       // bitflags
    pub inventory: Vec<ObjectId>,
    pub strategy: MonsterStrategy,
}

pub struct Object {
    pub id: u32,
    pub typ: ObjectTypeId,
    pub pos: Position,
    pub location: ObjectLocation,
    pub quantity: i32,
    pub enchantment: i8,
    pub buc: BucStatus,
    pub known: KnowledgeFlags,
    pub erosion: ErosionState,
}

pub enum ObjectLocation {
    Free, Floor, Contained(ObjectId), PlayerInventory,
    MonsterInventory(MonsterId), Migrating, Buried, OnBill,
}

pub struct Player {
    pub pos: Position,
    pub role: RoleKind,
    pub race: RaceKind,
    pub gender: GenderKind,
    pub alignment: AlignmentKind,
    pub level: i32,
    pub hp: i32, pub hp_max: i32,
    pub energy: i32, pub energy_max: i32,
    pub attributes: Attributes,
    pub inventory: Vec<ObjectId>,
    pub intrinsics: IntrinsicSet,
    pub hunger: i32,
    pub luck: i8,
    pub ac: i8,
    pub movement: i16,
    pub conducts: Conducts,
    pub achievements: Achievements,
}
```

### C Files Ported

- `src/makemon.c` — monster creation
- `src/mkobj.c` — object creation
- `src/u_init.c` — player initialization
- `src/o_init.c` — object class initialization, description shuffling
- `src/mondata.c` — monster property queries
- `src/artifact.c` — artifact definitions and creation
- `src/attrib.c` — attribute management
- `src/exper.c` — experience and leveling

### Crates

- `bitflags` — `MonsterStatus`, `KnowledgeFlags`, etc.

### Testable at End of Phase

- Create every monster type, verify stats match C definitions
- Create every object type, verify class/weight/cost match
- Initialize player for every role/race/gender/alignment combination
- Verify starting inventory matches C version for each role
- Object identification state machine tests
- BUC status distribution tests

---

## Phase 5: TUI Display and Input

**Goal:** Terminal display using crossterm and ratatui. Show a generated dungeon
level with the player character. Accept basic directional input. This is where
the game becomes interactive for the first time.

### Key Types

```rust
pub trait Display {
    fn init(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn render_map(&mut self, state: &GameState) -> Result<()>;
    fn render_status(&mut self, state: &GameState) -> Result<()>;
    fn show_message(&mut self, msg: &str) -> Result<()>;
    fn get_command(&mut self) -> Result<Command>;
    fn prompt_yn(&mut self, prompt: &str, choices: &str, default: char) -> Result<char>;
    fn show_menu(&mut self, items: &[MenuItem], how: MenuMode) -> Result<Vec<usize>>;
    fn get_line(&mut self, prompt: &str) -> Result<String>;
    fn get_direction(&mut self) -> Result<Option<Direction>>;
}

pub struct TerminalDisplay {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    message_history: VecDeque<String>,
}

pub enum Command {
    Move(Direction), MoveUntilInterrupt(Direction),
    Wait, Search, Quit, Save, Inventory,
    // extended commands...
}
```

### C Files Ported

- `include/winprocs.h` — window interface (becomes `Display` trait)
- `src/display.c` — glyph computation, what to show on screen
- `src/mapglyph.c` — glyph-to-character mapping
- `src/drawing.c` — symbol sets, default symbols
- `src/vision.c` — field-of-view calculation
- `src/botl.c` — bottom status line
- `src/pline.c` — message output with repeat prevention
- `win/tty/wintty.c` — reference for TTY-specific behavior (not ported directly)

### Crates

- `crossterm` — terminal I/O, raw mode, events
- `ratatui` — TUI widgets (map, status bar, message area)
- `tokio` — async event loop (optional; synchronous crossterm events may suffice initially)

### Testable at End of Phase

- Render a level to terminal, visually verify
- FOV: place player in a room, verify visibility is correct
- Memory: move away, verify previously-seen tiles display correctly
- Input mapping: vi keys, numpad, arrow keys all produce correct `Command`
- Message deduplication: "x - 3 times", history recall
- Status bar: all fields update when player state changes

---

## Phase 6: Core Game Systems

**Goal:** Implement the core gameplay mechanics. This is the largest phase
(~120k LOC equivalent) and should be subdivided into milestones.

### Sub-phases

**6a: Movement and Game Loop**

The turn-based loop with movement point system.

```rust
impl GameState {
    pub fn run_turn(&mut self, display: &mut dyn Display) -> Result<TurnResult> {
        self.player.movement -= NORMAL_SPEED;

        while self.player.movement < NORMAL_SPEED {
            self.run_monster_turns()?;
            if self.player.movement >= NORMAL_SPEED { break; }
            self.end_of_round()?;
            self.allocate_movement()?;
        }

        self.pre_player_turn(display)?;
        let cmd = display.get_command()?;
        self.execute_command(cmd, display)?;
        self.post_player_turn(display)?;

        Ok(TurnResult::Continue)
    }
}
```

C files: `allmain.c`, `hack.c`, `cmd.c`

**6b: Combat**

Player-vs-monster, monster-vs-player, monster-vs-monster. To-hit rolls, damage
dice, special effects, resistances.

C files: `uhitm.c`, `mhitu.c`, `mhitm.c`, `weapon.c`, `wield.c`, `worn.c`

**6c: Items**

Inventory management, wielding, wearing, eating, drinking, picking up, dropping.

C files: `invent.c`, `do_wear.c`, `eat.c`, `pickup.c`, `do.c`, `dothrow.c`

**6d: Magic**

Spell learning and casting (knowledge decay, success calculation), wand
zapping, scroll reading, potion quaffing, tool applying.

C files: `spell.c`, `zap.c`, `read.c`, `potion.c`, `apply.c`

**6e: Monster AI**

Movement, pathfinding, item usage, spell casting, pet behavior.

C files: `monmove.c`, `muse.c`, `mcast.c`, `mthrowu.c`, `dog.c`, `dogmove.c`

**6f: World Interactions**

Traps, shops, fountains, altars, thrones, digging, locks, teleportation,
prayer, engravings, explosions, light sources, polymorphing, lycanthropy,
riding, regions, ball-and-chain, long worms, sounds.

C files: `trap.c`, `shk.c`, `shknam.c`, `fountain.c`, `sit.c`, `pray.c`,
`priest.c`, `dig.c`, `lock.c`, `engrave.c`, `explode.c`, `light.c`,
`detect.c`, `teleport.c`, `polyself.c`, `were.c`, `steal.c`, `steed.c`,
`music.c`, `sounds.c`, `region.c`, `ball.c`, `worm.c`

### Testable at End of Phase

- Deterministic replay: given a seed and command sequence, verify game state
- Combat math: verify to-hit, damage, and special effects for specific scenarios
- AI behavior: verify monster pathfinding, fleeing, item usage
- Shopkeeper: buy/sell flow, theft consequences
- Trap effects: verify each trap type
- Spell system: verify energy costs, success rates

---

## Phase 7: Save/Restore, Endgame, and Polish

**Goal:** Save/restore behind an abstract trait, bones files, scoring, endgame
sequence (Elemental Planes, Astral Plane, ascension), options, quest text, data
file loading, death screen.

### Key Types

```rust
pub trait SaveFormat: Send + Sync {
    fn save(&self, state: &GameState, writer: &mut dyn Write) -> Result<()>;
    fn restore(&self, reader: &mut dyn Read) -> Result<GameState>;
    fn format_name(&self) -> &str;
}

pub struct BincodeSaveFormat;
impl SaveFormat for BincodeSaveFormat { /* ... */ }

// Future: C-compatible format
// pub struct ClassicSaveFormat;
// impl SaveFormat for ClassicSaveFormat { /* ... */ }
```

All `GameState` types must derive `serde::Serialize` and
`serde::Deserialize` — added from Phase 1 onward to avoid a massive
retrofit.

### C Files Ported

- `src/save.c`, `src/restore.c` — persistence
- `src/bones.c` — bones files
- `src/topten.c` — high scores
- `src/end.c` — game over
- `src/rip.c` — tombstone display
- `src/options.c` — game options
- `src/questpgr.c` — quest text
- `src/rumors.c` — rumors, oracles
- `src/pager.c` — data file lookups
- `src/files.c` — file I/O utilities
- `src/version.c` — version info

### Crates

- `serde` — serialization framework
- `bincode` — binary serialization (primary format)
- `rmp-serde` — MessagePack serialization (alternative format)

### Testable at End of Phase

- Save a game, restore it, verify state is identical
- Round-trip test with both bincode and MessagePack
- Bones file generation and loading
- Full seeded game: start, play several levels, save, restore, continue
- Endgame sequence smoke test

---

## Cross-Cutting Concerns

### Error Handling

All public functions return `Result<T>` using `color-eyre`. Every `?` operator
has a `.context("describing what we were doing")` call. Custom error types use
`thiserror`.

### Testing

Every phase includes unit tests. An integration test crate grows with each
phase. Deterministic RNG seeding enables reproducible tests.

### Serde Derives

Add `#[derive(Serialize, Deserialize)]` to all game state types from Phase 1
onward, even though save/restore is not implemented until Phase 7.

### Logging

Use the `log` crate throughout. Debug-level for game logic decisions,
trace-level for RNG rolls.
