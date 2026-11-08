//! Turnaround flavors: the two bars back to the I, and the substitutions players make in them.
//!
//! A question is three bars: `| I  x | y  V |` then the I it lands on (the door is there). The
//! doors name two flavors (they change from question to question); the band plays one.

/// A chord: root above the tonic (semitones) and quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ch(pub u8, pub Q);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Q {
    Maj7,
    M7,
    Dom7,
    Dim7,
    /// Half-diminished (m7b5), the minor ii.
    HalfDim,
    /// The minor V: a dominant with a flat 9 (its root left out of the voicing's top four).
    Dom7b9,
    /// The minor tonic.
    Min6,
}

impl Q {
    fn intervals(self) -> [u8; 4] {
        match self {
            Q::Maj7 => [0, 4, 7, 11],
            Q::M7 => [0, 3, 7, 10],
            Q::Dom7 => [0, 4, 7, 10],
            Q::Dim7 => [0, 3, 6, 9],
            Q::HalfDim => [0, 3, 6, 10],
            Q::Dom7b9 => [4, 7, 10, 13],
            Q::Min6 => [0, 3, 7, 9],
        }
    }

    fn chart(self) -> &'static str {
        match self {
            Q::Maj7 => "maj7",
            Q::M7 => "m7",
            Q::Dom7 => "7",
            Q::Dim7 => "dim7",
            Q::HalfDim => "m7b5",
            Q::Dom7b9 => "7b9",
            Q::Min6 => "m6",
        }
    }
}

/// A turnaround: four chords, two to a bar, into the I.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flavor {
    /// What the door says.
    pub label: &'static str,
    pub chords: [Ch; 4],
}

const I: Ch = Ch(0, Q::Maj7);
const II: Ch = Ch(2, Q::M7);
const V: Ch = Ch(7, Q::Dom7);

/// In the order they unlock: one more each step of the ladder.
pub const FLAVORS: [Flavor; 7] = [
    Flavor { label: "I vi ii V", chords: [I, Ch(9, Q::M7), II, V] },
    Flavor { label: "I VI7 ii V", chords: [I, Ch(9, Q::Dom7), II, V] },
    Flavor { label: "I #i° ii V", chords: [I, Ch(1, Q::Dim7), II, V] },
    Flavor { label: "iii VI7 ii V", chords: [Ch(4, Q::M7), Ch(9, Q::Dom7), II, V] },
    Flavor { label: "I VI7 ii bII7", chords: [I, Ch(9, Q::Dom7), II, Ch(1, Q::Dom7)] },
    Flavor { label: "I bIII7 ii bII7", chords: [I, Ch(3, Q::Dom7), II, Ch(1, Q::Dom7)] },
    Flavor { label: "Tadd: I bIII bVI bII", chords: [I, Ch(3, Q::Maj7), Ch(8, Q::Maj7), Ch(1, Q::Maj7)] },
];

/// How many flavors step `n` of the ladder plays from.
pub fn pool(n: u32) -> usize {
    (2 + n as usize).min(FLAVORS.len())
}

const FLAT: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"];
const SHARP: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// The chart token for `c` in the key of `tonic` (a pitch class): a diminished passing
/// chord is spelled sharp (it rises), the rest flat.
pub fn token(c: Ch, tonic: u8) -> String {
    let pc = ((tonic + c.0) % 12) as usize;
    let root = if c.1 == Q::Dim7 { SHARP[pc] } else { FLAT[pc] };
    format!("{root}{}", c.1.chart())
}

/// The chord as the comp voices it: close position, its lowest note from E below middle C
/// up to D# (so the voicings stay in one register and move little).
pub fn voicing(c: Ch, tonic: u8) -> [u8; 4] {
    let pc = (tonic + c.0) % 12;
    let low = 52 + (pc + 12 - 4) % 12; // 52 (E3) ..= 63 (D#4)
    c.1.intervals().map(|i| low + i)
}

/// How the comp voices a chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voicing {
    /// Close position from the root.
    Close,
    /// Root, 3rd and 7th (Bud Powell's shells).
    Shell,
    /// No root: 3 5 7 9 (or 3 13 7 9 on a dominant), Bill Evans / Wynton Kelly style.
    Rootless,
    /// Close position with the second voice from the top dropped an octave.
    Drop2,
}

impl Voicing {
    pub fn name(self) -> &'static str {
        match self {
            Voicing::Close => "close",
            Voicing::Shell => "shells",
            Voicing::Rootless => "rootless",
            Voicing::Drop2 => "drop 2",
        }
    }
}

/// `c` voiced `v`, lowest note first, in the comp's register.
pub fn voiced(c: Ch, tonic: u8, v: Voicing) -> Vec<u8> {
    let close = voicing(c, tonic);
    let root = close[0] - c.1.intervals()[0];
    let (third, seventh) = match c.1 {
        Q::Maj7 => (4, 11),
        Q::M7 | Q::HalfDim => (3, 10),
        Q::Dom7 | Q::Dom7b9 => (4, 10),
        Q::Dim7 => (3, 9),
        Q::Min6 => (3, 9),
    };
    match v {
        Voicing::Close => close.to_vec(),
        Voicing::Shell => vec![root, root + third, root + seventh],
        Voicing::Rootless => {
            let mut n: Vec<u8> = match c.1 {
                Q::Maj7 => vec![4, 7, 11, 14],
                Q::M7 => vec![3, 7, 10, 14],
                Q::Dom7 => vec![4, 9, 10, 14],
                Q::Dim7 => vec![3, 6, 9, 12],
                Q::HalfDim => vec![3, 6, 10, 12],
                Q::Dom7b9 => vec![4, 7, 10, 13],
                Q::Min6 => vec![3, 7, 9, 14],
            }
            .into_iter()
            .map(|i| root + i)
            .collect();
            // Keep it in the middle: rootless voicings sit from about F below middle C.
            if n[0] < 53 {
                n.iter_mut().for_each(|x| *x += 12);
            }
            if n[0] > 64 {
                n.iter_mut().for_each(|x| *x -= 12);
            }
            n
        }
        Voicing::Drop2 => {
            let [a, b, c2, d] = close;
            let mut n = vec![c2 - 12, a, b, d];
            if n[0] < 45 {
                n.iter_mut().for_each(|x| *x += 12);
            }
            n
        }
    }
}

/// The bass note for `c` (octave 2).
pub fn bass(c: Ch, tonic: u8) -> u8 {
    let pc = (tonic + c.0) % 12;
    36 + pc - if pc >= 8 { 12 } else { 0 } // Ab1 ..= G2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings_and_voicings() {
        assert_eq!(token(Ch(1, Q::Dim7), 0), "C#dim7");
        assert_eq!(token(Ch(1, Q::Dom7), 0), "Db7");
        assert_eq!(token(Ch(9, Q::M7), 7), "Em7");
        assert_eq!(voicing(I, 0), [60, 64, 67, 71]);
        assert_eq!(voiced(V, 0, Voicing::Rootless), vec![59, 64, 65, 69]);
        for t in 0..12 {
            for q in [Q::Maj7, Q::M7, Q::Dom7, Q::Dim7, Q::HalfDim, Q::Dom7b9, Q::Min6] {
                for v in [Voicing::Close, Voicing::Shell, Voicing::Rootless, Voicing::Drop2] {
                    let n = voiced(Ch(5, q), t, v);
                    assert!(n.windows(2).all(|w| w[0] < w[1]) && n[0] >= 45 && *n.last().unwrap() <= 79, "{q:?} {v:?} {n:?}");
                }
            }
        }
        assert_eq!(voicing(Ch(4, Q::M7), 0), [52, 55, 59, 62]);
        for t in 0..12 {
            for f in FLAVORS {
                for c in f.chords {
                    let b = bass(c, t);
                    assert!((32..=43).contains(&b), "{b}");
                    assert_eq!(b % 12, (t + c.0) % 12);
                }
            }
        }
    }
}
