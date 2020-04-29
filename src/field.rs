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
#[cfg(feature = "serialization")]
use serde::{Serialize, Deserialize};
use crate::{
    Tile, Flag, ClickOutcome,
    Clearing, ClearingMut,
    RowIter, ColumnIter,
    FieldRowsIter, FieldColumnsIter
};

/// Represents a playfield.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Field {
    storage: Vec<Tile>,
    dimensions: FieldDimensions
}
/// The dimensions of a field.
///
/// The first element specifies the width (the number of columns), while the second one specifies the height (number of rows). As required by `NonZeroUsize`, a field cannot be smaller than 1x1.
pub type FieldDimensions = [NonZeroUsize; 2];
/// The coordinates of a tile on a field.
///
/// The first element specifies the column index (X coordinate), while the second one specifies the row index (Y coordinate). This is different from `FieldDimensions`, since the coordinate system starts from zero, i.e. the coordinates `[0, 0]` correspond to the top left corner and the only tile of a 1x1 field.
pub type FieldCoordinates = [usize; 2];
/// The outcome of a chord operation.
///
/// The entries are the adjacent & diagonal tiles in clockwise order, starting from top-left: ↖, ↑, ↗, →, ↘, ↓, ↙, ←.
pub type ChordOutcome = [ClickOutcome; 8];
/// The outcome of one of the chords in a recursive chord operation.
///
/// The `Chord` variant of `ClickOutcome` does **not** require any processing — all chords reported by these have already been executed by the time the function finishes execution.
pub type RecursiveChordOutcome = (FieldCoordinates, ChordOutcome);
impl Field {
    /// Creates an empty field filled with unopened tiles, with the given dimensions.
    #[inline]
    #[must_use = "this performs a memory allocation as big as the area of the field"]
    pub fn empty(dimensions: FieldDimensions) -> Self {
        let (width, height) = (dimensions[0].get(), dimensions[1].get());
        let mut tfield = Self {
            storage: Vec::with_capacity(width * height),
            dimensions
        };
        for _ in 0..(width * height) {
            tfield.storage.push(Tile::default());
        }
        tfield
    }
    /// Adds mines with the selected percentage of mines and, optionally, a safe spot, which can never have any surrounding mines.
    #[cfg(feature = "generation")]
    pub fn populate(&mut self, mine_percentage: f64, safe_spot: Option<FieldCoordinates>) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let (width, height) = (self.dimensions[0].get(), self.dimensions[1].get());

        let area = width * height;
        let num_mines: usize = (area as f64 * mine_percentage).round() as usize; // The number of mines is usize because the area is usize.

        // We're using loop+counter instead of a range because we don't want to just discard a mine if it collides with the safe spot. Instead, we're going to
        // skip over the decrement and retry. This might freeze the game if the RNG chooses to hit the safe spot multiple times, but that's so unlikely that
        // we're going to disregard that for the sake of this example.
        let mut mines_left = num_mines;
        loop {
            let rnum: usize = rng.gen_range(0, area);
            let mine_location = [rnum % width, rnum / width];
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
    pub const fn dimensions(&self) -> FieldDimensions {
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
    pub fn count_neighboring_mines(&self, location: FieldCoordinates) -> u8 {
        let mut count = 0_u8;
        if let Some(b) = self.is_mine([location[0] - 1, location[1] + 1]) {if b {count += 1;}};
        if let Some(b) = self.is_mine([location[0]    , location[1] + 1]) {if b {count += 1;}};
        if let Some(b) = self.is_mine([location[0] + 1, location[1] + 1]) {if b {count += 1;}};

        if let Some(b) = self.is_mine([location[0] - 1, location[1]    ]) {if b {count += 1;}};
        // Skip center
        if let Some(b) = self.is_mine([location[0] + 1, location[1]    ]) {if b {count += 1;}};

        if let Some(b) = self.is_mine([location[0] - 1, location[1] - 1]) {if b {count += 1;}};
        if let Some(b) = self.is_mine([location[0]    , location[1] - 1]) {if b {count += 1;}};
        if let Some(b) = self.is_mine([location[0] + 1, location[1] - 1]) {if b {count += 1;}};
        count
    }
    /// Detects whether a location is a mine, or `None` if it's out of bounds.
    #[inline]
    pub fn is_mine(&self, location: FieldCoordinates) -> Option<bool> {
        let (width, height) = (self.dimensions[0].get(), self.dimensions[1].get());

        if location[0] > width || location[1] > height {
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
    pub fn get(&self, coordinates: FieldCoordinates) -> Option<&Tile> {
        let (width, height) = (self.dimensions[0].get(), self.dimensions[1].get());
        let (x, y) = (coordinates[0], coordinates[1]);

        if x > width || y > height {return None};
        Some(unsafe{self.storage.get_unchecked(
              x
            + y * width
        )})
    }
    /// Returns a mutable reference to the tile at the column `index.0` and row `index.1`, both starting at zero, or `None` if the index is out of bounds.
    ///
    /// This is the mutable version of `get`.
    #[inline]
    pub fn get_mut(&mut self, coordinates: FieldCoordinates) -> Option<&mut Tile> {
        let (width, height) = (self.dimensions[0].get(), self.dimensions[1].get());
        let (x, y) = (coordinates[0], coordinates[1]);

        if x > width || y > height {return None};
        Some(unsafe{self.storage.get_unchecked_mut(
            x
          + y * width
        )})
    }
    /// Returns the outcome of clicking the specified tile **without affecting the field**, or `None` if the index is out of bounds.
    pub fn peek(&self, coordinates: FieldCoordinates) -> Option<ClickOutcome> {
        if let Some(tile) = self.get(coordinates) {
            if let Some(outcome) = tile.peek_local() {
                Some(outcome)
            } else {
                let neighbors = self.count_neighboring_mines(coordinates);
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
    pub fn open(&mut self, coordinates: FieldCoordinates) -> Option<ClickOutcome> {
        if let Some(outcome) = self.peek(coordinates) {
            match outcome {
                ClickOutcome::OpenClearing => self[coordinates] = Tile::OpenEmpty,
                ClickOutcome::OpenNumber(num) => self[coordinates] = Tile::OpenNumber(num),
                _ => {}
            };
            Some(outcome)
        } else {None}
    }
    /// Performs a chord on the specified tile.
    ///
    /// Returns the oucomes for all 8 tiles touched.
    #[allow(clippy::redundant_closure_call)] // This lint shall not be a thing.
    pub fn chord(&mut self, coordinates: FieldCoordinates) -> ChordOutcome {
        let (x, y) = (coordinates[0], coordinates[1]);

        let mut result = [ClickOutcome::Nothing; 8];
        let num_mines = if let Some(tile) = self.get(coordinates) {
            if let Tile::OpenNumber(num_mines) = tile {
                num_mines.get()
            } else {return result}
        } else {return result};

        let mut num_flags = 0_u8;
        let mut ckflag = |coords: FieldCoordinates| {
            if let Some(tile) = self.get(coords) {
                if tile.is_flagged() {
                    num_flags += 1;
                }
            }
        };
        ckflag([x - 1, y + 1]); // Up-left,
        ckflag([x    , y + 1]); // up,
        ckflag([x + 1, y + 1]); // up-right,
        ckflag([x + 1, y    ]); // right,
        ckflag([x + 1, y - 1]); // down-right,
        ckflag([x    , y - 1]); // down,
        ckflag([x - 1, y - 1]); // down-left,
        ckflag([x - 1, y    ]); // and left.

        if num_flags < num_mines {
            return result // We can't chord without enough flags.
        };

        let calc_result = |coords: FieldCoordinates| {
            if let Some(tile) = self.get(coords) {
                if !tile.is_flagged() {
                self.peek(coords).unwrap_or_default()
                } else {
                    ClickOutcome::Nothing
                }
            } else {
                ClickOutcome::Nothing
            }
        };
        result[0] = calc_result([x    , y + 1]);
        result[1] = calc_result([x + 1, y + 1]);
        result[2] = calc_result([x + 1, y    ]);
        result[3] = calc_result([x + 1, y - 1]);
        result[4] = calc_result([x    , y - 1]);
        result[5] = calc_result([x - 1, y - 1]);
        result[6] = calc_result([x - 1, y    ]);
        result[7] = calc_result([x - 1, y + 1]);

        result
    }
    // TODO This is unfinished.
    /// Performs a chord on the specified tile recursively, i.e. runs chords for all number tiles which were uncovered from chording.
    ///
    /// The returned value contains one entry per chord operation
    pub fn recursive_chord(&mut self, index: FieldCoordinates) -> Vec<RecursiveChordOutcome> {
        // Similar to the clearing algorithm, we're using a stack frame type here.
        // The meanings of values are pretty similar to the ones seen there.
        // The key difference is that the second element is an array for the sake of readability.
        // The directions are, in this exact order, up-left, up, up-right, right, down-right, down, down-left, left.
        type StackFrame = (FieldCoordinates, [bool; 8]);
        // We're gonna need less recursion depth here.
        let mut stack = Vec::<StackFrame>::with_capacity(8);
        let mut stack_top = (index, [true; 8]);

        // The return value will be stored as a Vec of all the chord outcomes coupled with the coordinates at which they occurred.
        let mut chord_outcomes = Vec::<RecursiveChordOutcome>::with_capacity(8);
        loop {
            let chosen_location =
                 if stack_top.1[0] {stack_top.1[0] = false; 0}
            else if stack_top.1[1] {stack_top.1[1] = false; 1}
            else if stack_top.1[2] {stack_top.1[2] = false; 2}
            else if stack_top.1[3] {stack_top.1[3] = false; 3}
            else if stack_top.1[4] {stack_top.1[4] = false; 4}
            else if stack_top.1[5] {stack_top.1[5] = false; 5}
            else if stack_top.1[6] {stack_top.1[6] = false; 6}
            else if stack_top.1[7] {stack_top.1[7] = false; 7}
            else if let Some(new_top) = stack.pop() {
                stack_top = new_top;
                continue;
            } else {break};

            let location_to_chord = match chosen_location {
                0 => [stack_top.0[0] - 1, stack_top.0[1] + 1], // Up & left,
                1 => [stack_top.0[0]    , stack_top.0[1] + 1], // up,
                2 => [stack_top.0[0] + 1, stack_top.0[1] + 1], // up & right,
                3 => [stack_top.0[0] + 1, stack_top.0[1]    ], // right,
                4 => [stack_top.0[0] + 1, stack_top.0[1] - 1], // down & right,
                5 => [stack_top.0[0]    , stack_top.0[1] - 1], // down,
                6 => [stack_top.0[0] - 1, stack_top.0[1] - 1], // down & left,
                7 => [stack_top.0[0] - 1, stack_top.0[1]    ], // and left.
                _ => unreachable!()
            };

            let outcome = self.chord(location_to_chord);
            chord_outcomes.push((location_to_chord, outcome));
            if !(outcome == [ClickOutcome::Nothing; 8]) {
                stack.push(stack_top);
                stack_top = (location_to_chord, [true; 8]);
                continue;
            }
        };
        chord_outcomes
    }

    /// Returns an iterator over a single row.
    ///
    /// Said iterator can then also be indexed, thus serving as a versatile reference to a specific row.
    ///
    /// # Panics
    /// Panics if the specified row is out of range.
    #[inline(always)]
    pub fn row(&self, row: usize) -> RowIter<'_> {
        RowIter::new(self, row)
    }
    /// Returns an iterator over a single column.
    ///
    /// Said iterator can then also be indexed, thus serving as a versatile reference to a specific column.
    ///
    /// # Panics
    /// Panics if the specified column is out of range.
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
    pub fn clearing(&self, anchor_location: FieldCoordinates) -> Option<Clearing> {
        Clearing::new(self, anchor_location)
    }
    /// Returns a `ClearingMut` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn clearing_mut(&mut self, anchor_location: FieldCoordinates) -> Option<ClearingMut> {
        ClearingMut::new(self, anchor_location)
    }

    /// Calculates the smallest amount of clicks required to clear a field.
    ///
    /// Since the field is modified in an undefined way in the process, it is taken by value.
    #[must_use = "calculating the 3BV value for any possible field requires traversing the entire field two times and opening clearings"]
    pub fn calculate_3bv(mut self) -> usize {
        let mut result = 0_usize;
        // First pass: close all clearings.
        for y in 0..self.dimensions[1].get() {
            for x in 0..self.dimensions[0].get() {
                match self[[x, y]] {
                    Tile::OpenEmpty
                  | Tile::OpenNumber(_) => {
                        self[[x, y]] = Tile::ClosedEmpty(Flag::NotFlagged)
                    }
                    _ => {}
                };
            }
        }
        // Second pass: count numbered tiles and clearings.
        for y in 0..self.dimensions[1].get() {
            for x in 0..self.dimensions[0].get() {
                match self[[x, y]] {
                    Tile::ClosedEmpty(_) => {
                        let outcome = self.open([x, y])
                            .expect("unexpected out of index error during 3BV calculation");
                        match outcome {
                            ClickOutcome::OpenClearing => {
                                self.clearing_mut([x, y])
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
impl Index<FieldCoordinates> for Field {
    type Output = Tile;
    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero.
    ///
    /// # Panics
    /// Index checking is enabled for this method. For a version which returns an `Option` instead of panicking if the index is out of bounds, see `get`.
    #[inline(always)]
    fn index(&self, coordinates: FieldCoordinates) -> &Self::Output {
        self.get(coordinates).expect("index out of bounds")
    }
}
impl IndexMut<FieldCoordinates> for Field {
    #[inline(always)]
    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero.
    ///
    /// # Panics
    /// Index checking is enabled for this method. For a version which returns an `Option` instead of panicking if the index is out of bounds, see `get_mut`.
    #[inline(always)]
    fn index_mut(&mut self, coordinates: FieldCoordinates) -> &mut Self::Output {
        self.get_mut(coordinates).expect("index out of bounds")
    }
}