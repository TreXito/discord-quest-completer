// The Linux runner lives in main.c (see build.rs, which renames its `main` to
// `dqc_main`). This shim only forwards to it, so `cargo build --release` and
// `./build.sh` produce an equivalent binary. Kept deliberately dumb: any Rust
// runtime weight here would be paid once per fake game on disk.
#![no_main]

extern "C" {
    fn dqc_main(argc: i32, argv: *const *const u8) -> i32;
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    unsafe { dqc_main(argc, argv) }
}
