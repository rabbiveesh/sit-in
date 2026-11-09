//! The page's interface: plain `extern "C"` functions over one round at a time.
//!
//! JS calls [`round_new`], then [`round_fill`] for each buffer of audio and reads the
//! interleaved stereo samples at [`audio_ptr`]. Answers come from [`round_quality`] etc.

use std::cell::RefCell;

use std::cell::Cell;

use bandstand::live::font::FontVoicing;
use bandstand::{Frame, Sfx};

use crate::round::{Mode, Round, Rung};

thread_local! {
    static ROUND: RefCell<Option<Round>> = const { RefCell::new(None) };
    static AUDIO: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static TEXT: RefCell<String> = const { RefCell::new(String::new()) };
    /// The SoundFont the page loaded, and whether rounds play through it (else the chip).
    static FONT: RefCell<Option<FontVoicing>> = const { RefCell::new(None) };
    static FONT_ON: Cell<bool> = const { Cell::new(true) };
    static FONT_BYTES: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Room for a `.sf2` of `len` bytes: the page writes it at the returned pointer, then calls
/// [`font_load`].
#[unsafe(no_mangle)]
pub extern "C" fn font_buffer(len: u32) -> *mut u8 {
    FONT_BYTES.with_borrow_mut(|b| {
        *b = vec![0; len as usize];
        b.as_mut_ptr()
    })
}

/// Parse the SoundFont written at [`font_buffer`]; returns 1 on success. Rounds started after
/// it play through it (while [`font_use`] is on).
#[unsafe(no_mangle)]
pub extern "C" fn font_load() -> u32 {
    let bytes = FONT_BYTES.take();
    match FontVoicing::load(&bytes) {
        Ok(f) => {
            FONT.set(Some(crate::round::band_voicing(FontVoicing::combo(f))));
            1
        }
        Err(_) => 0,
    }
}

/// 1: rounds play through the SoundFont (once loaded), 0: the 8-bit chip.
#[unsafe(no_mangle)]
pub extern "C" fn font_use(on: u32) {
    FONT_ON.set(on != 0);
}

fn with<T>(f: impl FnOnce(&mut Round) -> T, or: T) -> T {
    ROUND.with_borrow_mut(|r| r.as_mut().map_or(or, f))
}

/// Start a round of `mode` (0 major or minor, 1 ii-V or V in the chords, 2 the same in the
/// solo, 3 turnarounds in the chords, 4 in the solo, 5 major or minor ii-V-I in the chords, 6 in
/// the solo, 7 which voicing, 8 a real tune) on step `rung` of its ladder, at `tempo` percent of its tempo. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn round_new(seed: u32, mode: u32, rung: u32, tempo: u32) -> u32 {
    let mode = Mode::from_index(mode);
    match Round::new(seed as u64, mode, Rung::for_mode(mode, rung).at_tempo(tempo)) {
        Ok(mut r) => {
            if let (true, Some(v)) = (FONT_ON.get(), FONT.with_borrow(Clone::clone)) {
                if r.engine.use_font(v).is_err() {
                    return 0;
                }
            }
            ROUND.set(Some(r));
            1
        }
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn round_bpm() -> f32 {
    with(|r| r.song.bpm, 0.0)
}

/// Where the tune ends, in beats.
#[unsafe(no_mangle)]
pub extern "C" fn round_end_beat() -> f64 {
    with(|r| r.end_beat(), 0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn round_questions() -> u32 {
    with(|r| r.questions.len() as u32, 0)
}

/// The beat question `q` starts asking on.
#[unsafe(no_mangle)]
pub extern "C" fn round_ask_beat(q: u32) -> f64 {
    with(|r| r.questions.get(q as usize).map_or(0.0, |q| q.ask), 0.0)
}

/// How the comp plays question `q` ("Charleston, rootless"), as UTF-8 at [`text_ptr`];
/// returns its length (0 when the comp lays out).
#[unsafe(no_mangle)]
pub extern "C" fn round_comp(q: u32) -> u32 {
    let comp = with(|r| r.questions.get(q as usize).and_then(|q| q.comp.clone()).unwrap_or_default(), String::new());
    TEXT.with_borrow_mut(|t| {
        *t = comp;
        t.len() as u32
    })
}

/// The tune's title, as UTF-8 at [`text_ptr`]; returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn round_title() -> u32 {
    let title = with(|r| r.song.title.clone(), String::new());
    TEXT.with_borrow_mut(|t| {
        *t = title;
        t.len() as u32
    })
}

/// 1 if question `q` is a free listen (the tune opens with them: the door shown, no score).
#[unsafe(no_mangle)]
pub extern "C" fn round_demo(q: u32) -> u32 {
    with(|r| r.questions.get(q as usize).map_or(0, |q| q.demo as u32), 0)
}

/// The beat question `q`'s door is at.
#[unsafe(no_mangle)]
pub extern "C" fn round_door_beat(q: u32) -> f64 {
    with(|r| r.questions.get(q as usize).map_or(0.0, |q| q.door), 0.0)
}

/// What question `q`'s upper (`up` 1) or lower door says, as UTF-8 at [`text_ptr`]; returns
/// its length (0: the mode's fixed doors).
#[unsafe(no_mangle)]
pub extern "C" fn round_label(q: u32, up: u32) -> u32 {
    let label = with(|r| r.questions.get(q as usize).and_then(|q| q.labels.clone()).map(|l| l[(up == 0) as usize].clone()).unwrap_or_default(), String::new());
    TEXT.with_borrow_mut(|t| {
        *t = label;
        t.len() as u32
    })
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

/// Skip the round's first `frames` frames, rendered and dropped (to replay from a point: the
/// page rebuilds the same round, which is deterministic, and skips to just before a question).
#[unsafe(no_mangle)]
pub extern "C" fn round_skip(frames: u32) {
    FRAMES.with_borrow_mut(|buf| {
        buf.clear();
        buf.resize(1024, Frame::ZERO);
        with(
            |r| {
                let mut left = frames as usize;
                while left > 0 {
                    let n = left.min(buf.len());
                    r.engine.fill(&mut buf[..n]);
                    left -= n;
                }
            },
            (),
        );
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
