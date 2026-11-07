# Sit In

An ear-training game on the bandstand music engine (crates.io `bandstand`, developed in
rabbiveesh/nat-han-adventures under `crates/bandstand`). The design doc (the ladder,
Blindfold mode, listening as the mechanic): https://claude.ai/artifact/HKzgS6WAw1DqCUgNVKW8uo

- Rust toolchain pinned (`rust-toolchain.toml`, with the wasm target). No wasm-bindgen: the
  crate is a bare `cdylib`, `src/ffi.rs` is the whole interface.
- The page (`web/index.html`) is plain JS + canvas; `scripts/build-web.sh` builds `dist/`.
- Never run `cargo fmt` over the repo (the style is wide lines, as in Nat Han).
- Run cargo under `nice -n 15`.
- Questions must be answerable by ear alone: never let a visual give the answer away before
  the door (the blindfold is the test).
- Engine features the game needs go into bandstand (in the Nat Han repo) and get released;
  depend on the released version here.
