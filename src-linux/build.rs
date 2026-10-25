fn main() {
    println!("cargo:rerun-if-changed=src/main.c");

    // Same shape as src-win: the real entry point lives in C, main.rs only
    // forwards to it. Renaming main -> dqc_main avoids the duplicate symbol.
    cc::Build::new()
        .file("src/main.c")
        .define("main", "dqc_main")
        .flag_if_supported("-Os")
        .flag_if_supported("-fno-ident")
        .flag_if_supported("-ffunction-sections")
        .flag_if_supported("-fdata-sections")
        .compile("dqc_runner");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,--gc-sections");
        println!("cargo:rustc-link-arg=-Wl,--build-id=none");
    }
}
