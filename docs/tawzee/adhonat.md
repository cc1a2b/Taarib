# الأذونات — what Taarib needs, what it must never ask for, and how it degrades

Task 11 of the Phase 22 dispatch (`docs/tawzee.md` §8). Every claim about this
product is cited to code. Where the code does not degrade cleanly, that is
recorded as a gap rather than described as a behaviour.

## 0. The webview's own permission set

`apps/studio/src-tauri/capabilities/default.json` grants the main window the
core defaults plus `core:event:allow-listen`/`allow-unlisten`, and six
`core:window` grants for the strip the product draws in place of the platform
frame on Windows (`tauri.windows.conf.json` sets `decorations: false`):
`start-dragging` and `internal-toggle-maximize` for the drag region, `minimize`,
`toggle-maximize` and `close` for its three controls, and `is-maximized` for
the middle control's glyph. Those, `listen` and `invoke` are the only Tauri
APIs the frontend imports (`hayat/nafidha.ts`, `hayat/jisr.ts`). No filesystem,
shell, http or dialog plugin permission is held: every one of those goes
through a Taarib command, which is not governed by that file at all and
validates its paths against `taarib_usus::masarat` before touching the machine.

One plugin permission is held: `opener:allow-open-url`, scoped to `https://**`
and nothing else, for `tauri-plugin-opener`. It exists for the game screen's
community-translations panel, whose one control opens a maker's own page in
the default browser. The frontend does not call the plugin's command; it calls
`mujtama_awamir::iftah_rabt` (`apps/studio/src-tauri/src/mujtama_awamir.rs`),
which refuses anything that is not an `https` address of at most 2048
characters, free of whitespace, control characters and credentials, on a host
the cached community index links to or one of the known mod platforms — and
only then hands the address to the plugin from Rust. The scope in the
capability is the second fence behind that check: were the plugin's own
command ever invoked from the webview, nothing but an `https` page could open.
No `opener:allow-open-path` and no `opener:allow-reveal-item-in-dir` is held,
so the plugin can open no file and reveal no directory.

`core:webview:allow-internal-toggle-devtools` remains listed because it is a
member of `core:webview:default` and cannot be withdrawn from a capability at
tauri 2.11.5. The gate that actually matters is the cargo feature: the
workspace manifest no longer enables tauri's `devtools` feature, so the
inspector and its IPC command exist only under `debug_assertions`. A release
build has no devtools to toggle, whatever the capability lists.

## 1. Windows

**Needs.**

- *Filesystem*: the data root under `%APPDATA%`/`%LOCALAPPDATA%`
  (`crates/taarib-usus/src/masarat.rs`), and read/write inside game directories
  the user pointed at. Both are ordinary per-user paths; no ACL change, no
  elevation.
- *Registry, read only*: launcher discovery under `HKCU`/`HKLM` uninstall keys
  and per-launcher keys (`crates/taarib-kashf/src/matajir/`). Reads only —
  nothing in the discovery path writes to the registry.
- *Outbound HTTPS*: the registry, the forge, and the update channel
  (`crates/taarib-mustawda/src/masadir.rs` sets `https_only(true)`).
- *Process inspection*: the "is this game running" refusal enumerates processes
  by name through `sysinfo` (`taarib_usus::manassa::amaliyat_bism`, used by
  `masar_tathbeet::la_tashtaghil`). It reads names and image paths of processes
  in the same session. It does not open handles into another process, does not
  read another process's memory, and needs no `SeDebugPrivilege`.
- *Injection*: by the game's own loader, not by force — a proxy module beside
  the executable, or a Wine override, or a launch option
  (`taarib_tathbeet::tarkib`). Writing those needs exactly the file-write
  permission the game directory already grants the user.

**Must never ask for.** Administrator elevation — the install is per-user by
contract (`nsis.installMode: "currentUser"`, `tawzee.md` §2), and no runtime
path requests it. Nothing writes to `Program Files`, `HKLM`, or a service.
No autostart entry. No telemetry: a sweep for `analytics|telemetry|sentry|
posthog|mixpanel|amplitude` across `crates/`, `apps/studio/src`,
`apps/studio/src-tauri/src` and `package.json` returns nothing.

**Degradation.** A game directory under an ACL the user cannot write is the
install's own named write failure with the path. An unreachable registry key
yields "launcher not available" and an empty result, never a scan failure
(verified across all eighteen launcher adapters by the first-run task). No
network is the registry client's own refusal; the product still opens, still
scans, still edits.

## 2. Linux

**Needs.** The two roots under `$XDG_DATA_HOME` and `$XDG_CONFIG_HOME`; read
access to launcher catalogues including the flatpak Steam prefix and SD-card
library roots from `libraryfolders.vdf`; outbound HTTPS; the same
`sysinfo`-based process-name read; write access inside game directories, and
inside a Proton/Wine prefix for the DLL-override case.

**A note on `ptrace`.** Taarib does not attach to a running process on Linux:
loading happens through `LD_PRELOAD` set on the game's own launch. So
`/proc/sys/kernel/yama/ptrace_scope` does not gate anything Taarib does — at
any of its values, including `3`. `sysinfo`'s process listing reads `/proc`,
which Yama does not restrict.

**Must never ask for.** `sudo`, `pkexec`, a polkit action, a system service, a
udev rule, or any write outside the home directory. Nothing installs into
`/usr` — that path is what the updater reads to conclude a package manager owns
the copy and to refuse replacing it.

**Degradation.** No Secret Service (`org.freedesktop.secrets`): every keychain
call returns an error rather than `NoEntry`, the owner-key probe answers
"contributor session", provider credentials refuse by name, and startup is
unaffected — see `steamdeck.md` §B, which verifies this on the platform where
it is the default state.

## 3. macOS

**Needs.** `~/Library/Application Support/taarib` and the settings root;
outbound HTTPS (`com.apple.security.network.client`); the login keychain
(`keychain-access-groups`); reads and writes inside user-selected game folders;
the same process-name read.

**Must never ask for.** Full Disk Access, Accessibility, Screen Recording,
Automation/Apple Events, or admin rights. The `.app` installs by drag; nothing
writes outside the user's home.

**Degradation.** SIP strips `DYLD_INSERT_LIBRARIES` for hardened or platform
binaries, so injection into such a game refuses by name and the overlay tier
remains — it reaches the process through the graphics API rather than a
preload. See `macos.md` §2. A keychain the user declines to unlock is the same
named refusal as an absent Secret Service on Linux.

## 4. Portable mode

`taarib.mahmul` beside the executable (or beside the outer AppImage) is the
user saying *leave this machine untouched*: the data and settings roots move to
`bayanat/` beside the marker, and no machine-global write is permitted —
no keychain entry, no registry value, nothing under `~/.config`.

`Masarat::iktashif` resolves the marker and `Masarat::mahmul()` reports it
(`crates/taarib-usus/src/masarat.rs`), and the module's own documentation is
explicit that enforcement belongs to the callers that can name their refusals.

**Enforced today:** `khzin_itimad_muzawwid`
(`apps/studio/src-tauri/src/idadat_awamir.rs`) consults `masarat.mahmul()` and
refuses with `TAARIB-E-9095` — "this is a portable installation, which writes
nothing into this machine's keychain" — before the credential reaches the
platform store. Deleting a credential is *not* gated: removing something from a
machine leaves it cleaner than it was, which is the direction the marker asks
for.

**Recorded limitation:** the contributor identity key is still minted into the
machine keychain on first submission (`taqdeem_awamir.rs`). Refusing it would
make submission impossible from a portable copy rather than merely local, which
is a product decision about where a portable contributor's identity should live
— not a bug to be silently patched. It is named here and left to the owner.
