use core::{
    num::{NonZeroUsize, NonZeroU8}
};
use super::{
    Field
};

/// A Minesweeper tile.
#[derive(Copy, Clone, Debug)]
pub enum Tile {
    /// A tile which is empty but hasn't been opened yet.
    ClosedEmpty(Flag),
    /// A tile which has been opened and doesn't have neighboring mines.
    OpenEmpty,
    /// A tile which has been opened and has neighboring mines.
    OpenNumber(NonZeroU8),
    /// A tile which has a mine inside, and whether it's marked or not.
    Mine(Flag)
}
impl Tile {
    /// Returns `true` if the tile is closed, `false` otherwise.
    #[inline]
    pub fn is_closed(self) -> bool {
        match self {
            Self::ClosedEmpty(_)
          | Self::Mine(_) => true,
            _ => false
        }
    }
    /// Returns `true` if the tile is open, `false` otherwise.
    #[inline]
    pub fn is_open(self) -> bool {
        match self {
              Self::OpenEmpty
            | Self::OpenNumber(_) => true,
            _ => false
        }
    }
    /// Returns `true` if the tile contains a mine, `false` otherwise.
    #[inline]
    pub fn is_mine(self) -> bool {
        match self {
            Self::Mine(_) => true,
            _ => false
        }
    }
    /// Returns `true` if clicking this tile does not end the game, `false` otherwise.
    #[inline(always)]
    pub fn is_safe(self) -> bool {
        !self.is_mine()
    }
    /// Returns `true` if this tile has to be clicked in order for the game to successfully end, `false` otherwise.
    ///
    /// This is `false` for open mines — returns `true` only for `ClosedEmpty`.
    #[inline]
    pub fn is_required_to_open(self) -> bool {
        match self {
            Self::ClosedEmpty(_) => true,
            _ => false
        }
    }
    /// Returns a [`ClickOutcome`][co] from the data known only to this specific tile, or `None` if returning one requires access to the field.
    ///
    /// [co]: enum.ClickOutcome.html "ClickOutcome — the event produced after clicking a tile"
    #[inline]
    pub fn peek_local(self) -> Option<ClickOutcome> {
        match self {
            Self::ClosedEmpty(_) => None,
            Self::OpenEmpty => Some(ClickOutcome::OpenClearing),
            Self::OpenNumber(_) => Some(ClickOutcome::Chord),
            Self::Mine(_) => Some(ClickOutcome::Explosion)
        }
    }
}
impl Default for Tile {
    #[inline(always)]
    /// Returns the `ClosedEmpty` variant.
    fn default() -> Self {
        Self::ClosedEmpty(Flag::default())
    }
}
impl PartialEq<Tile> for Tile {
    /// Compares two tiles.
    ///
    /// Two tiles are equal if they're both empty or they both contain a mine. Other factors, like the presence of a flag or amount of surrounding mines are not
    /// compared.
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Mine(_) => {
                if let Self::Mine(_) = other {
                    true
                }
                else {false}
            },
            _ => {
                match other {
                    Self::Mine(_) => false,
                    _ => true
                }
            },
        }
    }
}
impl Eq for Tile {}

/// The implementation for the clearing traversal algorithm which works for both mutable usage and immutable usage.
macro_rules! for_every_tile {
    ($field:expr, $anchor_location:expr, $f:expr, $include_shore:expr) => {
        // We're using this specific type as the type of a frame on the stack. It consists of two tuples:
        // - The location at which the "painter" is currently located.
        // - The state of the tile on the up, down, left and right directions.
        //   True means we'd like to look there.
        //   False means there's nothing of interest there, meaning that we've either looked there or there's a mine or a tile with a number.
        type StackFrame = ((usize, usize), (bool, bool, bool, bool));
        // We're using a heap-based stack imposter instead of the thread stack to avoid
        // overflowing. For large clearings, this will cause minor lag instead of
        // crashing. For smaller ones, this will hardly make a difference at all, since
        // we're preallocating it for a recursion depth of 10.
        let mut stack = Vec::<StackFrame>::with_capacity(10);
        let mut stack_top // Start at the anchor location.
            = ($anchor_location, (true, true, true, true));
        stack.push(stack_top);
        $f($field, stack_top.0); // Invoke the first run.
        loop { // While we haven't emptied the stack...
            let chosen_location
               = if stack_top .1 .0 {0} // Up,
            else if stack_top .1 .1 {1} // down,
            else if stack_top .1 .2 {2} // left,
            else if stack_top .1 .3 {3} // right.
            // If we have nowhere to go, return to where we came from.
            else if let Some(new_top) = stack.pop() {
                stack_top = new_top;
                continue;
            // If we have nowhere to return, we can stop!
            } else {break};

            let location_to_peek // Now find the coordinates which we're about to peek.
             =    if chosen_location == 0 {(stack_top .0 .0, stack_top .0 .1 + 1)}
             else if chosen_location == 1 {(stack_top .0 .0, stack_top .0 .1 - 1)}
             else if chosen_location == 2 {(stack_top .0 .0 - 1, stack_top .0 .1)}
             else if chosen_location == 3 {(stack_top .0 .0 + 1, stack_top .0 .1)}
             else {unreachable!()};

            if let Some(outcome) = $field.peek(location_to_peek) {
                match outcome {
                    ClickOutcome::OpenClearing
                    | ClickOutcome::Nothing => {
                        // We found more clear land!
                        // First of all, let's push the current state so that we can return to it later.
                        stack.push(stack_top);
                        // Then we'll set up the stack top for the next iteration.
                        stack_top.0 = location_to_peek;
                        stack_top.1 = (true, true, true, true);
                        $f($field, stack_top.0); // Run the closure, this is the point of our actions here.
                    },
                    ClickOutcome::Chord
                  | ClickOutcome::Explosion => {}
                    ClickOutcome::OpenNumber(_) => {
                        if $include_shore {
                            stack.push(stack_top);
                            stack_top.0 = location_to_peek;
                            stack_top.1 = (true, true, true, true);
                            $f($field, stack_top.0);
                        }
                    }
                }
            }
            match chosen_location {
                0 => stack_top .1 .0 = false,
                1 => stack_top .1 .1 = false,
                2 => stack_top .1 .2 = false,
                3 => stack_top .1 .3 = false,
                _ => unreachable!(),
            };
        }
    };
}
/// A clearing on the specified field.
///
/// This is merely a reference to the area on a field which is known to be a clearing. Nothing is owned by this structure.
#[derive(Copy, Clone)]
pub struct Clearing<'f> {
    field: &'f Field,
    anchor_location: (usize, usize)
}
impl<'f> Clearing<'f> {
    /// Returns a `Clearing` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn new(field: &'f Field, anchor_location: (usize, usize)) -> Option<Self> {
        if field.get(anchor_location).is_some() {
            if field.count_neighboring_mines(anchor_location) > 0 {
                None
            } else {
                Some(Self {
                    field, anchor_location
                })
            }
        } else {None}
    }
    /// Returns the field on which this clearing is located.
    #[inline(always)]
    pub fn field(self) -> &'f Field { self.field }
    /// Returns the location around which this clearing is formed.
    ///
    /// This can be any location inside the clearing. More specifically, the one used during creation is returned.
    #[inline(always)]
    pub fn anchor_location(self) -> (usize, usize) { self.anchor_location }

    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes a reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    #[cfg_attr(features = "track_caller", track_caller)]
    pub fn for_every_tile<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&'f Field, (usize, usize)) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Returns the size of the clearing, in tiles. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(features = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn size(self, include_shore: bool) -> NonZeroUsize {
        let mut counter = 0_usize;
        self.for_every_tile(include_shore, |_, _| counter += 1);
        NonZeroUsize::new(counter)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
    }
    /// Returns `true` if the given tile is inside the clearing, `false` otherwise. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(features = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn includes(self, index: (usize, usize), include_shore: bool) -> bool {
        let mut includes = false;
        self.for_every_tile(include_shore, |_, here| if here == index {includes = true});
        includes
    }
}
/// A **mutable** reference to a clearing on the specified field.
///
/// This is merely a **mutable** reference to the area on a field which is known to be clear land. Nothing is owned by this structure.
pub struct ClearingMut<'f> {
    field: &'f mut Field,
    anchor_location: (usize, usize)
}
impl<'f> ClearingMut<'f> {
    /// Returns a `ClearingMut` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn new(field: &'f mut Field, anchor_location: (usize, usize)) -> Option<Self> {
        if field.get(anchor_location).is_some() {
            if field.count_neighboring_mines(anchor_location) > 0 {
                None
            } else {
                Some(Self {
                    field, anchor_location
                })
            }
        } else {None}
    }
    /// Returns the field on which this clearing is located.
    #[inline(always)]
    pub fn field(self) -> &'f Field { self.field }
    /// Returns the location around which this clearing is formed.
    ///
    /// This can be any location inside the clearing. More specifically, the one used during creation is returned.
    #[inline(always)]
    pub fn anchor_location(self) -> (usize, usize) { self.anchor_location }

    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes an **immutable** reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    ///
    /// This is a version of `for_every_tile_mut` which doesn't allow mutating the field.
    #[cfg_attr(features = "track_caller", track_caller)]
    pub fn for_every_tile<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&'f Field, (usize, usize)) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes a **mutable** reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    #[cfg_attr(features = "track_caller", track_caller)]
    pub fn for_every_tile_mut<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&mut Field, (usize, usize)) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Returns the size of the clearing, in tiles. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// Use [`open`][opn] instead if you want to open the clearing afterwards, since it provides the size itself.
    ///
    /// [opn]: #method.open.html "open — fully opens the clearing on the field and returns the amount of tiles cleared"
    #[cfg_attr(features = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn size(self, include_shore: bool) -> NonZeroUsize {
        let mut counter = 0_usize;
        self.for_every_tile(include_shore, |_, _| counter += 1);
        NonZeroUsize::new(counter)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
    }
    /// Returns `true` if the given tile is inside the clearing, `false` otherwise. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(features = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn includes(self, index: (usize, usize), include_shore: bool) -> bool {
        let mut includes = false;
        self.for_every_tile(include_shore, |_, here| if here == index {includes = true});
        includes
    }
    /// Fully opens the clearing on the field and returns the amount of tiles opened. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The first number is the amount of tiles which were opened, and the second one is the total size of the clearing, which includes both previously closed and previously open tiles.
    pub fn open(self, include_shore: bool) -> (usize, NonZeroUsize) {
        let [mut opened_size, mut total_size] = [0_usize; 2];

        self.for_every_tile_mut(include_shore, |field, location| {
            total_size += 1;
            if let Tile::ClosedEmpty(_) = field[location] {
                field[location] = Tile::OpenEmpty;
                opened_size += 1;
            }
        });

        (opened_size, NonZeroUsize::new(total_size)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
        )
    }
}
impl<'f> From<ClearingMut<'f>> for Clearing<'f> {
    fn from(op: ClearingMut<'f>) -> Self {
        Self {field: op.field, anchor_location: op.anchor_location}
    }
}

/// Represents the state of a flag
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Flag {
    /// The player is absolutely sure that the tile this flag is applied to contains a mine.
    Flagged,
    /// The player knows about a possible mine hiding here, but lacks enough evidence to be able to prove that there's indeed a mine.
    QuestionMark,
    /// The player didn't mark this tile yet.
    ///
    /// Returned by the `Default` trait implementation.
    NotFlagged,
}
impl Default for Flag {
    /// Returns the `NotFlagged` state.
    #[inline(always)]
    fn default() -> Self {
        Self::NotFlagged
    }
}

/// The event produced after clicking a tile.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClickOutcome {
    /// Nothing happens.
    ///
    /// Produced when an empty tile without a number is clicked.
    Nothing,
    /// A clearing is opened. A clearing of only one empty numberless tile is still a clearing.
    ///
    /// **Cannot be obtained from a single `Tile`**, since it requires knowing about surrounding tiles.
    OpenClearing,
    /// An empty tile with a number is opened.
    ///
    /// **Cannot be obtained from a single `Tile`**, since it requires knowing about surrounding tiles.
    OpenNumber(NonZeroU8),
    /// A chord operation is invoked.
    ///
    /// Obtained from an `OpenNumber` tile.
    Chord,
    /// An explosion is triggered, ending the game.
    ///
    /// Obtained from a `Mine`.
    Explosion
}