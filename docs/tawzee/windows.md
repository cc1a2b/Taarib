# التوزيع — Windows (`windows-x64`)

Companion to `docs/tawzee.md` §2/§4/§6/§7 for the shipped Windows package. Everything below is
derived from the frozen contract, from `apps/studio/src-tauri/nsis/qalab.nsi`, and from the Tauri
sources at tag `tauri-v2.11.5` — the version the workspace pins. Nothing here is aspirational.

The installer, the install, the cold start and the uninstall described here were **run** on
2026-09-04; the measurements, the imports, the loaded-module list and the font evidence are in
`tahaqquq.md`. Where that file and this one disagree, that one is the observation.

The artifact is a per-user NSIS installer, `Taarib_<isdar>_x64-setup.exe`, written to
`target/[x86_64-pc-windows-msvc/]release/bundle/nsis/`. It requests `RequestExecutionLevel user`
and never triggers a UAC prompt at any point, including WebView2 installation (see below).

## The qalab and how Tauri consumes it

`bundle.windows.nsis.template` points the bundler at `nsis/qalab.nsi`. The bundler does not merge
or patch it: it registers the file as the *entire* Handlebars template in place of its built-in
`installer.nsi` (tauri-bundler, `src/bundle/windows/nsis/mod.rs`, the `custom_template_path`
branch), renders it with the same data map, helpers (`or`, `association-description`,
`no-escape`) and NSIS escaping, then compiles it with the bundler-pinned NSIS 3.11 plus the
`nsis_tauri_utils` 0.5.3 plugin. `utils.nsh` and `FileAssociation.nsh` are written by the bundler
beside the rendered script, so the stock `!include` lines keep working.

Consequences that must hold forever:

- `qalab.nsi` is the stock `tauri-v2.11.5` template with exactly five deviations, each marked
  with a `; Taarib:` comment. Its Handlebars substitution set is byte-identical to stock (this
  was diffed, not assumed). A template that drifts from the bundler's data map fails the build.
- The template path is resolved against `apps/studio/src-tauri/` because the CLI runs
  `set_current_dir(dirs.tauri)` before bundling (`tauri-cli/src/build.rs:158`,
  `bundle.rs:139`). Hence the value `"nsis/qalab.nsi"`.
- The bundling is performed by `@tauri-apps/cli` (`^2.11.1`). If that CLI is ever bumped across
  a bundler change, re-diff `qalab.nsi` against the new `installer.nsi` before shipping.

The deviations, in file order:

1. compile-time `!error` unless `INSTALLMODE == currentUser` — tawzee.md §2 is per-user only;
   a config that says otherwise fails the build by name instead of producing an admin installer.
2. two hardcoded Arabic strings (`RISALA_TASHGHIL`, `RISALA_ISTIADA`) so the §7 texts cannot
   vary with the configured installer language.
3. the uninstaller *refuses* to run while `taarib-studio.exe` runs — the stock kill-the-app
   prompt is replaced by an Arabic refusal (`تعريب قيد التشغيل. أغلق التطبيق أولًا ثم أعد محاولة
   الإزالة.`) and an abort; silent uninstalls print the same text to the console and abort.
4. before any file is deleted, and only when not updating, the uninstaller runs
   `"$INSTDIR\taarib-studio.exe" --istiada` and waits for it — the Phase 20G library-wide
   restore offer (tawzee.md §7). Its exit code is deliberately ignored: a declined or failed
   offer must not make the application un-uninstallable.
5. the opt-in data wipe additionally removes `%APPDATA%\Taarib` — the real data root per
   `taarib_usus::masarat` — because the stock template only knows Tauri's bundle-id folders.

## On-disk layout after install

Default and expected location (the user may change it on the directory page; the choice is
remembered in the registry and reused by upgrades):

```
%LOCALAPPDATA%\Taarib\
├─ taarib-studio.exe
├─ uninstall.exe
└─ mawarid\                                staged by taarib-tajmee, carried as bundle.resources
   ├─ bayan_mukawwinat.json
   ├─ basmat\basmat.json
   ├─ khutut\<sinf>\<malaf>                + every malaf_rukhsa
   └─ mukawwinat\
      ├─ jisr\windows\x64\taarib_jisr.dll
      ├─ unity\Taarib.Unity.{Jisr,Mono,Il2cpp}.dll
      ├─ bepinex\<hadaf>\<khalfiya>-<jeel>-<mimariya>\…
      ├─ mudkhal\windows\<mimariya>\version.dll   + tabaqa/muhawwil payloads beside (§3 E–G)
      └─ mulhaq\{wasm,electron,rpgmaker,renpy,vxace}\
```

Why the tree lands intact: the bundler expands `bundle.resources = ["mawarid"]` into one
`CreateDirectory` per staged directory and one `File /oname=<relative target>` per staged file,
all relative to `$INSTDIR` — the tree is reproduced file-for-file, and the uninstaller deletes
exactly that generated list. At runtime `resource_dir()` on Windows is precisely the directory
containing the executable (`tauri-utils/src/platform.rs`, the `cfg!(windows)` early return), so
`zamin_mukawwinat` reads `<exe dir>\mawarid\bayan_mukawwinat.json` and mirrors the tree into
`%APPDATA%\Taarib\mukawwinat\` on startup (tawzee.md §5). The installer itself never creates the
data root; first run does.

This installer produces the *installed* form only. The portable form (`taarib.mahmul` beside the
executable, data in `<exe dir>\bayanat`, zero registry writes) is a plain unzipped tree and is
not produced by, and must never be combined with, this installer.

## WebView2 (evergreen bootstrapper)

Mode: `downloadBootstrapper` — the evergreen bootstrapper, Tauri's default, pinned explicitly in
the config (below). The stock section first probes for an existing runtime by reading `pv` under:

```
HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}
HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}
```

If present (Windows 11 always; nearly all updated Windows 10), the section is a no-op. If absent
and not updating, it downloads `MicrosoftEdgeWebview2Setup.exe` from
`https://go.microsoft.com/fwlink/p/?LinkId=2124703` into `$TEMP` and runs it `/silent /install`.
Because the installer is non-elevated, the bootstrapper installs the runtime **per-user** — no
UAC prompt. Microsoft: "If you don't run the installer from an elevated process or command
prompt, the Runtime will be installed as per-user", and a per-user runtime is automatically
replaced by a per-machine one when a per-machine Edge updater exists. A failed download or
install aborts the installation with a named message; it never continues into a broken install.

Runtime binaries are Microsoft's and live outside the install directory. The app's browser
profile lives in `%LOCALAPPDATA%\com.cc1a2b.taarib\` (WebView2 keeps an `EBWebView` folder
inside it): on Windows, Tauri forces the webview `data_directory` to `LocalData/<identifier>`
when unset (`tauri/src/manager/webview.rs`). `bundle.windows.minimumWebview2Version` is unset,
so no forced runtime-update path is compiled in.

## Registry

Every write is per-user. `SetShellVarContext current` makes `SHCTX` = `HKCU`; `SetRegView 64`
applies on x64. Nothing is ever written under HKLM.

`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Taarib` (Add/Remove Programs):

| value | type | content |
| --- | --- | --- |
| `DisplayName` | REG_SZ | `Taarib` |
| `DisplayIcon` | REG_SZ | `"<instdir>\taarib-studio.exe"` |
| `DisplayVersion` | REG_SZ | the workspace version |
| `Publisher` | REG_SZ | `cc1a2b` |
| `InstallLocation` | REG_SZ | `"<instdir>"` |
| `UninstallString` | REG_SZ | `"<instdir>\uninstall.exe"` |
| `MainBinaryName` | REG_SZ | `taarib-studio.exe` (upgrade bookkeeping) |
| `NoModify`, `NoRepair` | REG_DWORD | `1`, `1` |
| `EstimatedSize` | REG_DWORD | install size in KiB |
| `URLInfoAbout`, `URLUpdateInfo`, `HelpLink` | REG_SZ | `https://github.com/cc1a2b/taarib` |

`HKCU\Software\cc1a2b\Taarib`: default value = the chosen install directory. Read back by every
later installer run (`RestorePreviousInstallLocation`) — this is what makes upgrades land in the
same place. The MUI `Installer Language` value under the same key is written only when a
language-selector dialog is shown; the single-language Arabic build never shows one, so it is
never written.

Uninstall deletes the uninstall key always, and defensively deletes a `Taarib` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (Taarib ships no autostart entry; the
delete is stock and harmless). `HKCU\Software\cc1a2b\Taarib` survives unless the data checkbox
is ticked. The WebView2 keys above are only ever read. No URL protocol keys are written — none
is configured.

**File associations are written**, under `HKCU\Software\Classes`, because
`bundle.fileAssociations` declares the `.ruqaa` document type and the qalab renders it through
`APP_ASSOCIATE` (`qalab.nsi:676`). Verified on a real install:

| key | content |
| --- | --- |
| `.ruqaa` | default = `Taarib.Ruqaa`; `Taarib.Ruqaa_backup` = the previous handler, saved for restore |
| `Taarib.Ruqaa` | default = `رقعة تعريب — Taarib translation patch` |
| `Taarib.Ruqaa\DefaultIcon` | `<instdir>\taarib-studio.exe,0` |
| `Taarib.Ruqaa\shell\open` | `Open with Taarib` |
| `Taarib.Ruqaa\shell\open\command` | `<instdir>\taarib-studio.exe "%1"` |

`APP_UNASSOCIATE` removes the `Taarib.Ruqaa` ProgID on uninstall but **leaves `.ruqaa`
behind**: it restores the saved previous handler, and when there was none it writes an empty
string back rather than deleting the key. So a clean machine that installs and uninstalls is
left with `HKCU\Software\Classes\.ruqaa` holding an empty default and a stale
`Taarib.Ruqaa_backup`. Harmless — an empty default means no handler — but it is not a
registry-clean uninstall, and this is stock `FileAssociation.nsh` behaviour rather than a qalab
deviation.

The `.ruqaa` association is also why `tauri-plugin-single-instance` is a dependency: the
`shell\open\command` above launches the executable again while it is already running.

## Uninstall (§7)

Order inside `Section Uninstall`, exactly as the qalab compiles it:

1. If `taarib-studio.exe` is running (per-user process scan), refuse: Arabic message, abort.
   The application is never killed on the user's behalf.
2. If not invoked with `/UPDATE` and the executable exists, run
   `"$INSTDIR\taarib-studio.exe" --istiada` and block until it exits — every game with Taarib
   content installed is offered a full removal/restore (`istiada_cli.rs`, dispatch task 8).
3. Delete the executable, every file the bundler staged under `mawarid\`, `uninstall.exe`, the
   now-empty directories, the shortcuts, and the uninstall key.
4. The user's data — `%APPDATA%\Taarib` with the projects, patches, backups, memory, keys, and
   the mirrored component store — is left in place. Only if the explicit checkbox on the
   confirm page is ticked does the uninstaller also remove `%APPDATA%\Taarib`,
   `%APPDATA%\com.cc1a2b.taarib`, `%LOCALAPPDATA%\com.cc1a2b.taarib` (the WebView2 profile),
   and the install-location key. With `languages = ["Arabic"]` the checkbox label and all stock
   installer strings come from the bundler's own `Arabic.nsh` (verified complete at the pinned
   tag); the §7 texts are hardcoded in the qalab regardless.

## Upgrade in place (§6)

- **Updater path** (`taarib-tahdith`): after verifying hash and signature, run the downloaded
  `Taarib_<isdar>_x64-setup.exe` with `/UPDATE` plus `/P` (passive) or `/S` (silent). With
  `/UPDATE` the reinstall logic proceeds *without* running the old uninstaller: files are
  overwritten inside the remembered `$INSTDIR`, shortcuts and their pins are preserved, the
  WebView2 section is skipped, `--istiada` never runs, and the data root is untouched. The
  uninstall key is rewritten with the new version. An interrupted copy leaves the previous
  `uninstall.exe` and registry intact.
- **Manual upgrade** (user double-clicks a newer installer): the previous install is detected
  through the uninstall key and a page offers "uninstall first" (default) or "install over".
  "Uninstall first" runs the *old* uninstaller without `/UPDATE`, so the §7 restore offer fires
  before the old files go — a real uninstall is happening, so this is contract behaviour, not a
  bug. "Install over" behaves like the updater path.
- **Downgrades**: `allowDowngrades` defaults to `true` at this Tauri version; a silent downgrade
  aborts only when it is disabled. tawzee.md pins no downgrade policy; the channel manifest's
  `adna_isdar` is the updater-level floor.
- The stock WiX-migration scan (looks for an msi install of the same name/publisher under HKLM)
  is retained to minimise template drift; Taarib never shipped an msi, so it never matches.

## Installer and uninstaller flags

All read straight from the qalab source; `taarib-tahdith` and packaging scripts may rely on them.

| flag | where | meaning |
| --- | --- | --- |
| `/S` | both | fully silent (NSIS built-in) |
| `/P` | both | passive — progress page only, no questions |
| `/NS` | installer | create no shortcuts |
| `/UPDATE` | both | upgrade-in-place semantics described above |
| `/R` | installer | relaunch the app after install (silent/passive runs only) |
| `/ARGS <a>` | installer | arguments for the `/R` relaunch |
| `/D=C:\dir` | installer | install directory override (NSIS built-in; must be the last flag) |

## Why `msi` is not shipped

tawzee.md §2, verbatim: "`msi` and `rpm` are deliberately not shipped: every shipped format must
have an owner and a tested update path, and those two would have neither." An msi would be a
second uninstall/upgrade surface that had to reimplement the §7 restore gate and the §6 swap
rules under WiX, with no owner in the dispatch table. The `"msi"` entry still present in
`bundle.targets` today therefore has to go (below).

## `tauri.conf.json` — keys the convergence step must set

`tauri.conf.json` is owned by the convergence step (tawzee.md §8); this task edits none of it.
For the Windows target to build and behave as specified above, convergence must set exactly:

| key | required value |
| --- | --- |
| `bundle.targets` | drop `"msi"`/`"rpm"` (§2) → `["deb", "appimage", "nsis", "app", "dmg"]` |
| `bundle.resources` | `["mawarid"]` (§4 — the staged tree, verbatim) |
| `bundle.windows.webviewInstallMode` | `{ "type": "downloadBootstrapper", "silent": true }` |
| `bundle.windows.nsis.template` | `"nsis/qalab.nsi"` |
| `bundle.windows.nsis.installMode` | `"currentUser"` (the qalab `!error`s on anything else) |
| `bundle.windows.nsis.languages` | `["Arabic"]` |

All six are set in `tauri.conf.json` as of Phase 28, and `bundle.fileAssociations` has since been
added for `.ruqaa` — which is what produces the `HKCU\Software\Classes` writes above. Both are
verified against a real install in `tahaqquq.md` §3.5.

Already correct and load-bearing — must not change without touching this contract:

- `mainBinaryName: "taarib-studio"` — the uninstaller invokes `taarib-studio.exe --istiada`.
- `productName: "Taarib"` — names the install dir, the uninstall key, and matches the hardcoded
  `%APPDATA%\Taarib` data root in both `masarat.rs` and the qalab's opt-in wipe.
- `identifier: "com.cc1a2b.taarib"` — names the WebView2 profile folder the wipe removes.
- `publisher: "cc1a2b"` — the registry `Publisher` value and the `Software\cc1a2b` key.

Authenticode is outside tawzee.md — §1's identities sign packages and channels, not PE files.
The qalab passes the stock `{{uninstaller_sign_cmd}}` hook through unchanged, so configuring
`bundle.windows.signCommand` later requires no template change.
