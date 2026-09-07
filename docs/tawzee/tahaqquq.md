# التحقّق — what was actually built, installed and run

Phase 28. The companion files in this directory (`windows.md`, `linux.md`,
`macos.md`, `steamdeck.md`, `adhonat.md`) describe what the packaging *should*
do, derived from the contract and from the Tauri sources. This file records what
happened when the artifacts were built and run on real hardware on
**2026-09-04**, what the two rebuilds of **2026-09-05** changed (§4.1a, §4.5,
§4.6), and states plainly which targets could not be reached from that machine
at all.

Nothing here is inferred from a config file. Every "yes" below has a
reproduction in §6.

> **Overtaken since it was written — re-read against the tree on 2026-09-06.**
> The measurements below are the measurements of the builds they name, and they
> are left as recorded. What has changed: the `mawarid/` tree that §1 and §5.1
> describe as holding only its README has since been fully staged by
> `scripts/isdar.sh` (1,598 files and a written manifest, Windows target,
> 2026-09-05), and an installer built from it — about 66 MB against the
> 10,590,543 bytes measured here — exists and has **not** been through this
> audit; and §5.1's "nothing in this tree enforces" that staging precedes
> bundling is no longer wholly true, because `tauri.conf.json`'s
> `beforeBundleCommand` now runs `src-tauri/tadqiq_mawarid.mjs`, which refuses a
> bundle whose fonts are missing or unshapeable (fonts only — an absent component
> is still named at startup and refused at install, not at bundle time), and
> `scripts/isdar.sh` runs the whole sequence in order.

## 1. The answer, first

> A user downloads one file and runs it. Does the application start, in Arabic,
> with its own fonts, having never seen a compiler?

**Windows: yes, verified end to end.** The installer runs without a UAC prompt,
the application opens in 0.5 s, the interface is Arabic and right-to-left, the
eight bundled IBM Plex faces are served by the application to its own webview
byte-for-byte, and the only non-OS library the process loads is Microsoft's own
WebView2 runtime. No Visual C++ redistributable is involved anywhere.

**Linux: yes, once the build moved off this machine.** Both bundles build. The
AppImage runs cold, resolves 148 of its libraries from inside its own mount, and
the bundled Arabic faces are provably present in the WebKit web process at
runtime. The AppImage as built on *this* machine required **glibc 2.42**, newer
than any current stable distribution — §4.1 — which was a build-host problem
rather than a packaging one and was the single thing standing between the Linux
artifact and the sentence above. **Fixed the next day** by building in an Ubuntu
22.04 container: the floor is `GLIBC_2.35`, and both artifacts were installed
and run on hosts where the 2026-09-04 pair dies at the loader (§4.1a, §4.5).
The desktop-integration pass of §4.6 then made a double-clicked `.ruqaa` reach
the process with its path, on the `.deb` path.

**macOS: unreachable.** No Apple hardware and no macOS VM exists here; the
`.app` and `.dmg` bundlers only run on macOS. Nothing about them was verified.

**Steam Deck: unreachable.** No Deck and no SteamOS image exists here. The
AppImage that a Deck would run was built and tested on Linux, but its glibc
floor (§4.1) means the copy built today would not start on one.

**And one thing that is true on every target that was reached:** the shipped
`mawarid/` tree contains only its README. The component store is empty, so the
application starts and the library works, but every patch install would refuse
each component by name. See §5.1.

## 2. The table

| target | built | installed | cold start | fonts verified |
| --- | --- | --- | --- | --- |
| **Windows x64** — `taarib-studio.exe` | yes | n/a (loose binary) | yes, 417 ms to window | yes, all 8 faces, byte-exact |
| **Windows x64** — `Taarib_1.0.0_x64-setup.exe` | yes | yes, silent, 3.9 s, no UAC | yes, 509 ms from `%LOCALAPPDATA%` | yes, all 8 faces, byte-exact |
| **Linux x64** — `Taarib_1.0.0_amd64.AppImage` | yes | n/a (no install step) | yes | yes, 5 of 8 faces found in the web process; the other 3 are unused on the first screen |
| **Linux x64** — `Taarib_1.0.0_amd64.deb` | yes | ~~no — needs `sudo`~~ **yes**, in a clean `ubuntu:22.04` container (§4.5, §4.6) | not run from `/usr` | 25 `.ttf` present under `usr/lib/Taarib/mawarid/khutut/` (§4.5) |
| **macOS aarch64 / x64** — `.app`, `.dmg` | **unreachable** | **unreachable** | **unreachable** | **unreachable** |
| **Steam Deck** — AppImage on SteamOS | **unreachable** | **unreachable** | **unreachable** | **unreachable** |

"Unreachable" means no hardware, VM or emulator for that platform exists on the
machine this was run from, and no substitute was accepted. It does not mean
"probably fine".

Verification host: Windows 11 build 10.0.26200.9168, and WSL2 Kali (glibc 2.42,
GTK 3.24.52, WebKitGTK 4.1 / 2.52.4) on the same machine.

## 3. Windows

### 3.1 The binary

`target/release/taarib-studio.exe`, 35,800,576 bytes,
`sha256 7b99b55ed250a7742829cf136a74dc9118312e1cc4b5c4ea2519d7328d53feaa`.

Header facts, read straight out of the PE:

| | |
| --- | --- |
| machine | `0x8664`, PE32+ — a real x64 image |
| subsystem | `WINDOWS_GUI` (2) — no console window behind the app, as `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` intends |
| DLL characteristics | `HIGH_ENTROPY_VA`, `DYNAMIC_BASE` (ASLR), `NX_COMPAT` (DEP), `TERMINAL_SERVER_AWARE` |
| Authenticode | absent — the build is unsigned, as `windows.md` says it is |
| resources | 9 icons + `GROUP_ICON`, `VERSION` (`CompanyName=cc1a2b`, `FileDescription=Taarib`, `FileVersion=1.0.0`, `ProductName=Taarib`), and a `MANIFEST` |

Two header notes worth recording rather than fixing here:

- **Control Flow Guard is not enabled** (`GUARD_CF` is clear in the DLL
  characteristics). Rust does not turn it on by default on `windows-msvc`; it is
  a `-C control-flow-guard=yes` decision, and it is a decision nobody has made.
- **The embedded manifest declares only the Common-Controls v6 dependency** — no
  `requestedExecutionLevel`, no `dpiAware`/`dpiAwareness`, no `longPathAware`.
  The first is harmless (`asInvoker` is the default). The second is also
  harmless *in practice*: DPI awareness is set at runtime instead, through
  `SetProcessDpiAwarenessContext` / `SetProcessDpiAwareness` /
  `shcore!GetDpiForMonitor`, all resolved by name at startup — the strings are in
  the binary and `IsProcessDPIAware` is imported. The third means paths longer
  than `MAX_PATH` still need the `\\?\` prefix, which the product already uses
  (it appears in the diagnostics log).

### 3.2 What it actually depends on

This is the question the brief asks: *does it need the Visual Studio
redistributables a developer has?* It does not, and there are two independent
proofs.

**Static.** The import table names 32 DLLs. Every one of them is either a
Windows system DLL (`kernel32`, `user32`, `gdi32`, `advapi32`, `ole32`,
`oleaut32`, `shell32`, `shlwapi`, `comctl32`, `dwmapi`, `ntdll`, `psapi`,
`pdh`, `powrprof`, `rpcrt4`, `bcrypt`, `bcryptprimitives`, `crypt32`,
`winhttp`, `ws2_32`) or a Universal CRT API set (`api-ms-win-crt-*`,
`api-ms-win-core-synch-l1-2-0`), which the OS resolves to `ucrtbase.dll` in
System32 through the API-set schema in `ntdll`. **There is no
`vcruntime140.dll`, no `vcruntime140_1.dll`, no `msvcp140.dll`, no `msvcr*.dll`
— and no delay-load imports at all.** The strings `vcruntime`, `msvcp`, `msvcr`
do not occur anywhere in the file.

**Empirical.** With the application running, its loaded-module list is 59
entries: 57 under `C:\Windows`, and exactly two elsewhere —

```
C:\Users\cc1a2b\AppData\Local\Taarib\taarib-studio.exe
C:\Program Files (x86)\Microsoft\EdgeWebView\Application\152.0.4191.62\EBWebView\x64\EmbeddedBrowserWebView.dll
```

itself, and Microsoft's WebView2 runtime. Nothing from a Rust toolchain, an
MSVC install, a `Program Files\Microsoft Visual Studio` directory or a
side-by-side assembly cache. The same two-entry result came from the loose
`target/release` binary and from the installed copy, so it is a property of the
binary and not of where it sits.

`WebView2Loader.dll` is **not** shipped beside the executable and **not**
imported: `webview2-com` 0.38.2 is linked statically, and wry's own runtime
probe (the strings `WebView2: Failed to find an installed WebView2 runtime…`,
`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`) does the discovery. So the WebView2
runtime is the one external requirement, and the installer's
`downloadBootstrapper` exists for exactly that. On the verification host it was
already present: `HKLM\…\EdgeUpdate\Clients\{F3017226-…}` `pv = 152.0.4191.62`,
which is the probe `windows.md` documents, so the bootstrapper section was a
no-op — that path was **not** exercised and remains unverified.

### 3.3 Cold start

Loose binary: window handle 417 ms after `CreateProcess`; native window title
`تعريب` (`U+062A U+0639 U+0631 U+064A U+0628`), i.e. the `app.windows[0].title`
from the config, correct and un-mangled. Six WebView2 child processes appear
under it (browser, crashpad-handler, gpu-process, two utility, renderer), so the
webview genuinely came up rather than the window merely existing.

Installed copy from `%LOCALAPPDATA%\Taarib`: window at 509 ms, same child set,
same module list.

The first screen is المكتبة with 16 real games discovered, cover art, the
Arabic sidebar, and the status bar reading `1.0.0 · ويندوز · x86-64` beside the
data-root path. It is right-to-left, dark, and Arabic without any user action.

### 3.4 The fonts — proved, not assumed

The eight faces in `apps/studio/src/khutut/` are emitted by Vite as eight
content-hashed files under `dist/assets/`, and Tauri compiles that directory
into the binary. Three layers were checked, each one stronger than the last.

1. **Embedded.** Each of the eight hashed filenames occurs exactly once as a
   string in `taarib-studio.exe` — they are the asset-store keys.
2. **Served.** With the application running, the page was asked to `fetch()`
   every `url()` in every `CSSFontFaceRule` it has. All eight returned
   `200 application/font-sfnt` from the app's own protocol
   (`http://tauri.localhost/assets/…`), with sfnt magic `0x00010000` and byte
   lengths of 236708 / 243028 / 245544 / 200500 / 202460 / 202632 / 155940 /
   156996. An FNV-1a-32 digest computed over each response inside the page
   matches the digest of the corresponding file on disk for **all eight** —
   the bytes the webview receives are the bytes that were bundled, unmodified.
3. **Loaded and used.** `await f.load()` on every `FontFace` resolved without
   error for all eight; `document.fonts.status` became `loaded`. Canvas
   `measureText` on the string `تعريب الألعاب ABC 123` gives 605.57 px at
   `IBM Plex Sans Arabic` and 633.60 px at `IBM Plex Mono`, against a serif
   fallback baseline of 576.20 px and a monospace baseline of 580.59 px — both
   differ from both fallbacks, so those faces are doing the shaping. A control
   family that does not exist measures at exactly the serif baseline, which is
   what a fall-through looks like.

`document.body`'s computed `font-family` is
`"IBM Plex Sans Arabic", "IBM Plex Sans", sans-serif`, `direction: rtl`,
`documentElement.dir = "rtl"`, `lang = "ar"`.

One caution for anyone repeating this: **`document.fonts.check()` is not a
usable test here.** It returns `true` for a family with no matching
`@font-face` rule at all — the control family `"Definitely Not Installed XYZ"`
answered `true`. Only the metric comparison discriminates.

The instrument was WebView2's own DevTools protocol, enabled without rebuilding
by setting `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=…`
before launch. The `devtools` Cargo feature is deliberately absent from the
release build (`Cargo.toml`, the `[workspace.dependencies.tauri]` comment), and
this technique does not turn it on: it is a WebView2 runtime switch, outside
Tauri, and it changes nothing about the shipped binary.

### 3.5 The installer

`target/release/bundle/nsis/Taarib_1.0.0_x64-setup.exe`, 10,590,543 bytes,
`sha256 069819a71325ee8a922bdf371fc2f2356d97a4218451bc166c116a63f2871407`. It is
a 32-bit x86 PE, which is normal and not a defect — `makensis` produces x86
stubs regardless of payload, and the PE timestamp (2025-03-08) is the stub's,
not the build's.

`Taarib_1.0.0_x64-setup.exe /S` → **exit 0 in 3,875 ms, no UAC prompt**, from a
machine with no previous install (no uninstall key, no `Software\cc1a2b` key).

What landed:

```
%LOCALAPPDATA%\Taarib\
├─ taarib-studio.exe   35,800,576
├─ uninstall.exe           79,169
└─ mawarid\README.md          790      ← and nothing else; see §5.1
```

Registry, all under HKCU, exactly as `windows.md` §"Registry" describes:
`…\Uninstall\Taarib` with `DisplayName`, `DisplayIcon`, `DisplayVersion 1.0.0`,
`Publisher cc1a2b`, `InstallLocation`, `UninstallString`, `MainBinaryName`,
`NoModify`/`NoRepair`, `EstimatedSize 35039`, and the three URL values; and
`HKCU\Software\cc1a2b\Taarib` = the install directory. Nothing under HKLM.

Shortcuts: `…\Start Menu\Programs\Taarib.lnk` and `Desktop\Taarib.lnk`, both
targeting the installed exe with the install directory as working directory.

**File associations are written, and `windows.md` said they were not.** The
installer creates `HKCU\Software\Classes\.ruqaa` → `Taarib.Ruqaa`, and the
`Taarib.Ruqaa` ProgID with `DefaultIcon = "<instdir>\taarib-studio.exe,0"` and
`shell\open\command = "<instdir>\taarib-studio.exe \"%1\""`. That is the
`bundle.fileAssociations` block being rendered through the qalab's
`APP_ASSOCIATE` at `nsis/qalab.nsi:676`. The claim in `windows.md` has been
corrected.

### 3.6 Uninstall

Two runs, in this order.

**With the application running.** `uninstall.exe /S` returned 0 and removed
nothing — the executable, the registry and the shortcuts were all still there
afterwards, and the running application was untouched. That is deviation 3 of
the qalab (`windows.md`), and it works.

**With the application closed.** `uninstall.exe /S` finished in about one
second. `%LOCALAPPDATA%\Taarib` is gone entirely — no leftover files, no empty
directories. The uninstall key, both shortcuts and the `Taarib.Ruqaa` ProgID
are gone.

The `--istiada` gate ran and did the right thing. `sijillat/istiada_uninstall.log`:

```
تعريب 1.0.0 — عرض الاستعادة قبل الإزالة (2026-09-04T18:56:58.480Z)
لا توجد أي لعبة عليها محتوى مثبَّت من تعريب؛ لا شيء يحتاج إلى استعادة قبل إزالة التطبيق.
```

No game carried Taarib content, so nothing was restored, no dialog appeared, and
no game directory was written to. Note what this means for coverage: **the
restore path itself was exercised only in its empty case.** A library with an
installed patch would take the branch that shows the Arabic Yes/No dialog and
then writes into game directories, and that branch is still unverified.

User data survives, as the contract requires: `%APPDATA%\Taarib` is intact (it
gained the istiada log), and `%LOCALAPPDATA%\com.cc1a2b.taarib` — the WebView2
profile — is intact.

**One leftover.** `HKCU\Software\Classes\.ruqaa` survives the uninstall, with an
empty default value and a stale `Taarib.Ruqaa_backup` value beside it. This is
stock `FileAssociation.nsh` behaviour: `APP_UNASSOCIATE` restores the saved
previous handler, and when there was none it writes an empty string back instead
of deleting the key. Harmless — an empty default means no handler — but it is a
key the uninstaller leaves behind, and `windows.md` should not claim a
registry-clean uninstall without naming it. `HKCU\Software\cc1a2b\Taarib` also
survives, which *is* documented and intentional.

## 4. Linux

Built with the workspace's own CLI (`@tauri-apps/cli` 2.11.4) against
`tauri` 2.11.5, on WSL2 Kali. Release compile 9 min 31 s, then both bundles.
`bundle.targets` also lists `nsis`, `app` and `dmg`; the CLI skipped all three
silently on Linux — no warning, no error, exit 0, "Finished 2 bundles".

| artifact | bytes | sha256 |
| --- | --- | --- |
| `Taarib_1.0.0_amd64.deb` | 17,861,680 | `781a68ce2a61bb244dc25718a1be8dbb87ac75677c86de9891e852bac5d0682b` |
| `Taarib_1.0.0_amd64.AppImage` | 117,840,376 | `b01eb5bb4fac0f4f4b5938d2423eaefca5b8b5c8f8d4da85e130bbc566370cd5` |

**Every `bundle.linux.deb.files` mapping resolved.** All ten sources exist —
`linux/taarib.desktop`, `linux/application-vnd.taarib.ruqaa.xml`, the seven
`ayqunat/ruqaa/*.png` and `ayqunat/ruqaa/taarib-ruqaa.svg` — and all ten landed
in the package. The `.ruqaa` document icon added recently does not break the
bundle.

### 4.1 The glibc floor — the blocker

The AppImage bundles 187 shared objects and the WebKitGTK stack, and at runtime
it used them: of the shared objects mapped into the running process, **148 came
from inside the AppImage mount** (`libwebkit2gtk-4.1`, `libjavascriptcoregtk-4.1`,
`libgtk-3`, `libgdk-3`, `libsoup-3.0`, and their transitive closure) and 48 came
from the host, all of them the graphics and driver stack (`libGL`, `libEGL`,
mesa) plus libc itself. That is exactly the split an AppImage is supposed to
have.

But an AppImage never bundles glibc or libstdc++, so the host's copies must be
new enough for everything inside. Measuring the highest versioned **undefined**
symbol across the binary and all 187 bundled libraries:

| | floor |
| --- | --- |
| `taarib-studio` itself | `GLIBC_2.39` (`pidfd_getpid`, `pidfd_spawnp`) |
| bundled libraries | **`GLIBC_2.42`** — one symbol, `__inet_pton_chk`, in `libwebkit2gtk-4.1.so.0` |
| bundled libraries | `GLIBCXX_3.4.30`, `CXXABI_1.3.15`, `GCC_13.0.0` from the host `libstdc++` |

The build host's own glibc is 2.42, so the AppImage inherited its floor from the
machine it was built on. **A host with an older glibc will refuse to start it**
with `version 'GLIBC_2.42' not found`, before any Taarib code runs. glibc 2.42
is newer than what current stable distributions ship, and newer than any SteamOS
release, so as built today this AppImage runs on almost nothing but its own
build host.

This is a build-environment defect, not a packaging one, and the fix is the
standard AppImage discipline that `steamdeck.md` §5 already flagged as an open
question: build the Linux artifact in an old-baseline container (Ubuntu 22.04
gives `GLIBC_2.35`; 24.04 gives `2.39`) rather than on a rolling distribution.
It needs an owner and a CI image, and there is no CI image defined anywhere in
this tree.

The `.deb` is not affected in the same way — `Depends: libwebkit2gtk-4.1-0`
means the *host's* WebKit is used, and dpkg refuses the install rather than
producing a broken one — but the binary's own `GLIBC_2.39` floor still applies,
and the package declares no `libc6 (>= 2.39)` because the bundler generates
dependencies from the shared-library needs it can see, not from symbol versions.

### 4.1a The floor, re-measured and fixed — 2026-09-05

Three things were wrong or missing in §4.1, and the fix now exists.

**The measurement is now a script, not a note.**
`apps/studio/src-tauri/linux/qias_qaa.sh` walks every ELF under a built tree,
takes the versioned undefined symbols, and folds away any family the tree
itself defines (a bundled `libmount` providing `MOUNT_2`, say) so that what is
printed is only what the *host* must own. Given `--haddu-aqsa GLIBC_2.35` it
exits non-zero when the floor moves. `ibni.sh` runs it on every build.

Two traps it had to be written around, both of which produce a wrong answer:

- a **copy relocation** — `stderr@GLIBC_2.2.5` appears as a *defined* symbol in
  `AppRun.wrapped` — makes `.dynsym` look as though the tree provides GLIBC.
  Definitions are therefore read from `.gnu.version_d`, not from `.dynsym`;
- `readelf` spells the header `Version definition section`, singular.

**Two host requirements §4.1 never listed.** Beyond glibc and libstdc++ the
image needs `ALSA_0.9`, `GPG_ERROR_1.0` and `ZLIB_1.2.3.4` from the host, and
twenty-three shared objects with no copy in the tree at all — `linux.md` §1 has
the table. Present on any desktop, but "the AppImage bundles all dependencies"
is not exactly true and the difference is now enumerated rather than assumed.
(Both numbers are the rolling-host build's. The container build carries no
`libflite`, so `ALSA_0.9` and `libasound.so.2` drop out and the set is
twenty-two — §4.6.)

**The `WEAK` symbols are not optional, and the `.deb` is not immune.**
`pidfd_getpid` and `pidfd_spawnp` are `WEAK` undefined symbols, which reads like
a reference the loader may skip. The loader gates on the version-*need* entry,
not the symbol, and `readelf -V` shows `Name: GLIBC_2.39  Flags: none`.
Demonstrated by running the 2026-09-04 binary on `ubuntu:22.04` (glibc 2.35)
with that host's own `libwebkit2gtk-4.1` installed, so nothing bundled is in
play:

```
taarib-studio: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found
taarib-studio: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

§4.1's closing paragraph said the `.deb` is not affected in the same way. It is:
`Depends:` is generated from `DT_NEEDED` names, never from symbol versions, so
the package installs cleanly on a glibc-2.36 host and the program then fails to
start. The failure is later and less legible, not absent.

**What SteamOS ships**, from Valve's own mirror
(`steamdeck-packages.steamos.cloud/archlinux-mirror/core-<channel>/os/x86_64/`):
3.5 → 2.37, 3.6 → 2.39, 3.7 → 2.40, 3.8.1x → 2.41. All below 2.42.

**The fix.** `apps/studio/src-tauri/linux/Dockerfile` (Ubuntu 22.04, glibc
2.35) plus `ibni.sh`. 22.04 is the newest base below every SteamOS release that
still carries a maintained `libwebkit2gtk-4.1` — `2.50.4-0ubuntu0.22.04.1`, so
nothing is given up by dropping back. `GLIBC_2.35` is the product's Linux floor
and the script asserts it.

### 4.2 The AppImage runs

Launched with no arguments on a host with `libfuse3` but **no `libfuse2`**, the
type-2 runtime mounted itself at `/tmp/.mount_Taarib…` and the application
started. `--appimage-extract-and-run` was not needed here; `linux.md` §1 keeps
it as the documented fallback, which is still right for hosts that do refuse.

Startup log, verbatim shape:

```
INFO taarib_studio Taarib Studio starting bayanat=~/.local/share/taarib
     idadat=~/.config/taarib/idadat.json isdar=1.0.0 mimariya=X8664 nizam=Linux
```

so the data root is `~/.local/share/taarib` and settings `~/.config/taarib`,
matching `steamdeck.md` A4/A5. The library scan found 0 games, which is correct
on a machine with no Steam install, and it wrote nothing outside the home
directory.

`resource_dir()` resolved to **`${APPDIR}/usr/lib/Taarib/mawarid`** — with the
product name, not the binary name. `linux.md` §4 said
`/usr/lib/taarib-studio/mawarid`; that was taken from Tauri's own doc comment,
which is stale. The code (`tauri-utils-2.9.3/src/platform.rs`,
`resource_dir_from`) formats `/usr/lib/{package_info.name}`, and `name` is the
`productName`. The bundler agrees with itself — the `.deb` also installs to
`/usr/lib/Taarib/` — so the two match and nothing is broken. `linux.md` has been
corrected.

### 4.3 The fonts, on WebKit

WebView2's DevTools protocol has no equivalent here: WebKitGTK's
`WEBKIT_INSPECTOR_SERVER` does start a listener (it was confirmed listening on
the port) but it answers no HTTP request and no WebSocket upgrade from a plain
client, so it was abandoned.

Instead, the proof is direct. The eight font files are embedded in the ELF (each
hashed filename occurs exactly once). If a face is really being served,
decompressed and parsed, its *decompressed* bytes must exist in the WebKit web
process. So: launch the application as a child process (Yama
`ptrace_scope = 1` permits reading a descendant), find `WebKitWebProcess`, and
search its anonymous mappings for a 64-byte needle taken from the middle of each
`.ttf`, plus one needle of random bytes as a false-positive control.

Result, over 1,017 MiB of anonymous memory:

```
FOUND      IBMPlexSansArabic-Regular-CjFS_rN4.ttf
FOUND      IBMPlexSansArabic-Medium-BQ11Nkav.ttf
FOUND      IBMPlexSansArabic-SemiBold-BdIziXfN.ttf
FOUND      IBMPlexSans-Regular-Bl2SjS7V.ttf
FOUND      IBMPlexMono-Regular-B6Yluqzl.ttf
not found  IBMPlexSans-Medium-DV05E5xX.ttf
not found  IBMPlexSans-SemiBold-B9auKknr.ttf
not found  IBMPlexMono-Medium-BHIZOIaS.ttf
not found  CONTROL(random)
```

The control needle was not found, so the method has no false positives. The
three faces that were not found are precisely the three that the first screen
never uses — on Windows, where each `FontFace` could be interrogated directly,
those same three were still `unloaded` until forced. Both platforms agree: the
bundled Arabic faces reach the renderer, and the unused Latin weights are
lazily deferred.

### 4.4 The `.deb`, structurally

`Package: taarib`, `Version: 1.0.0`, `Architecture: amd64`,
`Installed-Size: 41116`, maintainer `cc1a2b <cc1a2b@users.noreply.github.com>`,
Arabic `Description`. `md5sums` present; no maintainer scripts, which is correct
— `shared-mime-info`, `desktop-file-utils` and `hicolor-icon-theme` each own a
dpkg trigger on the directory this package writes into, so
`update-mime-database`, `update-desktop-database` and the icon cache all run on
install without a `postinst`. That is exactly why those three are in `depends`.

Three things are wrong with it, all reported in §5, and one of them turned out
not to be cosmetic. Two are **fixed as of 2026-09-05**; what follows is the
2026-09-04 record, not the current state — §4.6 and §5.4 are.

- `Depends:` lists `libwebkit2gtk-4.1-0` and `libgtk-3-0` **twice** — once from
  the configured `depends`, once from the bundler's generated set.
- The package ships **two** desktop entries. `com.cc1a2b.taarib.desktop` is the
  Arabic one from `bundle.linux.deb.files`; `Taarib.desktop` is the bundler's
  own, written unconditionally, with `Name=Taarib` and no `%U`. Both land in
  `/usr/share/applications/`, so the application menu gets two entries for one
  program.
- The Arabic entry says `Icon=taarib`, but the package installs its application
  icons as `hicolor/*/apps/taarib-studio.png`. The Arabic entry therefore has no
  icon; the bundler's English one, which says `Icon=taarib-studio`, does.

`linux.md` §3's claim that "the AppImage carries the same entry internally … so
the two must not drift" is not correct and cannot be made correct:
`bundle.linux.deb.files` is a `.deb`-only mechanism, and the AppImage's
`Taarib.desktop` is the bundler's, with the English name and no `%U`. That
section has been corrected.

Both `.desktop` files pass `desktop-file-validate` (one hint about
`Categories=Utility;Game;` containing two main categories).

## 4.5 The old-baseline build, measured — 2026-09-05

Built with `apps/studio/src-tauri/linux/ibni.sh` on `ubuntu:22.04`;
`Taarib_1.0.0_amd64.AppImage` (98 MB) and `Taarib_1.0.0_amd64.deb` (22 MB) in
`dist-linux/`.

| | 2026-09-04, rolling host | 2026-09-05, container |
| --- | --- | --- |
| GLIBC | **2.42** | **2.35** |
| GLIBCXX | 3.4.30 | 3.4.30 |
| CXXABI | 1.3.15 | **1.3.9** |
| GCC | 13.0.0 | **7.0.0** |
| ZLIB | 1.2.3.4 | 1.2.9 |

`GLIBCXX_3.4.30` does not move, because it is what the backported jammy WebKit
was itself built against. It is satisfied by any libstdc++ from GCC 12 onward,
which Ubuntu 22.04 ships and every SteamOS release far exceeds.

### The A/B, which is the actual proof

Both images extracted and started under `xvfb` in stock containers, with the
host libraries of §1 installed and nothing else:

| host | 2026-09-04 image | 2026-09-05 image |
| --- | --- | --- |
| `ubuntu:22.04`, glibc 2.35 | `libc.so.6: version 'GLIBC_2.38' not found` / `'GLIBC_2.39' not found` (the binary itself) | **starts** |
| `ubuntu:24.04`, glibc 2.39 | `libc.so.6: version 'GLIBC_2.42' not found (required by libwebkit2gtk-4.1.so.0)` | **starts** |

"Starts" means the loader resolved everything and the program reached its own
startup log — schema migration, path resolution, library scan — not that a
window was drawn, which `xvfb` cannot honestly assess.

### What that startup shows, on a machine that has never run it

A fresh user account on `ubuntu:22.04`, no Steam, no configuration:

```
INFO taarib_makhzan::hijra   schema migration applied hijra=1 ism=al-asas
INFO taarib_makhzan::wasl    database schema brought forward ila=1 min=0
INFO taarib_studio           Taarib Studio starting bayanat=~/.local/share/taarib
                             idadat=~/.config/taarib/idadat.json …
INFO taarib_studio           the library scan finished alaab=0 bi_ghilaf=0 ghaiba=0
                             muddat_ms=0 mutaadhira=0
```

- **The store is created and valid.** Twelve directories under
  `~/.local/share/taarib` — `dhakira`, `hajr`, `khutut`, `mafatih`, `makhbaa`,
  `mashari`, `mukawwinat`, `nusakh`, `ruqaa`, `sandooq`, `sijillat` — plus the
  database and its WAL, plus `~/.config/taarib/`. `idadat.json` is **not**
  written until a setting changes, which is `Idadat::iftah`'s documented
  behaviour (`crates/taarib-usus/src/idadat.rs:650-664`), not a failure: the
  in-memory value is a valid schema-2 store and the file is stamped
  `"mukhattat": 2` on first write.
- **The scan degrades to nothing, quietly and correctly.** `alaab=0`,
  `mutaadhira=0` — no error, no warning, no panic, on a machine with none of the
  ten launchers installed.
- **The fonts are in the bundle and the path resolves.** 25 `.ttf` under
  `usr/lib/Taarib/mawarid/khutut/{kufi,latin,naskh,sans}` in both artifacts,
  which is exactly what `mukawwinat_tahmil::judhur_khutut` builds from
  `resource_dir()`. §5.1's "the shipped `mawarid/` is empty" is now half true:
  the fonts are staged, `bayan_mukawwinat.json` still is not, so the component
  mirror still reports one problem and framework-tier installs still refuse.

### A twenty-fourth host library the symbol analysis cannot see

Without `libGLESv2.so.2` present, the process does not degrade — it prints
`Couldn't open libGLESv2.so.2` and **aborts (core dumped)** after the startup
log. It is absent from every `DT_NEEDED` in the tree, so `qias_qaa.sh` could not
list it and neither could any relocation-based analysis. It is present on any
machine with a working GPU stack, SteamOS included. The lesson for the §1 table
is that it enumerates the *link-time* contract and a `dlopen` can still be
waiting behind it.

`WEBKIT_DISABLE_COMPOSITING_MODE=1` gets past it, which is worth knowing for
headless smoke tests but is not something to ship.

**Followed up 2026-09-05, and one sentence above was wrong.** `qias_qaa.sh` now
reads the ELF string tables as well: a name shaped like a soname, carried inside
an ELF in the tree, matching no `DT_NEEDED` and no file the tree carries, is
something the code intends to open by hand. Run against the AppDir it reports
ten such names, and the four that matter are all opened by the **bundled
libepoxy**, not by WebKit directly:

```
REACHED BY NAME AT RUN TIME — a soname in a string table, in no DT_NEEDED
  libGLESv1_CM.so.1        /usr/lib/libepoxy.so.0
  libGLESv2.so.2           /usr/lib/libepoxy.so.0
  libGLX.so.1              /usr/lib/libepoxy.so.0
  libOpenGL.so.0           /usr/lib/libepoxy.so.0
  libcryptsetup.so.12      /usr/lib/libmount.so.1
  libdebuginfod.so.1       /usr/lib/libdw.so.1
  libnss_mdns.so.2         /usr/lib/libavahi-client.so.3
  libnss_mdns4.so.2        /usr/lib/libavahi-client.so.3
  libnss_mdns6.so.2        /usr/lib/libavahi-client.so.3
  libsepol.so.2            /usr/lib/libselinux.so.1
```

It is evidence rather than proof — a soname in a string table can also be a
diagnostic message — so the section reports and never gates. The decision taken
on the strength of it: the `.deb` **depends on `libgles2`** (libglvnd's package,
which owns `/usr/lib/x86_64-linux-gnu/libGLESv2.so.2` on both Debian and Ubuntu;
confirmed present on `ubuntu:22.04` as version 1.4.0-1), `ibni.sh` fails the
build if the scan names the library and the package does not depend on it, and
the AppImage documents it as a hard host requirement because that format has no
way to state a dependency. It is deliberately **not bundled**: on a glvnd system
this is the dispatch layer that finds the user's vendor driver, exactly like
`libEGL.so.1` and `libGL.so.1`, which the §1 table already excludes for that
reason. `linux.md` §1 carries the full argument.

## 4.6 The desktop-integration pass, measured — 2026-09-05

A second Linux build the same day, from `ibni.sh` with the `bundle.linux`
subtree of `tauri.conf.json` changed: `Taarib_1.0.0_amd64.AppImage` 98,413,048
bytes and `Taarib_1.0.0_amd64.deb` 22,807,002 bytes, against 98,064,888 and
22,483,174 for the morning's build.

### The floor did not move

| | 2026-09-05 morning | 2026-09-05 second pass |
| --- | --- | --- |
| GLIBC | 2.35 | **2.35** |
| GLIBCXX | 3.4.30 | 3.4.30 |
| CXXABI | 1.3.9 | 1.3.9 |
| GCC | 7.0.0 | 7.0.0 |
| ZLIB | 1.2.9 | 1.2.9 |
| host sonames with no copy in the tree | — | **22**, listed in `linux.md` §1 |

Which is the expected answer and is stated because it was measured:
`appimage.files` adds ten files and not one of them is an ELF, so it has nothing
to contribute to a symbol floor. `ibni.sh` runs
`qias_qaa.sh … --haddu-aqsa GLIBC_2.35` over the AppDir the bundler produced and
printed `ceiling GLIBC_2.35 held`.

The `22` corrects a number this file and `linux.md` both carried: the host set
was twenty-three on the 2026-09-04 rolling-host build, including
`libasound.so.2` reached through a bundled `libflite`. The jammy image pulls in
no `libflite`, so nothing links `libasound` and `ALSA_0.9` is gone from the
floor as well. The old number had been carried forward rather than re-derived.

### The AppDir, after the file map

Ten files, all present, and the desktop entry replaced rather than doubled:

```
usr/share/applications/Taarib.desktop                                  (ours)
usr/share/mime/packages/application-vnd.taarib.ruqaa.xml
usr/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256}/
    mimetypes/application-vnd.taarib.ruqaa.png
usr/share/icons/hicolor/scalable/mimetypes/application-vnd.taarib.ruqaa.svg
```

and `Taarib.AppDir/Taarib.desktop`, the symlink the AppImage advertises, still
resolves to `usr/share/applications/Taarib.desktop` — now the Arabic entry with
`Exec=taarib-studio %U`.

`%U` reaching the process was measured rather than assumed, against the
`AppRun.wrapped` the bundler ships: an AppDir whose only payload prints its own
arguments, run as `./AppRun /home/user/some patch.ruqaa`, reports `ARGC=2` with
both arguments verbatim. AppRun takes the first token of `Exec=` and forwards
the rest of its own command line; the field code is dropped and the paths are
not.

### The `.deb`, installed for real on a clean `ubuntu:22.04`

```
Depends: libwebkit2gtk-4.1-0, libgtk-3-0, libgles2, shared-mime-info,
         desktop-file-utils, hicolor-icon-theme, libwebkit2gtk-4.1-0, libgtk-3-0
control tarball: control md5sums          (still no maintainer scripts)
usr/share/applications/Taarib.desktop     (one entry, 2331 bytes)
usr/share/mime/packages/application-vnd.taarib.ruqaa.xml
```

After `apt-get install ./Taarib_1.0.0_amd64.deb`, apt pulled `libgles2` and
`dpkg -S /usr/lib/x86_64-linux-gnu/libGLESv2.so.2` answers `libgles2:amd64`, so
the abort of §4.5 cannot happen on this path any more. The registration chain
is in §5.4.

### The AppImage still starts

Extracted and run under `xvfb` as a fresh user on a stock `ubuntu:22.04`
(glibc 2.35) carrying only the §1 host libraries:

```
INFO taarib_makhzan::hijra   schema migration applied hijra=1 ism=al-asas
INFO taarib_studio           Taarib Studio starting … isdar=1.0.0 mimariya=X8664 nizam=Linux
WARN taarib_studio::mukawwinat_tahmil a bundled component did not settle into the store
INFO taarib_studio           the library scan finished alaab=0 … mutaadhira=0
```

Same as the morning's build, including §5.1's still-empty `mawarid/` — whose
remedy text has since been rewritten and no longer claims the binary was
launched from cargo, which was the secondary finding in §5.1.

## 5. What must change, and who owns it

None of this was edited by this task; the files below belong to other owners.

### 5.1 The shipped `mawarid/` is empty — the biggest gap

`apps/studio/src-tauri/mawarid/` contains one file, `README.md`. The staging
tool (`taarib-tajmee`) was never run before the bundle, so the installer, the
`.deb` and the AppImage all carry an empty resource tree. Every artifact tested
logs this at startup, and keeps going:

```
WARN taarib_studio::mukawwinat_tahmil a bundled component did not settle into the store
  khata=TAARIB-E-9110 The bundle carries no staging manifest at
  <resource dir>\mawarid\bayan_mukawwinat.json … the studio continues, and installs
  will refuse each absent component by name. Run the staging tool (taarib-tajmee),
  then relaunch.
WARN taarib_studio component mirror: 0 copied, 0 already identical, 0 verified clean, 1 problem(s)
```

So the answer to "does it start" is yes, and the answer to "can it install a
patch" is no. The build procedure has to run `taarib-tajmee` before
`tauri build`, on every platform, and nothing in this tree enforces that.

Secondary: the remedy text asserts the cause instead of describing the
symptom — English at `apps/studio/src-tauri/src/mukawwinat_tahmil.rs:919-924`
("A development build launched from cargo has none"), Arabic at
`mukawwinat_tahmil.rs:870` ("وهذا متوقَّع في بناء تطويري يعمل من cargo"). It was
observed verbatim on an *installed* build reporting
`\\?\~\AppData\Local\Taarib\mawarid\bayan_mukawwinat.json`, and on an AppImage
reporting a path under its own mount. Neither is a cargo run, and a user
reading either log is told it is. The sentence should name the missing manifest
and the staging tool without claiming to know how the binary was launched.

### 5.2 The artifacts built today trust the development key

The status bar of every build tested shows `نسخة تطوير` — development build —
because `maalumat.hawiyat_thiqa` came back `tatwir`
(`apps/studio/src/shashat/maktaba.tsx:1220`).
`crates/taarib-khatm/src/malik.rs:117-129`: `MIRSAT_MALIK` anchors to
`MIFTAH_TATWIR` unless `TAARIB_MIFTAH_ISDAR` is set at build time, and the
`isdar` feature (`malik.rs:131-135`) exists to turn a forgotten injection into a
compile error. The mechanism is right; it simply was not used. A release
artifact must be built with `TAARIB_MIFTAH_ISDAR=<64 hex characters>` **and**
`--features isdar`, and no artifact without the release anchor should be
published. This is the difference between a client that trusts the committed
development key and one that trusts the owner's — it is not cosmetic.

### 5.3 `apps/studio/src-tauri/linux/taarib.desktop:11` — **fixed 2026-09-05**

`Icon=taarib` → `Icon=taarib-studio`. The bundler installs the application icon
under that name, derived from `mainBinaryName`; as it stood the Arabic desktop
entry resolved to no icon and nothing reported it. Corrected in place, and
`desktop-file-validate` still passes with only the `Categories` hint.

### 5.4 The duplicate desktop entry in the `.deb` — **fixed 2026-09-05**

The bundler writes `/usr/share/applications/Taarib.desktop` itself and there is
no configuration key to suppress it, so `bundle.linux.deb.files` adding
`com.cc1a2b.taarib.desktop` produced two menu entries rather than replacing one.

It was recorded as cosmetic. Installing the package on `ubuntu:22.04` showed it
was not. `update-desktop-database` wrote both entries into the cache in
alphabetical order:

```
application/vnd.taarib.ruqaa=Taarib.desktop;com.cc1a2b.taarib.desktop;
```

so `xdg-mime query default application/vnd.taarib.ruqaa` answered
**`Taarib.desktop`** — the bundler's, whose `Exec=taarib-studio` carries **no
`%U`**. Opening a `.ruqaa` from a file manager therefore launched Taarib with no
argument and the patch the user double-clicked was never passed to it. The
Arabic entry, which does carry `%U`, was second and never consulted.

Three ways out were written down here. None of them was taken, because reading
the bundler produced a fourth that is better than all three:

| candidate | verdict |
| --- | --- |
| drop the `files` mapping, accept the bundler's entry | loses the Arabic name, `GenericName`, `Keywords`, the `Game` category **and** `%U` — the association still drops the argument. Not a fix |
| keep both, add `postinst`/`postrm` removing `Taarib.desktop` | works, and costs the package the maintainer-script-free property the trigger owners in `deb.depends` currently buy it |
| rename the Arabic entry so it sorts first | makes the right one the default, and still shows the program twice |
| **chosen: map the file onto the generated path** | one entry, Arabic, with `%U`, no maintainer scripts, and nothing depending on alphabetical order |

`bundle.linux.deb.files` now reads

```json
"/usr/share/applications/Taarib.desktop": "linux/taarib.desktop"
```

and the equivalent, AppDir-relative, in `bundle.linux.appimage.files`. The
bundler generates the entry and copies the file map **afterwards**, with
`fs::copy` — `debian.rs:81-83` for the package,
`appimage/linuxdeploy.rs:80-83` for the image, both in tauri-bundler as shipped
with `@tauri-apps/cli` 2.11.1 — so the map overwrites the generated file rather
than adding to it.

**Measured, not reasoned.** The shipped package with the mapping applied,
installed on a clean `ubuntu:22.04` with `shared-mime-info`,
`desktop-file-utils` and `xdg-utils`:

```
$ ls /usr/share/applications | grep -i taarib
Taarib.desktop
$ grep -i taarib /usr/share/applications/mimeinfo.cache
application/vnd.taarib.ruqaa=Taarib.desktop;
$ xdg-mime query default application/vnd.taarib.ruqaa
Taarib.desktop
$ grep ^Exec= /usr/share/applications/Taarib.desktop
Exec=taarib-studio %U
$ printf 'TRQ1\x01\x00rest' > t.ruqaa && xdg-mime query filetype t.ruqaa
application/vnd.taarib.ruqaa
```

One entry; it is the default; the default carries `%U`; and the type resolves
from the `TRQ1` magic rather than the extension. `desktop-file-validate` passes
the file with the one standing `Categories` hint.

**Two things this does not fix, both named rather than papered over.**

- The argument is delivered and then ignored. Nothing consumes `argv` yet: a
  second launch reaches the single-instance plugin, which logs the arguments and
  raises the existing window (`apps/studio/src-tauri/src/main.rs:905-915`); a
  first launch leaves them in `std::env::args()`. That is one call site, not a
  packaging problem.
- The replacement depends on the destination path matching `productName`, which
  is what `freedesktop::generate_desktop_file` builds the name from, and on the
  bundler's copy order. Both are silent when they break, so `ibni.sh` now
  asserts the resulting tree in both artifacts: one `.desktop`, named
  `Taarib.desktop`, with `%U`, the `MimeType=` line, `Name=تعريب`, and the MIME
  XML beside it.

The `.deb` side of the type registration itself was already verified working.
After `apt-get install` on a clean `ubuntu:22.04`,
`application/vnd.taarib.ruqaa` appears in `/usr/share/mime/types`, in `globs2`,
and the `TRQ1` magic is in `/usr/share/mime/magic` — the `shared-mime-info`
dpkg trigger fires and the package needs no `postinst` for that part.

### 5.5 Smaller

- `tauri.conf.json:105-106` duplicates two entries the bundler generates
  anyway; dropping `libwebkit2gtk-4.1-0` and `libgtk-3-0` from
  `bundle.linux.deb.depends` would leave `Depends:` clean. The other three
  (`shared-mime-info`, `desktop-file-utils`, `hicolor-icon-theme`) must stay —
  they own the dpkg triggers that make the package need no `postinst`.
- `docs/README.md:36` — the tawzee index table has a row per file in this
  directory and now needs one for `tawzee/tahaqquq.md`. That file is outside
  this task's ownership.
- Control Flow Guard is off on the Windows binary (§3.1). If it is wanted,
  it is `-C control-flow-guard=yes` in the Windows profile.
- `bundle.targets` carries `nsis`, `app` and `dmg` on a Linux build and the CLI
  drops them without a word. That is Tauri's behaviour, not a defect, but it
  means a release script cannot tell "this target was skipped because we are on
  the wrong OS" from "this target failed" by exit code alone. Every platform's
  release job must assert on the artifact list, not on the exit status.
- The bundler installs its application icons into
  `hicolor/256x256@2/apps/`. The freedesktop icon-theme scale suffix is `@2x`,
  not `@2`, so that directory is not in any theme's search path and the 256px
  high-DPI icon is dead weight. Cosmetic, and the bundler's to fix.

### 5.6 The AppImage carries no MIME declaration — **implemented 2026-09-05**

`bundle.linux.deb.files` is `.deb`-only, so the AppDir had no
`usr/share/mime/packages/` at all and a `.ruqaa` was unknown data to every file
manager on an AppImage-only host. Tauri v2 has the matching AppImage key, and
`bundle.linux.appimage.files` now carries the same ten files, with the desktop
entry mapped onto the generated path for the reason in §5.4:

```json
"linux": {
  "appimage": {
    "files": {
      "usr/share/applications/Taarib.desktop": "linux/taarib.desktop",
      "usr/share/mime/packages/application-vnd.taarib.ruqaa.xml": "linux/application-vnd.taarib.ruqaa.xml",
      "usr/share/icons/hicolor/scalable/mimetypes/application-vnd.taarib.ruqaa.svg": "ayqunat/ruqaa/taarib-ruqaa.svg",
      "usr/share/icons/hicolor/16x16/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/16x16.png",
      "usr/share/icons/hicolor/24x24/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/24x24.png",
      "usr/share/icons/hicolor/32x32/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/32x32.png",
      "usr/share/icons/hicolor/48x48/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/48x48.png",
      "usr/share/icons/hicolor/64x64/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/64x64.png",
      "usr/share/icons/hicolor/128x128/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/128x128.png",
      "usr/share/icons/hicolor/256x256/mimetypes/application-vnd.taarib.ruqaa.png": "ayqunat/ruqaa/256x256.png"
    }
  },
  "deb": { … unchanged … }
}
```

The AppImage paths are written **without a leading slash**, matching the
convention for that key. It makes no functional difference —
`fs_utils::copy_custom_files` strips a leading `/` before joining — but the
destination *does* have to sit under `usr/`: `linuxdeploy.rs:115` copies only
`data_dir/usr/` into the AppDir, so anything mapped elsewhere is written to a
staging directory and then dropped without a word.

**Be honest about what it buys: less than it looks like.** Nothing on the host
reads `/usr/share/mime` from inside a mounted image, and the mount is gone when
the app exits, so double-clicking a `.ruqaa` still opens nothing unless the user
runs `appimaged` or integrates the image by hand. What the block buys is a
correct AppDir for anyone who *does* integrate — instead of a desktop entry
whose `MimeType=` names a type nothing on the system defines — and a correct
source tree for a Flatpak or any future format built from the same AppDir. It
adds ten files and no ELF, so it cannot move the glibc floor, and the
re-measurement confirms it does not. `linux.md` §3a is the full picture.

### 5.7 The Flatpak blocker lives in `taarib-usus` — **predicate landed 2026-09-05**

`flatpak run` puts the application in its own PID namespace, so
`taarib_usus::manassa::amaliyat_bism` — `sysinfo` over `/proc` — sees only the
sandbox. `tashtaghil` then returned `false` for every host process, and two
refusals stopped firing without saying anything: `tathbeet_bilnaqra::thabbit`
patching a running game, and `itlaq::KhiyaratSteam::athbit` rewriting
`localconfig.vdf` while Steam holds it. On a Deck, Steam is always running.

`manassa.rs` now answers the question instead of guessing at it:
`fi_sunduq() -> Option<Sunduq>` names the sandbox, `ruyat_amaliyat()` says
whether the process table is the host's, and `halat_tashghil()` returns the
three-state `Tashtaghil` / `LaTashtaghil` / `GhayrMaaruf { sunduq }`.
`tashtaghil` keeps its signature and becomes the fail-safe fold of that
(`halat_tashghil(ism).yamnaa()`), so no existing caller silently loses a check
while the honest one is adopted. `linux.md` §9 has the full table, including why
the `/proc/self/ns/pid` inode test was rejected: WSL2 sits in its own PID
namespace and that test would call every development machine a sandbox.

**What is left, and it is not in `taarib-usus`.** `masar_tathbeet::la_tashtaghil`
still calls `amaliyat_bism` directly and so still reads a sandbox's empty list
as "the game is not running"; `itlaq::yashtaghil` goes through `tashtaghil` and
is therefore already fail-safe, but its `MunassaTaamal` refusal says "Steam is
running" where it means "this build cannot see whether Steam is running". Both
live in `crates/taarib-tathbeet`. Until they consume `halat_tashghil`, no
Flatpak should be published; `linux.md` §9 records the decision, which stands.

## 6. Reproducing this

### 6.1 The Windows build (the gotchas are real)

The frontend must be built on the Linux side, because `node_modules` was
installed there and its binaries are ELF. The Tauri CLI must be installed
separately on the Windows side. And the config override must be passed as a
**file** — `cmd.exe` mangles inline JSON on the command line.

```
wsl:   cd apps/studio && npm run build          # produces apps/studio/dist
win:   cargo install tauri-cli --version ^2.11
win:   cargo tauri build --config <file>.json   # never --config '{...}'
```

The artifacts land in `target\release\` and `target\release\bundle\nsis\`.

### 6.2 The Linux build

```
cd apps/studio && ./node_modules/.bin/tauri build
```

Nothing else. `linuxdeploy`, `AppRun`, the GTK and GStreamer plugins and
`linuxdeploy-plugin-appimage` are downloaded from GitHub at bundle time, so the
Linux build needs network access even when the workspace is fully vendored.
Note that this repository sets a global `CARGO_TARGET_DIR`
(`~/.cargo-target`), so the Linux bundles do **not** appear under
`Taarib/target/` — they appear under `~/.cargo-target/release/bundle/`.

### 6.3 The font check, on Windows

```
set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 --remote-allow-origins=*
taarib-studio.exe
```

then, over the DevTools protocol on that port, `Runtime.evaluate` an expression
that awaits `document.fonts.ready`, `await`s `f.load()` for every
`document.fonts` entry, `fetch()`es every `CSSFontFaceRule` `url()` and compares
`byteLength` and a digest against the files in `apps/studio/dist/assets/`, and
compares `measureText` widths against the `serif` and `monospace` baselines.
Do not rely on `document.fonts.check()`.

### 6.4 What was deliberately not run

**Installing the `.deb`.** It needs root. The command is

```
sudo dpkg -i ~/.cargo-target/release/bundle/deb/Taarib_1.0.0_amd64.deb
sudo apt-get -f install        # if the webkit/gtk depends are unsatisfied
```

and it was not run, so `/usr/lib/Taarib/mawarid` resolution, the dpkg triggers,
and the menu-entry duplication were verified from the package contents rather
than from an installed system.

**Windows Sandbox.** The right way to prove "a machine with no development
environment" on this host would be Windows Sandbox — a pristine, disposable
Windows image. It is not installed (`C:\Windows\System32\WindowsSandbox.exe`
does not exist). Enabling it needs an elevated

```
Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM
```

and a reboot. That was not done. What replaces it is §3.2: the binary's import
table and its runtime module list, which together say the same thing without a
second machine — but they say it about *this* machine's OS, and a genuinely
clean Windows 10 image was never tested.

## 7. macOS — what it would take

Not reachable. There is no Apple hardware, no macOS VM and no cross-compiler
here, and there is no substitute: `.app` and `.dmg` are produced by
`tauri-bundler` paths that shell out to `hdiutil`, `SetFile`, `codesign` and
`xcrun`, all macOS-only.

What the configuration currently claims, and what would have to be checked:

| claim | how it would be verified | risk if it is wrong |
| --- | --- | --- |
| `bundle.targets` includes `app` and `dmg` | `cargo tauri build --target aarch64-apple-darwin` on macOS 12+ produces `Taarib.app` and `Taarib_1.0.0_aarch64.dmg` | none — it is skipped silently on other hosts |
| `bundle.icon` includes `ayqunat/icon.icns` | the file is present and well-formed: `icns` magic, declared length 85,826 matching the file, TOC plus `ic04 ic05 ic07 ic08 ic09 ic10 ic11 ic12 ic13 ic14 icp4 icp5` — every retina type modern Finder wants, no legacy `is32`/`il32`. This much **was** verified here | a malformed icns fails the bundle, or the Dock falls back to a generic icon |
| `bundle.macOS.minimumSystemVersion = "12.0"` | `plutil -p Taarib.app/Contents/Info.plist` shows `LSMinimumSystemVersion 12.0`, and the app launches on a 12.x machine | the app silently refuses to open on older systems, or crashes on a WebKit API the floor does not guarantee |
| `bundle.macOS.entitlements = "macos/taarib.entitlements"` | `codesign -d --entitlements - Taarib.app` shows exactly `com.apple.security.network.client` and `keychain-access-groups` | the keyring calls fail on a hardened build; the update channel cannot reach the network |
| `fileAssociations` `exportedType` `com.cc1a2b.taarib.ruqaa` | `Info.plist` carries `CFBundleDocumentTypes` and `UTExportedTypeDeclarations`; `mdls` on a `.ruqaa` reports the UTI after Launch Services registration | `.ruqaa` opens in nothing, or in TextEdit |

The two real risks, both unconfigurable from here:

1. **Signing and notarization.** `bundle.macOS.signingIdentity` is unset, so the
   build is unsigned. `macos.md` §3 already states the consequence honestly: a
   quarantined `.dmg` from an unsigned or merely Developer-ID-signed build is
   refused by Gatekeeper, and the documented workaround is
   `xattr -dr com.apple.quarantine /Applications/Taarib.app`. Notarization needs
   a paid Apple Developer account, `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID`
   or `APPLE_API_KEY` in the build environment, and a stapling step — none of
   which exists. Until it does, the macOS artifact is not a
   download-and-double-click artifact for an ordinary user, and the download
   page must say so.
2. **App translocation.** A quarantined `.app` run from `~/Downloads` executes
   from a randomised read-only mount, so `current_exe()` does not point where
   the user put it. The `taarib.mahmul` portable marker, which is resolved
   relative to the executable on macOS, cannot work under translocation.
   `macos.md` §3 names this; nobody has tested it.

## 8. Steam Deck — what it would take

Not reachable. No Deck, no SteamOS image, and no way to fake one: SteamOS's
distinguishing properties are its immutable root, its exact glibc snapshot, and
Valve's session compositor, none of which a container reproduces.

`steamdeck.md` verifies the *code* claims thoroughly, against file and line.
What it cannot verify, and what a Deck would settle in an afternoon:

- **glibc.** §4.1 turns `steamdeck.md`'s open question into a measured, negative
  answer: the AppImage built today needs `GLIBC_2.42`, which no SteamOS release
  provides. This is the one thing that must be fixed before a Deck test is even
  worth attempting, and it is fixed in the build environment, not in the config.
- **The immutable root.** SteamOS's `/` is read-only and rewritten wholesale by
  updates, so the `.deb` is not merely unsupported — it is meaningless there,
  and anything a `pacman` install put outside `/home` is discarded on the next
  update. The AppImage is the only Deck artifact, it must live under
  `/home/deck` (`~/Applications` by convention), and both roots the product
  writes — `~/.local/share/taarib` and `~/.config/taarib` — are already under
  `/home`, so state survives an OS update. That much follows from the code and
  is verified in `steamdeck.md` A4/A5/A7.
- **FUSE.** The AppImage mounted on a host with only `libfuse3`, which is
  encouraging, but SteamOS's FUSE configuration is its own question and the
  `--appimage-extract-and-run` fallback stays in the install steps.
- **No Secret Service.** Stock SteamOS provides no `org.freedesktop.secrets`,
  so every keychain-backed feature must degrade by name. `steamdeck.md` §B
  verifies from the code that startup never touches the keychain and that each
  caller maps the failure to a named error — but finding F4 there records that
  "store unreachable" collapses into "keychain refused" for contributor flows,
  which on a Deck is the *normal* case, not an edge case.

Two findings in `steamdeck.md` §4 have since been overtaken and should be marked
resolved rather than left to mislead: **F6** (the component store is never
created or populated) — `mukawwinat_tahmil.rs` now exists and runs at startup,
though it has nothing to mirror, which is §5.1 here; and **F7**
(`tauri.conf.json` ships `msi`/`rpm` and declares no `bundle.resources`) — the
current config ships neither and declares `bundle.resources = ["mawarid"]`.

## 9. Coverage, stated plainly

Verified: Windows binary and installer, install, cold start, Arabic UI, font
loading, uninstall, running-app refusal, data preservation. Linux `.deb` and
AppImage build, AppImage cold start, library resolution, font loading.

Not verified, and not claimed: the WebView2 bootstrapper path (the runtime was
already present); a genuinely clean Windows machine (Windows Sandbox is not
installed); the `.deb` installed into `/usr` (needs root); the `--istiada`
restore path with an actual patched game (the library had none); anything at all
on macOS or on a Steam Deck; and the AppImage on any host other than the one
that built it — which §4.1 says would currently fail.
