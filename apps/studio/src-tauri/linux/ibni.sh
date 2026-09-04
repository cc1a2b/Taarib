#!/usr/bin/env bash
# ابنِ — build the Linux artifacts inside the old-baseline container.
#
# Usage, from anywhere:
#     apps/studio/src-tauri/linux/ibni.sh [--qias-faqat]
#
#   --qias-faqat   measure the floor of an artifact tree that is already built,
#                  and skip the build itself.
#
# The two artifacts land in `<repo>/dist-linux/` and the measured glibc floor is
# written beside them as `qaa_glibc.txt`.
set -euo pipefail

JIDHR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../../.." && pwd)
SURA=taarib-bina-linux:22.04
MAKHRAJ="${JIDHR}/dist-linux"
# The staging copy lives on a Linux filesystem on purpose: a cargo target
# directory on a DrvFs/9p mount is slower by an order of magnitude, and building
# straight into the working tree would leave root-owned files behind.
SAQALA="${TAARIB_SAQALA:-${HOME}/.cache/taarib-bina-linux}"

qul() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
ikhfaq() { printf '\033[1;31m!!\033[0m %s\n' "$*" >&2; exit 1; }

QIAS_FAQAT=0
[[ "${1:-}" == "--qias-faqat" ]] && QIAS_FAQAT=1

command -v docker >/dev/null || ikhfaq "docker is not on PATH; this build needs it."

if (( QIAS_FAQAT == 0 )); then
  # The frontend is built on the host, not in the container. The container
  # carries Node only to run the Tauri CLI; resolving the studio's own npm tree
  # inside it would mean a second, unpinned install of every frontend
  # dependency, and the lockfile in the tree is pnpm's while the build script is
  # npm's. `beforeBuildCommand` is overridden away below so the container never
  # tries.
  [[ -f "${JIDHR}/apps/studio/dist/index.html" ]] \
    || ikhfaq "apps/studio/dist is missing — run 'npm run build' in apps/studio first."

  qul "building the image (${SURA})"
  docker build -t "${SURA}" -f "${JIDHR}/apps/studio/src-tauri/linux/Dockerfile" \
    "${JIDHR}/apps/studio/src-tauri/linux"

  qul "staging the source tree into ${SAQALA}/masdar"
  mkdir -p "${SAQALA}/masdar" "${SAQALA}/cargo-registry" "${MAKHRAJ}"
  # node_modules and the host target directory are excluded: the first is a
  # host-resolved tree the container must not inherit, the second is built
  # against a different glibc and would be silently reused.
  tar -C "${JIDHR}" -cf - \
      --exclude='./target' \
      --exclude='./dist-linux' \
      --exclude='**/node_modules' \
      --exclude='./apps/studio/src-tauri/target' \
      . | tar -C "${SAQALA}/masdar" -xf -

  qul "compiling in the container"
  docker run --rm \
    -v "${SAQALA}/masdar:/bina" \
    -v "${SAQALA}/cargo-registry:/opt/cargo/registry" \
    -e CARGO_TARGET_DIR=/bina/target-linux \
    -e RUSTUP_TOOLCHAIN=1.95.0 \
    -u "$(id -u):$(id -g)" \
    -e HOME=/tmp \
    "${SURA}" \
    bash -euo pipefail -c '
      cd /bina/apps/studio
      # An empty beforeBuildCommand is how the CLI is told the frontend is
      # already built; the config here is merged over tauri.conf.json and does
      # not touch the file on disk.
      tauri build \
        --bundles deb,appimage \
        --config "{\"build\":{\"beforeBuildCommand\":\"\"}}"
    '

  qul "collecting artifacts into ${MAKHRAJ}"
  rm -rf "${MAKHRAJ:?}"/*.deb "${MAKHRAJ:?}"/*.AppImage
  find "${SAQALA}/masdar/target-linux/release/bundle" \
       \( -name '*.deb' -o -name '*.AppImage' \) -exec cp -f {} "${MAKHRAJ}/" \;
fi

# ---------------------------------------------------------------------------
# The measurement. This is the point of the whole exercise, so it runs every
# time and it runs against the AppDir the bundler actually produced — the
# symbols in the shipped tree, not the ones a manifest claims.
# ---------------------------------------------------------------------------
SHAJARA="${SAQALA}/masdar/target-linux/release/bundle/appimage/Taarib.AppDir"
[[ -d "${SHAJARA}" ]] || ikhfaq "no AppDir at ${SHAJARA}; build first."

# GLIBC_2.35 is Ubuntu 22.04's, and therefore the product's stated Linux floor
# (docs/tawzee/linux.md §5a). Passing it as a ceiling turns the measurement into
# a gate: a base image bumped without thinking fails here rather than in a
# stranger's hands.
qul "measuring the glibc floor"
set +e
"${JIDHR}/apps/studio/src-tauri/linux/qias_qaa.sh" "${SHAJARA}" \
  --haddu-aqsa GLIBC_2.35 | tee "${MAKHRAJ}/qaa_glibc.txt"
QAA=${PIPESTATUS[0]}
set -e
(( QAA == 0 )) || ikhfaq "the glibc ceiling was breached — see ${MAKHRAJ}/qaa_glibc.txt"

# ---------------------------------------------------------------------------
# The desktop registration, asserted rather than assumed.
#
# Both artifacts install the Arabic entry *over* the one the bundler generates,
# by naming the generated path — usr/share/applications/<productName>.desktop —
# in bundle.linux.{deb,appimage}.files. That works because the bundler copies
# the file map after generating the entry, and it is the whole reason the
# package ships one menu entry instead of two and the default handler for a
# .ruqaa carries %U. Both halves are silent when they break: rename
# productName, or let a future bundler copy the map first, and the English
# entry with no %U comes back as the default while everything still builds.
# ---------------------------------------------------------------------------
afhas_mudkhal() {
  local ism="$1" jidhr_shajara="$2"
  local mujallad="${jidhr_shajara}/usr/share/applications"
  local -a mudakhil=()
  [[ -d "${mujallad}" ]] || ikhfaq "${ism}: no usr/share/applications at all"
  mapfile -t mudakhil < <(find "${mujallad}" -maxdepth 1 -name '*.desktop' -printf '%f\n' | sort)
  (( ${#mudakhil[@]} == 1 )) \
    || ikhfaq "${ism}: expected one desktop entry, found ${#mudakhil[@]}: ${mudakhil[*]}"
  [[ ${mudakhil[0]} == Taarib.desktop ]] \
    || ikhfaq "${ism}: the entry is ${mudakhil[0]}, not the generated Taarib.desktop"
  local malaf="${mujallad}/${mudakhil[0]}"
  grep -qx 'Exec=taarib-studio %U' "${malaf}" \
    || ikhfaq "${ism}: the entry's Exec= carries no %U — a double-clicked .ruqaa loses its path"
  grep -qx 'MimeType=application/vnd.taarib.ruqaa;' "${malaf}" \
    || ikhfaq "${ism}: the entry declares no MimeType=application/vnd.taarib.ruqaa"
  grep -q 'Name=تعريب' "${malaf}" \
    || ikhfaq "${ism}: the entry is not the Arabic one — the bundler's own file survived"
  [[ -f "${jidhr_shajara}/usr/share/mime/packages/application-vnd.taarib.ruqaa.xml" ]] \
    || ikhfaq "${ism}: no MIME declaration — MimeType= names a type nothing defines"
  qul "${ism}: one entry, Arabic, %U present, MIME declaration present"
}

qul "checking the desktop registration in both artifacts"
afhas_mudkhal "AppImage" "${SHAJARA}"
SHAJARA_DEB=$(find "${SAQALA}/masdar/target-linux/release/bundle/deb" -maxdepth 2 -type d \
                   -name data -print -quit)
[[ -n "${SHAJARA_DEB}" ]] || ikhfaq "no unpacked .deb tree to check under bundle/deb"
afhas_mudkhal "deb" "${SHAJARA_DEB}"

# libGLESv2.so.2 is opened by name from the bundled libepoxy, so it appears in
# no DT_NEEDED and its absence aborts the process rather than degrading it. The
# .deb is the one artifact that can express the requirement, and it only helps
# while the two facts stay in step: if the measurement stops naming the library,
# or the package stops depending on it, this says so.
if grep -q 'libGLESv2\.so\.2' "${MAKHRAJ}/qaa_glibc.txt"; then
  grep -q '^Depends:.*libgles2' \
    "$(dirname -- "${SHAJARA_DEB}")/control/control" \
    || ikhfaq "the tree opens libGLESv2.so.2 but the package does not depend on libgles2"
  qul "libGLESv2.so.2 is reached by name, and the package depends on libgles2"
else
  qul "libGLESv2.so.2 is no longer reached by name — revisit the libgles2 dependency"
fi

qul "done — artifacts in ${MAKHRAJ}"
ls -la "${MAKHRAJ}"
