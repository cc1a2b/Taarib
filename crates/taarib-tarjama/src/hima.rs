//! الحماية — making a string survive a round trip through a translator that
//! does not understand it.
//!
//! The highest-stakes module in this crate. Everything else depends on it and
//! nothing is permitted to bypass it.
//!
//! Phase 12 already parsed every string's markup and format placeholders into
//! the Phase 1 span model. What arrives here is clean text plus spans. What
//! leaves is text a translation model can be handed, and what comes back is
//! verified before a single character of it is believed.
//!
//! ## Two kinds of thing need protecting, and they need different treatment
//!
//! **Atoms** — a `%s`, a `{name}`, a `\V[3]`, an inline sprite — are opaque.
//! Their text is not language and must come back byte-identical. They are
//! replaced outright by a token.
//!
//! **Style spans** — bold, a colour, a font change — cover text that *is*
//! language and *must* be translated. Replacing one outright would delete the
//! words inside it. They are therefore marked with a **pair** of tokens, one
//! before and one after the text they cover, and the text between is translated
//! normally.
//!
//! Collapsing these two into one mechanism is the obvious simplification and it
//! is wrong in both directions: tokenising a style span deletes translatable
//! text, and pairing an atom invites a model to put words inside something that
//! has no inside.
//!
//! ## The token form, and why this one
//!
//! `⟦0⟧` for an atom. `⟦0⟧ … ⟦/0⟧` for a span.
//!
//! U+27E6 and U+27E7, the mathematical white square brackets. Every property
//! was chosen against a specific failure:
//!
//! | requirement | why this form satisfies it |
//! | --- | --- |
//! | not confusable with the game's own markup | every dialect this product parses opens with `{`, `%`, `\`, `$`, `<` or `[`. These brackets appear in none of them, so a token can never be mistaken for a construct the game itself would interpret |
//! | not natural language in any script | `General_Category` is `Ps`/`Pe` — open and close punctuation, no script, no word. A model has nothing to translate |
//! | direction-neutral | bidi class `ON`. The token neither starts nor breaks a directional run, so inserting it into an English source does not change how the source reads, and it does not fight the Arabic that comes back |
//! | survives tokenization | short, and the interior is ASCII digits. No tokenizer splits a digit run into something unrecoverable |
//! | visible | deliberately **not** an invisible control or a private-use codepoint. An invisible token is one a model drops silently and a reviewer cannot see is missing; a visible one is both harder to drop and obvious when it survives into a shipped string |
//! | recoverable when several appear | the interior index distinguishes them, and the closing form carries `/` so an open and a close of the same index are distinct |
//!
//! ### The digit problem, which is real
//!
//! A model asked for Arabic output may render `⟦0⟧` as `⟦٠⟧`, substituting
//! Arabic-Indic digits. That is not corruption on the model's part — it is
//! doing exactly what it was asked — and a naive parser would call the token
//! missing and fail a translation that is perfectly good.
//!
//! So recovery accepts ASCII digits, Arabic-Indic (U+0660–U+0669) and Extended
//! Arabic-Indic (U+06F0–U+06F9), and normalises them. This is the one place
//! this module is lenient, it is lenient about a *notation* rather than about a
//! *fact*, and the leniency is written down rather than discovered.
//!
//! ## Failure fails the string
//!
//! A token missing, duplicated, or invented fails the whole string. Not a
//! warning, not a repair, not a best-effort restoration — the source is left
//! untranslated and [`KhataTarjama`] names which token and what came back instead.
//!
//! The precedent is Phase 12's and it is exact: an unresolvable field refuses
//! its object rather than naming it by number, because two unnamed fields would
//! derive one identity for two strings and the second would silently overwrite
//! the first. Here the equivalent is a `%d` that came back as `%s`: the string
//! reads fine, ships, and crashes the game's own formatter in front of a player.
//!
//! **Reordering is not failure.** Arabic routinely puts arguments in a different
//! order than English, so a token appearing at a different position is correct.
//! Only presence and count are enforced, never position.
//!
//! ## The protection is structural
//!
//! [`NassMahmi`] is the only thing a provider is given. It has no public
//! constructor, no public fields, and **no accessor that returns the raw source
//! text**. The single way to obtain one is [`ihmi`], which tokenises on the way
//! in; the single way to consume a reply is [`istaridd`], which verifies on the
//! way out.
//!
//! A future edit cannot send an unprotected string, because there is no type it
//! could put in the argument position to do so.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use taarib_mustalahat::nass::{NawNasq, NitaqNasq};

use crate::khata::KhataTarjama;

/// The opening bracket of every token.
pub const FATIHA: char = '\u{27E6}';

/// The closing bracket of every token.
pub const KHATIMA: char = '\u{27E7}';

/// The marker distinguishing a span's closing token from its opening one.
pub const ALAMAT_IGHLAQ: char = '/';

/// The largest number of protected items in one string.
///
/// Two hundred and fifty-six. A string with more markup spans than that is not
/// a sentence, it is a document, and a translation model handed two hundred and
/// fifty-seven opaque tokens will lose some of them — at which point every one
/// of those strings fails and the contributor sees a wall of failures with no
/// useful cause. Refusing to protect it at all, once, with a reason, is the
/// better outcome.
pub const AQSA_RUMUZ: usize = 256;

/// What a token stands for.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum NawRamz {
    /// An opaque atom: a placeholder, an inline sprite. One token, standalone.
    Dharra,
    /// A style span: bold, a colour. Two tokens, one either side of the text
    /// it covers.
    Nitaq,
}

/// One protected item, and what it must be restored to.
#[derive(Debug, Clone, PartialEq)]
struct Ramz {
    /// The index that appears inside the token's brackets.
    fahras: usize,
    /// Whether it is standalone or paired.
    naw: NawRamz,
    /// The span this came from, carried whole so restoration needs nothing else.
    asl: NitaqNasq,
}

/// A string that has been tokenised and is safe to send to a provider.
///
/// **The only type a provider ever receives.** It exposes the tokenised text and
/// the context a prompt needs, and deliberately exposes no way to reach the raw
/// source — it does not even hold one. A caller with one of these cannot send
/// unprotected text because there is none inside it to send, which is a stronger
/// guarantee than a private field with no accessor: a future edit cannot add the
/// accessor back without first adding the data.
///
/// Not [`Clone`] on purpose. A protected string is consumed by exactly one
/// request and restored by exactly one reply, and a copy would be a second reply
/// that could restore against a table the first had already used.
#[derive(Debug)]
pub struct NassMahmi {
    /// The clean text with every atom and span replaced or bracketed.
    matn: String,
    /// The table restoration reads, keyed by index so a lookup is not a scan.
    rumuz: BTreeMap<usize, Ramz>,
}

impl NassMahmi {
    /// The text to send.
    ///
    /// The only content this type yields, and it is already tokenised. There is
    /// deliberately no sibling method returning the untokenised source.
    #[must_use]
    pub fn matn(&self) -> &str {
        &self.matn
    }

    /// How many items are protected in this string.
    ///
    /// Used by the prompt builder to tell a model how many opaque tokens it must
    /// reproduce, which measurably reduces how often one goes missing.
    #[must_use]
    pub fn adad_rumuz(&self) -> usize {
        self.rumuz.len()
    }

    /// Whether anything at all needed protecting.
    ///
    /// A string with no markup still goes through this module — the round trip
    /// is identical and the alternative is a second, unprotected path.
    #[must_use]
    pub fn bila_rumuz(&self) -> bool {
        self.rumuz.is_empty()
    }

    /// Every token that must come back, in index order.
    ///
    /// Handed to the prompt so the instruction can enumerate them rather than
    /// describing them in prose. A model told "reproduce ⟦0⟧ and ⟦1⟧ exactly"
    /// loses them far less often than one told "preserve any placeholders".
    #[must_use]
    pub fn rumuz(&self) -> Vec<String> {
        self.rumuz
            .values()
            .flat_map(|ramz| match ramz.naw {
                NawRamz::Dharra => vec![ramz_mufrad(ramz.fahras)],
                NawRamz::Nitaq => {
                    vec![ramz_mufrad(ramz.fahras), ramz_ighlaq(ramz.fahras)]
                },
            })
            .collect()
    }
}

/// A restored translation, verified against its protection table.
///
/// Obtainable only from [`istaridd`], and therefore only for a reply whose every
/// token checked out. There is no way to construct one from an unverified
/// string.
#[derive(Debug, Clone, PartialEq)]
pub struct NassMustaad {
    /// The translated clean text, with tokens replaced by their originals.
    pub naqi: String,
    /// The spans over that text, with offsets recomputed for the Arabic.
    pub nasq: Vec<NitaqNasq>,
}

/// A single token, `⟦n⟧`.
#[must_use]
fn ramz_mufrad(fahras: usize) -> String {
    format!("{FATIHA}{fahras}{KHATIMA}")
}

/// A span's closing token, `⟦/n⟧`.
#[must_use]
fn ramz_ighlaq(fahras: usize) -> String {
    format!("{FATIHA}{ALAMAT_IGHLAQ}{fahras}{KHATIMA}")
}

/// Tokenises a string for sending.
///
/// `naqi` is the clean text Phase 12 produced and `nasq` are its spans. Atoms
/// are replaced; style spans are bracketed. The result is the only thing a
/// provider is ever given.
///
/// ## Why the spans are walked back to front
///
/// Every replacement changes the length of the text after it. Walking forward
/// would invalidate every subsequent span's offsets and require a running
/// adjustment, which is one arithmetic slip away from splicing a token into the
/// middle of a character. Walking from the highest offset down means no edit
/// ever moves a byte an unprocessed span still points at.
///
/// # Errors
///
/// [`KhataTarjama::RumuzKathira`] when the string carries more than
/// [`AQSA_RUMUZ`] protected items, and [`KhataTarjama::NitaqKharij`] when a span
/// names a range that is not inside the clean text or does not fall on a
/// character boundary — which means the extractor and the text disagree, and
/// splicing on that disagreement would corrupt the string being protected.
pub fn ihmi(naqi: &str, nasq: &[NitaqNasq]) -> Result<NassMahmi, KhataTarjama> {
    if nasq.len() > AQSA_RUMUZ {
        return Err(KhataTarjama::RumuzKathira {
            adad: nasq.len(),
            saqf: AQSA_RUMUZ,
        });
    }

    // Validate every span before touching the text. A partial splice followed by
    // a refusal would leave nothing usable, and validating first costs one pass.
    for nitaq in nasq {
        let bidaya = tul_usize(nitaq.bidaya);
        let nihaya = bidaya.saturating_add(tul_usize(nitaq.tul));
        if nihaya > naqi.len() || !naqi.is_char_boundary(bidaya) || !naqi.is_char_boundary(nihaya) {
            return Err(KhataTarjama::NitaqKharij {
                bidaya: nitaq.bidaya,
                tul: nitaq.tul,
                madaa: tul_u32(naqi.len()),
            });
        }
    }

    let mut rumuz = BTreeMap::new();
    let mut ahdath: Vec<Hadath> = Vec::with_capacity(nasq.len().saturating_mul(2));
    for (fahras, nitaq) in nasq.iter().enumerate() {
        let bidaya = tul_usize(nitaq.bidaya);
        let nihaya = bidaya.saturating_add(tul_usize(nitaq.tul));
        let naw = if nitaq.naw.dharra() {
            NawRamz::Dharra
        } else {
            NawRamz::Nitaq
        };
        match naw {
            NawRamz::Dharra => ahdath.push(Hadath::Dharra {
                fahras,
                bidaya,
                nihaya,
            }),
            NawRamz::Nitaq => {
                ahdath.push(Hadath::Iftah {
                    fahras,
                    mawdi: bidaya,
                    nihaya,
                });
                ahdath.push(Hadath::Ighlaq {
                    fahras,
                    mawdi: nihaya,
                    bidaya,
                });
            },
        }
        let _ = rumuz.insert(
            fahras,
            Ramz {
                fahras,
                naw,
                asl: nitaq.clone(),
            },
        );
    }
    ahdath.sort_by(Hadath::rattib);

    let mut matn = String::with_capacity(naqi.len());
    let mut maqru = 0_usize;
    for hadath in &ahdath {
        let mawdi = hadath.mawdi();
        // An event inside an atom's range would put a token in the middle of
        // text that is about to be replaced wholesale. It cannot happen — the
        // extractor never nests a span inside an atom — and if it ever did, the
        // token would be lost rather than misplaced, which is the failure the
        // caller must be told about rather than shown.
        if mawdi < maqru {
            return Err(KhataTarjama::NitaqKharij {
                bidaya: tul_u32(mawdi),
                tul: 0,
                madaa: tul_u32(naqi.len()),
            });
        }
        matn.push_str(naqi.get(maqru..mawdi).unwrap_or_default());
        maqru = mawdi;
        match *hadath {
            Hadath::Iftah { fahras, .. } => matn.push_str(&ramz_mufrad(fahras)),
            Hadath::Ighlaq { fahras, .. } => matn.push_str(&ramz_ighlaq(fahras)),
            Hadath::Dharra { fahras, nihaya, .. } => {
                // The atom's own text is not language. It goes entirely.
                matn.push_str(&ramz_mufrad(fahras));
                maqru = nihaya;
            },
        }
    }
    matn.push_str(naqi.get(maqru..).unwrap_or_default());

    Ok(NassMahmi { matn, rumuz })
}

/// One token to emit, at one offset in the clean text.
///
/// The splice used to run back to front over the text itself, editing it in
/// place. That is correct only while no two spans share an offset and none
/// encloses another — and both happen in real games: `<b><u>…</u></b>` is one
/// run wearing two styles, and a bold phrase containing an italic word is the
/// ordinary shape of emphasis. Editing in place under either one inserts a
/// token into the middle of a token already written, which either splits a
/// character and panics, or silently moves a style onto the wrong words. So the
/// text is not edited at all any more: it is walked once, forward, and the
/// tokens are emitted between its characters. Offsets into the clean text stay
/// meaningful throughout because nothing ever moves them.
enum Hadath {
    /// A style span begins here; `nihaya` is where it will end.
    Iftah {
        fahras: usize,
        mawdi: usize,
        nihaya: usize,
    },
    /// A style span ends here; `bidaya` is where it began.
    Ighlaq {
        fahras: usize,
        mawdi: usize,
        bidaya: usize,
    },
    /// An atom occupies `mawdi..nihaya` and replaces all of it.
    Dharra {
        fahras: usize,
        bidaya: usize,
        nihaya: usize,
    },
}

impl Hadath {
    /// Where in the clean text this token goes.
    const fn mawdi(&self) -> usize {
        match *self {
            Self::Iftah { mawdi, .. }
            | Self::Ighlaq { mawdi, .. }
            | Self::Dharra { bidaya: mawdi, .. } => mawdi,
        }
    }

    /// Emission order, which is what keeps the tokens properly nested.
    ///
    /// By offset first. Where several fall on one offset the rule is the one
    /// that makes brackets nest: everything ending here closes before anything
    /// starting here opens; among closes the span that started latest is the
    /// innermost and closes first; among opens the span that reaches furthest
    /// is the outermost and opens first. An atom sits between the closes and
    /// the opens of its own offset, so a zero-width atom at the start of a span
    /// lands inside that span — which is where the extractor said it was.
    ///
    /// Two spans covering exactly the same run — one string that is both bold
    /// and underlined, which is how a real game writes a heading — tie on every
    /// one of those, so the index breaks it: opens ascending, closes
    /// descending, which is the same span closing first that opened last. Any
    /// consistent order round-trips, because the tokens are opaque and the gate
    /// only asks that an opening token precede its own closing one. Properly
    /// nested is chosen anyway: `⟦0⟧⟦1⟧x⟦/1⟧⟦/0⟧` is what a reader of the
    /// bracketed text expects, and interleaving reads as a bug to whoever next
    /// looks at a string on its way to a provider.
    fn rattib(awwal: &Self, thani: &Self) -> Ordering {
        awwal
            .mawdi()
            .cmp(&thani.mawdi())
            .then_with(|| awwal.martaba().cmp(&thani.martaba()))
            .then_with(|| match (awwal, thani) {
                (
                    Self::Ighlaq {
                        bidaya: a,
                        fahras: fa,
                        ..
                    },
                    Self::Ighlaq {
                        bidaya: b,
                        fahras: fb,
                        ..
                    },
                ) => b.cmp(a).then_with(|| fb.cmp(fa)),
                (
                    Self::Iftah {
                        nihaya: a,
                        fahras: fa,
                        ..
                    },
                    Self::Iftah {
                        nihaya: b,
                        fahras: fb,
                        ..
                    },
                ) => b.cmp(a).then_with(|| fa.cmp(fb)),
                _ => Ordering::Equal,
            })
    }

    /// Closes, then atoms, then opens, at one offset.
    const fn martaba(&self) -> u8 {
        match *self {
            Self::Ighlaq { .. } => 0,
            Self::Dharra { .. } => 1,
            Self::Iftah { .. } => 2,
        }
    }
}

/// Verifies a provider's reply and restores every token.
///
/// The gate every translation passes through before it is believed.
///
/// ## What is checked, and what is deliberately not
///
/// **Checked:** every expected token is present exactly once; no token that was
/// not expected appears; a span's opening token precedes its own closing one.
///
/// **Not checked:** where the tokens are. Arabic reorders arguments relative to
/// English as a matter of grammar, so a reply with `⟦1⟧` before `⟦0⟧` is
/// correct and enforcing position would reject good translations by the
/// thousand.
///
/// # Errors
///
/// [`KhataTarjama::RamzMafqud`] when an expected token did not come back,
/// [`KhataTarjama::RamzMukarrar`] when one came back more than once,
/// [`KhataTarjama::RamzDakhil`] when the reply contains a token that was never
/// sent, and [`KhataTarjama::RamzMaqlub`] when a span's close precedes its open.
///
/// Every one of these fails the string. None of them is repaired: see this
/// module's header for why a plausible repair is worse than a refusal.
pub fn istaridd(mahmi: &NassMahmi, radd: &str) -> Result<NassMustaad, KhataTarjama> {
    let mawjuda = ijmaa_rumuz(radd)?;

    // Presence and count, in both directions.
    for ramz in mahmi.rumuz.values() {
        let mawqi_iftitah = mawjuda.get(&(ramz.fahras, false));
        let marra = mawqi_iftitah.map_or(0, Vec::len);
        if marra == 0 {
            return Err(KhataTarjama::RamzMafqud {
                ramz: ramz_mufrad(ramz.fahras),
                radd: mukhtasar(radd),
            });
        }
        if marra > 1 {
            return Err(KhataTarjama::RamzMukarrar {
                ramz: ramz_mufrad(ramz.fahras),
                adad: marra,
            });
        }

        if matches!(ramz.naw, NawRamz::Nitaq) {
            let mawqi_ighlaq = mawjuda.get(&(ramz.fahras, true));
            let marra_ighlaq = mawqi_ighlaq.map_or(0, Vec::len);
            if marra_ighlaq == 0 {
                return Err(KhataTarjama::RamzMafqud {
                    ramz: ramz_ighlaq(ramz.fahras),
                    radd: mukhtasar(radd),
                });
            }
            if marra_ighlaq > 1 {
                return Err(KhataTarjama::RamzMukarrar {
                    ramz: ramz_ighlaq(ramz.fahras),
                    adad: marra_ighlaq,
                });
            }
            // Open before close. Not a position check — the pair may sit
            // anywhere in the sentence — but a span whose close precedes its
            // open covers nothing, and restoring it would produce a negative
            // range.
            let iftitah = mawqi_iftitah.and_then(|q| q.first()).copied().unwrap_or(0);
            let ighlaq = mawqi_ighlaq.and_then(|q| q.first()).copied().unwrap_or(0);
            if ighlaq < iftitah {
                return Err(KhataTarjama::RamzMaqlub {
                    fahras: ramz.fahras,
                });
            }
        }
    }

    // Nothing invented. A model that helpfully added `⟦2⟧` to a string that had
    // two items has produced text this module cannot restore, and guessing what
    // it meant is exactly the repair this crate does not do.
    for (fahras, mughlaq) in mawjuda.keys() {
        match mahmi.rumuz.get(fahras) {
            Some(ramz) if !*mughlaq || matches!(ramz.naw, NawRamz::Nitaq) => {},
            _ => {
                let ramz = if *mughlaq {
                    ramz_ighlaq(*fahras)
                } else {
                    ramz_mufrad(*fahras)
                };
                return Err(KhataTarjama::RamzDakhil {
                    ramz,
                    radd: mukhtasar(radd),
                });
            },
        }
    }

    Ok(ibn_mustaad(mahmi, radd, &mawjuda))
}

/// Every token in a reply, mapped to where it occurs.
///
/// The map's key is `(index, is_closing)` and its value is the byte offsets the
/// token was found at — a list rather than a single offset precisely so that a
/// duplicate is *counted* and reported rather than silently taking the first.
///
/// # Errors
///
/// [`KhataTarjama::RamzTalif`] when a bracket opens and does not close, or
/// closes with something other than an index between — a reply where a model
/// wrote `⟦abc⟧` has damaged a token rather than moved it, and the two are not
/// the same failure.
fn ijmaa_rumuz(radd: &str) -> Result<BTreeMap<(usize, bool), Vec<usize>>, KhataTarjama> {
    let mut mawjuda: BTreeMap<(usize, bool), Vec<usize>> = BTreeMap::new();
    let mut baqi = radd;
    let mut asas = 0_usize;

    while let Some(izaha) = baqi.find(FATIHA) {
        let baad = baqi.get(izaha..).unwrap_or_default();
        let Some(tul_ighlaq) = baad.find(KHATIMA) else {
            return Err(KhataTarjama::RamzTalif {
                juz: mukhtasar(baad),
                sabab: "a token opened and never closed".to_owned(),
            });
        };
        let dakhil = baad.get(FATIHA.len_utf8()..tul_ighlaq).unwrap_or_default();

        let (mughlaq, raqm) = match dakhil.strip_prefix(ALAMAT_IGHLAQ) {
            Some(baqi_raqm) => (true, baqi_raqm),
            None => (false, dakhil),
        };
        let Some(fahras) = fahras_min_nass(raqm) else {
            return Err(KhataTarjama::RamzTalif {
                juz: mukhtasar(baad.get(..=tul_ighlaq).unwrap_or(baad)),
                sabab: format!("the token's interior is {raqm:?}, which is not an index"),
            });
        };

        mawjuda
            .entry((fahras, mughlaq))
            .or_default()
            .push(asas.saturating_add(izaha));

        let taqaddum = izaha
            .saturating_add(tul_ighlaq)
            .saturating_add(KHATIMA.len_utf8());
        asas = asas.saturating_add(taqaddum);
        baqi = baqi.get(taqaddum..).unwrap_or_default();
    }
    Ok(mawjuda)
}

/// A token's interior as an index, accepting the three digit forms.
///
/// ASCII, Arabic-Indic and Extended Arabic-Indic. A model producing Arabic that
/// localises the digits inside a token has done what it was asked; calling that
/// a lost placeholder would fail thousands of good translations over a notation.
///
/// [`None`] for anything that is not a run of digits, which is a damaged token
/// rather than a localised one.
fn fahras_min_nass(nass: &str) -> Option<usize> {
    if nass.is_empty() {
        return None;
    }
    let mut qeema = 0_usize;
    for harf in nass.chars() {
        let raqm = match harf {
            '0'..='9' => u32::from(harf) - u32::from('0'),
            // U+0660..=U+0669
            '\u{0660}'..='\u{0669}' => u32::from(harf) - 0x0660,
            // U+06F0..=U+06F9
            '\u{06F0}'..='\u{06F9}' => u32::from(harf) - 0x06F0,
            _ => return None,
        };
        qeema = qeema
            .checked_mul(10)?
            .checked_add(usize::try_from(raqm).ok()?)?;
    }
    Some(qeema)
}

/// Rebuilds the translated text with tokens replaced by their originals.
///
/// Walks the reply once, copying text and substituting at each token. Spans get
/// their offsets recomputed from where their tokens landed in the Arabic, which
/// is the whole reason the pair form exists: a style span's range in the
/// translation is not derivable from its range in the source.
fn ibn_mustaad(
    mahmi: &NassMahmi,
    radd: &str,
    mawjuda: &BTreeMap<(usize, bool), Vec<usize>>,
) -> NassMustaad {
    // Every token occurrence, in the order it appears in the reply.
    let mut mawaqi: Vec<(usize, usize, bool)> = Vec::with_capacity(mawjuda.len());
    for ((fahras, mughlaq), amakin) in mawjuda {
        for makan in amakin {
            mawaqi.push((*makan, *fahras, *mughlaq));
        }
    }
    mawaqi.sort_unstable();

    let mut naqi = String::with_capacity(radd.len());
    let mut nasq: Vec<NitaqNasq> = Vec::with_capacity(mahmi.rumuz.len());
    // Where each span's opening token landed in the rebuilt text.
    let mut iftitahat: BTreeMap<usize, u32> = BTreeMap::new();
    let mut sabiq = 0_usize;

    for (makan, fahras, mughlaq) in mawaqi {
        naqi.push_str(radd.get(sabiq..makan).unwrap_or_default());
        let tul_ramz = tul_ramz_fi(radd, makan);
        sabiq = makan.saturating_add(tul_ramz);

        let Some(ramz) = mahmi.rumuz.get(&fahras) else {
            continue;
        };
        match ramz.naw {
            NawRamz::Dharra => {
                let bidaya = tul_u32(naqi.len());
                // The atom's own raw text goes back exactly as it was.
                let khaam = khaam_dharra(&ramz.asl.naw);
                naqi.push_str(&khaam);
                nasq.push(NitaqNasq {
                    id: ramz.asl.id,
                    bidaya,
                    tul: tul_u32(khaam.len()),
                    naw: ramz.asl.naw.clone(),
                });
            },
            NawRamz::Nitaq if !mughlaq => {
                let _ = iftitahat.insert(fahras, tul_u32(naqi.len()));
            },
            NawRamz::Nitaq => {
                let bidaya = iftitahat.get(&fahras).copied().unwrap_or(0);
                nasq.push(NitaqNasq {
                    id: ramz.asl.id,
                    bidaya,
                    tul: tul_u32(naqi.len()).saturating_sub(bidaya),
                    naw: ramz.asl.naw.clone(),
                });
            },
        }
    }
    naqi.push_str(radd.get(sabiq..).unwrap_or_default());

    nasq.sort_by_key(|nitaq| (nitaq.bidaya, nitaq.tul));
    NassMustaad { naqi, nasq }
}

/// How many bytes the token at an offset occupies.
///
/// Measured from the text rather than reconstructed from the index, because a
/// reply that localised its digits has a token whose byte length differs from
/// the one that was sent, and rebuilding it from the index would skip the wrong
/// number of bytes and splice the remainder of the token into the translation.
fn tul_ramz_fi(radd: &str, makan: usize) -> usize {
    let baad = radd.get(makan..).unwrap_or_default();
    baad.find(KHATIMA).map_or(FATIHA.len_utf8(), |izaha| {
        izaha.saturating_add(KHATIMA.len_utf8())
    })
}

/// The exact source text of an atom.
///
/// An atom's whole contract is that it comes back byte-identical, and
/// [`NawNasq::Mawdi`] and [`NawNasq::Sura`] both carry their original text for
/// this reason. Anything else reaching here is a style span mislabelled as an
/// atom, which the empty string makes visible rather than papering over.
fn khaam_dharra(naw: &NawNasq) -> String {
    match naw {
        NawNasq::Mawdi { khaam } => khaam.clone(),
        NawNasq::Sura { marja } => marja.clone(),
        _ => String::new(),
    }
}

/// The first sixty-four characters of a string, for an error message.
///
/// Bounded because a failure report goes into a log and a diagnostics bundle,
/// and a model that returned four thousand words of apology instead of a
/// translation should not put four thousand words into either.
fn mukhtasar(nass: &str) -> String {
    nass.chars().take(64).collect()
}

/// A `u32` offset as a length, saturating.
fn tul_usize(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}

/// A length as a `u32`, saturating.
fn tul_u32(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}
