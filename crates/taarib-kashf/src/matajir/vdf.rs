//! VDF — ستيم يكتب كل شيء بصيغتين، وكلتاهما تُقرأ هنا.
//!
//! Valve's `KeyValues` format comes in two dialects and Taarib reads both from
//! scratch, in this crate, with no third-party parser underneath.
//!
//! **Why in-crate.** The ROADMAP settles this: binary VDF is undocumented,
//! versioned, and has changed shape more than once — `appinfo.vdf` alone is on
//! its third framing — and there is no maintained Rust crate that reads the
//! current one. Depending on a crate that half-works would mean the primary
//! discovery path on the primary platform breaks on a Steam client update, and
//! that is precisely the failure mode this product exists to avoid. Two file
//! formats is a small, bounded, well-understood cost; a broken library on
//! somebody else's release schedule is not.
//!
//! ## The text dialect
//!
//! `libraryfolders.vdf`, `appmanifest_*.acf`, `localconfig.vdf`, `config.vdf`,
//! `loginusers.vdf`. Nested `"key" "value"` and `"key" { ... }` pairs, quoted
//! and unquoted tokens, `//` comments, escape sequences, and the conditional
//! suffixes Valve's own writer sometimes emits (`"key" "value" [$WIN32]`).
//!
//! ## The binary dialect
//!
//! `appinfo.vdf` and `shortcuts.vdf`. A type-tagged stream: `0x00` opens a
//! nested map, `0x01` a string, `0x02` an `i32`, `0x07` a `u64`, `0x08` ends a
//! map — plus the several rarer tags Valve's writer still emits. `appinfo.vdf`
//! wraps that in its own outer framing, described on [`murur_appinfo`].
//!
//! ## Where this reader is tolerant, and where it is strict
//!
//! The rule is one sentence: **be tolerant where Valve's own parser is
//! tolerant, and strict wherever tolerance would silently lose data.** A reader
//! that quietly drops half a file is worse than one that says it cannot read
//! it, because the first produces a library that is wrong and the second
//! produces a diagnostic the user can act on.
//!
//! Tolerant, deliberately:
//!
//! - **Key lookup ignores ASCII case.** Steam writes `"AppState"` in one file
//!   and `"appstate"` in another, `"Valve"` in one client build and `"valve"`
//!   in the next. A case-sensitive reader finds half the games and no
//!   compatibility tools, which is the single easiest way to get this wrong.
//! - **Unquoted tokens** are accepted for both keys and values, as Valve does.
//! - **Several root nodes** in one file are accepted and returned side by side,
//!   rather than the first being kept and the rest discarded.
//! - **Duplicate keys** are preserved in file order; nothing is overwritten.
//!   [`QeemaVdf::bi_miftah`] returns the first, [`QeemaVdf::kul_bi_miftah`]
//!   returns them all.
//! - **A leading byte order mark** is skipped rather than becoming part of the
//!   first key.
//! - **An unknown escape** keeps both the backslash and the character that
//!   followed it, so a hand-edited `"C:\Users\..."` keeps its `U`.
//! - **Conditional suffixes** (`[$WIN32]`, `[!$X360]`, `[$WIN32||$LINUX]`) are
//!   parsed and discarded, and the entry they qualify is always kept. They are
//!   never evaluated: none of the files Taarib reads uses them to mean
//!   anything, and evaluating them would be a reader that deletes entries.
//! - **Binary string payloads decode lossily.** A single stray byte from a
//!   legacy code page inside one game's description must not remove that game
//!   from the library.
//! - **A binary stream that ends** at the outermost level without its final
//!   `0x08` is accepted; `shortcuts.vdf` is written both ways.
//!
//! Strict, equally deliberately:
//!
//! - An unterminated quoted string is an error, never a truncated value.
//! - A closing brace with nothing open, or an open brace still unclosed at end
//!   of file, is an error — both mean the tree would silently lose a subtree.
//! - A key with no value at end of file is an error.
//! - Every binary read is bounds-checked against the buffer and reports the
//!   byte offset it stopped at. There is no path in this module that can read
//!   past the end of a slice.
//! - An `appinfo.vdf` magic this build does not know is an error naming the
//!   magic it saw, never a guess at the framing.
//! - Nesting deeper than [`HADD_UMQ`] is an error. Both readers are iterative
//!   with an explicit stack, so a hostile file cannot overflow the real one,
//!   and the limit exists so that a corrupt file cannot make Taarib allocate
//!   without bound either.

use std::path::{Path, PathBuf};

use taarib_usus::khata::{Khata, Natija};

use crate::khata::KhataKashf;

/// The launcher every VDF file in this product belongs to, for the error that
/// names which catalogue would not read.
const MATJAR: &str = "steam";

/// How the source is named when bytes were handed in without a file behind
/// them — a value pulled from a database column, or a fragment under test.
const MASAR_DHAKIRA: &str = "<vdf>";

/// Deepest nesting either reader will follow.
///
/// Real Steam files reach seven or eight levels. Sixty-four is far above
/// anything Valve writes and far below anything that costs memory, so a file
/// that exceeds it is corrupt or hostile, and is reported as such.
pub const HADD_UMQ: usize = 64;

/// `appinfo.vdf` framing, first version Taarib knows.
///
/// No string table; every key inside every app is written inline.
pub const SIHR_APPINFO_27: u32 = 0x0756_4427;

/// `appinfo.vdf` framing with a second SHA-1 per app.
///
/// Identical to [`SIHR_APPINFO_27`] except that each app header carries a
/// second twenty-byte digest, over the binary form, after the change number.
pub const SIHR_APPINFO_28: u32 = 0x0756_4428;

/// `appinfo.vdf` framing with a string table at a footer offset.
///
/// The app headers are unchanged from [`SIHR_APPINFO_28`]; what changes is
/// inside each app's data, where every **key name** is a four-byte index into
/// one shared table instead of an inline string. String *values* stay inline.
/// With a quarter of a million apps repeating the same few hundred key names,
/// this is most of why the file stopped growing.
pub const SIHR_APPINFO_29: u32 = 0x0756_4429;

/// Every `appinfo.vdf` framing this build reads.
pub const SIHR_APPINFO: [u32; 3] = [SIHR_APPINFO_27, SIHR_APPINFO_28, SIHR_APPINFO_29];

/// Binary tag: a nested map, terminated by [`NAW_NIHAYA`].
pub const NAW_KAIN: u8 = 0x00;
/// Binary tag: a NUL-terminated UTF-8 string.
pub const NAW_NASS: u8 = 0x01;
/// Binary tag: a little-endian `i32`.
pub const NAW_SAHIH32: u8 = 0x02;
/// Binary tag: a little-endian `f32`. Kept as its text form, because nothing
/// in Steam's own files uses it and a float in a discovery record would only
/// ever be printed.
pub const NAW_ASHARI: u8 = 0x03;
/// Binary tag: a pointer, written as an `i32`. Meaningless off the machine
/// that wrote it; kept as a number so nothing is lost.
pub const NAW_MUASHIR: u8 = 0x04;
/// Binary tag: a NUL-terminated UTF-16 little-endian string.
pub const NAW_NASS_AREED: u8 = 0x05;
/// Binary tag: a colour, written as an `i32`.
pub const NAW_LAWN: u8 = 0x06;
/// Binary tag: a little-endian `u64`.
pub const NAW_KABIR: u8 = 0x07;
/// Binary tag: end of the current map.
pub const NAW_NIHAYA: u8 = 0x08;
/// Binary tag: a little-endian `i64`, kept as its unsigned bit pattern.
pub const NAW_SAHIH64: u8 = 0x0A;
/// Binary tag: the alternate end-of-map marker Valve's newer writer emits in
/// some files. Accepted wherever [`NAW_NIHAYA`] is.
pub const NAW_NIHAYA_BADILA: u8 = 0x0B;

/// Bytes of an `appinfo.vdf` app header after the size field, version 27:
/// info state, last updated, access token, text SHA-1, change number.
const TUL_TARWISA_27: usize = 4 + 4 + 8 + 20 + 4;

/// The same for versions 28 and 29, which add the binary SHA-1.
const TUL_TARWISA_28: usize = TUL_TARWISA_27 + 20;

// ---------------------------------------------------------------------------
// The value tree
// ---------------------------------------------------------------------------

/// One VDF value.
///
/// The text dialect has exactly two of these — a string or a map — because it
/// stores every number as text; `"StateFlags" "4"` is a string on disk.
/// [`QeemaVdf::raqm`] parses those, so a caller never has to care which dialect
/// a value came from.
#[derive(Debug, Clone, PartialEq)]
pub enum QeemaVdf {
    /// A string. Every value in a text VDF file is one of these.
    Nass(String),
    /// A thirty-two bit integer from the binary dialect.
    Raqm(i32),
    /// A sixty-four bit value from the binary dialect, kept unsigned. A signed
    /// `i64` is stored as its own bit pattern rather than being widened or
    /// clamped, so nothing is lost on the way in.
    Kabir(u64),
    /// A map, in file order, with duplicate keys preserved.
    Kain(Vec<(String, Self)>),
}

impl Default for QeemaVdf {
    fn default() -> Self {
        Self::Kain(Vec::new())
    }
}

impl QeemaVdf {
    /// Follows a chain of keys down the tree, ignoring ASCII case at every
    /// step.
    ///
    /// An empty path returns the value itself, which is what makes this safe to
    /// call with a prefix computed elsewhere.
    #[must_use]
    pub fn bi_masar(&self, masar: &[&str]) -> Option<&Self> {
        let mut hali = self;
        for miftah in masar {
            hali = hali.bi_miftah(miftah)?;
        }
        Some(hali)
    }

    /// The first child with this key, ignoring ASCII case.
    #[must_use]
    pub fn bi_miftah(&self, miftah: &str) -> Option<&Self> {
        match self {
            Self::Kain(abna) => abna
                .iter()
                .find(|(ism, _)| ism.eq_ignore_ascii_case(miftah))
                .map(|(_, qeema)| qeema),
            Self::Nass(_) | Self::Raqm(_) | Self::Kabir(_) => None,
        }
    }

    /// Every child with this key, in file order.
    ///
    /// Valve's conditional suffixes are the reason duplicates exist at all:
    /// `"key" "a" [$WIN32]` and `"key" "b" [$LINUX]` are two entries under one
    /// name, and a reader that kept only one would be choosing a platform on
    /// the user's behalf.
    #[must_use]
    pub fn kul_bi_miftah(&self, miftah: &str) -> Vec<&Self> {
        match self {
            Self::Kain(abna) => abna
                .iter()
                .filter(|(ism, _)| ism.eq_ignore_ascii_case(miftah))
                .map(|(_, qeema)| qeema)
                .collect(),
            Self::Nass(_) | Self::Raqm(_) | Self::Kabir(_) => Vec::new(),
        }
    }

    /// The value as text, when it is text.
    #[must_use]
    pub const fn nass(&self) -> Option<&str> {
        match self {
            Self::Nass(qeema) => Some(qeema.as_str()),
            Self::Raqm(_) | Self::Kabir(_) | Self::Kain(_) => None,
        }
    }

    /// The value as a signed integer.
    ///
    /// Parses a string, because the text dialect has no numbers: every count,
    /// flag word and timestamp in an `.acf` file is written as digits inside
    /// quotes. A `u64` too large for `i64` yields `None` rather than wrapping.
    #[must_use]
    pub fn raqm(&self) -> Option<i64> {
        match self {
            Self::Raqm(qeema) => Some(i64::from(*qeema)),
            Self::Kabir(qeema) => i64::try_from(*qeema).ok(),
            Self::Nass(qeema) => qeema.trim().parse::<i64>().ok(),
            Self::Kain(_) => None,
        }
    }

    /// The value as an unsigned integer, parsing a string for the same reason
    /// [`QeemaVdf::raqm`] does.
    #[must_use]
    pub fn kabir(&self) -> Option<u64> {
        match self {
            Self::Kabir(qeema) => Some(*qeema),
            Self::Raqm(qeema) => u64::try_from(*qeema).ok(),
            Self::Nass(qeema) => qeema.trim().parse::<u64>().ok(),
            Self::Kain(_) => None,
        }
    }

    /// The children, when this is a map.
    #[must_use]
    pub const fn kain(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Kain(abna) => Some(abna.as_slice()),
            Self::Nass(_) | Self::Raqm(_) | Self::Kabir(_) => None,
        }
    }

    /// Text at a path, in one call.
    #[must_use]
    pub fn nass_bi_masar(&self, masar: &[&str]) -> Option<&str> {
        self.bi_masar(masar)?.nass()
    }

    /// A signed integer at a path, in one call.
    #[must_use]
    pub fn raqm_bi_masar(&self, masar: &[&str]) -> Option<i64> {
        self.bi_masar(masar)?.raqm()
    }

    /// An unsigned integer at a path, in one call.
    #[must_use]
    pub fn kabir_bi_masar(&self, masar: &[&str]) -> Option<u64> {
        self.bi_masar(masar)?.kabir()
    }

    /// The children at a path, in one call.
    #[must_use]
    pub fn kain_bi_masar(&self, masar: &[&str]) -> Option<&[(String, Self)]> {
        self.bi_masar(masar)?.kain()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Builds the one error this module can produce: a catalogue whose framing is
/// wrong, named, explained, and located.
fn talif(masar: &Path, tafsil: String, mawdi: Option<usize>) -> Khata {
    Khata::min_tafsir(&KhataKashf::TarwisatFahrasTalifa {
        matjar: MATJAR,
        masar: masar.to_path_buf(),
        tafsil,
        mawdi: mawdi.and_then(|q| u64::try_from(q).ok()),
    })
}

/// The same, with the line number worked out from the offset — cheap, because
/// it only ever runs on the failure path, and worth it because "line 412" is
/// something a user can open a file to.
fn talif_nass(masar: &Path, bayt: &[u8], mawdi: usize, tafsil: &str) -> Khata {
    // Splitting on the newline yields one more segment than there are newlines,
    // which is already the 1-based line number.
    let satr = bayt
        .get(..mawdi.min(bayt.len()))
        .map_or(1, |sabiq| sabiq.split(|b| *b == b'\n').count());
    talif(
        masar,
        format!("{tafsil} (line {satr}, byte {mawdi})"),
        Some(mawdi),
    )
}

/// A path standing in for bytes that came from somewhere other than a file.
fn masar_dhakira() -> PathBuf {
    PathBuf::from(MASAR_DHAKIRA)
}

// ---------------------------------------------------------------------------
// The text dialect
// ---------------------------------------------------------------------------

/// Reads a text VDF document.
///
/// The result is always a [`QeemaVdf::Kain`] holding the document's roots, so
/// `iqra_nassi(nass)?.bi_masar(&["AppState", "name"])` works on an
/// `appmanifest_*.acf` without the caller unwrapping a layer first.
///
/// # Errors
///
/// [`KhataKashf::TarwisatFahrasTalifa`] for an unterminated string, an
/// unbalanced brace, a key with no value, or nesting past [`HADD_UMQ`], with
/// the line and byte offset in the message. Use [`iqra_nassi_bi_masar`] when
/// the text came from a file, so the error names it.
pub fn iqra_nassi(nass: &str) -> Natija<QeemaVdf> {
    iqra_nassi_bi_masar(&masar_dhakira(), nass)
}

/// Reads a text VDF document, naming the file it came from in any error.
///
/// # Errors
///
/// As [`iqra_nassi`].
pub fn iqra_nassi_bi_masar(masar: &Path, nass: &str) -> Natija<QeemaVdf> {
    let bayt = nass.as_bytes();
    let mut qari = QariNass { bayt, mawqi: 0 };
    qari.takhatti_alama();

    let mut hali: Vec<(String, QeemaVdf)> = Vec::new();
    let mut abaa: Vec<(String, usize, Vec<(String, QeemaVdf)>)> = Vec::new();

    while let Some((mawdi, wahda)) = qari.wahda(masar)? {
        match wahda {
            Wahda::Ighlaq => {
                let Some((miftah, _, mut walid)) = abaa.pop() else {
                    return Err(talif_nass(
                        masar,
                        bayt,
                        mawdi,
                        "a closing brace with no matching open brace",
                    ));
                };
                walid.push((miftah, QeemaVdf::Kain(std::mem::take(&mut hali))));
                hali = walid;
            },
            Wahda::Fath => {
                return Err(talif_nass(
                    masar,
                    bayt,
                    mawdi,
                    "an open brace where a key was expected",
                ));
            },
            Wahda::Kalima(miftah) => {
                qari.takhatti_shart();
                let Some((mawdi_qeema, baad)) = qari.wahda(masar)? else {
                    return Err(talif_nass(
                        masar,
                        bayt,
                        mawdi,
                        &format!("the key \"{miftah}\" has no value before the end of the file"),
                    ));
                };
                match baad {
                    Wahda::Fath => {
                        if abaa.len() >= HADD_UMQ {
                            return Err(talif_nass(
                                masar,
                                bayt,
                                mawdi_qeema,
                                &format!("nested deeper than {HADD_UMQ} levels"),
                            ));
                        }
                        abaa.push((miftah, mawdi, std::mem::take(&mut hali)));
                    },
                    Wahda::Kalima(qeema) => {
                        qari.takhatti_shart();
                        hali.push((miftah, QeemaVdf::Nass(qeema)));
                    },
                    Wahda::Ighlaq => {
                        return Err(talif_nass(
                            masar,
                            bayt,
                            mawdi_qeema,
                            &format!(
                                "the key \"{miftah}\" is followed by a closing brace instead of a \
                                 value"
                            ),
                        ));
                    },
                }
            },
        }
    }

    if let Some((miftah, mawdi, _)) = abaa.pop() {
        return Err(talif_nass(
            masar,
            bayt,
            mawdi,
            &format!(
                "the block opened for \"{miftah}\" is never closed ({} still open)",
                abaa.len() + 1
            ),
        ));
    }

    Ok(QeemaVdf::Kain(hali))
}

/// One thing the text tokenizer can hand back.
enum Wahda {
    /// `{`
    Fath,
    /// `}`
    Ighlaq,
    /// A key or a value, already unquoted and unescaped.
    Kalima(String),
}

/// A cursor over the bytes of a text VDF document.
///
/// Bytes rather than characters, because every delimiter the format uses is
/// ASCII and no ASCII byte can appear inside a multi-byte UTF-8 sequence — so
/// splitting on them can never cut a character in half, and offsets in error
/// messages line up with what a hex editor shows.
struct QariNass<'a> {
    bayt: &'a [u8],
    mawqi: usize,
}

impl QariNass<'_> {
    /// Skips a leading byte order mark, once, at the very start.
    fn takhatti_alama(&mut self) {
        if self.bayt.get(..3) == Some(&[0xEF, 0xBB, 0xBF]) {
            self.mawqi = 3;
        }
    }

    /// Skips whitespace and `//` comments until something else is next.
    fn takhatti(&mut self) {
        loop {
            match self.bayt.get(self.mawqi) {
                Some(b) if b.is_ascii_whitespace() => self.mawqi += 1,
                Some(b'/') if self.bayt.get(self.mawqi + 1) == Some(&b'/') => {
                    while let Some(b) = self.bayt.get(self.mawqi) {
                        if *b == b'\n' {
                            break;
                        }
                        self.mawqi += 1;
                    }
                },
                _ => return,
            }
        }
    }

    /// Consumes a conditional suffix if one is next.
    ///
    /// Valve recognises a token beginning with `[` as a conditional wherever a
    /// value may appear, so this reader does too: everything up to the closing
    /// bracket is consumed and thrown away, and the entry it qualified stays.
    fn takhatti_shart(&mut self) {
        self.takhatti();
        if self.bayt.get(self.mawqi) != Some(&b'[') {
            return;
        }
        while let Some(b) = self.bayt.get(self.mawqi) {
            self.mawqi += 1;
            if *b == b']' {
                return;
            }
        }
    }

    /// The next token, with the offset it started at, or `None` at end of file.
    fn wahda(&mut self, masar: &Path) -> Natija<Option<(usize, Wahda)>> {
        self.takhatti();
        let bidaya = self.mawqi;
        let Some(&awwal) = self.bayt.get(bidaya) else {
            return Ok(None);
        };
        match awwal {
            b'{' => {
                self.mawqi += 1;
                Ok(Some((bidaya, Wahda::Fath)))
            },
            b'}' => {
                self.mawqi += 1;
                Ok(Some((bidaya, Wahda::Ighlaq)))
            },
            b'"' => Ok(Some((bidaya, Wahda::Kalima(self.muqtabas(masar)?)))),
            _ => Ok(Some((bidaya, Wahda::Kalima(self.mujarrad(masar)?)))),
        }
    }

    /// Reads a quoted token, resolving escapes.
    fn muqtabas(&mut self, masar: &Path) -> Natija<String> {
        let bidaya = self.mawqi;
        self.mawqi += 1;
        let mut kharij: Vec<u8> = Vec::new();
        loop {
            let Some(&b) = self.bayt.get(self.mawqi) else {
                return Err(talif_nass(
                    masar,
                    self.bayt,
                    bidaya,
                    "a quoted string is never closed",
                ));
            };
            self.mawqi += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&tali) = self.bayt.get(self.mawqi) else {
                        return Err(talif_nass(
                            masar,
                            self.bayt,
                            self.mawqi,
                            "an escape sequence runs off the end of the file",
                        ));
                    };
                    self.mawqi += 1;
                    match tali {
                        b'n' => kharij.push(b'\n'),
                        b't' => kharij.push(b'\t'),
                        b'r' => kharij.push(b'\r'),
                        b'\\' => kharij.push(b'\\'),
                        b'"' => kharij.push(b'"'),
                        // Unknown escape: both bytes are kept. Dropping the
                        // backslash here is how a hand-written `"C:\Users"`
                        // silently becomes `"C:sers"`.
                        _ => {
                            kharij.push(b'\\');
                            kharij.push(tali);
                        },
                    }
                },
                _ => kharij.push(b),
            }
        }
        String::from_utf8(kharij).map_err(|_| {
            talif_nass(
                masar,
                self.bayt,
                bidaya,
                "a quoted string is not valid UTF-8",
            )
        })
    }

    /// Reads an unquoted token: everything up to whitespace, a brace, a quote,
    /// or the start of a comment. Escapes are not resolved here, matching
    /// Valve's own reader.
    fn mujarrad(&mut self, masar: &Path) -> Natija<String> {
        let bidaya = self.mawqi;
        while let Some(&b) = self.bayt.get(self.mawqi) {
            if b.is_ascii_whitespace() || matches!(b, b'{' | b'}' | b'"') {
                break;
            }
            if b == b'/' && self.bayt.get(self.mawqi + 1) == Some(&b'/') {
                break;
            }
            self.mawqi += 1;
        }
        let qita = self
            .bayt
            .get(bidaya..self.mawqi)
            .ok_or_else(|| talif_nass(masar, self.bayt, bidaya, "a token runs past the end"))?;
        String::from_utf8(qita.to_vec()).map_err(|_| {
            talif_nass(
                masar,
                self.bayt,
                bidaya,
                "an unquoted token is not valid UTF-8",
            )
        })
    }
}

// ---------------------------------------------------------------------------
// The binary dialect
// ---------------------------------------------------------------------------

/// Reads a binary VDF document.
///
/// `shortcuts.vdf` is exactly this and nothing else: one map named `shortcuts`
/// whose children are the entries, so
/// `iqra_thunai(&bayt)?.bi_masar(&["shortcuts"])` reaches them.
///
/// # Errors
///
/// [`KhataKashf::TarwisatFahrasTalifa`] naming the byte offset for an unknown
/// type tag, a string that is never terminated, a read that would run past the
/// end of the buffer, or nesting past [`HADD_UMQ`].
pub fn iqra_thunai(bayt: &[u8]) -> Natija<QeemaVdf> {
    iqra_thunai_bi_masar(&masar_dhakira(), bayt)
}

/// Reads a binary VDF document, naming the file it came from in any error.
///
/// # Errors
///
/// As [`iqra_thunai`].
pub fn iqra_thunai_bi_masar(masar: &Path, bayt: &[u8]) -> Natija<QeemaVdf> {
    let mut qari = QariThunai {
        bayt,
        mawqi: 0,
        asas: 0,
    };
    Ok(QeemaVdf::Kain(iqra_kain(&mut qari, masar, None)?))
}

/// Reads one map's worth of key/value pairs, iteratively.
///
/// `jadwal` is `appinfo.vdf` version 29's string table: when it is present,
/// every key is a four-byte index into it rather than an inline string. Values
/// are unaffected.
fn iqra_kain(
    qari: &mut QariThunai<'_>,
    masar: &Path,
    jadwal: Option<&[String]>,
) -> Natija<Vec<(String, QeemaVdf)>> {
    let mut hali: Vec<(String, QeemaVdf)> = Vec::new();
    let mut abaa: Vec<(String, Vec<(String, QeemaVdf)>)> = Vec::new();

    loop {
        if qari.intaha() {
            // A stream that stops at the outermost level is complete: Steam
            // writes the final terminator in some files and not in others.
            if abaa.is_empty() {
                return Ok(hali);
            }
            return Err(qari.khata(masar, "the stream ends inside an unclosed map"));
        }

        let mawdi = qari.mawqi;
        let naw = qari.bayt_wahid(masar)?;
        match naw {
            NAW_NIHAYA | NAW_NIHAYA_BADILA => match abaa.pop() {
                Some((miftah, mut walid)) => {
                    walid.push((miftah, QeemaVdf::Kain(std::mem::take(&mut hali))));
                    hali = walid;
                },
                None => return Ok(hali),
            },
            NAW_KAIN => {
                let miftah = qari.miftah(masar, jadwal)?;
                if abaa.len() >= HADD_UMQ {
                    return Err(qari.khata_fi(
                        masar,
                        mawdi,
                        &format!("nested deeper than {HADD_UMQ} levels"),
                    ));
                }
                abaa.push((miftah, std::mem::take(&mut hali)));
            },
            NAW_NASS => {
                let miftah = qari.miftah(masar, jadwal)?;
                let qeema = qari.nass_muntahi(masar)?;
                hali.push((miftah, QeemaVdf::Nass(qeema)));
            },
            NAW_NASS_AREED => {
                let miftah = qari.miftah(masar, jadwal)?;
                let qeema = qari.nass_areed(masar)?;
                hali.push((miftah, QeemaVdf::Nass(qeema)));
            },
            NAW_SAHIH32 | NAW_MUASHIR | NAW_LAWN => {
                let miftah = qari.miftah(masar, jadwal)?;
                let qeema = qari.sahih32(masar)?;
                hali.push((miftah, QeemaVdf::Raqm(qeema)));
            },
            NAW_ASHARI => {
                let miftah = qari.miftah(masar, jadwal)?;
                let qeema = qari.ashari(masar)?;
                hali.push((miftah, QeemaVdf::Nass(format!("{qeema}"))));
            },
            NAW_KABIR | NAW_SAHIH64 => {
                let miftah = qari.miftah(masar, jadwal)?;
                let qeema = qari.kabir(masar)?;
                hali.push((miftah, QeemaVdf::Kabir(qeema)));
            },
            _ => {
                return Err(qari.khata_fi(
                    masar,
                    mawdi,
                    &format!("unknown binary VDF type tag {naw:#04x}"),
                ));
            },
        }
    }
}

/// A bounds-checked cursor over a binary VDF stream.
///
/// `asas` is the offset of `bayt` within the file it was cut from, so that an
/// app entry parsed out of the middle of a 300 MB `appinfo.vdf` still reports
/// absolute file offsets. Every accessor goes through `slice::get`; there is no
/// indexing in this type, so no input can make it read out of bounds.
struct QariThunai<'a> {
    bayt: &'a [u8],
    mawqi: usize,
    asas: usize,
}

impl<'a> QariThunai<'a> {
    /// Whether the cursor is at or past the end.
    const fn intaha(&self) -> bool {
        self.mawqi >= self.bayt.len()
    }

    /// An error at the current position.
    fn khata(&self, masar: &Path, tafsil: &str) -> Khata {
        self.khata_fi(masar, self.mawqi, tafsil)
    }

    /// An error at a remembered position.
    fn khata_fi(&self, masar: &Path, mawdi: usize, tafsil: &str) -> Khata {
        let mutlaq = self.asas.saturating_add(mawdi);
        talif(masar, format!("{tafsil} at byte {mutlaq}"), Some(mutlaq))
    }

    /// Takes `adad` bytes, or fails naming where it ran out.
    ///
    /// The slice borrows the underlying data, not the cursor, so a caller can
    /// hold it and keep reading.
    fn khudh(&mut self, masar: &Path, adad: usize) -> Natija<&'a [u8]> {
        let nihaya = self
            .mawqi
            .checked_add(adad)
            .ok_or_else(|| self.khata(masar, "a length overflows the address space"))?;
        if nihaya > self.bayt.len() {
            return Err(self.khata_fi(
                masar,
                self.mawqi,
                &format!(
                    "a {adad}-byte read runs past the end of the data, which holds {} more byte(s)",
                    self.bayt.len().saturating_sub(self.mawqi)
                ),
            ));
        }
        let qita = self
            .bayt
            .get(self.mawqi..nihaya)
            .ok_or_else(|| self.khata(masar, "a read runs past the end of the data"))?;
        self.mawqi = nihaya;
        Ok(qita)
    }

    /// One byte.
    fn bayt_wahid(&mut self, masar: &Path) -> Natija<u8> {
        let qita = self.khudh(masar, 1)?;
        qita.first()
            .copied()
            .ok_or_else(|| self.khata(masar, "a byte read produced nothing"))
    }

    /// A little-endian `u32`.
    fn raqm32(&mut self, masar: &Path) -> Natija<u32> {
        let qita = self.khudh(masar, 4)?;
        <[u8; 4]>::try_from(qita)
            .map(u32::from_le_bytes)
            .map_err(|_| self.khata(masar, "a four-byte read produced the wrong width"))
    }

    /// A little-endian `i32`.
    fn sahih32(&mut self, masar: &Path) -> Natija<i32> {
        let qita = self.khudh(masar, 4)?;
        <[u8; 4]>::try_from(qita)
            .map(i32::from_le_bytes)
            .map_err(|_| self.khata(masar, "a four-byte read produced the wrong width"))
    }

    /// A little-endian `f32`.
    fn ashari(&mut self, masar: &Path) -> Natija<f32> {
        let qita = self.khudh(masar, 4)?;
        <[u8; 4]>::try_from(qita)
            .map(f32::from_le_bytes)
            .map_err(|_| self.khata(masar, "a four-byte read produced the wrong width"))
    }

    /// A little-endian sixty-four bit value, kept as its unsigned bit pattern
    /// so that an `i64` loses nothing on the way in.
    fn kabir(&mut self, masar: &Path) -> Natija<u64> {
        let qita = self.khudh(masar, 8)?;
        <[u8; 8]>::try_from(qita)
            .map(u64::from_le_bytes)
            .map_err(|_| self.khata(masar, "an eight-byte read produced the wrong width"))
    }

    /// A NUL-terminated UTF-8 string.
    ///
    /// Decoded lossily: one byte of legacy code page inside one game's name
    /// must cost that name a character, not cost the user the game.
    fn nass_muntahi(&mut self, masar: &Path) -> Natija<String> {
        let bidaya = self.mawqi;
        let mut nihaya = bidaya;
        loop {
            let Some(&b) = self.bayt.get(nihaya) else {
                return Err(self.khata_fi(
                    masar,
                    bidaya,
                    "a string is never terminated before the end of the data",
                ));
            };
            if b == 0 {
                break;
            }
            nihaya += 1;
        }
        let qita = self.bayt.get(bidaya..nihaya).ok_or_else(|| {
            self.khata_fi(masar, bidaya, "a string runs past the end of the data")
        })?;
        let nass = String::from_utf8_lossy(qita).into_owned();
        self.mawqi = nihaya + 1;
        Ok(nass)
    }

    /// A NUL-terminated UTF-16 little-endian string, decoded lossily for the
    /// same reason.
    fn nass_areed(&mut self, masar: &Path) -> Natija<String> {
        let bidaya = self.mawqi;
        let mut wahdat: Vec<u16> = Vec::new();
        loop {
            let zawj = self.khudh(masar, 2).map_err(|_| {
                self.khata_fi(
                    masar,
                    bidaya,
                    "a wide string is never terminated before the end of the data",
                )
            })?;
            let wahda = <[u8; 2]>::try_from(zawj)
                .map(u16::from_le_bytes)
                .map_err(|_| {
                    self.khata_fi(masar, bidaya, "a two-byte read produced the wrong width")
                })?;
            if wahda == 0 {
                return Ok(String::from_utf16_lossy(&wahdat));
            }
            wahdat.push(wahda);
        }
    }

    /// A key: an index into the string table when there is one, otherwise an
    /// inline NUL-terminated string.
    fn miftah(&mut self, masar: &Path, jadwal: Option<&[String]>) -> Natija<String> {
        match jadwal {
            Some(nusus) => {
                let mawdi = self.mawqi;
                let fahras = self.raqm32(masar)?;
                let mawqi_nass = usize::try_from(fahras).map_err(|_| {
                    self.khata_fi(
                        masar,
                        mawdi,
                        "a string table index does not fit in this address space",
                    )
                })?;
                nusus.get(mawqi_nass).cloned().ok_or_else(|| {
                    self.khata_fi(
                        masar,
                        mawdi,
                        &format!(
                            "string table index {fahras} is past the end of the table, which holds \
                             {} entries",
                            nusus.len()
                        ),
                    )
                })
            },
            None => self.nass_muntahi(masar),
        }
    }
}

// ---------------------------------------------------------------------------
// appinfo.vdf
// ---------------------------------------------------------------------------

/// One app out of `appinfo.vdf`.
///
/// The header carries more than this — an info state, an access token, a SHA-1
/// over the text form and, from version 28, a second over the binary form.
/// Those are read, their lengths are enforced, and they are then dropped:
/// nothing in discovery consults them, and keeping a digest Taarib never
/// verifies would be a field that lies about being meaningful.
#[derive(Debug, Clone, PartialEq)]
pub struct MadkhalAppinfo {
    /// The Steam application identifier.
    pub app: u32,
    /// When Steam last refreshed this app's metadata, Unix seconds. This is
    /// metadata freshness, not the install's `LastUpdated`.
    pub akhir_tahdith: u32,
    /// The PICS change number this entry was written at.
    pub raqm_taghyeer: u32,
    /// The app's data.
    ///
    /// A map with a single child named `appinfo`, exactly as the file stores
    /// it, so a lookup reads
    /// `bayanat.bi_masar(&["appinfo", "common", "name"])`. The wrapper is kept
    /// rather than unwrapped because it is what is on disk, and a reader that
    /// silently reshapes its input makes every path in every caller a guess.
    pub bayanat: QeemaVdf,
}

/// Reads every app in `appinfo.vdf`, strictly.
///
/// Any entry whose body will not parse fails the whole call. That is the right
/// contract for a caller that wants the file or nothing; a scan wants neither,
/// and uses [`murur_appinfo`] so that one unreadable app costs one app.
///
/// Be aware of the size: `appinfo.vdf` is a few hundred megabytes on a mature
/// account, and this materialises every app's tree at once. [`murur_appinfo`]
/// exists so the scanner never has to.
///
/// # Errors
///
/// [`KhataKashf::TarwisatFahrasTalifa`] for an unknown magic, a truncated
/// header, a truncated entry, a string table that does not fit, or any app body
/// that will not parse — always naming the byte offset it stopped at.
pub fn iqra_appinfo(bayt: &[u8]) -> Natija<Vec<MadkhalAppinfo>> {
    iqra_appinfo_bi_masar(&masar_dhakira(), bayt)
}

/// Reads every app in `appinfo.vdf`, naming the file it came from in any error.
///
/// # Errors
///
/// As [`iqra_appinfo`].
pub fn iqra_appinfo_bi_masar(masar: &Path, bayt: &[u8]) -> Natija<Vec<MadkhalAppinfo>> {
    let mut madakhil = Vec::new();
    let mut khata_madkhal = None;
    murur_appinfo(masar, bayt, &mut |natija| match natija {
        Ok(madkhal) => madakhil.push(madkhal),
        Err(khata) => {
            if khata_madkhal.is_none() {
                khata_madkhal = Some(khata);
            }
        },
    })?;
    khata_madkhal.map_or(Ok(madakhil), Err)
}

/// One pass over `appinfo.vdf`, handing each app to a visitor and dropping it.
///
/// This is the form a scan uses. The file is read once, each entry is projected
/// into whatever small record the caller keeps, and the entry's tree is freed
/// before the next one is built — which is the difference between a few
/// megabytes of working set and several gigabytes.
///
/// ## The framing, exactly as implemented
///
/// ```text
/// u32   magic          0x07564427 | 0x07564428 | 0x07564429
/// u32   universe
/// i64   string table offset            only when magic == 0x07564429
///
/// then, repeating until an app id of zero:
///   u32   app id                       0 terminates the list
///   u32   size                         bytes that follow this field, for this app
///   u32   info state          ─┐
///   u32   last updated         │
///   u64   access token         ├─ 40 bytes on 0x07564427
///   [20]  SHA-1 of the text form
///   u32   change number       ─┘
///   [20]  SHA-1 of the binary form      only on 0x07564428 and 0x07564429
///                                       ─ making the header 60 bytes there
///   ...   binary VDF, size - header bytes of it
///
/// and at the string table offset, on 0x07564429 only:
///   u32   count
///   count × NUL-terminated UTF-8 strings
/// ```
///
/// The size field is authoritative: the cursor advances by it whatever the
/// body's own reader did, so a single malformed app costs that app and the
/// sweep continues at the next one. A framing failure — an unknown magic, a
/// size that runs past the end of the file, a string table that is not there —
/// is returned, because after one of those there is no next entry to find.
///
/// # Errors
///
/// [`KhataKashf::TarwisatFahrasTalifa`] for a framing failure, naming the byte
/// offset. Per-app failures are handed to `zair` as `Err` instead.
pub fn murur_appinfo(
    masar: &Path,
    bayt: &[u8],
    zair: &mut dyn FnMut(Natija<MadkhalAppinfo>),
) -> Natija<()> {
    let mut tarwisa = QariThunai {
        bayt,
        mawqi: 0,
        asas: 0,
    };
    let sihr = tarwisa.raqm32(masar).map_err(|_| {
        talif(
            masar,
            "the file is too short to hold an appinfo.vdf header".to_owned(),
            Some(0),
        )
    })?;
    if !SIHR_APPINFO.contains(&sihr) {
        return Err(talif(
            masar,
            format!(
                "unknown appinfo.vdf magic {sihr:#010x}; this build reads \
                 {SIHR_APPINFO_27:#010x}, {SIHR_APPINFO_28:#010x} and {SIHR_APPINFO_29:#010x}"
            ),
            Some(0),
        ));
    }
    let _universe = tarwisa.raqm32(masar)?;

    let jadwal = if sihr == SIHR_APPINFO_29 {
        let mawdi_izaha = tarwisa.mawqi;
        let khaam = tarwisa.kabir(masar)?;
        let izaha = i64::from_le_bytes(khaam.to_le_bytes());
        Some(jadwal_nusus(masar, bayt, izaha, mawdi_izaha)?)
    } else {
        None
    };

    let tul_tarwisa = if sihr == SIHR_APPINFO_27 {
        TUL_TARWISA_27
    } else {
        TUL_TARWISA_28
    };
    let mut mawqi = tarwisa.mawqi;

    loop {
        let mut ras = QariThunai {
            bayt,
            mawqi,
            asas: 0,
        };
        let app = ras.raqm32(masar).map_err(|_| {
            talif(
                masar,
                "the entry list ends without its terminating zero app id, so the file is truncated"
                    .to_owned(),
                Some(mawqi),
            )
        })?;
        if app == 0 {
            return Ok(());
        }
        let hajm = ras.raqm32(masar)?;
        let bidayat_jism = ras.mawqi;
        let hajm_jism = usize::try_from(hajm).map_err(|_| {
            talif(
                masar,
                format!("app {app} declares a size that does not fit"),
                Some(mawqi),
            )
        })?;
        if hajm_jism < tul_tarwisa {
            return Err(talif(
                masar,
                format!(
                    "app {app} declares {hajm_jism} bytes, fewer than the {tul_tarwisa}-byte \
                     header this framing requires"
                ),
                Some(mawqi),
            ));
        }
        let nihaya = bidayat_jism.checked_add(hajm_jism).ok_or_else(|| {
            talif(
                masar,
                format!("app {app} declares a size that overflows"),
                Some(mawqi),
            )
        })?;
        let Some(jism) = bayt.get(bidayat_jism..nihaya) else {
            return Err(talif(
                masar,
                format!(
                    "app {app} declares {hajm_jism} bytes but only {} remain in the file, so it is \
                     truncated",
                    bayt.len().saturating_sub(bidayat_jism)
                ),
                Some(bidayat_jism),
            ));
        };

        zair(madkhal_wahid(
            masar,
            app,
            jism,
            bidayat_jism,
            tul_tarwisa,
            jadwal.as_deref(),
        ));
        mawqi = nihaya;
    }
}

/// Parses one app's body: the fixed header, then the binary VDF after it.
fn madkhal_wahid(
    masar: &Path,
    app: u32,
    jism: &[u8],
    asas: usize,
    tul_tarwisa: usize,
    jadwal: Option<&[String]>,
) -> Natija<MadkhalAppinfo> {
    let mut qari = QariThunai {
        bayt: jism,
        mawqi: 0,
        asas,
    };
    let _hala = qari.raqm32(masar)?;
    let akhir_tahdith = qari.raqm32(masar)?;
    let _ramz_wusul = qari.kabir(masar)?;
    let _basmat_nass = qari.khudh(masar, 20)?;
    let raqm_taghyeer = qari.raqm32(masar)?;
    if tul_tarwisa == TUL_TARWISA_28 {
        let _basmat_thunai = qari.khudh(masar, 20)?;
    }

    let bayanat_bayt = jism.get(tul_tarwisa..).ok_or_else(|| {
        talif(
            masar,
            format!("app {app} has no data after its header"),
            Some(asas),
        )
    })?;
    let mut bayanat_qari = QariThunai {
        bayt: bayanat_bayt,
        mawqi: 0,
        asas: asas.saturating_add(tul_tarwisa),
    };
    let bayanat = QeemaVdf::Kain(iqra_kain(&mut bayanat_qari, masar, jadwal)?);

    Ok(MadkhalAppinfo {
        app,
        akhir_tahdith,
        raqm_taghyeer,
        bayanat,
    })
}

/// Reads version 29's footer string table.
fn jadwal_nusus(masar: &Path, bayt: &[u8], izaha: i64, mawdi_izaha: usize) -> Natija<Vec<String>> {
    let bidaya = usize::try_from(izaha).map_err(|_| {
        talif(
            masar,
            format!("the string table offset {izaha} is negative or does not fit"),
            Some(mawdi_izaha),
        )
    })?;
    let mut qari = QariThunai {
        bayt,
        mawqi: bidaya,
        asas: 0,
    };
    let adad = qari.raqm32(masar).map_err(|_| {
        talif(
            masar,
            format!("the string table offset {bidaya} is past the end of the file"),
            Some(mawdi_izaha),
        )
    })?;
    let matlub = usize::try_from(adad).map_err(|_| {
        talif(
            masar,
            format!("the string table declares {adad} entries"),
            Some(bidaya),
        )
    })?;

    // Each entry costs at least its terminator, so a count larger than the
    // bytes that remain is corrupt — checked before allocating, so a bad count
    // cannot make Taarib reserve gigabytes it will never fill.
    let baqi = bayt.len().saturating_sub(qari.mawqi);
    if matlub > baqi {
        return Err(talif(
            masar,
            format!(
                "the string table declares {matlub} entries but only {baqi} bytes follow it in the \
                 file"
            ),
            Some(bidaya),
        ));
    }

    let mut nusus = Vec::with_capacity(matlub.min(1 << 16));
    for _ in 0..matlub {
        nusus.push(qari.nass_muntahi(masar)?);
    }
    Ok(nusus)
}
