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
/// # use sweeper::{Field, Tile, Flag, RowIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::empty( // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(4).unwrap()
/// );
/// field[(8, 3)] = Tile::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut rowiter = field.row(3); // Create an iterator over the fourth row
/// let mine_tile = rowiter.nth(8) // Find the nineth element in the row
///     .unwrap(); // Get rid of the Option wrap
/// assert_eq!(mine_tile, Tile::Mine(Flag::NotFlagged)); // It's a mine
/// ```
#[derive(Clone)]
pub struct RowIter<'f> {
    field: &'f Field,
    row: usize,
    index: Range<usize>
}
impl<'f> RowIter<'f> {
    /// Creates an iterator over the specified row of the specified field.
    #[inline(always)]
    pub fn new(field: &'f Field, row: usize) -> Self {
        Self {field, row, index: 0..field.dimensions().0.get()}
    }
    /// Returns the tile at the specified column, or `None` if such a column doesn't exist. The row for which the iterator was created is used.
    #[inline(always)]
    pub fn get(&self, column: usize) -> Option<Tile> {
        self.field.get((column, self.row)).copied()
    }
    /// Returns the tile at the specified column.
    ///
    /// Used as a convenience function, allowing you to write `field.row(y).column(x)` to find specific tiles.
    #[inline(always)]
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn column(&self, column: usize) -> Tile {
        self.get(column).expect("index out of bounds")
    }
    /// Returns the field which the iterator iterates over.
    #[inline(always)]
    pub fn field(&self) -> &'f Field {
        self.field
    }
}
impl<'f> Iterator for RowIter<'f> {
    type Item = Tile;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index.end - self.index.start == 0 {
            return None;
        }
        let el = self.field.get((self.index.start, self.row));
        self.index.start += 1;
        el.copied()
    }
    /// Returns the remaining amount of tiles to iterate upon.
    ///
    /// See `len` from the `ExactSizedIterator` trait.
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f> DoubleEndedIterator for RowIter<'f> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index.end - self.index.start == 0 {
            return None;
        }
        self.index.end -= 1;
        self.field.get((self.index.end, self.row)).copied()
    }
}
impl<'f> ExactSizeIterator for RowIter<'f> {
    /// Returns the remaining amount of tiles to iterate upon.
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl FusedIterator for RowIter<'_> {}
impl Index<usize> for RowIter<'_> {
    type Output = Tile;
    /// Returns the tile at the specified column.
    ///
    /// Used as a convenience function, allowing you to write `field.row(y)[x]` to find specific tiles.
    ///
    /// This differs from [`.column()`][0] in the return type: this one returns a reference while `.column()` returns the tile by-value.
    ///
    /// [0]: #method.column.html "column — returns the tile at the specified column"
    #[inline(always)]
    fn index(&self, column: usize) -> &Tile {
        self.field.get((column, self.row)).expect("index out of bounds")
    }
}

/// Iterates over a single field column.
///
/// Can also be indexed to pull arbitrary tiles from the column, regardless of the iterator state.
///
/// # Usage
/// ```
/// # use sweeper::{Field, Tile, Flag, ColumnIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::empty( // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(8).unwrap()
/// );
/// field[(8, 7)] = Tile::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut columniter = field.column(8); // Create an iterator over the nineth column
/// let mine_tile = columniter.nth(7) // Find the seventh element in the column
///     .unwrap(); // Get rid of the Option wrap
/// assert_eq!(mine_tile, Tile::Mine(Flag::NotFlagged)); // It's a mine
/// ```
#[derive(Clone)]
pub struct ColumnIter<'f> {
    field: &'f Field,
    column: usize,
    index: Range<usize>
}
impl<'f> ColumnIter<'f> {
    /// Creates an iterator over the specified column of the specified field.
    #[inline(always)]
    pub fn new(field: &'f Field, column: usize) -> Self {
        Self {field, column, index: 0..field.dimensions().0.get()}
    }
    /// Returns the tile at the specified row, or `None` if such a row doesn't exist. The column for which the iterator was created is used.
    #[inline(always)]
    pub fn get(&self, row: usize) -> Option<Tile> {
        self.field.get((self.column, row)).copied()
    }
    /// Returns the tile at the specified row.
    ///
    /// Used as a convenience function, allowing you to write `field.column(x).row(y)` to find specific tiles.
    #[inline(always)]
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn row(&self, row: usize) -> Tile {
        self.get(row).expect("index out of bounds")
    }
    /// Returns the field which the iterator iterates over.
    #[inline(always)]
    pub fn field(&self) -> &'f Field {
        self.field
    }
}
impl<'f> Iterator for ColumnIter<'f> {
    type Item = Tile;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        let el = self.field.get((self.column, self.index.start));
        self.index.start += 1;
        el.copied()
    }
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}
impl<'f> DoubleEndedIterator for ColumnIter<'f> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        self.field.get((self.column, self.index.end)).copied()
    }
}
impl<'f> ExactSizeIterator for ColumnIter<'f> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl FusedIterator for ColumnIter<'_> {}
impl Index<usize> for ColumnIter<'_> {
    type Output = Tile;
    /// Returns the tile at the specified row.
    ///
    /// Used as a convenience function, allowing you to write `field.column(x)[y]` to find specific tiles.
    ///
    /// This differs from [`.row()`][0] in the return type: this one returns a reference while `.row()` returns the tile by-value.
    ///
    /// [0]: #method.row.html "row — returns the tile at the specified row"
    #[inline(always)]
    fn index(&self, row: usize) -> &Tile {
        self.field.get((self.column, row)).expect("index out of bounds")
    }
}

/// An iterator over the rows of a field.
///
/// # Usage
/// ```
/// # use sweeper::{Field, Tile, Flag, FieldRowsIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::empty( // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(4).unwrap()
/// );
/// field[(8, 3)] = Tile::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut row_with_mine: Option<usize> = None; // Keep track of our findings using an Option
/// for (y, mut row) in field.rows().enumerate() { // In each row...
///     if row.find(|t| t.is_mine()).is_some() { // If the row contains a mine...
///         row_with_mine = Some(y); //...take the row number out of the loop.
///     }
/// }
/// assert_eq!(row_with_mine, Some(3)); // We indeed have found a mine in the 4th row.
/// ```
#[derive(Clone)]
pub struct FieldRowsIter<'f> {
    field: &'f Field,
    index: Range<usize>
}
impl<'f> FieldRowsIter<'f> {
    /// Returns an iterator over the specified field's rows.
    #[inline(always)]
    pub fn new(field: &'f Field) -> Self {
        Self {
            field, index: 0..field.dimensions().1.get()
        }
    }
}
impl<'f> Iterator for FieldRowsIter<'f> {
    type Item = RowIter<'f>;
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
impl<'f> DoubleEndedIterator for FieldRowsIter<'f> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        Some(self.field.row(self.index.end))
    }
}
impl<'f> ExactSizeIterator for FieldRowsIter<'f> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl FusedIterator for FieldRowsIter<'_> {}

/// An iterator over the columns of a field.
///
///  # Usage
/// ```
/// # use sweeper::{Field, Tile, Flag, FieldColumnsIter};
/// # use core::num::NonZeroUsize;
/// #
/// let mut field = Field::empty( // Create a field to work with
///     NonZeroUsize::new(9).unwrap(),
///     NonZeroUsize::new(8).unwrap()
/// );
/// field[(8, 7)] = Tile::Mine(Flag::NotFlagged); // Place a mine (remember that indicies start from 0)
/// let mut column_with_mine: Option<usize> = None; // Keep track of our findings using an Option
/// for (x, mut column) in field.columns().enumerate() { // In each column...
///     if column.find(|t| t.is_mine()).is_some() { // If the column contains a mine...
///         column_with_mine = Some(x); //...take the column number out of the loop.
///     }
/// }
/// assert_eq!(column_with_mine, Some(8)); // We indeed have found a mine in the 9th column.
/// ```
#[derive(Clone)]
pub struct FieldColumnsIter<'f> {
    field: &'f Field,
    index: Range<usize>
}
impl<'f> FieldColumnsIter<'f> {
    /// Returns an iterator over the specified field's columns.
    #[inline(always)]
    pub fn new(field: &'f Field) -> Self {
        Self {
            field, index: 0..field.dimensions().0.get()
        }
    }
}
impl<'f> Iterator for FieldColumnsIter<'f> {
    type Item = ColumnIter<'f>;
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
impl<'f> DoubleEndedIterator for FieldColumnsIter<'f> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len() == 0 {
            return None;
        }
        self.index.end -= 1;
        Some(self.field.column(self.index.end))
    }
}
impl<'f> ExactSizeIterator for FieldColumnsIter<'f> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index.end - self.index.start
    }
}
impl FusedIterator for FieldColumnsIter<'_> {}