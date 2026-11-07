//! A round: a few questions written into one tune for the band.

use bandstand::live::chorus::{Call, Chorus};
use bandstand::live::soloist::Soloist;
use bandstand::live::{Engine, EngineConfig, Input, SongFile};

/// The engine's sample rate (the page plays its buffers at this rate; the browser resamples).
pub const RATE: u32 = 32_000;
/// Questions in a round.
pub const QUESTIONS: usize = 8;
/// Bars before the first question (the drummer counts in).
pub const COUNT_IN: u32 = 1;
/// Bars per question: the band asks, then answers.
pub const BARS_PER_QUESTION: u32 = 2;

/// What the game asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A triad: major (up, the sunny door) or minor (down, the spooky door)?
    MajorMinor,
    /// Into the I chord: a ii-V (up) or just the V (down)? Heard in the comp's chords, or
    /// (`solo`) only in the soloist's line over a silent rhythm section.
    TwoFive { solo: bool },
}

impl Mode {
    pub fn from_index(i: u32) -> Mode {
        match i {
            1 => Mode::TwoFive { solo: false },
            2 => Mode::TwoFive { solo: true },
            _ => Mode::MajorMinor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Major,
    Minor,
}

impl Quality {
    fn third(self) -> u8 {
        match self {
            Quality::Major => 4,
            Quality::Minor => 3,
        }
    }
}

/// How hard a round is: the ladder's steps within "Major or minor".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rung {
    pub bpm: f32,
    /// Roots anywhere in the octave (else only around C and G).
    pub any_root: bool,
    /// The comp voices the chord in inversions.
    pub inversions: bool,
    /// The bass plays root and fifth under the stabs (a firmer floor to hear from).
    pub bass: bool,
}

impl Rung {
    /// Step `n` of the ladder (0-based) for `mode`, steeper the higher; past the last it just
    /// speeds up.
    pub fn for_mode(mode: Mode, n: u32) -> Rung {
        match mode {
            Mode::MajorMinor => Rung::step(n),
            // Changes go by faster than triads: start at a medium swing.
            Mode::TwoFive { solo } => {
                let start = if solo { 132.0 } else { 116.0 };
                Rung { bpm: (start + 16.0 * n as f32).min(232.0), any_root: n >= 1, inversions: false, bass: !solo }
            }
        }
    }

    /// Step `n` of the major-or-minor ladder.
    pub fn step(n: u32) -> Rung {
        match n {
            0 => Rung { bpm: 84.0, any_root: false, inversions: false, bass: true },
            1 => Rung { bpm: 96.0, any_root: true, inversions: false, bass: true },
            2 => Rung { bpm: 108.0, any_root: true, inversions: true, bass: true },
            3 => Rung { bpm: 120.0, any_root: true, inversions: true, bass: false },
            n => Rung { bpm: (120.0 + 12.0 * (n - 3) as f32).min(184.0), any_root: true, inversions: true, bass: false },
        }
    }
}

/// One question: the answer (up or down) and what was played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    /// Up (major; ii-V) or down (minor; just V).
    pub up: bool,
    /// What it was, to show after the door ("D minor", "Em7 A7 | D").
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triad {
    /// MIDI root in the comp's register.
    pub root: u8,
    pub quality: Quality,
    /// 0 root position, 1 first inversion, 2 second.
    pub inversion: u8,
}

impl Triad {
    pub fn name(&self) -> String {
        const NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
        let q = match self.quality {
            Quality::Major => "major",
            Quality::Minor => "minor",
        };
        format!("{} {q}", NAMES[(self.root % 12) as usize])
    }

    /// The triad as the comp voices it, lowest first.
    fn voicing(&self) -> [u8; 3] {
        let mut v = [self.root, self.root + self.quality.third(), self.root + 7];
        for _ in 0..self.inversion {
            v.rotate_left(1);
            v[2] += 12;
        }
        v
    }
}

/// A small PRNG (xorshift), so a seed replays a round exactly.
#[derive(Debug, Clone)]
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One round: its questions and the band playing them.
pub struct Round {
    pub mode: Mode,
    pub rung: Rung,
    pub questions: Vec<Question>,
    pub song: SongFile,
    pub engine: Engine,
}

/// No exact repeats back to back, and never three of one answer in a row (no streak to
/// coast on).
fn fits(answers: &[bool], up: bool) -> bool {
    let n = answers.len();
    !(n >= 2 && answers[n - 1] == up && answers[n - 2] == up)
}

impl Round {
    pub fn new(seed: u64, mode: Mode, rung: Rung) -> Result<Round, String> {
        match mode {
            Mode::MajorMinor => Round::major_minor(seed, rung),
            Mode::TwoFive { solo } => Round::two_five(seed, rung, solo),
        }
    }

    fn major_minor(seed: u64, rung: Rung) -> Result<Round, String> {
        let mut dice = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut questions = Vec::with_capacity(QUESTIONS);
        let mut last: Option<Triad> = None;
        while questions.len() < QUESTIONS {
            // Roots from G below middle C up to F#: the comp's middle register.
            let root = if rung.any_root {
                55 + dice.below(12) as u8
            } else {
                [60, 55][dice.below(2) as usize]
            };
            let quality = if dice.below(2) == 0 { Quality::Major } else { Quality::Minor };
            let inversion = if rung.inversions { dice.below(3) as u8 } else { 0 };
            let q = Triad { root, quality, inversion };
            let answers: Vec<bool> = questions.iter().map(|t: &Triad| t.quality == Quality::Major).collect();
            if last == Some(q) || !fits(&answers, quality == Quality::Major) {
                continue;
            }
            last = Some(q);
            questions.push(q);
        }
        let song = write_triads(&questions, rung)?;
        let config = EngineConfig { seed, intro: false, soloist: None, ..EngineConfig::on_grid() };
        let mut engine = Engine::with_config(&song, RATE, config)?;
        engine.post(Input::SetFreedom { lead: 0.0, comp: 0.0, bass: 0.0, drums: 0.15, dynamics: 0.2 });
        let questions = questions.iter().map(|t| Question { up: t.quality == Quality::Major, name: t.name() }).collect();
        Ok(Round { mode: Mode::MajorMinor, rung, questions, song, engine })
    }

    /// ii-V-I or V-I, in one key per round (a key of its own each round from the second rung).
    fn two_five(seed: u64, rung: Rung, solo: bool) -> Result<Round, String> {
        let mut dice = Dice(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        // The tonic in the comp's register: C, or any key from G below middle C to F#.
        let tonic = if rung.any_root { 55 + dice.below(12) as u8 } else { 60 };
        let mut answers = Vec::with_capacity(QUESTIONS);
        while answers.len() < QUESTIONS {
            let up = dice.below(2) == 0;
            if fits(&answers, up) {
                answers.push(up);
            }
        }
        let song = write_two_fives(&answers, tonic, rung, solo)?;
        // The band's own lead outlines the changes (chord tones on nearly every downbeat): the
        // first two rungs. Then the players, whose lines lean on the chords less plainly (Miles
        // sits out too many questions for this game).
        let soloist = if rung.any_root && rung.bpm >= 160.0 { [Soloist::Bird, Soloist::Sonny, Soloist::Trane, Soloist::Pops][dice.below(4) as usize] } else { Soloist::Band };
        let config = EngineConfig { seed, intro: false, soloist: solo.then_some(soloist), ..EngineConfig::on_grid() };
        let mut engine = Engine::with_config(&song, RATE, config)?;
        if solo {
            // The soloist blows over the changes; the comp and bass lay out (a comp under 0.6
            // never reharmonizes, so the changes stay as written).
            engine.post(Input::ForceChorus(Some(Call { chorus: Chorus::Blowing, key_up: false })));
            engine.post(Input::SetFreedom { lead: 0.7, comp: 0.0, bass: 0.0, drums: 0.2, dynamics: 0.4 });
            engine.post(Input::SetMix([1.0, 0.0, 0.0, 1.0]));
        } else {
            engine.post(Input::SetFreedom { lead: 0.0, comp: 0.0, bass: 0.0, drums: 0.15, dynamics: 0.2 });
        }
        let names = |up: bool| {
            let t = (tonic % 12) as usize;
            let (ii, v, i) = (CHORD_NAMES[(t + 2) % 12], CHORD_NAMES[(t + 7) % 12], CHORD_NAMES[t]);
            if up { format!("{ii}m7 {v}7 | {i}") } else { format!("{v}7 | {i}") }
        };
        let questions = answers.iter().map(|&up| Question { up, name: names(up) }).collect();
        Ok(Round { mode: Mode::TwoFive { solo }, rung, questions, song, engine })
    }

    /// The bar question `q` asks in (0-based, the count-in is bar 0).
    pub fn ask_bar(q: usize) -> u32 {
        COUNT_IN + q as u32 * BARS_PER_QUESTION
    }

    /// Bars in the whole tune (one to end on after the last answer).
    pub fn bars(&self) -> u32 {
        COUNT_IN + QUESTIONS as u32 * BARS_PER_QUESTION + 1
    }
}

const CHORD_NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// The ii-V or V into I, two bars a question, in the key of `tonic` (a MIDI note in the
/// comp's register). With `solo` only the chart and the drums are written: the soloist plays
/// the changes.
fn write_two_fives(answers: &[bool], tonic: u8, rung: Rung, solo: bool) -> Result<SongFile, String> {
    let t = tonic;
    let name = |pc: u8| CHORD_NAMES[(pc % 12) as usize];
    let (ii, v, i) = (format!("{}m7", name(t + 2)), format!("{}7", name(t + 7)), format!("{}maj7", name(t)));
    // Close voicings that lead into each other: ii (D F A C), V (F G B D), I (E G B D).
    let (vii, vv, vi) = ([t + 2, t + 5, t + 9, t + 12], [t + 5, t + 7, t + 11, t + 14], [t + 4, t + 7, t + 11, t + 14]);
    // The bass in octave 2 or so.
    let low = |n: u8| n - 24 - if n % 12 >= 7 && n >= 67 { 12 } else { 0 };
    let (mut lead, mut comp, mut bass, mut drums) = (String::from("v11 @2 "), String::from("v8 @1 "), String::new(), String::from("v8 "));
    let mut chart = format!("| {i} ");
    for p in [&mut lead, &mut comp, &mut bass] {
        p.push_str("r1 ");
    }
    drums.push_str("H4 h4 h4 h4 ");
    for &up in answers {
        lead.push_str("r1 r1 ");
        if solo {
            comp.push_str("r1 r1 ");
            bass.push_str("r1 r1 ");
        } else {
            if up {
                comp.push_str(&format!("{} {} ", chord(&vii, "2"), chord(&vv, "2")));
                bass.push_str(&format!("{}2 {}2 ", note(low(t + 2)), note(low(t + 7))));
            } else {
                comp.push_str(&format!("{} {} ", chord(&vv, "2"), chord(&vv, "2")));
                // Root and fifth: the bass moves in both, so the move alone gives nothing away.
                bass.push_str(&format!("{}2 {}2 ", note(low(t + 7)), note(low(t + 2))));
            }
            comp.push_str(&format!("{} ", chord(&vi, "1")));
            if rung.bass {
                bass.push_str(&format!("{}1 ", note(low(t))));
            } else {
                bass.push_str("r1 ");
            }
        }
        drums.push_str("k4 h4 s4 h4 k4 h4 s4 h8 h8 ");
        chart.push_str(&if up { format!("| {ii} {v} | {i} ") } else { format!("| {v} | {i} ") });
    }
    lead.push_str("r1");
    comp.push_str(&if solo { "r1".to_string() } else { chord(&vi, "1") });
    bass.push_str(&if solo { "r1".to_string() } else { format!("{}1", note(low(t))) });
    drums.push_str("x2 r2");
    chart.push_str(&format!("| {i} |"));
    SongFile::from_mml("Sit In: ii-V or V", rung.bpm, 0.5, false, t % 12, &chart, [&lead, &comp, &bass, &drums])
}

/// MML for one MIDI note (with its octave).
fn note(midi: u8) -> String {
    const PC: [&str; 12] = ["c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b"];
    format!("o{} {}", midi / 12 - 1, PC[(midi % 12) as usize])
}

/// MML for a chord (the chip's fast arpeggio), lowest note first.
fn chord(notes: &[u8], len: &str) -> String {
    let mut s = format!("o{} {{", notes[0] / 12 - 1);
    let mut oct = notes[0] / 12;
    const PC: [&str; 12] = ["c", "c+", "d", "d+", "e", "f", "f+", "g", "g+", "a", "a+", "b"];
    for (i, &n) in notes.iter().enumerate() {
        while n / 12 > oct {
            s.push('>');
            oct += 1;
        }
        if i > 0 {
            s.push(' ');
        }
        s.push_str(PC[(n % 12) as usize]);
    }
    s.push('}');
    s.push_str(len);
    s
}

fn write_triads(questions: &[Triad], rung: Rung) -> Result<SongFile, String> {
    let (mut lead, mut comp, mut bass, mut drums) = (String::from("v11 @2 "), String::from("v8 @1 "), String::new(), String::from("v8 "));
    // Count-in: four hats.
    for p in [&mut lead, &mut comp, &mut bass] {
        p.push_str("r1 ");
    }
    drums.push_str("H4 h4 h4 h4 ");
    for q in questions {
        let v = q.voicing();
        let (r, third, fifth) = (q.root, q.root + q.quality.third(), q.root + 7);
        // Ask: two stabs, the bass under them.
        lead.push_str("r1 ");
        comp.push_str(&format!("{} {} ", chord(&v, "2"), chord(&v, "2")));
        if rung.bass {
            bass.push_str(&format!("{}2 {}2 ", note(r - 24), note(fifth - 24)));
        } else {
            bass.push_str("r1 ");
        }
        drums.push_str("k4 h4 s4 h4 ");
        // Answer: the lead spells the chord, leaning on the third; the comp holds it.
        lead.push_str(&format!("{}8 {}8 {}8 {}8 {}4 r4 ", note(r + 12), note(third + 12), note(fifth + 12), note(r + 24), note(third + 12)));
        comp.push_str(&format!("{} ", chord(&v, "1")));
        bass.push_str(&format!("{}1 ", note(r - 24)));
        drums.push_str("k4 h4 s4 h8 h8 ");
    }
    // A bar to end on.
    lead.push_str("r1");
    comp.push_str("r1");
    bass.push_str("r1");
    drums.push_str("x2 r2");
    SongFile::from_mml("Sit In: major or minor", rung.bpm, 0.3, false, 0, "", [&lead, &comp, &bass, &drums])
}

#[cfg(test)]
mod tests {
    use super::*;
    use bandstand::Frame;

    #[test]
    fn chords_spell_in_mml() {
        assert_eq!(chord(&[60, 64, 67], "2"), "o4 {c e g}2");
        assert_eq!(chord(&[64, 67, 72], "2"), "o4 {e g> c}2");
        assert_eq!(note(43), "o2 g");
    }

    #[test]
    fn inversions_keep_the_notes() {
        let q = Triad { root: 62, quality: Quality::Minor, inversion: 2 };
        assert_eq!(q.voicing(), [69, 74, 77]);
        assert_eq!(q.name(), "D minor");
    }

    #[test]
    fn every_rung_writes_a_round_that_plays() {
        for (mode, n) in [0, 1, 2].into_iter().flat_map(|m| (0..6).map(move |n| (Mode::from_index(m), n))) {
            let mut round = Round::new(n as u64 + 1, mode, Rung::for_mode(mode, n)).unwrap();
            assert_eq!(round.questions.len(), QUESTIONS);
            // No three of a kind in a row.
            assert!(round.questions.windows(3).all(|w| !(w[0].up == w[1].up && w[1].up == w[2].up)));
            let secs = round.bars() as f32 * 4.0 * 60.0 / round.rung.bpm;
            let mut out = vec![Frame::ZERO; (secs * RATE as f32) as usize];
            round.engine.fill(&mut out);
            let rms = (out.iter().map(|f| (f.left as f64).powi(2)).sum::<f64>() / out.len() as f64).sqrt();
            assert!(rms > 0.01, "{mode:?} rung {n} is silent: {rms}");
            assert!(out.iter().all(|f| f.left.is_finite() && f.left.abs() <= 1.0));
        }
    }

    /// The beat the page computes from samples (on the grid, at a steady tempo) is the engine's.
    #[test]
    fn the_clock_is_steady() {
        let mut round = Round::new(3, Mode::MajorMinor, Rung::step(1)).unwrap();
        let mut out = vec![Frame::ZERO; RATE as usize * 7];
        round.engine.fill(&mut out);
        let beat = round.engine.beat_clock().position.song_beat;
        let want = 7.0 * round.rung.bpm as f64 / 60.0;
        assert!((beat - want).abs() < 0.01, "{beat} vs {want}");
    }
}

#[cfg(test)]
mod solo_tests {
    use super::*;
    use bandstand::Frame;
    use std::collections::BTreeMap;

    /// The soloist plays the changes as written: through the first half of each ii-V bar its
    /// strong notes sit in the ii more than in the V, and the reverse in a bar of just V.
    #[test]
    fn the_soloist_spells_the_changes() {
        let (mut ii_hits, mut ii_total, mut v_hits, mut v_total, mut silent) = (0, 0, 0, 0, 0);
        let (mut down_in, mut down_total) = (0u32, 0u32);
        for seed in 1..=6u64 {
            let mut round = Round::new(seed, Mode::TwoFive { solo: true }, Rung::for_mode(Mode::TwoFive { solo: true }, 1)).unwrap();
            let tonic = round.song.key as i32;
            let samples_per_beat = RATE as f64 * 60.0 / round.rung.bpm as f64;
            let mut lead: BTreeMap<u64, u8> = BTreeMap::new();
            let mut buf = vec![Frame::ZERO; 512];
            while !round.engine.finished() && round.engine.beat_clock().position.bar < round.bars() as u64 {
                round.engine.fill(&mut buf);
                for e in round.engine.pending_events() {
                    if let (0, bandstand::live::voice::Sound::Note(n)) = (e.ch, &e.sound) {
                        lead.insert(e.start, *n);
                    }
                }
            }
            for (q, question) in round.questions.iter().enumerate() {
                let bar0 = Round::ask_bar(q) as f64 * 4.0;
                // Notes in the first half of the asking bar.
                // The note sounding on beat 1 (started within an 8th before or after it).
                let at = |b: f64| lead.range(((b - 0.5) * samples_per_beat) as u64..((b + 0.25) * samples_per_beat) as u64).last().map(|(_, &n)| (n as i32 - tonic).rem_euclid(12));
                if let Some(p) = at(bar0) {
                    let tones: &[i32] = if question.up { &[2, 5, 9, 0] } else { &[7, 11, 2, 5] };
                    down_total += 1;
                    down_in += tones.contains(&p) as u32;
                }
                let notes: Vec<i32> = lead.range((bar0 * samples_per_beat) as u64..((bar0 + 2.0) * samples_per_beat) as u64).map(|(_, &n)| (n as i32 - tonic).rem_euclid(12)).collect();
                if notes.is_empty() {
                    silent += 1;
                    continue;
                }
                let ii = notes.iter().filter(|&&p| [2, 5, 9, 0].contains(&p)).count();
                let v = notes.iter().filter(|&&p| [7, 11, 2, 5].contains(&p)).count();
                // Tones that tell them apart: ii's A and C against V's G and B.
                let (ii_only, v_only) = (notes.iter().filter(|&&p| p == 9 || p == 0).count(), notes.iter().filter(|&&p| p == 7 || p == 11).count());
                if question.up {
                    ii_total += 1;
                    ii_hits += (ii_only > v_only) as u32;
                } else {
                    v_total += 1;
                    v_hits += (v_only > ii_only) as u32;
                }
                eprintln!("seed {seed} q{q} {} {notes:?} ii {ii} v {v}", question.name);
            }
        }
        eprintln!("downbeat chord tones {down_in}/{down_total}");
        eprintln!("ii bars told apart {ii_hits}/{ii_total}, V bars {v_hits}/{v_total}, silent {silent}");
        assert!(silent <= 2, "the soloist sat out {silent} questions");
        assert!(down_in * 10 >= down_total * 8, "downbeat chord tones {down_in}/{down_total}");
        let _ = (ii_hits, ii_total, v_hits, v_total);
    }
}
