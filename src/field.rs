//! The playfield of a Minesweeper game.
//!
//! This is the main point of interest for the game — everything happens here. For that reason, this module is the most detailed one.

use core::{
    ops::{Index, IndexMut},
    num::{NonZeroUsize, NonZeroU8}
};
use alloc::{
    vec::Vec
};
use crate::{
    Tile, Flag, ClickOutcome,
    Clearing, ClearingMut,
    RowIter, ColumnIter,
    FieldRowsIter, FieldColumnsIter
};

/// Represents a playfield.
pub struct Field {
    storage: Vec<Tile>,
    dimensions: (NonZeroUsize, NonZeroUsize)
}
impl Field {
    /// Creates an empty field filled with unopened tiles, with the given dimensions.
    #[inline]
    #[must_use = "this performs a memory allocation as big as the area of the field"]
    pub fn empty(width: NonZeroUsize, height: NonZeroUsize) -> Self {
        assert!(height.get() > 0);
        assert!(width.get()  > 0);
        let mut tfield = Self {
            storage: Vec::with_capacity(width.get() * height.get()),
            dimensions: (width, height)
        };
        for _ in 0..(width.get() * height.get()) {
            tfield.storage.push(Tile::default());
        }
        tfield
    }
    /// Adds mines with the selected percentage of mines and, optionally, a safe spot, which can never have any surrounding mines.
    #[cfg(feature = "generation")]
    pub fn populate(&mut self, mine_percentage: f64, safe_spot: Option<(usize, usize)>) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let area = self.dimensions.0.get() * self.dimensions.1.get();
        let num_mines: usize = (area as f64 * mine_percentage).round() as usize; // The number of mines is usize because the area is usize.

        // We're using loop+counter instead of a range because we don't want to just discard a mine if it collides with the safe spot. Instead, we're going to
        // skip over the decrement and retry. This might freeze the game if the RNG chooses to hit the safe spot multiple times, but that's so unlikely that
        // we're going to disregard that for the sake of this example.
        let mut mines_left = num_mines;
        loop {
            let rnum: usize = rng.gen_range(0, area);
            let mine_location = (rnum % self.dimensions.0.get(), rnum / self.dimensions.0.get());
            if let Some(spot) = safe_spot {
                if mine_location == spot {
                    continue; // Jumps over the decrement.
                }
            }
            self[mine_location] = Tile::Mine(Flag::NotFlagged); // Install the mine.
            if mines_left == 0 {break};
            mines_left -= 1; // Implicit else, decrements otherwise.
        }
    }
    /// Returns the width and height of the field.
    #[inline(always)]
    pub const fn dimensions(&self) -> (NonZeroUsize, NonZeroUsize) {
        self.dimensions
    }
    /// Returns `true` if the field is fully solved (game win condition), `false` otherwise.
    #[must_use = "traversing the entire field is obscenely expensive"]
    pub fn solved(&self) -> bool {
        self.tiles_to_open() > 0
    }
    /// Returns the amount of tiles which have been already opened.
    #[must_use = "traversing the entire field is obscenely expensive"]
    pub fn count_open_tiles(&self) -> usize {
        let mut count = 0_usize;
        for column in self.columns() {
            for tile in column {
                if tile.is_open() {count += 1};
            }
        }
        count
    }
    /// Returns the amount of tiles which have not been opened yet.
    #[must_use = "traversing the entire field is obscenely expensive"]
    pub fn count_closed_tiles(&self) -> usize {
        let mut count = 0_usize;
        for column in self.columns() {
            for tile in column {
                if tile.is_closed() {count += 1};
            }
        }
        count
    }
    /// Returns the amount of tiles which the player needs to open in order to win the game.
    ///
    /// This does not include already opened tiles and is not equal to the 3BV value for the field.
    #[must_use = "traversing the entire field is obscenely expensive"]
    pub fn tiles_to_open(&self) -> usize {
        let mut count = 0_usize;
        for column in self.columns() {
            for tile in column {
                if tile.is_required_to_open() {count += 1};
            }
        }
        count
    }
    /// Counts all neigboring mines around a spot.
    ///
    /// All directly and diagonally adjacent mines are considered neighboring. If the tile is a mine, the tile itself isn't counted.
    #[must_use = "this is a rather complex lookup with 16 branch points"]
    pub fn count_neighboring_mines(&self, location: (usize, usize)) -> u8 {
        let mut count = 0_u8;
        if let Some(b) = self.is_mine((location.0 - 1, location.1 + 1)) {if b {count += 1;}};
        if let Some(b) = self.is_mine((location.0    , location.1 + 1)) {if b {count += 1;}};
        if let Some(b) = self.is_mine((location.0 + 1, location.1 + 1)) {if b {count += 1;}};

        if let Some(b) = self.is_mine((location.0 - 1, location.1    )) {if b {count += 1;}};
        // Skip center
        if let Some(b) = self.is_mine((location.0 + 1, location.1    )) {if b {count += 1;}};

        if let Some(b) = self.is_mine((location.0 - 1, location.1 - 1)) {if b {count += 1;}};
        if let Some(b) = self.is_mine((location.0    , location.1 - 1)) {if b {count += 1;}};
        if let Some(b) = self.is_mine((location.0 + 1, location.1 - 1)) {if b {count += 1;}};
        count
    }
    /// Detects whether a location is a mine, or `None` if it's out of bounds.
    #[inline]
    pub fn is_mine(&self, location: (usize, usize)) -> Option<bool> {
        if location.0 > self.dimensions.0.get() || location.1 > self.dimensions.1.get() {
            return None;
        }
        let tile = self[location];
        if let Tile::Mine(_) = tile {
            Some(true)
        } else {Some(false)}
    }

    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero, or `None` if the index is out of bounds.
    ///
    /// This is the immutable version of `get_mut`.
    #[inline]
    pub fn get(&self, index: (usize, usize)) -> Option<&Tile> {
        if index.0 > self.dimensions.0.get() || index.1 > self.dimensions.1.get() {return None};
        Some(unsafe{self.storage.get_unchecked(
              index.0
            + index.1 * self.dimensions.0.get()
        )})
    }
    /// Returns a mutable reference to the tile at the column `index.0` and row `index.1`, both starting at zero, or `None` if the index is out of bounds.
    ///
    /// This is the mutable version of `get`.
    #[inline]
    pub fn get_mut(&mut self, index: (usize, usize)) -> Option<&mut Tile> {
        if index.0 > self.dimensions.0.get() || index.1 > self.dimensions.1.get() {return None};
        Some(unsafe{self.storage.get_unchecked_mut(
            index.0
          + index.1 * self.dimensions.0.get()
        )})
    }
    /// Returns the outcome of clicking the specified tile **without affecting the field**, or `None` if the index is out of bounds.
    pub fn peek(&self, index: (usize, usize)) -> Option<ClickOutcome> {
        if let Some(tile) = self.get(index) {
            if let Some(outcome) = tile.peek_local() {
                Some(outcome)
            } else {
                let neighbors = self.count_neighboring_mines(index);
                if neighbors > 0 {
                    Some(ClickOutcome::OpenNumber( unsafe {
                        NonZeroU8::new_unchecked(neighbors)
                    })) // We can go for unsafe here since being in this branch implies the check.
                } else {
                    Some(ClickOutcome::OpenClearing)
                }
            }
        } else {None}
    }
    /// Opens **exactly one** tile and returns the outcome of clicking it. **Chords and clearings are not handled** and must be executed manually.
    ///
    /// Essentially, this replaces a `ClosedEmpty` tile with either an `OpenEmpty` or an `OpenNumber` tile.
    pub fn open(&mut self, index: (usize, usize)) -> Option<ClickOutcome> {
        if let Some(outcome) = self.peek(index) {
            match outcome {
                ClickOutcome::OpenClearing => self[index] = Tile::OpenEmpty,
                ClickOutcome::OpenNumber(num) => self[index] = Tile::OpenNumber(num),
                _ => {}
            };
            Some(outcome)
        } else {None}
    }
    /// Performs a chord on the specified tile. Optionally can
    ///
    /// Returns the oucomes for all 8 tiles touched.
    #[allow(clippy::redundant_closure_call)] // This lint shall not be a thing.
    pub fn chord(&mut self, index: (usize, usize)) -> [ClickOutcome; 8] {
        let num_mines = self.count_neighboring_mines(index);
        let mut result = [ClickOutcome::Nothing; 8];
        if num_mines == 0 {
            return result; // Short-circuit if we're not in a valid chord position.
        }

        let mut num_flags = 0_u8;
        if self[(index.0 - 1, index.1 + 1)].is_flagged() {num_flags += 1}; // Up-left,
        if self[(index.0    , index.1 + 1)].is_flagged() {num_flags += 1}; // up,
        if self[(index.0 + 1, index.1 + 1)].is_flagged() {num_flags += 1}; // up-right,
        if self[(index.0 + 1, index.1    )].is_flagged() {num_flags += 1}; // right,
        if self[(index.0 + 1, index.1 - 1)].is_flagged() {num_flags += 1}; // down-right,
        if self[(index.0    , index.1 - 1)].is_flagged() {num_flags += 1}; // down,
        if self[(index.0 - 1, index.1 - 1)].is_flagged() {num_flags += 1}; // down-left,
        if self[(index.0 - 1, index.1    )].is_flagged() {num_flags += 1}; // and left.

        if num_flags < num_mines {
            return result // We can't chord without enough flags.
        };

        let calc_result = |coords: (usize, usize)| {
            let tile = self[coords];
            if !tile.is_flagged() {
                self.peek(coords).unwrap_or_default()
            } else {
                ClickOutcome::Nothing
            }
        };
        result[0] = calc_result((index.0    , index.1 + 1));
        result[1] = calc_result((index.0 + 1, index.1 + 1));
        result[2] = calc_result((index.0 + 1, index.1    ));
        result[3] = calc_result((index.0 + 1, index.1 - 1));
        result[4] = calc_result((index.0    , index.1 - 1));
        result[5] = calc_result((index.0 - 1, index.1 - 1));
        result[6] = calc_result((index.0 - 1, index.1    ));
        result[7] = calc_result((index.0 - 1, index.1 + 1));

        result
    }
    /* TODO This is unfinished.
    /// Performs a chord on the specified tile recursively,i.e. runs chords for all number tiles which were uncovered from chording.
    pub fn recursive_chord(&mut self, index: (usize, usize)) {
        // Similar to the clearing algorithm, we're using a stack frame type here.
            // The meanings of values are pretty similar to the ones seen there.
            // The key difference is that the second element is an array for the sake of readability.
            // The directions are, in this exact order, up-left, up, up-right, right, down-right, down, down-left, left.
            type StackFrame = ((usize, usize), [bool; 8]);
            // We're gonna need less recursion depth here.
            let mut stack = Vec::<StackFrame>::with_capacity(8);
            let mut stack_top = (index, [true; 8]);
    }*/

    /// Returns an iterator over a single row.
    ///
    /// Said iterator can then also be indexed, thus serving as a versatile reference to a specific row.
    #[inline(always)]
    pub fn row(&self, row: usize) -> RowIter<'_> {
        RowIter::new(self, row)
    }
    /// Returns an iterator over a single column.
    ///
    /// Said iterator can then also be indexed, thus serving as a versatile reference to a specific column.
    #[inline(always)]
    pub fn column(&self, column: usize) -> ColumnIter<'_> {
        ColumnIter::new(self, column)
    }

    /// Returns an iterator over the field's columns.
    #[inline(always)]
    pub fn rows(&self) -> FieldRowsIter<'_> {
        FieldRowsIter::new(self)
    }
    /// Returns an iterator over the field's columns.
    #[inline(always)]
    pub fn columns(&self) -> FieldColumnsIter<'_> {
        FieldColumnsIter::new(self)
    }
    /// Returns a `Clearing` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    #[inline(always)]
    pub fn clearing(&self, anchor_location: (usize, usize)) -> Option<Clearing> {
        Clearing::new(self, anchor_location)
    }
    /// Returns a `ClearingMut` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn clearing_mut(&mut self, anchor_location: (usize, usize)) -> Option<ClearingMut> {
        ClearingMut::new(self, anchor_location)
    }

    /// Calculates the smallest amount of clicks required to clear a field.
    ///
    /// Since the field is modified in an undefined way in the process, it is taken by value.
    #[must_use = "calculating the 3BV value for any possible field requires traversing the entire field two times and opening clearings"]
    pub fn calculate_3bv(mut self) -> usize {
        let mut result = 0_usize;
        // First pass: close all clearings.
        for y in 0..self.dimensions.1.get() {
            for x in 0..self.dimensions.0.get() {
                match self[(x, y)] {
                    Tile::OpenEmpty
                  | Tile::OpenNumber(_) => {
                        self[(x, y)] = Tile::ClosedEmpty(Flag::NotFlagged)
                    }
                    _ => {}
                };
            }
        }
        // Second pass: count numbered tiles and clearings.
        for y in 0..self.dimensions.1.get() {
            for x in 0..self.dimensions.0.get() {
                match self[(x, y)] {
                    Tile::ClosedEmpty(_) => {
                        let outcome = self.open((x, y))
                            .expect("unexpected out of index error during 3BV calculation");
                        match outcome {
                            ClickOutcome::OpenClearing => {
                                self.clearing_mut((x, y))
                                    .expect("unexpected out of index error during 3BV calculation")
                                    .open(true);
                                result += 1;
                            },
                            ClickOutcome::OpenNumber(_) => result += 1,
                            _ => {}
                        };
                    },
                    Tile::OpenNumber(_) => result += 1,
                    _ => {}
                };
            }
        }
        result
    }
}
impl Index<(usize, usize)> for Field {
    type Output = Tile;
    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero.
    ///
    /// # Panics
    /// Index checking is enabled for this method. For a version which returns an `Option` instead of panicking if the index is out of bounds, see `get`.
    #[inline(always)]
    #[cfg_attr(features = "track_caller", track_caller)]
    fn index(&self, index: (usize, usize)) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}
impl IndexMut<(usize, usize)> for Field {
    #[inline(always)]
    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero.
    ///
    /// # Panics
    /// Index checking is enabled for this method. For a version which returns an `Option` instead of panicking if the index is out of bounds, see `get_mut`.
    #[inline(always)]
    #[cfg_attr(features = "track_caller", track_caller)]
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}