//! Render a round through the page's SoundFont and the chip, to listen to (and time):
//! `cargo run --release --example font_take -- <out-dir> [mode]`.

use bandstand::Frame;
use bandstand::live::font::FontVoicing;
use sit_in::round::{band_voicing, Mode, Round, Rung, RATE};

fn wav(path: &str, frames: &[Frame]) {
    let mut b = Vec::new();
    let n = frames.len() as u32 * 4;
    b.extend(b"RIFF");
    b.extend((36 + n).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(RATE.to_le_bytes());
    b.extend((RATE * 4).to_le_bytes());
    b.extend(4u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(n.to_le_bytes());
    for f in frames {
        for s in [f.left, f.right] {
            b.extend(((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
    }
    std::fs::write(path, b).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).map_or("takes", |s| s.as_str());
    let mode = Mode::from_index(args.get(2).map_or(1, |m| m.parse().unwrap()));
    std::fs::create_dir_all(dir).unwrap();
    let font = FontVoicing::load(include_bytes!("../web/band.sf2")).unwrap();
    for (name, sf) in [("chip", false), ("font", true)] {
        let mut round = Round::new(7, mode, Rung::for_mode(mode, 3)).unwrap();
        if sf {
            round.engine.use_font(band_voicing(FontVoicing::combo(font.clone()))).unwrap();
        }
        let secs = (round.end_beat() * 60.0 / round.song.bpm as f64) as usize;
        let mut out = vec![Frame::ZERO; RATE as usize * secs];
        let t = std::time::Instant::now();
        for c in out.chunks_mut(1024) {
            round.engine.fill(c);
        }
        let took = t.elapsed().as_secs_f64();
        println!("{name}: {secs} s of music in {took:.2} s ({:.0}x real time)", secs as f64 / took);
        wav(&format!("{dir}/{mode:?}-{name}.wav"), &out);
    }
}
