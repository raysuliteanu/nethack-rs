# Phase 1: Core Types, Static Data, and RNG

## Context

This is Phase 1 of a multi-phase Rust port of NetHack 3.6. The C source is at
`nethack/` (git submodule). This phase produces no runnable game but creates the
type foundation and static data that everything else depends on.

## Workspace Structure

Convert from single crate to workspace:

```
nethack-rs/
├── Cargo.toml              (workspace root + binary crate)
├── crates/
│   ├── nethack-types/      enums, structs, constants, bitflags
│   ├── nethack-data/       static data tables (monsters, objects, roles)
│   └── nethack-rng/        dual-stream ISAAC64 RNG
├── src/main.rs             binary entry point
├── tools/
│   ├── extract_data.py     data table extraction script
│   └── isaac64_ref.c       C reference program for RNG verification
├── nethack/                C source (git submodule)
└── docs/
```

## Implementation Steps

### Step 1: Workspace Scaffolding

Create the Cargo workspace with three library crates. Workspace-level
dependencies:

- `thiserror`, `strum` (with derive), `serde` (with derive), `bitflags`,
  `color-eyre`, `log`, `env_logger`
- `rand_isaac`, `rand_core` (for nethack-rng)

Internal crate dependencies:
- `nethack-data` depends on `nethack-types`
- `nethack-rng` depends on `nethack-types`

**Verify:** `cargo check` passes.

### Step 2: ISAAC64 RNG — Tests First (TDD)

Start with the RNG crate using TDD. Write failing tests first, then implement.

**Step 2a: Write the C reference program** (`tools/isaac64_ref.c`)

A small C program that `#include`s NetHack's `isaac64.c` directly, seeds with
known values (42, 0, 12345), and prints:
- First 20 raw `uint64` values per seed
- First 20 `% 100` values per seed (matching `rn2(100)` behavior)

Compile and capture output as reference data.

**Step 2b: Write failing Rust tests**

In `nethack-rng`, write tests that hard-code the C reference output:

```rust
#[test]
fn rn2_matches_c_seed_42() {
    let mut rng = NhRng::new(42);
    let expected = [/* values from C program */];
    for &e in &expected {
        assert_eq!(rng.rn2(100), e);
    }
}
```

Also write property tests:
- `rn2(x)` always in `[0, x)`
- `rnd(x)` always in `[1, x]`
- `d(n, x)` always in `[n, n*x]`
- Dual-stream independence (calling Core doesn't affect Display)
- Determinism (same seed → same sequence)

**Step 2c: Implement NhRng**

Try `rand_isaac` first. If seeding doesn't match C output (likely due to
NetHack's custom 8-byte LE seeding convention), fall back to directly porting
`isaac64.c` (176 lines, CC0-licensed) into `nethack-rng/src/isaac64.rs`.

Public API:
```rust
pub struct NhRng { core: Isaac64Ctx, display: Isaac64Ctx }

impl NhRng {
    pub fn new(seed: u64) -> Self;
    pub fn rn2(&mut self, x: i32) -> i32;           // [0, x)
    pub fn rn2_on_display_rng(&mut self, x: i32) -> i32;
    pub fn rnd(&mut self, x: i32) -> i32;            // [1, x]
    pub fn d(&mut self, n: i32, x: i32) -> i32;      // sum of n d-x
    pub fn rnl(&mut self, x: i32, luck: i32) -> i32;  // luck-adjusted
    pub fn rne(&mut self, x: i32, ulevel: i32) -> i32; // exp-scaled
    pub fn rnz(&mut self, i: i32) -> i32;              // timeout
}
```

Invalid arguments → `log::warn!()` + safe default (matching C `impossible()`).

**Verify:** `cargo test -p nethack-rng` passes, including C-compatibility tests.

### Step 3: Foundational Enums (nethack-types)

Port all simple enums and bitflags from C headers. Each gets its own module file.
All types derive `Debug, Clone, Copy, PartialEq, Eq, Serialize`. Use `#[repr(u8)]`
or `#[repr(u16)]` with explicit discriminants matching C values.

| Module | Source Header | Contents |
|--------|--------------|----------|
| `color.rs` | `color.h` | `Color` enum (16 variants) |
| `alignment.rs` | `align.h` | `Alignment` enum, `AlignmentMask` bitflags |
| `monster_class.rs` | `monsym.h` | `MonsterClass` enum (61 variants) + `default_symbol()` |
| `attack.rs` | `monattk.h` | `AttackType` (18), `DamageType` (~45) |
| `monster_sound.rs` | `monflag.h` | `MonsterSound` (40 variants) |
| `monster_size.rs` | `monflag.h` | `MonsterSize` (6 variants) |
| `monster_flags.rs` | `monflag.h` | `MonsterFlags1/2/3` bitflags |
| `resistance.rs` | `monflag.h` | `Resistance` bitflags (8 bits) |
| `geno.rs` | `monflag.h` | `GenoFlags` bitflags + `frequency()` method |
| `object_class.rs` | `objclass.h` | `ObjectClass` enum (18) + `symbol()` |
| `material.rs` | `objclass.h` | `Material` enum (21 variants) |
| `armor_type.rs` | `objclass.h` | `ArmorType` enum (7 variants) |
| `location_type.rs` | `rm.h` | `LocationType` enum (36) + helper methods |
| `door_state.rs` | `rm.h` | `DoorState` bitflags |
| `property.rs` | `prop.h` | `Property` enum (67 variants) |
| `worn.rs` | `prop.h` | `WornMask` bitflags |

Tests per enum:
- Round-trip `as u8` / `TryFrom<u8>` or `from_repr`
- Discriminant values match C constants exactly
- `EnumIter` count matches expected count
- Bitflag composite values (e.g. `M1_OMNIVORE == M1_CARNIVORE | M1_HERBIVORE`)

**Verify:** `cargo test -p nethack-types`

### Step 4: Compound Structs (nethack-types)

Build on the leaf enums:

- `Attack` struct (attack_type, damage_type, dice_num, dice_sides) + `Attack::NONE`
- `MonsterType` struct (19 fields matching `struct permonst`)
- `ObjectType` struct (~20 fields matching `struct objclass`)
- `RoleKind` enum (13 roles), `RaceKind` (5), `Gender` (3)
- `RoleDefinition`, `RaceDefinition` structs with rank titles, gods, stats, etc.
- `MonsterId` enum (394 variants, `#[repr(u16)]`) — generated in Step 5
- `ObjectId` enum (~450 variants, `#[repr(u16)]`) — generated in Step 5

**Verify:** `cargo test -p nethack-types`

### Step 5: Data Extraction Script

Write `tools/extract_data.py` to read C source and emit Rust code:

**Inputs:**
- `nethack/src/monst.c` — 394 `MON()` macro calls
- `nethack/src/objects.c` — ~450 object macros (`WEAPON`, `ARMOR`, `RING`, etc.)
- `nethack/src/role.c` — role/race tables

**Outputs:**
- `crates/nethack-types/src/monster_id.rs` — `MonsterId` enum
- `crates/nethack-types/src/object_id.rs` — `ObjectId` enum
- `crates/nethack-data/src/monsters.rs` — `pub const MONSTERS: &[MonsterType]`
- `crates/nethack-data/src/objects.rs` — `pub const OBJECTS: &[ObjectType]`
- `crates/nethack-data/src/roles.rs` — role/race/gender/alignment tables

Script approach:
1. Strip C comments, find array initializer blocks
2. Parse macro calls with bracket-aware argument splitting
3. Map C constant names to Rust enum/bitflag names
4. Emit `const` array with Rust constructor expressions
5. Use `from_bits_truncate()` for bitflag combinations in const context

Generated files get a header comment marking them as generated. The script is
kept for re-verification but output is committed.

**Verify:** Script runs without errors, output compiles.

### Step 6: Static Data Tables (nethack-data)

The extraction script output populates this crate:

```
crates/nethack-data/src/
    lib.rs          — re-exports
    monsters.rs     — pub const MONSTERS: &[MonsterType; 394]
    objects.rs      — pub const OBJECTS: &[ObjectType; N]
    roles.rs        — roles, races, genders, alignments
```

Tests:
- `MONSTERS.len() == 394`
- Spot-checks: `MONSTERS[MonsterId::GiantAnt as usize]` has expected stats
- `OBJECTS.len()` matches expected total
- Spot-checks: arrow, long sword, plate mail have expected fields
- Every `MonsterId`/`ObjectId` variant maps to a valid array index
- `ROLES.len() == 13`, `RACES.len() == 5`

**Verify:** `cargo test -p nethack-data`

### Step 7: Integration Tests and Final Verification

Workspace-level integration tests confirming the crates compose correctly:

- Monster data completeness (every ID → valid entry, no empty names)
- Object data completeness
- RNG determinism with seed → consistent across crates
- Full workspace: `cargo build && cargo test && cargo clippy && cargo fmt --check`

## Dependency Order

```
Step 1: Workspace scaffolding
    |
    ├── Step 2: RNG (TDD — tests first, then implement)
    |
    └── Step 3: Leaf enums/bitflags
            |
        Step 4: Compound structs
            |
        Step 5: Extraction script
            |
        Step 6: Static data tables
            |
            +--- Step 2 (already done in parallel)
            |
        Step 7: Integration tests
```

Steps 2 and 3–6 are independent and can proceed in parallel.

## Key C Source Files

| File | Purpose |
|------|---------|
| `nethack/include/monflag.h` | M1/M2/M3/MR/MS/MZ/G flags |
| `nethack/include/monattk.h` | AT_*/AD_* attack/damage types |
| `nethack/include/monsym.h` | S_* monster class symbols |
| `nethack/include/permonst.h` | `struct permonst` fields |
| `nethack/include/objclass.h` | `struct objclass`, object classes, materials |
| `nethack/include/rm.h` | Terrain types, map cell, COLNO/ROWNO |
| `nethack/include/prop.h` | Properties, worn masks |
| `nethack/include/color.h` | Color constants |
| `nethack/include/align.h` | Alignment types |
| `nethack/src/monst.c` | 394 monster definitions |
| `nethack/src/objects.c` | ~450 object definitions |
| `nethack/src/role.c` | Role/race/gender/alignment tables |
| `nethack/src/isaac64.c` | ISAAC64 implementation (176 lines, CC0) |
| `nethack/src/rnd.c` | RNG wrappers (rn2, rnd, d, rnl, rne, rnz) |

## Notes

- All types get `#[derive(Serialize)]` from day one for Phase 7 save/restore.
  `Deserialize` deferred (needs `Cow<'static, str>` or custom deserializer for
  `&'static str` fields).
- `serde` on `bitflags` types may need `serde` feature flag on the `bitflags`
  crate.
- The C `NON_PM` sentinel (-1) becomes `Option<MonsterId>` with `None`.
- Invalid RNG arguments use `log::warn!()` + safe default, not `Result`.
