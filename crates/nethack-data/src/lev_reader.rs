//! Reader for the `.lev` binary format produced by C's `lev_comp`.
//!
//! Parses the binary opcode stream into the same [`SpLevOpcode`] representation
//! used by the Rust `.des` parser, enabling comparison between the two.

use nethack_types::sp_lev::{SpLevOpcode, SpOpcode, SpOperand};

/// Version header size: 5 × `unsigned long` (8 bytes each on 64-bit Linux).
const VERSION_HEADER_SIZE: usize = 40;

const SPOVAR_NULL: u8 = 0x00;
const SPOVAR_INT: u8 = 0x01;
const SPOVAR_STRING: u8 = 0x02;
const SPOVAR_VARIABLE: u8 = 0x03;
const SPOVAR_COORD: u8 = 0x04;
const SPOVAR_REGION: u8 = 0x05;
const SPOVAR_MAPCHAR: u8 = 0x06;
const SPOVAR_MONST: u8 = 0x07;
const SPOVAR_OBJ: u8 = 0x08;
const SPOVAR_SEL: u8 = 0x09;

/// Bit that marks a coord as random in the packed i64 representation.
const SP_COORD_IS_RANDOM: i64 = 0x0100_0000;

#[derive(Debug, thiserror::Error)]
pub enum LevReadError {
    #[error("unexpected end of data at offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("unknown opcode {value} at offset {offset}")]
    UnknownOpcode { value: i32, offset: usize },
    #[error("unknown spovartyp {value} at offset {offset}")]
    UnknownSpovartyp { value: u8, offset: usize },
    #[error("invalid UTF-8 string at offset {offset}")]
    InvalidUtf8 { offset: usize },
}

/// Cursor for reading little-endian binary data.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], LevReadError> {
        if self.remaining() < n {
            return Err(LevReadError::UnexpectedEof { offset: self.pos });
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, LevReadError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_i32(&mut self) -> Result<i32, LevReadError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes(bytes.try_into().expect("4 bytes")))
    }

    fn read_i64(&mut self) -> Result<i64, LevReadError> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_le_bytes(bytes.try_into().expect("8 bytes")))
    }

    fn skip(&mut self, n: usize) -> Result<(), LevReadError> {
        if self.remaining() < n {
            return Err(LevReadError::UnexpectedEof { offset: self.pos });
        }
        self.pos += n;
        Ok(())
    }
}

/// Unpack an `SP_COORD_PACK`ed i64 into `SpOperand::Coord` fields.
fn unpack_coord(packed: i64) -> SpOperand {
    if packed & SP_COORD_IS_RANDOM != 0 {
        // Random coord: lower bits are humidity flags
        let flags = (packed & 0xFF) as u32;
        SpOperand::Coord {
            x: -1,
            y: -1,
            is_random: true,
            flags,
        }
    } else {
        let x = (packed & 0xFF) as i16;
        let y = ((packed >> 16) & 0xFF) as i16;
        SpOperand::Coord {
            x,
            y,
            is_random: false,
            flags: 0,
        }
    }
}

/// Unpack an `SP_REGION_PACK`ed i64 into `SpOperand::Region` fields.
fn unpack_region(packed: i64) -> SpOperand {
    SpOperand::Region {
        x1: (packed & 0xFF) as i16,
        y1: ((packed >> 8) & 0xFF) as i16,
        x2: ((packed >> 16) & 0xFF) as i16,
        y2: ((packed >> 24) & 0xFF) as i16,
    }
}

/// Unpack an `SP_MAPCHAR_PACK`ed i64 into `SpOperand::MapChar` fields.
fn unpack_mapchar(packed: i64) -> SpOperand {
    let typ = (packed & 0xFF) as i16;
    let lit = (((packed >> 8) & 0xFFFF) - 10) as i16;
    SpOperand::MapChar { typ, lit }
}

/// Unpack an `SP_MONST_PACK`ed i64 into `SpOperand::Monst` fields.
fn unpack_monst(packed: i64) -> SpOperand {
    let class = (packed & 0xFF) as i16;
    let id = (((packed >> 8) & 0xFFFF) - 10) as i16;
    SpOperand::Monst { class, id }
}

/// Unpack an `SP_OBJ_PACK`ed i64 into `SpOperand::Obj` fields.
fn unpack_obj(packed: i64) -> SpOperand {
    let class = (packed & 0xFF) as i16;
    let id = (((packed >> 8) & 0xFFFF) - 10) as i16;
    SpOperand::Obj { class, id }
}

/// Read a `.lev` binary file and return its opcode stream.
///
/// The binary format (64-bit Linux, little-endian):
/// - 40-byte version header (skipped)
/// - `n_opcodes: i64`
/// - For each opcode: `opcode: i32`, then if `Push`: `spovartyp: u8` + payload
pub fn read_lev(data: &[u8]) -> Result<Vec<SpLevOpcode>, LevReadError> {
    let mut r = Reader::new(data);

    // Skip version_info header
    r.skip(VERSION_HEADER_SIZE)?;

    let n_opcodes = r.read_i64()?;
    let mut opcodes = Vec::with_capacity(n_opcodes as usize);

    for _ in 0..n_opcodes {
        let op_offset = r.pos;
        let raw_opcode = r.read_i32()?;
        let opcode = SpOpcode::from_repr(raw_opcode as u8).ok_or(LevReadError::UnknownOpcode {
            value: raw_opcode,
            offset: op_offset,
        })?;

        let operand = if opcode == SpOpcode::Push {
            let typ_offset = r.pos;
            let spovartyp = r.read_u8()?;
            match spovartyp {
                SPOVAR_NULL => None,
                SPOVAR_INT => {
                    let val = r.read_i64()?;
                    Some(SpOperand::Int(val))
                }
                SPOVAR_STRING => {
                    let len = r.read_i32()? as usize;
                    let bytes = r.read_bytes(len)?;
                    let s = std::str::from_utf8(bytes)
                        .map_err(|_| LevReadError::InvalidUtf8 { offset: r.pos })?;
                    Some(SpOperand::String(s.to_string()))
                }
                SPOVAR_VARIABLE => {
                    let len = r.read_i32()? as usize;
                    let bytes = r.read_bytes(len)?;
                    let s = std::str::from_utf8(bytes)
                        .map_err(|_| LevReadError::InvalidUtf8 { offset: r.pos })?;
                    Some(SpOperand::Variable(s.to_string()))
                }
                SPOVAR_COORD => {
                    let packed = r.read_i64()?;
                    Some(unpack_coord(packed))
                }
                SPOVAR_REGION => {
                    let packed = r.read_i64()?;
                    Some(unpack_region(packed))
                }
                SPOVAR_MAPCHAR => {
                    let packed = r.read_i64()?;
                    Some(unpack_mapchar(packed))
                }
                SPOVAR_MONST => {
                    let packed = r.read_i64()?;
                    Some(unpack_monst(packed))
                }
                SPOVAR_OBJ => {
                    let packed = r.read_i64()?;
                    Some(unpack_obj(packed))
                }
                SPOVAR_SEL => {
                    let len = r.read_i32()? as usize;
                    let bytes = r.read_bytes(len)?;
                    Some(SpOperand::Sel(bytes.to_vec()))
                }
                _ => {
                    return Err(LevReadError::UnknownSpovartyp {
                        value: spovartyp,
                        offset: typ_offset,
                    });
                }
            }
        } else {
            None
        };

        opcodes.push(SpLevOpcode { opcode, operand });
    }

    Ok(opcodes)
}
