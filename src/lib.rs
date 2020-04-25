//! A foundation for Minesweeper implementations in Rust.
//!
//! The purpose of such a crate is to have Minesweeper implementations depend on a common crate, i.e. for them to share the basic code related to managing a Minesweeper session and only bother writing the code for the UI, sound effects, input and such. Such implementations can be seen as frontends to this library.
//!
//! # Feature gates
//! - `std` — enable a dependency on the hosted standard library (**enabled by default**)
//!
//!   Without this feature, the crate only depends on `core` and `alloc` (meaning that usage in an environment without even a memory allocator is impossible), allowing it to run in a freestanding environment, allowing one to implement Minesweeper on a microcontroller, meaning Arduino Minesweeper, ESP32 Minesweeper, OSDev Minesweeper... you name it.
//!
//! - `generation` — enable random generation (**enabled by default**)
//!
//!   Enables the dependency on `rand`, used for generating random fields. Disable to remove said dependency if you'd like to use your own random field generator.
//!
//! - `track_caller` — use `track_caller` attributes
//!
//!   Places the `track_caller` attribute on indexing operators and other panicking methods, improving panic messages. **Requires a nightly compiler as of Rust 1.43.0**.

#![cfg_attr(feature = "track_caller", feature(track_caller))]
// Copypaste the following to enable this on specific methods:
//  #[cfg_attr(features = "track_caller", track_caller)]

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod field;
pub use field::*;