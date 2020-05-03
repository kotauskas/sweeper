//! The playfield of a Minesweeper game.
//!
//! This is the main point of interest for the game — everything happens here. For that reason, this module is the most detailed one.

use core::{
    ops::{Index, IndexMut},
    num::{NonZeroUsize, NonZeroU8},
};
#[cfg(feature = "serialization")]
use core::{
    fmt::{self, Formatter},
    marker::PhantomData,
};
use alloc::{
    vec::Vec
};
#[cfg(feature = "serialization")]
use serde::{
    Serialize, Deserialize,
    ser::{Serializer, SerializeStruct},
    de::{Deserializer, Visitor, MapAccess, SeqAccess}
};
use crate::{
    Tile, TileState, Flag, ClickOutcome,
    Clearing, ClearingMut,
    RowIter, ColumnIter,
    FieldRowsIter, FieldColumnsIter
};

/// Represents a playfield.
///
/// Fields in Minesweeper are matrices of [tiles][tile]. The winning condition is when all tiles without mines are opened. Sweeper doesn't automatically perform that: `Field` objects provide helpful methods which implementations call when the user performs certain input actions, like left-clicking a closed tile or right-clicking a number tile. The former typically maps to a call to [`open`][m_open], while the latter triggers either [`chord`][m_chord] or [`recursive_chord`][m_rechord], depending on user settings.
///
/// [tile]: struct.Tile.html "Tile — a tile on a Minesweeper field"
/// [m_open]: #method.open "open — opens exactly one tile and returns the outcome of clicking it"
/// [m_chord]: #method.chord "chord — performs a chord operation on the specified tile"
/// [m_rechord]: #method.recursive_chord "recursive_chord — performs a chord operation on the specified tile recursively, i.e. runs chords for all number tiles which were uncovered from chording"
pub struct Field<Ct: 'static, Cf: 'static> {
    dimensions: FieldDimensions,
    storage: Vec<Tile<Ct, Cf>>,
}
/// The dimensions of a field.
///
/// The first element specifies the width (the number of columns), while the second one specifies the height (number of rows). As required by `NonZeroUsize`, a field cannot be smaller than 1x1.
pub type FieldDimensions = [NonZeroUsize; 2];
/// The coordinates of a tile on a field.
///
/// The first element specifies the column index (X coordinate), while the second one specifies the row index (Y coordinate). This is different from `FieldDimensions`, since the coordinate system starts from zero, i.e. the coordinates `[0, 0]` correspond to the top left corner and the only tile of a 1x1 field.
pub type FieldCoordinates = [usize; 2];
/// The outcome of a [chord operation][m_chord].
///
/// The entries are the adjacent & diagonal tiles in clockwise order, starting from top-left: ↖, ↑, ↗, →, ↘, ↓, ↙, ←.
///
/// [m_chord]: #method.chord "chord — performs a chord operation on the specified tile"
pub type ChordOutcome = [ClickOutcome; 8];
/// The outcome of one of the chords in a [recursive chord operation][m_rechord].
///
/// The `Chord` variant of `ClickOutcome` does **not** require any processing — all chords reported by these have already been executed by the time the function finishes execution.
///
/// [m_rechord]: #method.recursive_chord "recursive_chord — performs a chord operation on the specified tile recursively, i.e. runs chords for all number tiles which were uncovered from chording"
pub type RecursiveChordOutcome = (FieldCoordinates, ChordOutcome);
impl<Ct: Default, Cf> Field<Ct, Cf> {
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
}
impl<Ct, Cf> Field<Ct, Cf> {
    /// Creates a field with the specified dimensions from the specified `Vec` of tiles, given in [row-major order][rmo].
    ///
    /// Keep in mind that indexing over fields is still done in column-major order.
    ///
    /// [rmo]: https://en.wikipedia.org/wiki/Row-_and_column-major_order "Row- and column-major order — Wikipedia"
    #[must_use]
    pub fn from_dimensions_and_storage(dimensions: FieldDimensions, storage: Vec<Tile<Ct, Cf>>) -> Option<Self> {
        let area = dimensions[0].get() * dimensions[1].get();
        if storage.len() == area {
            Some(Self {dimensions, storage})
        } else {
            None
        }
    }
    /// Adds mines with the selected percentage of mines and, optionally, a safe spot, which can never have any surrounding mines.
    #[cfg(feature = "generation")]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
    pub fn populate(&mut self, mine_percentage: f64, safe_spot: Option<FieldCoordinates>) {
        use rand::Rng;
        assert!(mine_percentage > 0.0); // no
        let mut rng = rand::thread_rng();

        let (width, height) = (self.dimensions[0].get(), self.dimensions[1].get());

        let area = width * height;
        let num_mines: usize = (area as f64 * mine_percentage).round() as usize; // The number of mines is usize because the area is usize.
        if safe_spot.is_some() {
            assert!(area > num_mines);
        } else {
            assert!(area >= num_mines);
        }

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
            self[mine_location].state = TileState::Mine(Flag::NotFlagged); // Install the mine.
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
                if tile.state.is_open() {count += 1};
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
                if tile.state.is_closed() {count += 1};
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
                if tile.state.is_required_to_open() {count += 1};
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
        let tile = &self[location];
        if let TileState::Mine(_) = tile.state {
            Some(true)
        } else {Some(false)}
    }

    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero, or `None` if the index is out of bounds.
    ///
    /// This is the immutable version of `get_mut`.
    #[inline]
    pub fn get(&self, coordinates: FieldCoordinates) -> Option<&Tile<Ct, Cf>> {
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
    pub fn get_mut(&mut self, coordinates: FieldCoordinates) -> Option<&mut Tile<Ct, Cf>> {
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
            if let Some(outcome) = tile.state.peek_local() {
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
                ClickOutcome::OpenClearing => self[coordinates].state = TileState::OpenEmpty,
                ClickOutcome::OpenNumber(num) => self[coordinates].state = TileState::OpenNumber(num),
                _ => {}
            };
            Some(outcome)
        } else {None}
    }
    /// Performs a chord on the specified tile and returns the [outcomes][chord_outcome] for all 8 tiles touched.
    ///
    /// Chord operations in Minesweeper are special convenience operations ran on number tiles. If the amount of mines around a number tile (displayed on its number) is exactly equal to the amount of flags around it, all other tiles can be opened, causing a gameover condition if the flags were placed incorrectly. This method performs just that: counts the surrounding flags and mines and opens the unflagged tiles if these two metrics match.
    ///
    /// [chord_outcome]: type.ChordOutcome.html "ChordOutcome — the outcome of a chord operation"
    #[allow(clippy::redundant_closure_call)] // This lint shall not be a thing.
    pub fn chord(&mut self, coordinates: FieldCoordinates) -> ChordOutcome {
        let (x, y) = (coordinates[0], coordinates[1]);

        let mut result = [ClickOutcome::Nothing; 8];
        let num_mines = if let Some(tile) = self.get(coordinates) {
            if let TileState::OpenNumber(num_mines) = tile.state {
                num_mines.get()
            } else {return result}
        } else {return result};

        let mut num_flags = 0_u8;
        let mut ckflag = |coords: FieldCoordinates| {
            if let Some(tile) = self.get(coords) {
                if tile.state.is_flagged() {
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

        if num_flags != num_mines {
            return result // We can't chord without enough flags or with too many.
        };

        let calc_result = |coords: FieldCoordinates| {
            if let Some(tile) = self.get(coords) {
                if !tile.state.is_flagged() {
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
    pub fn row(&self, row: usize) -> RowIter<'_, Ct, Cf> {
        RowIter::new(self, row)
    }
    /// Returns an iterator over a single column.
    ///
    /// Said iterator can then also be indexed, thus serving as a versatile reference to a specific column.
    ///
    /// # Panics
    /// Panics if the specified column is out of range.
    #[inline(always)]
    pub fn column(&self, column: usize) -> ColumnIter<'_, Ct, Cf> {
        ColumnIter::new(self, column)
    }

    /// Returns an iterator over the field's columns.
    #[inline(always)]
    pub fn rows(&self) -> FieldRowsIter<'_, Ct, Cf> {
        FieldRowsIter::new(self)
    }
    /// Returns an iterator over the field's columns.
    #[inline(always)]
    pub fn columns(&self) -> FieldColumnsIter<'_, Ct, Cf> {
        FieldColumnsIter::new(self)
    }
    /// Returns a `Clearing` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    #[inline(always)]
    pub fn clearing(&self, anchor_location: FieldCoordinates) -> Option<Clearing<Ct, Cf>> {
        Clearing::<'_, Ct, Cf>::new(self, anchor_location)
    }
    /// Returns a `ClearingMut` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn clearing_mut(&mut self, anchor_location: FieldCoordinates) -> Option<ClearingMut<Ct, Cf>> {
        ClearingMut::<'_, Ct, Cf>::new(self, anchor_location)
    }

    /// Calculates the 3BV value of the field.
    ///
    /// The Bechtel's Board Benchmark Value, or 3BV, is a way of measuring how difficult a Minesweeper field is. It is the smallest possible number of clicks which are required to win the field, ignoring all opportunities for chord operations to be able to calculate the value in a reasonable timespan. [Clearings][clearing] on a field add one point to this value per clearing. The remaining number tiles which are not surrounded by tiles without numbers also add one each. This metric favors players which do not use flags, but is still widely used nonetheless.
    ///
    /// Since the field is modified in an undefined way in the process, it is taken by value.
    ///
    /// [clearing]: struct.Clearing.html "Clearing — a clearing on the specified field"
    #[must_use = "calculating the 3BV value for any possible field requires traversing the entire field two times and opening clearings"]
    pub fn calculate_3bv(mut self) -> usize {
        let mut result = 0_usize;
        // First pass: close all clearings.
        for y in 0..self.dimensions[1].get() {
            for x in 0..self.dimensions[0].get() {
                match self[[x, y]].state {
                    TileState::OpenEmpty
                  | TileState::OpenNumber(_) => {
                        self[[x, y]].state = TileState::ClosedEmpty(Flag::NotFlagged)
                    }
                    _ => {}
                };
            }
        }
        // Second pass: count numbered tiles and clearings.
        for y in 0..self.dimensions[1].get() {
            for x in 0..self.dimensions[0].get() {
                match self[[x, y]].state {
                    TileState::ClosedEmpty(_) => {
                        let outcome = self.open([x, y])
                            .expect("unexpected out of index error during 3BV calculation");
                        match outcome {
                            ClickOutcome::OpenClearing => {
                                self.clearing_mut([x, y])
                                    .expect("unexpected out of index error during 3BV calculation")
                                    .open(true);
                                result += 1;
                            },
                            ClickOutcome::OpenNumber(_) => {
                                let belongs_to_a_clearing = |coords: FieldCoordinates| {
                                    let mut result = false;
                                    let mut cktile = |coords: FieldCoordinates| {
                                        if let Some(tile) = self.get(coords) {
                                            if let TileState::OpenEmpty = tile.state {result = true;}
                                            else {}
                                        } else {}
                                    };
                                    let [x, y] = coords;

                                    cktile([x - 1, y + 1]); // Up-left,
                                    cktile([x    , y + 1]); // up,
                                    cktile([x + 1, y + 1]); // up-right,
                                    cktile([x + 1, y    ]); // right,
                                    cktile([x + 1, y - 1]); // down-right,
                                    cktile([x    , y - 1]); // down,
                                    cktile([x - 1, y - 1]); // down-left,
                                    cktile([x - 1, y    ]); // and left.
                                    result
                                };
                                if belongs_to_a_clearing([x, y]) {
                                    result += 1;
                                }
                            },
                            _ => {}
                        };
                    },
                    TileState::OpenNumber(_) => result += 1,
                    _ => {}
                };
            }
        }
        result
    }
}
impl<Ct, Cf> Index<FieldCoordinates> for Field<Ct, Cf> {
    type Output = Tile<Ct, Cf>;
    /// Returns the tile at the column `index.0` and row `index.1`, both starting at zero.
    ///
    /// # Panics
    /// Index checking is enabled for this method. For a version which returns an `Option` instead of panicking if the index is out of bounds, see `get`.
    #[inline(always)]
    fn index(&self, coordinates: FieldCoordinates) -> &Self::Output {
        self.get(coordinates).expect("index out of bounds")
    }
}
impl<Ct, Cf> IndexMut<FieldCoordinates> for Field<Ct, Cf> {
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
#[cfg(feature = "serialization")]
impl<Ct, Cf> Serialize for Field<Ct, Cf>
where Ct: Serialize,
      Cf: Serialize {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
       let mut s = s.serialize_struct("Field", 2)?;
       s.serialize_field("dimensions", &self.dimensions)?;
       s.serialize_field("storage", &self.storage)?;
       s.end()
    }
}
#[cfg(feature = "serialization")]
impl<'de, Ct, Cf> Deserialize<'de> for Field<Ct, Cf>
where Ct: Deserialize<'de>,
      Cf: Deserialize<'de> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de;
        const FIELDS: &[&str] = &["storage", "dimensions"];
        enum StructField { Storage, Dimensions };

        // This part could also be generated independently by:
        //
        //    #[derive(Deserialize)]
        //    #[serde(field_identifier, rename_all = "lowercase")]
        //    enum Field { Secs, Nanos }
        impl<'de> Deserialize<'de> for StructField {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct StructFieldVisitor;

                impl<'de> Visitor<'de> for StructFieldVisitor {
                    type Value = StructField;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("`storage` or `dimensions`")
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<StructField, E> {
                        match value {
                            "storage" => Ok(StructField::Storage),
                            "dimensions" => Ok(StructField::Dimensions),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(StructFieldVisitor)
            }
        }

        struct FieldVisitor<Ct, Cf>(PhantomData<(Ct, Cf)>);

        impl<'de, Ct: 'static, Cf: 'static> Visitor<'de> for FieldVisitor<Ct, Cf>
        where Ct: Deserialize<'de>,
              Cf: Deserialize<'de> {
            type Value = Field<Ct, Cf>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("struct Field")
            }

            fn visit_seq<V: SeqAccess<'de>>(self, mut seq: V) -> Result<Self::Value, V::Error> {
                let dimensions = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let storage = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Field {dimensions, storage})
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
                let mut dimensions: Option<FieldDimensions> = None;
                let mut storage: Option<Vec<Tile<Ct, Cf>>> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        StructField::Dimensions => {
                            if dimensions.is_some() {
                                return Err(de::Error::duplicate_field("dimensions"));
                            }
                            dimensions = Some(map.next_value()?);
                        }
                        StructField::Storage => {
                            if storage.is_some() {
                                return Err(de::Error::duplicate_field("storage"));
                            }
                            storage = Some(map.next_value()?);
                        }
                    }
                }
                let dimensions = dimensions.ok_or_else(|| de::Error::missing_field("dimensions"))?;
                let storage = storage.ok_or_else(|| de::Error::missing_field("storage"))?;
                Ok(Field {dimensions, storage})
            }
        }
        d.deserialize_struct("Field", FIELDS, FieldVisitor(PhantomData))
    }
}