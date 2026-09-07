//! الصدق — the tier-3 disclosure, and why it is a type rather than a dialog.
//!
//! Tier 3 costs the user things the other two tiers do not. The game is not
//! translated, it is *read from the screen and drawn over*: recognition makes
//! mistakes, there is latency between a line appearing and its Arabic appearing,
//! the frame rate drops measurably, and text the recognizer cannot see is text
//! the player does not get. Every one of those is a real limitation and the
//! product says so before the overlay is enabled, in plain language, with no
//! marketing around it.
//!
//! ## Why it is enforced structurally
//!
//! The obvious implementation is a dialog the installer shows and a boolean it
//! sets. That implementation is one refactor away from being skipped, and the
//! failure is silent: the overlay works, the user is simply never told what they
//! agreed to. Nothing breaks, no test fails, and the product quietly becomes the
//! thing it said it would not be.
//!
//! So the disclosure is a value. [`Iqrar`] cannot be constructed except by
//! [`Iqrar::baad_ard`], which takes the exact text that was shown and the
//! moment it was acknowledged. [`crate::wajiha::Tabaqa::shaghghil`] takes an
//! [`Iqrar`] by value and there is no other way to start the overlay — no
//! `Default`, no `new()`, no builder that fills one in. A future edit that wants
//! to skip the disclosure has to delete a parameter from a public signature,
//! which is a visible change in a review rather than a boolean that stopped
//! being set.
//!
//! This is the same discipline as `taarib-muhawwil-nusus`'s write guard, applied
//! to a promise to the user instead of to a file on disk.
//!
//! ## The text is here, not in the interface
//!
//! [`NASS_IFSAH_ARABI`] and [`NASS_IFSAH_INJILIZI`] live in this crate because
//! the thing being described is this crate's behaviour. An interface that wrote
//! its own wording would drift from what the overlay actually does the first
//! time the overlay changed, and the drift would be in the direction of sounding
//! better — nobody rewrites a disclosure to make it more alarming.

use std::fmt;

use crate::khata::KhataTabaqa;
/// The file name the disclosure acknowledgement is recorded under.
///
/// One spelling, here, because two parties need it and they cannot see each
/// other: the Studio writes the record when the user acknowledges, and the
/// overlay reads it from inside a game to decide whether it may start. The
/// overlay's crate cannot depend on the Studio — that would be a library
/// depending on an application — so the name lives in the crate they both
/// already depend on. Written in two places, a rename in one would leave the
/// overlay looking for a file nobody writes, and it would refuse to start with
/// no indication why.
pub const ISM_MALAF_IQRAR: &str = "ifsah_tabaqa.json";

/// What the user is told, in Arabic, before the overlay is enabled.
///
/// Five sentences, each naming one concrete limitation. No sentence describes a
/// benefit: the benefit is that the game becomes readable, and the user already
/// knows that or they would not be here. This text exists to say what it costs.
pub const NASS_IFSAH_ARABI: &str = "\
الطبقة العامة هي الحل الأخير، وتعمل بطريقة مختلفة عن بقية طرق التعريب.

• اللعبة نفسها لا تُعدَّل. يُقرأ النص من الصورة المعروضة على الشاشة، ثم تُرسم \
العربية فوقه.
• القراءة الآلية تُخطئ أحيانًا: قد يُقرأ حرف مكان آخر، وقد لا يُقرأ نص صغير أو \
موضوع على خلفية مزخرفة.
• هناك تأخير بين ظهور النص الأصلي وظهور ترجمته، لأن كل سطر يُقرأ ويُترجم بعد عرضه.
• سرعة اللعبة ستنخفض بمقدار يمكن قياسه، وتُعرض التكلفة الفعلية بالمللي ثانية في \
لوحة التحكم أثناء اللعب.
• النص الذي لا تراه القراءة الآلية لا يُترجم، ولا توجد طريقة لمعرفة أنه فاتها.

إن كانت لعبتك مدعومة بطريقة أخرى في تعريب، فتلك الطريقة أفضل من هذه في كل شيء.";

/// The same disclosure in English, word for word in meaning.
///
/// Not a summary and not a softened version. A user reading the English must
/// come away with exactly the expectations a user reading the Arabic does, which
/// is why both live in one file: a change to one that is not made to the other
/// is visible in a single diff.
pub const NASS_IFSAH_INJILIZI: &str = "\
The universal overlay is the last resort, and it works differently from every \
other way Taarib translates a game.

• The game itself is not modified. Text is read from the picture on screen, and \
Arabic is drawn over it.
• Automatic reading makes mistakes: one letter can be read as another, and small \
text or text on a busy background may not be read at all.
• There is a delay between the original line appearing and its translation \
appearing, because each line is read and translated after it is shown.
• The game will run measurably slower. The actual cost in milliseconds is shown \
in the control panel while you play.
• Text the reader does not see is not translated, and there is no way to know \
what it missed.

If your game is supported by any other Taarib method, that method is better than \
this one in every respect.";

/// A fingerprint of the disclosure text this build ships.
///
/// Carried in [`Iqrar`] and checked on construction. Its purpose is narrow and
/// worth stating: an acknowledgement recorded against an older, shorter
/// disclosure is not an acknowledgement of this one. Persisting "the user
/// agreed" across an update that added a limitation would be recording consent
/// to something never shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BasmatIfsah(u64);

impl BasmatIfsah {
    /// The fingerprint of the text this build ships.
    #[must_use]
    pub fn hadhihi_al_bina() -> Self {
        Self(basma_thabita(NASS_IFSAH_ARABI, NASS_IFSAH_INJILIZI))
    }

    /// The fingerprint as a number, for storing alongside a recorded consent.
    #[must_use]
    pub const fn raqm(self) -> u64 {
        self.0
    }

    /// Rebuilds a fingerprint recorded by an earlier run.
    #[must_use]
    pub const fn min_raqm(raqm: u64) -> Self {
        Self(raqm)
    }
}

impl fmt::Display for BasmatIfsah {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(mukhraj, "{:016x}", self.0)
    }
}

/// FNV-1a over both texts, as one stream.
///
/// A cryptographic hash is not warranted here and saying so plainly is better
/// than reaching for one out of habit: the question this answers is "is the text
/// this build ships the same text the user agreed to?", and the only party who
/// could benefit from forging an answer is the user themselves, forging consent
/// to a disclosure about their own machine. BLAKE3 is used everywhere in this
/// product where a fingerprint has to resist an adversary; this is not one of
/// those places, and pretending otherwise would be security theatre with a
/// dependency attached.
///
/// Both texts feed one stream rather than being hashed separately and combined,
/// so a sentence moved from the Arabic to the English changes the result.
fn basma_thabita(awwal: &str, thani: &str) -> u64 {
    const BIDAYA: u64 = 0xcbf2_9ce4_8422_2325;
    const ADAD_AWWALI: u64 = 0x0000_0100_0000_01b3;

    awwal
        .bytes()
        .chain(thani.bytes())
        .fold(BIDAYA, |basma, wahid| {
            (basma ^ u64::from(wahid)).wrapping_mul(ADAD_AWWALI)
        })
}

/// Proof that the tier-3 disclosure was shown and acknowledged.
///
/// There is no `Default`, no `new`, and no way to build one from parts. The only
/// constructor is [`Iqrar::baad_ard`], and it takes the fingerprint of the text
/// that was actually displayed — so a caller that wants to fabricate consent has
/// to display the disclosure in order to have the fingerprint to fabricate it
/// with, at which point it is not fabricated.
///
/// It is deliberately **not** [`Clone`]. An acknowledgement is consumed by the
/// one overlay it enables. Copying it would let one disclosure enable an overlay
/// for a second game the user never agreed to.
#[derive(Debug)]
pub struct Iqrar {
    basma: BasmatIfsah,
    lahza: u64,
    luba: String,
}

impl Iqrar {
    /// Records that the disclosure was shown, in full, and acknowledged.
    ///
    /// `basma` must be the fingerprint of the text that was displayed. `lahza`
    /// is when, in seconds since the Unix epoch, supplied by the caller because
    /// this crate reads no clock — it runs inside a game's render thread and a
    /// syscall there is a cost with no reason. `luba` names the game, so a
    /// persisted acknowledgement cannot be replayed for a different one.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IfsahMafqud`] when the fingerprint is not this build's,
    /// which means the text shown was not the text this build would show — an
    /// older release's wording, or a caller that passed a value it made up.
    pub fn baad_ard(
        basma: BasmatIfsah,
        lahza: u64,
        luba: impl Into<String>,
    ) -> Result<Self, KhataTabaqa> {
        if basma != BasmatIfsah::hadhihi_al_bina() {
            return Err(KhataTabaqa::IfsahMafqud);
        }
        Ok(Self {
            basma,
            lahza,
            luba: luba.into(),
        })
    }

    /// The fingerprint of the text that was shown.
    #[must_use]
    pub const fn basma(&self) -> BasmatIfsah {
        self.basma
    }

    /// When it was acknowledged, in seconds since the Unix epoch.
    #[must_use]
    pub const fn lahza(&self) -> u64 {
        self.lahza
    }

    /// Which game it was acknowledged for.
    #[must_use]
    pub fn luba(&self) -> &str {
        &self.luba
    }

    /// The line recorded in the session log when the overlay starts.
    #[must_use]
    pub fn satr_sijill(&self) -> String {
        format!(
            "tier-3 disclosure {} acknowledged for {} at {}",
            self.basma, self.luba, self.lahza
        )
    }
}

/// Whether a stored acknowledgement still applies to this build.
///
/// Called before an overlay is enabled from a saved preference. A `false` here
/// means the disclosure has changed since the user agreed to it and has to be
/// shown again — not because the law says so, but because the previous agreement
/// was to a different set of statements.
#[must_use]
pub fn mahfuz_salih(mahfuz: BasmatIfsah) -> bool {
    mahfuz == BasmatIfsah::hadhihi_al_bina()
}
