//! بيان التثبيت — the manifest, and the single mechanism through which anything
//! in Taarib is allowed to change a game.
//!
//! `taarib-muhawwil-nusus`'s `hifz` module established this discipline for one
//! family of engines: no patcher owns a file handle, every write comes through a
//! guard that cannot be constructed without the original already being
//! preserved, and the record of the change is on the device before the change
//! happens. This module is that discipline generalized to every engine, plus the
//! four things a whole-product installer needs and a script-engine patcher did
//! not: **permissions**, **timestamps**, **launcher settings**, and the
//! separation of a **text** install from a **voice** install.
//!
//! ## The contract, in the order it must happen
//!
//! **1. Preserve before touching.** [`Tathbeet::ihfaz`] compresses a file's
//! original bytes into the backup directory and records its relative path, its
//! size, its BLAKE3 fingerprint, its permissions, its modification and access
//! times, and — the field most often got wrong — whether it existed at all
//! beforehand. A file Taarib *added* has no original: undoing it means deleting
//! it, not restoring it. Conflating the two is how an uninstall leaves litter
//! behind, or worse, writes an empty "original" over a file the game shipped.
//!
//! **2. The manifest entry is durable before the file is modified.** Not after,
//! and not in the same step. Reversing the order leaves a patch interrupted
//! between the write and the record as a modified game file with nothing on disk
//! saying it was modified — no backup key to find, no fingerprint to compare, no
//! way for any later run to tell a patched file from a file the game shipped.
//! That state is not recoverable afterwards by any amount of cleverness, which
//! is why the ordering is a property of the types here rather than a rule twelve
//! callers have to remember.
//!
//! **3. Every write is atomic.** Bytes go beside the target, are flushed to the
//! device, and are renamed over it, through
//! [`taarib_usus::masarat::kitaba_dharra`]. There is no second atomic-write
//! implementation in this crate, deliberately: two of them would be two sets of
//! fsync ordering rules, and the one used less often would be the one that was
//! wrong. A half-written `GameAssembly.dll` is not a damaged patch, it is an
//! unplayable game.
//!
//! **4. Text and voice are two installations, never one.** They have separate
//! manifests ([`NawTathbeet::ism_bayan`]), separate backup subdirectories, and
//! separate identities. A user who wants the Arabic subtitles gone and the
//! Arabic dubbing kept must get exactly that, and the way to guarantee it is not
//! a flag inside one manifest that a restore has to remember to honour — it is
//! two files, so that the code path removing the text patch has no way to name
//! the voice patch's records. `taraju` reads the manifest whose name comes from
//! the kind it was asked for and refuses a manifest whose own recorded kind does
//! not match, which catches the one remaining way they could be crossed: a
//! manifest copied or renamed by hand.
//!
//! ## The guard makes rule 1 structural, and rule "nothing outside the manifest"
//! with it
//!
//! [`HarisTathbeet`] has no public constructor and no public fields, and it is
//! the only type in this crate with a method that writes into a game directory.
//! [`Tathbeet`] exposes no `write(path, bytes)`. The only way to obtain a guard
//! is to call [`Tathbeet::ihfaz`] or [`Tathbeet::adif`] and have them succeed,
//! which means the backup exists and the manifest line naming it is already on
//! the device.
//!
//! The generalization `hifz` did not make is in the guard's *shape*. A guard
//! that carried `(manifest key, absolute path)` as two fields would be one
//! assignment away from a guard whose path and whose record disagree — and a
//! write through such a guard modifies a file that the manifest describes under
//! a different name, which is precisely "a file outside the manifest". So
//! [`HarisTathbeet`] stores **only the manifest key**. Its absolute path is
//! *derived*, every time it is needed, by joining the game root onto the key
//! that the record is filed under, through the workspace's one path-safety
//! check. There is no second source for the path, so there is nothing for the
//! record to disagree with: a write outside the manifest is not discouraged, it
//! is unrepresentable. Constructing a guard for a path with no record requires
//! calling a function that does not exist.
//!
//! **5. Additive delivery is preferred wherever the engine allows it.** Where
//! the payload can be a *new* file, [`Tathbeet::adif`] records an addition and
//! uninstall is a delete — no original to hold, no fingerprint to reproduce,
//! nothing to get wrong. It also survives the thing that kills most patches: a
//! game update overwrites the files it ships and leaves files it has never heard
//! of alone.
//!
//! **6. Restores are verified, never assumed**, and that is `taraju`'s job. What
//! this module owes it is a record complete enough that verification is
//! possible: the original's fingerprint, its size, its mode, its times, and a
//! per-line completion marker so an interrupted uninstall resumes rather than
//! restarts.
//!
//! ## Backups are compressed; the fingerprint is of the plaintext
//!
//! Originals are stored zstd-compressed, because a full text-bearing asset tree
//! is gigabytes and a user should not pay for the patch twice. The recorded
//! fingerprint, though, is of the **uncompressed** original, and that is not an
//! oversight.
//!
//! Hashing the compressed form would make every verification depend on the
//! compressor rather than on the user's file. zstd does not promise that two
//! releases produce identical frames for identical input — a level's parameters,
//! the window log, the block splitter and the entropy stage have all changed
//! across 1.x, and the library is vendored, so a Taarib build linking a newer
//! zstd would compute a different digest for the same original. Every backup
//! written by the older build would then read as corrupt to the newer one, and
//! the product would refuse to restore games it had perfectly good originals
//! for. Hashing the plaintext makes the digest a property of the bytes the user
//! owns, which is the only thing it should ever have been a property of. It also
//! means the verification after decompression is an end-to-end check of the
//! whole storage path — container, compressor and disk — rather than a check of
//! the disk alone.
//!
//! ## What this module does not decide
//!
//! It does not decide *what* to write — that is each engine's adapter — and it
//! does not own the installations ledger, which lives in `taarib-makhzan` and
//! records the same facts in `SQLite` for the library view. The manifest here is
//! the on-disk source of truth that works when the database does not: a user who
//! copies their game and their `nusakh/` directory to another machine can still
//! uninstall.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::manassa::NizamTashghil;
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::khata::{IttijahDaght, KhataTathbeet, min_khata_io, tul_u64};

/// The subdirectory of an installation's backup directory holding originals.
pub const ISM_MUJALLAD_ASL: &str = "asl";

/// The largest original this build will preserve, in bytes.
///
/// Four gibibytes. Larger than `hifz`'s ceiling because this crate preserves
/// whole engine assemblies and Unity bundle archives, not only text-bearing
/// script files, and a `resources.assets` above two gibibytes is real. It is
/// checked against the size the filesystem *declares*, before a single byte is
/// read, so a pathological file is a refusal naming the number rather than an
/// exhausted machine.
pub const AQSA_HAJM_NUSKHA: u64 = 4_294_967_296;

/// The zstd level backups are stored at.
///
/// Three, the library default. Higher levels buy single-digit percentages on
/// already-compressed game assets and cost multiples of the time, and this runs
/// while a user is watching a progress bar before their game is patched. The
/// level is recorded nowhere and does not need to be: decompression is
/// level-agnostic, which is exactly why the *level* may change between builds
/// and the *fingerprint* may not.
pub const MUSTAWA_DAGHT: i32 = 3;

/// The longest prefix of a file's own name kept in a backup key.
const HADD_ISM_MIFTAH: usize = 40;

/// Seconds in a day, for the manifest's own timestamp.
const THAWANI_YAWM: i64 = 86_400;

/// Which of a game's two possible Taarib installations a manifest describes.
///
/// Two installations, two manifests, two backup directories. The kind is not a
/// filter applied to one manifest — it selects which file is opened — because a
/// filter is a line of code that a later refactor can drop, and a user who
/// uninstalls Arabic text and loses their Arabic dubbing has no way to get it
/// back except by downloading it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawTathbeet {
    /// Translated text: strings, layouts, the glyph atlas and the font.
    Nass,

    /// Translated voice: dubbed audio replacing or accompanying the original.
    ///
    /// Overwhelmingly larger than a text patch and overwhelmingly more likely to
    /// be the thing a user wants to keep when they remove the other, which is
    /// why the separation matters more here than symmetry alone would suggest.
    Sawt,
}

impl NawTathbeet {
    /// Both kinds, in a stable order for a sweep.
    pub const KULL: [Self; 2] = [Self::Nass, Self::Sawt];

    /// The manifest file name for this kind.
    ///
    /// The file name *is* the separation. `taraju` never opens a manifest by a
    /// name a caller supplied; it opens the one this returns for the kind it was
    /// asked to remove.
    #[must_use]
    pub const fn ism_bayan(self) -> &'static str {
        match self {
            Self::Nass => "bayan-nass.json",
            Self::Sawt => "bayan-sawt.json",
        }
    }

    /// The subdirectory of the backup root holding this kind's originals.
    #[must_use]
    pub const fn ism_mujallad(self) -> &'static str {
        match self {
            Self::Nass => "nass",
            Self::Sawt => "sawt",
        }
    }

    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Nass => "text",
            Self::Sawt => "voice",
        }
    }

    /// The name shown to an Arabic-speaking user.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::Nass => "النصّ",
            Self::Sawt => "الصوت",
        }
    }
}

/// What an installation did to one path.
///
/// The distinction between the first two is the whole reversibility contract.
/// Undoing a [`NawTaghyeer::Tadeel`] means putting a specific original back and
/// proving it came back; undoing a [`NawTaghyeer::Idafa`] means removing a file
/// that was never the game's. Treating an addition as a modification restores a
/// nonexistent original; treating a modification as an addition deletes a file
/// the game needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawTaghyeer {
    /// A file that existed and was rewritten. Undone by restoring the backup.
    Tadeel,

    /// A file Taarib created that was not there before. Undone by deleting it.
    Idafa,

    /// A directory Taarib created to hold additions.
    ///
    /// Recorded so that an additive payload leaves nothing behind. Removed on
    /// uninstall only when it is empty, because a translator who dropped their
    /// own files into `BepInEx/plugins/` should not lose them to an uninstall.
    MujalladMudaf,
}

impl NawTaghyeer {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Tadeel => "modified",
            Self::Idafa => "added",
            Self::MujalladMudaf => "added directory",
        }
    }

    /// Whether undoing this means deleting rather than restoring.
    #[must_use]
    pub const fn yuhdhaf(self) -> bool {
        matches!(self, Self::Idafa | Self::MujalladMudaf)
    }
}

/// A file's original permissions, as two independent facts rather than one.
///
/// A single portable abstraction over these would lose information in both
/// directions, and both losses break real games.
///
/// A Unix mode is twelve bits: three independent triads plus setuid, setgid and
/// the sticky bit. A Windows read-only attribute is one bit. Collapsing the mode
/// into that bit throws away the execute bits — and a Ren'Py game ships
/// `lib/py3-linux-x86_64/pythonw`, a Unity game on Linux ships its player
/// binary, and a GOG installation ships `start.sh`; every one of them stops
/// starting the moment its execute bit does not come back. Expanding the bit
/// into a mode is worse, because there is no correct answer: a manifest written
/// on Windows knows nothing about who owned the file on the Linux machine it is
/// being restored on, so any mode it invents is a mode somebody chose at random.
///
/// So both are stored, each optional, each written only by the platform that
/// understands it, and a restore applies only the field its own platform owns.
/// The absent field is not a default and is never synthesised: a manifest
/// carrying no Unix mode says "this manifest was not written on a Unix system",
/// and a restore on Unix reading it leaves the file's existing mode alone rather
/// than inventing one. That is the honest behaviour — a game copied between
/// operating systems has already had its permissions rewritten by whatever
/// copied it, and Taarib is not the component that gets to guess what they were.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SalahiyatMalaf {
    /// The Unix permission bits, `st_mode & 0o7777`, when recorded on Unix.
    ///
    /// Masked to the permission bits rather than stored whole, because the file
    /// type bits in `st_mode` are not a permission and feeding them back to
    /// `chmod` is meaningless.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namat_unix: Option<u32>,

    /// The Windows read-only attribute, when recorded on Windows.
    ///
    /// Deliberately not derived from [`SalahiyatMalaf::namat_unix`]. `std`'s
    /// `Permissions::readonly` answers on both platforms, and on Unix it means
    /// "no write bit is set for anybody" — a different statement about a
    /// different thing. Recording it on Unix would put a value in this field
    /// that a Windows restore would then apply as though a Windows machine had
    /// chosen it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qira_faqat: Option<bool>,
}

impl SalahiyatMalaf {
    /// Reads the permissions of a file, recording only what this platform owns.
    #[must_use]
    pub fn min_bayanat(bayanat: &fs::Metadata) -> Self {
        // A target that is neither Unix nor Windows records nothing and restores
        // nothing, which is the correct behaviour for a platform whose permission
        // model this build has never seen — better an empty record than a field
        // some future restore would apply as though it meant something.
        let mut salahiyat = Self::default();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            salahiyat.namat_unix = Some(bayanat.permissions().mode() & 0o7777);
        }

        #[cfg(windows)]
        {
            salahiyat.qira_faqat = Some(bayanat.permissions().readonly());
        }

        #[cfg(not(any(unix, windows)))]
        {
            let _ = (bayanat, &mut salahiyat);
        }

        salahiyat
    }

    /// Whether anything at all was recorded.
    #[must_use]
    pub const fn khali(self) -> bool {
        self.namat_unix.is_none() && self.qira_faqat.is_none()
    }

    /// Applies the field this platform owns, leaving the other alone.
    ///
    /// A missing field for the running platform is a no-op and not a failure:
    /// the manifest was written somewhere else, and the file's current
    /// permissions are the ones whatever copied the game gave it. Overwriting
    /// them with a guess would be a change, not a restore.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::SalahiyatGhayrMustaada`] when the permission the
    /// manifest recorded cannot be applied, which on Unix means the process does
    /// not own the file and on Windows means the attribute could not be cleared.
    /// The bytes are already back at this point, so this is reported rather than
    /// silently accepted: a file with the right contents and the wrong mode is
    /// not a restored file.
    pub fn tatbeeq(self, masar: &Path) -> Result<(), KhataTathbeet> {
        #[cfg(unix)]
        {
            if let Some(namat) = self.namat_unix {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(masar, fs::Permissions::from_mode(namat)).map_err(
                    |sabab| KhataTathbeet::SalahiyatGhayrMustaada {
                        masar: masar.to_path_buf(),
                        sabab: format!("mode {namat:04o} could not be set: {sabab}"),
                    },
                )?;
            }
        }

        #[cfg(windows)]
        {
            if let Some(qira_faqat) = self.qira_faqat {
                let bayanat = fs::metadata(masar).map_err(|sabab| {
                    KhataTathbeet::SalahiyatGhayrMustaada {
                        masar: masar.to_path_buf(),
                        sabab: format!(
                            "the file's current attributes could not be read: {sabab}"
                        ),
                    }
                })?;
                let mut salahiyat = bayanat.permissions();
                salahiyat.set_readonly(qira_faqat);
                fs::set_permissions(masar, salahiyat).map_err(|sabab| {
                    KhataTathbeet::SalahiyatGhayrMustaada {
                        masar: masar.to_path_buf(),
                        sabab: format!(
                            "the read-only attribute could not be set to {qira_faqat}: {sabab}"
                        ),
                    }
                })?;
            }
        }

        let _ = (self, masar);
        Ok(())
    }

    /// Clears the read-only attribute so a file can be rewritten.
    ///
    /// Called before a restore writes over a file, not as part of one. A game
    /// that ships a read-only `config.ini` and a patch that modified it leaves a
    /// read-only file on disk; the atomic write renames over it, which Windows
    /// refuses for a read-only target. The original attribute is put back
    /// afterwards by [`SalahiyatMalaf::tatbeeq`] from the *recorded* value, so
    /// clearing it here cannot leak into the restored state.
    ///
    /// Returns whether anything was changed, so a caller can tell a file that
    /// needed unlocking from one that did not.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::SalahiyatGhayrMustaada`] when the attribute is set and
    /// cannot be cleared, which means the write that follows would fail anyway
    /// and failing here names the actual reason.
    pub fn ataih_kitaba(masar: &Path) -> Result<bool, KhataTathbeet> {
        let Ok(bayanat) = fs::metadata(masar) else { return Ok(false) };
        let mut salahiyat = bayanat.permissions();
        if !salahiyat.readonly() {
            return Ok(false);
        }

        // On Unix the owner's write bit is added rather than the read-only flag
        // cleared: `set_permissions` with `readonly(false)` sets all three write
        // bits, which would leave a game file world-writable for as long as the
        // restore runs — and permanently, if the manifest recorded no mode.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let namat = salahiyat.mode();
            salahiyat.set_mode(namat | 0o200);
        }
        #[cfg(not(unix))]
        {
            salahiyat.set_readonly(false);
        }

        fs::set_permissions(masar, salahiyat).map_err(|sabab| {
            KhataTathbeet::SalahiyatGhayrMustaada {
                masar: masar.to_path_buf(),
                sabab: format!(
                    "the file is read-only and the attribute could not be cleared, so it \
                     cannot be written: {sabab}"
                ),
            }
        })?;
        Ok(true)
    }
}

/// A point in time as the manifest stores it: whole seconds and nanoseconds
/// since the Unix epoch, signed.
///
/// Signed because a game archive extracted with a broken timestamp, or a file
/// copied from a filesystem with a different epoch, can genuinely carry a
/// modification time before 1970, and a `u64` would have to either clamp it to
/// the epoch or refuse the file. Clamping is a silent change to something the
/// manifest exists to preserve exactly.
///
/// Stored as two integers rather than as an RFC 3339 string because this value
/// is compared for equality on every startup sweep — the fast path in `tahaqquq`
/// asks whether a file's modification time is still the one recorded — and
/// comparing two formatted strings would make that answer depend on the
/// formatter's rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WaqtNizam {
    /// Whole seconds since the Unix epoch, negative before it.
    pub thawani: i64,
    /// Nanoseconds after `thawani`, always in `0..1_000_000_000`.
    pub nano: u32,
}

impl WaqtNizam {
    /// Reads a system time, or [`None`] when it is outside what this can hold.
    ///
    /// `None` rather than a clamp. A timestamp beyond a signed 64-bit second
    /// count is a filesystem reporting nonsense, and recording a made-up value
    /// for it would make the fast path in `tahaqquq` compare a real time against
    /// an invented one and conclude the file had changed on every sweep.
    #[must_use]
    pub fn min_nizam(waqt: SystemTime) -> Option<Self> {
        match waqt.duration_since(UNIX_EPOCH) {
            Ok(mudda) => Some(Self {
                thawani: i64::try_from(mudda.as_secs()).ok()?,
                nano: mudda.subsec_nanos(),
            }),
            Err(qabl) => {
                let mudda = qabl.duration();
                let thawani = i64::try_from(mudda.as_secs()).ok()?;
                let nano = mudda.subsec_nanos();
                if nano == 0 {
                    Some(Self { thawani: thawani.checked_neg()?, nano: 0 })
                } else {
                    // A duration *before* the epoch is measured backwards, so
                    // the sub-second part has to be turned around: 1.25s before
                    // the epoch is -2s plus 750ms, not -1s plus 250ms.
                    Some(Self {
                        thawani: thawani.checked_add(1)?.checked_neg()?,
                        nano: 1_000_000_000_u32.saturating_sub(nano),
                    })
                }
            }
        }
    }

    /// Rebuilds a system time, or [`None`] when this platform cannot represent
    /// it.
    #[must_use]
    pub fn ila_nizam(self) -> Option<SystemTime> {
        // Clamped rather than trusted. The invariant holds for every value this
        // module writes, and this value can also arrive from a manifest that was
        // edited by hand — where a nanosecond field of four billion would make
        // `Duration::new` carry into the seconds and abort the process.
        let nano = self.nano.min(999_999_999);
        if self.thawani >= 0 {
            let thawani = u64::try_from(self.thawani).ok()?;
            UNIX_EPOCH.checked_add(Duration::new(thawani, nano))
        } else {
            // Mirror of the conversion above: -2s plus 750ms is 1.25s before
            // the epoch.
            let mutlaq = self.thawani.checked_neg()?;
            let mut thawani = u64::try_from(mutlaq).ok()?;
            let baqi = if nano > 0 {
                thawani = thawani.checked_sub(1)?;
                1_000_000_000_u32.saturating_sub(nano)
            } else {
                0_u32
            };
            UNIX_EPOCH.checked_sub(Duration::new(thawani, baqi))
        }
    }
}

/// A file's original modification and access times.
///
/// Recorded because a restore that puts the bytes back and leaves the file
/// stamped with today's date has not put the file back. Three things notice.
/// A launcher's own integrity check compares size and modification time before
/// it compares anything expensive, so a game whose files all carry the date of
/// the uninstall looks entirely rewritten to it and triggers a multi-gigabyte
/// re-download. Incremental backup tools do the same and copy the whole game
/// again. And `tahaqquq`'s own startup sweep is built on exactly that
/// comparison, so a restore that did not put the times back would make the next
/// verification of a *reinstalled* patch hash every file for nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwqatMalaf {
    /// When the file was last modified.
    pub tadeel: WaqtNizam,

    /// When the file was last read, where the filesystem tracks it.
    ///
    /// Optional because a great many systems do not have a useful answer:
    /// `noatime` and `relatime` are the Linux defaults, and Windows disables
    /// last-access updates by default too. A manifest that recorded a synthetic
    /// access time would be restoring a fact nobody ever observed, so the field
    /// is absent when the platform did not offer one and the restore then leaves
    /// the access time to the filesystem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wusul: Option<WaqtNizam>,
}

impl AwqatMalaf {
    /// Reads a file's times, or [`None`] when the platform reports no
    /// modification time at all.
    #[must_use]
    pub fn min_bayanat(bayanat: &fs::Metadata) -> Option<Self> {
        let tadeel = WaqtNizam::min_nizam(bayanat.modified().ok()?)?;
        let wusul = bayanat.accessed().ok().and_then(WaqtNizam::min_nizam);
        Some(Self { tadeel, wusul })
    }

    /// Puts the times back on a file that has just been rewritten.
    ///
    /// Uses [`std::fs::File::set_times`], which is the only portable way to set
    /// both stamps and has been stable since 1.75; this crate deliberately does
    /// not take a dependency on `filetime` for it. The handle is opened for
    /// writing because Windows requires write access to change a file's times,
    /// while Unix does not — opening read-only would work on Linux and fail on
    /// the platform where most of the product's users are.
    ///
    /// Called *after* the content is written and *before* the permissions are put
    /// back, and neither half of that is interchangeable.
    ///
    /// After the content, because writing sets the modification time; doing this
    /// first would stamp the file and then immediately overwrite the stamp with
    /// the moment of the restore.
    ///
    /// Before the permissions, because this call needs a writable handle —
    /// Windows requires write access to change a file's times, while Unix does
    /// not — and a file whose recorded permissions have already been reapplied may
    /// be read-only again, at which point opening it fails and the times are lost.
    /// Putting the permissions back last is safe in the other direction: `chmod`
    /// updates the inode's change time and not its modification or access time,
    /// and setting the Windows read-only attribute does not touch the last-write
    /// time either, so neither disturbs what this call just set.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::SalahiyatGhayrMustaada`] when the file cannot be opened
    /// for writing or the times cannot be set — reported rather than ignored,
    /// for the same reason a wrong mode is reported: the file on disk is not the
    /// file that was taken away.
    pub fn tatbeeq(&self, masar: &Path) -> Result<(), KhataTathbeet> {
        let khalal = |sabab: String| KhataTathbeet::SalahiyatGhayrMustaada {
            masar: masar.to_path_buf(),
            sabab,
        };

        let Some(tadeel) = self.tadeel.ila_nizam() else {
            return Err(khalal(format!(
                "the recorded modification time ({}s + {}ns) is outside what this platform's \
                 clock can represent",
                self.tadeel.thawani, self.tadeel.nano
            )));
        };

        let malaf = fs::OpenOptions::new().write(true).open(masar).map_err(|sabab| {
            khalal(format!("the file could not be opened to set its timestamps: {sabab}"))
        })?;

        let mut awqat = fs::FileTimes::new().set_modified(tadeel);
        if let Some(wusul) = self.wusul.and_then(WaqtNizam::ila_nizam) {
            awqat = awqat.set_accessed(wusul);
        }
        malaf.set_times(awqat).map_err(|sabab| {
            khalal(format!("the file's timestamps could not be set: {sabab}"))
        })?;
        Ok(())
    }
}

/// Where a setting Taarib changed actually lives.
///
/// Kept as a closed enum rather than a free-text label because the restore path
/// dispatches on it: a registry value and a line in a launcher's configuration
/// are put back by entirely different machinery, and the manifest has to say
/// which one it means without a reader having to parse a string to find out.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum MahallIdad {
    /// A launcher's per-game launch options.
    ///
    /// Steam's `LaunchOptions`, Heroic's wrapper arguments, Lutris's command
    /// prefix. The most common setting an installation touches, because a
    /// framework that has to be preloaded is preloaded from here.
    KhiyaratTashghil {
        /// Which launcher, so a restore knows whose file to edit.
        manassa: String,
        /// The launcher's own identifier for the game.
        muarrif_luba: String,
    },

    /// A Windows registry value.
    SijillWindows {
        /// The hive and key path, exactly as the registry names it.
        miftah: String,
        /// The value name inside that key. Empty means the default value.
        qeema: String,
    },

    /// A key inside a configuration file that is not the game's own.
    ///
    /// A Proton or Wine prefix's `user.reg`, a launcher's `config.json`, a
    /// desktop entry. The path is absolute here and not relative to a game root,
    /// because these genuinely live outside it — which is why they are recorded
    /// as settings rather than as [`SijillTaghyeer`] entries. A file inside the
    /// game is a file; a file outside it is a setting, and the two obey
    /// different rules about what Taarib may do to them.
    MalafIdad {
        /// The file.
        masar: String,
        /// The key inside it.
        miftah: String,
    },

    /// An environment variable set for the game's process.
    MutaghayyirBeea {
        /// The variable's name.
        ism: String,
    },
}

impl MahallIdad {
    /// A one-line description for reports, logs and refusals.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::KhiyaratTashghil { manassa, muarrif_luba } => {
                format!("{manassa} launch options for {muarrif_luba}")
            }
            Self::SijillWindows { miftah, qeema } if qeema.is_empty() => {
                format!("registry {miftah} (default value)")
            }
            Self::SijillWindows { miftah, qeema } => format!("registry {miftah}\\{qeema}"),
            Self::MalafIdad { masar, miftah } => format!("{miftah} in {masar}"),
            Self::MutaghayyirBeea { ism } => format!("environment variable {ism}"),
        }
    }

    /// Which component owns putting this back.
    ///
    /// Named in the refusal a [`crate::taraju::RadIdad`] produces when it cannot
    /// restore a setting, so the message says who should have handled it instead
    /// of leaving a maintainer to work it out.
    #[must_use]
    pub const fn masul(&self) -> &'static str {
        match self {
            Self::KhiyaratTashghil { .. } => "the launcher integration (itlaq)",
            Self::SijillWindows { .. } => "the Windows registry writer",
            Self::MalafIdad { .. } => "the owner of that configuration file",
            Self::MutaghayyirBeea { .. } => "the launch environment builder",
        }
    }
}

/// One setting an installation changed, and what it was before.
///
/// The fifth kind of record, and the one `hifz` had no need of: a script-engine
/// patcher writes files and nothing else, while a whole-product installation
/// also has to put `WINEDLLOVERRIDES` in front of a game's command line or point
/// a launcher at a wrapper script. Those are not files, they cannot be backed up
/// into `nusakh/`, and a user whose launch options still carry Taarib's preload
/// argument after an uninstall is a user editing a text field to undo something
/// Taarib did.
///
/// The previous value is [`Option`] for the same reason
/// [`SijillTaghyeer::kan_mawjudan`] exists: a setting that did not exist before
/// is *removed* on uninstall, and one that existed is *put back*. Writing an
/// empty string where there was no value is not a restore — for a launch options
/// field it is indistinguishable, but for a registry value it leaves a key the
/// game never had, and for an environment variable it makes an unset variable
/// set-and-empty, which several launchers treat as a different thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillIdad {
    /// A stable key for this setting within the manifest.
    ///
    /// Derived from [`MahallIdad`] so that changing one setting twice updates
    /// one record instead of appending a second that claims a different
    /// "previous" value — the second of which would be Taarib's own write.
    pub muarrif: String,

    /// Where it lives.
    pub mahall: MahallIdad,

    /// What was there before, or [`None`] when nothing was.
    pub qeema_sabiqa: Option<String>,

    /// What Taarib put there.
    ///
    /// Recorded so verification can tell "the user changed it again since"
    /// from "it is still what Taarib wrote", which decides whether an uninstall
    /// should put the old value back or leave the user's newer choice alone.
    pub qeema_maktuba: Option<String>,

    /// Whether this record has already been put back.
    #[serde(default)]
    pub istiada_tammat: bool,
}

impl SijillIdad {
    /// Whether undoing this means removing the setting rather than rewriting it.
    #[must_use]
    pub const fn yuhdhaf(&self) -> bool {
        self.qeema_sabiqa.is_none()
    }

    /// A line for a report or a confirmation screen.
    #[must_use]
    pub fn wasf(&self) -> String {
        match (&self.qeema_sabiqa, &self.qeema_maktuba) {
            (None, Some(jadeed)) => format!("{}: set to {jadeed} (was unset)", self.mahall.wasf()),
            (Some(qadeem), Some(jadeed)) => {
                format!("{}: {qadeem} -> {jadeed}", self.mahall.wasf())
            }
            (Some(qadeem), None) => format!("{}: removed (was {qadeem})", self.mahall.wasf()),
            (None, None) => format!("{}: recorded, unchanged", self.mahall.wasf()),
        }
    }
}

/// One line of the manifest: everything needed to undo one path.
///
/// Every field a restore depends on is written before the corresponding file is
/// touched. The optional fields are optional because a directory and an addition
/// genuinely have no original, not because they are sometimes omitted;
/// [`SijillTaghyeer::tahaqquq`] refuses any combination that says otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillTaghyeer {
    /// The path relative to the game's root, with `/` separators.
    ///
    /// Never absolute. A manifest full of absolute paths stops working the
    /// moment the user moves the game to another drive, which is the moment they
    /// are most likely to want to uninstall.
    pub masar: String,

    /// What was done to it.
    pub naw: NawTaghyeer,

    /// Whether anything existed at this path before the installation.
    ///
    /// Recorded separately from [`SijillTaghyeer::naw`] rather than derived from
    /// it, because this is the fact an uninstall is wrong about when it leaves
    /// litter, and a fact worth storing twice is a fact worth cross-checking.
    pub kan_mawjudan: bool,

    /// The original's size in bytes, when there was an original.
    pub hajm_asli: Option<u64>,

    /// The original's fingerprint, for verifying that a restore reproduced it.
    ///
    /// Of the **uncompressed** original. See this module's header for why a
    /// fingerprint of the stored, compressed form would make verification depend
    /// on which zstd release a given Taarib build happened to link.
    pub basma_asliya: Option<Basma>,

    /// The original's permissions, when there was an original.
    pub salahiyat_asliya: Option<SalahiyatMalaf>,

    /// The original's modification and access times, when there was an original.
    pub awqat_asliya: Option<AwqatMalaf>,

    /// The backup's file name inside the originals directory.
    pub nuskha: Option<String>,

    /// How many bytes the compressed backup occupies.
    ///
    /// Diagnostics and the "how much space would uninstalling free" question
    /// only. Never used to verify anything, because it is a property of the
    /// compressor and not of the user's file.
    pub hajm_nuskha: Option<u64>,

    /// The fingerprint of what Taarib wrote, filled in after the write.
    ///
    /// Absent until the guard's write succeeds, which is exactly right: a record
    /// with an original and no new fingerprint describes a file that was
    /// preserved and then not modified, and that is a state a restore handles
    /// correctly.
    pub basma_maktuba: Option<Basma>,

    /// The size of what Taarib wrote, as observed after writing it.
    ///
    /// Half of `tahaqquq`'s fast path. Recorded from the filesystem after the
    /// rename rather than from the buffer's length, so that it is the number a
    /// later `stat` will actually return.
    pub hajm_maktub: Option<u64>,

    /// The modification time of what Taarib wrote, as observed after writing it.
    ///
    /// The other half of the fast path. See [`crate::tahaqquq`] for exactly why
    /// comparing this before hashing is sound for change detection and is not a
    /// tamper check.
    pub waqt_maktub: Option<WaqtNizam>,

    /// Whether this line has already been undone.
    ///
    /// The resumability marker. Written to the manifest after each file is
    /// restored *and verified*, so an uninstall interrupted by a crash, a full
    /// disk or a closed laptop resumes at the first line that is still false.
    #[serde(default)]
    pub istiada_tammat: bool,
}

impl SijillTaghyeer {
    /// Checks that this line is internally consistent.
    ///
    /// Run on every line when a manifest is read, before any of it is acted on.
    /// The rules mirror the `CHECK` constraints on `bayan_tathbeet` in the
    /// database, for the reason they exist there: an installer with a bug records
    /// the modification and forgets the copy, and nobody finds out until a user
    /// tries to uninstall and the original is already gone.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::BayanTalif`] naming the path and the specific
    /// contradiction — a modification with no backup, a backup with no
    /// fingerprint, an addition claiming an original, or a directory claiming
    /// bytes.
    pub fn tahaqquq(&self, bayan: &Path) -> Result<(), KhataTathbeet> {
        let khalal = |sabab: String| KhataTathbeet::BayanTalif {
            masar: bayan.to_path_buf(),
            sabab: format!("{}: {sabab}", self.masar),
        };

        if self.masar.is_empty() {
            return Err(KhataTathbeet::BayanTalif {
                masar: bayan.to_path_buf(),
                sabab: "the manifest holds a line with no path".to_owned(),
            });
        }

        match self.naw {
            NawTaghyeer::Tadeel => {
                if !self.kan_mawjudan {
                    return Err(khalal(
                        "recorded as a modification of a file that is also recorded as not \
                         having existed; one of the two is wrong and neither can be trusted"
                            .to_owned(),
                    ));
                }
                if self.nuskha.is_none() {
                    return Err(khalal(
                        "recorded as a modification with no backup naming its original, so \
                         there is nothing to restore"
                            .to_owned(),
                    ));
                }
                if self.basma_asliya.is_none() || self.hajm_asli.is_none() {
                    return Err(khalal(
                        "recorded as a modification with no original fingerprint or size, so \
                         a restore could not be verified even if it succeeded"
                            .to_owned(),
                    ));
                }
                if self.awqat_asliya.is_none() {
                    return Err(khalal(
                        "recorded as a modification with no original timestamps; the file \
                         could be given back its bytes and not its dates, which is not a \
                         restore"
                            .to_owned(),
                    ));
                }
            }
            NawTaghyeer::Idafa | NawTaghyeer::MujalladMudaf => {
                if self.kan_mawjudan {
                    return Err(khalal(
                        "recorded as something Taarib added, and also as having existed \
                         before the installation; deleting it on uninstall would delete a \
                         file the game shipped"
                            .to_owned(),
                    ));
                }
                if self.nuskha.is_some() || self.basma_asliya.is_some() {
                    return Err(khalal(
                        "recorded as something Taarib added, and also as having an original \
                         to restore"
                            .to_owned(),
                    ));
                }
                if self.salahiyat_asliya.is_some() || self.awqat_asliya.is_some() {
                    return Err(khalal(
                        "recorded as something Taarib added, and also as having original \
                         permissions or timestamps that predate it"
                            .to_owned(),
                    ));
                }
            }
        }

        if matches!(self.naw, NawTaghyeer::MujalladMudaf)
            && (self.hajm_asli.is_some()
                || self.basma_maktuba.is_some()
                || self.hajm_maktub.is_some())
        {
            return Err(khalal("recorded as a directory and also as having contents".to_owned()));
        }

        Ok(())
    }

    /// The path this line names, resolved against a game root.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] when the recorded path is not a relative
    /// path that stays inside the root — a manifest that has been edited, or one
    /// carrying an entry from a hostile patch.
    pub fn masar_kamil(&self, jidhr_luba: &Path) -> Result<PathBuf, KhataTathbeet> {
        dakhil_aw_khata(jidhr_luba, &self.masar)
    }

    /// Whether the size and modification time on disk still match what was
    /// recorded after the write.
    ///
    /// The cheap comparison `tahaqquq`'s sweep runs before it decides to hash.
    /// Answers `false` when either fact was never recorded, which is the safe
    /// direction: an unknown is treated as a mismatch and the file is hashed.
    #[must_use]
    pub fn yutabiq_bila_basma(&self, bayanat: &fs::Metadata) -> bool {
        let (Some(hajm), Some(waqt)) = (self.hajm_maktub, self.waqt_maktub) else {
            return false;
        };
        if bayanat.len() != hajm {
            return false;
        }
        bayanat
            .modified()
            .ok()
            .and_then(WaqtNizam::min_nizam)
            .is_some_and(|hali| hali == waqt)
    }
}

/// The manifest: every path and every setting one installation touched, and how
/// to put each one back.
///
/// Serialized as JSON with a schema version, through
/// [`taarib_usus::mukhattat`]. The version is read and checked *before any other
/// field is looked at*, so a manifest written by a newer build is refused
/// outright instead of being partially understood. A partial read here would be
/// uniquely bad: the fields a newer build added would be dropped, the file would
/// be rewritten without them at the next save, and the information needed to undo
/// the installation would be gone while the installation was still in place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BayanTathbeet {
    /// The identity of this installation's backup directory.
    ///
    /// The same string used for the directory under `nusakh/`, so a manifest and
    /// the originals it names cannot drift apart.
    pub huwiya: String,

    /// Which of the two installations this is.
    ///
    /// Cross-checked against the file the manifest was read from. A
    /// `bayan-nass.json` whose recorded kind is [`NawTathbeet::Sawt`] has been
    /// copied or renamed by hand, and acting on it would restore the wrong
    /// installation's files — the exact failure the two-file layout exists to
    /// prevent, arriving through the one door the layout leaves open.
    pub naw: NawTathbeet,

    /// The game this was installed into.
    pub luba: LubaId,

    /// The launcher the game was found through.
    pub masdar: MasdarLuba,

    /// The game's display name, for a report a person reads.
    pub ism_luba: String,

    /// The patch's lineage.
    pub ruqaa: RuqaaId,

    /// The revision that was installed.
    pub murajaa: RuqaaRevision,

    /// The build fingerprint the patch was installed against.
    ///
    /// What makes a store update detectable without asking the user whether
    /// anything updated.
    pub basma_bina: Option<Basma>,

    /// Where the game was when the installation ran.
    ///
    /// Diagnostics only. Every operation takes the root as an argument and never
    /// reads it from here, precisely so that a user who moved their game between
    /// installing and uninstalling can still uninstall.
    pub jidhr_luba_asli: PathBuf,

    /// The operating system the installation was performed on.
    ///
    /// Read by the restore to know whether a recorded permission field is one it
    /// should apply. A Unix mode in a manifest written on Windows would be a
    /// field nobody set; recording the platform makes "this manifest has no
    /// opinion about Unix modes" a statement rather than an inference.
    pub nizam: NizamTashghil,

    /// The version of Taarib that performed the installation.
    ///
    /// The first question asked of any restore that behaved oddly is "which
    /// build wrote this", and a manifest that cannot answer it turns a
    /// ten-minute diagnosis into a guess.
    pub bina_taarib: String,

    /// When the installation ran, RFC 3339 in UTC.
    ///
    /// Used by `tahaqquq` as the reference point that separates "changed since
    /// Taarib wrote it" from "was already like that", which is what makes the
    /// store-update and user-edit cases distinguishable at all.
    pub waqt: String,

    /// One line per path, keyed by the relative path.
    ///
    /// A map rather than a list, so one path cannot appear twice with two
    /// different originals — the state in which a restore's outcome depends on
    /// which entry it happened to read first.
    pub sijillat: BTreeMap<String, SijillTaghyeer>,

    /// One line per setting, keyed by its identifier.
    #[serde(default)]
    pub idadat: BTreeMap<String, SijillIdad>,
}

impl BayanTathbeet {
    /// An empty manifest for a new installation.
    #[must_use]
    pub fn jadeed(huwiya: impl Into<String>, naw: NawTathbeet, luba: &TarifLuba) -> Self {
        Self {
            huwiya: huwiya.into(),
            naw,
            luba: luba.luba,
            masdar: luba.masdar.clone(),
            ism_luba: luba.ism.clone(),
            ruqaa: luba.ruqaa,
            murajaa: luba.murajaa,
            basma_bina: luba.basma_bina,
            jidhr_luba_asli: luba.jidhr.clone(),
            nizam: NizamTashghil::hali(),
            bina_taarib: env!("CARGO_PKG_VERSION").to_owned(),
            waqt: waqt_alaan(),
            sijillat: BTreeMap::new(),
            idadat: BTreeMap::new(),
        }
    }

    /// How many paths the installation touched.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.sijillat.len()
    }

    /// Whether the installation touched nothing at all.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.sijillat.is_empty() && self.idadat.is_empty()
    }

    /// The line for one relative path.
    #[must_use]
    pub fn sijill(&self, masar: &str) -> Option<&SijillTaghyeer> {
        self.sijillat.get(masar)
    }

    /// Every line, in ascending path order.
    pub fn sijillat(&self) -> impl Iterator<Item = &SijillTaghyeer> {
        self.sijillat.values()
    }

    /// Every setting, in ascending identifier order.
    pub fn idadat(&self) -> impl Iterator<Item = &SijillIdad> {
        self.idadat.values()
    }

    /// How many paths and settings are still waiting to be undone.
    #[must_use]
    pub fn mutabaqqi(&self) -> usize {
        let malaffat = self.sijillat.values().filter(|s| !s.istiada_tammat).count();
        let idadat = self.idadat.values().filter(|s| !s.istiada_tammat).count();
        malaffat.saturating_add(idadat)
    }

    /// Every path and setting still waiting to be undone, in a stable order.
    ///
    /// What [`KhataTathbeet::IstiadaNaqisa`] carries when a restore stops part
    /// of the way through. Files first, then settings, each ascending, so two
    /// runs of the same failed uninstall produce the same list and a user
    /// comparing them can see progress.
    #[must_use]
    pub fn qaimat_mutabaqqi(&self) -> Vec<String> {
        let mut qaima: Vec<String> = self
            .sijillat
            .values()
            .filter(|s| !s.istiada_tammat)
            .map(|s| s.masar.clone())
            .collect();
        qaima.extend(
            self.idadat
                .values()
                .filter(|s| !s.istiada_tammat)
                .map(|s| format!("[setting] {}", s.mahall.wasf())),
        );
        qaima
    }

    /// Whether every line has been undone and verified.
    #[must_use]
    pub fn ustuidat_bilkamil(&self) -> bool {
        self.sijillat.values().all(|s| s.istiada_tammat)
            && self.idadat.values().all(|s| s.istiada_tammat)
    }

    /// How many bytes the backup directory holds for this installation.
    #[must_use]
    pub fn hajm_nusakh(&self) -> u64 {
        self.sijillat
            .values()
            .filter_map(|s| s.hajm_nuskha)
            .fold(0_u64, u64::saturating_add)
    }

    /// Checks every line for internal consistency, and that the manifest is the
    /// one it claims to be.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::BayanTalif`] when a line's key does not match the path
    /// inside it, when a setting's key does not match its identifier, when the
    /// recorded kind is not the kind the caller opened, or whatever
    /// [`SijillTaghyeer::tahaqquq`] refuses at the first inconsistent line.
    pub fn tahaqquq(&self, bayan: &Path, matlub: NawTathbeet) -> Result<(), KhataTathbeet> {
        if self.naw != matlub {
            return Err(KhataTathbeet::BayanTalif {
                masar: bayan.to_path_buf(),
                sabab: format!(
                    "this file is the {} manifest and the record inside it says {}. It has \
                     been copied or renamed outside Taarib, and acting on it would restore \
                     the other installation's files.",
                    matlub.ism(),
                    self.naw.ism()
                ),
            });
        }

        for (miftah, sijill) in &self.sijillat {
            if miftah != &sijill.masar {
                return Err(KhataTathbeet::BayanTalif {
                    masar: bayan.to_path_buf(),
                    sabab: format!(
                        "this line is filed under {miftah} and names {} inside itself; the \
                         manifest has been edited outside Taarib",
                        sijill.masar
                    ),
                });
            }
            sijill.tahaqquq(bayan)?;
        }

        for (miftah, idad) in &self.idadat {
            if miftah != &idad.muarrif {
                return Err(KhataTathbeet::BayanTalif {
                    masar: bayan.to_path_buf(),
                    sabab: format!(
                        "this setting is filed under {miftah} and names {} inside itself",
                        idad.muarrif
                    ),
                });
            }
        }

        Ok(())
    }

    /// The manifest as lines for the log, the console and the detail screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.sijillat.len().saturating_add(4));
        sutur.push(format!(
            "{} [{}] {} r{} installed {} by Taarib {}",
            self.ism_luba,
            self.naw.ism(),
            self.ruqaa.mukhtasar(),
            self.murajaa.qeema(),
            self.waqt,
            self.bina_taarib
        ));
        let (mut tadeel, mut idafa, mut mujallad) = (0_usize, 0_usize, 0_usize);
        for sijill in self.sijillat.values() {
            match sijill.naw {
                NawTaghyeer::Tadeel => tadeel = tadeel.saturating_add(1),
                NawTaghyeer::Idafa => idafa = idafa.saturating_add(1),
                NawTaghyeer::MujalladMudaf => mujallad = mujallad.saturating_add(1),
            }
        }
        sutur.push(format!(
            "  {tadeel} modified, {idafa} added, {mujallad} new director(ies), {} setting(s), \
             {} byte(s) of backups",
            self.idadat.len(),
            self.hajm_nusakh()
        ));
        for sijill in self.sijillat.values() {
            let alama = if sijill.istiada_tammat { "restored" } else { "in place" };
            sutur.push(format!("  {} [{}] {alama}", sijill.masar, sijill.naw.ism()));
        }
        for idad in self.idadat.values() {
            let alama = if idad.istiada_tammat { "restored" } else { "in place" };
            sutur.push(format!("  {} [{alama}]", idad.wasf()));
        }
        sutur
    }
}

impl DhuMukhattat for BayanTathbeet {
    const ISM: &'static str = "bayan_tathbeet";

    /// One. There has never been another shape of this file.
    const ISDAR: u32 = 1;

    fn hijra(min: u32, _qeema: Value) -> Natija<Value> {
        // Reached only for a manifest claiming a version below the first one,
        // which no build of Taarib ever wrote. Refusing is the only honest
        // answer: guessing at the shape of a manifest means guessing at which
        // backup holds which original, and a wrong guess writes one game file's
        // bytes over another's.
        Err(Khata::from(KhataTathbeet::IsdarBayanMajhul {
            masar: PathBuf::from(Self::ISM),
            mawjud: min,
            madum: Self::ISDAR,
        }))
    }
}

/// What an installation needs to know about a game and a patch before it starts.
///
/// Passed once to [`Tathbeet::ibda`] rather than as eight arguments, because
/// every one of these ends up in the manifest verbatim and a positional argument
/// list of eight strings is a list where two of them will eventually be swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarifLuba {
    /// The game's identity.
    pub luba: LubaId,
    /// The launcher it was found through.
    pub masdar: MasdarLuba,
    /// Its display name.
    pub ism: String,
    /// Where it is installed.
    pub jidhr: PathBuf,
    /// The patch's lineage.
    pub ruqaa: RuqaaId,
    /// The revision being installed.
    pub murajaa: RuqaaRevision,
    /// The build fingerprint the patch is being installed against.
    pub basma_bina: Option<Basma>,
}

/// An installation session: one game, one kind, one backup directory, one
/// manifest.
///
/// Held by whichever installer is running. It owns the manifest in memory and
/// rewrites it to disk atomically at every point where the on-disk record would
/// otherwise fall behind what has been done to the game.
#[derive(Debug)]
pub struct Tathbeet {
    jidhr_luba: PathBuf,
    jidhr_nusakh: PathBuf,
    mujallad_asl: PathBuf,
    masar_bayan: PathBuf,
    naw: NawTathbeet,
    bayan: BayanTathbeet,
}

impl Tathbeet {
    /// Opens a new session, creating the backup directory and an empty manifest.
    ///
    /// `jidhr_nusakh` is the game's directory under `nusakh/`, shared by both
    /// kinds; this call carves out the subdirectory and manifest belonging to
    /// `naw`, so a text install and a voice install of the same game live beside
    /// each other and never in each other.
    ///
    /// Refuses when a manifest for this kind is already there. Two installations
    /// of the same kind sharing one backup directory is the state in which
    /// uninstalling the second restores originals belonging to the first, and it
    /// is cheaper to refuse it here than to detect it afterwards.
    ///
    /// The empty manifest is written immediately rather than at the first
    /// preservation, so that a crash during the very first backup still leaves a
    /// directory that identifies itself as Taarib's, says which game and which
    /// patch it belongs to, and can be cleaned up by a later run.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::BayanMawjud`] when a manifest for this kind already
    /// exists, and [`KhataTathbeet::KhataMalaf`] or its siblings when the backup
    /// directory cannot be created or the empty manifest cannot be written — all
    /// of which mean nothing may be modified yet, which is the state this call
    /// exists to establish.
    pub fn ibda(
        jidhr_nusakh: &Path,
        naw: NawTathbeet,
        luba: &TarifLuba,
        huwiya: impl Into<String>,
    ) -> Result<Self, KhataTathbeet> {
        let masar_bayan = jidhr_nusakh.join(naw.ism_bayan());
        if masar_bayan.exists() {
            let huwiya_qadeema = mukhattat::iqra_malaf::<BayanTathbeet>(&masar_bayan)
                .map_or_else(|_| "an unreadable manifest".to_owned(), |sabiq| sabiq.huwiya);
            return Err(KhataTathbeet::BayanMawjud {
                masar: masar_bayan,
                huwiya: huwiya_qadeema,
            });
        }

        let mujallad_asl = jidhr_nusakh.join(naw.ism_mujallad()).join(ISM_MUJALLAD_ASL);
        insha_aw_khata(&mujallad_asl)?;

        let tathbeet = Self {
            jidhr_luba: luba.jidhr.clone(),
            jidhr_nusakh: jidhr_nusakh.to_path_buf(),
            mujallad_asl,
            masar_bayan,
            naw,
            bayan: BayanTathbeet::jadeed(huwiya, naw, luba),
        };
        tathbeet.iktub_bayan()?;
        Ok(tathbeet)
    }

    /// Reopens an existing session from its manifest.
    ///
    /// What an uninstall uses, and what a *resumed* uninstall uses: the manifest
    /// carries its own progress, so this is also how a half-finished restore is
    /// picked up. The game root is supplied rather than read from the manifest,
    /// so a game moved between installing and uninstalling is still reachable.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::BayanTalif`] when the manifest is absent, unreadable,
    /// internally inconsistent, or is the other kind's manifest under this
    /// kind's name, and [`KhataTathbeet::IsdarBayanMajhul`] when it was written
    /// by a newer build. Every one of those means the installation cannot be
    /// reliably undone, and saying so is better than undoing part of it.
    pub fn istanif(
        jidhr_luba: &Path,
        jidhr_nusakh: &Path,
        naw: NawTathbeet,
    ) -> Result<Self, KhataTathbeet> {
        let masar_bayan = jidhr_nusakh.join(naw.ism_bayan());
        let bayan: BayanTathbeet =
            mukhattat::iqra_malaf(&masar_bayan).map_err(|khata| KhataTathbeet::BayanTalif {
                masar: masar_bayan.clone(),
                sabab: khata.injilizi,
            })?;
        bayan.tahaqquq(&masar_bayan, naw)?;

        Ok(Self {
            jidhr_luba: jidhr_luba.to_path_buf(),
            jidhr_nusakh: jidhr_nusakh.to_path_buf(),
            mujallad_asl: jidhr_nusakh.join(naw.ism_mujallad()).join(ISM_MUJALLAD_ASL),
            masar_bayan,
            naw,
            bayan,
        })
    }

    /// Whether a manifest of this kind exists in a backup directory.
    ///
    /// Used by the library sweep to decide which of the two installations a game
    /// actually has, without opening either.
    #[must_use]
    pub fn mawjud(jidhr_nusakh: &Path, naw: NawTathbeet) -> bool {
        jidhr_nusakh.join(naw.ism_bayan()).is_file()
    }

    /// The game's root.
    #[must_use]
    pub fn jidhr_luba(&self) -> &Path {
        &self.jidhr_luba
    }

    /// The backup directory shared by both kinds for this game.
    #[must_use]
    pub fn jidhr_nusakh(&self) -> &Path {
        &self.jidhr_nusakh
    }

    /// The directory this kind's originals live in.
    #[must_use]
    pub fn mujallad_asl(&self) -> &Path {
        &self.mujallad_asl
    }

    /// The manifest's own path.
    #[must_use]
    pub fn masar_bayan(&self) -> &Path {
        &self.masar_bayan
    }

    /// Which installation this session is.
    #[must_use]
    pub const fn naw(&self) -> NawTathbeet {
        self.naw
    }

    /// The manifest as it currently stands.
    #[must_use]
    pub const fn bayan(&self) -> &BayanTathbeet {
        &self.bayan
    }

    /// Whether a path has already been preserved in this session.
    #[must_use]
    pub fn mahfuz(&self, nisbi: &str) -> bool {
        self.bayan.sijillat.contains_key(&masar_muwahhad(nisbi))
    }

    /// Preserves a file's original and returns the permission to modify it.
    ///
    /// This is the only route to a [`HarisTathbeet`], and a [`HarisTathbeet`] is
    /// the only thing in this crate that can write into a game directory. By the
    /// time this returns, the original's bytes, permissions and timestamps are in
    /// the backup directory and the manifest line naming them has been flushed to
    /// the device — in that order, and both before the caller is able to write
    /// anything.
    ///
    /// A path that does not exist yet is recorded as an addition rather than
    /// refused, and undoing it will delete rather than restore. That is the
    /// distinction the whole module turns on.
    ///
    /// Calling this twice for one path is safe and does not copy twice. That is
    /// not an optimization: the second copy would happen *after* the first write,
    /// so it would preserve the patched bytes as though they were the original
    /// and quietly destroy the only copy of the real one.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::HajmMufrit`] when the file declares more than
    /// [`AQSA_HAJM_NUSKHA`], checked before a byte is read.
    /// [`KhataTathbeet::DaghtFashil`] when the original cannot be compressed.
    /// [`KhataTathbeet::NuskhaTalifa`] when the backup that was just written does
    /// not read back and expand to the bytes that went into it.
    /// [`KhataTathbeet::MalafMaqful`], [`KhataTathbeet::SalahiyaMarfuda`] or
    /// [`KhataTathbeet::KhataMalaf`] when the original cannot be read or the
    /// backup cannot be written, and [`KhataTathbeet::MasarKharij`] for a path
    /// that escapes the game's root. In every case nothing has been modified,
    /// which is the point.
    pub fn ihfaz(&mut self, nisbi: &str) -> Result<HarisTathbeet<'_>, KhataTathbeet> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        // The kind is copied out before anything else happens so that the
        // manifest's borrow ends here rather than spanning the mutable borrow
        // the guard needs.
        if let Some(naw) = self.bayan.sijillat.get(&masar).map(|mawjud| mawjud.naw) {
            if matches!(naw, NawTaghyeer::MujalladMudaf) {
                return Err(KhataTathbeet::BayanTalif {
                    masar: self.masar_bayan.clone(),
                    sabab: format!(
                        "{masar} is recorded as a directory Taarib created, and a directory \
                         cannot be rewritten as a file"
                    ),
                });
            }
            return Ok(HarisTathbeet { tathbeet: self, miftah: masar });
        }

        let Ok(bayanat) = fs::metadata(&mutlaq) else {
            // Nothing there: this is an addition, and an addition has no
            // original. Recording it as one would mean writing an empty file
            // over the game on uninstall.
            return self.sajjil_wa_ihris(masar, SijillTaghyeer {
                masar: String::new(),
                naw: NawTaghyeer::Idafa,
                kan_mawjudan: false,
                hajm_asli: None,
                basma_asliya: None,
                salahiyat_asliya: None,
                awqat_asliya: None,
                nuskha: None,
                hajm_nuskha: None,
                basma_maktuba: None,
                hajm_maktub: None,
                waqt_maktub: None,
                istiada_tammat: false,
            });
        };

        if !bayanat.is_file() {
            return Err(KhataTathbeet::BayanTalif {
                masar: self.masar_bayan.clone(),
                sabab: format!(
                    "a directory or a special file is at {}, and only a regular file can be \
                     preserved and put back",
                    mutlaq.display()
                ),
            });
        }

        let hajm = bayanat.len();
        if hajm > AQSA_HAJM_NUSKHA {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "original file",
                qeema: hajm,
                saqf: AQSA_HAJM_NUSKHA,
            });
        }

        // Permissions and times are read from the same `Metadata` the size came
        // from, before anything is read or written, so all four facts describe
        // one instant. Reading them again after the copy would record the access
        // time the copy itself caused.
        let salahiyat = SalahiyatMalaf::min_bayanat(&bayanat);
        let Some(awqat) = AwqatMalaf::min_bayanat(&bayanat) else {
            return Err(KhataTathbeet::BayanTalif {
                masar: self.masar_bayan.clone(),
                sabab: format!(
                    "{} reports no modification time, so a restore could not put its date \
                     back and the file would come back looking rewritten to the launcher",
                    mutlaq.display()
                ),
            });
        };

        let asli = fs::read(&mutlaq)
            .map_err(|sabab| min_khata_io(&mutlaq, "reading the original to preserve it", sabab))?;
        let basma_asliya = basma_bayt(&asli);

        let miftah = miftah_nuskha(&masar);
        let hajm_nuskha = self.iktub_nuskha(&miftah, &mutlaq, &asli, basma_asliya)?;

        self.sajjil_wa_ihris(masar, SijillTaghyeer {
            masar: String::new(),
            naw: NawTaghyeer::Tadeel,
            kan_mawjudan: true,
            hajm_asli: Some(hajm),
            basma_asliya: Some(basma_asliya),
            salahiyat_asliya: Some(salahiyat),
            awqat_asliya: Some(awqat),
            nuskha: Some(miftah),
            hajm_nuskha: Some(hajm_nuskha),
            basma_maktuba: None,
            hajm_maktub: None,
            waqt_maktub: None,
            istiada_tammat: false,
        })
    }

    /// Records an addition and returns the permission to write it.
    ///
    /// The preferred route wherever an engine's delivery allows it, for two
    /// reasons. Uninstalling is a delete, so there is no original to hold and no
    /// fingerprint that has to be reproduced. And a game update rewrites the
    /// files the game ships while leaving files it has never heard of alone, so
    /// an additive installation commonly survives an update that would have
    /// reverted a modification without telling anyone.
    ///
    /// Refuses when something is already at the path. That is not pedantry: a
    /// file recorded as an addition is *deleted* on uninstall, so recording an
    /// existing file as one would arm Taarib to delete something it did not
    /// create. An installer that genuinely means to rewrite an existing file
    /// calls [`Tathbeet::ihfaz`], which preserves it first.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::BayanTalif`] when something already exists at the path or
    /// the path is already recorded as a modification, and
    /// [`KhataTathbeet::MasarKharij`] when it escapes the game's root.
    pub fn adif(&mut self, nisbi: &str) -> Result<HarisTathbeet<'_>, KhataTathbeet> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        if let Some(naw) = self.bayan.sijillat.get(&masar).map(|mawjud| mawjud.naw) {
            if matches!(naw, NawTaghyeer::Idafa) {
                return Ok(HarisTathbeet { tathbeet: self, miftah: masar });
            }
            return Err(KhataTathbeet::BayanTalif {
                masar: self.masar_bayan.clone(),
                sabab: format!(
                    "{masar} is already recorded as {}, and it cannot also be an addition",
                    naw.ism()
                ),
            });
        }

        if mutlaq.exists() {
            return Err(KhataTathbeet::BayanTalif {
                masar: self.masar_bayan.clone(),
                sabab: format!(
                    "something is already at {}, so it is not Taarib's to add. Uninstalling \
                     deletes additions, and deleting a file the game shipped is not an \
                     uninstall. Preserve it instead.",
                    mutlaq.display()
                ),
            });
        }

        self.sajjil_wa_ihris(masar, SijillTaghyeer {
            masar: String::new(),
            naw: NawTaghyeer::Idafa,
            kan_mawjudan: false,
            hajm_asli: None,
            basma_asliya: None,
            salahiyat_asliya: None,
            awqat_asliya: None,
            nuskha: None,
            hajm_nuskha: None,
            basma_maktuba: None,
            hajm_maktub: None,
            waqt_maktub: None,
            istiada_tammat: false,
        })
    }

    /// Creates a directory for additions and records it so uninstall can remove
    /// it.
    ///
    /// Returns whether the directory was Taarib's to create. A directory that
    /// already existed is the game's and is not recorded, because removing it on
    /// uninstall would remove a directory the game shipped.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] when the path escapes the game's root, and
    /// the I/O variants when the directory cannot be created or the manifest
    /// cannot be flushed.
    pub fn sajjil_mujallad(&mut self, nisbi: &str) -> Result<bool, KhataTathbeet> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        if self.bayan.sijillat.contains_key(&masar) || mutlaq.exists() {
            return Ok(false);
        }

        insha_aw_khata(&mutlaq)?;
        let _ = self.bayan.sijillat.insert(masar.clone(), SijillTaghyeer {
            masar,
            naw: NawTaghyeer::MujalladMudaf,
            kan_mawjudan: false,
            hajm_asli: None,
            basma_asliya: None,
            salahiyat_asliya: None,
            awqat_asliya: None,
            nuskha: None,
            hajm_nuskha: None,
            basma_maktuba: None,
            hajm_maktub: None,
            waqt_maktub: None,
            istiada_tammat: false,
        });
        self.iktub_bayan()?;
        Ok(true)
    }

    /// Records a setting's previous value, and flushes, *before* it is changed.
    ///
    /// The setting equivalent of [`Tathbeet::ihfaz`], and ordered the same way
    /// for the same reason: a launch options field rewritten before the record
    /// of what it used to say reaches the disk is a field nobody can put back.
    /// There is no guard here because there is nothing for a guard to
    /// authorize — this crate does not own the machinery that edits a launcher's
    /// configuration, and inventing a token it would have to be handed to would
    /// be theatre. What it does own is the record, and the record is durable
    /// before the caller is told it may proceed.
    ///
    /// Calling this twice for one setting keeps the *first* previous value and
    /// updates only what Taarib wrote, for exactly the reason preserving twice
    /// is refused: the second "previous" value would be Taarib's own.
    ///
    /// # Errors
    ///
    /// Whatever writing the manifest raises. The caller must not change the
    /// setting when this fails.
    pub fn sajjil_idad(
        &mut self,
        mahall: MahallIdad,
        qeema_sabiqa: Option<String>,
        qeema_maktuba: Option<String>,
    ) -> Result<(), KhataTathbeet> {
        self.sajjil_idad_dakhili(mahall, qeema_sabiqa, qeema_maktuba)
    }

    /// The one body behind both the inherent method and the [`Muthabbit`] one.
    ///
    /// A private worker rather than one calling the other, so that neither name
    /// resolution nor a future reader has to work out which of two identically
    /// named functions is in play.
    fn sajjil_idad_dakhili(
        &mut self,
        mahall: MahallIdad,
        qeema_sabiqa: Option<String>,
        qeema_maktuba: Option<String>,
    ) -> Result<(), KhataTathbeet> {
        let muarrif = mahall.wasf();
        match self.bayan.idadat.get_mut(&muarrif) {
            Some(mawjud) => {
                mawjud.qeema_maktuba = qeema_maktuba;
                mawjud.istiada_tammat = false;
            }
            None => {
                let _ = self.bayan.idadat.insert(muarrif.clone(), SijillIdad {
                    muarrif,
                    mahall,
                    qeema_sabiqa,
                    qeema_maktuba,
                    istiada_tammat: false,
                });
            }
        }
        self.iktub_bayan()
    }

    /// Compresses an original into the backup directory and proves it comes back.
    ///
    /// The round trip is not optional. A backup written to a failing disk, a full
    /// volume or a filesystem that silently truncated it is worth nothing, and
    /// the only moment it can be caught cheaply is now — while the real original
    /// is still on disk and untouched. Expanding it and re-hashing also exercises
    /// the compressor, which is the component whose output nothing else in the
    /// product ever checks.
    fn iktub_nuskha(
        &self,
        miftah: &str,
        asl: &Path,
        bayt: &[u8],
        basma: Basma,
    ) -> Result<u64, KhataTathbeet> {
        let masar_nuskha = self.mujallad_asl.join(miftah);

        let madghut = zstd::encode_all(bayt, MUSTAWA_DAGHT).map_err(|sabab| {
            KhataTathbeet::DaghtFashil {
                masar: masar_nuskha.clone(),
                ittijah: IttijahDaght::Daght,
                tafsil: sabab.to_string(),
            }
        })?;

        masarat::kitaba_dharra(&masar_nuskha, &madghut).map_err(|khata| {
            KhataTathbeet::KhataMalaf {
                masar: masar_nuskha.clone(),
                amal: "writing the compressed backup",
                sabab: std::io::Error::other(khata.injilizi),
            }
        })?;

        let murtajaa = self.iqra_nuskha(miftah, asl)?;
        let mahsuba = basma_bayt(&murtajaa);
        if mahsuba != basma {
            return Err(KhataTathbeet::NuskhaTalifa {
                masar: asl.to_path_buf(),
                miftah: miftah.to_owned(),
                muallana: basma.to_string(),
                mahsuba: mahsuba.to_string(),
            });
        }
        Ok(tul_u64(madghut.len()))
    }

    /// Reads a backup and expands it.
    ///
    /// `asl` is the game file the backup belongs to, carried through only so a
    /// failure names the file the user cares about rather than the opaque key it
    /// is stored under.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::NuskhaMafquda`] when the backup is not there, the I/O
    /// variants when it cannot be read, and [`KhataTathbeet::DaghtFashil`] when
    /// it is not a zstd frame any more.
    pub fn iqra_nuskha(&self, miftah: &str, asl: &Path) -> Result<Vec<u8>, KhataTathbeet> {
        let masar_nuskha = self.mujallad_asl.join(miftah);
        if !masar_nuskha.is_file() {
            return Err(KhataTathbeet::NuskhaMafquda {
                masar: asl.to_path_buf(),
                miftah: miftah.to_owned(),
                jidhr_nusakh: self.mujallad_asl.clone(),
            });
        }
        let madghut = fs::read(&masar_nuskha)
            .map_err(|sabab| min_khata_io(&masar_nuskha, "reading the backup", sabab))?;
        zstd::decode_all(madghut.as_slice()).map_err(|sabab| KhataTathbeet::DaghtFashil {
            masar: masar_nuskha,
            ittijah: IttijahDaght::Fakk,
            tafsil: sabab.to_string(),
        })
    }

    /// Files a manifest line, flushes the manifest, and mints the guard.
    ///
    /// The flush happens here, before the guard exists, which is what makes "the
    /// record is durable before the file is modified" a property of the type
    /// system rather than a rule twelve callers have to follow.
    fn sajjil_wa_ihris(
        &mut self,
        masar: String,
        mut sijill: SijillTaghyeer,
    ) -> Result<HarisTathbeet<'_>, KhataTathbeet> {
        sijill.masar.clone_from(&masar);
        sijill.tahaqquq(&self.masar_bayan)?;
        let _ = self.bayan.sijillat.insert(masar.clone(), sijill);
        self.iktub_bayan()?;
        Ok(HarisTathbeet { tathbeet: self, miftah: masar })
    }

    /// Writes the manifest atomically, through the shared mechanism.
    fn iktub_bayan(&self) -> Result<(), KhataTathbeet> {
        mukhattat::iktub_malaf(&self.masar_bayan, &self.bayan).map_err(|khata| {
            KhataTathbeet::KhataMalaf {
                masar: self.masar_bayan.clone(),
                amal: "flushing the installation manifest",
                sabab: std::io::Error::other(khata.injilizi),
            }
        })
    }

    /// Marks a file's line as undone and flushes, so a later run resumes past it.
    ///
    /// Visible to the crate rather than public: `taraju` owns the restore, and
    /// nothing outside this crate has any business declaring a path restored.
    pub(crate) fn allim_tammat(&mut self, masar: &str) -> Result<(), KhataTathbeet> {
        if let Some(sijill) = self.bayan.sijillat.get_mut(masar) {
            sijill.istiada_tammat = true;
        }
        self.iktub_bayan()
    }

    /// Marks a setting's line as undone and flushes.
    pub(crate) fn allim_idad_tammat(&mut self, muarrif: &str) -> Result<(), KhataTathbeet> {
        if let Some(idad) = self.bayan.idadat.get_mut(muarrif) {
            idad.istiada_tammat = true;
        }
        self.iktub_bayan()
    }

    /// An absolute path as the manifest names it, or a refusal saying why not.
    ///
    /// Every [`Muthabbit`] method starts here, so a path outside the game root,
    /// or one that cannot be written as UTF-8, is rejected once rather than in
    /// five places that could drift apart.
    fn nisbi_aw_khata(&self, masar: &Path) -> Result<String, KhataTathbeet> {
        nisbi_min(&self.jidhr_luba, masar).ok_or_else(|| KhataTathbeet::MasarKharij {
            masar: masar.to_path_buf(),
            jidhr: self.jidhr_luba.clone(),
            sabab: "it is not inside the game directory, or cannot be written as UTF-8, and \
                    an installation writes nowhere else"
                .to_owned(),
        })
    }

    /// Records every ancestor of a directory that Taarib has to create, from the
    /// outermost inwards.
    ///
    /// [`Tathbeet::sajjil_mujallad`] creates the whole chain but records only the
    /// path it was handed, which would leave `BepInEx/` behind after an uninstall
    /// that removed `BepInEx/plugins/`. Walking outermost-first is what makes the
    /// difference: `sajjil_mujallad` returns `false` for a directory that was
    /// already there, so exactly the ones this installation brought into
    /// existence end up in the manifest, and the ones the game shipped do not.
    fn sajjil_silsilat_mujalladat(&mut self, nisbi: &str) -> Result<(), KhataTathbeet> {
        let mut mutarakim = String::with_capacity(nisbi.len());
        for juz in nisbi.split('/').filter(|juz| !juz.is_empty()) {
            if !mutarakim.is_empty() {
                mutarakim.push('/');
            }
            mutarakim.push_str(juz);
            let _ = self.sajjil_mujallad(&mutarakim)?;
        }
        Ok(())
    }
}

/// Permission to write one file of a game, proving its original is preserved.
///
/// There is no public constructor and no public field. The only way to obtain one
/// is [`Tathbeet::ihfaz`] or [`Tathbeet::adif`] returning `Ok`, which happens only
/// after the original has been compressed into the backup directory, read back,
/// re-hashed, and the manifest line naming it flushed to the device.
/// [`Tathbeet`] has no method that takes a path and some bytes, so an installer
/// cannot reach a game file without one of these in its hand.
///
/// ## Why it holds a key and not a path
///
/// This is the difference between this guard and `hifz`'s, and it is the whole
/// of the "no installation ever touches a file outside the manifest" guarantee.
///
/// A guard carrying both a manifest key and an absolute path has two sources of
/// truth for the same fact. They agree the day it is written and stop agreeing
/// the first time someone adds a constructor, reorders a struct literal, or
/// introduces a path-rewriting step between preservation and writing — and when
/// they disagree, the write lands on a file that the manifest describes under a
/// different name. That file is, by definition, outside the manifest: nothing
/// records its original, nothing will restore it, and the uninstall will
/// cheerfully write some *other* file's original over it.
///
/// So this guard stores the key alone. The absolute path is *derived*, at the
/// moment of use, by joining the session's game root onto the key through the
/// workspace's one path-safety check ([`taarib_usus::masarat::dakhil`]). There
/// is no field to disagree with the record because there is no field. A caller
/// cannot hand the guard a different path — the parameter does not exist. A
/// caller cannot construct a guard for an unrecorded path — the constructor does
/// not exist, and the two functions that mint guards both file a record and
/// flush it first. Code that tries to write outside the manifest does not fail a
/// review or trip an assertion; it fails to compile, because the function it
/// would need is not there to call.
///
/// The guard is consumed by writing, so one preservation authorizes one write.
/// Dropping it without writing is harmless — the manifest then describes a file
/// that was preserved and left alone, and restoring it writes an identical
/// original back over itself and verifies.
#[derive(Debug)]
#[must_use = "obtaining a guard preserves the original; dropping it without writing \
              leaves a manifest line for a file that was never modified"]
pub struct HarisTathbeet<'a> {
    tathbeet: &'a mut Tathbeet,
    miftah: String,
}

impl HarisTathbeet<'_> {
    /// The path this guard authorizes, relative to the game's root.
    #[must_use]
    pub fn masar(&self) -> &str {
        &self.miftah
    }

    /// Whether undoing this write will restore an original or delete a file.
    ///
    /// Read out of the manifest rather than cached on the guard, for the same
    /// reason the path is: one fact, one place.
    #[must_use]
    pub fn naw(&self) -> Option<NawTaghyeer> {
        self.tathbeet.bayan.sijillat.get(&self.miftah).map(|sijill| sijill.naw)
    }

    /// The absolute path this guard authorizes, derived from the manifest key.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] when the key does not join onto the game
    /// root as a contained relative path. Unreachable through the two functions
    /// that mint guards, since both validate the key before filing it — and
    /// still an error rather than an assertion, because the alternative to
    /// returning it is aborting the process inside somebody's game directory.
    pub fn masar_kamil(&self) -> Result<PathBuf, KhataTathbeet> {
        dakhil_aw_khata(&self.tathbeet.jidhr_luba, &self.miftah)
    }

    /// Writes the file, atomically, and records what was written.
    ///
    /// Beside, fsync, rename, through
    /// [`taarib_usus::masarat::kitaba_dharra`] — so a machine that loses power
    /// mid-write comes back with either the whole old file or the whole new one.
    /// A `GameAssembly.dll` truncated at two thirds is not a damaged patch, it is
    /// a game that will not start.
    ///
    /// A read-only target is unlocked first, because Windows refuses to rename
    /// over one. The *recorded* attribute is what a restore puts back, so
    /// clearing it here cannot leak into the restored state.
    ///
    /// The fingerprint, size and modification time of what was written are added
    /// to the manifest afterwards, read back from the filesystem rather than
    /// taken from the buffer — the size a later `stat` returns is the number the
    /// fast path in `tahaqquq` will compare against, and computing it any other
    /// way would make that comparison disagree with the disk on a filesystem that
    /// pads or that reports a different granularity for times. That second flush
    /// is ordered after the write on purpose: it carries only drift-detection
    /// data, and the line that makes the change reversible was already durable
    /// before this function was reachable.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MalafMaqful`], [`KhataTathbeet::SalahiyaMarfuda`] or
    /// [`KhataTathbeet::KhataMalaf`] when the atomic write cannot complete, and
    /// the same when the manifest cannot be updated with what was written —
    /// which leaves the file modified and still restorable, since its original,
    /// its backup key, its permissions and its times were recorded before this
    /// call was reachable.
    pub fn iktub(self, bayt: &[u8]) -> Result<Basma, KhataTathbeet> {
        // Re-derived with links resolved, not reused from the check that
        // admitted the path. See `mutlaq_lil_kitaba` for the window this
        // closes.
        let mutlaq = mutlaq_lil_kitaba(&self.tathbeet.jidhr_luba, &self.miftah)?;
        let _ = SalahiyatMalaf::ataih_kitaba(&mutlaq)?;

        masarat::kitaba_dharra(&mutlaq, bayt).map_err(|khata| KhataTathbeet::KhataMalaf {
            masar: mutlaq.clone(),
            amal: "writing the patched file",
            sabab: std::io::Error::other(khata.injilizi),
        })?;

        let basma = basma_bayt(bayt);
        let bayanat = fs::metadata(&mutlaq).ok();
        let hajm = bayanat.as_ref().map_or_else(|| tul_u64(bayt.len()), fs::Metadata::len);
        let waqt = bayanat
            .as_ref()
            .and_then(|b| b.modified().ok())
            .and_then(WaqtNizam::min_nizam);

        if let Some(sijill) = self.tathbeet.bayan.sijillat.get_mut(&self.miftah) {
            sijill.basma_maktuba = Some(basma);
            sijill.hajm_maktub = Some(hajm);
            sijill.waqt_maktub = waqt;
        }
        self.tathbeet.iktub_bayan()?;
        Ok(basma)
    }

    /// Writes UTF-8 text, with no byte order mark.
    ///
    /// # Errors
    ///
    /// As [`HarisTathbeet::iktub`].
    pub fn iktub_nass(self, nass: &str) -> Result<Basma, KhataTathbeet> {
        self.iktub(nass.as_bytes())
    }
}

/// The only way anything in Taarib is permitted to change a game.
///
/// Every installer — the framework installers per engine and platform, the
/// engine adapters, the additive payload writers — takes a `&mut dyn Muthabbit`
/// and calls one of these five methods. None of them owns a file handle, which
/// is what makes "every change is recorded before it happens" a property of the
/// call graph rather than a convention a dozen authors have to remember.
/// [`Tathbeet`] is the implementation the product uses; the trait exists so an
/// installer can be exercised against a recorder that writes nowhere, and so a
/// reviewer grepping for `fs::File::create` outside this module finds nothing.
///
/// Absolute paths, because every caller already holds one — the game root joined
/// with an engine-specific layout. Converting to the manifest's relative form is
/// this trait's job, and doing it in one place is why a path outside the game
/// root is a single refusal instead of five separate opportunities to write into
/// somebody's home directory.
///
/// The fifth method is the one `hifz`'s `Hafiz` has no equivalent of, because a
/// script-engine patcher only ever writes files. An installation also changes
/// things that are not files, and a setting Taarib changed and cannot change back
/// is exactly as much of a broken promise as a file it cannot restore.
pub trait Muthabbit {
    /// Replaces a file that already exists, preserving its original bytes,
    /// fingerprint, permissions and timestamps first, and writing the
    /// replacement atomically.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the original cannot be preserved or the
    /// replacement cannot be written. An implementation must refuse rather than
    /// proceed: the original is the only copy of the game the player owns.
    fn iktub(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataTathbeet>;

    /// Creates a file the installation adds, recording it so uninstalling
    /// removes it.
    ///
    /// # Errors
    ///
    /// As [`Muthabbit::iktub`], plus whatever the guard raises when a file it was
    /// asked to create is already there and was not created by this installation.
    fn ansha(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataTathbeet>;

    /// Creates a directory the installation needs, recording it when it is
    /// Taarib's.
    ///
    /// `BepInEx/plugins/`, `game/tl/arabic/`, a Proton prefix's `drive_c` subtree
    /// — all directories an installation may have to make. A directory that
    /// already existed is the game's and is *not* recorded, so an uninstall does
    /// not remove it.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the path escapes the game root or the
    /// directory cannot be created.
    fn ansha_mujallad(&mut self, masar: &Path) -> Result<(), KhataTathbeet>;

    /// Deletes a file **this installation added**, and nothing else.
    ///
    /// The asymmetry with [`Muthabbit::ansha`] is the point. An uninstall that
    /// could delete an arbitrary path would be one manifest bug away from
    /// deleting a file the game shipped, so the implementation is required to
    /// refuse any path not recorded as a [`NawTaghyeer::Idafa`]. Deleting a path
    /// that was never added is not an error — uninstalling twice succeeds and the
    /// second run does nothing, which is what an uninstall after a failed install
    /// has to do. The return says which of the two happened.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the path is recorded as a modification
    /// rather than an addition, when it is not recorded at all and something is
    /// there, or when the file cannot be removed.
    fn ihdhif(&mut self, masar: &Path) -> Result<bool, KhataTathbeet>;

    /// Records a launcher setting or registry value about to be changed, with
    /// its previous value.
    ///
    /// The method `hifz` had no need of. Call this **before** writing the new
    /// value, never after: it flushes the record of what was there, and a value
    /// overwritten before that record reaches the device is a value nobody can
    /// put back. Pass `qeema_sabiqa: None` when nothing was set, because a
    /// setting that did not exist is *removed* on uninstall rather than rewritten
    /// as an empty string — the two are indistinguishable in a launch-options
    /// text box and are entirely different things for a registry value or an
    /// environment variable.
    ///
    /// # Errors
    ///
    /// Whatever writing the manifest raises. A caller that receives an error must
    /// not go on to change the setting.
    fn sajjil_idad(
        &mut self,
        mahall: MahallIdad,
        qeema_sabiqa: Option<String>,
        qeema_maktuba: Option<String>,
    ) -> Result<(), KhataTathbeet>;
}

impl Muthabbit for Tathbeet {
    fn iktub(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataTathbeet> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let _ = self.ihfaz(&nisbi)?.iktub(bayt)?;
        Ok(())
    }

    fn ansha(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataTathbeet> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let mifta = masar_muwahhad(&nisbi);
        if let Some((walid, _)) = mifta.rsplit_once('/') {
            self.sajjil_silsilat_mujalladat(walid)?;
        }
        let _ = self.adif(&nisbi)?.iktub(bayt)?;
        Ok(())
    }

    fn ansha_mujallad(&mut self, masar: &Path) -> Result<(), KhataTathbeet> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        self.sajjil_silsilat_mujalladat(&masar_muwahhad(&nisbi))
    }

    fn ihdhif(&mut self, masar: &Path) -> Result<bool, KhataTathbeet> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let mifta = masar_muwahhad(&nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &mifta)?;

        let Some(naw) = self.bayan.sijillat.get(&mifta).map(|sijill| sijill.naw) else {
            if mutlaq.exists() {
                return Err(KhataTathbeet::SijillMafqud {
                    masar: mutlaq,
                    bayan: self.masar_bayan.clone(),
                });
            }
            return Ok(false);
        };

        if !matches!(naw, NawTaghyeer::Idafa) {
            return Err(KhataTathbeet::BayanTalif {
                masar: self.masar_bayan.clone(),
                sabab: format!(
                    "{mifta} is recorded as {}, and undoing a modification means writing its \
                     original back, not deleting it",
                    naw.ism()
                ),
            });
        }

        let kan = mutlaq.exists();
        if kan {
            // Cleared first for the same reason a write clears it: Windows
            // refuses to unlink a read-only file, and a patch that added one is
            // a patch whose uninstall would otherwise be stuck on its own file.
            let _ = SalahiyatMalaf::ataih_kitaba(&mutlaq)?;
            fs::remove_file(&mutlaq)
                .map_err(|sabab| min_khata_io(&mutlaq, "deleting a file Taarib added", sabab))?;
        }
        self.allim_tammat(&mifta)?;
        Ok(kan)
    }

    fn sajjil_idad(
        &mut self,
        mahall: MahallIdad,
        qeema_sabiqa: Option<String>,
        qeema_maktuba: Option<String>,
    ) -> Result<(), KhataTathbeet> {
        self.sajjil_idad_dakhili(mahall, qeema_sabiqa, qeema_maktuba)
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// The BLAKE3 fingerprint of a buffer.
///
/// The same function the manifest, the restore check and the drift check all
/// use. One definition, so a fingerprint recorded at install and a fingerprint
/// computed at uninstall cannot be computed two different ways.
#[must_use]
pub fn basma_bayt(bayt: &[u8]) -> Basma {
    Basma::min_bayt(*blake3::hash(bayt).as_bytes())
}

/// The BLAKE3 fingerprint of a file on disk, without reading it into the heap.
///
/// Mapped rather than read, because verification runs over every file an
/// installation touched and a multi-gigabyte asset archive is not something to
/// load in order to hash. The mapping is read-only and lives for the length of
/// this call.
///
/// # Errors
///
/// [`KhataTathbeet::MalafMaqful`], [`KhataTathbeet::SalahiyaMarfuda`] or
/// [`KhataTathbeet::KhataMalaf`] naming the path when it cannot be opened or
/// mapped, which during a verification pass usually means the game was moved or
/// uninstalled underneath the patch.
pub fn basma_malaf(masar: &Path) -> Result<Basma, KhataTathbeet> {
    let mut hashib = blake3::Hasher::new();
    let _ = hashib
        .update_mmap(masar)
        .map_err(|sabab| min_khata_io(masar, "hashing a game file", sabab))?;
    Ok(Basma::min_bayt(*hashib.finalize().as_bytes()))
}

/// Normalizes a caller's relative path to the one form the manifest stores.
///
/// Backslashes become forward slashes, leading and trailing separators go, and
/// repeated separators collapse. Without this, `BepInEx/plugins/Taarib.dll` and
/// `BepInEx\plugins\Taarib.dll` are two manifest keys for one file — which means
/// two backups, the second one taken after the first write, holding the patched
/// bytes as though they were the original.
fn masar_muwahhad(nisbi: &str) -> String {
    let mubaddal = nisbi.replace('\\', "/");
    let maqsus = mubaddal.trim_matches('/');
    let mut nateeja = String::with_capacity(maqsus.len());
    let mut sabiq_fasil = false;
    for harf in maqsus.chars() {
        if harf == '/' {
            if sabiq_fasil {
                continue;
            }
            sabiq_fasil = true;
        } else {
            sabiq_fasil = false;
        }
        nateeja.push(harf);
    }
    nateeja
}

/// Joins a relative path onto a root through the workspace's only path-safety
/// check.
///
/// Shared by the guard, the manifest and the restore, so that "inside the game
/// directory" means one thing in this crate and is decided by one function.
pub(crate) fn dakhil_aw_khata(jidhr: &Path, nisbi: &str) -> Result<PathBuf, KhataTathbeet> {
    masarat::dakhil(jidhr, nisbi).map_err(|khata| KhataTathbeet::MasarKharij {
        masar: PathBuf::from(nisbi),
        jidhr: jidhr.to_path_buf(),
        sabab: khata.injilizi,
    })
}

/// Resolves a write destination with symbolic links followed, at write time.
///
/// [`dakhil_aw_khata`] is lexical and cannot see a directory inside the root
/// that is itself a link out of it. This creates and canonicalizes the parent,
/// proves it stays inside the canonical root, and returns a path built on the
/// real directory, so a link planted after the check cannot redirect the write.
/// The final component is left to the atomic rename, which replaces a symlink
/// rather than following it.
///
/// # Errors
///
/// [`KhataTathbeet::MasarKharij`] when the lexical check fails, the parent
/// canonicalizes to outside the root, or the path has no parent.
pub(crate) fn mutlaq_lil_kitaba(jidhr: &Path, nisbi: &str) -> Result<PathBuf, KhataTathbeet> {
    let mutlaq = dakhil_aw_khata(jidhr, nisbi)?;
    let (Some(walid), Some(ism)) = (mutlaq.parent(), mutlaq.file_name()) else {
        return Err(KhataTathbeet::MasarKharij {
            masar: mutlaq,
            jidhr: jidhr.to_path_buf(),
            sabab: "the destination has no parent directory inside the root".to_owned(),
        });
    };
    let (walid, ism) = (walid.to_path_buf(), ism.to_owned());
    insha_aw_khata(&walid)?;

    let haqiqi = masarat::tahaqquq_ihtiwa(jidhr, &walid).map_err(|khata| {
        KhataTathbeet::MasarKharij {
            masar: walid,
            jidhr: jidhr.to_path_buf(),
            sabab: khata.injilizi,
        }
    })?;
    Ok(haqiqi.join(ism))
}

/// Creates a directory, naming the real reason when it cannot be created.
fn insha_aw_khata(masar: &Path) -> Result<(), KhataTathbeet> {
    masarat::insha_mujallad(masar).map_err(|khata| KhataTathbeet::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: "creating a directory",
        sabab: std::io::Error::other(khata.injilizi),
    })
}

/// An absolute path expressed the way the manifest stores it, or [`None`] when
/// it is not inside the root or cannot be written as UTF-8.
pub(crate) fn nisbi_min(jidhr: &Path, masar: &Path) -> Option<String> {
    let baqi = masar.strip_prefix(jidhr).ok()?;
    let mut nateeja = String::new();
    for juz in baqi.components() {
        let std::path::Component::Normal(ism) = juz else { return None };
        let nass = ism.to_str()?;
        if !nateeja.is_empty() {
            nateeja.push('/');
        }
        nateeja.push_str(nass);
    }
    Some(nateeja)
}

/// The file name a preserved original is stored under.
///
/// Three parts, each load-bearing. The leading `n` guarantees the name's first
/// dot-separated component is never a Windows device name, so a game file
/// honestly called `con.json` does not turn into a handle on a serial port. The
/// middle is the file's own name with anything outside `[A-Za-z0-9._-]` replaced,
/// so a person browsing `nusakh/` can see what they are looking at. The suffix is
/// eight bytes of BLAKE3 over the *whole relative path*, which is what keeps
/// `Data/resources.assets` and `Plugins/resources.assets` in separate backups
/// instead of one overwriting the other.
fn miftah_nuskha(masar: &str) -> String {
    let ism = masar.rsplit('/').next().unwrap_or(masar);
    let mut nazif = String::with_capacity(HADD_ISM_MIFTAH);
    for harf in ism.chars().take(HADD_ISM_MIFTAH) {
        if harf.is_ascii_alphanumeric() || harf == '.' || harf == '-' || harf == '_' {
            nazif.push(harf);
        } else {
            nazif.push('_');
        }
    }
    while nazif.ends_with('.') {
        let _ = nazif.pop();
    }

    let basma = blake3::hash(masar.as_bytes());
    let mut lahiqa = String::with_capacity(16);
    for qeema in basma.as_bytes().iter().take(8) {
        let _ = write!(lahiqa, "{qeema:02x}");
    }
    format!("n{nazif}.{lahiqa}.zst")
}

/// The current time as RFC 3339 in UTC.
///
/// Written here rather than taken from a date library because this crate does
/// not depend on one and a single UTC conversion does not justify adding one to
/// a crate that installs files. The civil-date arithmetic is the standard
/// shifted-era algorithm: the year is re-based to start in March so that the leap
/// day lands at the end of it and no month-length table is needed.
///
/// A clock that reports a time before the epoch — a machine with a dead CMOS
/// battery, a container with no clock source — yields the epoch itself rather
/// than a negative date, because the only consumer of this field compares it
/// against file modification times to decide whether something changed *after*
/// the install, and the epoch is the answer that makes every later change look
/// later, which is the safe direction.
#[must_use]
pub fn waqt_alaan() -> String {
    let thawani = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|mudda| i64::try_from(mudda.as_secs()).ok())
        .unwrap_or(0);
    waqt_rfc3339(thawani)
}

/// Renders Unix seconds as RFC 3339 in UTC.
#[expect(
    clippy::integer_division,
    reason = "every division here is exact calendar arithmetic on a value already \
              range-reduced by the line above it"
)]
#[must_use]
pub fn waqt_rfc3339(thawani: i64) -> String {
    // Clamped to a range the era arithmetic below cannot overflow inside. The
    // bound is roughly year 292277026596, which no clock this product meets will
    // report and which is still cheaper to exclude than to reason about: an
    // arithmetic overflow here aborts a debug build in the middle of an install.
    let thawani = thawani.clamp(-62_167_219_200, 253_402_300_799);
    let ayyam = thawani.div_euclid(THAWANI_YAWM);
    let fi_al_yawm = thawani.rem_euclid(THAWANI_YAWM);
    let saa = fi_al_yawm / 3_600;
    let daqiqa = (fi_al_yawm % 3_600) / 60;
    let thaniya = fi_al_yawm % 60;

    let muzah = ayyam.saturating_add(719_468);
    let mabda = if muzah >= 0 { muzah } else { muzah - 146_096 };
    let ahd = mabda / 146_097;
    let fi_al_ahd = muzah - ahd * 146_097;
    let sana_fi_al_ahd =
        (fi_al_ahd - fi_al_ahd / 1_460 + fi_al_ahd / 36_524 - fi_al_ahd / 146_096) / 365;
    let sana = sana_fi_al_ahd + ahd * 400;
    let fi_as_sana = fi_al_ahd - (365 * sana_fi_al_ahd + sana_fi_al_ahd / 4 - sana_fi_al_ahd / 100);
    let shahr_muzah = (5 * fi_as_sana + 2) / 153;
    let yawm = fi_as_sana - (153 * shahr_muzah + 2) / 5 + 1;
    let shahr = if shahr_muzah < 10 { shahr_muzah + 3 } else { shahr_muzah - 9 };
    let sana = if shahr <= 2 { sana + 1 } else { sana };

    format!("{sana:04}-{shahr:02}-{yawm:02}T{saa:02}:{daqiqa:02}:{thaniya:02}Z")
}

/// Parses the seconds out of an RFC 3339 timestamp this module wrote.
///
/// Only the shape this module produces is accepted — `YYYY-MM-DDTHH:MM:SSZ`, UTC,
/// no fractional part, no offset — and anything else is [`None`]. Deliberately
/// narrow: this is used by `tahaqquq` to decide whether a file changed after the
/// installation, and a lenient parser that guessed at an offset would shift that
/// boundary by hours and silently reclassify a store update as a user's edit.
#[must_use]
pub fn thawani_min_rfc3339(nass: &str) -> Option<i64> {
    if nass.len() != 20 || !nass.ends_with('Z') || !nass.is_ascii() {
        return None;
    }
    let raqm = |min: usize, ila: usize| -> Option<i64> {
        nass.get(min..ila).and_then(|juz| juz.parse::<i64>().ok())
    };
    let sana = raqm(0, 4)?;
    let shahr = raqm(5, 7)?;
    let yawm = raqm(8, 10)?;
    let saa = raqm(11, 13)?;
    let daqiqa = raqm(14, 16)?;
    let thaniya = raqm(17, 19)?;
    if !(1..=12).contains(&shahr) || !(1..=31).contains(&yawm) {
        return None;
    }
    let ayyam = ayyam_min_taqweem(sana, shahr, yawm)?;
    ayyam
        .checked_mul(THAWANI_YAWM)?
        .checked_add(saa.checked_mul(3_600)?)?
        .checked_add(daqiqa.checked_mul(60)?)?
        .checked_add(thaniya)
}

/// Days since the Unix epoch for a civil date, the inverse of the era algorithm
/// above.
#[expect(
    clippy::integer_division,
    reason = "the three divisions are the exact leap-year counts of the proleptic \
              Gregorian calendar, on a year already shifted to start in March"
)]
fn ayyam_min_taqweem(sana: i64, shahr: i64, yawm: i64) -> Option<i64> {
    let sana = if shahr <= 2 { sana.checked_sub(1)? } else { sana };
    let ahd = (if sana >= 0 { sana } else { sana.checked_sub(399)? }) / 400;
    let sana_fi_al_ahd = sana - ahd * 400;
    let shahr_muzah = if shahr > 2 { shahr - 3 } else { shahr + 9 };
    let fi_as_sana = (153 * shahr_muzah + 2) / 5 + yawm - 1;
    let fi_al_ahd = sana_fi_al_ahd * 365 + sana_fi_al_ahd / 4 - sana_fi_al_ahd / 100 + fi_as_sana;
    ahd.checked_mul(146_097)?.checked_add(fi_al_ahd)?.checked_sub(719_468)
}
