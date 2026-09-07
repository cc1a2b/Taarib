//! الدلائل — the four independent sources of evidence.
//!
//! The ROADMAP names four, and the module tree is those four rather than one
//! module per engine. That is deliberate: an engine is identified by the
//! *agreement* of independent sources, and a tree organised by engine would push
//! every detector towards answering on its own.
//!
//! | source | modules | what it reads |
//! | --- | --- | --- |
//! | directory shape | [`binya`] | the layouts an engine's own runtime has to find, so it has to ship them |
//! | binary signatures | [`thunai`] | imported modules, section names, and version tags in the executable's constant data |
//! | embedded metadata | [`unity`], [`nusus`] | the engine's own declaration of its version, inside its own files |
//! | container headers | [`unreal`], [`godot`], [`bio4`], [`khassa`] | the framing of the archives an engine ships its content in |
//!
//! Several modules straddle two sources — [`unity`] reads both a container
//! header and a metadata blob — and that is fine. The point of the four is that
//! no single failure takes out more than one of them: a packed executable
//! defeats the binary reader and leaves the directory shape intact; a renamed
//! directory defeats the shape and leaves the container header intact.
//!
//! ## Overlap is intentional
//!
//! [`binya`] records that a game has a `.pak` directory; [`unreal`] reads that
//! pak's footer. Both are kept. [`crate::tahdid`] combines by taking the
//! strongest weight per engine family rather than by summing, so agreement
//! raises confidence through the corroboration term without letting six weak
//! sightings of the same fact outrank one conclusive one.
//!
//! The alternative — making each detector defer to a more authoritative one —
//! was rejected because it makes every detector depend on another's success. A
//! game whose pak footer is corrupt should still be identified as Unreal by its
//! directory shape, and it is.
//!
//! ## One reader per format, however many detectors read it
//!
//! [`tanfidhi`] is not a detector and answers nothing. It is the single parser
//! of a Windows PE image and its version resource, which [`bio4`] and [`khassa`]
//! both need. It exists because they each had their own copy of it and the two
//! copies disagreed — one of them reporting an engine's internal name as
//! `t(\u{1}LegalCopyright` and finding nothing at all in a binary whose version
//! block sits past a megabyte. Overlapping *evidence* is the design; overlapping
//! *parsers* is a defect that only shows up in one of them.

pub mod binya;
pub mod bio4;
pub mod godot;
pub mod khassa;
pub mod nusus;
pub mod tanfidhi;
pub mod thunai;
pub mod unity;
pub mod unreal;

use crate::fahs::Fahis;

pub use crate::dalail::binya::FahisBinya;
pub use crate::dalail::bio4::FahisBio4;
pub use crate::dalail::godot::FahisGodot;
pub use crate::dalail::khassa::{
    FahisAlchemy, FahisBlackSpace, FahisDantelion, FahisFrostbite, FahisRage, FahisSnowdrop,
};
pub use crate::dalail::nusus::{FahisNusus, HadafNusus};
pub use crate::dalail::thunai::FahisThunai;
pub use crate::dalail::unity::FahisUnity;
pub use crate::dalail::unreal::FahisUnreal;

/// Every detector, in the order a probe runs them.
///
/// [`binya`] is first and that is load-bearing rather than cosmetic. It is the
/// only detector that resolves a game's executable from an engine's own naming
/// rule — `Foo_Data` implies `Foo.exe`, and an Unreal shipping binary sits three
/// levels down where nothing else will look for it. Running it first lets the
/// probe hand that executable to every detector after it, so the binary reader
/// has something to read for a game whose launcher never named one. See
/// [`crate::Mifhas::ifhas`] for the two passes that arrangement produces.
///
/// The remaining order is not significant: detectors hold no state, observe
/// rather than conclude, and are combined by weight.
#[must_use]
pub fn kul() -> Vec<Box<dyn Fahis>> {
    let mut kul: Vec<Box<dyn Fahis>> = vec![
        Box::new(FahisBinya::jadeed()),
        Box::new(FahisThunai::jadeed()),
        Box::new(FahisUnity),
        Box::new(FahisUnreal::jadeed()),
        Box::new(FahisGodot::jadeed()),
        // Last of the engine detectors, and it costs one listing of the game
        // root for every game that is not one of its own: it reads nothing else
        // until that listing has already said the layout is there.
        Box::new(FahisBio4::jadeed()),
    ];
    // Six more, one per in-house engine, each gated on one listing of the game
    // root the same way the detector above it is. They are last because they
    // are the newest and because none of them can contradict anything in front
    // of them: an engine nobody licenses does not look like Unity.
    kul.extend(khassa::jamee());
    // Five detectors rather than one, because a single result carries a single
    // engine family and an RPG Maker project inside NW.js is genuinely two.
    for wahid in FahisNusus::jamee() {
        kul.push(Box::new(wahid));
    }
    kul
}

/// Whether a file name's extension is `matlub`, compared without regard to case.
///
/// `matlub` is the extension without its leading dot.
///
/// Case-insensitively because these names come off Windows and macOS
/// filesystems, which preserve whatever case the packager typed: `Game.EXE`,
/// `data.WIN` and `Assembly-CSharp.DLL` are all routine. A case-sensitive test
/// identifies such a game on Windows and reports it unsupported on Linux, where
/// the same bytes read back with the case intact.
///
/// Split on the last dot rather than through `Path::extension`, because these
/// are bare file names that may begin with one — `Path::extension` reports none
/// for `.app`, where a suffix test matches.
pub(crate) fn imtidad(ism: &str, matlub: &str) -> bool {
    ism.rsplit_once('.').is_some_and(|(_, lahiqa)| lahiqa.eq_ignore_ascii_case(matlub))
}
