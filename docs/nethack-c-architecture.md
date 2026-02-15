# NetHack 3.6 C Codebase Architecture

This document describes the architecture of the NetHack 3.6 C codebase as it
exists in the `nethack/` submodule. Its purpose is to serve as a reference for
the Rust port.

## Overview

NetHack 3.6 is approximately 192,000 lines of C across 108 source files and 96
headers. The game is a single-player, turn-based dungeon crawler where a hero
descends through procedurally generated dungeon levels, fights monsters,
collects items, and ultimately seeks the Amulet of Yendor.

## Project Structure

```
nethack/
├── dat/            data files: dungeon definitions, level descriptions,
│                   text databases (rumors, oracles, epitaphs, help)
├── doc/            documentation (guidebook, changelog, man pages)
├── include/        96 header files defining all types and constants
├── src/            108 core source files (~180k lines)
├── sys/            platform-specific code (unix, share, vms, msdos, winnt, etc.)
├── util/           build tools (makedefs, dgn_comp, lev_comp, dlb, recover)
├── win/            window port implementations (tty, curses, X11, Qt, gnome, chain)
└── dat/*.des       special level description files compiled by lev_comp
```

## Build Pipeline

The NetHack build involves several code-generation steps before the main game
can be compiled:

1. **makedefs** (`util/makedefs.c`) — generates headers from source data:
   - `pm.h` — monster type enum (from `monst.c`)
   - `onames.h` — object name enum (from `objects.c`)
   - `date.h` — build timestamp and copyright
   - `vis_tab.h` — vision table
   - Various data files (`data`, `rumors`, `oracles`, etc.)

2. **dgn_comp** (`util/dgn_comp.y`, `util/dgn_comp.l`) — dungeon compiler:
   - Reads `dat/dungeon.def` (human-readable dungeon topology)
   - Outputs `dungeon.pdf` (binary dungeon topology)
   - yacc/lex-based parser

3. **lev_comp** (`util/lev_comp.y`, `util/lev_comp.l`) — level compiler:
   - Reads `dat/*.des` files (human-readable special level descriptions)
   - Outputs `.lev` files (binary bytecode for the special level interpreter)
   - yacc/lex-based parser
   - 40+ `.des` files define special levels (quest levels, mines, castle, etc.)

4. **dlb** (`util/dlb_main.c`) — data librarian:
   - Packages data files into a single archive (`nhdat`)
   - Optional; files can be loaded individually

5. **Main game** — compiled and linked against platform-specific code and
   a chosen window port.

## Core Data Structures

### Player Character — `struct you` (`include/you.h`)

The player is represented by the global `u` variable of type `struct you`,
containing approximately 120 fields organized into groups:

- **Position and movement**: `ux`, `uy` (map coords), `dx`, `dy`, `dz`
  (move direction), `tx`, `ty` (travel destination), `uz` (dungeon level)
- **Vital stats**: `uhp`/`uhpmax`, `uen`/`uenmax`, `ulevel`/`ulevelmax`,
  `uac`, `uluck`/`moreluck`
- **Attributes**: `acurr`, `aexe`, `abon`, `amax`, `atemp`, `atime` — six
  parallel arrays for the six attributes (St, Dx, Co, In, Wi, Ch)
- **Intrinsic properties**: `uprops[LAST_PROP+1]` — array of ~80 properties
  (telepathy, flying, fire resistance, etc.), each storing timeout and
  source information
- **Polymorph state**: `umonnum`, `umonster`, `mh`/`mhmax`, `mtimedone`
- **Hunger**: `uhunger` (numeric), `uhs` (hunger state enum)
- **Combat**: `uhitinc`, `udaminc`, `twoweap`, weapon skill arrays
- **Room tracking**: `urooms[5]`, `ushops[5]`, `ushops_entered[5]` — which
  rooms the player is in or has entered this turn
- **Alignment and religion**: `ualign`, `ualignbase[CONVERT]`, `ugangr`,
  `ugifts`, `ublessed`
- **Conducts**: 12 `long` fields tracking pacifist, vegetarian, atheist, etc.
- **Achievements and events**: `uachieve`, `uevent`, `uhave` — bitfields
- **Special states**: `utrap`/`utraptype`, `uswallow`, `uinwater`, `uburied`,
  `ustuck`, `usteed`

### Monster Instance — `struct monst` (`include/monst.h`)

Each monster on the current level is a `struct monst` with approximately 45
fields:

- **Type**: `data` (pointer to static `struct permonst`), `mnum` (index into
  `mons[]`), `m_id` (unique ID)
- **Position**: `mx`, `my`, plus `mux`/`muy` (where the monster thinks the
  player is)
- **Health**: `mhp`, `mhpmax`, `m_lev` (difficulty level)
- **Movement**: `movement` (movement points, same system as player)
- **AI state**: `mstrategy` (32-bit field encoding goal type and target
  coordinates), `mtrack[4]` (recent positions for pathfinding)
- **Status bitfields** (21 flags across 4 packed words): `female`, `minvis`,
  `mcan` (cancelled), `mburied`, `msleeping`, `mblinded`, `mstun`,
  `mfrozen`/`mcanmove`, `mconf`, `mpeaceful`, `mtrapped`, `mleashed`,
  `mflee`/`mfleetim`, `mrevived`, `mcloned`
- **Special roles**: `isshk` (shopkeeper), `ispriest`, `isminion`, `isgd`
  (guard), `iswiz` (Wizard of Yendor)
- **Inventory**: `minvent` (linked list of objects), `mw` (wielded weapon),
  `misc_worn_check` (worn items bitmask)
- **Appearance**: `mappearance`, `m_ap_type` (for mimics)
- **Extended data**: `mextra` (pointer to optional data: name, shopkeeper
  info, priest info, etc.)

Monsters on a level form a singly-linked list via the `nmon` pointer, anchored
at the global `fmon`.

### Monster Type — `struct permonst` (`include/permonst.h`)

Static monster definitions live in `mons[]` (defined in `src/monst.c`, 380+
entries). Each `struct permonst` has 13 fields:

- `mname` — display name
- `mlet` — map symbol character
- `mlevel`, `difficulty` — level and toughness
- `mmove` — base movement speed
- `ac` — armor class
- `mr` — magic resistance (0–100)
- `maligntyp` — alignment
- `geno` — generation/genocide flags
- `mattk[6]` — up to 6 attacks, each with attack type, damage type, and
  damage dice (`damn`×`damd`)
- `cwt`, `cnutrit` — corpse weight and nutrition
- `msound` — noise type (6 bits)
- `msize` — physical size (3 bits)
- `mresists`, `mconveys` — resistance bitmasks
- `mflags1`, `mflags2`, `mflags3` — behavioral flags

There are 30+ attack types (`AT_CLAW`, `AT_BITE`, `AT_KICK`, `AT_WEAP`,
`AT_MAGC`, `AT_GAZE`, etc.) and 42+ damage types (`AD_PHYS`, `AD_FIRE`,
`AD_COLD`, `AD_MAGM`, `AD_DRST`, `AD_DISN`, etc.) defined in
`include/monattk.h`.

### Object Instance — `struct obj` (`include/obj.h`)

Each object in the game is a `struct obj` with approximately 35 fields:

- **Type**: `otyp` (index into `objects[]`), `oclass` (object class),
  `oartifact` (artifact index)
- **Position**: `ox`, `oy`
- **Location state**: `where` — one of 8 states: `OBJ_FREE`, `OBJ_FLOOR`,
  `OBJ_CONTAINED`, `OBJ_INVENT`, `OBJ_MINVENT`, `OBJ_MIGRATING`,
  `OBJ_BURIED`, `OBJ_ONBILL`
- **Quantity and weight**: `quan`, `owt`
- **Enchantment**: `spe` (enchantment level, charges, or special value)
- **BUC status**: `cursed`, `blessed` (1 bit each; neither = uncursed)
- **Knowledge**: `known`, `dknown`, `bknown`, `rknown` — what the player
  knows about the object
- **Condition**: `oeroded`/`oeroded2` (2 bits each), `oerodeproof`,
  `olocked`, `obroken`, `otrapped`, `lamplit`, `greased`
- **Linking**: `nobj` (next object in chain), `cobj` (contents if container)
- **Extended data**: `oextra` (optional name, attached monster, etc.)

Objects form linked lists: `invent` (player inventory), `fobj` (floor
objects), `level.buriedobjlist`, and per-monster `minvent`.

### Map Cell — `struct rm` (`include/rm.h`)

Each cell of the dungeon map is a `struct rm` with 8 fields packed into
a few bytes:

- `glyph` — what the hero currently sees (or remembers)
- `typ` — actual terrain type (36 types: `STONE`, `VWALL`, `HWALL`,
  `DOOR`, `CORR`, `ROOM`, `STAIRS`, `POOL`, `LAVAPOOL`, `FOUNTAIN`,
  `THRONE`, `ALTAR`, `ICE`, `AIR`, `CLOUD`, etc.)
- `seenv` — visibility octant bitmask (8 bits for 8 directions)
- `flags` — 5 bits of terrain-specific information (door state, altar
  type, etc.)
- `horizontal` — wall orientation (1 bit)
- `lit`, `waslit` — illumination state (1 bit each)
- `roomno` — room number (6 bits)
- `edge` — room boundary flag (1 bit)

The level map is a fixed 80×21 grid (`COLNO`×`ROWNO`).

There are 96 screen symbols (`S_stone` through `S_explode9`) covering dungeon
features, 23 trap types, 34 visual effects, and 9 explosion tiles.

### Game Flags — `struct flag` (`include/flag.h`)

Persistent game settings stored in `flags` (~60 fields):

- **Display**: `dark_room`, `lit_corridor`, `time`, `showexp`, `verbose`,
  `tombstone`, `menu_style`
- **Automation**: `autodig`, `autoquiver`, `autoopen`, `pickup`
- **Game modes**: `debug` (wizard mode), `explore` (discovery mode)
- **Character identity**: `female`, `initrole`, `initrace`, `initgend`,
  `initalign`
- **Inventory**: `inv_order[MAXOCLASSES]`, `pickup_types`, `sortloot`
- **Scoring**: `end_own`, `end_top`, `end_around`
- **Window capabilities**: 40+ `iflags` fields for color, fonts, alignment,
  mouse support, etc.

### Game Context — `struct context_info` (`include/context.h`)

Per-session transient state stored in `context` with 17 nested structures and
~16 direct fields:

- **Nested activity state**: `dig_info`, `tin_info`, `book_info`,
  `takeoff_info`, `victual_info` — track multi-turn activities
- **Monster tracking**: `warntype_info`, `polearm_info`
- **Direct fields**: `ident` (next monster ID), `run` (running mode),
  `mon_moving` (whose turn), `move` (time passing), `botl`/`botlx` (status
  line update flags), `travel`/`travel1` (autotravel state)

### Dungeon Topology — `struct dungeon` (`include/dungeon.h`)

The dungeon is defined by `dungeon.def` and stored at runtime in the
`dungeons[]` array. NetHack 3.6 defines 9 dungeons:

1. **The Dungeons of Doom** — 25 levels, main dungeon (unaligned)
2. **Gehennom** — 20 levels, hellish mazelike
3. **The Gnomish Mines** — 8 levels, lawful mazelike
4. **The Quest** — 5 levels, role-specific
5. **Sokoban** — 4 levels, neutral mazelike puzzles
6. **Fort Ludios** — 1 level, vault
7. **Vlad's Tower** — 3 levels, chaotic mazelike
8. **The Elemental Planes** — 6 levels, endgame
9. Implicit connections via branches

Key structures:

- `d_level` — dungeon number + level within dungeon (2 bytes)
- `d_flags` — `town`, `hellish`, `maze_like`, `rogue_like`, `align`
- `s_level` — special level definition (prototype file, bones ID, random count)
- `stairway` — position and destination
- `branch` — connection between two dungeons (types: `BR_STAIR`,
  `BR_NO_END1`, `BR_NO_END2`, `BR_PORTAL`)

## The Game Loop

The game loop lives in `moveloop()` in `src/allmain.c`. NetHack uses a
**movement point system** where both the player and monsters accumulate
movement points each turn, and can act when they have enough.

```
NORMAL_SPEED = 12 movement points per turn

for (;;) {
    get_nh_event()                  // poll for input/events

    if (context.move) {             // time is passing
        youmonst.movement -= NORMAL_SPEED

        do {
            // --- Monster turn phase ---
            context.mon_moving = TRUE
            do {
                monscanmove = movemon()     // each monster acts
            } while (monscanmove && hero out of movement)
            context.mon_moving = FALSE

            // --- Once-per-round phase ---
            if (!monscanmove && youmonst.movement < NORMAL_SPEED) {
                mcalcdistress()             // monster status effects
                nh_timeout()                // intrinsic timeouts
                run_regions()               // gas clouds, etc.

                // regeneration, prayer, encumbrance damage
                // energy regen, teleport checks, polymorph chance
                // hunger, spells, exercise, vault, amulet effects
                // demigod intervention, attribute restoration
                // environment effects (water, lava, etc.)

                // allocate new movement points
                youmonst.movement += moveamt
                monstermoves++
                moves++
            }
        } while (youmonst.movement < NORMAL_SPEED)

        // --- Player's turn ---
        recalc AC, vision, status
        get command input
        execute command via rhack()

        // handle level changes, status updates, display refresh
    }
}
```

### Movement Speed

- Base speed comes from `youmonst.data->mmove` (12 for human)
- Fast/Very Fast intrinsics occasionally grant bonus actions
- Encumbrance reduces movement by fractions (1/4, 1/2, 3/4, 7/8)
- Monsters use the same system: `mon->movement` accumulates and is spent

### Turn Processing Order

Each "turn" processes:

1. All monsters that have accumulated enough movement points
2. Once-per-round bookkeeping (timeouts, regeneration, environmental effects)
3. Movement point allocation for the next round
4. Player command input and execution

## Command System

The command system is in `src/cmd.c` (212KB, the largest single file). Commands
are dispatched by single characters (e.g., `o` to open, `z` to zap) or
extended commands (prefixed with `#`, e.g., `#pray`).

- Command handlers follow the naming convention `do<action>()` — `domove()`,
  `doopen()`, `dozap()`, `doeat()`, `doapply()`, etc.
- Each handler returns 1 if time passed (a turn was consumed) or 0 if not
- Numeric prefixes (`5s` = search 5 times) are handled via the `multi` counter
- Movement uses vi keys (`hjklyubn`), numpad, or arrow keys
- Running (shift+direction or `g`+direction) continues until interrupted
- Travel (`_`) uses automatic pathfinding

## Display System

The display system has three layers:

### 1. Vision Layer (`src/vision.c`)

Field-of-view calculation using octant-based ray casting. Determines which
map cells the player can currently see. The `seenv` bitfield in each `struct
rm` tracks which of 8 octants can see that cell.

### 2. Display Logic (`src/display.c`)

Decides what glyph to show at each position:

- If the cell is currently visible: show the actual contents
- If previously seen but not visible: show the remembered glyph
- If never seen: show stone/blank

Glyph computation considers: terrain type, objects on the floor, monsters
(visible or detected), traps (known or unknown), dungeon features.

### 3. Rendering (`src/mapglyph.c`, `src/drawing.c`)

Maps glyphs to actual characters and colors for the chosen window port.
The glyph system is an integer encoding:

```
Glyph ranges:
  [0, GLYPH_MON_OFF)          — monster glyphs
  [GLYPH_PET_OFF, ...)        — pet glyphs
  [GLYPH_INVIS_OFF, ...)      — invisible monster
  [GLYPH_DETECT_OFF, ...)     — detected monster
  [GLYPH_BODY_OFF, ...)       — corpse/body
  [GLYPH_RIDDEN_OFF, ...)     — ridden monster
  [GLYPH_OBJ_OFF, ...)        — object glyphs
  [GLYPH_CMAP_OFF, ...)       — dungeon feature glyphs
  [GLYPH_EXPLODE_OFF, ...)    — explosion effects
  [GLYPH_ZAP_OFF, ...)        — beam effects
  [GLYPH_SWALLOW_OFF, ...)    — swallowed view
  [GLYPH_WARNING_OFF, ...)    — warning symbols
  [GLYPH_STATUE_OFF, ...)     — statue glyphs
```

### Memory System

The player remembers what they have seen. Each map cell's `glyph` field stores
the last-seen contents. When the cell is out of sight, the memory glyph is
displayed instead of the actual contents. This is how explored but non-visible
corridors still show on the map.

## Windowing Abstraction

The window system is defined by `struct window_procs` in `include/winprocs.h`
— a vtable of **52 function pointers** organized by category:

| Category                   | Count | Examples                                                                    |
| -------------------------- | ----- | --------------------------------------------------------------------------- |
| Initialization & lifecycle | 3     | `init_nhwindows`, `player_selection`, `exit_nhwindows`                      |
| Window management          | 5     | `create_nhwindow`, `clear_nhwindow`, `display_nhwindow`, `destroy_nhwindow` |
| Text output                | 3     | `putstr`, `putmixed`, `display_file`                                        |
| Menu system                | 5     | `start_menu`, `add_menu`, `end_menu`, `select_menu`                         |
| Input & events             | 7     | `nhgetch`, `nh_poskey`, `getlin`, `yn_function`, `get_ext_cmd`              |
| Graphics                   | 4     | `print_glyph`, `raw_print`                                                  |
| Status                     | 5     | `status_init`, `status_update`, `update_inventory`                          |
| Miscellaneous              | 6     | `nhbell`, `delay_output`, `doprev_message`                                  |
| Death screen               | 1     | `outrip`                                                                    |
| Platform-specific          | 5+    | `number_pad`, `change_color`, `set_font_name`                               |

Window types used by the game:

- `WIN_MESSAGE` — scrolling message area (top of screen)
- `WIN_MAP` — the dungeon map (center)
- `WIN_STATUS` — status bars (bottom)
- `WIN_INVENT` — inventory display (on demand)

Each port (TTY, curses, X11, Qt, etc.) provides its own implementation of this
vtable. The TTY port (`win/tty/wintty.c`) is the reference implementation.

Capability flags (`wincap`, `wincap2`) let the core query what features a port
supports (color, mouse, font selection, etc.).

## Monster System

### Static Definitions (`src/monst.c`)

The `mons[]` array contains 380+ `struct permonst` entries, one per monster
type. Each defines: name, symbol, level, speed, AC, magic resistance, attacks
(up to 6), corpse properties, resistances, and behavioral flags.

Monster classes are identified by a single character: `a` (ant), `A`
(angelic), `d` (canine), `D` (dragon), `&` (demon), `@` (human), etc.

### Runtime Instances

Monsters on the current level form a linked list (`fmon` → `monst.nmon` → ...).
Each `struct monst` carries its own HP, movement points, inventory, AI state,
and status effects.

### AI and Strategy

Monster AI is distributed across several files:

- `src/monmove.c` — main movement and decision-making
- `src/muse.c` — item usage (potions, wands, scrolls)
- `src/mcast.c` — spell casting
- `src/mthrowu.c` — ranged attacks
- `src/dog.c`, `src/dogmove.c` — pet-specific behavior

The `mstrategy` field encodes the monster's current goal as a 32-bit value:

- High bits: strategy type (`STRAT_WAITFORU`, `STRAT_CLOSE`, `STRAT_HEAL`,
  `STRAT_GROUND`, `STRAT_MONSTR`, `STRAT_PLAYER`)
- Low bits: target coordinates (`STRAT_XMASK`, `STRAT_YMASK`)

Monsters share the same movement point system as the player. `movemon()`
iterates over all monsters on the level and lets each act if they have
accumulated enough movement points.

## Object System

### Static Definitions (`src/objects.c`)

The `objects[]` array contains 500+ `struct objclass` entries. Object classes
include: weapons, armor, rings, amulets, tools, food, potions, scrolls,
spellbooks, wands, coins, gems, rocks, balls, chains, venom.

### Identification

Objects have a multi-layered identification system:

- **Base type**: always known to the game engine
- **Appearance**: randomized per game (shuffled descriptions for potions,
  scrolls, rings, etc. via `o_init.c`)
- **Player knowledge**: tracked per-object via `known`, `dknown`, `bknown`,
  `rknown` bitfields
- **Named/called**: players can name individual objects or call object classes

### Location Tracking

Every object has a `where` field indicating one of 8 location states. Objects
transition between states as they are picked up, dropped, put in containers,
etc. Each state implies a different linked-list membership.

## Dungeon and Level Generation

### Dungeon Topology

The dungeon structure is defined in `dat/dungeon.def` and compiled to binary
by `dgn_comp`. At runtime, `init_dungeons()` in `src/dungeon.c` reads the
binary topology and builds the `dungeons[]` array.

Levels are connected by stairs and portals. Branches connect dungeons at
specific depth ranges (sometimes randomized within a range).

### Level Generation

Four generation modes, selected based on dungeon flags and level depth:

1. **Room-and-corridor** (`src/mklev.c`) — the classic style. Rectangular
   rooms connected by corridors. Uses rectangle splitting (`src/rect.c`)
   to place rooms without overlap.

2. **Maze** (`src/mkmaze.c`) — wall-based mazes used in Gehennom and some
   branches.

3. **Cave** (`src/mkmap.c`) — cellular-automata-like generation for organic
   cave layouts.

4. **Special levels** (`src/sp_lev.c`) — hand-designed levels defined in
   `.des` files and compiled to bytecode by `lev_comp`. The bytecode
   interpreter executes 76 opcodes (`SPO_*`) that can place rooms,
   corridors, monsters, objects, traps, and terrain.

### Special Level Bytecode

The `.des` file format compiles to a stack-based bytecode with 76 opcodes
(`include/sp_lev.h`):

- **Level construction**: `SPO_MAP`, `SPO_ROOM`, `SPO_SUBROOM`, `SPO_CORRIDOR`
- **Feature placement**: `SPO_DOOR`, `SPO_STAIR`, `SPO_ALTAR`, `SPO_FOUNTAIN`,
  `SPO_TRAP`, `SPO_OBJECT`, `SPO_MONSTER`, `SPO_GOLD`
- **Terrain modification**: `SPO_TERRAIN`, `SPO_REPLACETERRAIN`, `SPO_MAZEWALK`
- **Selection operations** (13 opcodes): geometric selections for bulk placement
  (`SPO_SEL_RECT`, `SPO_SEL_ELLIPSE`, `SPO_SEL_FLOOD`, etc.)
- **Control flow**: `SPO_JMP`, `SPO_JL`, `SPO_JLE`, `SPO_JG`, `SPO_JGE`,
  `SPO_JE`, `SPO_JNE`, `SPO_CMP`
- **Stack/variables**: `SPO_PUSH`, `SPO_POP`, `SPO_VAR_INIT`, `SPO_FRAME_PUSH`
- **Math**: `SPO_MATH_ADD/SUB/MUL/DIV/MOD`, `SPO_RN2`
- **Functions**: `SPO_CALL`, `SPO_RETURN`, `SPO_EXIT`

This is essentially a small virtual machine for level construction.

## Combat System

Combat is handled by three parallel systems:

### Player Attacks Monster (`src/uhitm.c`)

1. To-hit roll: `1d20 + modifiers` vs. monster AC
   - Modifiers include: weapon enchantment, player level, strength bonus,
     weapon skill, luck
2. Damage roll: weapon damage dice + enchantment + strength bonus + skill bonus
3. Special effects: poison, disease, level drain, petrification, etc.
4. Artifact effects: additional damage and special powers

### Monster Attacks Player (`src/mhitu.c`)

Each monster can make up to 6 attacks per round (from `mattk[]`). Each attack
has:

- Attack type (claw, bite, kick, touch, gaze, breath, magic, etc.)
- Damage type (physical, fire, cold, poison, drain, disease, etc.)
- Damage dice

Special attacks include: theft, seduction, engulfing, gaze attacks, touch
attacks (petrification, sliming), and spellcasting.

### Monster Attacks Monster (`src/mhitm.c`)

Uses the same attack framework but with monster-vs-monster to-hit and damage
calculations. Returns flags indicating the outcome: `MM_MISS`, `MM_HIT`,
`MM_DEF_DIED`, `MM_AGR_DIED`.

## Magic System

### Spells (`src/spell.c`)

- Players learn spells by reading spellbooks
- Knowledge stored in `struct spell` array: spell ID, level, knowledge value
- Knowledge decays over time (decremented each turn from `KEEN=20000`)
- Spells can be re-read at 10% knowledge threshold
- Success probability depends on: role spell aptitude, armor penalties (metal
  helmets and shields interfere), spell level
- Failure causes `spell_backfire()` with random negative effects

### Wands and Beams (`src/zap.c`)

- Wands have charges (`spe` field)
- Zapping creates directional beams or area effects
- Beams bounce off walls, can be reflected
- Effect types: `ZT_MAGIC_MISSILE`, `ZT_FIRE`, `ZT_COLD`, `ZT_SLEEP`,
  `ZT_DEATH`, `ZT_LIGHTNING`, `ZT_POISON_GAS`, `ZT_ACID`
- `bhitm()` applies beam effects to monsters
- `bhito()` applies beam effects to objects

### Potions (`src/potion.c`) and Scrolls (`src/read.c`)

Each potion/scroll type has a dedicated handler. Effects range from simple
(healing, identify) to complex (polymorph, genocide, wishing).

## Save/Restore System

The save system (`src/save.c`, `src/restore.c`) serializes all game state to
disk:

### What Gets Saved

- Player state (`struct you`)
- Current level (terrain, objects, monsters, traps)
- All other visited levels (each in its own segment)
- Dungeon topology and level chains
- Message history (optional, for `DUMPLOG`)
- Game flags and context

### Save Architecture

- Uses a `save_procs` abstraction supporting multiple compression backends
- Each level is saved separately, enabling modular persistence
- On restore, an ID mapping bucket system translates ghost-level IDs to
  current-game IDs
- `find_lev_obj()` rebuilds the `level.objects[][]` spatial grid after
  restoration

### Bones Files

When a player dies, the level where they died can be saved as a "bones file"
(`src/bones.c`). Future games may encounter this level, complete with the
previous player's ghost and dropped items.

## Random Number Generation

The RNG system (`src/rnd.c`) uses **two independent ISAAC64 streams**:

1. **Core stream** — used by `rn2()` for all gameplay-affecting randomness
   (monster actions, damage rolls, item generation, etc.)
2. **Display stream** — used by `rn2_on_display_rng()` for visual-only
   randomness (sparkle effects, etc.)

This separation prevents players from manipulating gameplay outcomes by
observing or influencing visual effects.

Key functions:

- `rn2(x)` — uniform random in `[0, x)` (core stream)
- `rnd(x)` — uniform random in `[1, x]`
- `d(n, x)` — roll `n` dice of `x` sides
- `rnl(x)` — luck-adjusted: shifts distribution by `Luck/3`
- `rne(x)` — experience-scaled: geometric distribution capped by player level
- `rnz(x)` — used for timeouts, combines multiple rolls

When `USE_ISAAC64` is defined (the default), the engine is the ISAAC64
cryptographic PRNG. Otherwise, it falls back to the system `Rand()`.

## Intrinsics and Timeout System

### Intrinsic Properties

The player has ~80 trackable properties (`include/prop.h`) stored in
`u.uprops[]`. Each property is a `long` value where:

- Lower bits store the timeout (in game turns)
- Higher bits distinguish permanent vs. temporary sources

Properties include: `FIRE_RES`, `COLD_RES`, `POISON_RES`, `TELEPAT`,
`FLYING`, `LEVITATION`, `INVISIBILITY`, `SEE_INVIS`, `HALLUC`, `CONFUSION`,
`STUNNED`, `BLINDED`, `STONED`, `SLIMED`, `STRANGLED`, etc.

### Timeout Processing

Each turn, `nh_timeout()` (`src/timeout.c`) decrements active timeouts.
When a timeout expires:

- Status effects end with appropriate messages
- Fatal timeouts (`STONED`, `SLIMED`, `STRANGLED`) trigger death
- Setter functions (`make_confused()`, `make_stunned()`, `make_sick()`)
  handle both applying and messaging

## Global State

NetHack stores all game state in global variables:

| Variable       | Type                  | Description                                       |
| -------------- | --------------------- | ------------------------------------------------- |
| `u`            | `struct you`          | The player character                              |
| `fmon`         | `struct monst *`      | Head of monster linked list (current level)       |
| `level`        | `dlevel_t`            | Current level data (terrain, objects, traps)      |
| `flags`        | `struct flag`         | Persistent game settings                          |
| `context`      | `struct context_info` | Per-turn transient state                          |
| `dungeons[]`   | `struct dungeon[]`    | Dungeon topology array                            |
| `mons[]`       | `struct permonst[]`   | Static monster type definitions                   |
| `objects[]`    | `struct objclass[]`   | Static object type definitions                    |
| `invent`       | `struct obj *`        | Player inventory (linked list)                    |
| `youmonst`     | `struct monst`        | Player-as-monster (for polymorph and shared code) |
| `moves`        | `long`                | Total game turns elapsed                          |
| `monstermoves` | `long`                | Total monster move rounds                         |

This pervasive global mutable state is one of the main architectural
challenges for the Rust port.

## Data Files

### Dungeon Definition (`dat/dungeon.def`)

Human-readable specification of the dungeon topology. Defines dungeon names,
depth ranges, alignment, flags (hellish, mazelike), branch connections, and
special level placements. Compiled to binary `dungeon.pdf` by `dgn_comp`.

### Special Level Descriptions (`dat/*.des`)

40+ files defining hand-designed levels:

- Quest levels (one per role: `Arch.des`, `Barb.des`, `Cav.des`, etc.)
- Dungeon landmarks (`castle.des`, `medusa.des`, `oracle.des`, `tower.des`)
- Gehennom levels (`baalz.des`, `juiblex.des`, `orcus.des`, `sanctum.des`)
- Mines (`minefill.des`, `minend.des`)
- Sokoban puzzles (`soko1-4.des`)
- Endgame (`endgame.des`)
- Special features (`knox.des` for Fort Ludios)

### Text Databases

- `data.base` — in-game encyclopedia entries (266KB)
- `quest.txt` — quest dialogue (per-role)
- `rumors.tru` / `rumors.fal` — true and false rumors
- `oracles.txt` — Oracle consultation messages
- `epitaph.txt` — gravestone inscriptions
- `engrave.txt` — random engravings
- `bogusmon.txt` — fake monster names (for hallucinatory encounters)
- `tribute` — tribute passages

### Help Files

- `help` — general help text
- `hh` — quick reference
- `cmdhelp` — per-command help
- `keyhelp` — key binding reference
- `opthelp` — option descriptions
- `history` — game history/lore

## Build Tools Summary

| Tool       | Input                        | Output                                   | Purpose                         |
| ---------- | ---------------------------- | ---------------------------------------- | ------------------------------- |
| `makedefs` | `monst.c`, `objects.c`, etc. | `pm.h`, `onames.h`, `date.h`, data files | Header and data generation      |
| `dgn_comp` | `dungeon.def`                | `dungeon.pdf`                            | Dungeon topology compiler       |
| `lev_comp` | `*.des`                      | `*.lev`                                  | Special level bytecode compiler |
| `dlb`      | all data files               | `nhdat`                                  | Data library archiver           |
| `recover`  | interrupted save files       | recovered save                           | Save file recovery tool         |
