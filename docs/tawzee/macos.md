# التوزيع / ماك — the macOS target

Task 4 of the Phase 22 dispatch (`docs/tawzee.md` §8). Two bundles, one per
architecture: `aarch64-apple-darwin` and `x86_64-apple-darwin`, each a `.app`
delivered in a `.dmg`. No universal binary — two `.dmg` files, named per
architecture, because a universal build doubles the download for everyone to
save a choice made once.

## 0. Verification status — a compile wall, measured

**No macOS bundle has been built or run.** As of Phase 28 (2026-09-05) no macOS
machine, VM or emulator is available to this project, and there is no
substitute: `tauri-bundler`'s `.app` and `.dmg` paths shell out to `hdiutil`,
`SetFile`, `codesign` and `xcrun`, none of which exists off macOS. A build on
another host does not fail on these targets — it skips them silently, with a
zero exit code — so nothing about the bundling half of this file has ever been
contradicted by a build either.

What *has* now been measured, on Linux with toolchain 1.95.0 and both Apple
targets installed, is how far `cargo check` gets before it hits that wall. See
§0.1. The short version: **7 of the 29 workspace packages type-check for Apple;
22 do not, and not one of the 22 fails in Taarib's own Rust.**

Exactly one other claim below has been checked from this side:
`ayqunat/icon.icns` is a well-formed icns — `icns` magic, declared length 85,826
matching the file, a TOC plus `ic04 ic05 ic07 ic08 ic09 ic10 ic11 ic12 ic13 ic14
icp4 icp5`, which is every type macOS 12 wants and no dead legacy
`is32`/`il32`. So the icon pipeline's output will not be what fails the bundle.

## 0.1 What compiles for Apple today

Method: `cargo check -p <pkg> --target <triple>` for each of the 29 workspace
packages, run once per Apple triple. Not `--workspace`: a workspace check aborts
at the first failing build script and reports one crate's problem as everything's.

Both triples give **exactly the same 7 / 22 split, with the same seven crates**.

| result | count | packages |
| --- | --- | --- |
| type-checks for `aarch64-apple-darwin` and `x86_64-apple-darwin` | 7 | `taarib-usus`, `taarib-jisr`, `taarib-lawha`, `taarib-mustalahat`, `taarib-saff`, `taarib-mudkhal`, `taarib-wasm` |
| blocked before Taarib's code is reached | 22 | everything else |

The seven also pass with `--all-targets`, so their tests type-check too. They are
the pure-Rust spine: the foundations, the text stack (shaping, layout,
terminology), the C-ABI core, and the preload loader.

**Why the other 22 fail, and what that does not prove.** Every one of the 22
fails inside a *third-party build script* that needs an Apple C or Objective-C
toolchain, which this machine does not have:

| build script | what it needs |
| --- | --- |
| `libsqlite3-sys` | compiles bundled SQLite from C |
| `blake3` | compiles C SIMD backends |
| `ring` | compiles C and per-architecture assembly |
| `objc2-exception-helper` | compiles `try_catch.m`, Objective-C |
| `libudis86-sys` (`taarib-haqn`, x86-64 only) | compiles the udis86 disassembler from C |

The failure is always the same shape — `cc: error: unrecognized command-line
option '-arch'`, because the host `cc` is a Linux gcc being handed Apple flags.
Which of the four names appears for a given crate is **not stable between runs**:
cargo builds scripts in parallel and reports whichever loses first, so the
per-crate attribution shifted between two runs of the same sweep. The stable fact
is the class, not the name.

The consequence matters more than the count. Cargo never reaches Taarib's own
source in those 22 crates, so **this exercise has not shown that their Rust is
Apple-clean — it has only shown they are blocked earlier.** Any estimate that
treats "22 failing" as 22 crates' worth of porting work is reading the number
backwards; the true figure is unknown and can only be established on a Mac.

One crate is expected to fail on its merits: `taarib-haqn` is Windows-only by
design, and on `aarch64-apple-darwin` the vendored `retour` fails in Rust with
`E0570: "win64" is not a supported ABI for the current target` plus a missing
`arch::meta` module. That is `retour` being an x86-plus-Windows crate, not a
defect in Taarib.

**The one real Apple-only source defect found, and fixed.**
`crates/taarib-usus/src/manassa.rs` computed free disk space as
`ihsaa.f_bavail * ihsaa.f_frsize`. The two `statvfs` fields are not the same
width on every unix: on Darwin `f_bavail` is a 32-bit `fsblkcnt_t` beside a
64-bit `f_frsize`, and on 32-bit Linux it is the other way round. Only Linux
x86-64, where both are `u64`, compiles it. Because `taarib-usus` is the
foundation every other crate depends on, this single line failed the *entire*
workspace for Apple — before the fix the Apple-passing set was one crate, and it
was `taarib-mudkhal`, which happens not to depend on `taarib-usus`. Both operands
are now widened with `u64::from`, which is a no-op where the field is already
64-bit, and the multiply is saturating. Linux is unaffected and still builds.

## 0.2 What a first macOS run has to establish, in order

1. `cargo check --workspace --target aarch64-apple-darwin` passes — the step
   §0.1 could not reach. This is the one that carries unknown risk: 22 crates'
   worth of Rust has never been type-checked for Apple, and the defect found in
   `manassa.rs` is proof that this workspace can carry Apple-only type errors
   that no amount of Linux and Windows building reveals. Budget for finding
   more of them here, not for finding none.
2. `cargo tauri build --target aarch64-apple-darwin` produces `Taarib.app` and
   `Taarib_1.0.0_aarch64.dmg` at all — the bundler's macOS path has never
   executed for this project.
3. `plutil -p Taarib.app/Contents/Info.plist` shows `LSMinimumSystemVersion`
   `12.0`, `CFBundleDocumentTypes` and `UTExportedTypeDeclarations` for
   `com.cc1a2b.taarib.ruqaa`, and the icon file reference.
4. `Taarib.app/Contents/Resources/mawarid/` contains the staged tree — which
   today would be empty everywhere, see `tahaqquq.md` §5.1.
5. The app opens on a clean macOS 12 machine, in Arabic, with the eight bundled
   IBM Plex faces served by `wry`'s custom protocol. On WKWebView the check is
   the same one used on Windows: fetch every `@font-face` `url()` from inside
   the page and compare bytes and metrics, since Safari's Web Inspector can be
   attached to a `WKWebView` with `WKWebViewDeveloperExtras`.
6. `codesign -d --entitlements - Taarib.app` shows exactly the entitlements
   §4 ends up granting and nothing else.

Steps 1–4 need only a Mac. Steps 5–6, on a machine that downloaded the `.dmg`
rather than built it, need §3 to be resolved first.

## 1. Where the staged tree lands

`PathResolver::resource_dir()` documents macOS as `${exe_dir}/../Resources`
inside the `.app`, so `bundle.resources = ["mawarid"]` puts the staged tree at:

```
Taarib.app/Contents/Resources/mawarid/
```

Read through `resource_dir()` at startup and never composed by hand.

## 2. The game-side libraries, and why `@rpath` does not apply

The `.dylib` files Taarib ships for games — `libtaarib_jisr.dylib`,
`libtaarib_muhammil.dylib`, the overlay and the engine adapters — are never
linked against anything. They are deployed **into a game's own directory** by
`taarib_tathbeet::tarkib` and loaded one of two ways:

- `DYLD_INSERT_LIBRARIES`, set on the game's launch, for the preload loader;
- an absolute-path `dlopen` by a payload the loader already brought in.

Both take a full path. `install_name`, `@rpath`, `@loader_path` and the rest of
the dynamic linker's search machinery decide where a *linked* dependency is
found, and nothing here is a linked dependency — so none of it applies, and the
one thing that matters is that the path Taarib writes is the path the file is
at. `taarib-mudkhal` resolves its own directory with `dladdr` on one of its own
functions rather than from the environment, for the same reason.

**What System Integrity Protection does to this.** SIP strips
`DYLD_INSERT_LIBRARIES` from the environment of any process that is:

- a platform binary (anything Apple ships), or
- signed with the hardened runtime *without*
  `com.apple.security.cs.allow-dyld-environment-variables`, or
- setuid/setgid.

An ordinary Steam or itch.io game is none of those, and the variable survives.
A hardened, notarized game is, and it does not. The degradation is named rather
than silent: the injection path refuses with the reason, and the overlay tier
remains — it does not need the variable, because on macOS the overlay reaches
the process through the graphics API, not through a preload.

## 3. Quarantine, Gatekeeper, and the honest v1 stance

A `.dmg` downloaded by a browser is written with the `com.apple.quarantine`
extended attribute, and everything inside inherits it. What happens on first
open depends on how the bundle is signed:

| signing | what the user sees |
| --- | --- |
| unsigned or ad-hoc | Gatekeeper refuses outright: "Taarib is damaged and can't be opened" or "cannot be opened because the developer cannot be verified". Ad-hoc signing changes the wording, not the refusal |
| Developer ID, not notarized | still refused on a quarantined copy; notarization is what Gatekeeper checks, not the certificate alone |
| Developer ID **and** notarized, stapled | opens normally, no warning |

There is a second effect worth naming: **app translocation**. A quarantined
`.app` launched from where it was downloaded runs from a read-only random
mount point, so anything that expects to sit beside its own file — the
`taarib.mahmul` portable marker most of all — behaves as though it is somewhere
else. Moving the `.app` into `/Applications` clears translocation.

**The v1 stance, stated plainly:** Taarib ships an unnotarized Developer ID
build or an unsigned one, depending on whether the owner holds a paid Apple
Developer account at release. Either way the first-open path is the one the user
must be told about, in the release notes and on the download page:

```
xattr -dr com.apple.quarantine /Applications/Taarib.app
```

That is a real instruction with a real effect, and it is stated rather than
hidden. It is not a substitute for notarization; when an account exists,
`bundle.macOS.signingIdentity` plus a notarization step removes the need for it
and this section gets shorter.

## 4. Entitlements

`apps/studio/src-tauri/macos/taarib.entitlements`, declared through
`bundle.macOS.entitlements`. Two entitlements are currently declared:

- `com.apple.security.network.client` — the registry, the forge, and the update
  channel; all outbound HTTPS. Correct, and needed.
- `keychain-access-groups` — declared for the signing keys and provider
  credentials the `keyring` crate stores in the login keychain. **This one
  should be removed. See below.**

### `keychain-access-groups` is wrong here, and can refuse to launch

The file grants `$(AppIdentifierPrefix)com.cc1a2b.taarib` with the comment
"the access group is the app's own, so nothing else can read them". That
reasoning inverts what the entitlement does. Keychain access groups do not
*restrict* access; they *widen* it, naming additional groups an app may share
items with. A non-sandboxed Developer ID app storing a generic password in the
file-based login keychain is already protected by the item's ACL and the app's
own code-signing identity, and needs no access group at all.

Declaring it is not merely redundant. `keychain-access-groups` is a
provisioning-profile-backed entitlement: unless the `.app` embeds a Developer ID
provisioning profile (`embedded.provisionprofile`) whose App ID carries the
matching capability, the entitlement is *unsatisfied*, and macOS refuses to
launch the app — `taskgated-helper` logs `Unsatisfied entitlements:
keychain-access-groups`. Tauri does not embed a provisioning profile by default.
So the current file is a plausible cause of a signed build that will not start
at all, on a path nobody here can test.

Two further reasons to drop it: it steers `SecItem` calls toward the data
protection keychain, which is not what the `keyring` crate's macOS store uses;
and Apple's own guidance for notarisation is to claim the least privilege that
works, because every entitlement is something the notary service inspects.

**Recommendation: delete the `keychain-access-groups` key and its array,
leaving `com.apple.security.network.client` as the only entitlement.** Left
unapplied here — `apps/studio/**` belongs to another agent this phase.

Verified against neither a Mac nor a signed build; this is a documentation and
Apple-behaviour argument, and it should be re-checked the first time a signed
`.app` launches.

Not granted, and not needed: App Sandbox (the product reads and writes inside
game folders anywhere on disk that the user points it at, which the sandbox
cannot express), JIT, unsigned executable memory, library-validation
exceptions, Apple Events, camera, microphone, location.

`com.apple.security.cs.allow-dyld-environment-variables` is **not** granted
either: that entitlement governs variables in *this* process's environment, and
the variable Taarib sets is in the *game's*. Granting it would weaken this
application's own hardening to no effect on the thing it looks like it fixes.

## 5. `tauri.conf.json` keys

Set by the convergence step:

| key | value |
| --- | --- |
| `bundle.macOS.minimumSystemVersion` | `"12.0"` — Safari 15 is the webview floor `vite.config.ts` already targets |
| `bundle.macOS.entitlements` | `"macos/taarib.entitlements"` |
| `bundle.macOS.signingIdentity` | left unset until an account exists; setting it is the whole change |

The first two are set in `tauri.conf.json` today, and `bundle.targets` carries
`"app"` and `"dmg"`. `signingIdentity` is still unset, so §3's unsigned row is
the row that applies.

A third key is worth knowing about even though it is left at its default:
`bundle.macOS.hardenedRuntime` defaults to **`true`** in the Tauri 2 bundler,
which is what notarisation requires. Do not set it to `false` — an unhardened
build cannot be notarised at all.

"Setting it is the whole change" is true of the *config* and not of the
release. Checked against the current Tauri 2 and Apple documentation on
2026-09-05, the release side needs all of:

| requirement | detail |
| --- | --- |
| a **paid** Apple Developer Program membership | US$99/year. Not optional and not substitutable: a free Apple ID cannot be issued a Developer ID Application certificate, and without that certificate there is nothing to notarise. Apple DTS states this directly |
| a Developer ID Application certificate | plus the hardened runtime, which Tauri already enables |
| signing credentials in the environment | `APPLE_SIGNING_IDENTITY`, or `APPLE_CERTIFICATE` (base64 `.p12`) + `APPLE_CERTIFICATE_PASSWORD` |
| notarisation credentials, one of two sets | App Store Connect API key: `APPLE_API_ISSUER` + `APPLE_API_KEY` + **`APPLE_API_KEY_PATH`**. Apple ID: `APPLE_ID` + `APPLE_PASSWORD` (an app-specific password) + `APPLE_TEAM_ID` |
| a Mac to run it on | Tauri's own documentation: "You also need an Apple device where you perform the code signing. This is required by the signing process and due to Apple's Terms and Conditions" |

Two corrections to what this file said before:

- The API-key route takes **three** variables, not two. `APPLE_API_KEY_PATH`
  (the path to the downloaded `.p8`) was missing, and the pair alone does not
  authenticate.
- **`xcrun stapler staple` is not a step to run by hand.** The Tauri bundler
  staples automatically once notarisation succeeds; the flag `--skip-stapling`
  exists precisely to opt *out*. Adding a manual stapling step to a release
  script would at best be redundant.

Also worth recording because it dates fast: `altool` is retired, and
`notarytool` is the only supported submission path. Anything found in an older
guide that drives `altool` is dead.

None of this exists anywhere in this tree — no CI at all (there is no
`.github/workflows`), no key, no account — so until it does the macOS artifact
is not a download-and-double-click artifact for an ordinary user, and the
download page owes them the `xattr` line above rather than leaving them at
"Taarib is damaged and can't be opened".

## 6. Artifact names

- `Taarib_<version>_aarch64.dmg`
- `Taarib_<version>_x64.dmg`

## 7. What game detection would actually mean on macOS

This is the question that decides whether macOS is a port or a rebuild, and the
answer is better than expected: **discovery is a real runtime abstraction, not
Windows code with a coat of portability.**

`taarib-kashf` dispatches on a *value*, not a `cfg`. `NizamTashghil`
(`crates/taarib-usus/src/manassa.rs:27`) is resolved once at
`NizamTashghil::hali()` and then carried as data through `SiyaqFahs`; the
`Matjar` trait (`crates/taarib-kashf/src/fahs.rs:307`) declares which platforms
each adapter serves via `manassat_maduma()`, and the scan loop skips adapters
that do not list the running platform. Of the 19 `#[cfg(windows)]` sites in the
crate, every one is a leaf that supplies an `Option` or a `Vec` and whose
non-Windows arm returns empty — not one gates a type, a trait, or control flow.
That is the structural proof that the abstraction is real.

**Ten of the eighteen storefront adapters already run on macOS**, with genuine
Apple roots rather than stubs: Steam
(`~/Library/Application Support/Steam`, `steam.rs:245`), Epic
(`/Users/Shared/Epic`), GOG Galaxy (the cross-platform SQLite database),
Battle.net (`/Users/Shared/Battle.net`), itch, Heroic, legendary, Playnite's
nominated root, plus the manual and portable scanners. Six are Windows-only by
construction and have no macOS meaning at all — EA, Ubisoft, Xbox/UWP, Amazon,
Rockstar, Riot — and two are Linux-only (Lutris, Bottles). The Mach-O reader
handles all six magics, and icon extraction reads `Contents/Info.plist` →
`CFBundleIconFile` → `Contents/Resources/*.icns` by hand. `/Volumes` is already
in the mount-parent list.

Three real gaps, in descending order of how much they matter:

1. **No translation-layer awareness whatsoever — the big one.** A large share of
   what a Mac gamer actually plays is a Windows game under CrossOver, Whisky, or
   Apple's Game Porting Toolkit, and none of the three exists in the model.
   `BeeatTawafuq` has exactly `Asli`, `Proton`, `Wine`, `Rosetta`; prefix
   discovery (`beea.rs:1424`) is a hardcoded XDG list — `~/.wine`,
   `~/.local/share/wineprefixes`, Lutris and Bottles trees — with no
   `~/Library/Application Support/CrossOver/Bottles` and no Whisky container
   path, so only a coincidental `$WINEPREFIX` or `~/.wine` would ever hit.
   Heroic *reads* `wineVersion.type` values of `crossover` and `toolkit` but
   collapses both to plain `Wine` (`heroic.rs:405`). And `Rosetta` is modelled
   but **never constructed anywhere in discovery** — the only place it is
   produced is deserialisation in `taarib-makhzan`. Worse than absent, it is
   mislabelled: `steam.rs:1527` returns `BeeatTawafuq::Asli` for every non-Linux
   platform, so a Windows Steam title running under Whisky is reported to
   everything downstream as a native Mac binary.
2. **The coarse folder-admission heuristics assume the flat Windows/Linux
   layout.** `mahmul.rs:820` admits a Unity game by finding a `<stem>_Data`
   directory beside the executable, or `unityplayer.dylib` at the install root.
   A native Mac Unity game has neither: it is `Foo.app/Contents/Resources/Data`
   with the dylib at `Foo.app/Contents/Frameworks/UnityPlayer.dylib`. Electron
   looks for `resources/app.asar` and not `Contents/Resources/app.asar`. So a
   hand-nominated folder of native Mac games likely yields nothing.
   The redeeming detail: **the correct rules already exist**, in
   `taarib-muharrik/src/dalail/unity.rs:498-535` and `nusus.rs:2729`, which do
   walk `*.app` and strip `Contents/Resources/Data`. `taarib-kashf` simply does
   not use them. This is a sharing problem, not a research problem.
3. **No default macOS scan roots** for the portable scanner, and no test
   coverage on any macOS branch — the crate has one test module.

So the honest scoping answer: detection on macOS is perhaps 70 percent there
for *native* Mac games and near zero for the translation-layer library that is
most of the real catalogue. Gap 2 is a day. Gap 1 is a design question — what
`BeeatTawafuq` should even mean on Apple silicon — and is the reason a macOS
release should not be promised on a fixed date.

Reported, not fixed: `crates/taarib-kashf/**` and `crates/taarib-muharrik/**`
belong to other agents this phase.

## 8. What finishing macOS actually requires

In dependency order. Nothing here is optional, and nothing here can be done
from this machine.

1. **A Mac, or a licensed cross-SDK.** There is no legal way around it. The
   Apple SDK may not be redistributed, so a Linux cross-build needs the SDK
   extracted from Xcode on a Mac the owner controls under the Xcode licence;
   and even then §5's own citation says signing must happen on Apple hardware.
   A rented CI Mac (GitHub-hosted `macos-*` runners, or any Mac cloud) satisfies
   both. This single item unblocks everything below.
2. **Get `cargo check --workspace` green for both Apple triples.** The 22
   blocked crates get type-checked for the first time. Expect defects of the
   `manassa.rs` family — `libc` struct fields that differ in width between
   Darwin and Linux are the specific hazard, because they compile everywhere
   else. This is unbudgetable from here; it is the first thing a Mac tells you.
3. **A paid Apple Developer Program membership**, US$99/year, and a Developer ID
   Application certificate issued from it. Without this, §3's unsigned row is
   permanent and every user needs the `xattr` incantation.
4. **Remove the `keychain-access-groups` entitlement** (§4) before the first
   signed build, or risk a build that will not launch.
5. **Bundle targets** are already correct: `bundle.targets` carries `"app"` and
   `"dmg"`, and `minimumSystemVersion` is `"12.0"`. Two `.dmg` files, one per
   architecture, per §0. No change needed.
6. **Notarise and staple** with the credentials in §5. Stapling is automatic.
7. **Verify on a clean machine** that never built the app: §0.2 steps 5–6.
8. **Then, and only then**, decide what to say about detection (§7).

Steps 1–2 are the weekend-versus-month question, and they cannot be answered
from Linux. Everything from step 3 down is well-trodden and predictable.

## 9. Should macOS be claimed today? No.

Stated plainly, because the brief asked for a position and not a hedge:
**Taarib does not support macOS today, and nothing should say it does until at
least §8 steps 1–3 are done.** The gap between "configured" and "supported" is
the whole of §8, and the project has never run a single line of its own code on
Apple hardware.

The evidence is not ambiguous. There is no CI. There is no Mac. 22 of 29
packages have never been type-checked for Apple, and the one crate that *could*
be checked turned out to contain an Apple-only type error that had gone
unnoticed for the project's whole life — which is the best possible argument
that the untested 22 are not clean either.

What the current wording implies, and why it is too strong:

| where | today | the problem |
| --- | --- | --- |
| `README.md:8` | a platform badge reading `Windows \| Linux \| macOS` | a badge is a support claim; a reader takes it as "there is a build" |
| `README.md:329` | "Operating systems. Windows 10 1809+, Linux (including Steam Deck), macOS" | lists macOS beside two platforms that genuinely work |
| `README.md:444` | the download table row for macOS 12+, naming both `.dmg` files | names artifacts that have never been produced. It does say "Configured", which is honest, but a table of downloads implies downloads |
| `docs/tawzee.md` §2 | `macos-arm64` and `macos-x64` as target rows | reads as a shipping matrix |

Recommended wording, for whoever owns those files — the two changes that matter
most are the badge and the download row:

- **Badge**: `platform-Windows | Linux-lightgrey`, and add a separate
  `macOS-planned-inactive` badge if macOS should stay visible at all. Do not
  list macOS in a platform badge that otherwise means "supported".
- **Operating systems line**: "Windows 10 1809+ and Linux (including Steam
  Deck). macOS is not supported yet — the code targets it and the bundle is
  configured, but no macOS build has ever been produced or run. See
  `docs/tawzee/macos.md`."
- **Download table**: replace the macOS row's artifact names with
  "Not yet available", keep the link to this file, and move the two `.dmg`
  names into this document where they belong as a plan. An artifact name in a
  download table is a promise.
- **`docs/tawzee.md` §2**: mark the two macOS rows "planned — never built; see
  `tawzee/macos.md` §0.1" rather than removing them. They are a real intent and
  the triples are correct; they are just not a shipping matrix yet.

The honest one-line version, if only one line is wanted: *macOS is a target,
not a platform. It builds nowhere, it has been run nowhere, and the seven
crates that compile for it are the ones with no C dependencies.*
