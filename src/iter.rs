//! Iterators useful for traversing a field.
//!
//! Currently available:
//! - [`ColumnIter`][columniter] — iterates over a single field column
//! - [`RowIter`][rowiter] — iterates over a single field row
//! - [`FieldRowsIter`][fri] — iterates over the rows of a field (each item is a [`RowIter`][rowiter])
//! - [`FieldColumnsIter`][fci] — iterates over the columns of a field (each item is a [`ColumnIter`][columniter])
//!
//! [rowiter]: struct.RowIter.html "RowIter — iterates over a single field row"
//! [columniter]: struct.ColumnIter.html "ColumnIter — iterates over a single field column"
//! [fri]: struct.FieldRowsIter.html "FieldRowsIter — an iterator over the rows of a field"
//! [fci]: struct.FieldColumnsIter.html "an iterator over the columns of a field"

use core::{
    ops::{Range, Index},
    iter::FusedIterator
};
use super::{
    Tile,
    Field
};

/// Iterates over a single field row.
///
/// Can also be indexed to pull arbitrary tiles from the row, regardless of the iterator state.
///
/// # Usage
/// ```
/// # use sweeper::{Field, TileState, Flag, RowIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::<(), ()>::empty([ // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(4).unwrap()
/// ]);
/// field[[8, 3]].state = TileState::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut rowiter = field.row(3); // Create an iterator over the fourth row
/// let mine_tile = rowiter.nth(8) // Find the nineth element in the row
///     .unwrap(); // Get rid of the Option wrap
/// assert_eq!(mine_tile.state, TileState::Mine(Flag::NotFlagged)); // It's a mine
/// ```
#[derive(Clone)]
pub struct RowIter<'f, Ct, Cf> {
    field: &'f Field<Ct, Cf>,
    row: usize,
    index: Range<usize>
}
impl<'f, Ct, Cf> RowIter<'f, Ct, Cf> {
    /// Creates an iterator over the specified row of the specified field.
    ///
    /// # Panics
    /// Panics if the specified row is out of range.
    #[inline(always)]
    pub fn new(field: &'f Field<Ct, Cf>, row: usize) -> Self {
        assert!(row < field.dimensions()[1].get());
        Self {field, row, index: 0..field.dimensions()[0].get()}
    }
    /// Returns the tile at the specified column, or `None` if such a column doesn't exist. The row for which the iterator was created is used.
    #[inline(always)]
    pub fn get(&self, column: usize) -> Option<&Tile<Ct, Cf>> {
        self.field.get([column, self.row])
    }
    /// Returns the tile at the specified column.
    ///
    /// Used as a convenience function, allowing you to write `field.row(y).column(x)` to find specific tiles.
    #[inline(always)]
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn column(&self, column: usize) -> &Tile<Ct, Cf> {
        self.get(column).expect("index out of bounds")
    }
    /// Returns the field which the iterator iterates over.
    #[inline(always)]
    pub const fn field(&self) -> &'f Field<Ct, Cf> {
        self.field
    }
}
impl<'f, Ct, Cf> Iterator for RowIter<'f, Ct, Cf> {
    type Item = &'f Tile<Ct, Cf>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index.end - self.index.start == 0 {
            return None;
        }
        let el = self.field.get([self.index.start, self.row]);
        self.index.start += 1;
        el
    }
    /// Returns the remaining amount of tiles to iterate upon.
    ///
    /// See `len` from the `ExactSizedIterator` trait.
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f, Ct, Cf> DoubleEndedIterator for RowIter<'f, Ct, Cf> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index.end - self.index.start == 0 {
            return None;
        }
        self.index.end -= 1;
        self.field.get([self.index.end, self.row])
    }
}
impl<'f, Ct, Cf> ExactSizeIterator for RowIter<'f, Ct, Cf> {
    /// Returns the remaining amount of tiles to iterate upon.
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl<Ct, Cf> FusedIterator for RowIter<'_, Ct, Cf> {}
impl<Ct, Cf> Index<usize> for RowIter<'_, Ct, Cf> {
    type Output = Tile<Ct, Cf>;
    /// Returns the tile at the specified column.
    ///
    /// Used as a convenience function, allowing you to write `field.row(y)[x]` to find specific tiles.
    ///
    /// This differs from [`.column()`][0] in the return type: this one returns a reference while `.column()` returns the tile by-value.
    ///
    /// [0]: #method.column.html "column — returns the tile at the specified column"
    #[inline(always)]
    fn index(&self, column: usize) -> &Tile<Ct, Cf> {
        self.field.get([column, self.row]).expect("index out of bounds")
    }
}

/// Iterates over a single field column.
///
/// Can also be indexed to pull arbitrary tiles from the column, regardless of the iterator state.
///
/// # Usage
/// ```
/// # use sweeper::{Field, TileState, Flag, ColumnIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::<(), ()>::empty([ // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(8).unwrap()
/// ]);
/// field[[8, 7]].state = TileState::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut columniter = field.column(8); // Create an iterator over the nineth column
/// let mine_tile = columniter.nth(7) // Find the seventh element in the column
///     .unwrap(); // Get rid of the Option wrap
/// assert_eq!(mine_tile.state, TileState::Mine(Flag::NotFlagged)); // It's a mine
/// ```
#[derive(Clone)]
pub struct ColumnIter<'f, Ct, Cf> {
    field: &'f Field<Ct, Cf>,
    column: usize,
    index: Range<usize>
}
impl<'f, Ct, Cf> ColumnIter<'f, Ct, Cf> {
    /// Creates an iterator over the specified column of the specified field.
    ///
    /// # Panics
    /// Panics if the specified row is out of range.
    #[inline(always)]
    pub fn new(field: &'f Field<Ct, Cf>, column: usize) -> Self {
        assert!(column < field.dimensions()[0].get());
        Self {field, column, index: 0..field.dimensions()[0].get()}
    }
    /// Returns the tile at the specified row, or `None` if such a row doesn't exist. The column for which the iterator was created is used.
    #[inline(always)]
    pub fn get(&self, row: usize) -> Option<&Tile<Ct, Cf>> {
        self.field.get([self.column, row])
    }
    /// Returns the tile at the specified row.
    ///
    /// Used as a convenience function, allowing you to write `field.column(x).row(y)` to find specific tiles.
    #[inline(always)]
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn row(&self, row: usize) -> &Tile<Ct, Cf> {
        self.get(row).expect("index out of bounds")
    }
    /// Returns the field which the iterator iterates over.
    #[inline(always)]
    pub const fn field(&self) -> &'f Field<Ct, Cf> {
        self.field
    }
}
impl<'f, Ct, Cf> Iterator for ColumnIter<'f, Ct, Cf> {
    type Item = &'f Tile<Ct, Cf>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        let el = self.field.get([self.column, self.index.start]);
        self.index.start += 1;
        el
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f, Ct, Cf> DoubleEndedIterator for ColumnIter<'f, Ct, Cf> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        self.field.get([self.column, self.index.end])
    }
}
impl<'f, Ct, Cf> ExactSizeIterator for ColumnIter<'f, Ct, Cf> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl<Ct, Cf> FusedIterator for ColumnIter<'_, Ct, Cf> {}
impl<Ct, Cf> Index<usize> for ColumnIter<'_, Ct, Cf> {
    type Output = Tile<Ct, Cf>;
    /// Returns the tile at the specified row.
    ///
    /// Used as a convenience function, allowing you to write `field.column(x)[y]` to find specific tiles.
    ///
    /// This differs from [`.row()`][0] in the return type: this one returns a reference while `.row()` returns the tile by-value.
    ///
    /// [0]: #method.row.html "row — returns the tile at the specified row"
    #[inline(always)]
    fn index(&self, row: usize) -> &Tile<Ct, Cf> {
        self.field.get([self.column, row]).expect("index out of bounds")
    }
}

/// An iterator over the rows of a field.
///
/// # Usage
/// ```
/// # use sweeper::{Field, TileState, Flag, FieldRowsIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::<(), ()>::empty([ // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(4).unwrap()
/// ]);
/// field[[8, 3]].state = TileState::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut row_with_mine: Option<usize> = None; // Keep track of our findings using an Option
/// for (y, mut row) in field.rows().enumerate() { // In each row...
///     if row.find(|t| (*t).state.is_mine()).is_some() { // If the row contains a mine...
///         row_with_mine = Some(y); //...take the row number out of the loop.
///     }
/// }
/// assert_eq!(row_with_mine, Some(3)); // We indeed have found a mine in the 4th row.
/// ```
#[derive(Clone)]
pub struct FieldRowsIter<'f, Ct, Cf> {
    field: &'f Field<Ct, Cf>,
    index: Range<usize>
}
impl<'f, Ct, Cf> FieldRowsIter<'f, Ct, Cf> {
    /// Returns an iterator over the specified field's rows.
    #[inline(always)]
    pub const fn new(field: &'f Field<Ct, Cf>) -> Self {
        Self {
            field, index: 0..field.dimensions()[1].get()
        }
    }
}
impl<'f, Ct, Cf> Iterator for FieldRowsIter<'f, Ct, Cf> {
    type Item = RowIter<'f, Ct, Cf>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        let el = Some(self.field.row(self.index.start));
        self.index.start += 1;
        el
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f, Ct, Cf> DoubleEndedIterator for FieldRowsIter<'f, Ct, Cf> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        Some(self.field.row(self.index.end))
    }
}
impl<'f, Ct, Cf> ExactSizeIterator for FieldRowsIter<'f, Ct, Cf> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl<Ct, Cf> FusedIterator for FieldRowsIter<'_, Ct, Cf> {}

/// An iterator over the columns of a field.
///
///  # Usage
/// ```
/// # use sweeper::{Field, TileState, Flag, FieldColumnsIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::<(), ()>::empty([ // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(8).unwrap()
/// ]);
/// field[[8, 7]].state = TileState::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut column_with_mine: Option<usize> = None; // Keep track of our findings using an Option
/// for (x, mut column) in field.columns().enumerate() { // In each column...
///     if column.find(|t| (*t).state.is_mine()).is_some() { // If the column contains a mine...
///         column_with_mine = Some(x); //...take the column number out of the loop.
///     }
/// }
/// assert_eq!(column_with_mine, Some(8)); // We indeed have found a mine in the 9th column.
/// ```
#[derive(Clone)]
pub struct FieldColumnsIter<'f, Ct, Cf> {
    field: &'f Field<Ct, Cf>,
    index: Range<usize>
}
impl<'f, Ct, Cf> FieldColumnsIter<'f, Ct, Cf> {
    /// Returns an iterator over the specified field's columns.
    #[inline(always)]
    pub const fn new(field: &'f Field<Ct, Cf>) -> Self {
        Self {
            field, index: 0..field.dimensions()[0].get()
        }
    }
}
impl<'f, Ct, Cf> Iterator for FieldColumnsIter<'f, Ct, Cf> {
    type Item = ColumnIter<'f, Ct, Cf>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        let el = Some(self.field.column(self.index.start));
        self.index.start += 1;
        el
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f, Ct, Cf> DoubleEndedIterator for FieldColumnsIter<'f, Ct, Cf> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        Some(self.field.column(self.index.end))
    }
}
impl<'f, Ct, Cf> ExactSizeIterator for FieldColumnsIter<'f, Ct, Cf> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl<Ct, Cf> FusedIterator for FieldColumnsIter<'_, Ct, Cf> {}