//! The page's interface: plain `extern "C"` functions over one round at a time.
//!
//! JS calls [`round_new`], then [`round_fill`] for each buffer of audio and reads the
//! interleaved stereo samples at [`audio_ptr`]. Answers come from [`round_quality`] etc.

use std::cell::RefCell;

use bandstand::{Frame, Sfx};

use crate::round::{Mode, Round, Rung};

thread_local! {
    static ROUND: RefCell<Option<Round>> = const { RefCell::new(None) };
    static AUDIO: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static TEXT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn with<T>(f: impl FnOnce(&mut Round) -> T, or: T) -> T {
    ROUND.with_borrow_mut(|r| r.as_mut().map_or(or, f))
}

/// Start a round of `mode` (0 major or minor, 1 ii-V or V in the chords, 2 the same in the
/// solo) on step `rung` of its ladder. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn round_new(seed: u32, mode: u32, rung: u32) -> u32 {
    let mode = Mode::from_index(mode);
    match Round::new(seed as u64, mode, Rung::for_mode(mode, rung)) {
        Ok(r) => {
            ROUND.set(Some(r));
            1
        }
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn round_bpm() -> f32 {
    with(|r| r.rung.bpm, 0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn round_bars() -> u32 {
    with(|r| r.bars(), 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn round_questions() -> u32 {
    with(|r| r.questions.len() as u32, 0)
}

/// The bar question `q` asks in.
#[unsafe(no_mangle)]
pub extern "C" fn round_ask_bar(q: u32) -> u32 {
    Round::ask_bar(q as usize)
}

/// Question `q`'s answer: 1 up (major; ii-V), 0 down (minor; just V).
#[unsafe(no_mangle)]
pub extern "C" fn round_answer(q: u32) -> u32 {
    with(|r| r.questions.get(q as usize).map_or(0, |q| q.up as u32), 0)
}

/// Question `q`'s chord name ("D minor"), as UTF-8 at [`text_ptr`]; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn round_name(q: u32) -> u32 {
    let name = with(|r| r.questions.get(q as usize).map(|q| q.name.clone()).unwrap_or_default(), String::new());
    TEXT.with_borrow_mut(|t| {
        *t = name;
        t.len() as u32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_ptr() -> *const u8 {
    TEXT.with_borrow(|t| t.as_ptr())
}

/// Render the next `n` frames of the round (silence without one) as interleaved stereo
/// at [`audio_ptr`].
#[unsafe(no_mangle)]
pub extern "C" fn round_fill(n: u32) {
    let n = n as usize;
    FRAMES.with_borrow_mut(|frames| {
        frames.clear();
        frames.resize(n, Frame::ZERO);
        with(|r| r.engine.fill(frames), ());
        AUDIO.with_borrow_mut(|a| {
            a.clear();
            a.extend(frames.iter().flat_map(|f| [f.left, f.right]));
        });
    });
}

/// Render a sound effect (0 right, 1 wrong, 2 menu) as interleaved stereo at [`audio_ptr`];
/// returns its length in frames (its rate is [`sfx_rate`]).
#[unsafe(no_mangle)]
pub extern "C" fn sfx_render(which: u32) -> u32 {
    let sfx = match which {
        0 => Sfx::Nugget,
        1 => Sfx::Boing,
        _ => Sfx::MenuSelect,
    };
    let r = bandstand::sfx::render(sfx);
    SFX_RATE.set(r.sample_rate);
    AUDIO.with_borrow_mut(|a| {
        a.clear();
        a.extend(r.frames.iter().flat_map(|f| [f.left, f.right]));
    });
    r.frames.len() as u32
}

thread_local! {
    static SFX_RATE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[unsafe(no_mangle)]
pub extern "C" fn sfx_rate() -> u32 {
    SFX_RATE.get()
}

#[unsafe(no_mangle)]
pub extern "C" fn audio_ptr() -> *const f32 {
    AUDIO.with_borrow(|a| a.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_rate() -> u32 {
    crate::round::RATE
}
