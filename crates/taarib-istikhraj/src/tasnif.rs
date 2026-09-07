//! التصنيف — one classification rule, six adapters.
//!
//! Every adapter in this crate ends up asking the same question about every
//! string it pulls out: *is a player going to read this, and if so, as what?*
//! The answer decides what the interface shows by default, how machine
//! translation is prompted, and which strings a translator sees first. Getting
//! it wrong in six places, six different ways, is how a product ends up with
//! Unreal strings that hide asset paths and Godot strings that do not — for no
//! reason a user could ever discover.
//!
//! So the rule lives here, once, and every adapter calls [`sannif`].
//!
//! ## The signals, strongest first
//!
//! | rank | signal | why it ranks there |
//! | --- | --- | --- |
//! | 1 | runtime capture saw it drawn | direct evidence, not inference |
//! | 2 | the container's own structure declares the kind | a statement, not a hint |
//! | 3 | the engine's localization system holds it | the developer filed it for translation |
//! | 4 | the field, asset or key path names it | the extractor knows this and the text does not |
//! | 5 | format placeholders are present | a substituted string is a rendered string |
//! | 6 | the text is shaped like a path or an identifier | proves internal, proves nothing else |
//! | 7 | character composition | the weakest thing there is, and scored like it |
//!
//! Rank 1 is not reachable from here. An adapter reading files has no idea what
//! the game drew, so [`sannif`] cannot see that signal at all;
//! [`aid_tasnif_bad_iltiqat`] exists for the merge step, which does, and it
//! overrides whatever static extraction concluded.
//!
//! ## Why the shape of the text ranks almost last
//!
//! `Attack` is an item name in one game, a menu label in another and an
//! animation state in a third, and nothing about the six characters
//! distinguishes them. The field it came out of does. So content is consulted
//! only after every structural signal has declined, and even then it is trusted
//! in one direction: composition can demonstrate that a string is **internal**
//! — `MAX_HEALTH` is not a line of dialogue in any game — but it cannot
//! demonstrate that a string is dialogue rather than a description. When
//! composition is all there is, this module answers
//! [`TasnifNass::Majhul`] with a low number rather than inventing a kind.
//!
//! ## Nothing here deletes anything
//!
//! A [`TasnifNass::Dakhili`] verdict hides a row behind a filter. It never
//! removes one. See [`taarib_mustalahat::nass::MudkhalNass::tasnif`] for the
//! argument; the short version is that a translator who cannot find the source
//! of one untranslated menu will not use the product again.

use taarib_mustalahat::nass::{MasdarIstikhraj, NawNasq, NitaqNasq, QuyudNass, TasnifNass};

use crate::jadwal::{MawqiNass, MudkhalMustakhraj, irfa_nasq};

/// Direct evidence: the string was seen on screen.
pub const THIQAT_ILTIQAT: u8 = 98;

/// The container's own structure states the kind outright.
///
/// RPG Maker's event command codes are the case this exists for: code 401 *is*
/// a dialogue line by the format's definition, not by anyone's reading of it.
pub const THIQAT_BUNYA: u8 = 95;

/// The engine's localization system holds this string, and the name says which
/// kind it is.
pub const THIQAT_TAWTIN_MUSAMMA: u8 = 88;

/// The engine's localization system holds this string and nothing names a kind.
///
/// **Low, and deliberately.** The number scores confidence in the
/// *classification*, and the classification in this case is
/// [`TasnifNass::Majhul`] — an admission rather than an answer. What is known
/// here is that a player reads the string, and that fact is recorded by the
/// verdict not being [`TasnifNass::Dakhili`], never by the number. A high
/// confidence attached to "I do not know" would sort the row away from the
/// review pass that exists to resolve exactly this case.
pub const THIQAT_TAWTIN: u8 = 40;

/// A field, asset or key path names the kind.
pub const THIQAT_ISM: u8 = 76;

/// The text is unambiguously a path.
pub const THIQAT_MASAR: u8 = 86;

/// The text is unambiguously an identifier rather than a sentence.
pub const THIQAT_MUARRIF: u8 = 72;

/// Format placeholders are present and nothing else spoke.
///
/// One of the four numbers below fifty, all of which attach to
/// [`TasnifNass::Majhul`] and score how much is known about the *kind* rather
/// than about whether a player sees it.
pub const THIQAT_MAWDI: u8 = 35;

/// Prose shape, and no structural signal at all. A guess, scored as one.
pub const THIQAT_TARKEEB: u8 = 28;

/// Nothing said anything.
pub const THIQAT_SAMT: u8 = 15;

/// The shortest token this module will call an identifier.
///
/// Four characters. `OK`, `On`, `Off` and `HP` are three of the most common
/// button labels in games, and every one of them is uppercase, wordless and
/// exactly the shape a naive identifier test flags. Below four characters the
/// test is switched off entirely rather than made cleverer.
pub const ADNA_MUARRIF: usize = 4;

/// How long an unbroken mixed-case run has to be before it reads as machine
/// output rather than a word.
///
/// Twelve. `PlayerName` is ten and is a plausible label a designer typed;
/// `a3f81c9d2b4e` is twelve and is not something a player was ever shown.
pub const ADNA_KHALEET: usize = 12;

/// Everything a classifier is allowed to look at.
///
/// Deliberately a struct rather than eight arguments: adapters gain signals over
/// time — Godot learned about `.po` message contexts after Unreal learned about
/// namespaces — and a positional call site is how one adapter ends up passing
/// its asset name where another passes its field name.
#[derive(Debug, Clone, Copy, Default)]
pub struct QaraainTasnif<'a> {
    /// The field or property the string sat in: `description`, `Text`,
    /// `parameters[0]`.
    pub haql: Option<&'a str>,
    /// The container it came out of, relative to the game's root.
    pub hawiya: &'a str,
    /// The asset inside that container, where the container holds several.
    pub asl: Option<&'a str>,
    /// The structural location: an object path, a namespace, a JSON pointer, a
    /// node path.
    pub masar: &'a str,
    /// Whether the engine's own localization system references this string.
    ///
    /// True for a `.locres` entry, a Godot `Translation` message, a Ren'Py
    /// `translate` block. False for a string dug out of a serialized field.
    /// This is the difference between "the developer filed this for translation"
    /// and "Taarib found some text in a struct".
    pub min_nizam_tawtin: bool,
    /// The markup and placeholder spans already lifted out of the text.
    pub nasq: &'a [NitaqNasq],
    /// The clean text.
    pub nass: &'a str,
    /// A kind the container stated outright, with the confidence that statement
    /// deserves.
    ///
    /// [`Some`] only where the format *declares* the kind — an RPG Maker event
    /// command code, a Ren'Py statement type. Never a caller's guess: a guess
    /// belongs in the signals below, where it competes with the others.
    pub tasrih: Option<(TasnifNass, u8)>,
}

/// Classifies one string.
///
/// Returns the kind and an honest confidence from zero to a hundred. The number
/// is not decoration: the interface sorts the uncertain rows to the top of the
/// review pass, machine translation is prompted more cautiously for them, and
/// [`crate::jadwal::JadwalNusus::adif`] uses it to decide which of two sightings
/// of one string wins. A guess that reported ninety would quietly outrank a
/// later sighting that actually knew.
#[must_use]
pub fn sannif(qaraain: &QaraainTasnif<'_>) -> (TasnifNass, u8) {
    // Rank 2. The container said so. Nothing below can improve on a format that
    // declares what its own fields hold.
    if let Some((tasnif, thiqa)) = qaraain.tasrih {
        return (tasnif, thiqa);
    }

    let asmaa = asmaa_lil_bahth(qaraain);

    // Rank 3 and 4 together, because when both are available they agree and
    // when only membership is available it still rules out `Dakhili`.
    if qaraain.min_nizam_tawtin {
        return match tasnif_min_ism(&asmaa) {
            // A localization entry whose key says `Dialogue` is dialogue. A
            // localization entry whose key says `Path` is still not internal —
            // the developer put it in the translation table, which is the one
            // act that settles whether a player reads it — so an internal
            // verdict from a *name* is discarded here rather than trusted.
            Some((TasnifNass::Dakhili, _)) | None => (TasnifNass::Majhul, THIQAT_TAWTIN),
            Some((tasnif, _)) => (tasnif, THIQAT_TAWTIN_MUSAMMA),
        };
    }

    // Rank 4 on its own.
    if let Some((tasnif, thiqa)) = tasnif_min_ism(&asmaa) {
        return (tasnif, thiqa);
    }

    // Rank 5. A string carrying `{0}` or `\V[3]` is a string the engine
    // substitutes into before drawing, and nothing substitutes into an asset
    // path. It does not say *what* kind of text it is, so it raises the floor
    // and leaves the kind unknown.
    let bihi_mawdi = fihi_mawdi(qaraain.nasq);

    // Rank 6. Shape. Only ever concludes `Dakhili`, and a placeholder overrides
    // it: `Assets/{0}/Panel.prefab` is a path built at runtime, but
    // `Loading {0}...` is not a path at all and the placeholder is what says so.
    if !bihi_mawdi {
        if yabdu_masaran(qaraain.nass) {
            return (TasnifNass::Dakhili, THIQAT_MASAR);
        }
        if yabdu_muarrifan(qaraain.nass) {
            return (TasnifNass::Dakhili, THIQAT_MUARRIF);
        }
    }

    if bihi_mawdi {
        return (TasnifNass::Majhul, THIQAT_MAWDI);
    }

    // Rank 7. Composition, in the one direction it can carry: several words of
    // real prose is not an identifier, so the row stays visible — as
    // *unclassified*, because which kind of prose it is has not been
    // established by anything.
    if yabdu_nathran(qaraain.nass) {
        return (TasnifNass::Majhul, THIQAT_TARKEEB);
    }

    (TasnifNass::Majhul, THIQAT_SAMT)
}
/// Re-classifies a string that runtime capture saw on screen.
///
/// **Capture proves visibility, and only visibility.** A string observed being
/// drawn is user-facing beyond argument — that is direct evidence, and it
/// outranks every inference the static classifier drew from the field it came
/// out of. What capture does *not* establish is which kind of user-facing
/// string it is: a component path and a screen rectangle say nothing about
/// dialogue versus a menu label.
///
/// So this function corrects exactly one verdict and leaves the rest alone:
///
/// - [`TasnifNass::Dakhili`] becomes [`TasnifNass::Majhul`] at
///   [`THIQAT_ILTIQAT`]. The static verdict was wrong about the one thing
///   capture can settle, and the honest replacement is "user-facing, kind
///   unknown" rather than a guess at the kind.
/// - Everything else keeps its classification and returns **confidence zero**.
///
/// That zero is doing real work and is not a placeholder. `JadwalNusus::adif`
/// overwrites a classification only when the incoming confidence is *strictly
/// greater*, so zero means "contribute nothing here" — which is correct,
/// because raising a shaky static guess of `Hiwar` to near-certainty on the
/// strength of having seen the string drawn would be claiming capture confirmed
/// something it never looked at.
///
/// Visibility is already carried separately by
/// `MudkhalMustakhraj::yaraha_allaib`, which consults the provenance rather
/// than the classification, so nothing is lost by declining to inflate the
/// number here.
#[must_use]
pub const fn aid_tasnif_bad_iltiqat(sabiq: TasnifNass) -> (TasnifNass, u8) {
    match sabiq {
        TasnifNass::Dakhili => (TasnifNass::Majhul, THIQAT_ILTIQAT),
        akhar => (akhar, 0),
    }
}

/// Everything an adapter has about one string before its markup is lifted.
///
/// Exists so that the three-step sequence every adapter performs — lift the
/// markup, classify with the spans in hand, build the entry — is written once.
/// The order is not interchangeable: the classifier's placeholder signal reads
/// the spans, so an adapter that classified before lifting would silently lose
/// that signal, and it would lose it *quietly*, in one adapter, in a way no test
/// distinguishes from a game that simply has no placeholders.
#[derive(Debug, Clone)]
pub struct TalabMudkhal<'a> {
    /// Where the string sits, in the engine's own structural terms.
    pub mawqi: MawqiNass,
    /// The source text exactly as the container stores it, markup included.
    pub khaam: &'a str,
    /// Whether the engine's own localization system references it.
    pub min_nizam_tawtin: bool,
    /// A kind the container declared outright, if it did.
    pub tasrih: Option<(TasnifNass, u8)>,
    /// The speaker, for dialogue.
    pub mutakallim: Option<String>,
    /// The lines around it.
    pub jiwar: Vec<String>,
    /// The encoding the container declared, when it declared one.
    pub tarmiz: Option<String>,
    /// What the engine enforces on this string.
    pub quyud: QuyudNass,
}

impl<'a> TalabMudkhal<'a> {
    /// The common case: a location, a text, and nothing else known yet.
    #[must_use]
    pub fn jadeed(mawqi: MawqiNass, khaam: &'a str) -> Self {
        Self {
            mawqi,
            khaam,
            min_nizam_tawtin: false,
            tasrih: None,
            mutakallim: None,
            jiwar: Vec::new(),
            tarmiz: None,
            quyud: QuyudNass::default(),
        }
    }

    /// Marks the string as one the engine's own localization system holds.
    #[must_use]
    pub const fn bi_nizam_tawtin(mut self) -> Self {
        self.min_nizam_tawtin = true;
        self
    }

    /// Records a kind the container stated outright.
    #[must_use]
    pub const fn bi_tasrih(mut self, tasnif: TasnifNass, thiqa: u8) -> Self {
        self.tasrih = Some((tasnif, thiqa));
        self
    }

    /// Records the speaker.
    #[must_use]
    pub fn bi_mutakallim(mut self, mutakallim: Option<String>) -> Self {
        self.mutakallim = mutakallim;
        self
    }

    /// Records the surrounding lines, which is what makes machine translation of
    /// dialogue worth anything.
    #[must_use]
    pub fn bi_jiwar(mut self, jiwar: Vec<String>) -> Self {
        self.jiwar = jiwar;
        self
    }

    /// Records the encoding the container declared.
    #[must_use]
    pub fn bi_tarmiz(mut self, tarmiz: Option<String>) -> Self {
        self.tarmiz = tarmiz;
        self
    }
}

/// Lifts the markup, classifies, and builds the table entry.
///
/// Every adapter in this crate produces its entries through here, so the
/// relationship between `khaam` and `naqi` is established in one place. That
/// relationship is load-bearing: `khaam` is what identity is derived from and
/// what a compiler writes back, `naqi` is what a translator edits, and an
/// adapter that put the cleaned text in both fields would produce a patch that
/// writes translated text into the game with the original's colour tags deleted.
///
/// ## Markup that will not parse does not lose the string
///
/// [`irfa_nasq`] can refuse — an unclosed placeholder is the usual cause. When
/// it does, the raw text is used as the clean text and no spans are recorded.
/// The alternative is dropping the string, and a string dropped here is a line
/// that never reaches a translator and whose absence nobody notices until a
/// player finds it in English. A string with unparsed markup is still worth
/// translating; it just cannot have its placeholders verified, and the empty
/// span list is what says so downstream.
#[must_use]
pub fn ansha_mudkhal(talab: TalabMudkhal<'_>) -> MudkhalMustakhraj {
    let (naqi, nasq) = match irfa_nasq(talab.khaam) {
        Ok(marfua) => marfua,
        Err(_) => (talab.khaam.to_owned(), Vec::new()),
    };

    let (tasnif, thiqa) = sannif(&QaraainTasnif {
        haql: talab.mawqi.haql.as_deref(),
        hawiya: &talab.mawqi.hawiya,
        asl: talab.mawqi.asl.as_deref(),
        masar: &talab.mawqi.mawqi,
        min_nizam_tawtin: talab.min_nizam_tawtin,
        nasq: &nasq,
        nass: &naqi,
        tasrih: talab.tasrih,
    });

    MudkhalMustakhraj {
        mawqi: talab.mawqi,
        khaam: talab.khaam.to_owned(),
        naqi,
        nasq,
        tasnif,
        thiqa,
        masdar: MasdarIstikhraj::Sakin,
        tarmiz: talab.tarmiz,
        quyud: talab.quyud,
        mutakallim: talab.mutakallim,
        jiwar: talab.jiwar,
        mashhad: None,
    }
}

/// The names a classification is read out of, lowercased and joined.
///
/// Field first, then the structural path, then the asset, then the container:
/// specific to general, which is the order the keyword scan wants, since the
/// first match wins and the field is the most specific thing available.
fn asmaa_lil_bahth(qaraain: &QaraainTasnif<'_>) -> String {
    let mut asmaa = String::with_capacity(96);
    if let Some(haql) = qaraain.haql {
        asmaa.push_str(&haql.to_lowercase());
        asmaa.push('\u{1}');
    }
    asmaa.push_str(&qaraain.masar.to_lowercase());
    asmaa.push('\u{1}');
    if let Some(asl) = qaraain.asl {
        asmaa.push_str(&asl.to_lowercase());
        asmaa.push('\u{1}');
    }
    asmaa.push_str(&qaraain.hawiya.to_lowercase());
    asmaa
}

/// Substrings that name a kind, most specific first.
///
/// Order is the whole design. `menuitem` has to be tested before `menu` or every
/// choice list in every game becomes a button label; `displayname` before `name`
/// or a display name becomes an item name; `errormessage` before `message` or an
/// error becomes a system notice. A hash set would lose all of that.
const DALALAT: &[(&str, TasnifNass)] = &[
    // Dialogue, and the words engines actually use for it.
    ("dialogue", TasnifNass::Hiwar),
    ("dialog", TasnifNass::Hiwar),
    ("conversation", TasnifNass::Hiwar),
    ("subtitle", TasnifNass::Hiwar),
    ("narration", TasnifNass::Hiwar),
    ("monologue", TasnifNass::Hiwar),
    ("cutscene", TasnifNass::Hiwar),
    ("barks", TasnifNass::Hiwar),
    ("bark", TasnifNass::Hiwar),
    ("speech", TasnifNass::Hiwar),
    ("utterance", TasnifNass::Hiwar),
    ("quest_text", TasnifNass::Hiwar),
    ("journal", TasnifNass::Hiwar),
    // Choices.
    ("menuitem", TasnifNass::Ikhtiyar),
    ("menu_item", TasnifNass::Ikhtiyar),
    ("choice", TasnifNass::Ikhtiyar),
    ("answer", TasnifNass::Ikhtiyar),
    ("response", TasnifNass::Ikhtiyar),
    // Tooltips before menus, because a tooltip lives inside a UI path and would
    // otherwise be swallowed by it.
    ("tooltip", TasnifNass::Tafseer),
    ("tool_tip", TasnifNass::Tafseer),
    ("hovertext", TasnifNass::Tafseer),
    ("hover", TasnifNass::Tafseer),
    ("hint", TasnifNass::Tafseer),
    ("helptext", TasnifNass::Tafseer),
    ("help_text", TasnifNass::Tafseer),
    // Errors before system messages, because an error is a system message and
    // the translator wants the narrower word.
    ("errormessage", TasnifNass::Khata),
    ("error", TasnifNass::Khata),
    ("failure", TasnifNass::Khata),
    ("failed", TasnifNass::Khata),
    ("warning", TasnifNass::Khata),
    ("exception", TasnifNass::Khata),
    // Credits and legal.
    ("credits", TasnifNass::Nusub),
    ("credit", TasnifNass::Nusub),
    ("licence", TasnifNass::Nusub),
    ("license", TasnifNass::Nusub),
    ("copyright", TasnifNass::Nusub),
    ("attribution", TasnifNass::Nusub),
    ("legal", TasnifNass::Nusub),
    ("eula", TasnifNass::Nusub),
    ("disclaimer", TasnifNass::Nusub),
    // Descriptions before names, because `itemdescription` contains neither
    // `item` nor `name` as its most meaningful part.
    ("description", TasnifNass::Wasf),
    ("descr", TasnifNass::Wasf),
    ("_desc", TasnifNass::Wasf),
    ("flavour", TasnifNass::Wasf),
    ("flavor", TasnifNass::Wasf),
    ("lore", TasnifNass::Wasf),
    ("summary", TasnifNass::Wasf),
    ("bodytext", TasnifNass::Wasf),
    // Names.
    ("displayname", TasnifNass::Ism),
    ("display_name", TasnifNass::Ism),
    ("charactername", TasnifNass::Ism),
    ("itemname", TasnifNass::Ism),
    ("skillname", TasnifNass::Ism),
    ("placename", TasnifNass::Ism),
    ("nickname", TasnifNass::Ism),
    ("speaker", TasnifNass::Ism),
    ("actor", TasnifNass::Ism),
    // Bare nouns for the things a game names, last in the group so that
    // `itemdescription` has already matched `description` above.
    ("item", TasnifNass::Ism),
    ("weapon", TasnifNass::Ism),
    ("armour", TasnifNass::Ism),
    ("armor", TasnifNass::Ism),
    ("skill", TasnifNass::Ism),
    ("enemy", TasnifNass::Ism),
    ("character", TasnifNass::Ism),
    ("faction", TasnifNass::Ism),
    ("location", TasnifNass::Ism),
    // System messages.
    ("notification", TasnifNass::Nizam),
    ("toast", TasnifNass::Nizam),
    ("systemmessage", TasnifNass::Nizam),
    ("system", TasnifNass::Nizam),
    ("prompt", TasnifNass::Nizam),
    ("confirm", TasnifNass::Nizam),
    // Menus and the interface generally, last of the visible kinds because they
    // are the broadest.
    ("button", TasnifNass::Qaima),
    ("caption", TasnifNass::Qaima),
    ("label", TasnifNass::Qaima),
    ("menu", TasnifNass::Qaima),
    ("hud", TasnifNass::Qaima),
    ("widget", TasnifNass::Qaima),
    ("settings", TasnifNass::Qaima),
    ("options", TasnifNass::Qaima),
    // Internal, and only words that cannot plausibly appear on a visible
    // control. `key` is here and `title` is not, for exactly that reason.
    ("shader", TasnifNass::Dakhili),
    ("guid", TasnifNass::Dakhili),
    ("debug", TasnifNass::Dakhili),
];

/// Short tokens that name a kind, matched as a **whole path segment** and never
/// as a substring.
///
/// `ui` is the case this table exists for. As a substring it fires on `build`,
/// `guide`, `require` and `quick`, so every asset under `Assets/Buildings/`
/// would come out as a menu label; as a whole segment it fires on
/// `Content/UI/Main` and on nothing else. Every token here is short enough that
/// substring matching would be wrong, which is precisely why they are in a
/// second table rather than in [`DALALAT`].
const AJZA_DALLA: &[(&str, TasnifNass)] = &[
    ("ui", TasnifNass::Qaima),
    ("gui", TasnifNass::Qaima),
    ("text", TasnifNass::Qaima),
    ("msg", TasnifNass::Nizam),
    ("msgs", TasnifNass::Nizam),
    ("err", TasnifNass::Khata),
    ("errs", TasnifNass::Khata),
    ("tip", TasnifNass::Tafseer),
    ("tips", TasnifNass::Tafseer),
    ("name", TasnifNass::Ism),
    ("names", TasnifNass::Ism),
    ("note", TasnifNass::Wasf),
    ("key", TasnifNass::Dakhili),
    ("keys", TasnifNass::Dakhili),
    ("path", TasnifNass::Dakhili),
    ("paths", TasnifNass::Dakhili),
    ("id", TasnifNass::Dakhili),
    ("uuid", TasnifNass::Dakhili),
    ("tag", TasnifNass::Dakhili),
    ("enum", TasnifNass::Dakhili),
];

/// What separates one segment of a name from the next.
const FAWASIL_ISM: &[char] = &[
    '/', '\\', '.', '_', '-', '[', ']', ':', ' ', '\u{1}', '\u{4}',
];

/// The kind a field, path or asset name implies, if any.
///
/// `asmaa` is expected lowercased already, which is what [`sannif`] hands it.
/// Exposed because the adapters occasionally have a name that is not part of the
/// location: Godot's `.po` message context, Unreal's namespace when the key
/// itself is opaque.
///
/// The two tables are consulted in order, and the order is the point. A specific
/// word anywhere in the name beats a short token that happens to be a segment,
/// so `Content/UI/Dialogue/Greeting` is dialogue rather than a menu.
#[must_use]
pub fn tasnif_min_ism(asmaa: &str) -> Option<(TasnifNass, u8)> {
    if let Some((_, tasnif)) = DALALAT.iter().find(|(kalima, _)| asmaa.contains(kalima)) {
        return Some((*tasnif, THIQAT_ISM));
    }
    asmaa.split(FAWASIL_ISM).find_map(|juz| {
        AJZA_DALLA
            .iter()
            .find(|(kalima, _)| *kalima == juz)
            .map(|(_, tasnif)| (*tasnif, THIQAT_ISM))
    })
}

/// Whether any span is a format placeholder or an inline sprite.
#[must_use]
pub fn fihi_mawdi(nasq: &[NitaqNasq]) -> bool {
    nasq.iter()
        .any(|nitaq| matches!(nitaq.naw, NawNasq::Mawdi { .. } | NawNasq::Sura { .. }))
}

/// Separators a path is allowed to use.
const FAWASIL: [char; 2] = ['/', '\\'];

/// Path roots that settle the question on their own.
const JUDHUR: &[&str] = &[
    "res://", "user://", "assets/", "content/", "./", "../", "game/", "data/",
];

/// Extensions that mean a token is a file rather than a word.
///
/// Deliberately not "anything after a dot". `Mr. Smith` has a dot, `3.5` has a
/// dot, and `etc.` has a dot; none of them is a filename, and a rule that read
/// the tail of a dotted string as an extension would classify all three as
/// internal.
const LAHIQAT: &[&str] = &[
    "prefab",
    "asset",
    "unity",
    "mat",
    "anim",
    "controller",
    "uasset",
    "umap",
    "pak",
    "utoc",
    "ucas",
    "locres",
    "locmeta",
    "tscn",
    "tres",
    "scn",
    "res",
    "pck",
    "png",
    "jpg",
    "jpeg",
    "tga",
    "dds",
    "bmp",
    "webp",
    "ttf",
    "otf",
    "woff",
    "wav",
    "ogg",
    "mp3",
    "bnk",
    "fbx",
    "obj",
    "shader",
    "hlsl",
    "glsl",
    "cginc",
    "json",
    "xml",
    "csv",
    "lua",
    "gd",
    "dll",
    "so",
    "dylib",
];

/// Whether the text is a path into the game's own data rather than a line a
/// player reads.
///
/// ## The false positive this is built around
///
/// `Assets/UI/Panel.prefab` is internal. `Go to the market/square` is a line of
/// dialogue that happens to contain a slash, and a naive "contains a slash" test
/// hides it behind the internal filter — where the translator never sees it, the
/// game ships with one English line in the middle of an Arabic quest, and nobody
/// can work out where it came from.
///
/// Three rules together avoid that:
///
/// 1. **Every segment must be identifier-shaped.** No whitespace, no sentence
///    punctuation, no apostrophes. `Go to the market` has spaces and fails on the
///    first segment, so the whole string is not a path however many slashes
///    follow.
/// 2. **One separator is not enough on its own.** `market/square` clears rule 1
///    and is still ordinary prose in a game about a market, so a single-separator
///    string must additionally carry a known root or a real file extension.
/// 3. **A backslash settles it.** No natural-language sentence contains one, and
///    every Windows path does.
#[must_use]
pub fn yabdu_masaran(nass: &str) -> bool {
    let munaqqa = nass.trim();
    if munaqqa.is_empty() || munaqqa.contains('\n') {
        return false;
    }

    let adad_fawasil = munaqqa
        .chars()
        .filter(|harf| FAWASIL.contains(harf))
        .count();
    if adad_fawasil == 0 {
        return false;
    }

    // Rule 1, applied to every segment including the first and the last. A
    // trailing separator leaves an empty segment, which is legal in a directory
    // path and is skipped rather than treated as a failure.
    let ajza = munaqqa.split(FAWASIL).filter(|juz| !juz.is_empty());
    let mut ra_juz = false;
    for juz in ajza {
        ra_juz = true;
        if !juz_muarrif(juz) {
            return false;
        }
    }
    if !ra_juz {
        return false;
    }

    // Rule 3.
    if munaqqa.contains('\\') {
        return true;
    }

    let munkhafid = munaqqa.to_lowercase();
    if JUDHUR.iter().any(|jidhr| munkhafid.starts_with(jidhr)) {
        return true;
    }
    if munkhafid.starts_with('/') || munkhafid.starts_with('~') || munkhafid.contains("://") {
        return true;
    }
    if lahu_lahiqa(&munkhafid) {
        return true;
    }

    // Rule 2: two or more separators, every segment identifier-shaped, and no
    // punctuation anywhere. Three segments of pure identifiers is a path in
    // every corpus this has been read against.
    adad_fawasil >= 2
}

/// Whether a single token is an internal identifier rather than a word a player
/// reads.
///
/// Answers `false` for anything containing whitespace, because a phrase is not
/// an identifier however it is capitalised, and answers `false` for anything
/// under [`ADNA_MUARRIF`] characters, because `OK`, `On` and `HP` are three of
/// the most common button labels in games and every one of them would otherwise
/// be flagged.
#[must_use]
pub fn yabdu_muarrifan(nass: &str) -> bool {
    let munaqqa = nass.trim();
    if munaqqa.chars().count() < ADNA_MUARRIF || munaqqa.chars().any(char::is_whitespace) {
        return false;
    }

    let munkhafid = munaqqa.to_lowercase();
    if lahu_lahiqa(&munkhafid) {
        return true;
    }
    if shakl_muarrif_alami(munaqqa) {
        return true;
    }
    if sittasi_ashri(munaqqa) {
        return true;
    }

    // SCREAMING_SNAKE_CASE. The underscore is required: a bare run of capitals
    // is `CONTINUE`, which is a button.
    let sarikh = munaqqa.contains('_')
        && munaqqa
            .chars()
            .all(|harf| harf.is_ascii_uppercase() || harf.is_ascii_digit() || harf == '_');
    if sarikh {
        return true;
    }

    // Any single token joined by underscores: `player_hp`, `on_button_pressed`,
    // `sprite_index`. The whitespace test above has already removed everything
    // that is a phrase, and prose does not put an underscore where a space
    // belongs — so what is left is a name. `game_over` is the false positive
    // this accepts, and it costs a row being hidden behind a filter a translator
    // can lift rather than a string being lost.
    if munaqqa.contains('_')
        && munaqqa
            .chars()
            .all(|harf| harf.is_alphanumeric() || harf == '_')
    {
        return true;
    }

    // A dotted key path: `ui.menu.play`, `error.network.timeout`. Every segment
    // has to be identifier-shaped and there must be at least two dots, so
    // `Mr. Smith` (whitespace, already rejected) and `3.5` (one dot) do not
    // reach here.
    let adad_nuqat = munaqqa.chars().filter(|harf| *harf == '.').count();
    if adad_nuqat >= 2
        && munaqqa
            .split('.')
            .filter(|juz| !juz.is_empty())
            .all(juz_muarrif)
    {
        return true;
    }

    // A long unbroken run mixing case and digits: a hash, a base64 fragment, a
    // minified symbol. Length is what makes this safe — `PlayerName` mixes case
    // and is ten characters, which is a plausible label somebody typed.
    let tawil = munaqqa.chars().count() >= ADNA_KHALEET;
    let bihi_raqm = munaqqa.chars().any(|harf| harf.is_ascii_digit());
    let bihi_kabir = munaqqa.chars().any(char::is_uppercase);
    let bihi_sagheer = munaqqa.chars().any(char::is_lowercase);
    tawil && bihi_raqm && bihi_kabir && bihi_sagheer
}

/// Whether the text reads as several words of prose.
///
/// Used only to keep a row visible and unclassified rather than to name a kind.
/// Four words is the threshold because three-word strings are overwhelmingly
/// labels — `New Game`, `Load Last Save` — and telling a label from a sentence
/// at that length is not something character composition can do.
fn yabdu_nathran(nass: &str) -> bool {
    nass.split_whitespace()
        .filter(|kalima| kalima.chars().any(char::is_alphabetic))
        .count()
        >= 4
}

/// Whether one path or key segment could be an identifier.
///
/// Rejects whitespace and every mark that belongs to a sentence rather than to a
/// name. Accepts letters in any script — a Japanese game names its assets in
/// Japanese and an Arabic-language mod names them in Arabic, and a rule that
/// only accepted ASCII would call both of those prose.
fn juz_muarrif(juz: &str) -> bool {
    if juz.is_empty() {
        return false;
    }
    !juz.chars().any(|harf| {
        harf.is_whitespace()
            || matches!(
                harf,
                ',' | ';'
                    | ':'
                    | '!'
                    | '?'
                    | '\''
                    | '"'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | '='
                    | '\u{60C}'
                    | '\u{61B}'
                    | '\u{61F}'
                    | '\u{2018}'
                    | '\u{2019}'
                    | '\u{201C}'
                    | '\u{201D}'
                    | '\u{2026}'
            )
    })
}

/// Whether a lowercased string ends in one of the known asset extensions.
fn lahu_lahiqa(munkhafid: &str) -> bool {
    match munkhafid.rsplit_once('.') {
        Some((jidhr, lahiqa)) => {
            !jidhr.is_empty()
                && !lahiqa.is_empty()
                && LAHIQAT.contains(&lahiqa)
                && !lahiqa.chars().any(char::is_whitespace)
        },
        None => false,
    }
}

/// Whether the text has the shape of a UUID or a GUID.
///
/// Checked structurally rather than by counting hyphens, because
/// `well-thought-out-plan` has four hyphens and is a phrase.
fn shakl_muarrif_alami(nass: &str) -> bool {
    let munaqqa = nass.trim_start_matches('{').trim_end_matches('}');
    let atwal = [8_usize, 4, 4, 4, 12];
    let mut ajza = munaqqa.split('-');
    for matlub in atwal {
        match ajza.next() {
            Some(juz) if juz.chars().count() == matlub => {
                if !juz.chars().all(|harf| harf.is_ascii_hexdigit()) {
                    return false;
                }
            },
            _ => return false,
        }
    }
    ajza.next().is_none()
}

/// Whether the text is a bare run of hexadecimal long enough to be a digest.
///
/// Eight characters minimum, which is [`ADNA_MUARRIF`] doubled, because
/// `beef`, `face` and `cafe` are all four-character hexadecimal and all three
/// are English words a game might legitimately display.
fn sittasi_ashri(nass: &str) -> bool {
    let adad = nass.chars().count();
    adad >= 8 && nass.chars().all(|harf| harf.is_ascii_hexdigit())
}
