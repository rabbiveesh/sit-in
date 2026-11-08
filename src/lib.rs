//! Sit In: an ear-training game where you play along with a live band.
//!
//! The first rung of the ladder, "Major or minor": the band stabs a chord, you pick the sunny
//! door (major) or the spooky door (minor) before you reach it, and the band answers with the
//! chord's arpeggio. Each round is a short tune the game writes itself (a count-in bar, then
//! two bars a question) and plays live through [`bandstand`].
//!
//! The crate is the game's music and rules; `web/` draws it. On the web it is a bare
//! `cdylib` (no wasm-bindgen): the page calls the `extern "C"` functions in [`ffi`] and reads
//! the audio straight out of linear memory.

pub mod ffi;
pub mod round;
pub mod turnaround;

pub use round::{Mode, Quality, Question, Round, Rung};
