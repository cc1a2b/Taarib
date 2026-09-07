//! مولّد المخطّطات — writes `schemas/` from the vocabulary this crate defines.
//!
//! `schemas/` is the third product of one set of definitions. The Rust type is
//! the first, the TypeScript type Studio imports is the second, and the JSON
//! Schema the registry validates its records against is this one. Nothing in
//! the product describes a game, a build, a patch, a string or a contributor
//! twice, and this binary is the half of that promise that faces the registry.
//!
//! Run it from anywhere in the workspace:
//!
//! ```text
//! cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat
//! ```
//!
//! Everything it writes is derived. The title is the Rust type's name, the file
//! name is that name in snake case, the description is the opening paragraph of
//! the type's own doc comment, and the body is what `schemars` produces for the
//! type. A file edited by hand in `schemas/` is reverted by the next run; the
//! type is what gets edited, and the schema follows.
//!
//! Two things are added on top of what `schemars` emits, both deliberate:
//!
//! * an `$id` per file, so a stored record can name the schema it was written
//!   against and a fork can resolve that schema without cloning the workspace;
//! * a fixed key order — `$schema`, `$id`, `title`, `description`, the type
//!   itself, then `$defs` — because `schemars` appends its metadata after the
//!   body, and a committed file that reshuffles on every run turns a one-field
//!   change into an unreadable diff.

use std::error::Error;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use schemars::generate::{SchemaGenerator, SchemaSettings};
use serde::Serialize;
use serde_json::{Map, Value};
use taarib_mustalahat::{
    AlamJawda, Basma, BinaId, HalatLuba, HalatRuqaa, Luba, LubaId, MasdarLuba, MudkhalNass,
    Muharrik, MulakhkhasRuqaa, Musahim, MusahimId, MutabaqaBina, NassId, NitaqNasq, RukhsaRuqaa,
    RuqaaId, Sumaa, Tabaqa, Taghtiya, TaqreerImkaniyat, TareeqaTarjama,
};
use taarib_usus::Khata;
use taarib_usus::khata::RisalatMustakhdim;
use taarib_usus::masarat;

/// The meta-schema every generated file declares.
const MITA_MUKHATTAT: &str = "https://json-schema.org/draft/2020-12/schema";

/// The base every `$id` is built from: the raw view of `schemas/` on the
/// default branch. It is a URL a validator, a fork, or an unrelated tool can
/// fetch, which is the whole point of committing the schemas rather than
/// generating them at install time.
const JIDHR_MUARRIF: &str = "https://raw.githubusercontent.com/cc1a2b/taarib/main/schemas";

/// The index file every consumer reads first.
const MALAF_FAHRAS: &str = "fahras.json";

/// The one hand-written schema, which the sweep in [`ihdhif_matruka`] must not
/// take for a leftover: it describes `assets/basmat/basmat.json` and is
/// deliberately outside [`MALAF_FAHRAS`], because nothing derives it.
const MALAF_BASMAT: &str = "basmat.schema.json";

/// The index's own title.
const UNWAN_FAHRAS: &str = "Fahras";

/// What the index says about itself.
const WASF_FAHRAS: &str = "The index of the JSON Schemas generated from the Taarib vocabulary in \
                           crates/taarib-mustalahat.";

/// The version of the index format, carried under the `mukhattat` key every
/// persisted Taarib structure uses. It moves when the shape of `fahras.json`
/// moves, never when a schema inside it does.
const ISDAR_FAHRAS: u32 = 1;

/// Produces one file and reports what the index records about it.
type Muwallid = fn(&Path) -> Result<Madkhal, Box<dyn Error>>;

/// Every vocabulary type that gets a file of its own, in the order the index
/// lists them: the game and where it came from, the build it is at, the engine
/// behind it, its strings, the patches over those strings, the contributors who
/// made them, and the error model underneath all of it.
///
/// A type reached only from one of these — `SuwarLuba`, `Mustatil`, `Daleel`,
/// `Ramz` and the rest — is defined under the `$defs` of whichever files reach
/// it, and does not get a file. A type that a registry record can be, that a
/// record refers to by name, or that crosses into the frontend on its own —
/// `HalatLuba` and `RisalatMustakhdim` are both computed rather than stored, and
/// both are rendered — does.
const MUKHATTATAT: [Muwallid; 27] = [
    mukhattat::<Luba>,
    mukhattat::<LubaId>,
    mukhattat::<MasdarLuba>,
    mukhattat::<HalatLuba>,
    mukhattat::<taarib_mustalahat::luba::HukmLughaRasmiya>,
    mukhattat::<BinaId>,
    mukhattat::<Basma>,
    mukhattat::<MutabaqaBina>,
    mukhattat::<Muharrik>,
    mukhattat::<TaqreerImkaniyat>,
    mukhattat::<Tabaqa>,
    mukhattat::<MudkhalNass>,
    mukhattat::<NassId>,
    mukhattat::<taarib_mustalahat::muraja::HalatMuraja>,
    mukhattat::<NitaqNasq>,
    mukhattat::<AlamJawda>,
    mukhattat::<MulakhkhasRuqaa>,
    mukhattat::<RuqaaId>,
    mukhattat::<HalatRuqaa>,
    mukhattat::<TareeqaTarjama>,
    mukhattat::<RukhsaRuqaa>,
    mukhattat::<Taghtiya>,
    mukhattat::<Musahim>,
    mukhattat::<MusahimId>,
    mukhattat::<Sumaa>,
    mukhattat::<Khata>,
    mukhattat::<RisalatMustakhdim>,
];

/// One generated file, as `fahras.json` records it.
#[derive(Debug, Serialize)]
struct Madkhal {
    /// The schema's title, which is the Rust type's name.
    unwan: String,
    /// The file name inside `schemas/`.
    malaf: String,
    /// The schema's own `$id`.
    #[serde(rename = "$id")]
    muarrif: String,
    /// The one-line description the schema carries.
    wasf: String,
    /// Bytes written. Reported in the summary, never recorded in the index,
    /// because a byte count in a committed file changes on every reformat.
    #[serde(skip)]
    hajm: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    // The workspace root is resolved from the manifest directory rather than
    // from the current directory, so the output lands in the same place whether
    // this is run from the workspace root, from the crate, or from an editor
    // with a working directory of its own.
    let bayan = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let jidhr = bayan
        .parent()
        .and_then(Path::parent)
        .ok_or("CARGO_MANIFEST_DIR does not sit two levels below the workspace root")?;
    let mujallad = jidhr.join("schemas");

    let mut madakhil = Vec::with_capacity(MUKHATTATAT.len());
    for tawlid in MUKHATTATAT {
        madakhil.push(tawlid(&mujallad)?);
    }
    let hajm_fahras = iktub_fahras(&mujallad, &madakhil)?;
    let matruka = ihdhif_matruka(&mujallad, &madakhil)?;

    let mut khuruj = std::io::stdout().lock();
    for madkhal in &madakhil {
        writeln!(
            khuruj,
            "{:<24} {:<20} {} bytes",
            madkhal.malaf, madkhal.unwan, madkhal.hajm
        )?;
    }
    writeln!(
        khuruj,
        "{MALAF_FAHRAS:<24} {UNWAN_FAHRAS:<20} {hajm_fahras} bytes"
    )?;
    for malaf in &matruka {
        writeln!(khuruj, "{malaf:<24} {:<20} removed", "—")?;
    }
    khuruj.flush()?;

    Ok(())
}

/// Deletes any schema left behind by a type that no longer exists.
///
/// Renaming a type used to leave its old file in place: it dropped out of
/// `fahras.json`, so nothing following the index could find it, while its `$id`
/// went on answering — validating a state no build can read and rejecting the
/// ones every current record uses. `HalatTarjama` did exactly that after it
/// became `HalatMuraja`. Writing without deleting is what made a rename
/// silently publish a lie, so the sweep belongs here rather than in a reviewer's
/// memory.
///
/// `basmat.schema.json` is hand-written and deliberately absent from
/// `fahras.json`, so it is preserved by name.
///
/// # Errors
///
/// When the directory cannot be read, or a stale file cannot be removed.
fn ihdhif_matruka(mujallad: &Path, madakhil: &[Madkhal]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut matruka = Vec::new();
    for madkhal in std::fs::read_dir(mujallad)? {
        let madkhal = madkhal?;
        let ism = madkhal.file_name().to_string_lossy().into_owned();
        if !ism.ends_with(".json")
            || ism == MALAF_FAHRAS
            || ism == MALAF_BASMAT
            || madakhil.iter().any(|m| m.malaf == ism)
        {
            continue;
        }
        std::fs::remove_file(madkhal.path())?;
        matruka.push(ism);
    }
    matruka.sort_unstable();
    Ok(matruka)
}

/// A generator configured the way every file in `schemas/` is written.
///
/// Draft 2020-12, with referenced subschemas collected under `$defs` instead of
/// inlined: `Basma` appears in four records and `AilatMuharrik` in three, and a
/// definition copied into every use is a definition that can be edited in one
/// place and not the others.
fn muwallid() -> SchemaGenerator {
    SchemaSettings::draft2020_12()
        .with(|idad| {
            idad.definitions_path = "/$defs".into();
            idad.inline_subschemas = false;
        })
        .into_generator()
}

/// Generates one type's schema and writes it.
///
/// # Errors
///
/// Fails when the type does not generate a JSON object — which would mean it
/// cannot carry the `$id` and `title` every file here declares — and when the
/// file cannot be serialized or written.
fn mukhattat<T: JsonSchema>(mujallad: &Path) -> Result<Madkhal, Box<dyn Error>> {
    let unwan = T::schema_name().into_owned();
    let malaf = ism_malaf(&unwan);
    let muarrif = format!("{JIDHR_MUARRIF}/{malaf}");

    let mut khaam = muwallid().into_root_schema_for::<T>().to_value();
    awjiz_shajara(&mut khaam);

    let Value::Object(jism) = khaam else {
        return Err(format!("{unwan} does not generate a JSON object").into());
    };
    let wasf = jism
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let mut nass = serde_json::to_string_pretty(&Value::Object(rattib(&unwan, &muarrif, jism)))?;
    nass.push('\n');
    masarat::kitaba_dharra_nass(&mujallad.join(&malaf), &nass)?;

    Ok(Madkhal {
        unwan,
        malaf,
        muarrif,
        wasf,
        hajm: nass.len(),
    })
}

/// Writes the index.
///
/// # Errors
///
/// Fails when the index cannot be serialized or written.
fn iktub_fahras(mujallad: &Path, madakhil: &[Madkhal]) -> Result<usize, Box<dyn Error>> {
    let mut fahras = Map::with_capacity(5);
    let _ = fahras.insert(
        "$id".to_owned(),
        format!("{JIDHR_MUARRIF}/{MALAF_FAHRAS}").into(),
    );
    let _ = fahras.insert("title".to_owned(), UNWAN_FAHRAS.into());
    let _ = fahras.insert("description".to_owned(), WASF_FAHRAS.into());
    let _ = fahras.insert("mukhattat".to_owned(), ISDAR_FAHRAS.into());
    let _ = fahras.insert("mukhattatat".to_owned(), serde_json::to_value(madakhil)?);

    let mut nass = serde_json::to_string_pretty(&Value::Object(fahras))?;
    nass.push('\n');
    masarat::kitaba_dharra_nass(&mujallad.join(MALAF_FAHRAS), &nass)?;

    Ok(nass.len())
}

/// Rebuilds a generated schema in the fixed key order every committed file
/// carries, and stamps it with its `$id`.
///
/// `schemars` appends `title`, `$schema` and `$defs` after the body it built,
/// which is correct and unreadable. The identifying keys move to the front, the
/// definitions to the back, and the body keeps the order the generator gave it
/// so that `properties` still matches the order of the fields in the Rust type.
fn rattib(unwan: &str, muarrif: &str, jism: Map<String, Value>) -> Map<String, Value> {
    let mut jasad = Map::with_capacity(jism.len());
    let mut wasf = None;
    let mut tarifat = None;

    for (miftah, qeema) in jism {
        match miftah.as_str() {
            "description" => wasf = Some(qeema),
            "$defs" => tarifat = Some(qeema),
            "$schema" | "$id" | "title" => {},
            _ => {
                let _ = jasad.insert(miftah, qeema);
            },
        }
    }

    let mut wathiqa = Map::with_capacity(jasad.len() + 5);
    let _ = wathiqa.insert("$schema".to_owned(), MITA_MUKHATTAT.into());
    let _ = wathiqa.insert("$id".to_owned(), muarrif.into());
    let _ = wathiqa.insert("title".to_owned(), unwan.into());
    if let Some(wasf) = wasf {
        let _ = wathiqa.insert("description".to_owned(), wasf);
    }
    for (miftah, qeema) in jasad {
        let _ = wathiqa.insert(miftah, qeema);
    }
    if let Some(tarifat) = tarifat {
        let _ = wathiqa.insert("$defs".to_owned(), tarifat);
    }

    wathiqa
}

/// The file a type is written to: its name in snake case, derived exactly the
/// way `serde` derives a `rename_all = "snake_case"` name, so the file name,
/// the Rust type and the generated TypeScript cannot drift apart.
fn ism_malaf(unwan: &str) -> String {
    let mut ism = String::with_capacity(unwan.len() + 8);
    for (mawqi, harf) in unwan.char_indices() {
        if mawqi > 0 && harf.is_uppercase() {
            ism.push('_');
        }
        ism.extend(harf.to_lowercase());
    }
    ism.push_str(".json");
    ism
}

/// The opening paragraph of a doc comment, with its line wrapping removed.
///
/// A doc comment is written for someone reading the crate: a summary, a blank
/// line, then the reasoning behind the design. A schema `description` is read
/// inside a validator's error message and on a registry listing, so only the
/// summary survives, and the hard wrapping the source file uses is collapsed
/// back into a single line.
fn awjiz(wasf: &str) -> String {
    let fiqra = wasf.split("\n\n").next().unwrap_or(wasf);
    let mut mawjaz = String::with_capacity(fiqra.len());
    for kalima in fiqra.split_whitespace() {
        if !mawjaz.is_empty() {
            mawjaz.push(' ');
        }
        mawjaz.push_str(kalima);
    }
    mawjaz
}

/// Applies [`awjiz`] to every description in a generated document.
///
/// Descriptions appear at four depths — the type, each of its fields, each
/// variant of an enum, and everything under `$defs` — so walking the whole
/// document is the only way to reach all of them.
fn awjiz_shajara(qeema: &mut Value) {
    match qeema {
        Value::Object(kain) => {
            // A property genuinely named `description` holds a schema object,
            // not a string, so the pattern below cannot mistake one for a
            // description of its own.
            if let Some(Value::String(wasf)) = kain.get_mut("description") {
                *wasf = awjiz(wasf);
            }
            for far in kain.values_mut() {
                awjiz_shajara(far);
            }
        },
        Value::Array(qaima) => {
            for far in qaima {
                awjiz_shajara(far);
            }
        },
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {},
    }
}
