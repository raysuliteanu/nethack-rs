# Phase 2: Dungeon Definition & Level Compiler (Text Parsers)

## Context

Phase 1 is complete — the workspace has `nethack-types` (enums, structs,
bitflags), `nethack-data` (384 monsters, 454 objects), and `nethack-rng`
(ISAAC64). Phase 2 parses the two text formats that define dungeon structure and
special level layouts:

- `nethack/dat/dungeon.def` — 9 dungeons with branches and special levels
- `nethack/dat/*.des` — 24 files defining special level layouts as a scripting
  language compiled to a stack-based bytecode (75 opcodes)

We are building **text parsers only** (no C binary format parsing). The C
compilers (`dgn_comp`, `lev_comp`) become irrelevant.

## Implementation Steps

### Step 1: Add `winnow` + Dungeon Topology Types

Add `winnow = "0.7"` to workspace dependencies and to `nethack-data`.

Create `crates/nethack-types/src/dungeon.rs` with:

```rust
pub struct DungeonTopology { pub dungeons: Vec<DungeonDef> }

pub struct DungeonDef {
    name, boneschar, base, rand: i16,
    flags: DungeonFlags, entry: i16,
    protofile: Option<String>,
    levels: Vec<LevelDef>, branches: Vec<BranchDef>,
}

pub struct LevelDef {
    name, boneschar, chain: Option<String>,
    offset_base, offset_rand: i16, rndlevs: u8, chance: u8,
    flags: DungeonFlags,
}

pub struct BranchDef {
    name, chain: Option<String>,
    offset_base, offset_rand: i16,
    branch_type: BranchType, direction: Option<BranchDirection>,
}

pub struct DungeonFlags { town, hellish, maze_like, rogue_like: bool, align: DungeonAlignment }
pub enum DungeonAlignment { Unaligned, Lawful, Neutral, Chaotic, Noalign }
pub enum BranchType { Stair, NoUp, NoDown, Portal }
pub enum BranchDirection { Up, Down }
```

Source reference: `nethack/include/dgn_file.h` (struct tmpdungeon/tmplevel/tmpbranch),
flags at lines 43–64.

**Verify:** `cargo check -p nethack-types`

### Step 2: Special Level Opcode Types

Create `crates/nethack-types/src/sp_lev.rs` with:

**`SpOpcode` enum** — 76 variants (0–75) with `#[repr(u8)]` matching C's
`enum opcode_defs` in `nethack/include/sp_lev.h:60-139`. Key groups:
- Placement: Monster(2), Object(3), Trap(14), Door(7), Stair(8), Altar(10)...
- Map: Map(23), InitLevel(58), LevelFlags(59), Terrain(35), ReplaceTerrain(36)
- Control: Cmp(27), Jmp(28), Jl–Jne(29–34), Exit(37)
- Stack: Push(40), Pop(41), Copy(51), Rn2(42), Dice(62), math ops(45–50)
- Variables: VarInit(60), ShuffleArray(61)
- Functions: FramePush(54), FramePop(55), Call(56), Return(57)
- Selection: SelAdd(63)–SelComplement(75)
- Rooms: Room(5), Subroom(6), EndRoom(38), RoomDoor(24), Corridor(16)

**`SpOperand` enum** — typed push data (matches SPOVAR_* from sp_lev.h:206-221):
- `Int(i64)`, `String(String)`, `Variable(String)`,
  `Coord { x, y, is_random, flags }`, `Region { x1, y1, x2, y2 }`,
  `MapChar { typ, lit }`, `Monst { class, id }`, `Obj { class, id }`,
  `Sel(Vec<u8>)`

**`SpLevOpcode`** — `{ opcode: SpOpcode, operand: Option<SpOperand> }`

**`LevelFlags`** — bitflags matching sp_lev.h:20-34 (NOTELEPORT through
CHECK_INACCESSIBLES, 13 flags).

**`LvlInitStyle`** — enum (None, SolidFill, MazeGrid, Mines, Rogue).

**`SpMonVarFlag`** / **`SpObjVarFlag`** — enums matching sp_lev.h:148-188.

**`SpecialLevel`** — `{ name: String, opcodes: Vec<SpLevOpcode> }`

**`DesFile`** — `{ levels: Vec<SpecialLevel> }` (a .des file can define multiple levels)

**Verify:** `cargo check -p nethack-types`

### Step 3: Dungeon Definition Parser

Create `crates/nethack-data/src/dungeon_parser.rs`.

The `dungeon.def` format is line-oriented (reference: `nethack/util/dgn_comp.y`
and `nethack/dat/dungeon.def`). Each line starts with a keyword followed by `:`
and arguments:

| Keyword | Example | Notes |
|---------|---------|-------|
| DUNGEON | `"name" "D" (25, 5)` | Starts new dungeon block |
| DESCRIPTION | `mazelike` | hellish/mazelike/roguelike/town |
| ALIGNMENT | `neutral` | lawful/neutral/chaotic/unaligned/noalign |
| ENTRY | `-1` | -1=bottom, -2=second from bottom |
| PROTOFILE | `"tower"` | Level file prefix |
| LEVEL | `"rogue" "R" @ (15, 4)` | Optional trailing chance |
| RNDLEVEL | `"bigrm" "B" @ (10, 3) 40 10` | Extra count + optional chance |
| CHAINLEVEL | `"wizard2" "X" "wizard1" + (1, 0)` | Offset from parent level |
| BRANCH | `"The Gnomish Mines" @ (2, 3)` | Optional type + direction |
| CHAINBRANCH | `"Sokoban" "oracle" + (1, 0) up` | Offset from parent level |
| LEVELDESC | `roguelike` | Modifies most-recent level |
| LEVALIGN | `chaotic` | Modifies most-recent level |

Parser approach: strip `#` comments and blank lines, then winnow or
line-by-line keyword dispatch. `DUNGEON` starts a new dungeon; subsequent
keywords modify the current dungeon or its most-recent level.

Tests:
- Parse actual `dungeon.def` → 9 dungeons
- Doom: base=25, rand=5, unaligned
- Gehennom: hellish + mazelike
- Sokoban: entry=-1, neutral, 4 RNDLEVEL entries
- Elemental Planes: entry=-2, 6 levels
- Vlad's Tower: protofile="tower"
- All CHAIN* references resolve to valid parent names
- Error cases: empty input, unknown keyword

**Verify:** `cargo test -p nethack-data -- dungeon`

### Step 4: .des Lexer

Create `crates/nethack-data/src/des_lexer.rs`.

Tokenizer for the `.des` scripting language (reference: `nethack/util/lev_comp.l`).
Handles:
- `#` line comments
- Keywords (MAZE, LEVEL, FLAGS, MAP/ENDMAP, MONSTER, OBJECT, etc.)
- Quoted strings `"text"`, character literals `'x'`
- Integers, dice notation `NdM`, percentages `N%`
- Variables `$name`, array access `$name[idx]`
- Punctuation: `:`, `,`, `(`, `)`, `{`, `}`, `[`, `]`, `+`, `-`, `--`
- MAP block: verbatim text capture between MAP and ENDMAP lines

The lexer produces `Vec<Token>` with source location tracking for error messages.

### Step 5: .des Parser (Opcode Emitter)

Create `crates/nethack-data/src/des_parser.rs`.

This is the largest piece. The parser consumes tokens and emits
`Vec<SpLevOpcode>` matching C `lev_comp`'s output semantics (reference:
`nethack/util/lev_comp.y`, 2721 lines of yacc). The parser maintains:
- Opcode accumulator per level
- Variable/type symbol table (per level)
- Function table
- Block nesting stack (IF/ELSE, SWITCH/CASE, FOR/LOOP, CONTAINER)

**Compilation patterns** (statement → opcodes):

| Statement | Opcodes emitted |
|-----------|----------------|
| `MAZE:"name",fill` | Reset accumulator; set MAZELEVEL flag |
| `FLAGS:noteleport,...` | `PUSH(flags_bits)` + `LEVEL_FLAGS` |
| `INIT_MAP:style,...` | PUSHes for each param + `INITLEVEL` |
| `MAP...ENDMAP` | `PUSH(maptext)` + `PUSH(halign)` + `PUSH(valign)` + `MAP` |
| `MONSTER:(cls,"nm"),coord` | Modifier pushes + `PUSH(monst)` + `PUSH(coord)` + `PUSH(count)` + `MONSTER` |
| `OBJECT:(cls,"nm"),coord` | Similar with obj pushes + `OBJECT` |
| `DOOR:state,(x,y)` | `PUSH(state)` + `PUSH(coord)` + `DOOR` |
| `IF [pct] {...}` | `PUSH(pct)` + `RN2` + `PUSH(0)` + `CMP` + `JNE→else` + body + `JMP→end` |
| `FOR $v=a TO b {...}` | `VAR_INIT` + loop body + `INC` + `CMP` + `JLE→top` |
| `$var = value` | `PUSH(value)` + `VAR_INIT(name)` |
| `SHUFFLE:$arr` | `PUSH(Variable(name))` + `SHUFFLE_ARRAY` |
| `ROOM/SUBROOM` | Param pushes + `ROOM`/`SUBROOM`, body, `ENDROOM` |
| `TERRAIN:sel,type` | Selection opcodes + `PUSH(mapchar)` + `TERRAIN` |

**Implementation order** (incremental, test against real files):

1. **Core statements:** MAZE/LEVEL definition, FLAGS, MAP/ENDMAP/NOMAP,
   INIT_MAP, GEOMETRY — target: `mines.des` (simplest maze file)
2. **Placement:** MONSTER, OBJECT, TRAP, DOOR, STAIR, ALTAR, FOUNTAIN, SINK,
   POOL, GOLD, ENGRAVING, DRAWBRIDGE, MAZEWALK, GRAVE, LADDER,
   NON_DIGGABLE, NON_PASSWALL, WALLIFY, MINERALIZE — target: `castle.des`
3. **Rooms:** ROOM, SUBROOM, ENDROOM, ROOMDOOR, CORRIDOR, REGION —
   target: `oracle.des`
4. **Control flow:** IF/ELSE, SWITCH/CASE/BREAK, FOR/LOOP, variables,
   SHUFFLE, FUNCTION — target: `bigroom.des`, `gehennom.des`
5. **Advanced:** CONTAINER/POP_CONTAINER, TERRAIN with selection expressions,
   REPLACE_TERRAIN, levregion syntax, TELEPORT_REGION, montype/m_feature
   modifiers, align[N] syntax — target: all 24 files

Tests:
- **Acceptance:** All 24 `.des` files parse without error
- Spot checks: `castle.des` has MAP + DOOR + DRAWBRIDGE opcodes
- `bigroom.des` produces 10 levels
- `sokoban.des` has PREMAPPED flag
- Control flow: IF/ELSE produces correct JMP/JNE structure
- Error cases: unterminated MAP, undefined variable, mismatched braces

**Verify:** `cargo test -p nethack-data`

### Step 6: Integration Tests + Final Verification

Create `crates/nethack-data/tests/parse_all.rs`:
- Parse `dungeon.def` → 9 dungeons
- Parse all 24 `.des` files → each produces ≥1 level with ≥1 opcode

Full workspace:
```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --check
```

## Files to Create/Modify

| File | Action |
|------|--------|
| `Cargo.toml` (workspace) | Add `winnow = "0.7"` |
| `crates/nethack-data/Cargo.toml` | Add `winnow.workspace = true` |
| `crates/nethack-types/src/dungeon.rs` | New: dungeon topology types |
| `crates/nethack-types/src/sp_lev.rs` | New: opcode/operand/level types |
| `crates/nethack-types/src/lib.rs` | Add `pub mod dungeon; pub mod sp_lev;` + re-exports |
| `crates/nethack-data/src/dungeon_parser.rs` | New: dungeon.def parser |
| `crates/nethack-data/src/des_lexer.rs` | New: .des tokenizer |
| `crates/nethack-data/src/des_parser.rs` | New: .des parser/compiler |
| `crates/nethack-data/src/lib.rs` | Add parser module declarations |
| `crates/nethack-data/tests/parse_all.rs` | New: integration tests |

## Key C Reference Files

| File | Purpose |
|------|---------|
| `nethack/include/dgn_file.h` | Dungeon struct layouts and flag constants |
| `nethack/include/sp_lev.h` | Opcode enum, SPOVAR_* types, packing macros |
| `nethack/util/dgn_comp.y` | Dungeon compiler grammar (output format) |
| `nethack/util/lev_comp.y` | Level compiler grammar (2721 lines, the primary reference) |
| `nethack/util/lev_comp.l` | Level compiler lexer (token definitions) |
| `nethack/util/lev_main.c` | Binary writer (defines opcode serialization order) |
| `nethack/src/sp_lev.c` | Runtime interpreter (defines how opcodes consume stack) |
| `nethack/dat/dungeon.def` | Input: dungeon definitions (9 dungeons) |
| `nethack/dat/*.des` | Input: 24 special level definition files |

## Dependency Order

```
Step 1: winnow + dungeon types (nethack-types)
Step 2: opcode types (nethack-types)
    ↓
Step 3: dungeon.def parser (nethack-data)
Step 4: .des lexer (nethack-data)
    ↓
Step 5: .des parser (nethack-data, incremental: mines → castle → oracle → all)
    ↓
Step 6: integration tests
```

Steps 1–2 are prerequisites. Steps 3 and 4 are independent of each other.
Step 5 depends on both Steps 2 and 4.
