//! Parser/compiler for the NetHack `.des` level scripting language.
//!
//! Consumes tokens from [`des_lexer`] and emits [`SpLevOpcode`] bytecode
//! matching the semantics of C's `lev_comp` (`nethack/util/lev_comp.y`).

use crate::des_lexer::{Located, Token};
use crate::monsters::MONSTERS;
use crate::objects::OBJECTS;
use nethack_types::sp_lev::{
    DesFile, LevelFlags, SpLevOpcode, SpMonVarFlag, SpObjVarFlag, SpOpcode, SpOperand, SpecialLevel,
};

#[derive(Debug, thiserror::Error)]
pub enum DesParseError {
    #[error("line {line}: {msg}")]
    Parse { line: usize, msg: String },
}

/// Variable type tracking for the symbol table (used for future type checking).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum VarType {
    Int,
    String,
    Coord,
    Region,
    MapChar,
    Monst,
    Obj,
    Sel,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VarDef {
    typ: VarType,
    is_array: bool,
}

/// Convert a display character to a terrain type, matching C's `what_map_char()`.
///
/// This is the same mapping used by `lev_main.c` to convert characters in
/// `.des` MAP blocks and fill specifications to `rm.h` terrain type values.
/// Returns 127 (`INVALID_TYPE`) for unrecognized characters.
const INVALID_TYPE: i16 = 127;
const MAX_TYPE: i16 = 36;

fn what_map_char(c: char) -> i16 {
    match c {
        ' ' => 0,        // STONE
        '#' => 23,       // CORR
        '.' => 24,       // ROOM
        '-' => 2,        // HWALL
        '|' => 1,        // VWALL
        '+' => 22,       // DOOR
        'A' => 34,       // AIR
        'B' => 7,        // CROSSWALL (boundary)
        'C' => 35,       // CLOUD
        'S' => 14,       // SDOOR
        'H' => 15,       // SCORR
        '{' => 27,       // FOUNTAIN
        '\\' => 28,      // THRONE
        'K' => 29,       // SINK
        '}' => 17,       // MOAT
        'P' => 16,       // POOL
        'L' => 20,       // LAVAPOOL
        'I' => 32,       // ICE
        'W' => 18,       // WATER
        'T' => 13,       // TREE
        'F' => 21,       // IRONBARS (Fe = iron)
        'x' => MAX_TYPE, // "see-through"
        _ => INVALID_TYPE,
    }
}

/// Result of `scan_map()` conversion.
struct ScanMapResult {
    /// Converted map data: each char is `what_map_char(c) + 1`, rows padded to max width.
    data: String,
    height: usize,
    width: usize,
}

/// Replicate C's `scan_map()` from `lev_main.c`.
///
/// 1. Strip digits 0-9 (C uses these for line numbering in some maps)
/// 2. Find max row width
/// 3. Convert each character through `what_map_char()`, add +1
/// 4. Pad shorter rows with `STONE + 1 = 1`
fn scan_map(raw: &str) -> ScanMapResult {
    // Strip digits
    let stripped: String = raw.chars().filter(|c| !c.is_ascii_digit()).collect();

    // Split into rows and find max width
    let rows: Vec<&str> = stripped.split('\n').collect();
    let max_len = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let max_hig = rows.len();

    // Convert each character and pad
    let mut buf = Vec::with_capacity(max_hig * max_len);
    for row in &rows {
        for ch in row.chars() {
            let terrain = what_map_char(ch);
            let terrain = if terrain == INVALID_TYPE { 0 } else { terrain }; // STONE for invalid
            buf.push((terrain as u8).wrapping_add(1));
        }
        // Pad to max_len with STONE+1 = 1
        for _ in row.len()..max_len {
            buf.push(1); // STONE + 1
        }
    }

    // Convert to string (C stores as char array)
    let data = buf.into_iter().map(|b| b as char).collect();

    ScanMapResult {
        data,
        height: max_hig,
        width: max_len,
    }
}

/// Resolve a monster name to its index in `MONSTERS`, optionally filtered by class char.
///
/// Matches C's `get_monster_id()` in `lev_main.c`: exact match first, then
/// case-insensitive fallback. The class char filters by monster symbol.
fn get_monster_id(name: &str, class_char: char) -> Option<i16> {
    // Exact match
    for (i, m) in MONSTERS.iter().enumerate() {
        if class_char != '\0' && m.symbol != class_char {
            continue;
        }
        if m.name == name {
            return Some(i as i16);
        }
    }
    // Case-insensitive fallback
    let name_lower = name.to_lowercase();
    for (i, m) in MONSTERS.iter().enumerate() {
        if class_char != '\0' && m.symbol != class_char {
            continue;
        }
        if m.name.to_lowercase() == name_lower {
            return Some(i as i16);
        }
    }
    None
}

/// Resolve an object name to its index in `OBJECTS`, optionally filtered by class char.
///
/// Matches C's `get_object_id()` in `lev_main.c`. The class char filters by
/// the object class's display symbol.
fn get_object_id(name: &str, class_char: char) -> Option<i16> {
    use nethack_types::ObjectClass;

    let filter_class = if class_char != '\0' {
        // Map display char to ObjectClass
        (0..ObjectClass::MAX)
            .map(|i| unsafe { std::mem::transmute::<u8, ObjectClass>(i as u8) })
            .find(|c| c.symbol() == class_char)
    } else {
        None
    };

    // Exact match
    for (i, o) in OBJECTS.iter().enumerate() {
        if let Some(fc) = filter_class {
            if o.class != fc {
                continue;
            }
        }
        if o.name == name {
            return Some(i as i16);
        }
    }
    // Case-insensitive fallback
    let name_lower = name.to_lowercase();
    for (i, o) in OBJECTS.iter().enumerate() {
        if let Some(fc) = filter_class {
            if o.class != fc {
                continue;
            }
        }
        if o.name.to_lowercase() == name_lower {
            return Some(i as i16);
        }
    }
    None
}

/// Resolve a trap name to its type ID, matching C's `get_trap_type()`.
fn get_trap_type(name: &str) -> Option<i64> {
    match name {
        "arrow" => Some(1),
        "dart" => Some(2),
        "falling rock" => Some(3),
        "board" => Some(4),
        "bear" => Some(5),
        "land mine" => Some(6),
        "rolling boulder" => Some(7),
        "sleep gas" => Some(8),
        "rust" => Some(9),
        "fire" => Some(10),
        "pit" => Some(11),
        "spiked pit" => Some(12),
        "hole" => Some(13),
        "trap door" => Some(14),
        "teleport" => Some(15),
        "level teleport" => Some(16),
        "magic portal" => Some(17),
        "web" => Some(18),
        "statue" => Some(19),
        "magic" => Some(20),
        "anti magic" => Some(21),
        "polymorph" => Some(22),
        "vibrating square" => Some(23),
        _ => None,
    }
}

/// Parser state for compiling a `.des` file.
struct Parser {
    tokens: Vec<Located<Token>>,
    pos: usize,
    /// Per-level opcode accumulator.
    opcodes: Vec<SpLevOpcode>,
    /// Variable symbol table (per level, reset on each MAZE/LEVEL).
    vars: std::collections::HashMap<String, VarDef>,
    /// Container nesting depth.
    container_depth: u32,
    /// Collected levels.
    levels: Vec<SpecialLevel>,
    /// Current level name.
    level_name: String,
    /// Roomfill value from GEOMETRY (C default = 1).
    roomfill: i64,
}

impl Parser {
    fn new(tokens: Vec<Located<Token>>) -> Self {
        Self {
            tokens,
            pos: 0,
            opcodes: Vec::new(),
            vars: std::collections::HashMap::new(),
            container_depth: 0,
            levels: Vec::new(),
            level_name: String::new(),
            roomfill: 1,
        }
    }

    fn current_line(&self) -> usize {
        self.tokens.get(self.pos).map(|t| t.line).unwrap_or(0)
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.value)
            .unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = self
            .tokens
            .get(self.pos)
            .map(|t| &t.value)
            .unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), DesParseError> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!("expected {expected:?}, got {tok:?}")))
        }
    }

    fn expect_colon(&mut self) -> Result<(), DesParseError> {
        self.expect(&Token::Colon)
    }

    fn expect_comma(&mut self) -> Result<(), DesParseError> {
        self.expect(&Token::Comma)
    }

    fn err(&self, msg: &str) -> DesParseError {
        DesParseError::Parse {
            line: self.current_line(),
            msg: msg.into(),
        }
    }

    fn emit(&mut self, opcode: SpOpcode) {
        self.opcodes.push(SpLevOpcode {
            opcode,
            operand: None,
        });
    }

    fn emit_push_int(&mut self, val: i64) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Int(val)),
        });
    }

    fn emit_push_str(&mut self, val: &str) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::String(val.to_string())),
        });
    }

    fn emit_push_coord(&mut self, x: i16, y: i16, is_random: bool, flags: u32) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Coord {
                x,
                y,
                is_random,
                flags,
            }),
        });
    }

    fn emit_push_region(&mut self, x1: i16, y1: i16, x2: i16, y2: i16) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Region { x1, y1, x2, y2 }),
        });
    }

    fn emit_push_mapchar(&mut self, typ: i16, lit: i16) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::MapChar { typ, lit }),
        });
    }

    fn emit_push_monst(&mut self, class: i16, id: i16) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Monst { class, id }),
        });
    }

    fn emit_push_obj(&mut self, class: i16, id: i16) {
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Obj { class, id }),
        });
    }

    fn emit_push_var(&mut self, name: &str) {
        // C preserves the `$` prefix in variable names
        let var_name = if name.starts_with('$') {
            name.to_string()
        } else {
            format!("${name}")
        };
        self.opcodes.push(SpLevOpcode {
            opcode: SpOpcode::Push,
            operand: Some(SpOperand::Variable(var_name)),
        });
    }

    fn emit_var_init(&mut self, name: &str, count: i64) {
        // C preserves the `$` prefix in variable names
        let var_name = if name.starts_with('$') {
            name.to_string()
        } else {
            format!("${name}")
        };
        self.emit_push_int(count);
        self.emit_push_str(&var_name);
        self.emit(SpOpcode::VarInit);
    }

    fn is_in_container(&self) -> bool {
        self.container_depth > 0
    }

    fn current_offset(&self) -> usize {
        self.opcodes.len()
    }

    /// Patch a previously emitted PUSH(int) to contain a relative jump offset.
    fn patch_jump(&mut self, push_idx: usize) {
        let target = self.current_offset() as i64;
        if let Some(SpLevOpcode {
            operand: Some(SpOperand::Int(val)),
            ..
        }) = self.opcodes.get_mut(push_idx)
        {
            *val = target - *val;
        }
    }

    fn finish_level(&mut self) {
        if !self.level_name.is_empty() {
            let opcodes = std::mem::take(&mut self.opcodes);
            self.levels.push(SpecialLevel {
                name: std::mem::take(&mut self.level_name),
                opcodes,
            });
            self.vars.clear();
            self.container_depth = 0;
            self.roomfill = 1;
        }
    }

    // ---- Top-level parsing ----

    fn parse(mut self) -> Result<DesFile, DesParseError> {
        while *self.peek() != Token::Eof {
            // Handle optional percent prefix: [75%]: statement
            let pct = self.try_percent_prefix()?;
            if pct.is_some() {
                // Expect colon after percent
                self.expect_colon()?;
            }

            match self.peek().clone() {
                Token::Maze => self.parse_maze()?,
                Token::Level => self.parse_level_def()?,
                Token::Eof => break,
                _ => {
                    if let Some(pct_val) = pct {
                        self.parse_pct_statement(pct_val)?;
                    } else {
                        self.parse_statement()?;
                    }
                }
            }
        }
        self.finish_level();
        Ok(DesFile {
            levels: self.levels,
        })
    }

    fn try_percent_prefix(&mut self) -> Result<Option<i64>, DesParseError> {
        if let Token::Percent(n) = *self.peek() {
            self.advance();
            Ok(Some(n))
        } else {
            Ok(None)
        }
    }

    fn parse_pct_statement(&mut self, pct: i64) -> Result<(), DesParseError> {
        // Emit: IF [pct%] { statement }
        let if_start = self.emit_percent_condition(pct);
        self.parse_statement()?;
        self.patch_jump(if_start);
        Ok(())
    }

    /// Emits a percent-chance condition. Returns the index of the jump target
    /// PUSH that needs to be patched after the body.
    fn emit_percent_condition(&mut self, pct: i64) -> usize {
        self.emit_push_int(pct);
        self.emit_push_int(100);
        self.emit(SpOpcode::Rn2);
        self.emit_push_int(0);
        self.emit(SpOpcode::Cmp);
        let jmp_idx = self.current_offset();
        self.emit_push_int(jmp_idx as i64 + 1);
        // JG = jump if greater (i.e., skip if rn2 result > 0 → pct failed)
        self.emit(SpOpcode::Jg);
        jmp_idx
    }

    /// Parse a comparison operator and return the *negated* jump opcode.
    /// For `==` we want to jump when NOT equal (Jne), etc.
    fn parse_comparison_op(&mut self) -> Result<SpOpcode, DesParseError> {
        let op = match self.peek() {
            Token::CompareEq => SpOpcode::Jne,
            Token::CompareNe => SpOpcode::Je,
            Token::CompareLt => SpOpcode::Jge,
            Token::CompareLe => SpOpcode::Jg,
            Token::CompareGt => SpOpcode::Jle,
            Token::CompareGe => SpOpcode::Jl,
            _ => return Err(self.err("expected comparison operator")),
        };
        self.advance();
        Ok(op)
    }

    fn parse_maze(&mut self) -> Result<(), DesParseError> {
        self.finish_level();
        self.advance(); // MAZE
        self.expect_colon()?;

        let name = self.parse_string()?;
        self.level_name = name;
        self.expect_comma()?;

        // Fill char or 'random' — C's mazefiller rule converts through what_map_char
        let fill_val = match self.peek().clone() {
            Token::Random => {
                self.advance();
                -1i64
            }
            Token::Char(c) => {
                self.advance();
                what_map_char(c) as i64
            }
            _ => 0,
        };

        // MAZE emits: INITLEVEL + LEVEL_FLAGS(MAZELEVEL)
        if fill_val == -1 {
            // Random fill → mazegrid
            self.emit_push_int(2); // LVLINIT_MAZEGRID
            self.emit_push_int(2); // HWALL
        } else {
            // C's MAZE_ID rule calls what_map_char((char) $5) again on the
            // already-converted value from mazefiller. This double-conversion
            // is a bug in lev_comp.y, but we replicate it for compatibility.
            let bg = what_map_char(fill_val as u8 as char) as i64;
            self.emit_push_int(1); // LVLINIT_SOLIDFILL
            self.emit_push_int(bg);
        }
        // Remaining INITLEVEL params: smoothed=0, lit=0, joined=0, hushed=0, bg=0, fg=0
        for _ in 0..6 {
            self.emit_push_int(0);
        }
        self.emit(SpOpcode::InitLevel);

        let flags = LevelFlags::MAZELEVEL;
        self.emit_push_int(flags.bits() as i64);
        self.emit(SpOpcode::LevelFlags);

        // C's grammar: `level : level_def flags levstatements`
        // The `flags` production always emits a LEVEL_FLAGS opcode.
        self.parse_mandatory_flags()?;

        Ok(())
    }

    fn parse_level_def(&mut self) -> Result<(), DesParseError> {
        self.finish_level();
        self.advance(); // LEVEL
        self.expect_colon()?;
        let name = self.parse_string()?;
        self.level_name = name;
        // C's LEVEL_ID rule only calls start_level_def() — no opcodes emitted.
        // But the `flags` production always fires next in the grammar.
        self.parse_mandatory_flags()?;
        Ok(())
    }

    /// Emit the mandatory `flags` production from C's grammar.
    ///
    /// In C's `lev_comp.y`, `flags` always runs after `level_def`, emitting
    /// `PUSH(flag_bits) + LEVEL_FLAGS`. If no `FLAGS:` keyword is present,
    /// it emits `PUSH(0) + LEVEL_FLAGS`.
    fn parse_mandatory_flags(&mut self) -> Result<(), DesParseError> {
        if *self.peek() == Token::Flags {
            self.parse_flags()
        } else {
            self.emit_push_int(0);
            self.emit(SpOpcode::LevelFlags);
            Ok(())
        }
    }

    fn parse_statement(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::Flags => self.parse_flags(),
            Token::InitMap => self.parse_init_map(),
            Token::Geometry => self.parse_geometry(),
            Token::Nomap => self.parse_nomap(),
            Token::Map => self.parse_map_statement(),
            Token::Message => self.parse_message(),
            Token::Monster => self.parse_monster(),
            Token::Object => self.parse_object(),
            Token::Container => self.parse_container(),
            Token::Trap => self.parse_trap(),
            Token::Door => self.parse_door(),
            Token::RoomDoor => self.parse_roomdoor(),
            Token::Drawbridge => self.parse_drawbridge(),
            Token::Fountain => self.parse_fountain(),
            Token::Sink => self.parse_sink(),
            Token::Pool => self.parse_pool(),
            Token::Ladder => self.parse_ladder(),
            Token::Stair => self.parse_stair(),
            Token::Altar => self.parse_altar(),
            Token::TeleportRegion => self.parse_teleport_region(),
            Token::Branch => self.parse_branch_region(),
            Token::Portal => self.parse_portal_region(),
            Token::Gold => self.parse_gold(),
            Token::Engraving => self.parse_engraving(),
            Token::Grave => self.parse_grave(),
            Token::MazeWalk => self.parse_mazewalk(),
            Token::Wallify => self.parse_wallify(),
            Token::Mineralize => self.parse_mineralize(),
            Token::NonDiggable => self.parse_non_diggable(),
            Token::NonPasswall => self.parse_non_passwall(),
            Token::Terrain => self.parse_terrain(),
            Token::ReplaceTerrain => self.parse_replace_terrain(),
            Token::Region => self.parse_region(),
            Token::Room => self.parse_room(false),
            Token::Subroom => self.parse_room(true),
            Token::Corridor => self.parse_corridor(),
            Token::RandomCorridors => self.parse_random_corridors(),
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::Loop => self.parse_loop(),
            Token::Switch => self.parse_switch(),
            Token::Function => self.parse_function(),
            Token::Exit => self.parse_exit(),
            Token::Shuffle => self.parse_shuffle(),
            Token::Variable(_) => self.parse_variable_assignment(),
            _ => {
                let tok = self.peek().clone();
                Err(self.err(&format!("unexpected token: {tok:?}")))
            }
        }
    }

    // ---- Primitive parsers ----

    fn parse_string(&mut self) -> Result<String, DesParseError> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(self.err("expected string")),
        }
    }

    fn parse_integer(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::Integer(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.err("expected integer")),
        }
    }

    /// Parse an integer, dice notation, or variable reference, pushing the
    /// result onto the stack.
    fn parse_integer_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::Integer(n) => {
                self.advance();
                self.emit_push_int(n);
                Ok(())
            }
            Token::Dice { num, die } => {
                self.advance();
                self.emit_push_int(num);
                self.emit_push_int(die);
                self.emit(SpOpcode::Dice);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                    self.emit_push_var(&name);
                } else {
                    self.emit_push_var(&name);
                }
                Ok(())
            }
            _ => Err(self.err("expected integer, dice, or variable")),
        }
    }

    /// Parse a math expression (integer, dice, or variable) with optional
    /// arithmetic operators.
    fn parse_math_expr(&mut self) -> Result<(), DesParseError> {
        self.parse_integer_or_var()?;
        // Handle binary ops: +, -, etc.
        loop {
            match self.peek() {
                Token::Plus => {
                    self.advance();
                    self.parse_integer_or_var()?;
                    self.emit(SpOpcode::MathAdd);
                }
                Token::Minus => {
                    self.advance();
                    self.parse_integer_or_var()?;
                    self.emit(SpOpcode::MathSub);
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Parse a string expression (string literal or variable), pushing onto stack.
    fn parse_string_expr(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                self.emit_push_str(&s);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            _ => Err(self.err("expected string or variable")),
        }
    }

    /// Parse a coordinate: `(x,y)`, `random`, or `$var` / `rndcoord($sel)`.
    fn parse_coord_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::Random => {
                self.advance();
                self.emit_push_coord(-1, -1, true, 0);
                Ok(())
            }
            Token::LParen => {
                self.advance();
                let x = self.parse_integer()? as i16;
                self.expect_comma()?;
                let y = self.parse_integer()? as i16;
                self.expect(&Token::RParen)?;
                self.emit_push_coord(x, y, false, 0);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            Token::RndCoord => {
                self.advance();
                self.expect(&Token::LParen)?;
                self.parse_ter_selection()?;
                self.expect(&Token::RParen)?;
                self.emit(SpOpcode::SelRndCoord);
                Ok(())
            }
            _ => Err(self.err("expected coordinate, random, or variable")),
        }
    }

    /// Parse a region: `(x1,y1,x2,y2)` or `$var`.
    fn parse_region_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let x1 = self.parse_integer()? as i16;
                self.expect_comma()?;
                let y1 = self.parse_integer()? as i16;
                self.expect_comma()?;
                let x2 = self.parse_integer()? as i16;
                self.expect_comma()?;
                let y2 = self.parse_integer()? as i16;
                self.expect(&Token::RParen)?;
                self.emit_push_region(x1, y1, x2, y2);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            _ => Err(self.err("expected region or variable")),
        }
    }

    /// Parse a map character: `'x'`, `('.', lit)`, `random`, or `$var`.
    ///
    /// Characters are converted through `what_map_char` to terrain type integers,
    /// matching C's `mapchar` rule in `lev_comp.y`.
    fn parse_mapchar_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::Char(c) => {
                self.advance();
                self.emit_push_mapchar(what_map_char(c), -1);
                Ok(())
            }
            Token::LParen => {
                // Tuple form: ('x', lit/unlit)
                self.advance();
                let c = match self.peek().clone() {
                    Token::Char(c) => {
                        self.advance();
                        what_map_char(c)
                    }
                    _ => return Err(self.err("expected char in mapchar tuple")),
                };
                self.expect_comma()?;
                let lit = match self.peek() {
                    Token::Lit => {
                        self.advance();
                        1i16
                    }
                    Token::Unlit => {
                        self.advance();
                        0
                    }
                    Token::Random => {
                        self.advance();
                        -1
                    }
                    _ => return Err(self.err("expected lit/unlit in mapchar tuple")),
                };
                self.expect(&Token::RParen)?;
                self.emit_push_mapchar(c, lit);
                Ok(())
            }
            Token::Random => {
                self.advance();
                self.emit_push_mapchar(-1, -1);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            _ => Err(self.err("expected map char, random, or variable")),
        }
    }

    /// Parse a monster specifier: `('c',"name")`, `'c'`, `random`, or `$var`.
    ///
    /// C resolves monster names at compile time via `get_monster_id()` and packs
    /// the result with `SP_MONST_PACK(id, class_char)`. Named monsters emit NO
    /// string push — just the packed Monst operand.
    fn parse_monster_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let class_char = match self.peek().clone() {
                    Token::Char(c) => {
                        self.advance();
                        c
                    }
                    _ => return Err(self.err("expected monster class char")),
                };
                self.expect_comma()?;
                let name = self.parse_string()?;
                self.expect(&Token::RParen)?;
                let id = get_monster_id(&name, class_char).unwrap_or(-1);
                self.emit_push_monst(class_char as i16, id);
                Ok(())
            }
            Token::Char(c) => {
                self.advance();
                self.emit_push_monst(c as i16, -1);
                Ok(())
            }
            Token::Random => {
                self.advance();
                // C: -1 unpacks via SP_MONST_CLASS/PM to class=255, id=-11
                self.emit_push_monst(255, -11);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            _ => Err(self.err("expected monster spec, random, or variable")),
        }
    }

    /// Parse an object specifier: `('c',"name")`, `'c'`, `"name"`, `random`, or `$var`.
    ///
    /// C resolves object names at compile time via `get_object_id()` and packs
    /// with `SP_OBJ_PACK(id, class_char)`. For name-only objects, C uses class=1
    /// to force specific item generation. Named objects emit NO string push.
    fn parse_object_or_var(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let class_char = match self.peek().clone() {
                    Token::Char(c) => {
                        self.advance();
                        c
                    }
                    _ => return Err(self.err("expected object class char")),
                };
                self.expect_comma()?;
                let name = self.parse_string()?;
                self.expect(&Token::RParen)?;
                let id = get_object_id(&name, class_char).unwrap_or(-1);
                self.emit_push_obj(class_char as i16, id);
                Ok(())
            }
            Token::Char(c) => {
                self.advance();
                self.emit_push_obj(c as i16, -1);
                Ok(())
            }
            Token::Random => {
                self.advance();
                // C: -1 unpacks via SP_OBJ_CLASS/TYP to class=255, id=-11
                self.emit_push_obj(255, -11);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            _ => Err(self.err("expected object spec, random, or variable")),
        }
    }

    /// Parse a terrain selection expression (can be a coord, region, selection
    /// function, or variable).
    fn parse_ter_selection(&mut self) -> Result<(), DesParseError> {
        self.parse_ter_selection_x()?;
        // Handle '&' composition (selection union)
        while self.peek() == &Token::Ampersand {
            self.advance();
            self.parse_ter_selection_x()?;
            self.emit(SpOpcode::SelAdd);
        }
        Ok(())
    }

    fn parse_ter_selection_x(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::LParen => {
                // Check if next token is a selection function keyword
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len() {
                    match &self.tokens[next_pos].value {
                        Token::RandLine
                        | Token::Line
                        | Token::Rect
                        | Token::FillRect
                        | Token::Grow
                        | Token::FloodFill
                        | Token::Filter
                        | Token::Complement
                        | Token::Ellipse
                        | Token::Circle
                        | Token::Gradient => {
                            self.advance(); // consume LParen
                            self.parse_ter_selection_x()?;
                            self.expect(&Token::RParen)?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                // Single coordinate → point selection
                self.parse_coord_or_var()?;
                self.emit(SpOpcode::SelPoint);
                Ok(())
            }
            Token::Rect => {
                self.advance();
                self.parse_region_or_var()?;
                self.emit(SpOpcode::SelRect);
                Ok(())
            }
            Token::FillRect => {
                self.advance();
                self.parse_region_or_var()?;
                self.emit(SpOpcode::SelFillRect);
                Ok(())
            }
            Token::Line => {
                self.advance();
                self.parse_coord_or_var()?;
                self.expect_comma()?;
                self.parse_coord_or_var()?;
                self.emit(SpOpcode::SelLine);
                Ok(())
            }
            Token::RandLine => {
                self.advance();
                self.parse_coord_or_var()?;
                self.expect_comma()?;
                self.parse_coord_or_var()?;
                self.expect_comma()?;
                self.parse_math_expr()?;
                self.emit(SpOpcode::SelRndLine);
                Ok(())
            }
            Token::Grow => {
                self.advance();
                self.expect(&Token::LParen)?;
                // Optional direction arg
                let dir = self.parse_optional_grow_dir();
                self.parse_ter_selection()?;
                self.expect(&Token::RParen)?;
                self.emit_push_int(dir);
                self.emit(SpOpcode::SelGrow);
                Ok(())
            }
            Token::FloodFill => {
                self.advance();
                // floodfill takes (x,y) — parse coord including parens
                self.parse_coord_or_var()?;
                self.emit(SpOpcode::SelFlood);
                Ok(())
            }
            Token::Filter => {
                self.advance();
                self.expect(&Token::LParen)?;
                self.parse_filter_args()?;
                self.expect(&Token::RParen)?;
                Ok(())
            }
            Token::Complement => {
                self.advance();
                self.expect(&Token::LParen)?;
                self.parse_ter_selection_x()?;
                self.expect(&Token::RParen)?;
                self.emit(SpOpcode::SelComplement);
                Ok(())
            }
            Token::Ellipse => {
                self.advance();
                self.expect(&Token::LParen)?;
                self.parse_coord_or_var()?;
                self.expect_comma()?;
                self.parse_math_expr()?; // rx
                self.expect_comma()?;
                self.parse_math_expr()?; // ry
                let fill = if self.peek() == &Token::Comma {
                    self.advance();
                    self.parse_integer()?
                } else {
                    1
                };
                self.expect(&Token::RParen)?;
                self.emit_push_int(fill);
                self.emit(SpOpcode::SelEllipse);
                Ok(())
            }
            Token::Circle => {
                self.advance();
                self.expect(&Token::LParen)?;
                self.parse_coord_or_var()?;
                self.expect_comma()?;
                self.parse_math_expr()?; // radius
                let fill = if self.peek() == &Token::Comma {
                    self.advance();
                    self.parse_integer()?
                } else {
                    1
                };
                self.expect(&Token::RParen)?;
                self.emit(SpOpcode::Copy);
                self.emit_push_int(fill);
                self.emit(SpOpcode::SelEllipse);
                Ok(())
            }
            Token::Gradient => {
                self.advance();
                self.expect(&Token::LParen)?;
                let grad_type = match self.peek() {
                    Token::Radial => {
                        self.advance();
                        0i64
                    }
                    Token::Square => {
                        self.advance();
                        1i64
                    }
                    _ => return Err(self.err("expected gradient type")),
                };
                self.expect_comma()?;
                self.parse_math_expr()?; // range
                self.expect_comma()?;
                self.parse_coord_or_var()?; // center
                let limited = if self.peek() == &Token::Comma {
                    self.advance();
                    self.parse_integer()?
                } else {
                    0
                };
                self.expect(&Token::RParen)?;
                self.emit_push_int(limited);
                self.emit_push_int(grad_type);
                self.emit(SpOpcode::SelGradient);
                Ok(())
            }
            Token::Variable(name) => {
                self.advance();
                if self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_integer()?;
                    self.expect(&Token::RBracket)?;
                    self.emit_push_int(idx);
                }
                self.emit_push_var(&name);
                Ok(())
            }
            Token::Random => {
                self.advance();
                self.emit_push_coord(-1, -1, true, 0);
                self.emit(SpOpcode::SelPoint);
                Ok(())
            }
            _ => Err(self.err("expected selection expression")),
        }
    }

    fn parse_optional_grow_dir(&mut self) -> i64 {
        // W_ANY = 15
        let mut dir = 0i64;
        let mut found = false;
        loop {
            match self.peek() {
                Token::North => {
                    self.advance();
                    dir |= 1;
                    found = true;
                }
                Token::South => {
                    self.advance();
                    dir |= 2;
                    found = true;
                }
                Token::East => {
                    self.advance();
                    dir |= 4;
                    found = true;
                }
                Token::West => {
                    self.advance();
                    dir |= 8;
                    found = true;
                }
                _ => break,
            }
            if self.peek() == &Token::Comma {
                // peek at what follows — if it's another direction, consume comma
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len() {
                    match self.tokens[next_pos].value {
                        Token::North | Token::South | Token::East | Token::West => {
                            self.advance(); // consume comma
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }
        }
        if found {
            if self.peek() == &Token::Comma {
                self.advance();
            }
            dir
        } else {
            15 // W_ANY
        }
    }

    fn parse_filter_args(&mut self) -> Result<(), DesParseError> {
        match self.peek().clone() {
            Token::Integer(_) | Token::Percent(_) => {
                // filter(percent, selection)
                let pct = match self.peek().clone() {
                    Token::Integer(n) => {
                        self.advance();
                        n
                    }
                    Token::Percent(n) => {
                        self.advance();
                        n
                    }
                    _ => unreachable!(),
                };
                self.expect_comma()?;
                self.parse_ter_selection()?;
                self.emit_push_int(pct);
                self.emit_push_int(0); // SPOFILTER_PERCENT
                self.emit(SpOpcode::SelFilter);
                Ok(())
            }
            Token::Char(_) => {
                // filter(mapchar, selection)
                self.parse_mapchar_or_var()?;
                self.expect_comma()?;
                self.parse_ter_selection()?;
                self.emit_push_int(2); // SPOFILTER_MAPCHAR
                self.emit(SpOpcode::SelFilter);
                Ok(())
            }
            _ => {
                // filter(selection, selection)
                self.parse_ter_selection()?;
                self.expect_comma()?;
                self.parse_ter_selection()?;
                self.emit_push_int(1); // SPOFILTER_SELECTION
                self.emit(SpOpcode::SelFilter);
                Ok(())
            }
        }
    }

    // ---- Statement parsers ----

    fn parse_flags(&mut self) -> Result<(), DesParseError> {
        self.advance(); // FLAGS
        self.expect_colon()?;
        let mut flags = LevelFlags::empty();
        while let Token::FlagType(ref name) = self.peek().clone() {
            let f = match name.as_str() {
                "noteleport" => LevelFlags::NOTELEPORT,
                "hardfloor" => LevelFlags::HARDFLOOR,
                "nommap" => LevelFlags::NOMMAP,
                "shortsighted" => LevelFlags::SHORTSIGHTED,
                "arboreal" => LevelFlags::ARBOREAL,
                "mazelevel" => LevelFlags::MAZELEVEL,
                "premapped" => LevelFlags::PREMAPPED,
                "shroud" => LevelFlags::SHROUD,
                "graveyard" => LevelFlags::GRAVEYARD,
                "icedpools" => LevelFlags::ICEDPOOLS,
                "solidify" => LevelFlags::SOLIDIFY,
                "corrmaze" => LevelFlags::CORRMAZE,
                "inaccessibles" => LevelFlags::CHECK_INACCESSIBLES,
                _ => return Err(self.err(&format!("unknown flag: {name}"))),
            };
            flags |= f;
            self.advance();
            if self.peek() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.emit_push_int(flags.bits() as i64);
        self.emit(SpOpcode::LevelFlags);
        Ok(())
    }

    fn parse_init_map(&mut self) -> Result<(), DesParseError> {
        self.advance(); // INIT_MAP
        self.expect_colon()?;

        let style = match self.peek().clone() {
            Token::Mines => {
                self.advance();
                3i64
            }
            Token::SolidFill => {
                self.advance();
                1
            }
            Token::MazeGrid => {
                self.advance();
                2
            }
            Token::RogueLev => {
                self.advance();
                4
            }
            _ => return Err(self.err("expected init map style")),
        };

        self.emit_push_int(style);

        if style == 3 {
            // mines: fg, bg, smoothed, joined, lit, walled
            // INIT_MAP: mines, '.' , ' ' , true , true , random , true
            self.expect_comma()?;
            let fg = self.parse_char_or_random()?;
            self.emit_push_int(fg);
            self.expect_comma()?;
            let bg = self.parse_char_or_random()?;
            // For mines: params order is style, filling(fg), walled, lit, joined, smoothed, bg, fg
            // Actually looking at the C: LVLINIT_MINES, filling, walled, lit, joined, smoothed, bg, fg
            // Let me just push the remaining params
            self.expect_comma()?;
            let smoothed = self.parse_bool_or_random()?;
            self.expect_comma()?;
            let joined = self.parse_bool_or_random()?;
            self.expect_comma()?;
            let lit = self.parse_bool_or_random()?;
            self.expect_comma()?;
            let walled = self.parse_bool_or_random()?;
            // Push remaining 6 params: filling=fg, walled, lit, joined, smoothed, bg
            // Actually the C order for INITLEVEL is 8 pushes total:
            // style, filling, smoothed, lit, joined, hushed, bg, fg
            // We already pushed style. Now push the rest.
            // Wait - we already pushed style and fg... Let me re-read the pattern.
            // Actually in C it's: add_opvars(splev, "iiiiiiiio",
            //   LVLINIT_MINES, filling, walled, lit, joined, smoothed, bg, fg, SPO_INITLEVEL)
            // So we push: style, filling, walled, lit, joined, smoothed, bg, fg
            // We pushed style. Now filling=fg_char above. Then the rest.
            // But we already pushed fg as the filling. Let me emit the remaining 5.
            self.emit_push_int(walled);
            self.emit_push_int(lit);
            self.emit_push_int(joined);
            self.emit_push_int(smoothed);
            self.emit_push_int(bg);
        } else if style == 4 {
            // rogue: no extra params
            for _ in 0..6 {
                self.emit_push_int(0);
            }
        } else {
            // solidfill/mazegrid: filling char, then zeros
            self.expect_comma()?;
            let filling = self.parse_char_or_random()?;
            self.emit_push_int(filling);
            for _ in 0..5 {
                self.emit_push_int(0);
            }
        }

        self.emit(SpOpcode::InitLevel);
        Ok(())
    }

    /// Parse a terrain character or `random`, converting through `what_map_char`
    /// to match C's `terrain_type` rule which returns map type integers.
    fn parse_char_or_random(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::Char(c) => {
                self.advance();
                Ok(what_map_char(c) as i64)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected char or random")),
        }
    }

    fn parse_bool_or_random(&mut self) -> Result<i64, DesParseError> {
        match self.peek() {
            Token::BoolTrue => {
                self.advance();
                Ok(1)
            }
            Token::BoolFalse => {
                self.advance();
                Ok(0)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            Token::Lit => {
                self.advance();
                Ok(1)
            }
            Token::Unlit => {
                self.advance();
                Ok(0)
            }
            _ => Err(self.err("expected true, false, or random")),
        }
    }

    fn parse_geometry(&mut self) -> Result<(), DesParseError> {
        self.advance(); // GEOMETRY
        self.expect_colon()?;
        let h = self.parse_halign()?;
        self.expect_comma()?;
        let v = self.parse_valign()?;
        // C's `roomfill` production defaults to 1 when not explicitly specified
        self.roomfill = 1;
        self.emit_push_coord(h, v, false, 0);
        self.emit_push_int(1); // has geometry
        self.emit_push_int(self.roomfill);
        Ok(())
    }

    fn parse_halign(&mut self) -> Result<i16, DesParseError> {
        match self.peek() {
            Token::Left => {
                self.advance();
                Ok(1)
            }
            Token::HalfLeft => {
                self.advance();
                Ok(2)
            }
            Token::Center => {
                self.advance();
                Ok(3)
            }
            Token::HalfRight => {
                self.advance();
                Ok(4)
            }
            Token::Right => {
                self.advance();
                Ok(5)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected horizontal alignment")),
        }
    }

    fn parse_valign(&mut self) -> Result<i16, DesParseError> {
        match self.peek() {
            Token::Top => {
                self.advance();
                Ok(1)
            }
            Token::Center => {
                self.advance();
                Ok(3)
            }
            Token::Bottom => {
                self.advance();
                Ok(5)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected vertical alignment")),
        }
    }

    fn parse_nomap(&mut self) -> Result<(), DesParseError> {
        self.advance(); // NOMAP
        // C: add_opvars(splev, "ciisiio",
        //     VA_PASS7(0, 0, 1, (char *) 0, 0, 0, SPO_MAP));
        self.emit_push_coord(0, 0, false, 0);
        self.emit_push_int(0); // not has_geom
        self.emit_push_int(1); // nomap marker
        self.emit_push_str("");
        self.emit_push_int(0);
        self.emit_push_int(0);
        self.emit(SpOpcode::Map);
        Ok(())
    }

    fn parse_map_statement(&mut self) -> Result<(), DesParseError> {
        self.advance(); // Map token
        // Next token should be MapData
        let map_data = match self.peek().clone() {
            Token::MapData(s) => {
                self.advance();
                s
            }
            _ => return Err(self.err("expected map data after MAP")),
        };

        // Replicate C's scan_map(): strip digits, convert chars, pad rows
        let converted = scan_map(&map_data);
        self.emit_push_str(&converted.data);
        self.emit_push_int(converted.height as i64);
        self.emit_push_int(converted.width as i64);
        self.emit(SpOpcode::Map);
        Ok(())
    }

    fn parse_message(&mut self) -> Result<(), DesParseError> {
        self.advance(); // MESSAGE
        self.expect_colon()?;
        self.parse_string_expr()?;
        self.emit(SpOpcode::Message);
        Ok(())
    }

    fn parse_monster(&mut self) -> Result<(), DesParseError> {
        self.advance(); // MONSTER
        self.expect_colon()?;

        // C: monster_desc = monster_or_var ',' coord_or_var monster_infos
        // monster_or_var pushes monster spec first
        self.parse_monster_or_var()?;
        self.expect_comma()?;
        // coord_or_var pushes coord
        self.parse_coord_or_var()?;

        // monster_infos base case pushes End sentinel
        self.emit_push_int(SpMonVarFlag::End as i64);

        // Parse optional modifiers after the coordinate
        self.parse_monster_modifiers()?;

        // Emit count (0 = no inventory)
        self.emit_push_int(0);
        self.emit(SpOpcode::Monster);
        Ok(())
    }

    fn parse_monster_modifiers(&mut self) -> Result<(), DesParseError> {
        while self.peek() == &Token::Comma {
            self.advance();
            match self.peek().clone() {
                Token::Peaceful => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Peaceful as i64);
                }
                Token::Hostile => {
                    self.advance();
                    self.emit_push_int(0);
                    self.emit_push_int(SpMonVarFlag::Peaceful as i64);
                }
                Token::Asleep => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Asleep as i64);
                }
                Token::Awake => {
                    self.advance();
                    self.emit_push_int(0);
                    self.emit_push_int(SpMonVarFlag::Asleep as i64);
                }
                Token::Female => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Female as i64);
                }
                Token::Invisible => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Invis as i64);
                }
                Token::Cancelled => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Cancelled as i64);
                }
                Token::Revived => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Revived as i64);
                }
                Token::Avenge => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Avenge as i64);
                }
                Token::Stunned => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Stunned as i64);
                }
                Token::Confused => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpMonVarFlag::Confused as i64);
                }
                Token::Alignment(ref s) => {
                    // noalign for aligned priests, etc.
                    let val = match s.as_str() {
                        "noalign" => 4i64,
                        "law" => 1,
                        "neutral" => 0,
                        "chaos" => -1,
                        _ => 0,
                    };
                    self.advance();
                    self.emit_push_int(val);
                    self.emit_push_int(SpMonVarFlag::Align as i64);
                }
                Token::MObject | Token::MFeature | Token::MMonster => {
                    let appear_type = match self.peek() {
                        Token::MObject => 2,
                        Token::MFeature => 0,
                        Token::MMonster => 1,
                        _ => unreachable!(),
                    };
                    self.advance();
                    self.parse_string_expr()?;
                    self.emit_push_int(appear_type);
                    self.emit_push_int(SpMonVarFlag::Appear as i64);
                }
                Token::Name => {
                    self.advance();
                    self.expect_colon()?;
                    self.parse_string_expr()?;
                    self.emit_push_int(SpMonVarFlag::Name as i64);
                }
                Token::String(_) => {
                    // Bare string as monster proper name
                    self.parse_string_expr()?;
                    self.emit_push_int(SpMonVarFlag::Name as i64);
                }
                _ => {
                    // Unknown modifier — back up and stop
                    break;
                }
            }
        }
        Ok(())
    }

    fn parse_object(&mut self) -> Result<(), DesParseError> {
        self.advance(); // OBJECT
        self.expect_colon()?;

        // C: object_desc = object_or_var object_infos
        // object_or_var pushes the object spec FIRST
        self.parse_object_or_var()?;

        // C: object_infos base case pushes End sentinel
        self.emit_push_int(SpObjVarFlag::End as i64);

        // Coordinate is a modifier in C's grammar (object_info: coord_or_var)
        if self.peek() == &Token::Comma {
            // Peek past comma to see if next looks like a coord or modifier
            let next_pos = self.pos + 1;
            let next_is_coord = if next_pos < self.tokens.len() {
                matches!(
                    self.tokens[next_pos].value,
                    Token::LParen | Token::Random | Token::Variable(_) | Token::RndCoord
                )
            } else {
                false
            };

            if next_is_coord {
                self.advance(); // consume comma
                self.parse_coord_or_var()?;
                self.emit_push_int(SpObjVarFlag::Coord as i64);
            } else if !self.is_in_container() {
                // No coord and not in container — push random coord
                self.emit_push_coord(-1, -1, true, 0);
                self.emit_push_int(SpObjVarFlag::Coord as i64);
            }
        } else if !self.is_in_container() {
            // No coord and not in container — push random coord
            self.emit_push_coord(-1, -1, true, 0);
            self.emit_push_int(SpObjVarFlag::Coord as i64);
        }

        // Parse optional modifiers
        self.parse_object_modifiers()?;

        let cnt = if self.container_depth > 0 { 1 } else { 0 };
        self.emit_push_int(cnt);
        self.emit(SpOpcode::Object);
        Ok(())
    }

    fn parse_object_modifiers(&mut self) -> Result<(), DesParseError> {
        while self.peek() == &Token::Comma {
            self.advance();
            match self.peek().clone() {
                Token::CurseType(ref ct) => {
                    let val = match ct.as_str() {
                        "blessed" => 1i64,
                        "uncursed" => 2,
                        "cursed" => 3,
                        _ => return Err(self.err("unknown curse type")),
                    };
                    self.advance();
                    self.emit_push_int(val);
                    self.emit_push_int(SpObjVarFlag::Curse as i64);
                }
                Token::MonType => {
                    self.advance();
                    self.expect_colon()?;
                    // montype can be "string" or 'c' (char class)
                    match self.peek().clone() {
                        Token::Char(c) => {
                            self.advance();
                            self.emit_push_str(&c.to_string());
                        }
                        _ => {
                            self.parse_string_expr()?;
                        }
                    }
                    self.emit_push_int(SpObjVarFlag::CorpseNm as i64);
                }
                Token::Name => {
                    self.advance();
                    self.expect_colon()?;
                    self.parse_string_expr()?;
                    self.emit_push_int(SpObjVarFlag::Name as i64);
                }
                Token::Quantity => {
                    self.advance();
                    self.expect_colon()?;
                    self.parse_integer_or_var()?;
                    self.emit_push_int(SpObjVarFlag::Quan as i64);
                }
                Token::Buried => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Buried as i64);
                }
                Token::Lit => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Lit as i64);
                }
                Token::Unlit => {
                    self.advance();
                    self.emit_push_int(0);
                    self.emit_push_int(SpObjVarFlag::Lit as i64);
                }
                Token::Eroded => {
                    self.advance();
                    self.parse_integer_or_var()?;
                    self.emit_push_int(SpObjVarFlag::Eroded as i64);
                }
                Token::ErodeProof => {
                    self.advance();
                    self.emit_push_int(-1);
                    self.emit_push_int(SpObjVarFlag::Eroded as i64);
                }
                Token::DoorState(ref s) if s == "locked" => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Locked as i64);
                }
                Token::Trapped => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Trapped as i64);
                }
                Token::NotTrapped => {
                    self.advance();
                    self.emit_push_int(0);
                    self.emit_push_int(SpObjVarFlag::Trapped as i64);
                }
                Token::Recharged => {
                    self.advance();
                    self.parse_integer_or_var()?;
                    self.emit_push_int(SpObjVarFlag::Recharged as i64);
                }
                Token::Invisible => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Invis as i64);
                }
                Token::Greased => {
                    self.advance();
                    self.emit_push_int(1);
                    self.emit_push_int(SpObjVarFlag::Greased as i64);
                }
                Token::Integer(n) => {
                    // Bare integer after object = spe value
                    self.advance();
                    self.emit_push_int(n);
                    self.emit_push_int(SpObjVarFlag::Spe as i64);
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_container(&mut self) -> Result<(), DesParseError> {
        self.advance(); // CONTAINER
        self.expect_colon()?;

        // C: COBJECT uses same object_desc as OBJECT
        // object_or_var pushes object spec first
        self.parse_object_or_var()?;

        // sentinel pushed after object spec
        self.emit_push_int(SpObjVarFlag::End as i64);

        self.expect_comma()?;

        // Optional trapped state
        let mut trapped = -1i64;
        match self.peek() {
            Token::Trapped => {
                self.advance();
                trapped = 1;
                self.expect_comma()?;
            }
            Token::NotTrapped => {
                self.advance();
                trapped = 0;
                self.expect_comma()?;
            }
            _ => {}
        }

        // Coord is an object_info modifier (with SP_O_V_COORD flag)
        self.parse_coord_or_var()?;
        self.emit_push_int(SpObjVarFlag::Coord as i64);

        if trapped >= 0 {
            self.emit_push_int(trapped);
            self.emit_push_int(SpObjVarFlag::Trapped as i64);
        }

        // Parse additional modifiers
        self.parse_object_modifiers()?;

        let cnt = 2 | if self.container_depth > 0 { 1 } else { 0 }; // SP_OBJ_CONTAINER
        self.emit_push_int(cnt);
        self.emit(SpOpcode::Object);

        self.container_depth += 1;
        // Parse block
        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;
        self.container_depth -= 1;

        self.emit(SpOpcode::PopContainer);
        Ok(())
    }

    fn parse_trap(&mut self) -> Result<(), DesParseError> {
        self.advance(); // TRAP
        self.expect_colon()?;
        // C: TRAP_ID ':' trap_name ',' coord_or_var
        //    add_opvars("io", trap_type, SPO_TRAP)
        // trap_name is parsed first (value captured), then coord pushes to stack,
        // then trap_type is pushed as Int
        let trap_id = match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                get_trap_type(&s).unwrap_or(-1)
            }
            Token::Random => {
                self.advance();
                -1
            }
            Token::Integer(n) => {
                self.advance();
                n
            }
            _ => return Err(self.err("expected trap type")),
        };
        self.expect_comma()?;
        self.parse_coord_or_var()?;
        self.emit_push_int(trap_id);
        self.emit(SpOpcode::Trap);
        Ok(())
    }

    fn parse_door(&mut self) -> Result<(), DesParseError> {
        self.advance(); // DOOR
        self.expect_colon()?;
        let state = self.parse_door_state()?;
        self.expect_comma()?;
        // C: ter_selection goes on stack first, then Int(state), then Door
        self.parse_ter_selection()?;
        self.emit_push_int(state);
        self.emit(SpOpcode::Door);
        Ok(())
    }

    fn parse_door_state(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::DoorState(ref s) => {
                let val = match s.as_str() {
                    "open" => 1i64,
                    "closed" => 2,
                    "locked" => 4,
                    "nodoor" => 8,
                    "broken" => 16,
                    "secret" => 32,
                    _ => return Err(self.err(&format!("unknown door state: {s}"))),
                };
                self.advance();
                Ok(val)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected door state")),
        }
    }

    fn parse_roomdoor(&mut self) -> Result<(), DesParseError> {
        self.advance(); // ROOMDOOR
        self.expect_colon()?;
        // secret, state, wall, pos
        let secret = match self.peek() {
            Token::BoolTrue => {
                self.advance();
                1i64
            }
            Token::BoolFalse => {
                self.advance();
                0
            }
            _ => return Err(self.err("expected true/false for secret")),
        };
        self.expect_comma()?;
        let state = self.parse_door_state()?;
        self.expect_comma()?;
        let wall = self.parse_direction()?;
        self.expect_comma()?;
        let pos = match self.peek().clone() {
            Token::Random => {
                self.advance();
                -1i64
            }
            Token::Integer(n) => {
                self.advance();
                n
            }
            _ => return Err(self.err("expected position or random")),
        };
        self.emit_push_int(pos);
        self.emit_push_int(state);
        self.emit_push_int(secret);
        self.emit_push_int(wall);
        self.emit(SpOpcode::RoomDoor);
        Ok(())
    }

    fn parse_single_direction(&mut self) -> Result<i64, DesParseError> {
        match self.peek() {
            Token::North => {
                self.advance();
                Ok(1)
            }
            Token::South => {
                self.advance();
                Ok(2)
            }
            Token::East => {
                self.advance();
                Ok(4)
            }
            Token::West => {
                self.advance();
                Ok(8)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected direction")),
        }
    }

    fn parse_direction(&mut self) -> Result<i64, DesParseError> {
        let mut val = self.parse_single_direction()?;
        // Handle north|south|east|west combinations via '|' (Pipe token)
        while self.peek() == &Token::Pipe {
            self.advance();
            val |= self.parse_single_direction()?;
        }
        Ok(val)
    }

    fn parse_drawbridge(&mut self) -> Result<(), DesParseError> {
        self.advance(); // DRAWBRIDGE
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let dir = self.parse_direction()?;
        self.expect_comma()?;
        let raw_state = self.parse_door_state()?;
        // C normalizes: D_ISOPEN(1)→1, D_CLOSED(2)→0, random(-1)→-1
        let state = match raw_state {
            1 => 1,   // open
            2 => 0,   // closed
            -1 => -1, // random
            _ => 0,
        };
        // C converts W_NORTH/S/E/W(1/2/4/8) → DB_NORTH/S/E/W(0/1/2/3)
        let db_dir = match dir {
            1 => 0, // W_NORTH → DB_NORTH
            2 => 1, // W_SOUTH → DB_SOUTH
            4 => 2, // W_EAST → DB_EAST
            8 => 3, // W_WEST → DB_WEST
            _ => dir,
        };
        self.emit_push_int(state);
        self.emit_push_int(db_dir);
        self.emit(SpOpcode::Drawbridge);
        Ok(())
    }

    fn parse_fountain(&mut self) -> Result<(), DesParseError> {
        self.advance(); // FOUNTAIN
        self.expect_colon()?;
        self.parse_ter_selection_as_coord()?;
        self.emit(SpOpcode::Fountain);
        Ok(())
    }

    fn parse_sink(&mut self) -> Result<(), DesParseError> {
        self.advance(); // SINK
        self.expect_colon()?;
        self.parse_ter_selection_as_coord()?;
        self.emit(SpOpcode::Sink);
        Ok(())
    }

    fn parse_pool(&mut self) -> Result<(), DesParseError> {
        self.advance(); // POOL
        self.expect_colon()?;
        self.parse_ter_selection_as_coord()?;
        self.emit(SpOpcode::Pool);
        Ok(())
    }

    /// Parse a coordinate that becomes a point selection for fountain/sink/pool.
    fn parse_ter_selection_as_coord(&mut self) -> Result<(), DesParseError> {
        self.parse_coord_or_var()?;
        self.emit(SpOpcode::SelPoint);
        Ok(())
    }

    fn parse_ladder(&mut self) -> Result<(), DesParseError> {
        self.advance(); // LADDER
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let dir = self.parse_up_or_down()?;
        self.emit_push_int(dir);
        self.emit(SpOpcode::Ladder);
        Ok(())
    }

    fn parse_stair(&mut self) -> Result<(), DesParseError> {
        self.advance(); // STAIR
        self.expect_colon()?;

        // Could be levregion stair or simple stair
        // Region form: levregion(...) or (x1,y1,x2,y2) with 4 coords
        if self.peek() == &Token::LevRegionKw || self.is_region_4_ahead() {
            return self.parse_stair_region();
        }

        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let dir = self.parse_up_or_down()?;
        self.emit_push_int(dir);
        self.emit(SpOpcode::Stair);
        Ok(())
    }

    fn parse_stair_region(&mut self) -> Result<(), DesParseError> {
        let (x1, y1, x2, y2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let (dx1, dy1, dx2, dy2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let dir = self.parse_up_or_down()?;
        let lr_type = if dir == 1 { 2i64 } else { 3 };

        self.emit_push_int(x1 as i64);
        self.emit_push_int(y1 as i64);
        self.emit_push_int(x2 as i64);
        self.emit_push_int(y2 as i64);
        self.emit_push_int(1);
        self.emit_push_int(dx1 as i64);
        self.emit_push_int(dy1 as i64);
        self.emit_push_int(dx2 as i64);
        self.emit_push_int(dy2 as i64);
        self.emit_push_int(0);
        self.emit_push_int(lr_type);
        self.emit_push_int(0);
        self.emit_push_str("");
        self.emit(SpOpcode::LevRegion);
        Ok(())
    }

    fn parse_levregion_coords(&mut self) -> Result<(i16, i16, i16, i16), DesParseError> {
        self.expect(&Token::LevRegionKw)?;
        self.expect(&Token::LParen)?;
        let x1 = self.parse_integer()? as i16;
        self.expect_comma()?;
        let y1 = self.parse_integer()? as i16;
        self.expect_comma()?;
        let x2 = self.parse_integer()? as i16;
        self.expect_comma()?;
        let y2 = self.parse_integer()? as i16;
        self.expect(&Token::RParen)?;
        Ok((x1, y1, x2, y2))
    }

    fn parse_up_or_down(&mut self) -> Result<i64, DesParseError> {
        match self.peek() {
            Token::Up => {
                self.advance();
                Ok(1)
            }
            Token::Down => {
                self.advance();
                Ok(0)
            }
            _ => Err(self.err("expected up or down")),
        }
    }

    fn parse_altar(&mut self) -> Result<(), DesParseError> {
        self.advance(); // ALTAR
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let align = self.parse_altar_alignment()?;
        self.expect_comma()?;
        let shrine = self.parse_altar_type()?;
        self.emit_push_int(shrine);
        self.emit_push_int(align);
        self.emit(SpOpcode::Altar);
        Ok(())
    }

    fn parse_altar_alignment(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::Alignment(ref s) => {
                let val = match s.as_str() {
                    "noalign" => 0i64,
                    "law" => 1,
                    "neutral" => 0,
                    "chaos" => -1,
                    "coaligned" => 4,
                    "noncoaligned" => 5,
                    _ => return Err(self.err("unknown alignment")),
                };
                self.advance();
                Ok(val)
            }
            Token::AlignReg => {
                self.advance();
                // align[N] syntax
                self.expect(&Token::LBracket)?;
                let n = self.parse_integer()?;
                self.expect(&Token::RBracket)?;
                // Encode as alignment register reference
                Ok(100 + n)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected alignment")),
        }
    }

    fn parse_altar_type(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::AltarType(ref s) => {
                let val = match s.as_str() {
                    "altar" => 0i64,
                    "shrine" => 1,
                    "sanctum" => 2,
                    _ => 0,
                };
                self.advance();
                Ok(val)
            }
            _ => Err(self.err("expected altar type")),
        }
    }

    fn parse_teleport_region(&mut self) -> Result<(), DesParseError> {
        self.advance(); // TELEPORT_REGION
        self.expect_colon()?;

        let (sx1, sy1, sx2, sy2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let (dx1, dy1, dx2, dy2) = self.parse_region_4_coords()?;
        let is_lev = 0i64;

        // Optional direction for tele type
        let mut lr_type = 6i64; // LR_TELE
        if self.peek() == &Token::Comma {
            self.advance();
            match self.peek() {
                Token::Up => {
                    self.advance();
                    lr_type = 4; // LR_UPTELE
                }
                Token::Down => {
                    self.advance();
                    lr_type = 5; // LR_DOWNTELE
                }
                _ => {}
            }
        }

        self.emit_push_int(sx1 as i64);
        self.emit_push_int(sy1 as i64);
        self.emit_push_int(sx2 as i64);
        self.emit_push_int(sy2 as i64);
        self.emit_push_int(1); // src is lev
        self.emit_push_int(dx1 as i64);
        self.emit_push_int(dy1 as i64);
        self.emit_push_int(dx2 as i64);
        self.emit_push_int(dy2 as i64);
        self.emit_push_int(is_lev);
        self.emit_push_int(lr_type);
        self.emit_push_int(0);
        self.emit_push_str("");
        self.emit(SpOpcode::LevRegion);
        Ok(())
    }

    fn parse_branch_region(&mut self) -> Result<(), DesParseError> {
        self.advance(); // BRANCH
        self.expect_colon()?;

        let (sx1, sy1, sx2, sy2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let (dx1, dy1, dx2, dy2) = self.parse_region_4_coords()?;

        self.emit_push_int(sx1 as i64);
        self.emit_push_int(sy1 as i64);
        self.emit_push_int(sx2 as i64);
        self.emit_push_int(sy2 as i64);
        self.emit_push_int(0);
        self.emit_push_int(dx1 as i64);
        self.emit_push_int(dy1 as i64);
        self.emit_push_int(dx2 as i64);
        self.emit_push_int(dy2 as i64);
        self.emit_push_int(0);
        self.emit_push_int(7); // LR_BRANCH
        self.emit_push_int(0);
        self.emit_push_str("");
        self.emit(SpOpcode::LevRegion);
        Ok(())
    }

    /// Look ahead to detect (num,num,num,num) pattern (4-coord region).
    fn is_region_4_ahead(&self) -> bool {
        if self.peek() != &Token::LParen {
            return false;
        }
        // Count commas inside the parenthesized group
        let mut depth = 0;
        let mut commas = 0;
        for i in self.pos..self.tokens.len() {
            match &self.tokens[i].value {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return commas >= 3;
                    }
                }
                Token::Comma if depth == 1 => commas += 1,
                _ => {}
            }
        }
        false
    }

    /// Parse (x1,y1,x2,y2) — either from levregion() or plain parens.
    fn parse_region_4_coords(&mut self) -> Result<(i16, i16, i16, i16), DesParseError> {
        if self.peek() == &Token::LevRegionKw {
            self.parse_levregion_coords()
        } else {
            self.expect(&Token::LParen)?;
            let x1 = self.parse_integer()? as i16;
            self.expect_comma()?;
            let y1 = self.parse_integer()? as i16;
            self.expect_comma()?;
            let x2 = self.parse_integer()? as i16;
            self.expect_comma()?;
            let y2 = self.parse_integer()? as i16;
            self.expect(&Token::RParen)?;
            Ok((x1, y1, x2, y2))
        }
    }

    fn parse_portal_region(&mut self) -> Result<(), DesParseError> {
        self.advance(); // PORTAL
        self.expect_colon()?;
        // Similar to branch but with LR_PORTAL
        let (sx1, sy1, sx2, sy2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let (dx1, dy1, dx2, dy2) = self.parse_region_4_coords()?;
        self.expect_comma()?;
        let name = self.parse_string()?;

        self.emit_push_int(sx1 as i64);
        self.emit_push_int(sy1 as i64);
        self.emit_push_int(sx2 as i64);
        self.emit_push_int(sy2 as i64);
        self.emit_push_int(1);
        self.emit_push_int(dx1 as i64);
        self.emit_push_int(dy1 as i64);
        self.emit_push_int(dx2 as i64);
        self.emit_push_int(dy2 as i64);
        self.emit_push_int(1);
        self.emit_push_int(1); // LR_PORTAL
        self.emit_push_int(0);
        self.emit_push_str(&name);
        self.emit(SpOpcode::LevRegion);
        Ok(())
    }

    fn parse_gold(&mut self) -> Result<(), DesParseError> {
        self.advance(); // GOLD
        self.expect_colon()?;
        self.parse_math_expr()?;
        self.expect_comma()?;
        self.parse_coord_or_var()?;
        self.emit(SpOpcode::Gold);
        Ok(())
    }

    fn parse_engraving(&mut self) -> Result<(), DesParseError> {
        self.advance(); // ENGRAVING
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let engr_type = self.parse_engraving_type()?;
        self.expect_comma()?;
        self.parse_string_expr()?;
        self.emit_push_int(engr_type);
        self.emit(SpOpcode::Engraving);
        Ok(())
    }

    fn parse_engraving_type(&mut self) -> Result<i64, DesParseError> {
        match self.peek().clone() {
            Token::EngravingType(ref s) => {
                let val = match s.as_str() {
                    "dust" => 1i64,
                    "engrave" => 2,
                    "burn" => 3,
                    "mark" => 4,
                    "blood" => 5,
                    _ => return Err(self.err("unknown engraving type")),
                };
                self.advance();
                Ok(val)
            }
            _ => Err(self.err("expected engraving type")),
        }
    }

    fn parse_grave(&mut self) -> Result<(), DesParseError> {
        self.advance(); // GRAVE
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        if self.peek() == &Token::Comma {
            self.advance();
            self.parse_string_expr()?;
            self.emit_push_int(2);
        } else {
            self.emit_push_str("");
            self.emit_push_int(1);
        }
        self.emit(SpOpcode::Grave);
        Ok(())
    }

    fn parse_mazewalk(&mut self) -> Result<(), DesParseError> {
        self.advance(); // MAZEWALK
        self.expect_colon()?;
        self.parse_coord_or_var()?;
        self.expect_comma()?;
        let dir = self.parse_direction()?;

        // Optional: , steppable_floor_bool
        let mut steppable = 1i64; // true by default
        let mut fill = 0i64;
        if self.peek() == &Token::Comma {
            self.advance();
            match self.peek() {
                Token::BoolTrue => {
                    self.advance();
                    steppable = 1;
                }
                Token::BoolFalse => {
                    self.advance();
                    steppable = 0;
                }
                _ => {}
            }
        }
        if self.peek() == &Token::Comma {
            self.advance();
            fill = self.parse_char_or_random()?;
        }

        self.emit_push_int(dir);
        self.emit_push_int(steppable);
        self.emit_push_int(fill);
        self.emit(SpOpcode::MazeWalk);
        Ok(())
    }

    fn parse_wallify(&mut self) -> Result<(), DesParseError> {
        self.advance(); // WALLIFY
        self.emit_push_region(-1, -1, -1, -1);
        self.emit_push_int(0);
        self.emit(SpOpcode::Wallify);
        Ok(())
    }

    fn parse_mineralize(&mut self) -> Result<(), DesParseError> {
        self.advance(); // MINERALIZE
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit(SpOpcode::Mineralize);
        Ok(())
    }

    fn parse_non_diggable(&mut self) -> Result<(), DesParseError> {
        self.advance(); // NON_DIGGABLE
        self.expect_colon()?;
        self.parse_region_or_var()?;
        self.emit(SpOpcode::NonDiggable);
        Ok(())
    }

    fn parse_non_passwall(&mut self) -> Result<(), DesParseError> {
        self.advance(); // NON_PASSWALL
        self.expect_colon()?;
        self.parse_region_or_var()?;
        self.emit(SpOpcode::NonPasswall);
        Ok(())
    }

    fn parse_terrain(&mut self) -> Result<(), DesParseError> {
        self.advance(); // TERRAIN
        self.expect_colon()?;

        // Can be: coord, selection expression, or "selection:" prefix
        self.parse_terrain_selection()?;
        self.expect_comma()?;
        self.parse_mapchar_or_var()?;
        self.emit(SpOpcode::Terrain);
        Ok(())
    }

    fn parse_terrain_selection(&mut self) -> Result<(), DesParseError> {
        // Check for selection function keywords
        match self.peek().clone() {
            Token::FillRect
            | Token::Rect
            | Token::Line
            | Token::RandLine
            | Token::Grow
            | Token::FloodFill
            | Token::Filter
            | Token::Complement
            | Token::Ellipse
            | Token::Circle
            | Token::Gradient => {
                self.parse_ter_selection_x()?;
            }
            Token::LParen => {
                // Could be a coord (2 values) or a selection function in parens
                // like (randline (37,7),(62,02),7)
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len() {
                    match &self.tokens[next_pos].value {
                        Token::RandLine
                        | Token::Line
                        | Token::Rect
                        | Token::FillRect
                        | Token::Grow
                        | Token::FloodFill
                        | Token::Filter
                        | Token::Complement
                        | Token::Ellipse
                        | Token::Circle
                        | Token::Gradient => {
                            // Parenthesized selection function
                            self.advance(); // consume LParen
                            self.parse_ter_selection_x()?;
                            self.expect(&Token::RParen)?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                // Simple coordinate → point selection
                self.parse_coord_or_var()?;
                self.emit(SpOpcode::SelPoint);
            }
            Token::Variable(_) => {
                self.parse_coord_or_var()?;
                // If it's a selection var, no SelPoint needed; if coord, we need it
                // For now, just push the variable (runtime resolves type)
            }
            Token::Random => {
                self.parse_coord_or_var()?;
                self.emit(SpOpcode::SelPoint);
            }
            _ => {
                return Err(self.err("expected terrain selection"));
            }
        }
        Ok(())
    }

    fn parse_replace_terrain(&mut self) -> Result<(), DesParseError> {
        self.advance(); // REPLACE_TERRAIN
        self.expect_colon()?;
        self.parse_region_or_var()?;
        self.expect_comma()?;
        self.parse_mapchar_or_var()?; // from terrain
        self.expect_comma()?;
        self.parse_mapchar_or_var()?; // to terrain
        self.expect_comma()?;
        // percentage
        let pct = match self.peek().clone() {
            Token::Percent(n) => {
                self.advance();
                n
            }
            Token::Integer(n) => {
                self.advance();
                n
            }
            _ => return Err(self.err("expected percentage")),
        };
        self.emit_push_int(pct);
        self.emit(SpOpcode::ReplaceTerrain);
        Ok(())
    }

    fn parse_region(&mut self) -> Result<(), DesParseError> {
        self.advance(); // REGION
        self.expect_colon()?;
        self.parse_region_or_var()?;
        self.expect_comma()?;

        let lit = match self.peek() {
            Token::Lit => {
                self.advance();
                1i64
            }
            Token::Unlit => {
                self.advance();
                0
            }
            Token::Random => {
                self.advance();
                -1
            }
            _ => -1,
        };
        self.expect_comma()?;
        let room_type_str = self.parse_string()?;
        let room_type = room_type_to_int(&room_type_str);

        // Optional modifiers: filled/unfilled, irregular, joined
        let mut region_flags = 0i64;
        while self.peek() == &Token::Comma {
            self.advance();
            match self.peek() {
                Token::Filled => {
                    self.advance();
                    region_flags |= 1;
                }
                Token::Unfilled => {
                    self.advance();
                    // nothing
                }
                Token::Irregular => {
                    self.advance();
                    region_flags |= 2;
                }
                Token::Regular => {
                    self.advance();
                }
                Token::Joined => {
                    self.advance();
                }
                Token::Unjoined => {
                    self.advance();
                    region_flags |= 4;
                }
                _ => break,
            }
        }

        self.emit_push_int(lit);
        self.emit_push_int(room_type);
        self.emit_push_int(region_flags);
        self.emit(SpOpcode::Region);

        // Optional block with inline statements (e.g. ROOMDOOR inside REGION)
        if self.peek() == &Token::LBrace {
            self.advance();
            self.parse_block()?;
            self.expect(&Token::RBrace)?;
        }
        Ok(())
    }

    fn parse_room(&mut self, is_sub: bool) -> Result<(), DesParseError> {
        self.advance(); // ROOM or SUBROOM
        self.expect_colon()?;

        // room_begin: type [pct%], lit
        let room_type_str = self.parse_string()?;
        let room_type = room_type_to_int(&room_type_str);

        let chance = if let Token::Percent(n) = *self.peek() {
            self.advance();
            n
        } else {
            100
        };

        self.expect_comma()?;

        let lit = self.parse_lit_state()?;
        self.expect_comma()?;

        if is_sub {
            // SUBROOM: room_begin, subroom_pos(x,y), room_size(w,h)
            let (x, y) = self.parse_int_pair()?;
            self.expect_comma()?;
            let (w, h) = self.parse_int_pair()?;

            let flags = self.parse_optional_room_flags()?;

            self.emit_push_int(room_type);
            self.emit_push_int(chance);
            self.emit_push_int(lit);
            self.emit_push_int(flags);
            self.emit_push_int(-1); // h_just (unused for sub)
            self.emit_push_int(-1); // v_just
            self.emit_push_int(x);
            self.emit_push_int(y);
            self.emit_push_int(w);
            self.emit_push_int(h);
            self.emit(SpOpcode::Subroom);
        } else {
            // ROOM: room_begin, room_pos(x,y), room_align(h,v), room_size(w,h)
            let (pos_x, pos_y) = self.parse_pair_or_random()?;
            self.expect_comma()?;
            let (align_h, align_v) = self.parse_pair_or_random()?;
            self.expect_comma()?;
            let (w, h) = self.parse_pair_or_random()?;

            let flags = self.parse_optional_room_flags()?;

            self.emit_push_int(room_type);
            self.emit_push_int(chance);
            self.emit_push_int(lit);
            self.emit_push_int(flags);
            self.emit_push_int(align_h);
            self.emit_push_int(align_v);
            self.emit_push_int(pos_x);
            self.emit_push_int(pos_y);
            self.emit_push_int(w);
            self.emit_push_int(h);
            self.emit(SpOpcode::Room);
        }

        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;
        self.emit(SpOpcode::EndRoom);
        Ok(())
    }

    fn parse_lit_state(&mut self) -> Result<i64, DesParseError> {
        match self.peek() {
            Token::Lit => {
                self.advance();
                Ok(1)
            }
            Token::Unlit => {
                self.advance();
                Ok(0)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            _ => Err(self.err("expected lit, unlit, or random")),
        }
    }

    fn parse_int_pair(&mut self) -> Result<(i64, i64), DesParseError> {
        self.expect(&Token::LParen)?;
        let a = self.parse_integer()?;
        self.expect_comma()?;
        let b = self.parse_integer()?;
        self.expect(&Token::RParen)?;
        Ok((a, b))
    }

    fn parse_pair_or_random(&mut self) -> Result<(i64, i64), DesParseError> {
        match self.peek() {
            Token::Random => {
                self.advance();
                Ok((-1, -1))
            }
            Token::LParen => {
                self.expect(&Token::LParen)?;
                let a = self.parse_room_align_or_random()?;
                self.expect_comma()?;
                let b = self.parse_room_align_or_random()?;
                self.expect(&Token::RParen)?;
                Ok((a, b))
            }
            _ => Err(self.err("expected (a,b) or random")),
        }
    }

    fn parse_optional_room_flags(&mut self) -> Result<i64, DesParseError> {
        // C's `optroomregionflags` returns -1 when no flags are specified.
        // The ROOM/SUBROOM rule then converts -1 to (1 << 0) = 1 (filled).
        let mut flags = -1i64;
        let mut has_flags = false;
        while self.peek() == &Token::Comma {
            let next_pos = self.pos + 1;
            if next_pos < self.tokens.len() {
                match &self.tokens[next_pos].value {
                    Token::Filled
                    | Token::Unfilled
                    | Token::Irregular
                    | Token::Regular
                    | Token::Joined
                    | Token::Unjoined => {
                        self.advance(); // comma
                    }
                    _ => break,
                }
            } else {
                break;
            }
            if !has_flags {
                flags = 0;
                has_flags = true;
            }
            match self.peek() {
                Token::Filled => {
                    self.advance();
                    flags |= 1;
                }
                Token::Unfilled => {
                    self.advance();
                }
                Token::Irregular => {
                    self.advance();
                    flags |= 2;
                }
                Token::Regular => {
                    self.advance();
                }
                Token::Joined => {
                    self.advance();
                }
                Token::Unjoined => {
                    self.advance();
                    flags |= 4;
                }
                _ => break,
            }
        }
        // C's room_def converts -1 (no flags) to 1 (filled)
        if flags == -1 {
            flags = 1;
        }
        Ok(flags)
    }

    fn parse_room_align_or_random(&mut self) -> Result<i64, DesParseError> {
        match self.peek() {
            Token::Left => {
                self.advance();
                Ok(1)
            }
            Token::HalfLeft => {
                self.advance();
                Ok(2)
            }
            Token::Center => {
                self.advance();
                Ok(3)
            }
            Token::HalfRight => {
                self.advance();
                Ok(4)
            }
            Token::Right => {
                self.advance();
                Ok(5)
            }
            Token::Top => {
                self.advance();
                Ok(1)
            }
            Token::Bottom => {
                self.advance();
                Ok(5)
            }
            Token::Random => {
                self.advance();
                Ok(-1)
            }
            Token::Integer(n) => {
                let val = *n;
                self.advance();
                Ok(val)
            }
            _ => Err(self.err("expected alignment or random")),
        }
    }

    fn parse_corridor(&mut self) -> Result<(), DesParseError> {
        self.advance(); // CORRIDOR
        self.expect_colon()?;
        // src_room, src_door, src_wall, dst_room, dst_door, dst_wall
        let r1 = self.parse_integer()?;
        self.expect_comma()?;
        let d1 = self.parse_integer()?;
        self.expect_comma()?;
        let w1 = self.parse_integer()?;
        self.expect_comma()?;
        let r2 = self.parse_integer()?;
        self.expect_comma()?;
        let d2 = self.parse_integer()?;
        self.expect_comma()?;
        let w2 = self.parse_integer()?;
        self.emit_push_int(r1);
        self.emit_push_int(d1);
        self.emit_push_int(w1);
        self.emit_push_int(r2);
        self.emit_push_int(d2);
        self.emit_push_int(w2);
        self.emit(SpOpcode::Corridor);
        Ok(())
    }

    fn parse_random_corridors(&mut self) -> Result<(), DesParseError> {
        self.advance(); // RANDOM_CORRIDORS
        // C: add_opvars("iiiiiio", -1, 0, -1, -1, -1, -1, SPO_CORRIDOR)
        self.emit_push_int(-1);
        self.emit_push_int(0); // corridor count default (not -1)
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit_push_int(-1);
        self.emit(SpOpcode::Corridor);
        Ok(())
    }

    fn parse_exit(&mut self) -> Result<(), DesParseError> {
        self.advance(); // EXIT
        self.emit(SpOpcode::Exit);
        Ok(())
    }

    fn parse_shuffle(&mut self) -> Result<(), DesParseError> {
        self.advance(); // SHUFFLE
        self.expect_colon()?;
        match self.peek().clone() {
            Token::Variable(name) => {
                self.advance();
                // C preserves $ prefix in variable names
                let var_name = if name.starts_with('$') {
                    name
                } else {
                    format!("${name}")
                };
                self.emit_push_str(&var_name);
                self.emit(SpOpcode::ShuffleArray);
                Ok(())
            }
            _ => Err(self.err("expected variable for SHUFFLE")),
        }
    }

    // ---- Control flow ----

    fn parse_if(&mut self) -> Result<(), DesParseError> {
        self.advance(); // IF

        // Condition: [pct%] or comparison
        let jmp_idx = match self.peek().clone() {
            Token::Percent(pct) => {
                self.advance();
                self.emit_percent_condition(pct)
            }
            Token::LBracket => {
                // Bracketed condition: [$var == expr] or [expr op expr]
                self.advance(); // consume [
                self.parse_math_expr()?; // left side
                let jmp_op = self.parse_comparison_op()?;
                self.parse_math_expr()?; // right side
                self.expect(&Token::RBracket)?;
                self.emit(SpOpcode::Cmp);
                let idx = self.current_offset();
                self.emit_push_int(idx as i64 + 1);
                self.emit(jmp_op);
                idx
            }
            _ => {
                // General condition: truthy check (expr != 0)
                self.parse_math_expr()?;
                self.emit_push_int(0);
                self.emit(SpOpcode::Cmp);
                let idx = self.current_offset();
                self.emit_push_int(idx as i64 + 1);
                self.emit(SpOpcode::Jne);
                idx
            }
        };

        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;

        if self.peek() == &Token::Else {
            self.advance();
            // Jump past else block
            let else_jmp_idx = self.current_offset();
            self.emit_push_int(else_jmp_idx as i64 + 1);
            self.emit(SpOpcode::Jmp);
            // Patch the if-false jump to here
            self.patch_jump(jmp_idx);

            self.expect(&Token::LBrace)?;
            self.parse_block()?;
            self.expect(&Token::RBrace)?;
            self.patch_jump(else_jmp_idx);
        } else {
            self.patch_jump(jmp_idx);
        }

        Ok(())
    }

    fn parse_for(&mut self) -> Result<(), DesParseError> {
        self.advance(); // FOR

        // FOR $var = start TO end { body }
        let var_name = match self.peek().clone() {
            Token::Variable(name) => {
                self.advance();
                name
            }
            _ => return Err(self.err("expected variable in FOR")),
        };

        self.expect(&Token::Equals)?;
        self.parse_math_expr()?; // start value

        // C uses "$varname end" and "$varname step" (with $ prefix and space separator)
        let full_name = if var_name.starts_with('$') {
            var_name.clone()
        } else {
            format!("${var_name}")
        };
        let end_var = format!("{full_name} end");
        let step_var = format!("{full_name} step");

        self.expect(&Token::To)?;
        self.parse_math_expr()?; // end value

        // Store end value
        self.emit_var_init(&end_var, 0);
        // Store start as loop var
        self.emit_var_init(&var_name, 0);

        // Calculate step = sign(end - start)
        self.emit_push_var(&end_var);
        self.emit_push_var(&var_name);
        self.emit(SpOpcode::MathSub);
        self.emit(SpOpcode::MathSign);
        self.emit_var_init(&step_var, 0);

        let loop_start = self.current_offset();

        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;

        // Compare and loop back
        self.emit_push_var(&var_name);
        self.emit_push_var(&end_var);
        self.emit(SpOpcode::Cmp);
        // Increment
        self.emit_push_var(&step_var);
        self.emit_push_var(&var_name);
        self.emit(SpOpcode::MathAdd);
        self.emit_var_init(&var_name, 0);
        // Jump back if not equal
        let jmp_offset = loop_start as i64 - self.current_offset() as i64 - 1;
        self.emit_push_int(jmp_offset);
        self.emit(SpOpcode::Jne);

        // Track variable
        self.vars.insert(
            var_name,
            VarDef {
                typ: VarType::Int,
                is_array: false,
            },
        );

        Ok(())
    }

    fn parse_loop(&mut self) -> Result<(), DesParseError> {
        self.advance(); // LOOP

        // LOOP [count] { body }
        // Count can be [N + MdK] or just [expr]
        self.expect(&Token::LBracket)?;
        self.parse_math_expr()?;
        self.expect(&Token::RBracket)?;

        let loop_top = self.current_offset();
        self.emit(SpOpcode::Dec);

        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;

        // Copy count, compare to 0, jump back if > 0
        self.emit(SpOpcode::Copy);
        self.emit_push_int(0);
        self.emit(SpOpcode::Cmp);
        let jmp_offset = loop_top as i64 - self.current_offset() as i64 - 1;
        self.emit_push_int(jmp_offset);
        self.emit(SpOpcode::Jg);
        self.emit(SpOpcode::Pop); // discard counter

        Ok(())
    }

    fn parse_switch(&mut self) -> Result<(), DesParseError> {
        self.advance(); // SWITCH
        self.expect(&Token::LBracket)?;
        self.parse_math_expr()?;
        self.expect(&Token::RBracket)?;

        // Jump to the case-checking section
        let check_jmp_idx = self.current_offset();
        self.emit_push_int(check_jmp_idx as i64 + 1);
        self.emit(SpOpcode::Jmp);

        // Collect case bodies and default
        let mut case_addresses: Vec<(i64, usize)> = Vec::new(); // (case_value, body_start_offset)
        let mut default_address: Option<usize> = None;
        let mut break_targets: Vec<usize> = Vec::new();

        self.expect(&Token::LBrace)?;

        loop {
            match self.peek().clone() {
                Token::Case => {
                    self.advance();
                    let val = self.parse_integer()?;
                    self.expect_colon()?;
                    case_addresses.push((val, self.current_offset()));
                    self.parse_case_body(&mut break_targets)?;
                }
                Token::Default => {
                    self.advance();
                    self.expect_colon()?;
                    default_address = Some(self.current_offset());
                    self.parse_case_body(&mut break_targets)?;
                }
                Token::RBrace => {
                    self.advance();
                    break;
                }
                _ => return Err(self.err("expected CASE, DEFAULT, or '}'")),
            }
        }

        // Now emit the case-checking code
        self.patch_jump(check_jmp_idx);

        for (val, body_addr) in &case_addresses {
            self.emit(SpOpcode::Copy);
            self.emit_push_int(*val);
            self.emit(SpOpcode::Cmp);
            let offset = *body_addr as i64 - self.current_offset() as i64 - 1;
            self.emit_push_int(offset);
            self.emit(SpOpcode::Je);
        }

        if let Some(addr) = default_address {
            let offset = addr as i64 - self.current_offset() as i64 - 1;
            self.emit_push_int(offset);
            self.emit(SpOpcode::Jmp);
        }

        // Pop the switch value
        self.emit(SpOpcode::Pop);

        // Patch all break targets to here
        let end_offset = self.current_offset();
        for idx in break_targets {
            if let Some(SpLevOpcode {
                operand: Some(SpOperand::Int(val)),
                ..
            }) = self.opcodes.get_mut(idx)
            {
                *val = end_offset as i64 - *val;
            }
        }

        Ok(())
    }

    fn parse_case_body(&mut self, break_targets: &mut Vec<usize>) -> Result<(), DesParseError> {
        loop {
            match self.peek() {
                Token::Case | Token::Default | Token::RBrace => break,
                Token::Break => {
                    self.advance();
                    let idx = self.current_offset();
                    self.emit_push_int(idx as i64 + 1);
                    self.emit(SpOpcode::Jmp);
                    break_targets.push(idx);
                }
                _ => {
                    // Check for percent prefix
                    let pct = self.try_percent_prefix()?;
                    if let Some(pct_val) = pct {
                        self.expect_colon()?;
                        self.parse_pct_statement(pct_val)?;
                    } else {
                        self.parse_statement()?;
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_function(&mut self) -> Result<(), DesParseError> {
        self.advance(); // FUNCTION
        // Function definitions not commonly used in standard .des files;
        // skip to end of function body
        // FUNCTION name ( params ) { body }
        let _name = self.parse_string()?;
        self.expect(&Token::LParen)?;
        // Skip params
        while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
            self.advance();
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(())
    }

    fn parse_variable_assignment(&mut self) -> Result<(), DesParseError> {
        let name = match self.peek().clone() {
            Token::Variable(n) => {
                self.advance();
                n
            }
            _ => return Err(self.err("expected variable")),
        };

        self.expect(&Token::Equals)?;

        // Determine what kind of value
        match self.peek().clone() {
            Token::LBrace => {
                // Array: { val1, val2, ... }
                self.advance();
                let mut count = 0i64;
                let mut var_type = VarType::Int;

                loop {
                    if self.peek() == &Token::RBrace {
                        self.advance();
                        break;
                    }
                    if count > 0 {
                        self.expect_comma()?;
                        if self.peek() == &Token::RBrace {
                            self.advance();
                            break;
                        }
                    }
                    match self.peek().clone() {
                        Token::LParen => {
                            self.parse_coord_or_var()?;
                            var_type = VarType::Coord;
                        }
                        Token::Char(_) => {
                            self.parse_mapchar_or_var()?;
                            var_type = VarType::MapChar;
                        }
                        Token::Integer(_) | Token::Dice { .. } => {
                            self.parse_math_expr()?;
                            var_type = VarType::Int;
                        }
                        Token::String(_) => {
                            self.parse_string_expr()?;
                            var_type = VarType::String;
                        }
                        _ => {
                            self.parse_math_expr()?;
                        }
                    }
                    count += 1;
                }

                self.emit_var_init(&name, count);
                self.vars.insert(
                    name,
                    VarDef {
                        typ: var_type,
                        is_array: true,
                    },
                );
            }
            Token::Selection => {
                // $var = selection: expr
                self.advance(); // selection
                self.expect_colon()?;
                self.parse_ter_selection()?;
                self.emit_var_init(&name, 0);
                self.vars.insert(
                    name,
                    VarDef {
                        typ: VarType::Sel,
                        is_array: false,
                    },
                );
            }
            Token::Object => {
                // $var = object: { ... }
                self.advance();
                self.expect_colon()?;
                self.parse_typed_array(&name, VarType::Obj, true)?;
            }
            Token::Monster => {
                // $var = monster: { ... }
                self.advance();
                self.expect_colon()?;
                self.parse_typed_array(&name, VarType::Monst, false)?;
            }
            Token::Terrain => {
                // $var = TERRAIN:{ char1, char2, ... }
                self.advance();
                self.expect_colon()?;
                self.expect(&Token::LBrace)?;
                let mut count = 0i64;
                loop {
                    if self.peek() == &Token::RBrace {
                        self.advance();
                        break;
                    }
                    if count > 0 {
                        self.expect_comma()?;
                        if self.peek() == &Token::RBrace {
                            self.advance();
                            break;
                        }
                    }
                    self.parse_mapchar_or_var()?;
                    count += 1;
                }
                self.emit_var_init(&name, count);
                self.vars.insert(
                    name,
                    VarDef {
                        typ: VarType::MapChar,
                        is_array: true,
                    },
                );
            }
            Token::String(ref s) if s == "object" || s == "monster" => {
                let is_obj = s == "object";
                self.advance();
                self.expect_colon()?;
                if is_obj {
                    self.parse_typed_array(&name, VarType::Obj, true)?;
                } else {
                    self.parse_typed_array(&name, VarType::Monst, false)?;
                }
            }
            _ => {
                // Scalar: math expr, string, coord, etc.
                self.parse_math_expr()?;
                self.emit_var_init(&name, 0);
                self.vars.insert(
                    name,
                    VarDef {
                        typ: VarType::Int,
                        is_array: false,
                    },
                );
            }
        }

        Ok(())
    }

    fn parse_typed_array(
        &mut self,
        name: &str,
        var_type: VarType,
        is_obj: bool,
    ) -> Result<(), DesParseError> {
        self.expect(&Token::LBrace)?;
        let mut count = 0i64;
        loop {
            if self.peek() == &Token::RBrace {
                self.advance();
                break;
            }
            if count > 0 {
                self.expect_comma()?;
            }
            if is_obj {
                self.parse_object_or_var()?;
            } else {
                self.parse_monster_or_var()?;
            }
            count += 1;
        }
        self.emit_var_init(name, count);
        self.vars.insert(
            name.to_string(),
            VarDef {
                typ: var_type,
                is_array: true,
            },
        );
        Ok(())
    }

    /// Parse a block of statements (inside { }).
    fn parse_block(&mut self) -> Result<(), DesParseError> {
        loop {
            match self.peek() {
                Token::RBrace | Token::Eof => break,
                _ => {
                    let pct = self.try_percent_prefix()?;
                    if let Some(pct_val) = pct {
                        self.expect_colon()?;
                        self.parse_pct_statement(pct_val)?;
                    } else {
                        self.parse_statement()?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Parse a `.des` file from its token stream.
pub fn parse_des(tokens: Vec<Located<Token>>) -> Result<DesFile, DesParseError> {
    Parser::new(tokens).parse()
}

/// Parse a `.des` file from source text (lex + parse).
pub fn parse_des_file(input: &str) -> Result<DesFile, Box<dyn std::error::Error>> {
    let tokens = crate::des_lexer::lex(input)?;
    let des = parse_des(tokens)?;
    Ok(des)
}

fn room_type_to_int(s: &str) -> i64 {
    match s {
        "ordinary" => 0,
        "throne" => 2,
        "swamp" => 3,
        "vault" => 4,
        "beehive" => 5,
        "morgue" => 6,
        "barracks" => 7,
        "zoo" => 8,
        "delphi" => 9,
        "temple" => 10,
        "anthole" => 11,
        "cocknest" => 12,
        "leprehall" => 13,
        "shop" => 14,
        "armor shop" => 14,
        "scroll shop" => 14,
        "potion shop" => 14,
        "weapon shop" => 14,
        "food shop" => 14,
        "ring shop" => 14,
        "wand shop" => 14,
        "tool shop" => 14,
        "book shop" => 14,
        "candle shop" => 14,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::des_lexer;

    fn parse_file(name: &str) -> DesFile {
        let path = format!("{}/../../nethack/dat/{name}", env!("CARGO_MANIFEST_DIR"));
        let input = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path}"));
        let tokens = des_lexer::lex(&input).unwrap_or_else(|e| panic!("lex {name}: {e}"));
        parse_des(tokens).unwrap_or_else(|e| panic!("parse {name}: {e}"))
    }

    #[test]
    fn parse_mines() {
        let des = parse_file("mines.des");
        assert!(
            des.levels.len() >= 3,
            "mines.des should have ≥3 levels, got {}",
            des.levels.len()
        );
    }

    #[test]
    fn parse_castle() {
        let des = parse_file("castle.des");
        assert_eq!(des.levels.len(), 1);
        // Should contain MAP, DOOR, DRAWBRIDGE opcodes
        let ops: Vec<_> = des.levels[0].opcodes.iter().map(|o| o.opcode).collect();
        assert!(ops.contains(&SpOpcode::Map), "castle should have MAP");
        assert!(ops.contains(&SpOpcode::Door), "castle should have DOOR");
        assert!(
            ops.contains(&SpOpcode::Drawbridge),
            "castle should have DRAWBRIDGE"
        );
    }

    #[test]
    fn parse_bigroom() {
        let des = parse_file("bigroom.des");
        assert_eq!(des.levels.len(), 10, "bigroom.des should have 10 levels");
    }

    #[test]
    fn parse_sokoban() {
        let des = parse_file("sokoban.des");
        assert!(des.levels.len() >= 4);
        // Should contain PREMAPPED flag
        let has_premapped = des.levels.iter().any(|l| {
            l.opcodes.iter().any(|op| {
                op.opcode == SpOpcode::LevelFlags
                    && matches!(
                        op.operand, None // flags are pushed separately
                    )
            })
        });
        // The flag value is in a preceding PUSH, check for that
        let has_premapped_flag = des.levels.iter().any(|l| {
            l.opcodes.windows(2).any(|w| {
                matches!(
                    (&w[0], &w[1]),
                    (
                        SpLevOpcode {
                            opcode: SpOpcode::Push,
                            operand: Some(SpOperand::Int(flags)),
                        },
                        SpLevOpcode {
                            opcode: SpOpcode::LevelFlags,
                            ..
                        }
                    ) if *flags & LevelFlags::PREMAPPED.bits() as i64 != 0
                )
            })
        });
        assert!(
            has_premapped || has_premapped_flag,
            "sokoban should have PREMAPPED flag"
        );
    }

    #[test]
    fn parse_all_des_files() {
        let dat_dir =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../nethack/dat"));
        let mut count = 0;
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(dat_dir).expect("read dat dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "des") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let input = std::fs::read_to_string(&path)
                    .unwrap_or_else(|_| panic!("read {}", path.display()));
                let tokens = match des_lexer::lex(&input) {
                    Ok(t) => t,
                    Err(e) => {
                        failures.push(format!("{name}: lex error: {e}"));
                        count += 1;
                        continue;
                    }
                };
                match parse_des(tokens) {
                    Ok(des) => {
                        assert!(!des.levels.is_empty(), "{name} should have ≥1 level");
                        for level in &des.levels {
                            assert!(
                                !level.opcodes.is_empty(),
                                "{name}:{} should have ≥1 opcode",
                                level.name
                            );
                        }
                    }
                    Err(e) => {
                        failures.push(format!("{name}: parse error: {e}"));
                    }
                }
                count += 1;
            }
        }
        assert_eq!(count, 24, "should process 24 .des files");
        assert!(
            failures.is_empty(),
            "parse failures:\n{}",
            failures.join("\n")
        );
    }
}
