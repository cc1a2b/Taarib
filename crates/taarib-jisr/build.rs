//! Generates `include/taarib.h` from the crate's public surface through
//! cbindgen, configured by `cbindgen.toml` beside this file.
//!
//! The header is committed, so consumers bind against the ABI without a Rust
//! toolchain — which also means regeneration here is best-effort, never
//! load-bearing. A vendored build with a read-only source tree, a CI checkout
//! with unusual permissions, or a sandboxed packager must all be able to build
//! this crate from the committed header alone. So nothing in this script fails
//! the build: a header that cannot be generated or cannot be written becomes a
//! `cargo:warning` naming the reason, and compilation continues.
//!
//! The one thing this script never does is write a header that differs from
//! the surface silently: when generation succeeds, the produced bytes are
//! compared with the committed file and the file is rewritten only on a real
//! difference, so an unchanged surface leaves the working tree untouched and
//! an incremental build does not churn the file's timestamp.

use std::path::PathBuf;

#[expect(
    clippy::disallowed_methods,
    reason = "a build script's environment is Cargo's protocol, not application \
              configuration; CARGO_MANIFEST_DIR has no other transport and the \
              settings layer the rule points at does not exist at build time"
)]
fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let Ok(jidhr) = std::env::var("CARGO_MANIFEST_DIR") else {
        println!(
            "cargo:warning=taarib-jisr: CARGO_MANIFEST_DIR is not set; \
             include/taarib.h was not regenerated and the committed header is used as-is"
        );
        return;
    };

    // Generation walks the whole crate. A parse failure here is a diagnostic,
    // not a build failure: the compiler itself will report the same breakage
    // with a better message, and the committed header still describes the last
    // surface that built.
    let rubat = match cbindgen::generate(&jidhr) {
        Ok(rubat) => rubat,
        Err(sabab) => {
            println!(
                "cargo:warning=taarib-jisr: cbindgen could not generate include/taarib.h \
                 ({sabab}); the committed header is used as-is"
            );
            return;
        },
    };

    // Render into memory first. Writing to a Vec cannot fail, which separates
    // "the header could not be produced" from "the header could not be
    // written" — two different warnings, two different fixes.
    let mut bayt: Vec<u8> = Vec::new();
    rubat.write(&mut bayt);

    let masar = PathBuf::from(&jidhr).join("include").join("taarib.h");
    if let Ok(mawjud) = std::fs::read(&masar)
        && mawjud == bayt
    {
        return;
    }

    let natija = match masar.parent() {
        Some(mujallad) => {
            std::fs::create_dir_all(mujallad).and_then(|()| std::fs::write(&masar, &bayt))
        },
        None => std::fs::write(&masar, &bayt),
    };
    if let Err(sabab) = natija {
        // A read-only source tree is a real consumer, not an error state. The
        // committed header is the product; regeneration is a courtesy.
        println!(
            "cargo:warning=taarib-jisr: include/taarib.h could not be written ({sabab}); \
             the committed header is used as-is"
        );
    }
}
