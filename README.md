# Sit In

An ear-training game where you play along with a live band. The band is
[bandstand](https://crates.io/crates/bandstand), the music engine behind Nat Han Adventures:
the game writes each round as a tune, and the band plays it live.

Listening is the mechanic: the doors look the same until you pick one. Hear the band, pick
the door before your runner reaches it.

## Modes (the first rungs of the ladder)

1. **Major or minor?** The band stabs a triad: the sunny door (major, up) or the spooky door
   (minor, down). Then the lead spells the chord as you go through. Levels: C and G in root
   position, any key, inversions, no bass, faster.
2. **ii-V or just V? (chords)** Into the I chord: did the comp play a ii-V, or just the V?
   The bass moves in both (D-G against G-D), so the move alone gives nothing away.
3. **ii-V or just V? (solo)** The same changes, with the comp and bass laid out: only the
   soloist's line tells you. First the band's own lead (a chord tone on nearly every
   downbeat), then Bird, Sonny, Trane or Pops (bandstand's profiles), who say it less plainly.

Eight questions a tune; 7 or 8 right moves you up a level, 4 or fewer down one. **B** (or
the corner on the title) puts the blindfold on: the screen goes dark and only the sounds
tell you how you did.

## Play

```sh
scripts/build-web.sh           # dist/index.html + dist/sit_in.wasm, and dist/single.html
python3 -m http.server -d dist # then open http://localhost:8000
```

`dist/single.html` is the whole game in one file (the module inlined), so it also opens
straight from disk.

Keys: Up/W and Down/S pick a door, 1-3 pick a mode, B blindfold, M menu. Touch: the top
half of the screen is the upper door, the bottom half the lower one.

## Layout

- `src/round.rs` a round: the questions (the dice), the tune written for them (MML for the
  chip channels, and a chord chart for the solo), the band configured to play it.
- `src/ffi.rs` the page's interface: plain `extern "C"` functions (no wasm-bindgen), the
  audio read straight out of the module's memory.
- `web/index.html` the game: drawing, input, and the audio scheduler (it renders the band in
  1024-frame buffers 0.2 s ahead and plays them back to back at 32 kHz; the runner moves on
  the heard beat: the audio clock minus the output latency).

`cargo test` checks every mode and level writes a tune that plays, the clock stays on the
written tempo, and the solo mode's lead outlines the changes.
