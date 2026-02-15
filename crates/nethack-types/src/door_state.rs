use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    /// Door state flags from `rm.h` (D_* constants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct DoorState: u8 {
        const NODOOR  = 0;
        const BROKEN  = 1;
        const ISOPEN  = 2;
        const CLOSED  = 4;
        const LOCKED  = 8;
        const TRAPPED = 16;
        const SECRET  = 32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(DoorState::BROKEN.bits(), 1);
        assert_eq!(DoorState::CLOSED.bits(), 4);
        assert_eq!(DoorState::LOCKED.bits(), 8);
        assert_eq!(DoorState::TRAPPED.bits(), 16);
        assert_eq!(DoorState::SECRET.bits(), 32);
    }

    #[test]
    fn combinations() {
        let locked_trapped = DoorState::LOCKED | DoorState::TRAPPED;
        assert!(locked_trapped.contains(DoorState::LOCKED));
        assert!(locked_trapped.contains(DoorState::TRAPPED));
        assert!(!locked_trapped.contains(DoorState::BROKEN));
    }
}
