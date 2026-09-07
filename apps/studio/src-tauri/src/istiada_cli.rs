//! الاستعادة قبل الإزالة — the uninstaller's restore offer, headless.
//!
//! `docs/tawzee.md` §7: before the application is removed, every game that
//! carries Taarib content is offered the Phase 20G library-wide restore. The
//! NSIS section and the Linux/macOS uninstall instructions both reach it the
//! same way — `taarib-studio --istiada` — so the whole flow runs before any
//! window, any Tauri state or any interface exists, and must therefore carry
//! its own resolution, its own confirmation and its own report.
//!
//! The flow answers to two module contracts it does not restate:
//! [`taarib_tathbeet::taraju::istiada_al_maktaba`] promises that every failure
//! is preserved in full — code, path and remedy, never collapsed into a count —
//! and this module's only job on top of that promise is to surface it, on
//! stdout, in `sijillat/istiada_uninstall.log`, and in a closing dialog. And
//! `main` promises one startup order — paths, settings, diagnostics, database —
//! which is mirrored here verbatim so a failure in any of the four lands in
//! the same log a failure at startup would.
//!
//! Nothing in this module panics. Every failure prints its Arabic sentence and
//! either skips one game or ends the flow cleanly; the uninstall proceeds
//! either way, and the user has seen exactly what happened and what did not.

// `main` declares this module private, so clippy reads the `pub(crate)` entry point below as
// reachable only from inside it and asks for plain `pub`. Writing `pub` makes
// `unreachable_pub`, which the workspace denies, fire on the same item instead; the two rules
// only reconcile where the module is declared.
#![expect(
    clippy::redundant_pub_crate,
    reason = "`pub` here trips the workspace's denied `unreachable_pub` on a private module"
)]

use std::io::Write as _;
use std::path::Path;

use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use taarib_makhzan::sijillat::{SijillAlaab, TalabMaktaba};
use taarib_makhzan::wasl::{Makhzan, alaan};
use taarib_tathbeet::bayan::NawTathbeet;
use taarib_tathbeet::taraju::{
    MawqiTathbeet, RadLaShay, SiyasatIstiada, TaqreerMaktaba, istiada_al_maktaba,
};
use taarib_usus::idadat::MakhzanIdadat;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::Masarat;
use taarib_usus::{ISDAR, sijill};

use crate::luba_awamir::{jidhr_nusakh, muthabbat};

/// The argument that turns the studio binary into the uninstall restore flow.
const WASITAT_ISTIADA: &str = "--istiada";

/// The flow's own record, inside the data root's `sijillat/` directory.
const ISM_SIJILL_ISTIADA: &str = "istiada_uninstall.log";

/// Printed when the library carries nothing of Taarib's — the whole answer.
const JUMLAT_LA_SHAY: &str =
    "لا توجد أي لعبة عليها محتوى مثبَّت من تعريب؛ لا شيء يحتاج إلى استعادة قبل إزالة التطبيق.";

/// Printed when the user declines the offer.
const JUMLAT_RAFD: &str = "لم تُنفَّذ الاستعادة بناءً على اختيارك. يبقى محتوى تعريب داخل الألعاب، وتبقى \
     النسخ الأصلية في مجلد بيانات تعريب، ويمكن تنفيذ الاستعادة لاحقًا بتشغيل taarib-studio --istiada.";

/// The closing sentence: the uninstall goes on, and the data root stays put.
const JUMLAT_MUTABAA: &str = "سيتابع المزيل الآن إزالة التطبيق. يبقى مجلد بيانات تعريب — ومعه النسخ \
     الأصلية وسجلاتها — في مكانه ما لم تطلب حذفه صراحة.";

/// Runs the uninstall restore offer when the process was started with
/// `--istiada`, and says whether it ran.
///
/// `false` means the arguments did not ask for it, nothing at all was touched,
/// and the caller starts the window as usual. `true` means the whole flow ran —
/// resolution, enumeration, confirmation, restore and report — and the caller
/// exits instead of opening the window. The flow returns `true` even when a
/// step inside it failed: the uninstall proceeds, and the failure has already
/// been printed as its own Arabic sentence.
pub(crate) fn shaghghil_idha_talab() -> bool {
    if !std::env::args_os().any(|wasita| wasita == WASITAT_ISTIADA) {
        return false;
    }
    ajri();
    true
}

/// The whole offer, in the order the contract states it.
///
/// Resolution mirrors `main` exactly — paths, then settings, then diagnostics,
/// then the database — with one difference of shape only: nothing is handed to
/// Tauri, because there is no Tauri yet, so the values live on this stack for
/// the flow's lifetime instead of in managed state.
fn ajri() {
    let mut musajjil = Musajjil::jadeed();

    let masarat = match Masarat::iktashif() {
        Ok(masarat) => masarat,
        Err(khata) => {
            musajjil.inha_bi_khata(&khata);
            return;
        },
    };
    if let Err(khata) = masarat.takid() {
        musajjil.inha_bi_khata(&khata);
        return;
    }
    let makhzan = match MakhzanIdadat::iftah(&masarat) {
        Ok(makhzan) => makhzan,
        Err(khata) => {
            musajjil.inha_bi_khata(&khata);
            return;
        },
    };
    let idadat = makhzan.hali();

    // Held for the flow's lifetime so library events reach the standard
    // diagnostics log; a refusal costs tracing, never the restore itself.
    let _haris = match sijill::hayyi(&masarat, idadat.tashkhis.mustawa) {
        Ok(haris) => Some(haris),
        Err(khata) => {
            musajjil.sattir(&format!("تحذير: {}", khata.arabi));
            None
        },
    };

    let qaida = match Makhzan::iftah(&masarat) {
        Ok(qaida) => qaida,
        Err(khata) => {
            musajjil.inha_bi_khata(&khata);
            return;
        },
    };

    // The store's clock, as every timestamp in the product; a store that
    // cannot answer costs the header its time, not the flow its record.
    let waqt = qaida.bil_qira(alaan).unwrap_or_else(|_| "-".to_owned());
    musajjil.iftah_malaf(&masarat.sijillat().join(ISM_SIJILL_ISTIADA));
    musajjil.sattir(&format!("تعريب {ISDAR} — عرض الاستعادة قبل الإزالة ({waqt})"));

    let mawaqi = match ijma_mawaqi(&masarat, &qaida, &mut musajjil) {
        Ok(mawaqi) => mawaqi,
        Err(khata) => {
            musajjil.inha_bi_khata(&khata);
            return;
        },
    };

    if mawaqi.is_empty() {
        musajjil.sattir(JUMLAT_LA_SHAY);
        musajjil.ikhtim();
        return;
    }

    let jumla = jumlat_takid(mawaqi.len());
    musajjil.sattir(&jumla);
    if !istifham(&jumla, &mut musajjil) {
        musajjil.sattir(JUMLAT_RAFD);
        musajjil.ikhtim();
        return;
    }

    let taqreer = istaid_al_kul(&mawaqi, &mut musajjil);
    let sutur = sutur_khitamiya(&taqreer);
    for satr in &sutur {
        musajjil.sattir(satr);
    }
    if shasha_mutaha() {
        aarid_taqreer(&sutur, taqreer.najahat());
    }

    musajjil.sattir(JUMLAT_MUTABAA);
    musajjil.ikhtim();
}

/// Every game the store knows that currently carries a Taarib installation.
///
/// The same joins the library screen makes and nothing new: the games come
/// from `SijillAlaab::qaima` — the visible set and then the hidden set,
/// because hiding a game from the grid never uninstalled anything — the backup
/// directory from the active installation record via [`jidhr_nusakh`], and
/// "carries" from [`muthabbat`], which is manifest truth rather than database
/// truth. No launcher is rescanned: an uninstall answers for what Taarib
/// wrote, not for what a launcher currently lists.
///
/// # Errors
///
/// Whatever the store raises listing the library. A single game whose backup
/// directory cannot be resolved is reported by name and skipped, because one
/// unreadable record must not cost the user the restore of everything else.
fn ijma_mawaqi(
    masarat: &Masarat,
    qaida: &Makhzan,
    musajjil: &mut Musajjil,
) -> Natija<Vec<MawqiTathbeet>> {
    let alaab = qaida.bil_qira(|ittisal| {
        let sijill = SijillAlaab::jadeed(ittisal);
        let mut kull = sijill.qaima(&TalabMaktaba::default())?;
        let makhfiya = sijill.qaima(&TalabMaktaba {
            mukhfiya: true,
            ..TalabMaktaba::default()
        })?;
        kull.extend(makhfiya);
        Ok(kull)
    })?;

    let mut mawaqi = Vec::new();
    for luba in alaab {
        let nusakh = match jidhr_nusakh(masarat, qaida, luba.id) {
            Ok(nusakh) => nusakh,
            Err(khata) => {
                musajjil.sattir(&format!("تخطّي {}: {}", luba.ism, khata.arabi));
                musajjil.sattir(&format!("  {}", khata.li_sijill()));
                continue;
            },
        };
        let yahmil = NawTathbeet::KULL
            .into_iter()
            .any(|naw| muthabbat(&luba.jidhr, &nusakh, naw));
        if yahmil {
            mawaqi.push(MawqiTathbeet {
                ism: luba.ism,
                jidhr_luba: luba.jidhr,
                jidhr_nusakh: nusakh,
            });
        }
    }
    Ok(mawaqi)
}

/// Runs the Phase 20G library-wide restore and streams each game's outcome.
///
/// [`istiada_al_maktaba`] is invoked once per entry rather than once over the
/// whole slice for exactly one reason: it reports only when it returns, and
/// this flow owes the user a line per game while the sweep is running. Its
/// semantics survive intact — the policy is the one the detail screen's
/// removal passes ([`SiyasatIstiada::Muhafiza`], `azil_ruqaa` with
/// `sarim: false`), the settings restorer is the same [`RadLaShay`], the sweep
/// still never aborts on a failure, and every outcome is kept in one
/// [`TaqreerMaktaba`] exactly as a single call would have kept it.
fn istaid_al_kul(mawaqi: &[MawqiTathbeet], musajjil: &mut Musajjil) -> TaqreerMaktaba {
    let majmu = mawaqi.len();
    let mut radd = RadLaShay;
    let mut kulli = TaqreerMaktaba {
        alaab: Vec::with_capacity(majmu),
    };

    for (fihris, mawqi) in mawaqi.iter().enumerate() {
        musajjil.sattir(&format!(
            "[{}/{majmu}] {} — جارٍ إعادة الملفات الأصلية: {}",
            fihris.saturating_add(1),
            mawqi.ism,
            mawqi.jidhr_luba.display()
        ));
        let juzi = istiada_al_maktaba(
            std::slice::from_ref(mawqi),
            SiyasatIstiada::Muhafiza,
            &mut radd,
        );
        for natija in &juzi.alaab {
            for satr in natija.kul.taqreer() {
                musajjil.sattir(&format!("  {satr}"));
            }
            for khata in natija.kul.akhta() {
                let kamil = Khata::min_tafsir(khata);
                musajjil.sattir(&format!("  [{}] {}", kamil.ramz, kamil.arabi));
                musajjil.sattir(&format!("  {}", kamil.li_sijill()));
            }
        }
        kulli.alaab.extend(juzi.alaab);
    }
    kulli
}

/// The final report: one Arabic verdict, the sweep's own lines verbatim, then
/// every failure again with its code, its Arabic sentence and its full detail.
///
/// The sweep's report is surfaced whole — no line dropped, no failure folded
/// into a count — because that is the promise [`istiada_al_maktaba`] makes and
/// this flow merely carries. The failures are then repeated with their
/// permanent codes and remedies, so the sentence a user reads and the line a
/// maintainer greps for are both in the same block.
#[must_use]
fn sutur_khitamiya(taqreer: &TaqreerMaktaba) -> Vec<String> {
    let mut sutur = vec!["— تقرير الاستعادة الكامل —".to_owned()];
    sutur.push(if taqreer.najahat() {
        "أُزيل محتوى تعريب من كل الألعاب وعادت ملفاتها الأصلية كما كانت.".to_owned()
    } else {
        "لم تكتمل الاستعادة في بعض الألعاب؛ كل إخفاقة مذكورة أدناه باسمها ورمزها ومسارها وخطوة علاجها."
            .to_owned()
    });
    sutur.push(format!(
        "النتيجة: {} نجحت، و{} أخفقت، و{} لم يكن فيها شيء مثبَّت، من أصل {}.",
        taqreer.adad_najah(),
        taqreer.adad_fashal(),
        taqreer.adad_faragh(),
        taqreer.alaab.len()
    ));
    sutur.extend(taqreer.taqreer());
    for luba in &taqreer.alaab {
        for khata in luba.kul.akhta() {
            let kamil = Khata::min_tafsir(khata);
            sutur.push(format!("{} — [{}] {}", luba.ism, kamil.ramz, kamil.arabi));
            sutur.push(format!("  {}", kamil.li_sijill()));
        }
    }
    sutur
}

/// Asks the yes/no question, preferring the native dialog and falling back to
/// stdin only when no display can answer.
///
/// `rfd`'s synchronous dialog has no error channel: on Linux a missing portal
/// or a missing `zenity` comes back as [`MessageDialogResult::Cancel`], which
/// a two-button question can never produce from a person — so `Cancel` (and
/// anything else that is not a yes or a no) routes to the stdin fallback. The
/// display itself is checked first, because with no display server there is
/// nothing worth asking to fail.
fn istifham(jumla: &str, musajjil: &mut Musajjil) -> bool {
    if shasha_mutaha() {
        let natija = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("تعريب — الاستعادة قبل الإزالة")
            .set_description(jumla)
            .set_buttons(MessageButtons::YesNo)
            .show();
        match natija {
            MessageDialogResult::Yes => {
                musajjil.sattir("الجواب: نعم — تبدأ الاستعادة الآن.");
                return true;
            },
            MessageDialogResult::No => {
                musajjil.sattir("الجواب: لا.");
                return false;
            },
            MessageDialogResult::Ok
            | MessageDialogResult::Cancel
            | MessageDialogResult::Custom(_) => {
                musajjil.sattir("تعذّر عرض نافذة التأكيد؛ سيُطلب الجواب من سطر الأوامر.");
            },
        }
    }
    jawab_min_mudkhal(musajjil)
}

/// The stdin fallback: prints the question, reads one line, and treats
/// anything that is not an explicit yes as a no.
///
/// A closed stdin — an uninstaller running with no terminal at all — reads as
/// end of file and therefore as a no, because writing into somebody's games
/// on an unanswered question is the one thing this flow must never do. The
/// backups and manifests stay in the data root either way, so a declined or
/// unanswerable offer loses nothing that a later `--istiada` cannot restore.
fn jawab_min_mudkhal(musajjil: &mut Musajjil) -> bool {
    musajjil.sattir("أدخل الجواب ثم اضغط إدخال: نعم [y] / لا [n]");
    let mut satr = String::new();
    if std::io::stdin().read_line(&mut satr).is_err() {
        musajjil.sattir("تعذّرت قراءة الجواب من سطر الأوامر؛ عُدَّ الجواب «لا».");
        return false;
    }
    let jawab = satr.trim();
    let naam = jawab.eq_ignore_ascii_case("y")
        || jawab.eq_ignore_ascii_case("yes")
        || jawab == "نعم"
        || jawab == "ن";
    musajjil.sattir(if naam {
        "الجواب: نعم — تبدأ الاستعادة الآن."
    } else {
        "الجواب: لا."
    });
    naam
}

/// Shows the final report in a dialog, for the user who never saw stdout.
///
/// The text is the same block that already went to stdout and the log, whole.
/// A dialog that cannot be shown is not a failure of the flow: by the time
/// this runs, the report has been printed and written twice.
fn aarid_taqreer(sutur: &[String], najahat: bool) {
    let mustawa = if najahat {
        MessageLevel::Info
    } else {
        MessageLevel::Warning
    };
    let _ = MessageDialog::new()
        .set_level(mustawa)
        .set_title("تعريب — تقرير الاستعادة")
        .set_description(sutur.join("\n"))
        .set_buttons(MessageButtons::Ok)
        .show();
}

/// Whether a display can host a dialog at all.
///
/// On Linux the display server announces itself in the environment — these
/// are the platform's own variables, not Taarib configuration, which is why
/// they are read here rather than resolved through the settings store — and
/// their absence (an SSH session, a headless uninstall) is exactly the case
/// the stdin fallback exists for. Windows and macOS uninstalls always run
/// inside a session that can draw.
fn shasha_mutaha() -> bool {
    if cfg!(target_os = "linux") {
        #[expect(
            clippy::disallowed_methods,
            reason = "the display server announces itself in the environment; whether a screen \
                      exists is not a setting the configuration layer can answer"
        )]
        let mawjud = |ism: &str| std::env::var_os(ism).is_some_and(|qeema| !qeema.is_empty());
        mawjud("DISPLAY") || mawjud("WAYLAND_DISPLAY")
    } else {
        true
    }
}

/// The confirmation sentence, with the count in the grammatical form Arabic
/// gives it.
///
/// The six-form rule the interface applies through its plural string families
/// (`lugha.ts` `jam`), written out plainly here because this flow runs before
/// any interface exists: zero, one, two, three-to-ten and their compounds, and
/// everything above ten. The zero arm is unreachable — an empty library
/// returned before the question — and is kept so the rule is stated whole.
///
/// The rule's last two forms — eleven-to-ninety-nine by the last two digits,
/// and the hundreds that wrap past them — put the noun in the same singular
/// accusative, so one arm spells both.
#[must_use]
fn jumlat_takid(adad: usize) -> String {
    match (adad, adad % 100) {
        (0, _) => "لا توجد ألعاب عليها محتوى مثبَّت من تعريب.".to_owned(),
        (1, _) => "توجد لعبة واحدة عليها محتوى مثبَّت من تعريب. أتريد إزالة المحتوى وإعادة ملفاتها \
             الأصلية قبل إزالة التطبيق؟"
            .to_owned(),
        (2, _) => "توجد لعبتان عليهما محتوى مثبَّت من تعريب. أتريد إزالة المحتوى وإعادة ملفاتهما \
             الأصلية قبل إزالة التطبيق؟"
            .to_owned(),
        (_, 3..=10) => format!(
            "توجد {adad} ألعاب عليها محتوى مثبَّت من تعريب. أتريد إزالة المحتوى وإعادة ملفاتها \
             الأصلية قبل إزالة التطبيق؟"
        ),
        _ => format!(
            "توجد {adad} لعبة عليها محتوى مثبَّت من تعريب. أتريد إزالة المحتوى وإعادة ملفاتها \
             الأصلية قبل إزالة التطبيق؟"
        ),
    }
}

/// Writes every line of the flow to stdout and, once it opened, to the log.
///
/// The one output surface the flow has, so stdout and the file can never
/// disagree about what was said. `writeln!` on a locked handle rather than
/// `println!`, because a closed stdout — an uninstaller detached from any
/// console, a pipe torn down mid-run — must be ignored, never panicked on.
#[derive(Debug)]
struct Musajjil {
    malaf: Option<std::fs::File>,
}

impl Musajjil {
    /// Starts with stdout alone; the log joins once the data root is known.
    const fn jadeed() -> Self {
        Self { malaf: None }
    }

    /// Opens the flow's log file, appending so an earlier interrupted run's
    /// record survives beside this one. Failure costs the file, not the flow.
    fn iftah_malaf(&mut self, masar: &Path) {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(masar)
        {
            Ok(malaf) => self.malaf = Some(malaf),
            Err(sabab) => self.sattir(&format!(
                "تعذّر فتح ملف السجل {}؛ سيستمر العرض على سطر الأوامر وحده. ({sabab})",
                masar.display()
            )),
        }
    }

    /// One line, to both surfaces, flushed so the stream is live.
    fn sattir(&mut self, satr: &str) {
        {
            let mut kharj = std::io::stdout().lock();
            let _ = writeln!(kharj, "{satr}");
            let _ = kharj.flush();
        }
        let fashal = self
            .malaf
            .as_mut()
            .is_some_and(|malaf| writeln!(malaf, "{satr}").is_err());
        if fashal {
            // Said once, then stdout carries on alone: repeating the warning
            // on every remaining line would bury the report it apologises for.
            self.malaf = None;
            let mut kharj = std::io::stdout().lock();
            let _ = writeln!(
                kharj,
                "تعذّرت الكتابة إلى ملف السجل؛ سيستمر العرض على سطر الأوامر وحده."
            );
        }
    }

    /// Prints a failure in both languages — the Arabic sentence a person
    /// reads, then the coded line a maintainer greps for — and closes the
    /// record. The uninstall proceeds; that is [`JUMLAT_MUTABAA`]'s job.
    fn inha_bi_khata(&mut self, khata: &Khata) {
        self.sattir(&khata.arabi);
        self.sattir(&khata.li_sijill());
        self.sattir(JUMLAT_MUTABAA);
        self.ikhtim();
    }

    /// Flushes the log to the device, once the flow is done with it.
    fn ikhtim(&mut self) {
        if let Some(malaf) = self.malaf.take() {
            let _ = malaf.sync_all();
        }
    }
}
