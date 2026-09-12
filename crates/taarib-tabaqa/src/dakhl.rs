//! الدخل — reading the panel's chords off the keyboard, from inside a frame.
//!
//! The panel in [`crate::lawhat_tahakkum`] has had a chord table since it was
//! written, and nothing in the payload ever read a key. This is that reader,
//! and it is deliberately the smallest one that works: on every frame it asks
//! the operating system which keys are down and fires a command on the frame
//! a chord *becomes* held. No hook is installed and no message is intercepted.
//!
//! ## What that cannot do, stated plainly
//!
//! A poll sees keys; it does not own them. When the panel is focused and the
//! player presses Escape to close it, the game receives that Escape too.
//! [`crate::lawhat_tahakkum::HalatLawha::yabtali_dakhl`] describes the input
//! ownership a hook on the window's message queue would give, and this module
//! does not give it — which is why every chord that ships is `Ctrl+Shift+…`,
//! the emptiest corner of the keyboard in the games this was tried against,
//! and why the panel takes no typed text at all through this reader.
//!
//! On platforms other than Windows nothing is polled: the OpenGL path on Linux
//! has no process-wide key state to ask for without a display connection, and
//! opening one from inside a game's frame is not a thing this crate does. The
//! reader answers no commands there and says so through [`RasidDakhl::yaqra`].

use crate::lawhat_tahakkum::{Amr, Ikhtisarat, Muaddil, RamzMiftah, Watar};

/// The chord reader: which chords were held on the previous poll.
#[derive(Debug, Default)]
pub struct RasidDakhl {
    sabiqa: Vec<Watar>,
}

impl RasidDakhl {
    /// A reader with nothing held.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { sabiqa: Vec::new() }
    }

    /// Whether this platform's reader can see keys at all.
    #[must_use]
    pub const fn yaqra() -> bool {
        cfg!(windows)
    }

    /// The commands whose chords became held since the previous poll.
    ///
    /// Edge-triggered: a chord held across ten frames fires once. The
    /// modifiers must match the chord exactly — `Ctrl+Shift+T` does not fire
    /// `Ctrl+T`, and a bare key does not fire while any modifier is down — so
    /// two bindings that share a key cannot both fire from one press.
    /// [`Amr::Ighlaq`], which ships on bare Escape, is consulted only while the
    /// panel is visible, exactly as its documentation promises.
    pub fn iqra(&mut self, ikhtisarat: &Ikhtisarat, lawha_zahira: bool) -> Vec<Amr> {
        let muaddilat = muaddilat_al_aan();
        let mut mahdutha: Vec<Watar> = Vec::new();
        let mut awamir: Vec<Amr> = Vec::new();
        for (amr, watar) in ikhtisarat.rubut() {
            if !watar.salih() {
                continue;
            }
            if matches!(amr, Amr::Ighlaq) && !lawha_zahira {
                continue;
            }
            let mamsuk = watar.muaddilat == muaddilat && madghut(watar.miftah);
            if !mamsuk {
                continue;
            }
            mahdutha.push(watar);
            if !self.sabiqa.contains(&watar) {
                awamir.push(amr);
            }
        }
        self.sabiqa = mahdutha;
        awamir
    }
}

/// The modifiers held right now.
#[cfg(windows)]
fn muaddilat_al_aan() -> Muaddil {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let mut muaddilat = Muaddil::LA;
    if madghut_vk(VK_CONTROL.0) {
        muaddilat = muaddilat.ma(Muaddil::TAHAKKUM);
    }
    if madghut_vk(VK_SHIFT.0) {
        muaddilat = muaddilat.ma(Muaddil::SIFT);
    }
    if madghut_vk(VK_MENU.0) {
        muaddilat = muaddilat.ma(Muaddil::BADEEL);
    }
    if madghut_vk(VK_LWIN.0) || madghut_vk(VK_RWIN.0) {
        muaddilat = muaddilat.ma(Muaddil::NAFIDHA);
    }
    muaddilat
}

/// The modifiers held right now: none, because nothing is polled here.
#[cfg(not(windows))]
const fn muaddilat_al_aan() -> Muaddil {
    Muaddil::LA
}

/// Whether a key is down right now.
#[cfg(windows)]
fn madghut(miftah: RamzMiftah) -> bool {
    vk_min_hid(miftah).is_some_and(madghut_vk)
}

/// Whether a key is down right now: never, because nothing is polled here.
#[cfg(not(windows))]
const fn madghut(miftah: RamzMiftah) -> bool {
    let _ = miftah;
    false
}

/// Whether a virtual key is down right now.
///
/// `GetAsyncKeyState` reports the key's state at the moment of the call in its
/// high bit, for the whole session rather than for a window, which is what a
/// poll from a render thread that owns no window wants.
#[cfg(windows)]
fn madghut_vk(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    // SAFETY: `GetAsyncKeyState` takes a virtual-key code by value, reads no
    // memory of ours and writes none, and is documented to accept any value
    // — an unknown code answers zero.
    let hala = unsafe { GetAsyncKeyState(i32::from(vk)) };
    hala.cast_unsigned() & 0x8000 != 0
}

/// The Windows virtual-key code for a HID keyboard usage, when the table
/// names it.
///
/// Positional, like the usages themselves: the letters map to the virtual
/// keys of their QWERTY positions, which is what `GetAsyncKeyState` answers
/// for the physical key regardless of the player's layout — the same rule
/// every game's `W` `A` `S` `D` follows.
#[cfg(windows)]
const fn vk_min_hid(miftah: RamzMiftah) -> Option<u16> {
    let usage = miftah.hid();
    Some(match usage {
        // `a`..`z` are consecutive on both pages.
        0x04..=0x1D => 0x41_u16.saturating_add(usage.saturating_sub(0x04)),
        // `1`..`9`, then `0`.
        0x1E..=0x26 => 0x31_u16.saturating_add(usage.saturating_sub(0x1E)),
        0x27 => 0x30,
        0x28 => 0x0D,
        0x29 => 0x1B,
        0x2A => 0x08,
        0x2B => 0x09,
        0x2C => 0x20,
        0x2D => 0xBD,
        0x2E => 0xBB,
        0x2F => 0xDB,
        0x30 => 0xDD,
        0x31 => 0xDC,
        0x33 => 0xBA,
        0x34 => 0xDE,
        0x35 => 0xC0,
        0x36 => 0xBC,
        0x37 => 0xBE,
        0x38 => 0xBF,
        // `F1`..`F12`.
        0x3A..=0x45 => 0x70_u16.saturating_add(usage.saturating_sub(0x3A)),
        0x46 => 0x2C,
        0x47 => 0x91,
        0x48 => 0x13,
        0x49 => 0x2D,
        0x4A => 0x24,
        0x4B => 0x21,
        0x4C => 0x2E,
        0x4D => 0x23,
        0x4E => 0x22,
        0x4F => 0x27,
        0x50 => 0x25,
        0x51 => 0x28,
        0x52 => 0x26,
        _ => return None,
    })
}

#[cfg(all(test, windows))]
mod ikhtibarat {
    use super::*;

    #[test]
    fn al_huruf_wa_al_arqam_tuqabil_mawadiaha() {
        assert_eq!(vk_min_hid(RamzMiftah::HARF_T), Some(0x54));
        assert_eq!(vk_min_hid(RamzMiftah::HARF_H), Some(0x48));
        assert_eq!(vk_min_hid(RamzMiftah::min_hid(0x1E)), Some(0x31));
        assert_eq!(vk_min_hid(RamzMiftah::min_hid(0x27)), Some(0x30));
        assert_eq!(vk_min_hid(RamzMiftah::F1), Some(0x70));
        assert_eq!(vk_min_hid(RamzMiftah::F12), Some(0x7B));
        assert_eq!(vk_min_hid(RamzMiftah::HURUB), Some(0x1B));
        assert_eq!(vk_min_hid(RamzMiftah::min_hid(0xE0)), None);
    }
}
