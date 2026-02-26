//! Rectangle splitting algorithm for room placement.
//!
//! Ports C's `rect.c`. Maintains a pool of free rectangles representing
//! available space for room placement. When a room is placed, the rectangle
//! it occupies is removed and the remaining space is split into up to 4
//! sub-rectangles.

use nethack_rng::NhRng;
use serde::Serialize;

use crate::level::{COLNO, ROWNO};

/// Maximum rectangles in the pool, matching C's `MAXRECT`.
const MAX_RECT: usize = 50;
/// Minimum horizontal border width, matching C's `XLIM`.
const XLIM: i32 = 4;
/// Minimum vertical border width, matching C's `YLIM`.
const YLIM: i32 = 3;

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NhRect {
    pub lx: i32,
    pub ly: i32,
    pub hx: i32,
    pub hy: i32,
}

impl NhRect {
    pub fn new(lx: i32, ly: i32, hx: i32, hy: i32) -> Self {
        Self { lx, ly, hx, hy }
    }

    pub fn width(&self) -> i32 {
        self.hx - self.lx + 1
    }

    pub fn height(&self) -> i32 {
        self.hy - self.ly + 1
    }

    /// Whether `self` fully contains `other`.
    pub fn contains(&self, other: &NhRect) -> bool {
        other.lx >= self.lx && other.ly >= self.ly && other.hx <= self.hx && other.hy <= self.hy
    }
}

/// Pool of free rectangles for room placement.
pub struct RectPool {
    rects: Vec<NhRect>,
}

impl RectPool {
    /// Initialize with a single rectangle spanning the entire level.
    pub fn new() -> Self {
        Self {
            rects: vec![NhRect::new(0, 0, COLNO as i32 - 1, ROWNO as i32 - 1)],
        }
    }

    /// Number of rectangles in the pool.
    pub fn len(&self) -> usize {
        self.rects.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Find a free rectangle that fully contains `r`.
    pub fn get_containing(&self, r: &NhRect) -> Option<usize> {
        self.rects.iter().position(|rect| rect.contains(r))
    }

    /// Return a random rectangle from the pool.
    pub fn random(&self, rng: &mut NhRng) -> Option<NhRect> {
        if self.rects.is_empty() {
            return None;
        }
        let idx = rng.rn2(self.rects.len() as i32) as usize;
        Some(self.rects[idx])
    }

    /// Remove a rectangle by index (swap-and-shrink).
    pub fn remove(&mut self, idx: usize) {
        self.rects.swap_remove(idx);
    }

    /// Add a rectangle to the pool (rejected if already contained in an existing rect).
    pub fn add(&mut self, r: NhRect) {
        if self.rects.len() >= MAX_RECT {
            return;
        }
        if self.get_containing(&r).is_some() {
            return;
        }
        self.rects.push(r);
    }

    /// Compute the intersection of two rectangles.
    pub fn intersect(r1: &NhRect, r2: &NhRect) -> Option<NhRect> {
        if r2.lx > r1.hx || r2.ly > r1.hy || r2.hx < r1.lx || r2.hy < r1.ly {
            return None;
        }
        let r3 = NhRect {
            lx: r1.lx.max(r2.lx),
            ly: r1.ly.max(r2.ly),
            hx: r1.hx.min(r2.hx),
            hy: r1.hy.min(r2.hy),
        };
        if r3.lx > r3.hx || r3.ly > r3.hy {
            return None;
        }
        Some(r3)
    }

    /// Split `r1` around `r2` (room placement), creating up to 4 sub-rectangles.
    ///
    /// Removes `r1` from the pool and adds the remaining pieces. Also recursively
    /// splits any other rectangles that intersect with `r2`.
    pub fn split(&mut self, r1_idx: usize, r2: &NhRect) {
        let r1 = self.rects[r1_idx];
        self.remove(r1_idx);

        // Check if r1 is on the level boundary
        let on_left = r1.lx == 0;
        let on_right = r1.hx == COLNO as i32 - 1;
        let on_top = r1.ly == 0;
        let on_bottom = r1.hy == ROWNO as i32 - 1;

        // Top piece (above r2)
        if r2.ly - r1.ly > (if on_top { 2 * YLIM } else { YLIM + 1 }) + 4 {
            self.add(NhRect::new(r1.lx, r1.ly, r1.hx, r2.ly - 2));
        }

        // Bottom piece (below r2)
        if r1.hy - r2.hy > (if on_bottom { 2 * YLIM } else { YLIM + 1 }) + 4 {
            self.add(NhRect::new(r1.lx, r2.hy + 2, r1.hx, r1.hy));
        }

        // Left piece
        if r2.lx - r1.lx > (if on_left { 2 * XLIM } else { XLIM + 1 }) + 4 {
            self.add(NhRect::new(r1.lx, r1.ly, r2.lx - 2, r1.hy));
        }

        // Right piece
        if r1.hx - r2.hx > (if on_right { 2 * XLIM } else { XLIM + 1 }) + 4 {
            self.add(NhRect::new(r2.hx + 2, r1.ly, r1.hx, r1.hy));
        }

        // Recursively split any other rects that overlap with r2
        let mut i = 0;
        while i < self.rects.len() {
            if let Some(_isect) = Self::intersect(&self.rects[i], r2) {
                self.split(i, r2);
                // After split, indices shifted — restart scan
                i = 0;
            } else {
                i += 1;
            }
        }
    }
}

impl Default for RectPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_has_one_rect() {
        let pool = RectPool::new();
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn intersect_overlapping() {
        let r1 = NhRect::new(0, 0, 10, 10);
        let r2 = NhRect::new(5, 5, 15, 15);
        let r3 = RectPool::intersect(&r1, &r2).expect("should intersect");
        assert_eq!(r3, NhRect::new(5, 5, 10, 10));
    }

    #[test]
    fn intersect_non_overlapping() {
        let r1 = NhRect::new(0, 0, 5, 5);
        let r2 = NhRect::new(10, 10, 15, 15);
        assert!(RectPool::intersect(&r1, &r2).is_none());
    }

    #[test]
    fn split_creates_sub_rects() {
        let mut pool = RectPool::new();
        let room = NhRect::new(20, 5, 30, 10);
        // The initial rect should contain the room
        let idx = pool.get_containing(&room).expect("should contain room");
        pool.split(idx, &room);
        // After splitting, we should have some rectangles remaining
        assert!(pool.len() >= 1, "split should leave at least one free rect");
    }

    #[test]
    fn contains_check() {
        let outer = NhRect::new(0, 0, 79, 20);
        let inner = NhRect::new(10, 5, 20, 10);
        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));
    }
}
