//! لوحة التحكّم — the in-game control panel, drawn by Taarib's own renderer.
//!
//! Every other overlay of this kind reaches for a UI library. This one cannot,
//! and would not want to: the panel is drawn inside a game that has never heard
//! of Taarib, through the game's own device, in the four milliseconds between
//! the last draw call and the present. There is no window to put a widget in,
//! no event loop to pump, and no guarantee that any toolkit exists in the
//! process. So the panel is a list of rectangles with roles, and the same quad
//! batch that draws translated dialogue draws it.
//!
//! ## Why `bina` produces elements and not quads
//!
//! [`LawhatTahakkum::bina`] returns [`AnsurLawha`] — a rectangle, a role, a
//! colour and possibly a string — in **normalized** coordinates. It does not
//! return [`crate::wajiha::QitaRasm`], and the distinction is the single most
//! important rule in this crate: **there is no second text path in this
//! product.** Every glyph the overlay draws, the panel's own labels included,
//! goes through `saff` for shaping and comes out of Taarib's atlas through
//! `jisr`. A panel that turned a label into quads here would be the one place
//! in the product where Arabic could be laid out wrongly with nothing else to
//! catch it — no shaping tests, no atlas, no bidi. So this module lays out
//! boxes, one layer up shapes the text into them, and the strings that leave
//! here are strings.
//!
//! It is also why nothing here wraps a line. Wrapping requires measuring, and
//! measuring Arabic requires shaping it, which happens above. A long line is
//! emitted whole and the layer that can measure it decides where it breaks.
//!
//! ## Focus is a decision, not a side effect
//!
//! [`HalatLawha::yabtali_dakhl`] exists because a keystroke has exactly one
//! owner. When the panel has focus and the player types a region name, the game
//! must not also receive those keys — otherwise naming a region "north gate"
//! walks the character north, opens the inventory and swings a sword. And when
//! the panel does *not* have focus, the game must receive everything, because a
//! translation overlay that eats input is an overlay a player uninstalls. There
//! is no third behaviour and no heuristic; the hook asks this method and obeys
//! the answer.
//!
//! The global shortcuts in [`Ikhtisarat`] are the deliberate exception: they are
//! consulted before the focus question is asked, because the chord that opens
//! the panel has to work while the panel is closed.
//!
//! ## Two pages quote rather than summarise
//!
//! The budget page shows [`crate::wajiha::MeezaniyatItar::wasf`] verbatim, and
//! the disclosure page shows [`crate::sidq::NASS_IFSAH_ARABI`] verbatim. Neither
//! is reworded, shortened or paraphrased anywhere in this module. Both texts are
//! the product's honest account of what tier 3 costs, and a panel that
//! summarised them would be a panel that eventually summarised them favourably —
//! nobody rewrites a disclosure to make it more alarming.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::khata::KhataTabaqa;
use crate::manatiq::{MajmuatManatiq, MuarrifMintaqa, QaidatTarjama};
use crate::qissa::HalatKhayt;
use crate::sidq::{self, BasmatIfsah, NASS_IFSAH_ARABI};
use crate::sijill_qira::{MadkhalQira, MuarrifMadkhal, TaqreerSijill};
use crate::wajiha::{HalatTabaqa, MeezaniyatItar, MustatilNisbi, WajihatRusum, WasfSath};

/// The file the keyboard bindings are stored in.
pub const ISM_MALAF_IKHTISARAT: &str = "ikhtisarat.json";

/// Everything the panel can be asked to do.
///
/// One flat list rather than a tree of menus. A player looking for "turn it
/// off" while a boss is on screen should not be navigating, and every entry
/// here has a chord that reaches it from anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Amr {
    /// Turn the overlay's drawing on or off.
    Tabdeel,
    /// Capture, read and translate every region right now.
    TarjimAlan,
    /// Open the region editor.
    TahrirManatiq,
    /// Open the reading history.
    ArdSijill,
    /// Make the panel more opaque.
    ZidAtaama,
    /// Make the panel more transparent.
    AnqisAtaama,
    /// Make the panel's text larger.
    ZidHajmKhat,
    /// Make the panel's text smaller.
    AnqisHajmKhat,
    /// Stand the overlay down without unhooking it.
    Iqaf,
    /// Show the tier-3 disclosure again.
    ArdIfsah,
    /// Close the panel, leaving the overlay as it was.
    Ighlaq,
}

impl Amr {
    /// Every command, in the order the main page lists them.
    #[must_use]
    pub const fn jamee() -> [Self; 11] {
        [
            Self::Tabdeel,
            Self::TarjimAlan,
            Self::TahrirManatiq,
            Self::ArdSijill,
            Self::Iqaf,
            Self::ZidAtaama,
            Self::AnqisAtaama,
            Self::ZidHajmKhat,
            Self::AnqisHajmKhat,
            Self::ArdIfsah,
            Self::Ighlaq,
        ]
    }

    /// The name used in the bindings file, in log lines and in a bug report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Tabdeel => "toggle",
            Self::TarjimAlan => "translate now",
            Self::TahrirManatiq => "edit regions",
            Self::ArdSijill => "show history",
            Self::ZidAtaama => "opacity up",
            Self::AnqisAtaama => "opacity down",
            Self::ZidHajmKhat => "font size up",
            Self::AnqisHajmKhat => "font size down",
            Self::Iqaf => "pause",
            Self::ArdIfsah => "show disclosure",
            Self::Ighlaq => "close",
        }
    }

    /// The label the panel shows, in Arabic.
    #[must_use]
    pub const fn unwan(self) -> &'static str {
        match self {
            Self::Tabdeel => "تشغيل/إيقاف الطبقة",
            Self::TarjimAlan => "ترجم الآن",
            Self::TahrirManatiq => "تحرير المناطق",
            Self::ArdSijill => "سجلّ القراءة",
            Self::ZidAtaama => "زيادة العتامة",
            Self::AnqisAtaama => "إنقاص العتامة",
            Self::ZidHajmKhat => "تكبير الخط",
            Self::AnqisHajmKhat => "تصغير الخط",
            Self::Iqaf => "إيقاف مؤقّت",
            Self::ArdIfsah => "عرض شرح الطبقة",
            Self::Ighlaq => "إغلاق اللوحة",
        }
    }

    /// What this command does, and what it costs when the answer is not obvious.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Tabdeel => {
                "Stops the overlay drawing and stops it capturing. The hooks stay installed, \
                 so turning it back on is immediate."
            },
            Self::TarjimAlan => {
                "Captures, reads and translates every enabled region on this frame, whatever \
                 their rules say. This is the one action that ignores the rate limit, so it \
                 costs one full pass over every region at once."
            },
            Self::TahrirManatiq => {
                "Opens the region editor: draw a new region, drag one to move it, drag a \
                 corner to resize it. The game keeps running underneath."
            },
            Self::ArdSijill => {
                "Shows every line that was read and translated this session, so a line that \
                 left the screen before you finished reading it can be read again."
            },
            Self::ZidAtaama => "Makes the panel more opaque, in steps of five percent.",
            Self::AnqisAtaama => {
                "Makes the panel more transparent, in steps of five percent, down to a floor \
                 that keeps it visible — a panel faded to nothing still swallows the keys \
                 you type at it."
            },
            Self::ZidHajmKhat => "Enlarges the panel's own text. Does not affect the overlay.",
            Self::AnqisHajmKhat => "Shrinks the panel's own text. Does not affect the overlay.",
            Self::Iqaf => {
                "Stands the overlay down without unhooking it, for a cutscene or a fight \
                 where the frame cost matters more than the text."
            },
            Self::ArdIfsah => {
                "Shows the tier-3 disclosure again, in full and unchanged, whenever you want \
                 to read what this method actually costs."
            },
            Self::Ighlaq => {
                "Closes the panel and gives every key back to the game. The overlay keeps \
                 doing whatever it was doing."
            },
        }
    }

    /// Whether this command changes the overlay rather than the panel.
    #[must_use]
    pub const fn yamiss_tabaqa(self) -> bool {
        matches!(self, Self::Tabdeel | Self::TarjimAlan | Self::Iqaf)
    }

    /// The chord this command ships bound to.
    ///
    /// Almost everything is `Ctrl+Shift+…`. Games bind bare keys heavily and
    /// `Ctrl+key` moderately; `Ctrl+Shift+key` is the emptiest corner of the
    /// keyboard across the titles this was tried against, and the whole table
    /// moves with [`Ikhtisarat::arbut`] when a particular game disagrees.
    ///
    /// [`Amr::Ighlaq`] is the exception and is bound to bare Escape, because a
    /// panel that cannot be dismissed with Escape is a panel players fight. It
    /// is safe precisely because it is only consulted while the panel has focus
    /// — see [`HalatLawha::yabtali_dakhl`] — so a game that uses Escape for its
    /// own menu keeps it whenever the panel is not in front.
    #[must_use]
    pub const fn iftiradi(self) -> Watar {
        let tahakkum_sift = Muaddil::TAHAKKUM.ma(Muaddil::SIFT);
        match self {
            Self::Tabdeel => Watar::jadeed(tahakkum_sift, RamzMiftah::HARF_T),
            Self::TarjimAlan => Watar::jadeed(tahakkum_sift, RamzMiftah::IDKHAL),
            Self::TahrirManatiq => Watar::jadeed(tahakkum_sift, RamzMiftah::HARF_R),
            Self::ArdSijill => Watar::jadeed(tahakkum_sift, RamzMiftah::HARF_H),
            Self::ZidAtaama => Watar::jadeed(tahakkum_sift, RamzMiftah::SAHM_AALA),
            Self::AnqisAtaama => Watar::jadeed(tahakkum_sift, RamzMiftah::SAHM_ASFAL),
            Self::ZidHajmKhat => Watar::jadeed(tahakkum_sift, RamzMiftah::SAHM_YAMEEN),
            Self::AnqisHajmKhat => Watar::jadeed(tahakkum_sift, RamzMiftah::SAHM_YASAR),
            Self::Iqaf => Watar::jadeed(tahakkum_sift, RamzMiftah::HARF_P),
            Self::ArdIfsah => Watar::jadeed(tahakkum_sift, RamzMiftah::HARF_I),
            Self::Ighlaq => Watar::jadeed(Muaddil::LA, RamzMiftah::HURUB),
        }
    }
}

/// The modifier keys held with a chord, as bits.
///
/// Bits rather than a set of booleans so a chord is one comparison and one
/// number in a settings file. Left and right variants are deliberately not
/// distinguished: no player has ever meant "left control specifically", and
/// keeping them apart would produce a binding that works on one side of the
/// keyboard and silently does not on the other.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct Muaddil(u8);

impl Muaddil {
    /// No modifiers.
    pub const LA: Self = Self(0);
    /// Either shift key.
    pub const SIFT: Self = Self(1);
    /// Either control key.
    pub const TAHAKKUM: Self = Self(2);
    /// Either alt key, which is option on macOS.
    pub const BADEEL: Self = Self(4);
    /// Either windows or command key.
    pub const NAFIDHA: Self = Self(8);

    /// Every bit this build understands.
    const MAQNA: u8 = 0b1111;

    /// This set with another added.
    #[must_use]
    pub const fn ma(self, akhar: Self) -> Self {
        Self(self.0 | akhar.0)
    }

    /// This set with another removed.
    #[must_use]
    pub const fn duna(self, akhar: Self) -> Self {
        Self(self.0 & !akhar.0)
    }

    /// Whether every bit of `akhar` is present.
    #[must_use]
    pub const fn yahwi(self, akhar: Self) -> bool {
        self.0 & akhar.0 == akhar.0
    }

    /// Whether nothing is held.
    #[must_use]
    pub const fn khali(self) -> bool {
        self.0 == 0
    }

    /// The raw bits.
    #[must_use]
    pub const fn bitat(self) -> u8 {
        self.0
    }

    /// Bits from a stored value, with anything this build does not know masked
    /// off.
    ///
    /// Masked rather than trusted: a settings file written by a later build that
    /// added a fifth modifier would otherwise produce a chord that can never
    /// match, and a shortcut that silently never fires is worse than one that
    /// fires without a modifier nobody has.
    #[must_use]
    pub const fn min_bitat(bitat: u8) -> Self {
        Self(bitat & Self::MAQNA)
    }

    /// The modifiers written the way a shortcut is written, `Ctrl+Shift+`.
    ///
    /// Empty for no modifiers, and always ends with a `+` when it is not empty,
    /// so a caller concatenates the key name onto it without a separator rule of
    /// its own.
    #[must_use]
    pub fn ism(self) -> String {
        let mut ism = String::new();
        if self.yahwi(Self::TAHAKKUM) {
            ism.push_str("Ctrl+");
        }
        if self.yahwi(Self::BADEEL) {
            ism.push_str("Alt+");
        }
        if self.yahwi(Self::SIFT) {
            ism.push_str("Shift+");
        }
        if self.yahwi(Self::NAFIDHA) {
            ism.push_str("Meta+");
        }
        ism
    }
}

/// A key, as a USB HID keyboard usage identifier.
///
/// Platform-neutral, and neutral in a way that is checkable rather than
/// invented: these are the numbers from the HID Usage Tables' keyboard page,
/// which every operating system's input stack is ultimately translating *from*.
/// They are deliberately **not** Windows virtual key codes, X11 keysyms or macOS
/// key codes — a bindings file written on one platform and copied to another has
/// to mean the same thing, and a virtual key code does not survive that trip.
/// Each hook translates the platform's own numbering into this one, in one place
/// per platform.
///
/// They are also positional rather than symbolic, which is the right answer for
/// a game shortcut: `Ctrl+Shift+T` is the key where `T` is on a QWERTY board,
/// whatever the player's layout prints on it, exactly like every game's own `W`
/// `A` `S` `D`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct RamzMiftah(u16);

impl RamzMiftah {
    /// No key at all, which is not a bindable chord.
    pub const LA_SHAY: Self = Self(0);
    /// The `H` position.
    pub const HARF_H: Self = Self(0x0B);
    /// The `I` position.
    pub const HARF_I: Self = Self(0x0C);
    /// The `P` position.
    pub const HARF_P: Self = Self(0x13);
    /// The `R` position.
    pub const HARF_R: Self = Self(0x15);
    /// The `T` position.
    pub const HARF_T: Self = Self(0x17);
    /// Return or enter.
    pub const IDKHAL: Self = Self(0x28);
    /// Escape.
    pub const HURUB: Self = Self(0x29);
    /// The space bar.
    pub const MASAFA: Self = Self(0x2C);
    /// `F1`.
    pub const F1: Self = Self(0x3A);
    /// `F12`.
    pub const F12: Self = Self(0x45);
    /// Right arrow.
    pub const SAHM_YAMEEN: Self = Self(0x4F);
    /// Left arrow.
    pub const SAHM_YASAR: Self = Self(0x50);
    /// Down arrow.
    pub const SAHM_ASFAL: Self = Self(0x51);
    /// Up arrow.
    pub const SAHM_AALA: Self = Self(0x52);

    /// The largest usage this build will accept.
    ///
    /// The HID keyboard page ends well below this; the ceiling is here so a
    /// hand-edited or corrupted bindings file cannot introduce a chord that no
    /// hook can ever produce and that the user therefore cannot rebind away.
    pub const AQSA: u16 = 0x00FF;

    /// A key from a HID usage identifier, or [`RamzMiftah::LA_SHAY`] when it is
    /// outside the keyboard page this build handles.
    #[must_use]
    pub const fn min_hid(usage: u16) -> Self {
        if usage > Self::AQSA {
            Self::LA_SHAY
        } else {
            Self(usage)
        }
    }

    /// The raw usage identifier.
    #[must_use]
    pub const fn hid(self) -> u16 {
        self.0
    }

    /// Whether this is a real key.
    #[must_use]
    pub const fn salih(self) -> bool {
        self.0 != 0
    }

    /// The name a shortcut is written with.
    ///
    /// Falls back to the raw usage for a key this table does not name, which is
    /// honest and still rebindable — a player who bound something exotic sees
    /// `HID 0x64` rather than a blank, and knows which row to change.
    #[must_use]
    pub fn ism(self) -> String {
        match self.0 {
            0 => "—".to_owned(),
            usage @ 0x04..=0x1D => {
                let huruf = b"abcdefghijklmnopqrstuvwxyz";
                let mawqi = usize::from(usage - 0x04);
                huruf.get(mawqi).map_or_else(
                    || format!("HID {usage:#04x}"),
                    |harf| char::from(harf.to_ascii_uppercase()).to_string(),
                )
            },
            usage @ 0x1E..=0x26 => format!("{}", usage - 0x1D),
            0x27 => "0".to_owned(),
            0x28 => "Enter".to_owned(),
            0x29 => "Esc".to_owned(),
            0x2A => "Backspace".to_owned(),
            0x2B => "Tab".to_owned(),
            0x2C => "Space".to_owned(),
            0x2D => "-".to_owned(),
            0x2E => "=".to_owned(),
            0x2F => "[".to_owned(),
            0x30 => "]".to_owned(),
            0x31 => "\\".to_owned(),
            0x33 => ";".to_owned(),
            0x34 => "'".to_owned(),
            0x35 => "`".to_owned(),
            0x36 => ",".to_owned(),
            0x37 => ".".to_owned(),
            0x38 => "/".to_owned(),
            usage @ 0x3A..=0x45 => format!("F{}", usage - 0x39),
            0x46 => "PrintScreen".to_owned(),
            0x47 => "ScrollLock".to_owned(),
            0x48 => "Pause".to_owned(),
            0x49 => "Insert".to_owned(),
            0x4A => "Home".to_owned(),
            0x4B => "PageUp".to_owned(),
            0x4C => "Delete".to_owned(),
            0x4D => "End".to_owned(),
            0x4E => "PageDown".to_owned(),
            0x4F => "Right".to_owned(),
            0x50 => "Left".to_owned(),
            0x51 => "Down".to_owned(),
            0x52 => "Up".to_owned(),
            usage => format!("HID {usage:#04x}"),
        }
    }
}

/// A key with its modifiers: one shortcut.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Watar {
    /// Which modifiers are held.
    pub muaddilat: Muaddil,
    /// Which key is pressed.
    pub miftah: RamzMiftah,
}

impl Watar {
    /// A chord from its parts.
    #[must_use]
    pub const fn jadeed(muaddilat: Muaddil, miftah: RamzMiftah) -> Self {
        Self { muaddilat, miftah }
    }

    /// A chord as the hook saw it, with unknown modifier bits and out-of-range
    /// keys discarded.
    #[must_use]
    pub const fn min_khaam(bitat: u8, usage: u16) -> Self {
        Self {
            muaddilat: Muaddil::min_bitat(bitat),
            miftah: RamzMiftah::min_hid(usage),
        }
    }

    /// Whether this chord can be bound to anything.
    ///
    /// A chord with no key is modifiers alone, which every game holds down for
    /// seconds at a time.
    #[must_use]
    pub const fn salih(self) -> bool {
        self.miftah.salih()
    }

    /// The shortcut as it is written in the panel: `Ctrl+Shift+T`.
    #[must_use]
    pub fn ism(self) -> String {
        format!("{}{}", self.muaddilat.ism(), self.miftah.ism())
    }
}

/// The answer to a request to rebind a chord.
///
/// A value rather than an error, for the same reason [`crate::manatiq`] returns
/// one: a chord that is already taken is something the rebinding row says
/// inline, next to the field, naming the command that holds it. Silently
/// stealing the chord would leave the previous command unreachable with nothing
/// on screen explaining why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatijatRabt {
    /// The chord is now bound to the requested command.
    Rubita,

    /// The chord has no key, only modifiers, and cannot be bound.
    WatarKhali,

    /// Another command already holds it.
    Tanazu {
        /// Which command.
        sahib: Amr,
        /// The chord that is taken.
        watar: Watar,
    },
}

impl NatijatRabt {
    /// Whether the binding was made.
    #[must_use]
    pub const fn najahat(self) -> bool {
        matches!(self, Self::Rubita)
    }

    /// The sentence the rebinding row shows, in Arabic.
    #[must_use]
    pub fn unwan(self) -> String {
        match self {
            Self::Rubita => "تمّ الربط.".to_owned(),
            Self::WatarKhali => "لا يمكن ربط مفاتيح التعديل وحدها؛ اختر مفتاحًا معها.".to_owned(),
            Self::Tanazu { sahib, watar } => {
                format!(
                    "الاختصار ({}) مرتبط بالفعل بـ({}).",
                    watar.ism(),
                    sahib.unwan()
                )
            },
        }
    }

    /// The same sentence in English, for a log line.
    #[must_use]
    pub fn wasf(self) -> String {
        match self {
            Self::Rubita => "bound".to_owned(),
            Self::WatarKhali => "a chord needs a key, not modifiers alone".to_owned(),
            Self::Tanazu { sahib, watar } => {
                format!("{} is already bound to {}", watar.ism(), sahib.ism())
            },
        }
    }
}

/// The keyboard table: one chord per command, rebindable, conflict-checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ikhtisarat {
    rubut: BTreeMap<Amr, Watar>,
}

impl Default for Ikhtisarat {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

impl Ikhtisarat {
    /// The table this build ships with.
    #[must_use]
    pub fn iftiradiya() -> Self {
        let mut rubut = BTreeMap::new();
        for amr in Amr::jamee() {
            let _ = rubut.insert(amr, amr.iftiradi());
        }
        Self { rubut }
    }

    /// The chord bound to a command.
    ///
    /// A command with no row in the table falls back to its shipped default
    /// rather than to nothing. That matters for a bindings file written by an
    /// older build: a command added since would otherwise have no chord at all
    /// and no way for a player to discover that rebinding it is even possible.
    /// A row that is *present* and empty is respected — that is a player who
    /// deliberately unbound it.
    #[must_use]
    pub fn watar(&self, amr: Amr) -> Watar {
        self.rubut
            .get(&amr)
            .copied()
            .unwrap_or_else(|| amr.iftiradi())
    }

    /// Every binding, in command order.
    pub fn rubut(&self) -> impl Iterator<Item = (Amr, Watar)> {
        Amr::jamee().into_iter().map(|amr| (amr, self.watar(amr)))
    }

    /// Which command a chord fires, if any.
    ///
    /// The lookup the input hook performs on every key press, before it asks
    /// anything about focus — the chord that opens the panel has to work while
    /// the panel is closed.
    #[must_use]
    pub fn amr_li(&self, watar: Watar) -> Option<Amr> {
        if !watar.salih() {
            return None;
        }
        Amr::jamee()
            .into_iter()
            .find(|amr| self.watar(*amr) == watar)
    }

    /// Which command holds a chord, ignoring one command's own row.
    #[must_use]
    pub fn tanazu(&self, watar: Watar, nafsuhu: Amr) -> Option<Amr> {
        if !watar.salih() {
            return None;
        }
        Amr::jamee()
            .into_iter()
            .find(|amr| *amr != nafsuhu && self.watar(*amr) == watar)
    }

    /// Binds a chord to a command, refusing one another command already holds.
    ///
    /// Refusing rather than stealing. A rebinding UI that quietly unbinds the
    /// previous owner produces a player who has lost a shortcut and has no idea
    /// which action they broke — so the conflict comes back naming the command,
    /// and unbinding it first is a thing the player does on purpose.
    pub fn arbut(&mut self, amr: Amr, watar: Watar) -> NatijatRabt {
        if !watar.salih() {
            return NatijatRabt::WatarKhali;
        }
        if let Some(sahib) = self.tanazu(watar, amr) {
            return NatijatRabt::Tanazu { sahib, watar };
        }
        let _ = self.rubut.insert(amr, watar);
        NatijatRabt::Rubita
    }

    /// Removes a command's chord, leaving it reachable only from the panel.
    ///
    /// Stored as an empty row rather than as a missing one, so that
    /// [`Ikhtisarat::watar`] does not helpfully restore the default the player
    /// just removed.
    pub fn ifsil(&mut self, amr: Amr) {
        let _ = self.rubut.insert(amr, Watar::default());
    }

    /// Puts one command back to its shipped chord, whatever holds it now.
    ///
    /// Whatever holds it now: this is the escape hatch from a table a player has
    /// tangled, so it takes the chord back rather than reporting a conflict, and
    /// unbinds the command that had it.
    pub fn aid_amr(&mut self, amr: Amr) {
        let iftiradi = amr.iftiradi();
        if let Some(sahib) = self.tanazu(iftiradi, amr) {
            let _ = self.rubut.insert(sahib, Watar::default());
        }
        let _ = self.rubut.insert(amr, iftiradi);
    }

    /// Puts the whole table back to what this build ships.
    pub fn aid_taayeen(&mut self) {
        *self = Self::iftiradiya();
    }

    /// Where the bindings file lives under a directory.
    #[must_use]
    pub fn masar_malaf(mujallad: &Path) -> PathBuf {
        mujallad.join(ISM_MALAF_IKHTISARAT)
    }

    /// Writes the table atomically, stamped with the schema version.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the write cannot complete, and
    /// [`KhataTabaqa::MalafGhayrMafhum`] when the table does not serialize to a
    /// JSON object.
    pub fn ihfaz(&self, masar: &Path) -> Result<(), KhataTabaqa> {
        let bayt = mukhattat::iktub(self).map_err(|khata| KhataTabaqa::MalafGhayrMafhum {
            masar: masar.to_path_buf(),
            sigha: Self::ISM,
            sabab: khata.injilizi,
        })?;
        masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataTabaqa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab: std::io::Error::other(khata.injilizi),
        })
    }

    /// Reads the table, or ships the defaults when there is no file yet.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the file exists and cannot be read, and
    /// [`KhataTabaqa::MalafGhayrMafhum`] when it is not JSON, is not an object,
    /// carries no schema version, or carries one this build does not know —
    /// naming both numbers. Nothing migrates: a bindings file from a newer build
    /// holds commands this one has never heard of, and rewriting it after a
    /// partial read would delete their bindings.
    pub fn hammil(masar: &Path) -> Result<Self, KhataTabaqa> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::iftiradiya());
            },
            Err(sabab) => {
                return Err(KhataTabaqa::KhataMalaf {
                    masar: masar.to_path_buf(),
                    sabab,
                });
            },
        };
        let ghalat = |sabab: String| KhataTabaqa::MalafGhayrMafhum {
            masar: masar.to_path_buf(),
            sigha: Self::ISM,
            sabab,
        };
        let qeema: Value = serde_json::from_slice(&bayt)
            .map_err(|khata| ghalat(format!("it is not JSON: {khata}")))?;
        let Some(kain) = qeema.as_object() else {
            return Err(ghalat("its top level is not a JSON object".to_owned()));
        };
        let Some(mawjud) = kain.get(mukhattat::HAQL).and_then(Value::as_u64) else {
            return Err(ghalat(format!("it carries no `{}` field", mukhattat::HAQL)));
        };
        if mawjud != u64::from(Self::ISDAR) {
            return Err(ghalat(format!(
                "it declares schema version {mawjud}, and this build reads and writes \
                 version {}. It was written by a build that knew commands this one does \
                 not, and saving over it would unbind them.",
                Self::ISDAR
            )));
        }
        let mut ikhtisarat =
            mukhattat::min_qeema::<Self>(qeema).map_err(|khata| ghalat(khata.injilizi))?;
        ikhtisarat.tashih();
        Ok(ikhtisarat)
    }

    /// Drops rows this build cannot honour.
    ///
    /// A stored chord whose key is outside the range a hook can produce is
    /// replaced by an empty row rather than kept: keeping it would leave a
    /// command bound to something no keyboard emits, which reads to the player
    /// as a shortcut that stopped working. A duplicate chord — two commands with
    /// the same one, which the writer's own conflict check prevents but a
    /// hand-edited file does not — is left on the command that sorts first and
    /// cleared from the other, because [`Ikhtisarat::amr_li`] must have one
    /// answer.
    fn tashih(&mut self) {
        let mut mustaamala: BTreeMap<Watar, Amr> = BTreeMap::new();
        for amr in Amr::jamee() {
            let watar = self.watar(amr);
            let mahdud = Watar::min_khaam(watar.muaddilat.bitat(), watar.miftah.hid());
            let nihai = if !mahdud.salih() || mustaamala.contains_key(&mahdud) {
                Watar::default()
            } else {
                mahdud
            };
            if nihai.salih() {
                let _ = mustaamala.insert(nihai, amr);
            }
            let _ = self.rubut.insert(amr, nihai);
        }
    }
}

impl DhuMukhattat for Ikhtisarat {
    const ISM: &'static str = "ikhtisarat_tabaqa";

    /// One. There has never been another shape of this file.
    const ISDAR: u32 = 1;

    fn hijra(min: u32, _qeema: Value) -> Natija<Value> {
        Err(Khata::from(KhataTabaqa::MalafGhayrMafhum {
            masar: PathBuf::from(ISM_MALAF_IKHTISARAT),
            sigha: Self::ISM,
            sabab: format!(
                "the shortcut table claims schema {min}, and this build knows only {}",
                Self::ISDAR
            ),
        }))
    }
}

/// Which page of the panel is showing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum SafhatLawha {
    /// State, the main actions, and where everything else is.
    #[default]
    Raisiya,
    /// The region list and the editor's own instructions.
    Manatiq,
    /// The reading history.
    Sijill,
    /// The tier-3 disclosure, verbatim.
    Ifsah,
    /// The frame cost, verbatim.
    Meezaniya,
}

impl SafhatLawha {
    /// Every page, in tab order.
    #[must_use]
    pub const fn jamee() -> [Self; 5] {
        [
            Self::Raisiya,
            Self::Manatiq,
            Self::Sijill,
            Self::Meezaniya,
            Self::Ifsah,
        ]
    }

    /// The name used in a log line.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Raisiya => "main",
            Self::Manatiq => "regions",
            Self::Sijill => "history",
            Self::Ifsah => "disclosure",
            Self::Meezaniya => "budget",
        }
    }

    /// The tab label the panel shows, in Arabic.
    #[must_use]
    pub const fn unwan(self) -> &'static str {
        match self {
            Self::Raisiya => "الرئيسية",
            Self::Manatiq => "المناطق",
            Self::Sijill => "السجلّ",
            Self::Ifsah => "الشرح",
            Self::Meezaniya => "التكلفة",
        }
    }

    /// Whether this page scrolls, and therefore whether the scroll offset means
    /// anything on it.
    #[must_use]
    pub const fn tumarrar(self) -> bool {
        matches!(self, Self::Manatiq | Self::Sijill | Self::Ifsah)
    }
}

/// What the caller has to do outside the panel after a command ran.
///
/// The panel owns its own state — which page, how opaque, how large — and
/// applies those itself. It does not own the overlay, the capture scheduler or
/// the hooks, so the commands that touch those come back out as one of these
/// rather than being reached for through a back door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtharAmr {
    /// Nothing outside the panel needs to happen.
    LaShay,
    /// Turn the overlay's drawing on or off.
    BaddilTabaqa,
    /// Run one immediate pass over every enabled region.
    TarjimAlan,
    /// Stand the overlay down without unhooking.
    IqafMuaqqat,
    /// The panel closed and gave input back to the game.
    Ughliqat,
    /// The panel moved to another page.
    Intaqalat(SafhatLawha),
    /// The panel's opacity changed, as a percentage.
    TaghayyarAtaama(u8),
    /// The panel's font scale changed, as a percentage.
    TaghayyarKhat(u16),
}

/// The panel's own state: page, opacity, font scale, focus, scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HalatLawha {
    safha: SafhatLawha,
    ataama: u8,
    miqyas_khat: u16,
    murakkaza: bool,
    zahira: bool,
    tamreer: usize,
}

impl Default for HalatLawha {
    fn default() -> Self {
        Self::jadeeda()
    }
}

impl HalatLawha {
    /// The opacity the panel opens at, as a percentage.
    pub const ATAAMA_IFTIRADIYA: u8 = 88;

    /// The most transparent the panel may become.
    ///
    /// Twenty percent, and it is a floor rather than a preference. A panel faded
    /// to nothing is still focused and still swallowing every key the player
    /// types — they would be looking at their game, pressing keys, and getting
    /// nothing, with no visible cause anywhere on screen.
    pub const ADNA_ATAAMA: u8 = 20;

    /// Fully opaque.
    pub const AQSA_ATAAMA: u8 = 100;

    /// How much one opacity step moves.
    pub const KHUTWAT_ATAAMA: u8 = 5;

    /// The font scale the panel opens at, as a percentage of the base size.
    pub const MIQYAS_IFTIRADI: u16 = 100;

    /// The smallest font scale.
    pub const ADNA_MIQYAS: u16 = 60;

    /// The largest font scale.
    ///
    /// Two hundred and fifty percent. A ceiling exists because the panel is laid
    /// out in rows of a fixed height: past this, one row is taller than the
    /// panel and the page shows a header and nothing else.
    pub const AQSA_MIQYAS: u16 = 250;

    /// How much one font-size step moves.
    pub const KHUTWAT_MIQYAS: u16 = 10;

    /// A closed panel at the defaults.
    #[must_use]
    pub const fn jadeeda() -> Self {
        Self {
            safha: SafhatLawha::Raisiya,
            ataama: Self::ATAAMA_IFTIRADIYA,
            miqyas_khat: Self::MIQYAS_IFTIRADI,
            murakkaza: false,
            zahira: false,
            tamreer: 0,
        }
    }

    /// Which page is showing.
    #[must_use]
    pub const fn safha(self) -> SafhatLawha {
        self.safha
    }

    /// The opacity, zero to a hundred.
    #[must_use]
    pub const fn ataama(self) -> u8 {
        self.ataama
    }

    /// The font scale as a percentage.
    #[must_use]
    pub const fn miqyas_khat(self) -> u16 {
        self.miqyas_khat
    }

    /// The font scale as a multiplier.
    #[must_use]
    pub fn miqyas(self) -> f32 {
        f32::from(self.miqyas_khat) / 100.0
    }

    /// Whether the panel is drawn at all.
    #[must_use]
    pub const fn zahira(self) -> bool {
        self.zahira
    }

    /// Whether the panel currently owns the keyboard.
    #[must_use]
    pub const fn murakkaza(self) -> bool {
        self.murakkaza
    }

    /// How far the current page is scrolled, in rows.
    #[must_use]
    pub const fn tamreer(self) -> usize {
        self.tamreer
    }

    /// **Whether the hook must keep this input away from the game.**
    ///
    /// A keystroke has exactly one owner. While the panel is visible and
    /// focused, the player is typing *at it* — a region name, a search term, an
    /// interval — and every one of those characters is also a game binding. In a
    /// typical action game, typing `north gate` walks the character forward,
    /// opens the inventory, swings, and quicksaves over the file they were
    /// about to reload. So the hook consults this method on every input event
    /// and drops the event when it is true.
    ///
    /// The inverse matters just as much and is the reason this is not simply
    /// "is the panel open". A translation overlay that eats input while it is
    /// merely *visible* is an overlay that makes the game unplayable, and it is
    /// the single fastest way for a tool like this to be uninstalled. Visible
    /// and unfocused means the game gets everything.
    ///
    /// The chords in [`Ikhtisarat`] are outside this rule by design: the hook
    /// matches them *before* asking this question, because the chord that opens
    /// the panel has to work when the panel is neither visible nor focused.
    #[must_use]
    pub const fn yabtali_dakhl(self) -> bool {
        self.zahira && self.murakkaza
    }

    /// Shows the panel and gives it focus.
    pub const fn irfa(&mut self) {
        self.zahira = true;
        self.murakkaza = true;
    }

    /// Hides the panel and gives input back to the game.
    ///
    /// Focus is dropped with visibility, never separately. A hidden panel that
    /// kept focus would swallow every key with nothing on screen to explain it,
    /// which is the exact failure [`HalatLawha::yabtali_dakhl`] exists to
    /// prevent.
    pub const fn akhfi(&mut self) {
        self.zahira = false;
        self.murakkaza = false;
    }

    /// Toggles visibility.
    pub const fn baddil_zuhur(&mut self) {
        if self.zahira {
            self.akhfi();
        } else {
            self.irfa();
        }
    }

    /// Takes or releases the keyboard without hiding the panel.
    ///
    /// For a player who wants the panel on screen while they play: it keeps
    /// showing the frame cost and the last lines read, and every key goes to the
    /// game.
    pub const fn rakkiz(&mut self, murakkaza: bool) {
        self.murakkaza = murakkaza && self.zahira;
    }

    /// Moves to a page, resetting the scroll.
    ///
    /// Reset because a scroll offset is a position in *this* page's rows: eight
    /// rows down a forty-line history is a sensible place to be, and eight rows
    /// down a five-row region list is an empty screen.
    pub const fn ruh_ila(&mut self, safha: SafhatLawha) {
        self.safha = safha;
        self.tamreer = 0;
    }

    /// Scrolls the current page, clamped against a row count.
    ///
    /// The count is passed in because the panel does not own the lists it shows
    /// — the history and the region set both live elsewhere and both change
    /// between frames.
    pub fn marrir(&mut self, khutwa: i32, adad_sutur: usize) {
        if !self.safha.tumarrar() {
            self.tamreer = 0;
            return;
        }
        let hali = i64::try_from(self.tamreer).unwrap_or(i64::MAX);
        let jadeed = hali.saturating_add(i64::from(khutwa)).max(0);
        let saqf = i64::try_from(adad_sutur.saturating_sub(1)).unwrap_or(i64::MAX);
        self.tamreer = usize::try_from(jadeed.min(saqf.max(0))).unwrap_or(0);
    }

    /// Raises the opacity by one step.
    pub const fn zid_ataama(&mut self) -> u8 {
        let khaam = self.ataama.saturating_add(Self::KHUTWAT_ATAAMA);
        self.ataama = if khaam > Self::AQSA_ATAAMA {
            Self::AQSA_ATAAMA
        } else {
            khaam
        };
        self.ataama
    }

    /// Lowers the opacity by one step, never past the floor.
    pub const fn anqis_ataama(&mut self) -> u8 {
        let khaam = self.ataama.saturating_sub(Self::KHUTWAT_ATAAMA);
        self.ataama = if khaam < Self::ADNA_ATAAMA {
            Self::ADNA_ATAAMA
        } else {
            khaam
        };
        self.ataama
    }

    /// Sets the opacity directly, clamped.
    pub const fn ihdud_ataama(&mut self, ataama: u8) {
        self.ataama = if ataama < Self::ADNA_ATAAMA {
            Self::ADNA_ATAAMA
        } else if ataama > Self::AQSA_ATAAMA {
            Self::AQSA_ATAAMA
        } else {
            ataama
        };
    }

    /// Enlarges the panel's text by one step.
    pub const fn zid_khat(&mut self) -> u16 {
        let khaam = self.miqyas_khat.saturating_add(Self::KHUTWAT_MIQYAS);
        self.miqyas_khat = if khaam > Self::AQSA_MIQYAS {
            Self::AQSA_MIQYAS
        } else {
            khaam
        };
        self.miqyas_khat
    }

    /// Shrinks the panel's text by one step.
    pub const fn anqis_khat(&mut self) -> u16 {
        let khaam = self.miqyas_khat.saturating_sub(Self::KHUTWAT_MIQYAS);
        self.miqyas_khat = if khaam < Self::ADNA_MIQYAS {
            Self::ADNA_MIQYAS
        } else {
            khaam
        };
        self.miqyas_khat
    }

    /// Sets the font scale directly, clamped.
    pub const fn ihdud_khat(&mut self, miqyas: u16) {
        self.miqyas_khat = if miqyas < Self::ADNA_MIQYAS {
            Self::ADNA_MIQYAS
        } else if miqyas > Self::AQSA_MIQYAS {
            Self::AQSA_MIQYAS
        } else {
            miqyas
        };
    }

    /// Applies a command to the panel, and reports what the caller must do.
    ///
    /// Every command that only concerns the panel is finished here and returns
    /// [`AtharAmr::LaShay`] or a change notification. The three that concern the
    /// overlay come back out untouched, because this type has no business
    /// holding a [`crate::wajiha::Tabaqa`].
    pub const fn naffidh(&mut self, amr: Amr) -> AtharAmr {
        match amr {
            Amr::Tabdeel => AtharAmr::BaddilTabaqa,
            Amr::TarjimAlan => AtharAmr::TarjimAlan,
            Amr::Iqaf => AtharAmr::IqafMuaqqat,
            Amr::TahrirManatiq => {
                self.irfa();
                self.ruh_ila(SafhatLawha::Manatiq);
                AtharAmr::Intaqalat(SafhatLawha::Manatiq)
            },
            Amr::ArdSijill => {
                self.irfa();
                self.ruh_ila(SafhatLawha::Sijill);
                AtharAmr::Intaqalat(SafhatLawha::Sijill)
            },
            Amr::ArdIfsah => {
                self.irfa();
                self.ruh_ila(SafhatLawha::Ifsah);
                AtharAmr::Intaqalat(SafhatLawha::Ifsah)
            },
            Amr::ZidAtaama => AtharAmr::TaghayyarAtaama(self.zid_ataama()),
            Amr::AnqisAtaama => AtharAmr::TaghayyarAtaama(self.anqis_ataama()),
            Amr::ZidHajmKhat => AtharAmr::TaghayyarKhat(self.zid_khat()),
            Amr::AnqisHajmKhat => AtharAmr::TaghayyarKhat(self.anqis_khat()),
            Amr::Ighlaq => {
                self.akhfi();
                AtharAmr::Ughliqat
            },
        }
    }
}

/// A surface dimension as a float, with the precision question answered once.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// One sRGB channel as a linear value.
fn khatti(qanat: u8) -> f32 {
    let nisbi = f32::from(qanat) / 255.0;
    if nisbi <= 0.040_45 {
        nisbi / 12.92
    } else {
        ((nisbi + 0.055) / 1.055).powf(2.4)
    }
}

/// A theme colour written as sRGB, delivered as premultiplied linear RGBA.
///
/// Both halves of that sentence are load-bearing and both are the thing overlays
/// get wrong. Colours are *authored* in sRGB because that is what a person picks
/// in any tool; they are *delivered* linear because the blend happens in linear
/// space, and premultiplied because [`crate::wajiha::QitaRasm::lawn`] says so.
/// Handing a straight-alpha sRGB triple to a premultiplied blend produces a
/// panel that is visibly too bright over dark scenery and too dark over light
/// scenery — which reads as "the overlay looks wrong" while every individual
/// step looks right. See [`crate::wajiha::WasfSath::sirgb`].
#[must_use]
pub fn lawn_min_srgb(ahmar: u8, akhdar: u8, azraq: u8, shafafiya: f32) -> [f32; 4] {
    let alfa = if shafafiya.is_finite() {
        shafafiya.clamp(0.0, 1.0)
    } else {
        1.0
    };
    [
        khatti(ahmar) * alfa,
        khatti(akhdar) * alfa,
        khatti(azraq) * alfa,
        alfa,
    ]
}

/// A premultiplied colour faded by an opacity percentage.
///
/// All four components are scaled, not just alpha. That is what premultiplied
/// means: the colour channels already carry the alpha, so scaling alpha alone
/// would produce a colour brighter than its own coverage — the classic halo
/// around faded text. Written here once so the panel has exactly one fade.
#[must_use]
pub fn bi_ataama(lawn: [f32; 4], ataama: u8) -> [f32; 4] {
    let nisba = f32::from(ataama.min(100)) / 100.0;
    let [ahmar, akhdar, azraq, alfa] = lawn;
    [ahmar * nisba, akhdar * nisba, azraq * nisba, alfa * nisba]
}

/// What a panel element is for, and therefore what colour it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DawrAnsur {
    /// The plate everything else sits on.
    Khalfiya,
    /// The title bar.
    Tarwisa,
    /// A field name.
    Wasm,
    /// A field's value, or a line of quoted text.
    Qeema,
    /// Something that can be pressed.
    Zir,
    /// A horizontal rule between groups.
    Fasil,
    /// The selected tab, the current row, the thing being pointed at.
    Ibraz,
}

impl DawrAnsur {
    /// The name used in a log line and a layout dump.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Khalfiya => "panel",
            Self::Tarwisa => "header",
            Self::Wasm => "label",
            Self::Qeema => "value",
            Self::Zir => "button",
            Self::Fasil => "separator",
            Self::Ibraz => "highlight",
        }
    }

    /// The colour the theme assigns this role, premultiplied and linear.
    ///
    /// One dark theme, deliberately. A panel drawn over an arbitrary game has no
    /// background to harmonise with and no system accent colour to inherit, and
    /// a light theme over a dark game is a rectangle that hurts to look at in
    /// exactly the situation this panel is opened in.
    #[must_use]
    pub fn lawn(self) -> [f32; 4] {
        match self {
            Self::Khalfiya => lawn_min_srgb(0x10, 0x14, 0x1C, 0.92),
            Self::Tarwisa => lawn_min_srgb(0x1B, 0x23, 0x30, 0.98),
            Self::Wasm => lawn_min_srgb(0x9A, 0xA7, 0xBD, 1.0),
            Self::Qeema => lawn_min_srgb(0xE6, 0xED, 0xF7, 1.0),
            Self::Zir => lawn_min_srgb(0x26, 0x32, 0x4A, 0.95),
            Self::Fasil => lawn_min_srgb(0x2F, 0x3B, 0x52, 0.90),
            Self::Ibraz => lawn_min_srgb(0x3D, 0x7D, 0xD8, 0.95),
        }
    }

    /// Whether this role's element normally carries a string.
    #[must_use]
    pub const fn yahmil_nassan(self) -> bool {
        matches!(
            self,
            Self::Tarwisa | Self::Wasm | Self::Qeema | Self::Zir | Self::Ibraz
        )
    }
}

/// What pressing an element means.
///
/// One field rather than three optional ones, so a click handler is a single
/// exhaustive match and an element cannot claim to be two things at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HadafAnsur {
    /// Decoration. Pressing it does nothing.
    LaShay,
    /// Runs a command.
    Amr(Amr),
    /// Moves to a page.
    Safha(SafhatLawha),
    /// Selects a region on the regions page.
    Mintaqa(MuarrifMintaqa),
    /// Selects a line on the history page.
    Madkhal(MuarrifMadkhal),
}

impl HadafAnsur {
    /// Whether this element responds to a press.
    #[must_use]
    pub const fn qabil_lil_naqr(self) -> bool {
        !matches!(self, Self::LaShay)
    }
}

/// One rectangle of the panel, in normalized coordinates.
///
/// Not a [`crate::wajiha::QitaRasm`], and the difference is the point: the text
/// here is a `String`, because it still has to go through `saff` for shaping and
/// come out of Taarib's atlas. There is no second text path in this product.
#[derive(Debug, Clone, PartialEq)]
pub struct AnsurLawha {
    /// Where it goes, normalized against the surface it was built for.
    pub mustatil: MustatilNisbi,

    /// What it is.
    pub dawr: DawrAnsur,

    /// The string to shape into it, when there is one.
    ///
    /// Unwrapped and unmeasured. Measuring Arabic means shaping it, which
    /// happens one layer up, so a line that does not fit is broken there — not
    /// guessed at here with a character count.
    pub nass: Option<String>,

    /// What pressing it does.
    pub hadaf: HadafAnsur,

    /// The colour, premultiplied linear RGBA, already faded by the panel's
    /// opacity.
    pub lawn: [f32; 4],
}

impl AnsurLawha {
    /// An element with no text and nothing to press.
    #[must_use]
    pub fn zukhrufi(mustatil: MustatilNisbi, dawr: DawrAnsur, ataama: u8) -> Self {
        Self {
            mustatil,
            dawr,
            nass: None,
            hadaf: HadafAnsur::LaShay,
            lawn: bi_ataama(dawr.lawn(), ataama),
        }
    }

    /// An element carrying a string.
    #[must_use]
    pub fn bi_nass(
        mustatil: MustatilNisbi,
        dawr: DawrAnsur,
        nass: impl Into<String>,
        ataama: u8,
    ) -> Self {
        Self {
            mustatil,
            dawr,
            nass: Some(nass.into()),
            hadaf: HadafAnsur::LaShay,
            lawn: bi_ataama(dawr.lawn(), ataama),
        }
    }

    /// The same element, made pressable.
    #[must_use]
    pub const fn ila(mut self, hadaf: HadafAnsur) -> Self {
        self.hadaf = hadaf;
        self
    }
}

/// One region as the panel shows it.
///
/// A copy, taken once per refresh. The panel is laid out inside a frame and the
/// region set is edited from another thread; holding a borrow of it across the
/// layout would hand the render thread's cost to whoever is editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrMintaqa {
    /// Which region.
    pub muarrif: MuarrifMintaqa,
    /// Its name at the moment of the refresh.
    pub ism: String,
    /// When it is read.
    pub qaida: QaidatTarjama,
    /// Whether it is on.
    pub mumakkana: bool,
}

impl SatrMintaqa {
    /// The row's text.
    #[must_use]
    pub fn nass(&self) -> String {
        format!(
            "{} — {} — {}",
            self.ism,
            self.qaida.unwan(),
            if self.mumakkana {
                "مفعّلة"
            } else {
                "موقوفة"
            }
        )
    }
}

/// A cursor that hands out rows down a column and stops when it runs out.
///
/// Stopping rather than overflowing is the whole reason it exists. The panel's
/// height is fixed by the surface, the row height is chosen by the player
/// through the font scale, and a history page at 250 percent has room for four
/// lines — so the layout has to be able to say "no more", once, in one place,
/// rather than at every emission site.
#[derive(Debug, Clone, Copy)]
struct Takhtit {
    yasar: f32,
    ard: f32,
    hali: f32,
    asfal: f32,
    irtifa_saf: f32,
    fajwa: f32,
}

impl Takhtit {
    /// The next row, `irtifaat` rows tall, or [`None`] when the column is full.
    fn saf(&mut self, irtifaat: f32) -> Option<MustatilNisbi> {
        let irtifa = self.irtifa_saf * irtifaat;
        if self.hali + irtifa > self.asfal {
            return None;
        }
        let mustatil = MustatilNisbi {
            yasar: self.yasar,
            aala: self.hali,
            ard: self.ard,
            irtifa,
        };
        self.hali += irtifa + self.fajwa;
        Some(mustatil)
    }

    /// A row pinned to the bottom of the column, whatever is above it.
    ///
    /// For the footer alone. Going through [`Takhtit::saf`] would make the close
    /// hint the first thing a full page pushed off the panel, and a panel that
    /// does not say how to close it is a panel a player force-quits the game to
    /// get rid of.
    fn saf_asfal(&self, irtifaat: f32) -> MustatilNisbi {
        let irtifa = self.irtifa_saf * irtifaat;
        MustatilNisbi {
            yasar: self.yasar,
            aala: (self.asfal - irtifa).max(self.hali),
            ard: self.ard,
            irtifa,
        }
    }
}

/// The overlay's state, in Arabic, for the panel.
///
/// [`HalatTabaqa::ism`] answers the same question in English because it is read
/// out of a log by a contributor. This one is read off a screen by a player mid
/// game, and the two audiences do not overlap.
const fn unwan_hala(hala: HalatTabaqa) -> &'static str {
    match hala {
        HalatTabaqa::Amila => "تعمل",
        HalatTabaqa::Mutawaqqifa => "موقوفة مؤقّتًا",
        HalatTabaqa::Muattala => "مُعطَّلة بعد خلل",
    }
}

/// The in-game control panel.
///
/// Holds its own state and *snapshots* of everything else it shows. It never
/// borrows the region set, the history or the overlay while it lays itself out,
/// because laying out happens inside a frame and holding a lock across it hands
/// the render thread's cost to whichever thread wanted the data next.
#[derive(Debug, Clone)]
pub struct LawhatTahakkum {
    hala: HalatLawha,
    ikhtisarat: Ikhtisarat,
    meezaniya: MeezaniyatItar,
    hala_tabaqa: HalatTabaqa,
    wajiha: Option<WajihatRusum>,
    ism_luba: String,
    manatiq: Vec<SatrMintaqa>,
    sijill: Vec<MadkhalQira>,
    taqreer: Option<TaqreerSijill>,
    /// The recognition worker's last word about itself; [`None`] until the hook
    /// has polled it once, which is a different fact from "healthy".
    khayt: Option<HalatKhayt>,
}

impl LawhatTahakkum {
    /// The gap between the panel and the edge of the screen, in pixels.
    pub const HASHIYA_BIKSEL: f32 = 24.0;

    /// The gap between the panel's edge and its contents, in pixels.
    pub const HASHIYA_DAKHILIYA_BIKSEL: f32 = 12.0;

    /// One row's height at a font scale of one hundred percent, in pixels.
    pub const IRTIFA_SAF_BIKSEL: f32 = 26.0;

    /// The gap between rows, in pixels.
    pub const FAJWA_BIKSEL: f32 = 4.0;

    /// The panel's width as a fraction of the surface's **height**.
    ///
    /// Of the height, not the width, and that is the reason
    /// [`LawhatTahakkum::bina`] is given a [`WasfSath`] at all. A panel that is
    /// a fixed fraction of the width is a sensible size at 16:9, a slab at 32:9
    /// and a sliver on a portrait surface. Sizing against the height and
    /// converting through the aspect ratio gives the same panel on every monitor
    /// somebody actually owns.
    pub const NISBAT_ARD: f32 = 0.42;

    /// The narrowest the panel may be, normalized.
    pub const ADNA_ARD: f32 = 0.20;

    /// The widest the panel may be, normalized.
    pub const AQSA_ARD: f32 = 0.60;

    /// How many tabs the page strip holds.
    const ADAD_SAFAHAT: f32 = 5.0;

    /// A closed panel for a game.
    #[must_use]
    pub fn jadeeda(ism_luba: impl Into<String>) -> Self {
        Self {
            hala: HalatLawha::jadeeda(),
            ikhtisarat: Ikhtisarat::iftiradiya(),
            meezaniya: MeezaniyatItar::jadeeda(),
            hala_tabaqa: HalatTabaqa::Mutawaqqifa,
            wajiha: None,
            ism_luba: ism_luba.into(),
            manatiq: Vec::new(),
            sijill: Vec::new(),
            taqreer: None,
            khayt: None,
        }
    }

    /// The panel's own state.
    #[must_use]
    pub const fn hala(&self) -> &HalatLawha {
        &self.hala
    }

    /// The panel's own state, to change it.
    pub const fn hala_mut(&mut self) -> &mut HalatLawha {
        &mut self.hala
    }

    /// The keyboard table.
    #[must_use]
    pub const fn ikhtisarat(&self) -> &Ikhtisarat {
        &self.ikhtisarat
    }

    /// The keyboard table, to rebind it.
    pub const fn ikhtisarat_mut(&mut self) -> &mut Ikhtisarat {
        &mut self.ikhtisarat
    }

    /// Replaces the keyboard table, after it was read from disk.
    pub fn ihmil_ikhtisarat(&mut self, ikhtisarat: Ikhtisarat) {
        self.ikhtisarat = ikhtisarat;
    }

    /// Refreshes the frame cost the budget page quotes.
    ///
    /// Called every frame by the hook. Copied rather than borrowed for the
    /// reason this whole type copies: the budget belongs to
    /// [`crate::wajiha::Tabaqa`], and the panel holding a reference into it
    /// would mean the panel and the overlay could not be touched by two threads.
    pub const fn hadith_meezaniya(&mut self, meezaniya: MeezaniyatItar) {
        self.meezaniya = meezaniya;
    }

    /// Refreshes the overlay's state and which graphics API is under it.
    pub const fn hadith_tabaqa(&mut self, hala: HalatTabaqa, wajiha: Option<WajihatRusum>) {
        self.hala_tabaqa = hala;
        self.wajiha = wajiha;
    }

    /// Refreshes what the recognition worker last said about itself.
    ///
    /// Read off [`crate::qissa::KhaytQissa::hala`] by the hook and copied here
    /// for the reason this whole type copies. This is the line that turns a
    /// blank overlay into a sentence: a recognizer that is gone for the session
    /// says so in the header, on the main page with its reason, and on the
    /// budget page in full — rather than as a count of refusals climbing four
    /// times a second.
    pub fn hadith_khayt(&mut self, hala: HalatKhayt) {
        self.khayt = Some(hala);
    }

    /// What the recognition worker last said about itself, if it has been asked.
    #[must_use]
    pub const fn khayt(&self) -> Option<&HalatKhayt> {
        self.khayt.as_ref()
    }

    /// Refreshes the region list from the live set.
    pub fn hadith_manatiq(&mut self, majmua: &MajmuatManatiq) {
        self.manatiq = majmua
            .manatiq()
            .iter()
            .map(|mintaqa| SatrMintaqa {
                muarrif: mintaqa.muarrif,
                ism: mintaqa.ism.clone(),
                qaida: mintaqa.qaida,
                mumakkana: mintaqa.mumakkana,
            })
            .collect();
        majmua.ism_luba().clone_into(&mut self.ism_luba);
    }

    /// Refreshes the history rows and the history summary.
    pub fn hadith_sijill(&mut self, madakhil: Vec<MadkhalQira>, taqreer: TaqreerSijill) {
        self.sijill = madakhil;
        self.taqreer = Some(taqreer);
    }

    /// Which command a chord fires.
    ///
    /// Consulted by the hook *before* [`HalatLawha::yabtali_dakhl`], because the
    /// chord that opens the panel has to work while the panel is closed.
    #[must_use]
    pub fn amr_li(&self, watar: Watar) -> Option<Amr> {
        self.ikhtisarat.amr_li(watar)
    }

    /// Runs a command against the panel and reports what is left for the caller.
    pub const fn naffidh(&mut self, amr: Amr) -> AtharAmr {
        self.hala.naffidh(amr)
    }

    /// How many rows the current page has, for clamping the scroll.
    #[must_use]
    pub fn sutur_safha(&self) -> usize {
        match self.hala.safha() {
            SafhatLawha::Raisiya | SafhatLawha::Meezaniya => 0,
            SafhatLawha::Manatiq => self.manatiq.len(),
            SafhatLawha::Sijill => self.sijill.len(),
            SafhatLawha::Ifsah => NASS_IFSAH_ARABI.lines().count(),
        }
    }

    /// Scrolls the current page by a number of rows.
    pub fn marrir(&mut self, khutwa: i32) {
        let adad = self.sutur_safha();
        self.hala.marrir(khutwa, adad);
    }

    /// **Whether the disclosure has to be shown before the overlay is enabled.**
    ///
    /// Delegates the judgement to [`crate::sidq::mahfuz_salih`] rather than
    /// restating it, so there is exactly one definition of "this acknowledgement
    /// is still an acknowledgement of this text". [`None`] means nothing was
    /// ever recorded, and a recorded fingerprint that is not this build's means
    /// the disclosure changed since the user agreed to it — in both cases the
    /// answer is yes, show it.
    #[must_use]
    pub fn yajib_ard_ifsah(mahfuz: Option<BasmatIfsah>) -> bool {
        !mahfuz.is_some_and(sidq::mahfuz_salih)
    }

    /// Lays the panel out for a surface, in normalized coordinates.
    ///
    /// Returns an empty layout when the panel is hidden or the surface has no
    /// area — the latter happens for the frame or two around a minimize, and
    /// laying out against a zero dimension would divide by it.
    ///
    /// What comes back is rectangles, roles, colours and **strings**. It is not
    /// [`crate::wajiha::QitaRasm`], and it never will be: the text has to go
    /// through `saff` for shaping first, and that happens one layer up. There is
    /// no second text path in this product — the panel's own labels come out of
    /// the same shaper and the same atlas as the translated dialogue, so there
    /// is no corner of the product where Arabic can be laid out wrongly with
    /// nothing else to catch it.
    #[must_use]
    pub fn bina(&self, sath: WasfSath) -> Vec<AnsurLawha> {
        let mut ansur = Vec::new();
        if !self.hala.zahira() || sath.ard == 0 || sath.irtifa == 0 {
            return ansur;
        }

        let ard_px = madaa_f32(sath.ard);
        let irtifa_px = madaa_f32(sath.irtifa);
        let ataama = self.hala.ataama();

        let hashiya_u = (Self::HASHIYA_BIKSEL / ard_px).min(0.2);
        let hashiya_r = (Self::HASHIYA_BIKSEL / irtifa_px).min(0.2);
        let dakhil_u = (Self::HASHIYA_DAKHILIYA_BIKSEL / ard_px).min(0.1);
        let dakhil_r = (Self::HASHIYA_DAKHILIYA_BIKSEL / irtifa_px).min(0.1);
        let ard_lawha = ((Self::NISBAT_ARD * irtifa_px) / ard_px)
            .clamp(Self::ADNA_ARD, Self::AQSA_ARD)
            .min(2.0f32.mul_add(-hashiya_u, 1.0));
        let irtifa_lawha = 2.0f32.mul_add(-hashiya_r, 1.0);
        if ard_lawha <= 0.0 || irtifa_lawha <= 0.0 {
            return ansur;
        }

        // Anchored to the right edge. The product is Arabic-first and its
        // reading order starts there, so the panel starts there too.
        let yasar = (1.0 - ard_lawha - hashiya_u).max(0.0);
        let lawha = MustatilNisbi {
            yasar,
            aala: hashiya_r,
            ard: ard_lawha,
            irtifa: irtifa_lawha,
        };
        ansur.push(AnsurLawha::zukhrufi(lawha, DawrAnsur::Khalfiya, ataama));

        let irtifa_saf =
            ((Self::IRTIFA_SAF_BIKSEL * self.hala.miqyas()) / irtifa_px).clamp(0.001, irtifa_lawha);
        let fajwa = (Self::FAJWA_BIKSEL / irtifa_px).min(irtifa_saf);
        let mut takhtit = Takhtit {
            yasar: yasar + dakhil_u,
            ard: 2.0f32.mul_add(-dakhil_u, ard_lawha).max(0.0),
            hali: hashiya_r + dakhil_r,
            asfal: hashiya_r + irtifa_lawha - dakhil_r,
            irtifa_saf,
            fajwa,
        };

        if let Some(saf) = takhtit.saf(1.4) {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Tarwisa,
                self.unwan_tarwisa(),
                ataama,
            ));
        }
        if let Some(saf) = takhtit.saf(1.0) {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Qeema,
                self.satr_hala(),
                ataama,
            ));
        }
        self.bina_alsina(&mut takhtit, &mut ansur, ataama);
        if let Some(saf) = takhtit.saf(0.25) {
            ansur.push(AnsurLawha::zukhrufi(saf, DawrAnsur::Fasil, ataama));
        }

        // The footer is reserved before the body is laid out, not appended
        // after it. Appending would let a long page consume the last row and
        // leave the panel with no visible way to close it.
        let dhayl_irtifa = irtifa_saf + fajwa;
        takhtit.asfal -= dhayl_irtifa;

        match self.hala.safha() {
            SafhatLawha::Raisiya => self.bina_raisiya(&mut takhtit, &mut ansur, ataama),
            SafhatLawha::Manatiq => self.bina_manatiq(&mut takhtit, &mut ansur, ataama),
            SafhatLawha::Sijill => self.bina_sijill(&mut takhtit, &mut ansur, ataama),
            SafhatLawha::Meezaniya => self.bina_meezaniya(&mut takhtit, &mut ansur, ataama),
            SafhatLawha::Ifsah => self.bina_ifsah(&mut takhtit, &mut ansur, ataama),
        }

        takhtit.asfal += dhayl_irtifa;
        let dhayl = takhtit.saf_asfal(1.0);
        ansur.push(
            AnsurLawha::bi_nass(dhayl, DawrAnsur::Wasm, self.satr_dhayl(), ataama)
                .ila(HadafAnsur::Amr(Amr::Ighlaq)),
        );
        ansur
    }

    /// The title bar's text.
    fn unwan_tarwisa(&self) -> String {
        if self.ism_luba.trim().is_empty() {
            "تعريب — لوحة التحكّم".to_owned()
        } else {
            format!("تعريب — {}", self.ism_luba)
        }
    }

    /// The state line under the title.
    fn satr_hala(&self) -> String {
        let wajiha = self.wajiha.map_or("—", WajihatRusum::ism);
        // A stopped recognizer is on every page's status line, because the
        // player who opens the panel over a blank overlay opens it on
        // whichever page it was left on.
        let qiraa = if self.khayt.as_ref().is_some_and(HalatKhayt::mutawaqqifa) {
            " · القراءة متوقّفة"
        } else {
            ""
        };
        format!(
            "الطبقة: {} · الواجهة: {wajiha}{qiraa}",
            unwan_hala(self.hala_tabaqa)
        )
    }

    /// The short word for the worker's state, for the main page's row.
    const fn unwan_khayt(&self) -> &'static str {
        match self.khayt {
            None => "—",
            Some(HalatKhayt::Salima) => "سليمة",
            Some(HalatKhayt::Aabira { .. }) => "رفض عابر",
            Some(HalatKhayt::Mutawaqqifa { .. }) => "متوقّفة",
        }
    }

    /// The footer's text: how to close the panel.
    fn satr_dhayl(&self) -> String {
        format!(
            "{} — {}",
            Amr::Ighlaq.unwan(),
            self.ikhtisarat.watar(Amr::Ighlaq).ism()
        )
    }

    /// The tab strip, laid out right to left.
    fn bina_alsina(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        let Some(saf) = takhtit.saf(1.0) else {
            return;
        };
        let ard_lisan = saf.ard / Self::ADAD_SAFAHAT;
        let mut yasar = saf.yasar + saf.ard - ard_lisan;
        for safha in SafhatLawha::jamee() {
            let dawr = if safha == self.hala.safha() {
                DawrAnsur::Ibraz
            } else {
                DawrAnsur::Zir
            };
            let mustatil = MustatilNisbi {
                yasar: yasar.max(saf.yasar),
                aala: saf.aala,
                ard: ard_lisan,
                irtifa: saf.irtifa,
            };
            ansur.push(
                AnsurLawha::bi_nass(mustatil, dawr, safha.unwan(), ataama)
                    .ila(HadafAnsur::Safha(safha)),
            );
            yasar -= ard_lisan;
        }
    }

    /// A row split into a label and a value, label on the right.
    ///
    /// Right first because the panel is Arabic and Arabic starts there. The
    /// split is by fraction rather than by measured text, because measuring is
    /// shaping and shaping happens one layer up.
    fn saf_thunai(
        takhtit: &mut Takhtit,
        ansur: &mut Vec<AnsurLawha>,
        wasm: &str,
        qeema: &str,
        ataama: u8,
    ) {
        let Some(saf) = takhtit.saf(1.0) else {
            return;
        };
        let ard_wasm = saf.ard * 0.42;
        ansur.push(AnsurLawha::bi_nass(
            MustatilNisbi {
                yasar: saf.yasar + saf.ard - ard_wasm,
                aala: saf.aala,
                ard: ard_wasm,
                irtifa: saf.irtifa,
            },
            DawrAnsur::Wasm,
            wasm,
            ataama,
        ));
        ansur.push(AnsurLawha::bi_nass(
            MustatilNisbi {
                yasar: saf.yasar,
                aala: saf.aala,
                ard: (saf.ard - ard_wasm).max(0.0),
                irtifa: saf.irtifa,
            },
            DawrAnsur::Qeema,
            qeema,
            ataama,
        ));
    }

    /// The main page: what the overlay is doing, and the things to do to it.
    fn bina_raisiya(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        let mumakkana = self.manatiq.iter().filter(|satr| satr.mumakkana).count();
        Self::saf_thunai(
            takhtit,
            ansur,
            "المناطق",
            &format!("{mumakkana} مفعّلة من {}", self.manatiq.len()),
            ataama,
        );
        Self::saf_thunai(
            takhtit,
            ansur,
            "السجلّ",
            &self
                .taqreer
                .map_or_else(|| "—".to_owned(), |taqreer| taqreer.unwan()),
            ataama,
        );
        // The short form only. The honest, unabridged sentence lives on the
        // budget page, where it is quoted rather than summarised.
        Self::saf_thunai(
            takhtit,
            ansur,
            "تكلفة الإطار",
            &format!(
                "{} ميكروثانية، والسقف {}",
                self.meezaniya.akhir_mikro, self.meezaniya.saqf_mikro
            ),
            ataama,
        );
        Self::saf_thunai(
            takhtit,
            ansur,
            "العتامة",
            &format!("{}٪", self.hala.ataama()),
            ataama,
        );
        Self::saf_thunai(
            takhtit,
            ansur,
            "حجم الخط",
            &format!("{}٪", self.hala.miqyas_khat()),
            ataama,
        );
        Self::saf_thunai(takhtit, ansur, "القراءة", self.unwan_khayt(), ataama);
        // The reason goes here, on the page the panel opens to, and not only on
        // the budget page: a session that has stopped reading is the one thing
        // a player needs to know before they press anything below.
        if let Some(khayt @ HalatKhayt::Mutawaqqifa { .. }) = self.khayt.as_ref()
            && let Some(saf) = takhtit.saf(2.0)
        {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Qeema,
                khayt.wasf_arabi(),
                ataama,
            ));
        }

        if let Some(saf) = takhtit.saf(0.25) {
            ansur.push(AnsurLawha::zukhrufi(saf, DawrAnsur::Fasil, ataama));
        }

        let awamir = [
            Amr::Tabdeel,
            Amr::TarjimAlan,
            Amr::TahrirManatiq,
            Amr::ArdSijill,
            Amr::Iqaf,
            Amr::ArdIfsah,
        ];
        for zawj in awamir.chunks(2) {
            let Some(saf) = takhtit.saf(1.0) else {
                break;
            };
            let nisf = saf.ard * 0.5;
            if let Some(amr) = zawj.first() {
                ansur.push(self.zir(
                    MustatilNisbi {
                        yasar: saf.yasar + nisf,
                        aala: saf.aala,
                        ard: nisf,
                        irtifa: saf.irtifa,
                    },
                    *amr,
                    ataama,
                ));
            }
            if let Some(amr) = zawj.get(1) {
                ansur.push(self.zir(
                    MustatilNisbi {
                        yasar: saf.yasar,
                        aala: saf.aala,
                        ard: nisf,
                        irtifa: saf.irtifa,
                    },
                    *amr,
                    ataama,
                ));
            }
        }
    }

    /// One command button, labelled with its chord.
    fn zir(&self, mustatil: MustatilNisbi, amr: Amr, ataama: u8) -> AnsurLawha {
        let watar = self.ikhtisarat.watar(amr);
        let nass = if watar.salih() {
            format!("{}  ({})", amr.unwan(), watar.ism())
        } else {
            amr.unwan().to_owned()
        };
        AnsurLawha::bi_nass(mustatil, DawrAnsur::Zir, nass, ataama).ila(HadafAnsur::Amr(amr))
    }

    /// The regions page: what is defined, and how to define more.
    fn bina_manatiq(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        if self.manatiq.is_empty() {
            for satr in [
                "لا توجد مناطق بعد.",
                "اسحب بالفأرة فوق الصندوق الذي يظهر فيه النص لرسم منطقة.",
                "اسحب المنطقة لتحريكها، واسحب أحد أركانها لتغيير حجمها.",
                "المنطقة تُحفظ بنسبة من الشاشة، فلا تتأثّر بتغيير الدقّة.",
            ] {
                if let Some(saf) = takhtit.saf(1.0) {
                    ansur.push(AnsurLawha::bi_nass(saf, DawrAnsur::Wasm, satr, ataama));
                }
            }
            return;
        }

        for satr in self.manatiq.iter().skip(self.hala.tamreer()) {
            let Some(mustatil) = takhtit.saf(1.0) else {
                break;
            };
            let dawr = if satr.mumakkana {
                DawrAnsur::Qeema
            } else {
                DawrAnsur::Wasm
            };
            ansur.push(
                AnsurLawha::bi_nass(mustatil, dawr, satr.nass(), ataama)
                    .ila(HadafAnsur::Mintaqa(satr.muarrif)),
            );
        }
    }

    /// The history page: what was read, oldest of the visible window first.
    fn bina_sijill(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        if self.sijill.is_empty() {
            for satr in [
                "لم يُقرأ نصّ بعد في هذه الجلسة.",
                "كلّ سطر يُقرأ ويُترجم يُحفظ هنا حتى تُغلق اللعبة.",
            ] {
                if let Some(saf) = takhtit.saf(1.0) {
                    ansur.push(AnsurLawha::bi_nass(saf, DawrAnsur::Wasm, satr, ataama));
                }
            }
            return;
        }

        for madkhal in self.sijill.iter().skip(self.hala.tamreer()) {
            let Some(mustatil) = takhtit.saf(1.0) else {
                break;
            };
            // A line whose translation has not arrived is shown in the label
            // colour rather than hidden. A history that only listed finished
            // translations would be missing exactly the lines a player opened it
            // to ask about.
            let dawr = if madkhal.tarjuma_wasalat() {
                DawrAnsur::Qeema
            } else {
                DawrAnsur::Wasm
            };
            ansur.push(
                AnsurLawha::bi_nass(mustatil, dawr, madkhal.satr(), ataama)
                    .ila(HadafAnsur::Madkhal(madkhal.muarrif)),
            );
        }
    }

    /// The budget page, which quotes the frame cost and does not summarise it.
    fn bina_meezaniya(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        if let Some(saf) = takhtit.saf(1.0) {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Wasm,
                "التكلفة الحقيقية كما تُقاس، بلا تلطيف:",
                ataama,
            ));
        }
        // Verbatim. `MeezaniyatItar::wasf` is the product's own account of what
        // the overlay costs, in microseconds and as a share of a frame, and it
        // is reproduced here exactly — not rounded, not abbreviated, and not
        // reworded into something that sounds better.
        if let Some(saf) = takhtit.saf(2.0) {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Qeema,
                self.meezaniya.wasf(),
                ataama,
            ));
        }
        if let Some(saf) = takhtit.saf(0.25) {
            ansur.push(AnsurLawha::zukhrufi(saf, DawrAnsur::Fasil, ataama));
        }
        if let Some(saf) = takhtit.saf(1.0) {
            ansur.push(AnsurLawha::bi_nass(
                saf,
                DawrAnsur::Wasm,
                "ما تكلّفه كلّ قاعدة من قواعد المناطق:",
                ataama,
            ));
        }
        for qaida in QaidatTarjama::jamee() {
            if let Some(saf) = takhtit.saf(1.0) {
                ansur.push(AnsurLawha::bi_nass(
                    saf,
                    DawrAnsur::Ibraz,
                    qaida.unwan(),
                    ataama,
                ));
            }
            if let Some(saf) = takhtit.saf(2.0) {
                ansur.push(AnsurLawha::bi_nass(
                    saf,
                    DawrAnsur::Qeema,
                    qaida.wasf(),
                    ataama,
                ));
            }
        }
        if let Some(taqreer) = self.taqreer {
            if let Some(saf) = takhtit.saf(0.25) {
                ansur.push(AnsurLawha::zukhrufi(saf, DawrAnsur::Fasil, ataama));
            }
            if let Some(saf) = takhtit.saf(1.5) {
                ansur.push(AnsurLawha::bi_nass(
                    saf,
                    DawrAnsur::Qeema,
                    taqreer.wasf(),
                    ataama,
                ));
            }
        }
        // Verbatim, as the budget is: the worker's own sentence about its last
        // refusal, whichever kind it was.
        if let Some(khayt) = self.khayt.as_ref() {
            if let Some(saf) = takhtit.saf(0.25) {
                ansur.push(AnsurLawha::zukhrufi(saf, DawrAnsur::Fasil, ataama));
            }
            if let Some(saf) = takhtit.saf(1.0) {
                ansur.push(AnsurLawha::bi_nass(
                    saf,
                    DawrAnsur::Wasm,
                    "حالة القراءة:",
                    ataama,
                ));
            }
            if let Some(saf) = takhtit.saf(2.0) {
                ansur.push(AnsurLawha::bi_nass(
                    saf,
                    DawrAnsur::Qeema,
                    khayt.wasf_arabi(),
                    ataama,
                ));
            }
        }
    }

    /// The disclosure page, which reproduces the text and nothing else.
    ///
    /// Split on the newlines the constant already has, one element per line, so
    /// that what reaches the shaper is [`crate::sidq::NASS_IFSAH_ARABI`]
    /// character for character. Nothing here shortens it, reorders it, drops the
    /// bullet that is least flattering, or replaces it with a summary — the
    /// whole reason that constant lives in [`crate::sidq`] rather than in an
    /// interface is that a copy somewhere else drifts, and drifts in the
    /// direction of sounding better.
    fn bina_ifsah(&self, takhtit: &mut Takhtit, ansur: &mut Vec<AnsurLawha>, ataama: u8) {
        for satr in NASS_IFSAH_ARABI.lines().skip(self.hala.tamreer()) {
            if satr.trim().is_empty() {
                // A blank line in the source is a gap, not a rule. Consuming
                // half a row keeps the paragraph shape the text was written
                // with without inventing a separator the author did not put in.
                let _ = takhtit.saf(0.5);
                continue;
            }
            let Some(mustatil) = takhtit.saf(1.0) else {
                break;
            };
            ansur.push(AnsurLawha::bi_nass(
                mustatil,
                DawrAnsur::Qeema,
                satr,
                ataama,
            ));
        }
    }
}
