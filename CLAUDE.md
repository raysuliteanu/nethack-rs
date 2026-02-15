# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust port of NetHack 3.6.7 for learning purposes. The C source lives in `nethack/` as a git submodule (branch `NetHack-3.6`). Currently in Phase 1 (core types, static data, RNG) — no runnable game yet. See `docs/porting-plan.md` for the full multi-phase roadmap.

## Build Commands

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all 98 tests
cargo test -p nethack-rng        # RNG tests only
cargo test -p nethack-types      # type tests only
cargo test -p nethack-data       # data table tests only
cargo test -p nethack-data -- long_sword_spot_check  # single test
cargo clippy --workspace         # lint (must be clean)
cargo fmt --check                # format check (must be clean)
```

## Data Generation

Four source files are generated from C source by `tools/extract_data.py`:
- `crates/nethack-types/src/monster_id.rs` — `MonsterId` enum (384 variants)
- `crates/nethack-types/src/object_id.rs` — `ObjectId` enum (454 variants)
- `crates/nethack-data/src/monsters.rs` — `MONSTERS` static array
- `crates/nethack-data/src/objects.rs` — `OBJECTS` static array

Regenerate with `python3 tools/extract_data.py`. Generated files are committed. The script handles C preprocessing (comment stripping, `#if 0` removal, continuation line joining, inline macro expansion) and parses 25+ object wrapper macros.

## Architecture

Three library crates with a binary shell:

```
nethack-types  ←── nethack-data    (data depends on types)
                   nethack-rng     (standalone)
```

**nethack-types**: All enums, structs, and bitflags ported from C headers. Each enum gets its own module file. Enums use `#[repr(u8)]` or `#[repr(u16)]` with explicit discriminants matching C values. All types derive `Debug, Clone, Copy, PartialEq, Eq, Serialize`. Bitflag types use the `bitflags!` macro.

**nethack-data**: Static data tables indexed by `MonsterId`/`ObjectId`. Access pattern: `MONSTERS[MonsterId::GiantAnt as usize]`. Role/race data from `role.c` is not yet extracted (deferred — different structure than simple macro calls).

**nethack-rng**: Dual-stream ISAAC64 RNG matching NetHack's output exactly. `NhRng` has `core` (gameplay) and `display` (cosmetic) streams. Uses a direct port of `isaac64.c` (not `rand_isaac`) because NetHack's custom 8-byte little-endian seeding must be matched for save/replay compatibility. Invalid arguments log warnings and return safe defaults (matching C's `impossible()` pattern).

## Key Conventions

- C constant values are the ground truth — enum discriminants, flag bits, and array indices must match exactly
- RNG tests verify output against a C reference program (`tools/isaac64_ref.c`) with known seeds (42, 0, 12345)
- `Material` enum has no variant for C value 0; currently uses `Material::Liquid` as placeholder
- Monster/object counts (384/454) exclude `#if 0` blocks (9 deferred monsters, 4 deferred objects) and the terminator sentinel
- Were-creatures appear twice in the monster array (animal form and human form) with names like `Werewolf` / `HumanWerewolf`
- `SESSION.md` tracks deferred items and implementation notes — add non-blocking issues there rather than fixing immediately
