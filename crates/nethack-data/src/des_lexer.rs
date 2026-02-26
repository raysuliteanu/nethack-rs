//! Tokenizer for the NetHack `.des` level scripting language.
//!
//! Produces a stream of tokens matching the grammar defined in
//! `nethack/util/lev_comp.l`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Structure
    Maze,
    Level,
    Flags,
    InitMap,
    Geometry,
    Nomap,
    Message,

    // Map block
    Map,
    MapData(String),

    // Placement
    Monster,
    Object,
    Container,
    Trap,
    Door,
    RoomDoor,
    Drawbridge,
    Fountain,
    Sink,
    Pool,
    Ladder,
    Stair,
    Altar,
    Portal,
    TeleportRegion,
    Branch,
    Gold,
    Engraving,
    Grave,
    MazeWalk,
    Wallify,
    Mineralize,
    NonDiggable,
    NonPasswall,

    // Terrain
    Terrain,
    ReplaceTerrain,
    Region,

    // Room
    Room,
    Subroom,
    Corridor,
    RandomCorridors,

    // Control flow
    If,
    Else,
    For,
    To,
    Loop,
    Switch,
    Case,
    Default,
    Break,
    Function,
    Exit,

    // Selection operations
    Selection,
    Rect,
    FillRect,
    Line,
    RandLine,
    Grow,
    FloodFill,
    RndCoord,
    Circle,
    Ellipse,
    Filter,
    Gradient,
    Complement,

    // Misc keywords
    Shuffle,
    Name,
    MonType,
    Quantity,
    Buried,
    Eroded,
    ErodeProof,
    Recharged,
    Invisible,
    Greased,
    Female,
    Cancelled,
    Revived,
    Avenge,
    Fleeing,
    Blinded,
    Paralyzed,
    Stunned,
    Confused,
    SeenTraps,
    All,

    // Init map styles
    MazeGrid,
    SolidFill,
    Mines,
    RogueLev,

    // Flag names
    FlagType(String),

    // Direction
    North,
    East,
    South,
    West,
    Horizontal,
    Vertical,

    // Up/Down
    Up,
    Down,

    // Door state
    DoorState(String),

    // Light state
    Lit,
    Unlit,

    // Alignment
    Alignment(String),

    // Altar type
    AltarType(String),

    // Monster attitude
    Peaceful,
    Hostile,
    Asleep,
    Awake,

    // Monster appearance
    MFeature,
    MMonster,
    MObject,

    // Filling
    Filled,
    Unfilled,

    // Room shape
    Regular,
    Irregular,
    Joined,
    Unjoined,
    Limited,
    Unlimited,

    // Position
    Left,
    HalfLeft,
    Center,
    HalfRight,
    Right,
    Top,
    Bottom,
    AlignReg,

    // Engraving type
    EngravingType(String),

    // Curse state
    CurseType(String),

    // Boolean
    BoolTrue,
    BoolFalse,

    // Random
    Random,

    // None
    NoneVal,

    // Gradient types
    Radial,
    Square,

    // Humidity
    Dry,
    Wet,
    Hot,
    Solid,
    Any,

    // Comparison
    CompareEq,
    CompareNe,
    CompareLt,
    CompareGt,
    CompareLe,
    CompareGe,

    // Trapped state
    Trapped,
    NotTrapped,

    // Levregion
    LevRegionKw,

    // Literals
    String(String),
    Char(char),
    Integer(i64),
    Dice { num: i64, die: i64 },
    Percent(i64),

    // Variables
    Variable(String),

    // Punctuation
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Plus,
    Minus,
    DashDash,
    Equals,

    Pipe,

    // Selection composition
    Ampersand,

    // End of input
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::String(s) => write!(f, "\"{s}\""),
            Token::Integer(n) => write!(f, "{n}"),
            Token::Variable(v) => write!(f, "${v}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Located<T> {
    pub value: T,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("line {line}, col {col}: {msg}")]
    Error {
        line: usize,
        col: usize,
        msg: String,
    },
}

#[allow(unused_assignments)] // col tracking can be overwritten at loop top
pub fn lex(input: &str) -> Result<Vec<Located<Token>>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line = 1usize;
    let mut col = 1usize;

    while let Some(&ch) = chars.peek() {
        // Track position
        let start_line = line;
        let start_col = col;

        // Skip whitespace (except newlines which we track)
        if ch == '\n' {
            chars.next();
            line += 1;
            col = 1;
            continue;
        }
        if ch == '\r' {
            chars.next();
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            line += 1;
            col = 1;
            continue;
        }
        if ch == ' ' || ch == '\t' {
            chars.next();
            col += 1;
            continue;
        }

        // Comments
        if ch == '#' {
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
                col += 1;
            }
            continue;
        }

        // String literals
        if ch == '"' {
            chars.next();
            col += 1;
            let mut s = std::string::String::new();
            loop {
                match chars.peek() {
                    Some(&'"') => {
                        chars.next();
                        col += 1;
                        break;
                    }
                    Some(&c) => {
                        s.push(c);
                        chars.next();
                        col += 1;
                    }
                    None => {
                        return Err(LexError::Error {
                            line: start_line,
                            col: start_col,
                            msg: "unterminated string".into(),
                        });
                    }
                }
            }
            tokens.push(Located {
                value: Token::String(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Character literals: 'x' or '\x' (but '\'' means char '\')
        if ch == '\'' {
            chars.next();
            col += 1;
            let c = match chars.peek() {
                Some(&'\\') => {
                    // Peek two ahead to decide: if next-next is '\'', then
                    // the char is '\' (flex's '.' pattern wins over '\\.')
                    let mut lookahead = chars.clone();
                    lookahead.next(); // consume '\'
                    match lookahead.peek() {
                        Some(&'\'') => {
                            // Pattern: '\' → char is backslash
                            chars.next(); // consume '\'
                            col += 1;
                            '\\'
                        }
                        Some(&c) => {
                            // Pattern: '\x' → escape sequence
                            chars.next(); // consume '\'
                            col += 1;
                            chars.next(); // consume escaped char
                            col += 1;
                            c
                        }
                        None => {
                            return Err(LexError::Error {
                                line: start_line,
                                col: start_col,
                                msg: "unterminated char literal".into(),
                            });
                        }
                    }
                }
                Some(&c) => {
                    chars.next();
                    col += 1;
                    c
                }
                None => {
                    return Err(LexError::Error {
                        line: start_line,
                        col: start_col,
                        msg: "unterminated char literal".into(),
                    });
                }
            };
            match chars.peek() {
                Some(&'\'') => {
                    chars.next();
                    col += 1;
                }
                _ => {
                    return Err(LexError::Error {
                        line: start_line,
                        col: start_col,
                        msg: "unterminated char literal".into(),
                    });
                }
            }
            tokens.push(Located {
                value: Token::Char(c),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Variable references: $name or $name[
        if ch == '$' {
            chars.next();
            col += 1;
            let mut name = std::string::String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(c);
                    chars.next();
                    col += 1;
                } else {
                    break;
                }
            }
            tokens.push(Located {
                value: Token::Variable(name),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Percent in brackets: [50%]
        if ch == '[' {
            // Peek ahead: could be [N%] or just [
            let saved: Vec<char> = chars.clone().skip(1).take(20).collect();
            let s: std::string::String = saved.iter().collect();
            if let Some(pct_end) = s.find('%') {
                let num_str: std::string::String = s[..pct_end]
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                // Check if there's a ] after the %
                let rest = &s[pct_end + 1..];
                let close_pos = rest.find(']');
                if let (Ok(n), Some(_close)) = (num_str.parse::<i64>(), close_pos) {
                    // It's [N%] — consume all of it
                    let total = pct_end + 1 + close_pos.expect("close_pos already matched") + 1;
                    chars.next(); // [
                    col += 1;
                    for _ in 0..total {
                        chars.next();
                        col += 1;
                    }
                    tokens.push(Located {
                        value: Token::Percent(n),
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }
            }
            // Just a bracket
            chars.next();
            col += 1;
            tokens.push(Located {
                value: Token::LBracket,
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Punctuation
        match ch {
            ':' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::Colon,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            ',' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::Comma,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '(' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::LParen,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            ')' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::RParen,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '{' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::LBrace,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '}' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::RBrace,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            ']' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::RBracket,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '&' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::Ampersand,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '|' => {
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::Pipe,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            '=' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Located {
                        value: Token::CompareEq,
                        line: start_line,
                        col: start_col,
                    });
                } else {
                    tokens.push(Located {
                        value: Token::Equals,
                        line: start_line,
                        col: start_col,
                    });
                }
                continue;
            }
            '!' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Located {
                        value: Token::CompareNe,
                        line: start_line,
                        col: start_col,
                    });
                } else {
                    return Err(LexError::Error {
                        line: start_line,
                        col: start_col,
                        msg: "unexpected '!'".into(),
                    });
                }
                continue;
            }
            '<' => {
                chars.next();
                col += 1;
                match chars.peek() {
                    Some(&'=') => {
                        chars.next();
                        col += 1;
                        tokens.push(Located {
                            value: Token::CompareLe,
                            line: start_line,
                            col: start_col,
                        });
                    }
                    Some(&'>') => {
                        chars.next();
                        col += 1;
                        tokens.push(Located {
                            value: Token::CompareNe,
                            line: start_line,
                            col: start_col,
                        });
                    }
                    _ => {
                        tokens.push(Located {
                            value: Token::CompareLt,
                            line: start_line,
                            col: start_col,
                        });
                    }
                }
                continue;
            }
            '>' => {
                chars.next();
                col += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    col += 1;
                    tokens.push(Located {
                        value: Token::CompareGe,
                        line: start_line,
                        col: start_col,
                    });
                } else {
                    tokens.push(Located {
                        value: Token::CompareGt,
                        line: start_line,
                        col: start_col,
                    });
                }
                continue;
            }
            '+' => {
                chars.next();
                col += 1;
                // Check for +N (positive integer)
                if chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                    let mut s = std::string::String::new();
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            s.push(d);
                            chars.next();
                            col += 1;
                        } else {
                            break;
                        }
                    }
                    let n: i64 = s.parse().expect("digits");
                    tokens.push(Located {
                        value: Token::Integer(n),
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }
                tokens.push(Located {
                    value: Token::Plus,
                    line: start_line,
                    col: start_col,
                });
                continue;
            }
            _ => {}
        }

        // Numbers (possibly negative, possibly dice notation, possibly percent)
        if ch == '-' || ch.is_ascii_digit() {
            let is_neg = ch == '-';
            let mut s = std::string::String::new();

            if is_neg {
                chars.next();
                col += 1;
                // Check if next is a digit
                if let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        s.push('-');
                    } else if c == '-' {
                        // -- token
                        chars.next();
                        col += 1;
                        tokens.push(Located {
                            value: Token::DashDash,
                            line: start_line,
                            col: start_col,
                        });
                        continue;
                    } else {
                        tokens.push(Located {
                            value: Token::Minus,
                            line: start_line,
                            col: start_col,
                        });
                        continue;
                    }
                } else {
                    tokens.push(Located {
                        value: Token::Minus,
                        line: start_line,
                        col: start_col,
                    });
                    continue;
                }
            }

            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    chars.next();
                    col += 1;
                } else {
                    break;
                }
            }

            // Check for dice notation: NdM
            if let Some(&'d') = chars.peek() {
                let num: i64 = s.parse().expect("digits");
                chars.next();
                col += 1;
                let mut die_s = std::string::String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        die_s.push(c);
                        chars.next();
                        col += 1;
                    } else {
                        break;
                    }
                }
                let die: i64 = die_s.parse().unwrap_or(0);
                tokens.push(Located {
                    value: Token::Dice { num, die },
                    line: start_line,
                    col: start_col,
                });
                continue;
            }

            // Check for percent: N%
            if let Some(&'%') = chars.peek() {
                let n: i64 = s.parse().expect("digits");
                chars.next();
                col += 1;
                tokens.push(Located {
                    value: Token::Percent(n),
                    line: start_line,
                    col: start_col,
                });
                continue;
            }

            let n: i64 = s.parse().expect("digits");
            tokens.push(Located {
                value: Token::Integer(n),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Keywords and identifiers
        if ch.is_alphabetic() || ch == '_' {
            let mut word = std::string::String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    word.push(c);
                    chars.next();
                    col += 1;
                } else {
                    break;
                }
            }

            let tok = match word.as_str() {
                // Special: MAP keyword starts map block capture
                "MAP" => {
                    // Consume rest of line (should be empty or whitespace)
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            chars.next();
                            line += 1;
                            col = 1;
                            break;
                        }
                        chars.next();
                        col += 1;
                    }
                    // Capture lines until ENDMAP
                    let mut map_data = std::string::String::new();
                    loop {
                        // Read a line
                        let mut line_buf = std::string::String::new();
                        let mut found_newline = false;
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                chars.next();
                                found_newline = true;
                                break;
                            }
                            if c == '\r' {
                                chars.next();
                                if chars.peek() == Some(&'\n') {
                                    chars.next();
                                }
                                found_newline = true;
                                break;
                            }
                            line_buf.push(c);
                            chars.next();
                        }
                        if line_buf.trim() == "ENDMAP" {
                            line += 1;
                            col = 1;
                            break;
                        }
                        map_data.push_str(&line_buf);
                        map_data.push('\n');
                        if found_newline {
                            line += 1;
                            col = 1;
                        }
                        if !found_newline && chars.peek().is_none() {
                            return Err(LexError::Error {
                                line: start_line,
                                col: start_col,
                                msg: "unterminated MAP block".into(),
                            });
                        }
                    }
                    // Remove trailing newline
                    if map_data.ends_with('\n') {
                        map_data.pop();
                    }
                    tokens.push(Located {
                        value: Token::Map,
                        line: start_line,
                        col: start_col,
                    });
                    tokens.push(Located {
                        value: Token::MapData(map_data),
                        line: start_line + 1,
                        col: 1,
                    });
                    continue;
                }

                // Structure
                "MAZE" => Token::Maze,
                "LEVEL" => Token::Level,
                "FLAGS" => Token::Flags,
                "INIT_MAP" => Token::InitMap,
                "GEOMETRY" => Token::Geometry,
                "NOMAP" => Token::Nomap,
                "MESSAGE" => Token::Message,

                // Placement
                "MONSTER" | "monster" => Token::Monster,
                "OBJECT" | "obj" | "object" => Token::Object,
                "CONTAINER" => Token::Container,
                "TRAP" => Token::Trap,
                "DOOR" => Token::Door,
                "ROOMDOOR" => Token::RoomDoor,
                "DRAWBRIDGE" => Token::Drawbridge,
                "FOUNTAIN" => Token::Fountain,
                "SINK" => Token::Sink,
                "POOL" => Token::Pool,
                "LADDER" => Token::Ladder,
                "STAIR" => Token::Stair,
                "ALTAR" => Token::Altar,
                "PORTAL" => Token::Portal,
                "TELEPORT_REGION" => Token::TeleportRegion,
                "BRANCH" => Token::Branch,
                "GOLD" => Token::Gold,
                "ENGRAVING" => Token::Engraving,
                "GRAVE" => Token::Grave,
                "MAZEWALK" => Token::MazeWalk,
                "WALLIFY" => Token::Wallify,
                "MINERALIZE" => Token::Mineralize,
                "NON_DIGGABLE" => Token::NonDiggable,
                "NON_PASSWALL" => Token::NonPasswall,

                // Terrain
                "TERRAIN" | "terrain" => Token::Terrain,
                "REPLACE_TERRAIN" => Token::ReplaceTerrain,
                "REGION" => Token::Region,

                // Room
                "ROOM" => Token::Room,
                "SUBROOM" => Token::Subroom,
                "CORRIDOR" => Token::Corridor,
                "RANDOM_CORRIDORS" => Token::RandomCorridors,

                // Control flow
                "IF" => Token::If,
                "ELSE" => Token::Else,
                "FOR" => Token::For,
                "TO" => Token::To,
                "LOOP" => Token::Loop,
                "SWITCH" => Token::Switch,
                "CASE" => Token::Case,
                "DEFAULT" => Token::Default,
                "BREAK" => Token::Break,
                "FUNCTION" => Token::Function,
                "EXIT" => Token::Exit,

                // Selection
                "selection" => Token::Selection,
                "rect" => Token::Rect,
                "fillrect" => Token::FillRect,
                "line" => Token::Line,
                "randline" => Token::RandLine,
                "grow" => Token::Grow,
                "floodfill" => Token::FloodFill,
                "rndcoord" => Token::RndCoord,
                "circle" => Token::Circle,
                "ellipse" => Token::Ellipse,
                "filter" => Token::Filter,
                "gradient" => Token::Gradient,
                "complement" => Token::Complement,

                // Misc
                "SHUFFLE" => Token::Shuffle,
                "NAME" | "name" => Token::Name,
                "montype" => Token::MonType,
                "quantity" => Token::Quantity,
                "buried" => Token::Buried,
                "eroded" => Token::Eroded,
                "erodeproof" => Token::ErodeProof,
                "recharged" => Token::Recharged,
                "invisible" => Token::Invisible,
                "greased" => Token::Greased,
                "female" => Token::Female,
                "cancelled" => Token::Cancelled,
                "revived" => Token::Revived,
                "avenge" => Token::Avenge,
                "fleeing" => Token::Fleeing,
                "blinded" => Token::Blinded,
                "paralyzed" => Token::Paralyzed,
                "stunned" => Token::Stunned,
                "confused" => Token::Confused,
                "seen_traps" => Token::SeenTraps,
                "all" => Token::All,

                // Init map styles
                "mazegrid" => Token::MazeGrid,
                "solidfill" => Token::SolidFill,
                "mines" => Token::Mines,
                "rogue" => Token::RogueLev,

                // Flag names
                "noteleport" | "hardfloor" | "nommap" | "arboreal" | "shortsighted"
                | "mazelevel" | "premapped" | "shroud" | "graveyard" | "icedpools" | "solidify"
                | "corrmaze" | "inaccessibles" => Token::FlagType(word),

                // Direction
                "north" => Token::North,
                "east" => Token::East,
                "south" => Token::South,
                "west" => Token::West,
                "horizontal" => Token::Horizontal,
                "vertical" => Token::Vertical,

                // Up/Down
                "up" => Token::Up,
                "down" => Token::Down,

                // Door state
                "open" | "closed" | "locked" | "nodoor" | "broken" | "secret" => {
                    Token::DoorState(word)
                }

                // Light state
                "lit" => Token::Lit,
                "unlit" => Token::Unlit,

                // Alignment
                "noalign" | "law" | "neutral" | "chaos" | "coaligned" | "noncoaligned" => {
                    Token::Alignment(word)
                }

                // Altar type
                "altar" => Token::AltarType("altar".into()),
                "shrine" => Token::AltarType("shrine".into()),
                "sanctum" => Token::AltarType("sanctum".into()),

                // Monster attitude
                "peaceful" => Token::Peaceful,
                "hostile" => Token::Hostile,
                "asleep" => Token::Asleep,
                "awake" => Token::Awake,

                // Monster appearance
                "m_feature" => Token::MFeature,
                "m_monster" => Token::MMonster,
                "m_object" => Token::MObject,

                // Filling
                "filled" => Token::Filled,
                "unfilled" => Token::Unfilled,

                // Room shape
                "regular" => Token::Regular,
                "irregular" => Token::Irregular,
                "joined" => Token::Joined,
                "unjoined" => Token::Unjoined,
                "limited" => Token::Limited,
                "unlimited" => Token::Unlimited,

                // Position
                "left" => Token::Left,
                "half-left" => Token::HalfLeft,
                "center" => Token::Center,
                "half-right" => Token::HalfRight,
                "right" => Token::Right,
                "top" => Token::Top,
                "bottom" => Token::Bottom,
                "align" => Token::AlignReg,

                // Engraving type
                "dust" | "engrave" | "burn" | "mark" | "blood" => Token::EngravingType(word),

                // Curse state
                "blessed" | "uncursed" | "cursed" => Token::CurseType(word),

                // Boolean
                "true" => Token::BoolTrue,
                "false" => Token::BoolFalse,

                // Random
                "random" => Token::Random,

                // None
                "none" => Token::NoneVal,

                // Gradient types
                "radial" => Token::Radial,
                "square" => Token::Square,

                // Humidity
                "dry" => Token::Dry,
                "wet" => Token::Wet,
                "hot" => Token::Hot,
                "solid" => Token::Solid,
                "any" => Token::Any,

                // Trapped state
                "trapped" => Token::Trapped,
                "not_trapped" => Token::NotTrapped,

                // Levregion
                "levregion" => Token::LevRegionKw,

                // Unknown — treat as a string-like identifier
                _ => Token::String(word),
            };

            tokens.push(Located {
                value: tok,
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // Unknown character — skip
        chars.next();
        col += 1;
    }

    tokens.push(Located {
        value: Token::Eof,
        line,
        col,
    });

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple_maze() {
        let tokens = lex("MAZE: \"castle\", random\n").expect("lex");
        assert!(matches!(tokens[0].value, Token::Maze));
        assert!(matches!(tokens[1].value, Token::Colon));
        assert!(matches!(tokens[2].value, Token::String(ref s) if s == "castle"));
        assert!(matches!(tokens[3].value, Token::Comma));
        assert!(matches!(tokens[4].value, Token::Random));
    }

    #[test]
    fn lex_map_block() {
        let input = "MAP\n.|.|\n-+-+\nENDMAP\n";
        let tokens = lex(input).expect("lex");
        assert!(matches!(tokens[0].value, Token::Map));
        assert!(matches!(tokens[1].value, Token::MapData(ref s) if s == ".|.|\n-+-+"));
    }

    #[test]
    fn lex_percent() {
        let tokens = lex("[75%]: SUBROOM").expect("lex");
        assert!(matches!(tokens[0].value, Token::Percent(75)));
    }

    #[test]
    fn lex_dice() {
        let tokens = lex("2d6").expect("lex");
        assert!(matches!(tokens[0].value, Token::Dice { num: 2, die: 6 }));
    }

    #[test]
    fn lex_variable() {
        let tokens = lex("$place[0]").expect("lex");
        assert!(matches!(tokens[0].value, Token::Variable(ref s) if s == "place"));
        assert!(matches!(tokens[1].value, Token::LBracket));
        assert!(matches!(tokens[2].value, Token::Integer(0)));
        assert!(matches!(tokens[3].value, Token::RBracket));
    }

    #[test]
    fn lex_char_literal() {
        let tokens = lex("' '").expect("lex");
        assert!(matches!(tokens[0].value, Token::Char(' ')));
    }

    #[test]
    fn lex_negative_integer() {
        let tokens = lex("-5").expect("lex");
        assert!(matches!(tokens[0].value, Token::Integer(-5)));
    }

    #[test]
    fn lex_simple_percent() {
        let tokens = lex("18%").expect("lex");
        assert!(matches!(tokens[0].value, Token::Percent(18)));
    }

    #[test]
    fn lex_comparison_ops() {
        let tokens = lex("== != < > <= >=").expect("lex");
        assert!(matches!(tokens[0].value, Token::CompareEq));
        assert!(matches!(tokens[1].value, Token::CompareNe));
        assert!(matches!(tokens[2].value, Token::CompareLt));
        assert!(matches!(tokens[3].value, Token::CompareGt));
        assert!(matches!(tokens[4].value, Token::CompareLe));
        assert!(matches!(tokens[5].value, Token::CompareGe));
    }

    #[test]
    fn lex_mines_des() {
        let input = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../nethack/dat/mines.des"
        ))
        .expect("mines.des");
        let tokens = lex(&input).expect("lex mines.des");
        assert!(tokens.len() > 100);
        // Should end with Eof
        assert!(matches!(tokens.last().unwrap().value, Token::Eof));
    }

    #[test]
    fn lex_all_des_files() {
        let dat_dir =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../nethack/dat"));
        let mut count = 0;
        for entry in std::fs::read_dir(dat_dir).expect("read dat dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "des") {
                let input = std::fs::read_to_string(&path)
                    .unwrap_or_else(|_| panic!("read {}", path.display()));
                lex(&input).unwrap_or_else(|e| panic!("lex {}: {e}", path.display()));
                count += 1;
            }
        }
        assert_eq!(count, 24);
    }
}
