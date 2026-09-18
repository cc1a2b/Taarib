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

    tahaqquq_badhra();
    tauri_build::build();
}

/// Refuses a release build whose embedded revocation seed the release anchor
/// cannot verify.
///
/// The seed in `assets/qaimat_sahb.json` is committed signed by the development
/// key, and `scripts/isdar.sh`'s `badhra` stage is what re-mints it under the
/// release key for the length of a release build. Build any other way — a bare
/// `cargo tauri build` with the anchor exported, which is the obvious shortcut
/// and the fast one — and the binary carries the release anchor over a seed the
/// development key signed. It compiles, it installs, it starts, and then every
/// launch answers `TAARIB-E-6303`: the revocation list does not verify against
/// the owner key. That is a build that is wrong about itself, and the person who
/// finds out is the user.
///
/// `asas_sahb` already makes exactly this check and already documents the
/// mismatch as meaning "the build itself is inconsistent". It made it at run
/// time. Here it is a failed build.
fn tahaqquq_badhra() {
    println!("cargo:rerun-if-env-changed=TAARIB_MIFTAH_ISDAR");
    println!("cargo:rerun-if-changed=../../../assets/qaimat_sahb.json");

    // Gated on the anchor's own identity, not on a feature flag. The release
    // build turns the feature on for `taarib-khatm`, not for this crate, so
    // `CARGO_FEATURE_ISDAR` is never set here — a gate on it reads as correct,
    // compiles, and checks nothing, which is the same class of mistake it was
    // written to catch. A development anchor signed the committed seed, so
    // there the two agree by construction and there is nothing to check.
    if !matches!(
        taarib_khatm::MIRSAT_MALIK.hawiya,
        taarib_khatm::HawiyatThiqa::Isdar
    ) {
        return;
    }

    let masar =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/qaimat_sahb.json");
    let Ok(bayt) = std::fs::read(&masar) else {
        panic!("the revocation seed is not at {}", masar.display());
    };
    let Ok(miftah) = taarib_khatm::MiftahAam::min_bayt(&taarib_khatm::MIRSAT_MALIK.miftah) else {
        panic!("TAARIB_MIFTAH_ISDAR is not a public key");
    };
    if let Err(khata) = taarib_aman::qaimat_sahb::QaimatSahb::min_bayt(&bayt, &miftah) {
        panic!(
            "this build carries the release anchor but the revocation seed at {} does not verify against it ({khata}). The seed is committed signed by the development key; the `badhra` stage of scripts/isdar.sh is what re-mints it under the release key. Build through that script rather than calling `cargo tauri build` directly, or the binary answers TAARIB-E-6303 on every launch.",
            masar.display()
        );
    }
}
