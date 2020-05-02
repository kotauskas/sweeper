use core::{
    num::{NonZeroUsize, NonZeroU8},
};
#[cfg(feature = "serialization")]
use core::{
    fmt::{self, Formatter},
    marker::PhantomData,
};
#[cfg(feature = "serialization")]
use serde::{
    ser::{Serializer, SerializeStruct, SerializeTupleVariant},
    de::{self, Deserializer, Visitor, SeqAccess, MapAccess, EnumAccess, VariantAccess},
    Serialize, Deserialize,
};
use alloc::{
    vec::Vec
};
use super::{
    Field, FieldCoordinates,
};

/// A tile on a Minesweeper field.
///
/// This groups the state of the tile and the custom payload.
pub struct Tile<Ct, Cf> {
    /// The state of the tile. See the `TileState` enum for an explanation of what exactly does this field store.
    pub state: TileState<Cf>,
    /// The custom payload. Typically `()` for implementations which use a low-level rendering library, like SDL2, or the entity type for implementations which use an entity-component-system architecture.
    pub payload: Ct
}
impl<Ct: Default, Cf> From<TileState<Cf>> for Tile<Ct, Cf> {
    fn from(state: TileState<Cf>) -> Self {
        Self { state, payload: Ct::default() }
    }
}
impl<Ct: Default, Cf> Default for Tile<Ct, Cf> {
    /// Returns the default value both the payload and the state.
    fn default() -> Self {
        Self {
            state: TileState::default(),
            payload: Ct::default()
        }
    }
}
#[cfg(feature = "serialization")]
impl<Ct, Cf> Serialize for Tile<Ct, Cf>
where Ct: Serialize,
      Cf: Serialize {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut s = s.serialize_struct("Tile", 2)?;
        s.serialize_field("state", &self.state)?;
        s.serialize_field("payload", &self.payload)?;
        s.end()
    }
}
#[cfg(feature = "serialization")]
impl<'de, Ct, Cf> Deserialize<'de> for Tile<Ct, Cf>
where Ct: Deserialize<'de>,
      Cf: Deserialize<'de> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        const FIELDS: &[&str] = &["state", "payload"];
        enum StructField { State, Payload }
        impl<'de> Deserialize<'de> for StructField {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct StructFieldVisitor;

                impl<'de> Visitor<'de> for StructFieldVisitor {
                    type Value = StructField;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("`state` or `payload`")
                    }

                    fn visit_str<E: de::Error>(self, value: &str) -> Result<StructField, E> {
                        match value {
                            "state" => Ok(StructField::State),
                            "payload" => Ok(StructField::Payload),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(StructFieldVisitor)
            }
        }

        struct TileVisitor<Ct, Cf>(PhantomData<(Ct, Cf)>);
        impl<'de, Ct, Cf> Visitor<'de> for TileVisitor<Ct, Cf>
        where Ct: Deserialize<'de>,
              Cf: Deserialize<'de> {
            type Value = Tile<Ct, Cf>;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("struct Tile")
            }
            fn visit_seq<V: SeqAccess<'de>>(self, mut seq: V) -> Result<Self::Value, V::Error> {
                let state = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let payload = seq.next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Tile {state, payload})
            }
            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Self::Value, V::Error> {
                let mut state: Option<TileState<Cf>> = None;
                let mut payload: Option<Ct> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        StructField::State => {
                            if state.is_some() {
                                return Err(de::Error::duplicate_field("state"));
                            }
                            state = Some(map.next_value()?);
                        }
                        StructField::Payload => {
                            if payload.is_some() {
                                return Err(de::Error::duplicate_field("payload"));
                            }
                            payload = Some(map.next_value()?);
                        }
                    }
                }
                let state = state.ok_or_else(|| de::Error::missing_field("state"))?;
                let payload = payload.ok_or_else(|| de::Error::missing_field("payload"))?;
                Ok(Tile {state, payload})
            }
        }
        d.deserialize_struct("Tile", FIELDS, TileVisitor(PhantomData))
    }
}

/// The state of a tile.
#[derive(Copy, Clone, Debug)]
pub enum TileState<Cf> {
    /// A tile which is empty but hasn't been opened yet.
    ClosedEmpty(Flag<Cf>),
    /// A tile which has been opened and doesn't have neighboring mines.
    OpenEmpty,
    /// A tile which has been opened and has neighboring mines.
    OpenNumber(NonZeroU8),
    /// A tile which has a mine inside, and whether it's marked or not.
    Mine(Flag<Cf>)
}
impl<Cf> TileState<Cf> {
    /// Returns `true` if the tile is closed, `false` otherwise.
    #[inline]
    pub fn is_closed(&self) -> bool {
        match self {
            Self::ClosedEmpty(_)
          | Self::Mine(_) => true,
            _ => false
        }
    }
    /// Returns `true` if the tile is open, `false` otherwise.
    #[inline]
    pub fn is_open(&self) -> bool {
        match self {
              Self::OpenEmpty
            | Self::OpenNumber(_) => true,
            _ => false
        }
    }
    /// Returns `true` if the tile contains a mine, `false` otherwise.
    #[inline]
    pub fn is_mine(&self) -> bool {
        match self {
            Self::Mine(_) => true,
            _ => false
        }
    }
    /// Returns `true` if clicking this tile does not end the game, `false` otherwise.
    #[inline(always)]
    pub fn is_safe(&self) -> bool {
        !self.is_mine()
    }
    /// Returns `true` if this tile has to be clicked in order for the game to successfully end, `false` otherwise.
    ///
    /// This is `false` for open mines — returns `true` only for `ClosedEmpty`.
    #[inline]
    pub fn is_required_to_open(&self) -> bool {
        match self {
            Self::ClosedEmpty(_) => true,
            _ => false
        }
    }
    /// Returns the type of flag installed on this tile, or `None` if this tile is open and thus cannot hold a flag.
    #[inline]
    pub fn flag_state(&self) -> Option<&Flag<Cf>> {
        match self {
            Self::ClosedEmpty(flag) => Some(flag),
            Self::Mine(flag) => Some(flag),
            _ => None
        }
    }
    /// Returns `true` if the `flag_state` is `Some(Flag::Flagged)`, `false` otherwise.
    #[inline]
    pub fn is_flagged(&self) -> bool {
        if let Some(flag) = self.flag_state() {
            if let Flag::Flagged = flag { true }
            else { false }
        } else { false }
    }
    /// Returns a [`ClickOutcome`][co] from the data known only to this specific tile, or `None` if returning one requires access to the field.
    ///
    /// [co]: enum.ClickOutcome.html "ClickOutcome — the event produced after clicking a tile"
    #[inline]
    pub fn peek_local(&self) -> Option<ClickOutcome> {
        match self {
            Self::ClosedEmpty(_) => None,
            Self::OpenEmpty => Some(ClickOutcome::OpenClearing),
            Self::OpenNumber(_) => Some(ClickOutcome::Chord),
            Self::Mine(_) => Some(ClickOutcome::Explosion)
        }
    }
}
impl<Cf> Default for TileState<Cf> {
    #[inline(always)]
    /// Returns the `ClosedEmpty` variant.
    fn default() -> Self {
        Self::ClosedEmpty(Flag::default())
    }
}
impl<Cf> PartialEq<TileState<Cf>> for TileState<Cf> {
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
impl<Cf> Eq for TileState<Cf> {}

#[cfg(feature = "serialization")]
impl<Cf: Serialize> Serialize for TileState<Cf> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ClosedEmpty(flag) => {
                let mut s = s.serialize_tuple_variant("TileState", 0, "ClosedEmpty", 1)?;
                s.serialize_field(flag)?;
                s.end()
            },
            Self::OpenEmpty => {
                s.serialize_unit_variant("TileState", 1, "OpenEmpty")
            },
            Self::OpenNumber(mines) => {
                let mut s = s.serialize_tuple_variant("TileState", 2, "OpenNumber", 1)?;
                s.serialize_field(mines)?;
                s.end()
            },
            Self::Mine(flag) => {
                let mut s = s.serialize_tuple_variant("TileState", 3, "Mine", 1)?;
                s.serialize_field(flag)?;
                s.end()
            },
        }
    }
}
#[cfg(feature = "serialization")]
impl<'de, Cf: Deserialize<'de>> Deserialize<'de> for TileState<Cf> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        const VARIANTS: &[&str] = &["ClosedEmpty", "OpenEmpty", "OpenNumber", "Mine"];
        #[derive(Deserialize)]
        #[repr(u8)]
        enum Tag {
            ClosedEmpty, OpenEmpty, OpenNumber, Mine
        }

        struct TsVisitor<Cf>(PhantomData<Cf>);
        impl<'de, Cf: Deserialize<'de>> Visitor<'de> for TsVisitor<Cf> {
            type Value = TileState<Cf>;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, // Inform that we need an enumeration storead according to the format.
                    "enum TileState<Cf>")
            }
            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
                // Unpack the enum value.
                let (tag, variant_data) = data.variant::<Tag>()?;
                match tag {
                    Tag::ClosedEmpty => {
                        let flag = variant_data.tuple_variant(1, FlagVisitor(PhantomData))?;
                        Ok(TileState::ClosedEmpty(flag))
                    },
                    Tag::OpenEmpty => {
                        variant_data.unit_variant()?;
                        Ok(TileState::OpenEmpty)
                    },
                    Tag::OpenNumber => {
                        let num = variant_data.tuple_variant(1, Nzu8Visitor)?;
                        Ok(TileState::OpenNumber(num))
                    },
                    Tag::Mine => {
                        let flag = variant_data.tuple_variant(1, FlagVisitor(PhantomData))?;
                        Ok(TileState::Mine(flag))
                    }
                }
            }
        }
        struct Nzu8Visitor;
        impl<'de> Visitor<'de> for Nzu8Visitor {
            type Value = NonZeroU8;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                write!(f,
                    "a non-zero u8")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                NonZeroU8::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = None;
                while let Some(nzu8) = seq.next_element()? {
                    if result.is_some() {
                        return Err(serde::de::Error::duplicate_field("nearby mine count"));
                    }
                    result = Some(nzu8);
                };
                if let Some(nzu8) = result {
                    Ok(nzu8)
                } else {
                    Err(serde::de::Error::missing_field("nearby mine count"))
                }
            }
        }
        struct FlagVisitor<Cf>(PhantomData<Cf>);
        impl<'de, Cf: Deserialize<'de>> Visitor<'de> for FlagVisitor<Cf> {
            type Value = Flag<Cf>;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                write!(f,
                    "enum Flag<Cf>")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                Flag::<Cf>::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = None;
                while let Some(flag) = seq.next_element()? {
                    if result.is_some() {
                        return Err(serde::de::Error::duplicate_field("flag"));
                    }
                    result = Some(flag);
                };
                if let Some(flag) = result {
                    Ok(flag)
                } else {
                    Err(serde::de::Error::missing_field("flag"))
                }
            }
        }
        d.deserialize_enum("TileState", VARIANTS, TsVisitor(PhantomData))
    }
}

/// The implementation for the clearing traversal algorithm which works for both mutable usage and immutable usage.
macro_rules! for_every_tile {
    ($field:expr, $anchor_location:expr, $f:expr, $include_shore:expr) => {
        // We're using this specific type as the type of a frame on the stack. It consists of two tuples:
        // - The location at which the "painter" is currently located.
        // - The state of the tile on the up, down, left and right directions.
        //   True means we'd like to look there.
        //   False means there's nothing of interest there, meaning that we've either looked there or there's a mine or a tile with a number.
        type StackFrame = (FieldCoordinates, [bool; 4]);
        // We're using a heap-based stack imposter instead of the thread stack to avoid
        // overflowing. For large clearings, this will cause minor lag instead of
        // crashing. For smaller ones, this will hardly make a difference at all, since
        // we're preallocating it for a recursion depth of 10.
        let mut stack = Vec::<StackFrame>::with_capacity(10);
        let mut stack_top // Start at the anchor location.
            = ($anchor_location, [true; 4]);
        $f($field, stack_top.0); // Invoke the first run.
        loop { // While we haven't emptied the stack...
            let chosen_location
               = if stack_top.1[0] {stack_top.1[0] = false; 0} // Up,
            else if stack_top.1[1] {stack_top.1[1] = false; 1} // down,
            else if stack_top.1[2] {stack_top.1[2] = false; 2} // left,
            else if stack_top.1[3] {stack_top.1[3] = false; 3} // right.
            // If we have nowhere to go, return to where we came from.
            else if let Some(new_top) = stack.pop() {
                stack_top = new_top;
                continue;
            // If we have nowhere to return, we can stop!
            } else {break};

            let location_to_peek // Now find the coordinates which we're about to peek.
             =    if chosen_location == 0 {[stack_top.0[0], stack_top.0[1] + 1]}
             else if chosen_location == 1 {[stack_top.0[0], stack_top.0[1] - 1]}
             else if chosen_location == 2 {[stack_top.0[0] - 1, stack_top.0[1]]}
             else if chosen_location == 3 {[stack_top.0[0] + 1, stack_top.0[1]]}
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
                        stack_top.1 = [true; 4];
                        $f($field, stack_top.0); // Run the closure, this is the point of our actions here.
                    },
                    ClickOutcome::Chord
                  | ClickOutcome::Explosion => {}
                    ClickOutcome::OpenNumber(_) => {
                        if $include_shore {
                            stack.push(stack_top);
                            stack_top.0 = location_to_peek;
                            stack_top.1 = [true; 4];
                            $f($field, stack_top.0);
                        }
                    }
                }
            }
        }
    };
}
/// A clearing on the specified field.
///
/// This is merely a reference to the area on a field which is known to be a clearing. Nothing is owned by this structure.
#[derive(Copy, Clone)]
pub struct Clearing<'f, Ct: 'static, Cf: 'static> {
    field: &'f Field<Ct, Cf>,
    anchor_location: FieldCoordinates
}
impl<'f, Ct: 'static, Cf: 'static> Clearing<'f, Ct, Cf> {
    /// Returns a `Clearing` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn new(field: &'f Field<Ct, Cf>, anchor_location: FieldCoordinates) -> Option<Self> {
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
    pub fn field(self) -> &'f Field<Ct, Cf> { self.field }
    /// Returns the location around which this clearing is formed.
    ///
    /// This can be any location inside the clearing. More specifically, the one used during creation is returned.
    #[inline(always)]
    pub fn anchor_location(self) -> FieldCoordinates { self.anchor_location }

    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes a reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn for_every_tile<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&'f Field<Ct, Cf>, FieldCoordinates) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Returns the size of the clearing, in tiles. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(feature = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn size(self, include_shore: bool) -> NonZeroUsize {
        let mut counter = 0_usize;
        self.for_every_tile(include_shore, |_, _| counter += 1);
        NonZeroUsize::new(counter)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
    }
    /// Returns `true` if the given tile is inside the clearing, `false` otherwise. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(feature = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn includes(self, coordinates: FieldCoordinates, include_shore: bool) -> bool {
        let mut includes = false;
        self.for_every_tile(include_shore, |_, here| if here == coordinates {includes = true});
        includes
    }
}
/// A **mutable** reference to a clearing on the specified field.
///
/// This is merely a **mutable** reference to the area on a field which is known to be clear land. Nothing is owned by this structure.
pub struct ClearingMut<'f, Ct: 'static, Cf: 'static> {
    field: &'f mut Field<Ct, Cf>,
    anchor_location: FieldCoordinates
}
impl<'f, Ct: 'static, Cf: 'static> ClearingMut<'f, Ct, Cf> {
    /// Returns a `ClearingMut` on the specified `Field`, or `None` if the location has 1 or more neighboring mines or is out of bounds.
    pub fn new(field: &'f mut Field<Ct, Cf>, anchor_location: FieldCoordinates) -> Option<Self> {
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
    pub fn field(self) -> &'f Field<Ct, Cf> { self.field }
    /// Returns the location around which this clearing is formed.
    ///
    /// This can be any location inside the clearing. More specifically, the one used during creation is returned.
    #[inline(always)]
    pub fn anchor_location(self) -> FieldCoordinates { self.anchor_location }

    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes an **immutable** reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    ///
    /// This is a version of `for_every_tile_mut` which doesn't allow mutating the field.
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn for_every_tile<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&'f Field<Ct, Cf>, FieldCoordinates) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Executes the specified closure on every tile inside the clearing. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The closure takes a **mutable** reference to the field as the first argument and the location of the tile as the second one. No return value is expected.
    #[cfg_attr(feature = "track_caller", track_caller)]
    pub fn for_every_tile_mut<F>(self, include_shore: bool, mut f: F)
    where F: FnMut(&mut Field<Ct, Cf>, FieldCoordinates) {
        for_every_tile!(self.field, self.anchor_location, f, include_shore);
    }
    /// Returns the size of the clearing, in tiles. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// Use [`open`][opn] instead if you want to open the clearing afterwards, since it provides the size itself.
    ///
    /// [opn]: #method.open.html "open — fully opens the clearing on the field and returns the amount of tiles cleared"
    #[cfg_attr(feature = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn size(self, include_shore: bool) -> NonZeroUsize {
        let mut counter = 0_usize;
        self.for_every_tile(include_shore, |_: &Field<Ct, Cf>, _| counter += 1);
        NonZeroUsize::new(counter)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
    }
    /// Returns `true` if the given tile is inside the clearing, `false` otherwise. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    #[cfg_attr(feature = "track_caller", track_caller)]
    #[must_use = "fully traversing a clearing is an expensive operation involving memory allocation"]
    pub fn includes(self, coordinates: FieldCoordinates, include_shore: bool) -> bool {
        let mut includes = false;
        self.for_every_tile(include_shore, |_, here| if here == coordinates {includes = true});
        includes
    }
    /// Fully opens the clearing on the field and returns the amount of tiles opened. Optionally can include the "shore" (tiles with numbers) as a part of the clearing.
    ///
    /// The first number is the amount of tiles which were opened, and the second one is the total size of the clearing, which includes both previously closed and previously open tiles.
    pub fn open(self, include_shore: bool) -> (usize, NonZeroUsize) {
        let [mut opened_size, mut total_size] = [0_usize; 2];

        self.for_every_tile_mut(include_shore, |field, location| {
            total_size += 1;
            if let TileState::ClosedEmpty(_) = field[location].state {
                field[location].state = TileState::OpenEmpty;
                opened_size += 1;
            }
        });

        (opened_size, NonZeroUsize::new(total_size)
            .expect("unexpected zero clearing size (nonzero clearing size is a safety guarantee)")
        )
    }
}
impl<'f, Ct: 'static, Cf: 'static> From<ClearingMut<'f, Ct, Cf>> for Clearing<'f, Ct, Cf> {
    fn from(op: ClearingMut<'f, Ct, Cf>) -> Self {
        Self {field: op.field, anchor_location: op.anchor_location}
    }
}

/// Represents the state of a flag on a closed tile.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Flag<Cf> {
    /// The player didn't mark this tile yet.
    ///
    /// Returned by the `Default` trait implementation.
    NotFlagged,
    /// The player is absolutely sure that the tile this flag is applied to contains a mine.
    Flagged,
    /// A nonstandard flag type, as specified by the generic argument.
    Custom(Cf),
}
impl<Cf> Default for Flag<Cf> {
    /// Returns the `NotFlagged` state.
    #[inline(always)]
    fn default() -> Self {
        Self::NotFlagged
    }
}
impl<Cf> From<Cf> for Flag<Cf> {
    /// Returns the custom flag state with the specified value.
    #[inline(always)]
    fn from(op: Cf) -> Self {
        Self::Custom(op)
    }
}
#[cfg(feature = "serialization")]
impl<Cf: Serialize> Serialize for Flag<Cf> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let (variant, variant_index, len) = match self {
            Self::NotFlagged => ("NotFlagged", 0, 0),
            Self::Flagged    => ("Flagged", 1, 0),
            Self::Custom(_)  => ("Custom", 2, 1),
        };
        if let Self::Custom(cf) = self {
            let mut s = s.serialize_tuple_variant("Flag", variant_index, variant, len)?;
            s.serialize_field(cf)?;
            s.end()
        } else {
            s.serialize_unit_variant("Flag", variant_index, variant)
        }
    }
}
#[cfg(feature = "serialization")]
impl<'de, Cf: Deserialize<'de>> Deserialize<'de> for Flag<Cf> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        const VARIANTS: &[&str] = &["NotFlagged", "Flagged", "Custom"];
        #[derive(Deserialize)]
        #[repr(u8)]
        // This is basically the same enum but without the value for the Custom state. Serde already knows how to deserialize it.
        enum Tag {
            NotFlagged, Flagged, Custom
        }

        struct FlagVisitor<Cf>(PhantomData<Cf>);
        impl<'de, Cf: Deserialize<'de>> Visitor<'de> for FlagVisitor<Cf> {
            type Value = Flag<Cf>;
            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, // Inform that we need an enumeration storead according to the format.
                    "enum Flag<Cf>")
            }
            fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, A::Error> {
                // Unpack the enum value.
                let (tag, variant_data) = data.variant::<Tag>()?;
                match tag {
                    Tag::NotFlagged => {
                        variant_data.unit_variant()?;
                        Ok(Flag::NotFlagged)
                    },
                    Tag::Flagged => {
                        variant_data.unit_variant()?;
                        Ok(Flag::Flagged)
                    },
                    Tag::Custom => {
                        let cf = variant_data.tuple_variant(1, CfVisitor::<Cf>(PhantomData))?;
                        Ok(Flag::Custom(cf))
                    }
                }
            }
        }
        struct CfVisitor<Cf>(PhantomData<Cf>);
        impl<'de, Cf: Deserialize<'de>> Visitor<'de> for CfVisitor<Cf> {
            type Value = Cf;

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
                write!(f,
                    "a tuple enum variant with the custom flag as the only field")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                Cf::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = None;
                while let Some(cf) = seq.next_element()? {
                    if result.is_some() {
                        return Err(serde::de::Error::duplicate_field("custom flag"));
                    }
                    result = Some(cf);
                };
                if let Some(cf) = result {
                    Ok(cf)
                } else {
                    Err(serde::de::Error::missing_field("custom flag"))
                }
            }
        }
        d.deserialize_enum("Flag", VARIANTS, FlagVisitor::<Cf>(PhantomData))
    }
}

/// The event produced after clicking a tile.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
impl Default for ClickOutcome {
    /// Returns the `Nothing` variant.
    #[inline(always)]
    fn default() -> Self {
        Self::Nothing
    }
}