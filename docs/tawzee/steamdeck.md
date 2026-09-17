# التوزيع / ستيم دك — the Steam Deck verification of the linux-x64 AppImage

This file is task 3 of the Phase 22 dispatch (`docs/tawzee.md` §8). It verifies
the `steamdeck` row of the target table (`docs/tawzee.md` §2, lines 26–27) —
"read-only `/`, data under `~/.local/share/taarib`, Steam flatpak and native
library roots both scanned" — against SteamOS reality and against this source
tree as it stands. Every row below is either verified from the code (file:line)
or from an authoritative external source; anything that could not be verified
either way is a named open question, and nothing here is aspirational. Findings
discovered during verification are collected at the end; this file changes no
code.

> **Overtaken since it was written — re-read against the tree on 2026-09-06.**
> This is an audit record and its body is left as it was. These parts no longer
> describe the tree:
>
> - Checklist rows E1, E3 and E4 record `bundle.resources`, `mawarid/`, the
>   `mukawwinat_tahmil.rs` / `bidaya.rs` / `istiada_cli.rs` files and the
>   `taarib-tajmee` / `taarib-tahdith` / `taarib-mudkhal` crates as absent. All
>   of them exist, and `mawarid/` on the authoring machine holds a fully staged
>   component tree with its manifest.
> - Finding F1 ("recorded launch options are never applied or shown") is closed
>   by code: `masar_tathbeet::thabbit` calls `naffidh_idadat`, which reads the
>   requirement from the manifest through `itlaq::talabat_steam` and performs it
>   through `itlaq::naffidh_talabat_steam` into every signed-in account's
>   `localconfig.vdf`; `taraju::nafidh` restores it on uninstall through
>   `RadItlaq`. That wiring is days old, was still being edited when this note
>   was written, and has not been exercised against a real Steam account, so the
>   manual step in §3 remains the safe instruction.
> - Finding F3's "harmless today because of F1" no longer holds, and the hazard
>   it feared does not arise: `naffidh_talabat_steam` checks whether Steam is
>   running before it touches any account file, independently of the
>   `HalatIdadat` seam it describes (which is still passed as `None`).
> - Finding F5 ("one Steam root is scanned, not both") is wrong about the
>   current code: `steam::hall_judhur` collects every valid candidate,
>   including the flatpak and snap roots, deduplicated.
> - The `file:line` citations in §2 and §4 have drifted as `tarkib.rs` and
>   `itlaq.rs` grew; the function names beside them are what to grep for.

## 1. SteamOS, in the terms that decide this target

| fact | consequence for Taarib | source |
| --- | --- | --- |
| The root filesystem is immutable; `steamos-readonly` gates writes to it, and Valve's own guidance is that "anything installed outside of flatpak (via pacman for instance) may be wiped with the next SteamOS update" | nothing Taarib does may depend on a system package, a file under `/usr`, or a pacman install; the shipped artifact must be self-contained and live in the home directory | [Steam Deck partner FAQ](https://partner.steamgames.com/doc/steamdeck/faq); [`steamos-readonly` man page](https://linuxcommandlibrary.com/man/steamos-readonly) |
| Only `/home` survives an OS update | the AppImage, the data root and the settings root must all be under `/home/deck` | same FAQ; community reports of everything outside `/home` being reset by updates |
| Stock SteamOS provides **no** `org.freedesktop.secrets` Secret Service — apps that need one fail with "The name org.freedesktop.secrets was not provided by any .service files" | every keychain-backed feature must degrade to a named error and never block startup or the core product | [ValveSoftware/SteamOS#928](https://github.com/ValveSoftware/SteamOS/issues/928) (open request to add libsecret); [reproductions in GitHub Desktop](https://github.com/desktop/desktop/issues/17964) and Steam community threads |
| SD cards mount under `/run/media/…`, and the exact path **changed across SteamOS versions** (`/run/media/mmcblk0p1` before 3.4, `/run/media/deck/<name>` after) | hard-coding a mount path would break on either side of the change; the only stable source of SD-card library paths is Steam's own `libraryfolders.vdf`, which is what the scanner reads | community reports of the 3.4 path change breaking hard-coded paths (EmuDeck and manual shortcuts) |
| AppImages run in desktop mode once marked executable; they need FUSE (libfuse2) to mount, with `--appimage-extract-and-run` as the documented fallback; `~/Applications` is the ecosystem's conventional location (watched by `appimaged` alongside `~/.local/bin`, `~/Downloads`, `/opt`) | the install steps in §3 are double-click-level; no packaging step may assume a system webkit, and the Tauri AppImage "bundles all dependencies and files needed by the application" | [AppImage FUSE troubleshooting](https://docs.appimage.org/user-guide/troubleshooting/fuse.html); [appimaged watch list](https://github.com/probonopd/go-appimage); [Tauri v2 AppImage docs](https://v2.tauri.app/distribute/appimage/) |
| Game mode runs only what Steam launches; the studio is a desktop application; the per-game **Launch Options** field exists in both modes, and Valve's own Proton documentation defines the mechanism as an environment assignment before `%command%` ("input `PROTON_USE_WINED3D=1 %command%`") | desktop mode is the supported environment for the studio itself; the launch-option string Taarib constructs (§2, D-rows) is exactly the form Valve documents, so it is honoured wherever the game is launched from — game mode included | [Proton README, runtime config options](https://github.com/ValveSoftware/Proton) |

## 2. Verification checklist

Status values: **verified (code)** with file:line, **verified (external)** with
the source above, **gap** with a finding number from §4, **open** with an owner
from the §8 dispatch table.

### A — artifact placement, data root, first run

| # | claim | how verified | status |
| --- | --- | --- | --- |
| A1 | An AppImage bundle target is configured for the studio | `tauri.conf.json` `bundle.targets` lists `"appimage"`; `msi`/`rpm` were removed at convergence, closing F7 | verified (code) |
| A2 | The AppImage is self-contained — webkit2gtk and friends ride inside it, nothing is installed into the read-only root | Tauri v2: "AppImage … bundles all dependencies and files needed by the application" | verified (external) |
| A3 | The binary makes no assumption about its own location — it can live in `~/Applications`, `~/Downloads`, or an SD card | no code in the tree derives paths from the executable's directory (the portable marker of `tawzee.md` §5 is not implemented yet — see A8) | verified (code, by absence) |
| A4 | The data root on Linux resolves to `$XDG_DATA_HOME/taarib`, i.e. `~/.local/share/taarib` on a stock Deck | `crates/taarib-usus/src/masarat.rs:13` (layout doc), `masarat.rs:82-88` (`BaseDirs::data_dir().join("taarib")` behind the `TAARIB_BAYANAT` override at `masarat.rs:73-76`) | verified (code) |
| A5 | Settings resolve to `$XDG_CONFIG_HOME/taarib`, i.e. `~/.config/taarib` | `masarat.rs:90-96` | verified (code) |
| A6 | First run writes only under the home directory: it creates the data-root subtree and opens settings, log and database — no root-filesystem write, no package install, no keychain touch | `apps/studio/src-tauri/src/main.rs:489-512` (startup order: `iktashif` → `takid` → settings → log → database; no `mafatih`/`keyring` call anywhere in `main.rs`), `masarat.rs:257-274` (`takid` creates eleven directories, all under the two roots) | verified (code) |
| A7 | Both roots being under `/home` means the whole product state survives a SteamOS update | A4 + A5 + the platform facts in §1 | verified (code + external) |
| A8 | The `taarib.mahmul` portable marker of `tawzee.md` §5 | `crates/taarib-usus/src/masarat.rs` — `ALAMAT_MAHMUL`, resolved in `iktashif()` and reported by `mahmul()`; `$APPIMAGE`'s parent is the marker directory inside an image | verified (code) |
| A9 | Self-update swap rules for the AppImage (`tawzee.md` §6: write beside, fsync, atomic rename, keep `*.sabiq`) | `crates/taarib-tahdith/src/tabdil.rs` — `istadill` reads `$APPIMAGE`, `naffidh` does the staged-rename dance with both fsyncs, `tahaqqaq_bad_iqla` settles or undoes an interrupted swap at startup | verified (code) |
| A10 | `.desktop` entry for desktop-mode menus | `apps/studio/src-tauri/linux/taarib.desktop`, wired through `bundle.linux.deb.files` **and** `bundle.linux.appimage.files`, in both cases mapped onto the path the bundler generates so it replaces the English entry rather than adding a second one; carries the Arabic name, `Exec=taarib-studio %U` and `StartupWMClass=taarib-studio` | verified (built + installed, `tahaqquq.md` §5.4) |

### B — keyring reality

The premise: stock SteamOS has no Secret Service (§1). The `keyring` dependency
is pinned at the workspace root, `Cargo.toml:180`:
`keyring = { version = "4.1.6", features = ["v1"] }`. On docs.rs, keyring 4's
crate root is `pub use v1::*;` plus `pub use cli::*;`, so
`keyring::Entry`/`keyring::Error` (used by `taarib-khatm`) and
`keyring::v1::Entry`/`keyring::v1::Error` (used by `taarib-tarjama`) are the
same types, and the `v1` module's Linux backend is the Secret Service over
D-Bus. On a machine without one, access fails with `NoStorageAccess` ("the
underlying secure storage holding saved items could not be accessed") or
`PlatformFailure` — **not** `NoEntry`, which specifically means the store was
reachable and held nothing.

| # | claim | how verified | status |
| --- | --- | --- | --- |
| B1 | All key custody goes through one module, service name `taarib.tawqee`, with no panic, `unwrap` or `expect` on any path | `crates/taarib-khatm/src/mafatih.rs:9` (service name), whole file `mafatih.rs:1-87` — every function returns `Result`, every keyring error is mapped | verified (code) |
| B2 | A dead Secret Service surfaces as `KhataKhatm::KhataMiftah` (named error), never as a panic: only `keyring::Error::NoEntry` maps to `MiftahMafqud` | `mafatih.rs:53-71` — `hat` matches `NoEntry` at `mafatih.rs:56-58`, everything else at `mafatih.rs:59-64` | verified (code); the "unavailable ≠ absent" collapse is finding F4 |
| B3 | Startup never touches the keychain, so a stock Deck launches cleanly | `main.rs:489-512` and the whole of `main.rs` — no keychain call; `grep` across `apps/studio/src-tauri/src` finds keychain use only inside `taqdeem_awamir.rs` commands | verified (code) |
| B4 | The session strip (`jalsati`) works without a keychain: the editor identity is a **file**, `<data root>/muharrir.json`, and the owner check swallows keychain errors into `false` | `apps/studio/src-tauri/src/taqdeem_awamir.rs:767-770`; `apps/studio/src-tauri/src/warsha_awamir.rs:413-444` (file-minted identity); `crates/taarib-khatm/src/malik.rs:146-148` (`huwa_malik` is `hat_malik().is_ok()`) | verified (code) |
| B5 | Every owner-review command degrades to the "owner only" refusal rather than a keychain error | `taqdeem_awamir.rs:432-438` (`salahiyat_malik` maps **any** `hat_malik` failure to `KhataTaqdeemAmr::MalikFaqat`), applied at `taqdeem_awamir.rs:1390, 1482, 1560, 1617, 1701, 1837, 1938, 2084, 2112` | verified (code) |
| B6 | Contributor signing fails as a named command error, not a crash: `jahhiz_taqdeem` and `sallim_taqdeem` call `miftah_musahim`, which propagates `KhataKhatm` into the command `Result` | `taqdeem_awamir.rs:423-429` (mint-on-first-use), call sites `taqdeem_awamir.rs:896` (inside `jahhiz_taqdeem`, starts :802) and `taqdeem_awamir.rs:1220` (inside `sallim_taqdeem`, starts :1100) | verified (code); UX consequence is finding F4 |
| B7 | The forge token store fails closed with a named error; an absent token is `None` only when the store answered | `crates/taarib-taqdeem/src/irsal.rs:401-441` (`hat_ramz`: `MiftahMafqud` → `Ok(None)` at :404, anything else → `Err` at :405), `irsal.rs:357-360` ("the OS keychain refused: …"), writes at `irsal.rs:368-393`, deletes at `irsal.rs:448-454` | verified (code); see F4 |
| B8 | Translation-provider credentials degrade with an error that **distinguishes** "no key yet" from "store unreachable" ("locked keychain, dead D-Bus session") | `crates/taarib-tarjama/src/muzawwidun.rs:240-247` (`NoEntry` → `Ok(None)`), `muzawwidun.rs:236-239` (the distinction, stated), `muzawwidun.rs:312-317` ("the platform secret store failed: …"); studio commands `apps/studio/src-tauri/src/idadat_awamir.rs:127-161` | verified (code) |
| B9 | Nothing on the Deck's ordinary path — scan, translate offline, one-click install, restore — needs the keychain at all | keychain callers enumerated by grep: `malik.rs`, `irsal.rs`, `muzawwidun.rs`, `taqdeem_awamir.rs`, `idadat_awamir.rs` only; none is reached by `maktaba`, `thabbit_ruqaa` or the workshop commands (`main.rs:514-583` command list) | verified (code) |

### C — game discovery on a Deck

| # | claim | how verified | status |
| --- | --- | --- | --- |
| C1 | The native root is found: candidates are `~/.steam/steam`, `~/.local/share/Steam`, `~/.steam/root`, `~/.steam/debian-installation`, in that order — the first two are exactly SteamOS's layout (`~/.steam/steam` is a symlink to `~/.local/share/Steam`, and `sawwi`/canonicalize collapses the pair) | `crates/taarib-kashf/src/matajir/steam.rs:217-221`; symlink collapse `steam.rs:392-394` | verified (code) |
| C2 | The flatpak Steam prefix `~/.var/app/com.valvesoftware.Steam/data/Steam` is a candidate, and the container switch defaults **on** for Linux | `steam.rs:222-231` behind `siyaq.yashmal_hawiyat`; default `crates/taarib-kashf/src/lib.rs:248-255` (`yashmal_hawiyat: cfg!(target_os = "linux")` at :253); the studio builds its context through that constructor at `main.rs:252` | verified (code) |
| C3 | The snap layout is also a candidate | `steam.rs:232-241` | verified (code) |
| C4 | SD-card libraries are found through `libraryfolders.vdf` — both the pre-2021 string shape and the current object shape, and the file is looked for in all three historical locations — so the SteamOS 3.4 mount-path change (§1) cannot break discovery | `steam.rs:443-539` (`maktabat`), shapes handled `steam.rs:501-535`, locations `steam.rs:473-483` | verified (code + external context) |
| C5 | An unmounted SD card degrades to a per-library warning naming the path; the internal library still scans | `steam.rs:456-465` | verified (code) |
| C6 | Proton prefixes are resolved **per library** (`steamapps/compatdata` next to the game, not under the Steam root), so a game installed to the SD card finds its prefix on the SD card | `steam.rs:417-420` (`MaktabatSteam::compatdata`), used at `steam.rs:1530-1533` | verified (code) |
| C7 | Non-Steam shortcuts — most of a Deck's sideloaded library — are read from every account's `shortcuts.vdf`, including the `FlatpakAppID`-only kind, and their prefixes are searched across **all** libraries "because Steam creates it under whichever library was current … not on a Deck with an SD card" | `steam.rs:1282-1363` (reader), `steam.rs:1757-1794` (`luba_min_ikhtisar`, prefix search loop :1784-1794) | verified (code) |
| C8 | User grid artwork (the SteamGridDB collections Deck users curate) wins over the client cache | `steam.rs:1416-1457` | verified (code) |
| C9 | Both native **and** flatpak roots scanned *in the same pass* | was `hall_jidhr` returning the **first** valid candidate; now `hall_judhur` returns every valid candidate, deduplicated through `sawwi`, and `ifhas` builds one flat library list across all of them with a per-root `SiyaqSteam` | **verified (code)** — F5 closed, see §4b |

### D — Proton: the prefix and the launch options

| # | claim | how verified | status |
| --- | --- | --- | --- |
| D1 | The prefix path Taarib constructs is `steamapps/compatdata/<appid>/pfx`, matching what Proton builds | `steam.rs:1520-1534` (`beeat_app`), `crates/taarib-kashf/src/beea.rs:1339-1356` (`beea_steam`, incl. the `SteamApps` spelling and the bare-directory layout) | verified (code) |
| D2 | An unbuilt prefix (created at configure time, empty until first launch) is never treated as usable: a prefix requires `drive_c/` **and** `user.reg` | `beea.rs:162-180` (`hal_beea`/`hiya_beea`), install-side re-check `crates/taarib-tathbeet/src/tarkib.rs:768-827` (`tahaqquq_beea`) | verified (code) |
| D3 | The Proton build is resolved down the ladder `config.vdf` mapping → global default (key `0`, the "all other titles" switch) → the `version` file beside the prefix → the prefix itself, and never guessed | `steam.rs:976-1014` (`kharitat_tawafuq`), `steam.rs:1536-1573`; `beea.rs:448-483` (markers: `version`, `config_info`, `tracked_files`, `pfx.lock`) | verified (code) |
| D4 | The prefix profile is `steamuser` first — Proton's unconditional name, "so on a Steam Deck this is always the answer" | `beea.rs:1514-1531` | verified (code) |
| D5 | The drive map is read from `dosdevices`, never assumed — translated paths on a Deck land where the game looks | `beea.rs:314-343` | verified (code) |
| D6 | The override Taarib needs is `winhttp=n,b` carried in `WINEDLLOVERRIDES`, merged into whatever the user already has (semicolon entries preserved, `*`-prefixed and `n`/`b` alias forms recognised) | `tarkib.rs:29-35` (constants), `tarkib.rs:1035-1053` (`damj_tajawuz`); same constants and merge on the itlaq side `crates/taarib-tathbeet/src/itlaq.rs:27-31, 78-110` | verified (code) |
| D7 | For a Steam game the assignment is carried in the **launch options** and nowhere else, as `WINEDLLOVERRIDES="winhttp=n,b" %command%`, inserted before an existing `%command%` and preserving everything around it | `tarkib.rs:1100-1152` (`idadat_tahmil`; the Steam branch :1133-1145), `tarkib.rs:1074-1096` (`damj_khiyarat` + quoting), equivalent construction `itlaq.rs:112-153` | verified (code) |
| D8 | That written mechanism is compatible with Steam on the Deck: an environment assignment before `%command%` is Valve's own documented way to configure a Proton game, and the Launch Options field is present in game mode and desktop mode alike | Proton README ("Set the variable, followed by `%command%`"), §1 | verified (external) |
| D9 | The `localconfig.vdf` writer edits every signed-in account's file, refuses while Steam is running (`MunassaTaamal`), and surgically replaces only the one key | `itlaq.rs:826-832` (`li_kul_hisab`), `itlaq.rs:875-888` (`athbit`, guard at :876), `itlaq.rs:312-320` + `crates/taarib-usus/src/manassa.rs:188-214` (process check by name: `steam.exe`, `steam`, `steam_osx`), splice `itlaq.rs:722-760` | verified (code); **but see findings F1–F3 — nothing in the studio invokes this writer** |
| D10 | Games mid-download/mid-update are never offered as patchable (`StateFlags` bit 2 required, sixteen "changing" bits excluded), and an install refuses while the game itself runs | `steam.rs:91-176` (`hala`, `muktamila`), `crates/taarib-mustawda/src/tathbeet_bilnaqra.rs:129-131` (executable name for the running check, enforced inside `thabbit`) | verified (code) |
| D11 | Framework component deployment reads only Taarib's own component store under the data root | `tarkib.rs:937-1010` (`hamil_mukawwin`), store path `masarat.rs:209-211` (`<data root>/mukawwinat`) | verified (code); the store is never populated today — **finding F6** |

### E — the packaging state this tree is actually in

| # | claim | how verified | status |
| --- | --- | --- | --- |
| E1 | `bundle.resources = ["mawarid"]` (`tawzee.md` §4) | absent from `apps/studio/src-tauri/tauri.conf.json`; no `mawarid/` directory exists | open — owned by convergence + task 5; noted in finding F7 |
| E2 | `msi` and `rpm` "deliberately not shipped" (`tawzee.md` §2) | `tauri.conf.json:42` still lists both | **gap — finding F7** (file owned by convergence; report-only) |
| E3 | Startup component mirroring (`mukawwinat_tahmil.rs`), first-run (`bidaya.rs`), restore-on-uninstall (`istiada_cli.rs`) | none of the three exists under `apps/studio/src-tauri/src/` | open — owned by tasks 12, 9, 8 |
| E4 | `taarib-tajmee`, `taarib-tahdith`, `taarib-mudkhal` crates | none exists under `crates/` | open — owned by tasks 5, 6–7, convergence |

## 3. Installing on a Steam Deck — the steps that work today

Everything below was verified against the code paths cited in §2; nothing
requires `sudo`, `steamos-readonly`, pacman, or a developer mode switch.

1. **Switch to desktop mode** (power menu → *Switch to Desktop*). The studio
   is a desktop application; game mode is not a supported environment for it.
2. **Place the AppImage.** Download `Taarib_<version>_amd64.AppImage`, create
   `~/Applications` if it does not exist, and put the file there (the
   conventional, tool-watched location — any user-writable path works, per
   A3). In Dolphin: right-click → *Properties* → *Permissions* → tick *Allow
   executing file as program*. Double-click to launch.
   - If the launcher reports a FUSE/mount error (rare on stock SteamOS), run
     it once from Konsole as
     `./Taarib_<version>_amd64.AppImage --appimage-extract-and-run`.
3. **First launch** creates `~/.local/share/taarib` (database, patches,
   backups, projects, logs) and `~/.config/taarib` (settings) — nothing else,
   anywhere (A6). Both survive SteamOS updates (A7).
4. **The library fills itself.** The scanner finds the native Steam root, every
   SD-card library Steam lists, Proton prefixes beside each game, and
   non-Steam shortcuts, with per-item warnings for anything unreadable
   (C1–C8). No path needs configuring. A game that is downloading or updating
   is deliberately not offered for patching until Steam finishes (D10).
5. **Installing a translation** is the one-click flow: quarantine → safety
   checks → write, with automatic backups under the data root. Close the game
   first; the installer refuses while it runs (D10).
6. **Proton games that need the injected framework** (the studio names these
   during install): the loader files are deployed into the game, but the
   launch option that makes Wine load them is **not written automatically
   today** (findings F1–F3). Until that lands, set it yourself — Steam →
   right-click the game → *Properties* → *Launch Options*:

   ```
   WINEDLLOVERRIDES="winhttp=n,b" %command%
   ```

   This is the Valve-documented Proton mechanism (D8) and works whether the
   game is then launched from game mode or desktop mode. If you already have
   launch options, put the assignment in front of your existing `%command%`.
   Pure text patches and overlay-tier games need no launch option at all.
7. **Contributor features** — publishing a patch, the forge sign-in, cloud
   translation-provider keys — need an OS Secret Service, and stock SteamOS
   has none (§1, B-rows). These commands fail with a named keychain error
   while everything else keeps working (B4–B9). To use them on a Deck: in
   desktop mode install **KeePassXC** from Discover (flatpak), enable
   *Settings → Secret Service Integration*, keep the database unlocked while
   publishing. Signing in from a desktop PC instead is the simpler path.
8. **Updating Taarib**: download the new AppImage and replace the old file.
   There is no built-in updater in this tree yet (A9); the data root is
   untouched by a swap.
9. **Removing Taarib**: restore your games first (each game's page offers full
   removal of installed content), then delete the AppImage. The data root
   stays until you delete `~/.local/share/taarib` and `~/.config/taarib`
   yourself — deleting it does not un-patch games, which is why restore comes
   first.

## 4. Findings

Numbered for the Phase 22 report; none of these is fixed by this file.

- **F1 — recorded launch options are never applied or shown.**
  `tarkib::rakkib_itar` computes and *records* the Steam launch-option value
  (`tarkib.rs:1133-1145`, via `sajjil_idadat` `tarkib.rs:1185-1197`), and
  `itlaq.rs` contains a complete, guarded `localconfig.vdf` writer
  (`KhiyaratSteam::athbit`, `itlaq.rs:875-888`) — but no code in the studio or
  any crate calls that writer, and the frontend contains no occurrence of
  `WINEDLLOVERRIDES` or `%command%` to instruct the user. Failure scenario: a
  Deck user one-click-installs into a Proton Unity game; BepInEx's
  `winhttp.dll` proxy lands beside the executable; Wine loads its builtin
  `winhttp` instead; the game boots in English with no error anywhere — the
  exact silent failure `beea.rs:1259-1268` warns about.
- **F2 — the install path feeds `HalatIdadat` all-`None`.**
  `apps/studio/src-tauri/src/tathbeet_awamir.rs:1280-1285` passes
  `khiyarat_tashghil: None` although discovery read the user's real options
  (`steam.rs:1139-1188`) and the library stored them (`main.rs:351`). The
  recorded "previous value" is therefore `None` and the recorded target value
  is computed as if the user had no options. Failure scenario: a user with
  `mangohud %command%` set — common on Deck — gets a manifest whose restore
  step would erase that option, and (once F1 is wired) a written value that
  drops it.
- **F3 — the Steam-is-running refusal is disarmed on the install path.**
  `tahaqquq_manassa` only fires when the caller supplies
  `malaf_idadat_manassa` (`tarkib.rs:1174`), and `tathbeet_awamir.rs:1284`
  supplies `None`. Harmless today because of F1; the moment F1 is fixed
  without also arming this, launch options get written while Steam is running
  — and on a Deck Steam is effectively *always* running — so Steam overwrites
  the edit from memory on exit and the loader silently never activates again.
- **F4 — "store unreachable" collapses into "keychain refused" for
  contributor flows.** `mafatih::hat` maps everything but `NoEntry` to
  `KhataKhatm::KhataMiftah` (`mafatih.rs:56-64`); on a stock Deck the Secret
  Service is absent, so `hat_ramz` returns `Err` instead of `Ok(None)`
  (`irsal.rs:401-406`) and `sallim_taqdeem` dies at `taqdeem_awamir.rs:1185`
  with "the OS keychain refused …" before the sign-in flow is even offered.
  Not a crash — but `muzawwidun.rs:236-239` shows the codebase already knows
  these are different user problems ("you have not entered a key" vs "your
  key is in a store this machine cannot open"), and the khatm side does not
  make the distinction.
- **F5 — one Steam root is scanned, not both.** `hall_jidhr` takes the first
  valid candidate (`steam.rs:352`); on a Deck the native root always exists,
  so a flatpak Steam installed in desktop mode (and its separate library) is
  invisible, with no warning naming the unscanned root. The contract row
  (`tawzee.md` §2) says "Steam flatpak and native library roots both
  scanned".
- **F6 — the component store is never created or populated.**
  `Masarat::takid` (`masarat.rs:257-274`) creates eleven directories but not
  `mukawwinat()` (`masarat.rs:209-211`), and the startup mirror that would
  fill it (`mukawwinat_tahmil.rs`, dispatch task 12) does not exist. Failure
  scenario today, on every host including the Deck: any framework-tier
  install reaches `hamil_mukawwin` and refuses with `MukawwinMafqud`
  (`tarkib.rs:948-953`).
- **F7 — `tauri.conf.json` contradicts the contract.** `tauri.conf.json:42`
  ships `"msi"` and `"rpm"`, which `tawzee.md` §2 (lines 31–32) forbids, and
  the file declares no `bundle.resources = ["mawarid"]` (`tawzee.md` §4,
  lines 78–79). The file is owned by the convergence step; recorded here
  report-only.

## 4a. Phase 28 addendum — 2026-09-04

No Deck was reached, and none of §2 was executed on SteamOS. What changed is
that the artifact a Deck would run now exists and has been measured on ordinary
Linux, which settles one open question and stales two findings.

**The glibc question is answered, and the answer is no.** §5 below lists "glibc
baseline of the release build" as unverifiable from source. It is verifiable
from the artifact, and the artifact fails. Taking the highest versioned
undefined symbol across `taarib-studio` and all 187 libraries the AppImage
bundles:

| | floor |
| --- | --- |
| `taarib-studio` itself | `GLIBC_2.39` |
| bundled libraries | **`GLIBC_2.42`** — `__inet_pton_chk`, in `libwebkit2gtk-4.1.so.0` |
| bundled libraries | `GLIBCXX_3.4.30`, `CXXABI_1.3.15`, `GCC_13.0.0` from the host `libstdc++` |

An AppImage never bundles glibc or `libstdc++`, so those come from the host. The
2026-09-04 build was made on a rolling distribution carrying glibc 2.42 and
inherited its floor; no SteamOS release provides that, so **the image built
today would not start on a Deck** — `version 'GLIBC_2.42' not found`, before any
Taarib code runs. This is a build-environment defect: the Linux artifact has to
be built in an old-baseline container (Ubuntu 22.04 → `GLIBC_2.35`, 24.04 →
`2.39`). It must be fixed before a Deck test is worth attempting at all.
Detail in `tahaqquq.md` §4.1, and the shipping rule in `linux.md` §5a.

**Two findings in §4 are overtaken and should not be read as current.**

- **F6** — the component store now exists: `mukawwinat_tahmil.rs` runs at
  startup and `Masarat` gains the directory. What replaced the finding is
  narrower and still real: `apps/studio/src-tauri/mawarid/` contains only its
  `README.md`, so the mirror has nothing to copy and every framework-tier
  install still refuses by name. `taarib-tajmee` has to run before
  `tauri build`, on every platform (`tahaqquq.md` §5.1).
- **F7** — `tauri.conf.json` no longer ships `msi` or `rpm`, and it declares
  `bundle.resources = ["mawarid"]`. Closed.

**What a Deck would settle that nothing here can.** Once the glibc floor is
fixed: whether SteamOS's FUSE mounts the image (it mounted on a `libfuse3`-only
host with no `libfuse2`, which is encouraging and not transferable); whether
Valve's session compositor is happy with the window; whether the desktop entry
appears in the KDE menu — the AppImage now carries the Arabic entry at the name
the bundler generates, `Taarib.desktop`, rather than the English one
(`linux.md` §3), but only `appimaged` or a manual integration puts it in the
menu at all; and whether
the keyring degradation of §B is tolerable in practice given that F4 makes the
Deck's normal case — no Secret Service — read as "the OS keychain refused".

## 4b. Phase 28 Stage 4 — 2026-09-05

Still no Deck. What this pass adds is the **number SteamOS actually ships**, a
build environment that gets under it, and one closed finding.

### How far under it we had to get

Read from Valve's own mirror, `steamdeck-packages.steamos.cloud`, package index
of `core-<channel>/os/x86_64/`:

| SteamOS channel | glibc |
| --- | --- |
| 3.5 | 2.37 |
| 3.6 | 2.39 |
| 3.7 | 2.40 |
| 3.8.1x — current stable | 2.41 |

`2.41 < 2.42`. The 2026-09-04 image starts on **no SteamOS release that has ever
shipped**, the newest included — §4a's "would not start on a Deck" was right and
is not a margin call.

Two corrections to §4a while the numbers are in front of us:

- The binary's own `GLIBC_2.39` requirement comes from `pidfd_getpid` and
  `pidfd_spawnp`, and both are **`WEAK`** undefined symbols. That looks like an
  optional reference the loader may skip. It is not: weakness belongs to the
  symbol, the loader gates on the version-need entry, and `readelf -V` shows
  `Name: GLIBC_2.39  Flags: none`. Running the 2026-09-04 binary on a glibc-2.35
  host with that host's own `libwebkit2gtk-4.1` — nothing bundled involved —
  dies with `version 'GLIBC_2.39' not found`. So SteamOS 3.5 could not have run
  even a perfectly-bundled build of it.
- The `.deb` was said to be immune because `Depends:` would refuse an
  unsatisfiable install. `Depends:` is generated from `DT_NEEDED` names, never
  from symbol versions, so it installs on a glibc-2.36 host and then fails to
  start. Both artifacts carry the build host's floor.

### The build that gets under it

`apps/studio/src-tauri/linux/` now holds a `Dockerfile` (Ubuntu 22.04, glibc
**2.35**), `ibni.sh` which drives the whole build through it, and `qias_qaa.sh`
which measures the floor out of the produced AppDir and fails the build when it
moves. 22.04 is the newest base under every SteamOS release that still carries a
maintained `libwebkit2gtk-4.1` (`2.50.4-0ubuntu0.22.04.1`), which Tauri v2
requires. `linux.md` §5a and §8 are the detail.

### F5 is closed

`hall_jidhr` took the first valid Steam root, so on any machine carrying both a
native and a Flatpak Steam the second one's whole library was invisible with
nothing said. It is now `hall_judhur`, returning every valid root deduplicated
through `sawwi` — which collapses the `~/.steam/steam` → `~/.local/share/Steam`
symlink pair, the case that made "first wins" look correct on a Deck. `ifhas`
builds one flat library list across all roots, deduplicated by canonical path
because two clients routinely list the same external drive, and keeps a
**per-root** `SiyaqSteam`: `appinfo.vdf`, `config.vdf` and `userdata` all belong
to one client, and reading one root's metadata against another root's games
would mis-report both. Shortcuts are admitted once per CRC-32 identifier.

`NatijatMatjar::jidhr_matjar` still names the first root, because it is a
display field and a game is a game whichever client installed it.

### Proton, re-checked rather than re-assumed

Rows C6 and D1–D5 were re-read against the current tree and hold. Nothing about
Proton is unhandled: `crates/taarib-kashf/src/beea.rs` is 1,600 lines whose only
subject is prefixes — the drive map from `dosdevices`, the `.reg` parser, the
`drive_c` + `user.reg` validity gate, the build ladder — and every Linux-ish
adapter (`bottles`, `lutris`, `heroic`, `legendary`) routes through it instead
of reimplementing it. `steam.rs::beeat_app` resolves
`<library>/steamapps/compatdata/<appid>/pfx` per library, so an SD-card game
finds its prefix on the SD card, and `taarib-tathbeet` re-validates the prefix
before writing (`tarkib::tahaqquq_beea`) and picks prefix-shaped destinations
only for a game whose files really live under `drive_c` — which a Steam/Proton
game's do not.

Two inconsistencies, reported not fixed.

**One prefix path, two resolvers.** `beea::beea_steam` is a public, validated
Steam-prefix resolver that `steam.rs::beeat_app` does not call — it builds the
path itself and gates on `is_dir()` rather than `hiya_beea` (`drive_c` **and**
`user.reg`). An unbuilt prefix therefore reports as Proton with a warning
attached rather than as native. The install side re-checks properly, so this is
belt-and-braces rather than a live defect, but two resolvers for one path is one
too many.

**The script-engine dispatcher does not case-fold, and the install path does.**
`taarib_tathbeet::masar_tathbeet::thabbit` now routes text installs through
`nusus::raqqi_nusus`, which identifies the engine from the game directory with
`taarib_muhawwil_nusus::tarkeeb::ayn_hadaf`. Two of the four locators there use
literal joins:

- Ren'Py — `jidhr.join("renpy")`, `jidhr.join("game").join("script.rpyc")`
  (`tarkeeb.rs:221-223`);
- GameMaker — `jidhr.join(nisbi)` over the candidate list
  (`gamemaker.rs:4522-4526`).

RPG Maker does not: `yujad_malaf_mashru` (`rpgmaker.rs:341-349`) lists the
directory and lowercases each entry, which is the right shape.

A Windows game's files reached a Linux disk through some Windows tool, and Wine
resolves paths case-insensitively so the game itself never notices a `Game/`
that should be `game/`. Rust's `join` does. `ayn_hadaf` then answers `None`,
`raqqi_nusus` returns `Ok(None)`, and `thabbit` treats that as "not a script
engine" and completes — a reported-successful install that translated nothing.
That is exactly the silent class of failure the prefix work exists to avoid, and
it is inconsistent with the destination side of the same install:
`tarkib::MawqiTarkib::mutlaq` case-folds through `beea::hall_bila_hala` before
writing.

Low probability for a Steam title — depots preserve the developer's casing, and
Ren'Py and RPG Maker both emit lowercase — and materially higher for the
Bottles, Lutris and Heroic games whose files a Windows installer wrote inside a
prefix. The fix is a case-folded lookup in those two locators, the way
`rpgmaker.rs` and `beea::hall_bila_hala` already do it. `taarib-muhawwil-nusus`
is an adapter crate and is not this task's to edit.

### The verdict a Deck owner cares about

**Before this pass: no.** The AppImage would not start on any SteamOS release —
loader error, before a line of Taarib code ran.

**After: yes, for the scan-and-read half of the product; not yet for a
framework-tier install.**

The image built by `ibni.sh` was extracted and started under `xvfb` on stock
`ubuntu:22.04` (glibc 2.35) and `ubuntu:24.04` (2.39), where the 2026-09-04
image dies at the loader on both. On a fresh account with nothing installed it
brought the database schema forward, created the twelve directories of the data
root and `~/.config/taarib`, and finished a library scan with `alaab=0` and no
error. 2.35 is under SteamOS 3.5's 2.37, so every SteamOS release clears it.

What still stands between that and a Deck:

- **`bayan_mukawwinat.json` is still not staged.** The fonts now are — 25 faces
  under `mawarid/khutut/` in both artifacts — but the component manifest is not,
  so the startup mirror still reports one problem and every framework-tier
  install still refuses by name (`tahaqquq.md` §5.1). Text-only and overlay-tier
  patches are unaffected.
- **The artifacts are development-signed** (`tahaqquq.md` §5.2). A release needs
  `TAARIB_MIFTAH_ISDAR` and `--features isdar`. Since 2026-09-17 `ibni.sh`
  carries both once the variable is exported, and says which identity it built
  when it is not — until then it passed neither, so no AppImage from this path
  has ever been a release one.
- **No Deck has run it.** FUSE on SteamOS, Valve's session compositor, and
  whether the KDE menu shows the bundler's English entry are all still untested
  on the device. Everything above was measured on ordinary Linux.
- **No Secret Service on stock SteamOS**, unchanged, and F4 still makes that
  read as "the OS keychain refused" (§B).

### Flatpak was considered and is not shipped

It would make the floor irrelevant and would register `.ruqaa` properly, and it
is the Deck's own install path. It is blocked on something that is not a
packaging problem: `flatpak run` gives the app its own PID namespace, so
`taarib_usus::manassa::amaliyat_bism` sees no host processes, and the two
refusals built on it — "the game is running" and "Steam is running" — stopped
firing without saying anything. `linux.md` §9 has the change.

**Half of that is now done.** As of 2026-09-05 `manassa.rs` carries
`fi_sunduq()`, which names the sandbox from `/.flatpak-info`, `$FLATPAK_ID`,
`$SNAP`+`$SNAP_NAME`, `$container`, `/run/.containerenv` or `/.dockerenv`;
`halat_tashghil()`, which has a third state meaning "not knowable here"; and a
`tashtaghil()` that folds that state into "treat as running" rather than "no".
So a sandboxed build can no longer lose a refusal silently. What is still
missing is the two call sites in `crates/taarib-tathbeet` consuming the honest
answer and naming the sandbox in their message — `masar_tathbeet::la_tashtaghil`
in particular still reads an empty process list as "the game is not running".
Until that lands the decision not to ship a Flatpak stands, and the second
blocker — `--filesystem=host` or a `flatpak override` per launcher — is
untouched and is the harder one.

On a Deck this matters more than anywhere else: Steam is always running there,
so the refusal that a sandbox disarms is the one that fires every single time.

## 5. Open questions

- **Secret Service on future SteamOS.** Whether Valve ships a
  `org.freedesktop.secrets` provider is tracked upstream
  ([SteamOS#928](https://github.com/ValveSoftware/SteamOS/issues/928)); until
  it lands, §3 step 7 is the honest guidance.
- ~~**glibc baseline of the release build.**~~ **Answered in §4a, negatively.**
  The 2026-09-04 AppImage needs `GLIBC_2.42` because it was built on a host
  with that glibc. No SteamOS release provides it. The CI image for the
  linux-x64 build is still not defined anywhere in this tree, which is the
  actual root cause and still has no owner.
- **FUSE on every SteamOS build.** AppImages running out of the box on the
  Deck is well-attested but community-verified, not Valve-documented; the
  `--appimage-extract-and-run` fallback in §3 covers the failure case either
  way.
- **Update swap semantics on the Deck** (`tawzee.md` §6: rename beside the
  running AppImage, `*.sabiq` retention) cannot be verified until
  `taarib-tahdith` exists (tasks 6–7); the home-directory placement from §2-A
  is what makes that design possible on a read-only OS at all.
