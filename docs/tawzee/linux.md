# التوزيع / لينكس — the Linux target

Task 2 of the Phase 22 dispatch (`docs/tawzee.md` §8). Two artifacts from one
build: a portable **AppImage** that runs on any distribution new enough to load
it, and a **`.deb`** for the distributions that would rather own the install.
The Steam Deck takes the AppImage unchanged — see `steamdeck.md`.

`rpm` is deliberately not shipped (`tawzee.md` §2): every shipped format needs
an owner and a tested update path, and it would have neither.

Both bundles were **built** on 2026-09-04 and the AppImage was **run**; the
`.deb` was verified from its contents rather than from an installed system,
because installing it needs root. Measurements and hashes are in `tahaqquq.md`.

That build had one blocker, §5a: it inherited its build host's glibc and would
start on almost nothing. **Fixed on 2026-09-05** — the Linux artifacts are now
built in an Ubuntu 22.04 container (§8), the floor is `GLIBC_2.35`, and both
artifacts were rebuilt, installed and run on hosts where the previous ones die
at the loader. The `.deb` was installed for real this time, on a clean
`ubuntu:22.04`.

A second pass the same day closed the desktop-integration defects: one desktop
entry instead of two, so the default handler for a `.ruqaa` is the entry that
carries `%U` (§3, §3a); the MIME declaration and the document icons now reach
the AppImage as well as the package (§3a); and the library that libepoxy opens
by name, which aborts the process when it is missing, is now found by the
measurement and depended on by the package (§1). Everything in those three was
rebuilt and re-measured, not argued from the config file.

## 1. What the AppImage contains, and what it still needs from the host

Tauri's Linux bundler runs `linuxdeploy` and pulls the WebKitGTK stack and its
transitive libraries into the image, which is why the AppImage runs on hosts
whose own `webkit2gtk-4.1` is older than the build machine's — or absent.

It also carries `usr/bin/xdg-open`, copied out of the build image's `/usr/bin`
by the bundler. `AppRun.wrapped` exports
`PATH=$APPDIR/usr/bin/:…:$PATH` — read from the shipped binary's own format
string, not assumed — so the bundled copy is the one found first and opening a
link or a folder works from the image on a host that owns no `xdg-utils` at all.
The bundler will not finish an AppImage without one to copy, which is why the
build image installs the package (§8).

What it does **not** carry, and therefore what the host must still provide:

- **FUSE**, to mount the image at all. The documented fallback needs no install:

  ```
  ./Taarib_1.0.0_amd64.AppImage --appimage-extract-and-run
  ```

  On 2026-09-04 the built image mounted itself at `/tmp/.mount_Taarib…` and
  started normally on a host carrying `libfuse3` and **no `libfuse2`**, so the
  fallback was not needed there. Keep it in the install steps anyway: it costs a
  line and it is the only recourse on a host whose FUSE is refused or absent,
  and it is not a claim that can be made once and reused across distributions.

- **A Secret Service** (`org.freedesktop.secrets`), for provider credentials and
  the owner key. Its absence is a named refusal in the feature that wanted it,
  never a startup failure — see `adhonat.md` and `steamdeck.md` §B.
- The **graphics stack** the game side needs. That is the game's own
  requirement, unchanged by Taarib.
- **Twenty-two shared objects**, enumerated rather than assumed.
  `qias_qaa.sh` subtracts every `DT_SONAME` in the AppDir from every
  `DT_NEEDED` in it; what is left is what the host owns:

  | group | sonames | why it is not bundled |
  | --- | --- | --- |
  | the C/C++ runtime | `ld-linux-x86-64.so.2`, `libc.so.6`, `libm.so.6`, `libresolv.so.2`, `libgcc_s.so.1`, `libstdc++.so.6` | an AppImage cannot carry these; they are what §5a's floor is about |
  | graphics and display | `libEGL.so.1`, `libGL.so.1`, `libgbm.so.1`, `libdrm.so.2`, `libX11.so.6`, `libX11-xcb.so.1`, `libxcb.so.1` | driver-coupled: a bundled copy would be the wrong one on the user's GPU |
  | text shaping and fonts | `libfontconfig.so.1`, `libfreetype.so.6`, `libharfbuzz.so.0`, `libfribidi.so.0`, `libexpat.so.1` | the AppImage excludelist keeps these on the host so the system's own font configuration and its Arabic faces are the ones used |
  | crypto and support | `libgpg-error.so.0`, `libgmp.so.10`, `libcom_err.so.2`, `libz.so.1` | dependencies of bundled `libgcrypt`/`libgnutls`/`libkrb5` that `linuxdeploy` did not follow. Present on any desktop; the honest statement is that they are a host requirement, not that they are bundled |

  **Twenty-three, with `libasound.so.2` for audio, was the 2026-09-04 number**
  and it was carried forward here without being re-derived. Re-measured against
  the container build on 2026-09-05: the jammy image pulls in no `libflite`, so
  nothing in the tree links `libasound` and the `ALSA_0.9` version family is
  gone from the floor as well. The number is what the script printed, not a
  count anybody keeps by hand.

  Nothing in that table is missing on a stock SteamOS desktop session, which is
  why the image runs there once §5a is fixed. It is listed because "the AppImage
  bundles all dependencies" is the claim Tauri's documentation makes and it is
  not exactly true.

- **`libGLESv2.so.2`**, and three more like it. The table above is built from
  `DT_NEEDED`, which is the *link-time* contract; a library opened by name at
  run time appears in no `DT_NEEDED` and in no relocation, so no symbol analysis
  can list it. Its absence is not a degradation: the process prints
  `Couldn't open libGLESv2.so.2` after the startup log and **aborts**.

  `qias_qaa.sh` now finds these too, by reading the string tables — a name that
  looks like a soname, carried inside an ELF, present nowhere in the tree and
  named by no `DT_NEEDED`, is something the code intends to open by hand. It
  reports four graphics libraries in this class, all four carried by the same
  file:

  | soname | named inside |
  | --- | --- |
  | `libGLESv2.so.2` | `usr/lib/libepoxy.so.0` |
  | `libGLESv1_CM.so.1` | `usr/lib/libepoxy.so.0` |
  | `libGLX.so.1` | `usr/lib/libepoxy.so.0` |
  | `libOpenGL.so.0` | `usr/lib/libepoxy.so.0` |

  Which corrects an earlier claim here that WebKit opens it: the caller is
  **libepoxy**, the GL dispatch shim GTK and WebKit go through, and it is
  bundled *inside* the AppImage. That is why the requirement travels with the
  artifact and cannot be dropped by changing anything on the host side.

  **What is done about it, per artifact:**

  - the `.deb` **depends on `libgles2`** — the libglvnd package that owns
    `/usr/lib/x86_64-linux-gnu/libGLESv2.so.2` on Debian and Ubuntu. So apt
    resolves it at install time and the abort cannot happen. `ibni.sh` fails the
    build if the measurement names the library and the package does not depend
    on it, so the two cannot drift apart;
  - the **AppImage has no way to express a dependency**, so this is a
    documented hard requirement: a host with no GLES dispatch library cannot run
    the image. It is present wherever a GPU stack is, SteamOS included;
    `WEBKIT_DISABLE_COMPOSITING_MODE=1` gets past it for a headless smoke test
    and is not something to ship.

  It is **not bundled**, on purpose, and for the same reason `libEGL.so.1` and
  `libGL.so.1` are not: on a glvnd system these are the dispatch layer, which
  finds the user's vendor driver. A copy carried inside the image would be
  loaded ahead of the host's and would be the wrong one on somebody's GPU —
  trading a legible abort on a machine with no GL for an illegible one on a
  machine that has it.

  The remaining gap is honest and worth stating: on the AppImage path a host
  without `libGLESv2.so.2` still aborts, with libepoxy's own one-line message on
  stderr and nothing at all if the image was double-clicked. Closing that needs
  a probe before the webview is created, which can only live in
  `apps/studio/src-tauri/src/main.rs` — the earliest code in the image that is
  ours, since linuxdeploy owns `AppRun` and rewrites it after the bundler's file
  map is applied.

## 2. `.deb`

`bundle.linux.deb` in `tauri.conf.json` declares:

| key | value | why |
| --- | --- | --- |
| `depends` | `libwebkit2gtk-4.1-0`, `libgtk-3-0` | the two the webview cannot run without |
| `depends` | `libgles2` | the GLES dispatch library the bundled libepoxy opens **by name**, so it is in no `DT_NEEDED` and its absence aborts the process rather than degrading it (§1). This is the one artifact that can state the requirement instead of documenting it |
| `depends` | `shared-mime-info`, `desktop-file-utils`, `hicolor-icon-theme` | not libraries — **trigger owners**. Each registers a dpkg trigger on a directory this package writes into, so `update-mime-database`, `update-desktop-database` and the icon cache all run on install. That is why the package needs no `postinst`, and it is verified: the built package has `md5sums` and no maintainer scripts |
| `depends` | `xdg-utils` | **added 2026-09-17.** Not a library and not a trigger owner — a program the product *runs*. `tashkhis_awamir::iftah_tashkhis` spawns `xdg-open` by name on Linux with **no fallback at all**, so without it "open the diagnostics folder" is a dead control; `tauri_plugin_opener`, which opens a community translation's page, tries `xdg-open` first and only then `gio open`, `gnome-open`, `kde-open`. The AppImage carries its own copy at `usr/bin/xdg-open` because the bundler puts it there (§8), so leaving this out would make the two artifacts behave differently in the same feature — the exact divergence the `libgles2` row exists to prevent. It is not the `gnome-keyring` case: `xdg-utils` is `Architecture: all`, 323 KB of POSIX shell scripts with no `Depends:` of its own, and nothing daemon-shaped comes with it |
| `recommends` | `gnome-keyring \| libsecret-1-0` | a Secret Service provider. `Recommends` and not `Depends`: the product runs without one, with the keychain features refusing by name, and a hard dependency would drag a keyring daemon onto a machine that deliberately has none |
| `files` | `/usr/share/applications/Taarib.desktop` ← `linux/taarib.desktop` | the Arabic desktop entry (§3), installed **over** the one the bundler generates rather than beside it |
| `files` | `/usr/share/mime/packages/application-vnd.taarib.ruqaa.xml` ← `linux/application-vnd.taarib.ruqaa.xml` | what a `.ruqaa` **is**, for `shared-mime-info`: `TRQ1` magic at offset 0 at priority 70, plus the glob. Without it the desktop entry's `MimeType=` names a type nothing has declared |
| `files` | eight icon paths under `/usr/share/icons/hicolor/*/mimetypes/` ← `ayqunat/ruqaa/*` | the document icon at 16/24/32/48/64/128/256 plus the scalable SVG |

All ten `files` sources were confirmed present and all ten landed in the built
package, so the `.ruqaa` icon set added recently does not break the bundle.

The same ten paths are declared a second time under
`bundle.linux.appimage.files`, AppDir-relative and without the leading slash,
because `deb.files` is a `.deb`-only mechanism (§3a).

One observation from that build, cosmetic and not fixed here: the emitted
`Depends:` reads `libwebkit2gtk-4.1-0, libgtk-3-0, libgles2, shared-mime-info,
desktop-file-utils, hicolor-icon-theme, libwebkit2gtk-4.1-0, libgtk-3-0` — the
two library entries appear twice, because the bundler generates them from the
binary's own `DT_NEEDED` as well. dpkg does not mind. Dropping them from the
configured list would leave the field clean; `libgles2` cannot be dropped that
way, because the binary does not link it and the bundler therefore never
derives it. `xdg-utils`, added since, joins the configured list for the same
reason: the binary does not link it, it spawns it.

## 3. The desktop entry

`apps/studio/src-tauri/linux/taarib.desktop`. Two points are load-bearing:

- `Name=تعريب` with `Name[en]=Taarib`. A desktop environment running an Arabic
  locale shows the Arabic name; every other locale falls back to the `[en]` key.
- `StartupWMClass=taarib-studio`, matching `mainBinaryName`. Without it, a
  launched window is a second, unnamed taskbar entry beside the launcher icon
  instead of grouping under it.

### How it gets installed, and why that is not obvious

The bundler generates its own entry, in English and without `%U`, and offers no
configuration key to suppress it. Until 2026-09-05 this file was installed
*beside* it as `com.cc1a2b.taarib.desktop`, which made things worse rather than
cosmetic — see the next section. What is installed now is this file **over** the
generated one, at the generated path:

```json
"/usr/share/applications/Taarib.desktop": "linux/taarib.desktop"
```

and the same, AppDir-relative, under `bundle.linux.appimage.files`.

That works for a reason worth writing down, because nothing in Tauri's
documentation says it. Both bundlers build the tree first and copy the `files`
map **afterwards**, with `fs::copy`, which overwrites:

| bundler | generation | the `files` map |
| --- | --- | --- |
| `.deb` | `debian.rs:81` → `freedesktop::generate_desktop_file` | `debian.rs:82` |
| AppImage | `appimage/linuxdeploy.rs:80` → the same `debian::generate_data` | `linuxdeploy.rs:82` |

(tauri-bundler as shipped with `@tauri-apps/cli` 2.11.1, which is the version
`Dockerfile` pins.) So naming the generated path replaces the generated file,
and each artifact ships exactly one desktop entry: this one.

Two properties of that arrangement are load-bearing and silent when they break,
which is why `ibni.sh` asserts both on every build (§8):

- **the file name is `productName`'s**, not the reverse-DNS application id.
  `generate_desktop_file` builds it as `{product_name}.desktop`. Rename
  `productName` and the replacement stops replacing anything: the build still
  succeeds and the English entry comes back;
- **the copy order is the bundler's**, not ours. If a future Tauri copies the
  map before generating, the same thing happens.

For the AppImage there is a third consequence, in our favour: the AppDir's
top-level `Taarib.desktop` is a symlink to `usr/share/applications/Taarib.desktop`
(`linuxdeploy.rs:178-181`), so replacing the target replaces what the AppImage
advertises to `appimaged` and to AppRun. AppRun takes the first token of `Exec=`
and forwards its own arguments verbatim, so `%U` costs nothing there and the
path a file manager passes arrives intact — measured against the shipped
`AppRun.wrapped` with a two-argument probe, not assumed.

`desktop-file-validate` passes this file with one standing hint, that
`Categories=Utility;Game;` names two main categories. Left as it is: a
game-arabization tool belongs in both menus, and the cost is a duplicate entry
only in shells that split the menu by main category.

~~This file says `Icon=taarib`~~ **fixed 2026-09-05.** The package installs its
application icons as `hicolor/*/apps/taarib-studio.png`, derived from
`mainBinaryName`; the entry said `Icon=taarib` and therefore resolved to no icon
at all, while the generated English one resolved correctly. It now reads
`Icon=taarib-studio`. This is the failure mode that reports nothing: no launcher
warns about an icon name it cannot find, it just draws the placeholder.

## 3a. The `.ruqaa` association, per artifact

Registering a file type on Linux is two declarations and one trigger. Both
artifacts now carry both declarations; only one of them gets the trigger.

| | what a `.ruqaa` **is** | what opens it | who runs the trigger |
| --- | --- | --- | --- |
| `.deb` | `/usr/share/mime/packages/application-vnd.taarib.ruqaa.xml` | the one desktop entry, `Exec=taarib-studio %U` | `shared-mime-info`'s dpkg trigger runs `update-mime-database`; `desktop-file-utils` runs `update-desktop-database` |
| AppImage | the same file at the same path *inside the AppDir* | the same entry, inside the image | **nobody** |

### The `.deb`, measured

On a clean `ubuntu:22.04`, after `apt-get install ./Taarib_1.0.0_amd64.deb`:

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

That is the whole chain: the type is declared, the magic is registered, one
entry claims it, that entry is the default, and the default carries `%U`. The
last line is content detection — the file is named `t.ruqaa` there, but the
answer comes from the `TRQ1` magic at priority 70, so a patch renamed by a
browser download resolves the same way.

**What still does not follow: the app opening it.** The path reaches the process
as `argv[1]`; nothing consumes it yet. A second launch while the studio is
running hands its arguments to the single-instance plugin, which logs them and
raises the existing window (`apps/studio/src-tauri/src/main.rs:905-915`); a
first launch leaves the argument in `std::env::args()` unread. The opener is a
later phase. So today the association delivers the path and the product ignores
it — which is a different failure from the one this section used to describe,
and the difference matters: it is now one call site away rather than a
packaging problem.

### The AppImage, honestly

`bundle.linux.appimage.files` puts the declaration and the eight document icons
into the AppDir, and `tahaqquq.md` §5.6 has the block. **It buys less than it
looks like.** An AppImage is a file, not an installation: nothing on the system
reads `/usr/share/mime` *inside* a mounted image, and the mount is gone when the
app exits. Desktop integration happens only when `appimaged` is running or the
user integrates the image by hand — and only then does the AppDir's content
become the system's. So on the AppImage path, which is the Steam Deck path,
double-clicking a `.ruqaa` still opens nothing unless the user has integrated
the image, and that is a property of the format.

What the block does buy: a complete AppDir for anyone who *does* integrate,
instead of a desktop entry whose `MimeType=` names a type nothing on the system
defines; and a correct source tree for a Flatpak or any other format built from
the same AppDir. It also costs nothing measurable — ten files, no ELF, so the
glibc floor cannot move, and it does not.

A Flatpak would register it properly, because `flatpak` exports
`share/applications` and `share/mime/packages` from the app and runs the
database updates itself. §9 is why one is still not shipped.

## 4. Where the staged tree lands

`bundle.resources = ["mawarid"]` puts the whole `taarib-tajmee` output in the
resource directory. Tauri's `PathResolver::resource_dir()` doc comment says
`${APPDIR}/usr/lib/${exe_name}` — **that comment is wrong**, and following it
was an error in an earlier revision of this file. The code
(`tauri-utils-2.9.3/src/platform.rs`, `resource_dir_from`) formats
`/usr/lib/{package_info.name}`, and `name` is `productName`, not
`mainBinaryName`. The bundler agrees with the runtime, so both artifacts are
internally consistent:

- **AppImage** — `${APPDIR}/usr/lib/Taarib/mawarid`, `APPDIR` being the
  runtime's mount point; read-only, and it disappears when the image unmounts.
- **`.deb`** — `/usr/lib/Taarib/mawarid`.

Confirmed from a running AppImage on 2026-09-04, which reported the resolved
path as `/tmp/.mount_Taarib…/usr/lib/Taarib/mawarid/bayan_mukawwinat.json`, and
from `dpkg-deb -c` on the package, which installs `usr/lib/Taarib/mawarid/`.
If `productName` ever changes, this path changes with it.

Nothing in the product ever writes these paths by hand: `main.rs` asks
`resource_dir()` and hands the answer to `mukawwinat_tahmil::zamin_mukawwinat`,
which mirrors the component subtree into the writable data root because a
read-only mount that vanishes on exit is not somewhere an installer can read a
framework from three days later.

## 5. Artifact names

Both are named from `productName`, not from `mainBinaryName` — observed, not
predicted:

- `Taarib_<version>_amd64.AppImage`
- `Taarib_<version>_amd64.deb` (the Debian `Package:` field is `taarib`)

## 5a. The glibc floor — read this before shipping a Linux build

An AppImage bundles the WebKitGTK stack but never bundles glibc or `libstdc++`,
so the *host's* copies must be new enough for everything inside the image. The
floor is therefore a property of the machine that performed the build, not of
anything in this repository, and it changes silently whenever that machine is
upgraded. `apps/studio/src-tauri/linux/qias_qaa.sh` measures it out of a built
tree and, given `--haddu-aqsa`, fails the build when it has moved.

### What "the floor" is measured from

Versioned undefined symbols in `.dynsym`, not `ldd`. `ldd` answers "does this
resolve *here*", which is a fact about the machine running it, and on a build
host every requirement resolves by construction. A versioned undefined symbol
answers "what must a host provide", which is a fact about the artifact and
travels with it.

A version family whose definitions are themselves inside the AppDir — `MOUNT_2`
from a bundled `libmount`, say — is not a host requirement and is folded away.
What is left is the real contract with the host.

### Measured, 2026-09-04 build (the rolling-distribution host, glibc 2.42)

| | floor |
| --- | --- |
| `taarib-studio` itself | `GLIBC_2.39` — `pidfd_getpid`, `pidfd_spawnp` |
| bundled libraries | **`GLIBC_2.42`** — one symbol, `__inet_pton_chk`, in `libwebkit2gtk-4.1.so.0` |
| bundled libraries | `GLIBCXX_3.4.30`, `CXXABI_1.3.15`, `GCC_13.0.0` from the host `libstdc++` |
| not bundled at all | `ALSA_0.9` (`libasound`), `GPG_ERROR_1.0` (`libgpg-error`), `ZLIB_1.2.3.4` (`libz`) |

Reproduced on a stock `ubuntu:24.04` container: extracting the image and loading
`libwebkit2gtk-4.1.so.0` against that host's glibc 2.39 gives

```
libwebkit2gtk-4.1.so.0: /lib/x86_64-linux-gnu/libc.so.6:
    version `GLIBC_2.42' not found
```

### Which of that is a hard requirement — the weak-symbol trap

`pidfd_getpid` and `pidfd_spawnp` are `WEAK` undefined symbols, which reads like
an optional reference the loader could skip. It is not. Weakness is a property
of the *symbol*; the loader gates on the *version need* entry, and
`readelf -V` shows `Name: GLIBC_2.39  Flags: none` — no `VER_FLG_WEAK`, so
`_dl_check_map_versions` treats a missing `GLIBC_2.39` as fatal.

Confirmed by running the 2026-09-04 binary on `ubuntu:22.04` (glibc 2.35) with
that host's own `libwebkit2gtk-4.1` installed, so nothing bundled is involved:

```
taarib-studio: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found
taarib-studio: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

**This corrects an earlier claim in this file.** The `.deb` was said not to have
the problem because `Depends:` makes dpkg refuse an unsatisfiable install. It
refuses on *shared-library names* only — the bundler derives dependencies from
`DT_NEEDED`, never from symbol versions — so on a glibc-2.36 host the package
installs cleanly and the program then fails to start. Both artifacts carry the
build host's floor; the `.deb` merely fails later and less legibly.

### What SteamOS provides

Read from Valve's own package mirror, `steamdeck-packages.steamos.cloud`,
`core-<channel>/os/x86_64/`:

| SteamOS channel | glibc |
| --- | --- |
| 3.5 | 2.37 |
| 3.6 | 2.39 |
| 3.7 | 2.40 |
| 3.8.1x (current stable) | 2.41 |

Every one of them is below 2.42. The 2026-09-04 image starts on **no** SteamOS
release that has ever shipped, current included.

### The fix: build in an old-baseline container

`apps/studio/src-tauri/linux/Dockerfile` pins **Ubuntu 22.04**, and
`apps/studio/src-tauri/linux/ibni.sh` drives the whole build through it:

```
apps/studio/src-tauri/linux/ibni.sh
```

22.04 is the newest base whose glibc (2.35) sits under every SteamOS release,
and the oldest base that still carries a maintained `libwebkit2gtk-4.1` — Tauri
v2 needs the 4.1 (libsoup3) API and 20.04 has only 4.0. `libwebkit2gtk-4.1-dev`
is `2.50.4-0ubuntu0.22.04.1` there, so nothing is given up by dropping back.

**`GLIBC_2.35` is the product's Linux floor.** It is asserted, not assumed:
`ibni.sh` runs `qias_qaa.sh … --haddu-aqsa GLIBC_2.35` over the AppDir the
bundler produced and exits non-zero if anything in it asks for more. Raising the
base tag raises the floor; do not bump it without changing that number here and
in the ceiling the script is given.

## 6. Portable installs

The `taarib.mahmul` marker (`tawzee.md` §5) is honoured beside the **outer**
AppImage file rather than beside the binary inside its mount: inside an image,
`current_exe()` points into a temporary mount that is not where the user keeps
anything. `Masarat::iktashif` reads `$APPIMAGE` for exactly this case
(`crates/taarib-usus/src/masarat.rs`). So an AppImage on a USB stick, with an
empty file named `taarib.mahmul` beside it, keeps its whole state in `bayanat/`
on that stick and writes nothing to the machine.

## 7. Updating

- **AppImage** — self-replacing, per `tawzee.md` §6: the verified download is
  staged beside the image, both are flushed, the running image is renamed to
  `*.sabiq`, and the new one takes its place. `taarib_tahdith::tabdil` refuses
  the swap outright when the download and the image are on different
  filesystems, because only a same-filesystem rename can replace a running
  application without a window in which nothing is runnable.
- **`.deb`** — `TareeqatTabdil::Mudar`: the package manager owns it. The updater
  detects the `/usr` prefix and refuses with "this copy was installed by the
  system's package manager; update it from there" rather than fighting `dpkg`.

## 8. Building a release: the one command

```
apps/studio/src-tauri/linux/ibni.sh
```

Three files, all under `apps/studio/src-tauri/linux/`:

| file | what it is |
| --- | --- |
| `Dockerfile` | the Ubuntu 22.04 build image: webkit2gtk-4.1, the GTK stack, the Rust channel from `rust-toolchain.toml`, a Node tarball for the Tauri CLI, `xdg-utils` because the AppImage bundler copies `/usr/bin/xdg-open` into the AppDir and stops at `xdg-open binary not found` without it, and `APPIMAGE_EXTRACT_AND_RUN=1` because an unprivileged container has no FUSE for `linuxdeploy` to mount itself with |
| `ibni.sh` | stages the tree onto a Linux filesystem, runs `tauri build` inside the image, collects the `.deb` and the AppImage into `dist-linux/`, then runs the four gates below |
| `qias_qaa.sh` | the measurement of §5a plus the run-time-`dlopen` scan of §1, usable on its own against any AppDir or unpacked package |

### The trust anchor — the half of "release" that is not the artifact

```
TAARIB_MIFTAH_ISDAR=<64 hex characters> apps/studio/src-tauri/linux/ibni.sh
```

`taarib_khatm::MIRSAT_MALIK` reads that variable through `option_env!` at
compile time. Without it a build anchors to `MIFTAH_TATWIR`, the key committed
in this repository, and the client it produces reports `tatwir` in its
provenance and **refuses every patch the owner actually signed**. Nothing about
such an artifact looks wrong: it installs, starts, scans games, and then says no
to the whole registry. That is why the variable is worth a section.

Until 2026-09-17 `ibni.sh` passed neither the variable nor the feature, and a
`docker run` inherits nothing from the shell that started it, so **every Linux
artifact this path has ever produced is `tatwir`** — 1.0.0 and 1.0.1 included.
What it does now, in order:

1. **validates before any work.** A `TAARIB_MIFTAH_ISDAR` that is not exactly 64
   hexadecimal characters fails in a second, with the command that prints the
   real one, rather than thirty minutes into a container build. Same check, same
   wording as `scripts/isdar.sh`;
2. **names the variable on the `docker run`**, which is the only way it reaches
   cargo and therefore rustc;
3. **adds `--features taarib-khatm/isdar`** inside the container whenever an
   anchor is present. The feature is not decoration: it turns on a `const`
   assertion that this build resolved to the release identity, so an anchor lost
   anywhere between the shell and rustc becomes a compile error instead of a
   silently mis-anchored client;
4. **cleans `taarib-khatm` when the anchor changed.** cargo cannot see an
   `option_env!`, so nothing it fingerprints moves when the key does, and the
   staging tree under `~/.cache/taarib-bina-linux` is reused between runs by
   design. Development ↔ release is caught for free, because the `isdar` feature
   *is* part of the fingerprint; one release key ↔ another is caught by nothing,
   so the script records the anchor it used beside the staging tree and runs
   `cargo clean --release -p taarib-khatm` when this run's differs.

**Without an anchor it builds anyway, and says so — it does not refuse.** That
is `scripts/isdar.sh`'s answer to the same question and there is no reason for
the two to disagree; the Linux case has the stronger argument for it, because
this container is the *only* way to produce a Linux artifact with the right
glibc floor (§5a). Refusing would leave the desktop registration, the AppImage,
the `.deb` and the floor measurement itself with no way to be exercised by
anyone who is not holding the release key. What is not acceptable is a *quiet*
development build, so the identity is stated three times:

- a yellow warning before the image is built, naming the consequence;
- the same warning again after the artifacts are collected, because a
  half-hour build scrolls the first one off the screen;
- `dist-linux/hawiyat_thiqa.txt`, written beside the artifacts — the only one of
  the three that is still there three days later, and the reason it exists.

The private half of the release key is minted inside the owner's keychain by
`cargo run -p taarib-khatm --bin isdar -- wallid` and never leaves it. Nothing
in this directory reads, writes or needs it.

### What the build refuses to ship

Each of these fails silently if it is not asserted, which is the only reason
they are asserted:

1. **the glibc ceiling** — `qias_qaa.sh … --haddu-aqsa GLIBC_2.35` over the
   AppDir the bundler produced. A base image bumped without thinking fails here
   rather than in a stranger's hands;
2. **the desktop registration**, in *both* trees — exactly one `.desktop` under
   `usr/share/applications`, named `Taarib.desktop`, carrying
   `Exec=taarib-studio %U`, `MimeType=application/vnd.taarib.ruqaa;` and
   `Name=تعريب`, with the MIME declaration beside it. This is the §3 mechanism:
   renaming `productName`, or a future bundler that copies the file map before
   generating the entry, silently restores the English entry with no `%U` and
   the build otherwise succeeds;
3. **the GLES dependency** — if the `dlopen` scan names `libGLESv2.so.2`, the
   built package's `Depends:` must contain `libgles2`. If the scan stops naming
   it, the script says so instead, because that means the dependency needs
   revisiting rather than keeping;
4. **the `xdg-open` dependency**, the same shape — if the AppDir carries
   `usr/bin/xdg-open`, the package's `Depends:` must contain `xdg-utils`. The
   two artifacts satisfy one requirement in two different ways, the image by
   carrying the program and the package by naming it (§2), and neither says
   anything when the other stops.

Two decisions in `ibni.sh` worth knowing about:

- **The frontend is built outside the container.** The image carries Node only
  to run the Tauri CLI; `beforeBuildCommand` is overridden to empty through
  `tauri build --config`, which merges over `tauri.conf.json` without touching
  the file. Resolving the studio's npm tree a second time inside the image would
  be an unpinned install of every frontend dependency, and the lockfile in the
  tree is pnpm's while the build script is npm's. So `apps/studio/dist` must
  already exist; the script refuses by name if it does not.
- **The tree is copied, not bind-mounted.** A cargo target directory on a
  DrvFs/9p mount is slower by an order of magnitude, and building straight into
  the working tree would leave root-owned artifacts in it. The staging copy and
  its target directory live under `~/.cache/taarib-bina-linux`, so a second run
  is incremental. `TAARIB_SAQALA` moves that.

## 9. Flatpak — why there is no manifest here yet

Flatpak is the obvious answer to §5a: the app runs against the *runtime's*
glibc, so the host's version stops mattering, and it is the only install path a
Steam Deck offers from Discover. It also fixes §3a for free. Two things stop it,
and the first is not a packaging problem.

**Process visibility.** `taarib_usus::manassa::amaliyat_bism` enumerates running
processes through `sysinfo`, which reads `/proc`. `flatpak run` puts the
application in its own PID namespace with a fresh `/proc`, so the sandbox sees
only itself. Two safety refusals are then silently disarmed:

- `tathbeet_bilnaqra::thabbit` refuses to patch a game that is running. Under
  Flatpak it would find no game process and patch a running game's files.
- `itlaq::KhiyaratSteam::athbit` refuses to rewrite `localconfig.vdf` while
  Steam is running (`MunassaTaamal`). Under Flatpak it would find no Steam
  process and edit a file Steam holds in memory and overwrites on exit — and on
  a Deck, Steam is always running.

Neither failed loudly; both just stopped firing. **The platform layer now knows
better, as of 2026-09-05** (`crates/taarib-usus/src/manassa.rs`):

| item | what it answers |
| --- | --- |
| `fi_sunduq() -> Option<Sunduq>` | which sandbox this build is inside: `Flatpak` (`/.flatpak-info`, or `$FLATPAK_ID`), `Snap` (`$SNAP` **and** `$SNAP_NAME`), `Hawiya` — a container (`$container`, `/run/.containerenv`, `/.dockerenv`) |
| `ruyat_amaliyat() -> RuyatAmaliyat` | whether the process table is the host's (`Kamila`) or the sandbox's (`Maazula { sunduq }`) |
| `halat_tashghil(ism) -> HalatTashghil` | the three-state answer: `Tashtaghil`, `LaTashtaghil`, `GhayrMaaruf { sunduq }` |
| `HalatTashghil::yamnaa()` | whether a guard built on this must refuse — true for everything except `LaTashtaghil` |
| `tashtaghil(ism) -> bool` | unchanged signature, **changed contract**: it is now the fail-safe fold, `halat_tashghil(ism).yamnaa()` |

The last row is the part that matters for safety today. `tashtaghil` is
deliberately not "is it running" any more — it is "must this be treated as
running" — because every caller of it is a refusal, and a check that cannot see
the process table has to refuse rather than wave the operation through. Outside
a sandbox nothing changes: `fi_sunduq()` returns `None` and the answer is the
old one. Inside one, `itlaq::manassa_mughlaqa` now refuses instead of silently
passing.

A positive match is still trusted wherever it is found, because the sandbox's
own processes are real ones; only the *absence* of a match is downgraded.

What deliberately did **not** go in: a comparison of `/proc/self/ns/pid` against
the kernel's initial-namespace inode, which looks like the more direct
measurement and is a trap. WSL2 puts every distribution in its own PID
namespace — measured, `4026532219` against the initial `4026531836` — so that
test calls an ordinary development machine sandboxed and then refuses every
guarded operation on it. The marker files are narrower and correct for the three
ways this product could ever be packaged into a sandbox. A hand-rolled
`unshare -p` with no marker is missed, and is not a packaging format.

**Still to do, and not in this task's ownership** (`crates/taarib-tathbeet`):
the two refusals should consume `halat_tashghil` and name the sandbox in their
message — `HalatTashghil::sunduq()` and `Sunduq::ism()`/`ism_arabi()` exist for
exactly that. `masar_tathbeet::la_tashtaghil` still calls `amaliyat_bism`
directly and so still reads a sandbox's empty list as "the game is not
running"; `itlaq::yashtaghil` goes through `tashtaghil` and is therefore already
fail-safe, but says "Steam is running" where it means "this build cannot see
whether Steam is running".

**Filesystem breadth.** Taarib's job is writing into game directories that ten
different launchers put wherever they like, including SD cards under
`/run/media`. The honest `finish-args` would be close to `--filesystem=host`,
which Flathub reviews hard and which gives away most of what a sandbox is for.
The narrower set — the Steam roots, `/run/media`, and one `~/.var/app/...` entry
per launcher — is writable but leaves every game outside it needing a
`flatpak override`, which is not a step to put in front of a Deck user.

The AppImage keeps both properties: real process visibility, and whatever access
the user already has. It stays the Deck artifact.

So the standing decision is unchanged — **no Flatpak manifest is shipped** — but
the reason has narrowed. The first blocker is no longer "the sandbox lies";
it is "two call sites have not adopted the honest answer yet", and the second
blocker, filesystem breadth, is untouched and is the harder of the two.
