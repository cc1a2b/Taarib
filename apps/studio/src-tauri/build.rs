//! Generates the Tauri context — the parsed configuration, the bundled
//! frontend, the capability set and the platform resources — that
//! `tauri::generate_context!()` expands into at compile time, and records the
//! target triple this build is for.

fn main() {
    // The update channel's manifest is keyed by Rust target triple, and the
    // only place that string is known exactly is here: `std::env::consts`
    // gives an OS and an architecture, not a triple, and the two do not
    // reconstruct `x86_64-pc-windows-msvc` from inside the built binary.
    #[expect(
        clippy::disallowed_methods,
        reason = "cargo hands a build script its inputs only as environment variables, so there \
                  is no configuration layer here to resolve TARGET through"
    )]
    let hadaf = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=TAARIB_HADAF={hadaf}");
    println!("cargo:rerun-if-env-changed=TARGET");

    tauri_build::build();
}
