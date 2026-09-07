//! What the placeholder scanner does to strings taken out of shipped games.
//!
//! Every string asserted on here was read out of a real installation, not
//! written for the test: the Unity ones out of R.E.P.O.'s Addressables string
//! tables, the Unreal ones out of Little Nightmares' `.locres` and `.pak`
//! members. The point of using them is that a placeholder the scanner does not
//! see is a placeholder [`taarib_saff::nasq`] hands to a translator as ordinary
//! text — no protected span, no atom in the table, and therefore nothing
//! downstream that can refuse a reply which translated, reordered or dropped
//! it. That failure is silent all the way to the player's screen, so the only
//! honest test of it is against the strings the games really ship.
//!
//! Three defects are covered, one per game feature:
//!
//! - **Smart Strings.** Unity Localization lets a placeholder's format field
//!   contain brace-delimited arguments. `{players:list:{}|,   |   and   }` is
//!   R.E.P.O.'s own player list, and the inner `{}` used to abort the scan.
//! - **Decoration.** `<u>` is in Unity's tag vocabulary and was parsed, but
//!   nothing carried the fact out of the module, so `<b><u>IMPORTANT</u></b>`
//!   kept its bold and lost its underline.
//! - **Glyph slots.** Little Nightmares writes its input prompts as `%0` and
//!   `%1`. No printf conversion follows the digits, so nothing was lifted.
//!
//! The last two tests are the other half of the contract: what must *not* be
//! lifted. A false positive costs a translator a refusal they cannot act on, so
//! the percentages in both games' prose are asserted to stay text.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use taarib_saff::nasq::{
    self, DharraMustakhraja, KhiyaratNasq, LahjatNasq, NassNaqi, NawDharra, Zakhrafa,
};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Extracts, or fails the test with the engine's own sentence.
#[track_caller]
fn istakhrij(khaam: &str, khiyarat: &KhiyaratNasq) -> NassNaqi {
    match nasq::istakhrij(khaam, khiyarat) {
        Ok(naqi) => naqi,
        Err(khata) => panic!("extracting {khaam:?}: {khata}"),
    }
}

/// The options an Unreal adapter runs with: the engine's rich text, plus the
/// glyph slots that engine's games write by hand.
fn khiyarat_unreal() -> KhiyaratNasq {
    KhiyaratNasq {
        lahjat: vec![LahjatNasq::Unreal, LahjatNasq::UnrealRumuz],
        ..KhiyaratNasq::default()
    }
}

/// Every property the guard downstream relies on, checked at once.
///
/// - the raw text rebuilds byte for byte;
/// - every atom occupies exactly one [`nasq::BADEEL_DHARRA`] in the clean text,
///   at the span the atom names;
/// - the reconstruction record at that offset holds the atom's raw bytes
///   unaltered, exactly once;
/// - no atom's raw text is empty, which would make two atoms indistinguishable
///   to a check that compares them as strings.
#[track_caller]
fn fahs_uqud(khaam: &str, naqi: &NassNaqi) {
    assert_eq!(
        nasq::aid_binaa(naqi),
        khaam,
        "round trip differs for {khaam:?}"
    );
    match nasq::tahaqquq(naqi) {
        Ok(()) => {},
        Err(khata) => panic!("spans do not land on the clean text of {khaam:?}: {khata}"),
    }

    for dharra in &naqi.dharrat {
        let Some(nitaq) = naqi.nitaq_dharra(dharra) else {
            panic!(
                "atom {:?} of {khaam:?} names a span that was never emitted",
                dharra.khaam
            );
        };
        let bidaya = usize::try_from(nitaq.bidaya).unwrap_or(usize::MAX);
        let nihaya = usize::try_from(nitaq.nihaya()).unwrap_or(usize::MAX);
        assert_eq!(
            naqi.nass.get(bidaya..nihaya),
            Some(nasq::BADEEL_DHARRA.to_string().as_str()),
            "atom {:?} of {khaam:?} does not occupy one object replacement character",
            dharra.khaam
        );
        assert!(
            !dharra.khaam.is_empty(),
            "an atom of {khaam:?} has no raw text"
        );

        let hunaka: Vec<&nasq::AtharNasq> = naqi
            .aathar
            .iter()
            .filter(|athar| athar.mawqi == nitaq.bidaya)
            .collect();
        assert_eq!(
            hunaka.len(),
            1,
            "the record for atom {:?} of {khaam:?} is not a single trace",
            dharra.khaam
        );
        let Some(athar) = hunaka.first() else {
            panic!("unreachable: the length was just asserted");
        };
        assert_eq!(
            athar.khaam, dharra.khaam,
            "the record at the span of atom {:?} in {khaam:?} holds different bytes",
            dharra.khaam
        );
    }
}

/// The raw text of every atom, in order.
fn khaamat(dharrat: &[DharraMustakhraja]) -> Vec<&str> {
    dharrat.iter().map(|dharra| dharra.khaam.as_str()).collect()
}

// ---------------------------------------------------------------------------
// a. Unity Localization Smart Strings — a nested brace is still a placeholder
// ---------------------------------------------------------------------------

/// R.E.P.O.'s player list is one atom, whole, in every language it ships.
///
/// `{players:list:{}|,   |   and   }` is a Smart String: the argument is
/// `players`, the formatter is `list`, and the three `|`-separated fields after
/// it are the item template and the two separators. The item template is a bare
/// `{}`, which is why the construct nests at all.
///
/// It has to come out as a *single* atom rather than as a placeholder plus some
/// text. The separators are inside the format specifier, and a translator who
/// was shown `,` and `and` as editable text and replaced them with Arabic would
/// produce a specifier Smart.Format cannot parse — the game then throws where
/// it used to print a list of players.
#[test]
fn smart_string_list_formatter_is_lifted_as_one_atom() {
    // Every language R.E.P.O. ships this string in, plus the machine-tagged
    // Brazilian row, exactly as the string table stores them.
    let haqiqiya = [
        "{players:list:{}|,   |   and   }",
        "{players:list:{}|,   |   ja   }",
        "{players:list:{}|,   |   och   }",
        "{players:list:{}|,   |   og   }",
        "(pt-BR) {players:list:{}|,   |   and   }",
        "(pt-PT) {players:list:{}|,   |   and   }",
    ];

    for khaam in haqiqiya {
        let naqi = istakhrij(khaam, &KhiyaratNasq::default());
        fahs_uqud(khaam, &naqi);

        assert_eq!(
            naqi.dharrat.len(),
            1,
            "{khaam:?} should hold exactly one placeholder, found {:?}",
            khaamat(&naqi.dharrat)
        );
        let Some(dharra) = naqi.dharrat.first() else {
            panic!("unreachable: the length was just asserted");
        };
        assert_eq!(dharra.naw, NawDharra::Mawdi);
        assert!(
            dharra.khaam.starts_with("{players:list:{}|") && dharra.khaam.ends_with('}'),
            "the atom lifted from {khaam:?} is {:?}, not the whole smart string",
            dharra.khaam
        );
        // `players` is a name, not a number, so there is no written position to
        // report and reporting one would invent an ordering constraint.
        assert_eq!(dharra.tarteeb, None);
        // The clean text keeps the prose around it and nothing else.
        assert!(
            !naqi.nass.contains('{') && !naqi.nass.contains('}'),
            "clean text of {khaam:?} still carries braces: {:?}",
            naqi.nass
        );
    }
}

/// Nesting is bounded, and past the bound the braces are text again.
///
/// Four levels is the ceiling — double the deepest construct Unity's own
/// documentation shows — and the fifth level is refused rather than swallowed,
/// so a brace-heavy string cannot be turned into one opaque atom that hides
/// every word inside it from the translator. Both halves are asserted, because
/// a limit that only ever rejected would be as wrong as no limit at all.
#[test]
fn nesting_inside_a_format_field_stops_at_the_depth_limit() {
    let mawjud = "{a:{{{{}}}}}";
    let naqi = istakhrij(mawjud, &KhiyaratNasq::default());
    fahs_uqud(mawjud, &naqi);
    assert_eq!(
        khaamat(&naqi.dharrat),
        vec![mawjud],
        "four levels must still be a placeholder"
    );

    let zaid = "{a:{{{{{}}}}}}";
    let naqi = istakhrij(zaid, &KhiyaratNasq::default());
    fahs_uqud(zaid, &naqi);
    assert!(
        naqi.dharrat.is_empty(),
        "five levels must not be lifted, found {:?}",
        khaamat(&naqi.dharrat)
    );
}

/// A format field that never closes is refused, nested or not.
///
/// The two spellings must agree. `{a:b and more text` already failed with
/// [`taarib_saff::khata::SababNasq::MawdiTalif`] before nesting was understood,
/// because a placeholder that commits and then never finishes is a string every
/// formatter throws on; `{a:{b} and more text` is the same defect written with
/// a brace in it, and allowing nesting must not turn one of them into an error
/// and leave the other silently swallowing the rest of the sentence.
#[test]
fn an_unfinished_format_field_is_refused_with_or_without_nesting() {
    for khaam in [
        "{a:b and more text",
        "{a:{b} and more text",
        "{players:list:{|,   |   and   ",
        "{players:list:{}|,   |   and   ",
    ] {
        let khata = match nasq::istakhrij(khaam, &KhiyaratNasq::default()) {
            Ok(naqi) => {
                panic!(
                    "{khaam:?} should have been refused, lifted {:?}",
                    khaamat(&naqi.dharrat)
                )
            },
            Err(khata) => khata,
        };
        // The named reason, not just any failure: refusing this string for an
        // unclosed *tag* would send a contributor looking at the wrong thing.
        assert!(
            khata.injilizi.contains("an incomplete format placeholder"),
            "{khaam:?} was refused for the wrong reason: {khata}"
        );
    }
}

// ---------------------------------------------------------------------------
// b. Decoration — the underline is carried out of the module
// ---------------------------------------------------------------------------

/// R.E.P.O.'s tutorial text keeps its underline as well as its bold.
///
/// `<b><u>IMPORTANT</u></b>` produces two spans over the same clean text. Bold
/// lands in the layout style as a weight, because a weight is something the
/// font chain can act on; the underline lands in
/// [`NassNaqi::zakhrafat`], because no font property expresses a rule drawn
/// under a run and dropping it here would lose it for good.
#[test]
fn underline_survives_beside_bold() {
    const KHAAM: &str = "Fill C.A.R.T.s with valuables. C.A.R.T.s are <b><u>IMPORTANT</u></b> for carrying more \
         stuff.";

    let naqi = istakhrij(KHAAM, &KhiyaratNasq::default());
    fahs_uqud(KHAAM, &naqi);
    assert!(
        !naqi.nass.contains('<'),
        "clean text still carries a tag: {:?}",
        naqi.nass
    );

    let ghaliz: Vec<&taarib_saff::talab::NitaqUslub> = naqi
        .nitaqat
        .iter()
        .filter(|nitaq| nitaq.uslub.wazn == Some(700))
        .collect();
    assert_eq!(ghaliz.len(), 1, "expected one bold span");

    let taht: Vec<&taarib_saff::talab::NitaqUslub> = naqi
        .nitaqat
        .iter()
        .filter(|nitaq| naqi.zakhrafat_nitaq(nitaq.id) == Some(Zakhrafa::TahtKhat))
        .collect();
    assert_eq!(
        taht.len(),
        1,
        "expected one underlined span, found {}",
        taht.len()
    );

    let (Some(ghaliz), Some(taht)) = (ghaliz.first(), taht.first()) else {
        panic!("unreachable: both lengths were just asserted");
    };
    // The two tags nest, so both spans cover the same word.
    assert_eq!((ghaliz.bidaya, ghaliz.tul), (taht.bidaya, taht.tul));
    let bidaya = usize::try_from(taht.bidaya).unwrap_or(usize::MAX);
    let nihaya = usize::try_from(taht.nihaya()).unwrap_or(usize::MAX);
    assert_eq!(naqi.nass.get(bidaya..nihaya), Some("IMPORTANT"));
}

/// Every shipped R.E.P.O. string that underlines something keeps the underline.
///
/// One of them underlines twice in one sentence, which is the case a table
/// keyed by span identifier has to get right and a single "this string was
/// underlined somewhere" flag could not.
#[test]
fn every_underlined_string_in_the_game_keeps_its_underlines() {
    let haqiqiya = [
        (
            "Activate and fill <b><u>EXTRACTION POINTS</u></b> with valuables.",
            1,
        ),
        ("Fill <b><u>EXTRACTION POINTS</u></b> with valuables.", 1),
        (
            "HELLO! FILL THE <b><u>EXTRACTION POINT</u></b> WITH VALUABLES!",
            1,
        ),
        (
            "If it gets damaged, you can <b><u>HEAL</u></b> it using your own health.",
            1,
        ),
        (
            "Insert <b><u>TAX TOKENS</u></b> into the shop machine to claim your rewards.",
            1,
        ),
        (
            "Enter the <b><u>TRUCK</u></b> and send a message to your boss in order to leave a \
             level.",
            1,
        ),
        (
            "Only one <b><u>EXTRACTION POINT</u></b> can be active at a time, find the current \
             one using your map!",
            1,
        ),
        (
            "Buy <b><u>ENERGY CRYSTALS</u></b> for the <b><u>CHARGING STATION</u></b> to \
             recharge and repair your gear!",
            2,
        ),
        (
            "Hvis den tager skade, kan du <b><u>HELE</u></b> den med dit eget liv.",
            1,
        ),
        (
            "Om den skadas så kan du <b><u>HELA</u></b> den med din egen hälsa.",
            1,
        ),
    ];

    for (khaam, mutawaqqa) in haqiqiya {
        let naqi = istakhrij(khaam, &KhiyaratNasq::default());
        fahs_uqud(khaam, &naqi);
        let adad = naqi
            .zakhrafat
            .iter()
            .filter(|zakhrafa| zakhrafa.naw == Zakhrafa::TahtKhat)
            .count();
        assert_eq!(adad, mutawaqqa, "{khaam:?} lost an underline");
        // Every decoration names a span that really exists.
        for zakhrafa in &naqi.zakhrafat {
            assert!(
                naqi.nitaqat.iter().any(|nitaq| nitaq.id == zakhrafa.nitaq),
                "{khaam:?} records a decoration on a span that was never emitted"
            );
        }
    }
}

/// Strikethrough travels the same channel, and the two are told apart.
///
/// The same tag table drives both, so a mapping that answered "decorated" and
/// not "decorated how" would pass every underline test and still turn a struck
/// word into an underlined one in the patch.
#[test]
fn strikethrough_and_underline_are_distinguished() {
    let naqi = istakhrij("<s>old</s> <u>new</u>", &KhiyaratNasq::default());
    fahs_uqud("<s>old</s> <u>new</u>", &naqi);
    assert_eq!(naqi.nass, "old new");

    let anwa: Vec<Zakhrafa> = naqi.zakhrafat.iter().map(|zakhrafa| zakhrafa.naw).collect();
    assert_eq!(anwa, vec![Zakhrafa::Shatb, Zakhrafa::TahtKhat]);
}

// ---------------------------------------------------------------------------
// c. Unreal glyph slots
// ---------------------------------------------------------------------------

/// Little Nightmares' input prompts are protected when the dialect is on.
///
/// `%0` is the slot the game fills with a button icon or a key name. It is a
/// positional argument, so the written number is reported as its position and a
/// reordering check downstream can compare `%0` against `%0` rather than
/// against `{0}`.
#[test]
fn unreal_glyph_slots_are_lifted_in_their_dialect() {
    let haqiqiya: [(&str, &[&str]); 8] = [
        ("Press %0 to Start", &["%0"]),
        ("Hold %0 to sneak", &["%0"]),
        ("%0 Equip / Unequip", &["%0"]),
        ("Press %0 to jump and %1 to grab", &["%0", "%1"]),
        ("Hold %0 and press %1 to climb", &["%0", "%1"]),
        ("Hold %0 and move %1 to drag", &["%0", "%1"]),
        ("Aim the flashlight with %0", &["%0"]),
        ("Use %0 + %1 to climb", &["%0", "%1"]),
    ];

    let khiyarat = khiyarat_unreal();
    for (khaam, mutawaqqa) in haqiqiya {
        let naqi = istakhrij(khaam, &khiyarat);
        fahs_uqud(khaam, &naqi);
        assert_eq!(
            khaamat(&naqi.dharrat),
            mutawaqqa.to_vec(),
            "wrong atoms for {khaam:?}"
        );
        for dharra in &naqi.dharrat {
            assert_eq!(dharra.naw, NawDharra::Mawdi);
            let raqm = dharra
                .khaam
                .get(1..)
                .and_then(|juz| juz.parse::<u32>().ok());
            assert_eq!(dharra.tarteeb, raqm, "{khaam:?} lost the slot number");
        }
    }
}

/// The same strings lift nothing under the default options.
///
/// This is the whole reason the rule is a dialect rather than part of the
/// shared scanner: several languages write a percentage with the sign in front,
/// so `%` followed by digits cannot mean "placeholder" everywhere. A caller who
/// has not said the text came from a game that writes slots gets the safe
/// reading.
#[test]
fn glyph_slots_are_not_lifted_without_the_dialect() {
    let iftiradi = KhiyaratNasq::default();
    assert!(
        !iftiradi.tashmal(LahjatNasq::UnrealRumuz),
        "the dialect must be opt-in"
    );

    for khaam in [
        "Press %0 to Start",
        "Hold %0 to sneak",
        "%0 Equip / Unequip",
    ] {
        let naqi = istakhrij(khaam, &iftiradi);
        fahs_uqud(khaam, &naqi);
        assert!(
            naqi.dharrat.is_empty(),
            "{khaam:?} lifted {:?} with the dialect off",
            khaamat(&naqi.dharrat)
        );
        assert_eq!(
            naqi.nass, khaam,
            "the text must be untouched with the dialect off"
        );
    }

    // The cost of the dialect, stated as a test rather than left to be
    // discovered: with it on, a prefix-written percentage is read as a slot.
    // Turkish writes fifty percent exactly this way, which is why no adapter
    // for a Turkish-source project may enable it.
    let naqi = istakhrij("%50 indirim", &khiyarat_unreal());
    assert_eq!(khaamat(&naqi.dharrat), vec!["%50"]);
}

/// C conversions keep their meaning with the dialect on.
///
/// printf is tried first, so `%0d` is zero-padded decimal rather than slot zero
/// followed by a stray letter, and `%1$s` stays one positional conversion
/// rather than being halved. Both spellings are in Little Nightmares' own
/// engine strings.
#[test]
fn printf_conversions_win_over_glyph_slots() {
    let khiyarat = khiyarat_unreal();
    let haqiqiya: [(&str, &[&str]); 5] = [
        ("%d Pending Actions: %s", &["%d", "%s"]),
        ("Delay (%.3f seconds left)", &["%.3f"]),
        ("Window : %s ", &["%s"]),
        (
            "Unrecognized curve easing function type [%i] for FCurveHandle",
            &["%i"],
        ),
        ("%0d and %1$s", &["%0d", "%1$s"]),
    ];

    for (khaam, mutawaqqa) in haqiqiya {
        let naqi = istakhrij(khaam, &khiyarat);
        fahs_uqud(khaam, &naqi);
        assert_eq!(
            khaamat(&naqi.dharrat),
            mutawaqqa.to_vec(),
            "wrong atoms for {khaam:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// d. The contract across both games at once
// ---------------------------------------------------------------------------

/// Every placeholder-bearing string read out of both games, end to end.
///
/// One test rather than one per string, because the property being checked is
/// the same for all of them and it is the property everything downstream is
/// built on: the raw text rebuilds byte for byte, each atom occupies exactly
/// one position in the clean text, and the bytes recorded at that position are
/// the placeholder's own, unaltered.
#[test]
fn every_real_placeholder_bearing_string_keeps_its_atoms() {
    // R.E.P.O., read with the options the Unity adapter uses.
    let unity: [(&str, &[&str]); 15] = [
        (
            "{players:list:{}|,   |   and   }",
            &["{players:list:{}|,   |   and   }"],
        ),
        (
            "{players:list:{}|,   |   ja   }",
            &["{players:list:{}|,   |   ja   }"],
        ),
        (
            "{players:list:{}|,   |   och   }",
            &["{players:list:{}|,   |   och   }"],
        ),
        (
            "{players:list:{}|,   |   og   }",
            &["{players:list:{}|,   |   og   }"],
        ),
        ("Level {number}", &["{number}"]),
        ("Bana {number}", &["{number}"]),
        ("Niveau {number}", &["{number}"]),
        ("Taso {number}", &["{number}"]),
        ("{ping} ms", &["{ping}"]),
        ("Hold to Reroll ({cost})", &["{cost}"]),
        ("Håll för att slumpa ny ({cost})", &["{cost}"]),
        ("Uusi valikoima (pidä pohjassa) ({cost})", &["{cost}"]),
        // A real line feed, not the two-character escape: the string table
        // stores U+000A, so it is a character in the clean text rather than an
        // atom, and only the placeholder after it is lifted.
        ("Game lobby is using version:\n{version}", &["{version}"]),
        ("Aula käyttää peliversiota:\n{version}", &["{version}"]),
        ("Lobbyn använder denna version\n{version}", &["{version}"]),
    ];

    // Little Nightmares, read with the options the Unreal adapter uses.
    let unreal: [(&str, &[&str]); 10] = [
        ("Press %0 to Start", &["%0"]),
        ("Press %0 to Resume Game", &["%0"]),
        ("Press %0 to Restart Demo", &["%0"]),
        ("Press %0 to use lighter", &["%0"]),
        ("Hold %0 to sneak", &["%0"]),
        ("Hold %0 to sprint", &["%0"]),
        ("%0 Equip / Unequip", &["%0"]),
        ("%0 Browse", &["%0"]),
        ("Use %0 to swing and %1 to jump", &["%0", "%1"]),
        ("Segure %0 e aperte %1 para escalar", &["%0", "%1"]),
    ];

    let iftiradi = KhiyaratNasq::default();
    let unrealiya = khiyarat_unreal();
    let mut adad = 0_usize;

    for (khaam, mutawaqqa) in unity {
        let naqi = istakhrij(khaam, &iftiradi);
        fahs_uqud(khaam, &naqi);
        assert_eq!(
            khaamat(&naqi.dharrat),
            mutawaqqa.to_vec(),
            "wrong atoms for {khaam:?}"
        );
        adad = adad.saturating_add(1);
    }
    for (khaam, mutawaqqa) in unreal {
        let naqi = istakhrij(khaam, &unrealiya);
        fahs_uqud(khaam, &naqi);
        assert_eq!(
            khaamat(&naqi.dharrat),
            mutawaqqa.to_vec(),
            "wrong atoms for {khaam:?}"
        );
        adad = adad.saturating_add(1);
    }

    assert_eq!(adad, 25, "the corpus lost strings");
}

// ---------------------------------------------------------------------------
// e. What must not be lifted
// ---------------------------------------------------------------------------

/// Percentages in both games' prose stay prose.
///
/// Every one of these is a shipped string. A translator shown a protected atom
/// where the source says "25%" would be refused for editing text that is text,
/// with nothing they could do about it — the failure mode a dialect flag exists
/// to prevent.
#[test]
fn percentages_in_prose_are_not_placeholders() {
    let haqiqiya = [
        // R.E.P.O.
        "Monsters now overcharge your grabber by a total increase of 25%",
        "Monsters now overcharge your grabber by a total increase of 50%",
        "Hirviöt ylikuormittavat kouraasi 25% enemmän",
        "Monster överbelastar din greppstråle med en ökning av 50%",
        "Monstre overbelaster nu din hapsestråle med en samlet forøgelse på 25%.",
        // Little Nightmares.
        "Apunta la linterna con 0%",
        "% of frame:",
        "% do quadro:",
        "Exc Time (%)",
        "Inc Time (%)",
    ];

    // Checked under both readings: the default, and the one that does read
    // `%0`. Neither may touch a percentage written after its number.
    for khiyarat in [KhiyaratNasq::default(), khiyarat_unreal()] {
        for khaam in haqiqiya {
            let naqi = istakhrij(khaam, &khiyarat);
            fahs_uqud(khaam, &naqi);
            assert!(
                naqi.dharrat.is_empty(),
                "{khaam:?} lifted {:?}",
                khaamat(&naqi.dharrat)
            );
            assert_eq!(naqi.nass, khaam, "{khaam:?} was altered");
        }
    }
}

/// Braces around ordinary words stay ordinary words.
///
/// Neither game ships a brace that is not a placeholder, so these are the two
/// cases the module's own reasoning names: a sentence someone put in braces,
/// and the Arabic convention of bracketing a Quranic quotation that way. Both
/// would be deleted from the visible text if a brace plus a word were enough to
/// commit to a placeholder.
#[test]
fn braces_around_prose_are_not_placeholders() {
    let khiyarat = KhiyaratNasq::default();
    for khaam in [
        "{Hello world}",
        "{إنا أعطيناك الكوثر}",
        "Press {the button} twice",
        "{Hello world} and {goodbye world}",
    ] {
        let naqi = istakhrij(khaam, &khiyarat);
        fahs_uqud(khaam, &naqi);
        assert!(
            naqi.dharrat.is_empty(),
            "{khaam:?} lifted {:?}",
            khaamat(&naqi.dharrat)
        );
        assert_eq!(naqi.nass, khaam, "{khaam:?} was altered");
    }
}
