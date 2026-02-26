//! Central game state struct.
//!
//! `GameState` is the root of all mutable game state, passed by reference
//! through the game logic. This replaces C's scattered global variables.

use nethack_rng::NhRng;

use crate::dungeon::DungeonState;
use crate::level::Level;

/// Central game state, replacing C's global variables.
///
/// Everything that changes during gameplay lives here or is reachable from here.
/// This struct is designed to be passed by `&mut` reference rather than using
/// global mutable state.
pub struct GameState {
    /// The dungeon topology: which dungeons exist, their connections, special levels.
    pub dungeon: DungeonState,
    /// The current level the player is on.
    pub current_level: Level,
    /// The dual-stream RNG.
    pub rng: NhRng,
    /// Game turn counter.
    pub turn: u64,
    /// Monster movement counter.
    pub monster_moves: u64,
}
