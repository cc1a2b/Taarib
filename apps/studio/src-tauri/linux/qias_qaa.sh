#!/usr/bin/env bash
# قياس القاع — read the true host floor out of a built tree.
#
#     qias_qaa.sh <dir> [--haddu-aqsa GLIBC_2.35] [--kul]
#
# Walks every ELF file under <dir>, collects the versioned undefined symbols
# they import, and reports the highest version of each family that the *host*
# must provide, naming the file and symbol that set it.
#
# WHY A SCRIPT AND NOT A NOTE IN A DOC.
#
# An AppImage carries its libraries but never carries glibc or libstdc++, so the
# floor is a property of the *build host* and it changes silently whenever that
# host is upgraded. It cannot be asserted once. `--haddu-aqsa` makes it a gate:
# the script exits non-zero when anything in the tree demands more than the
# named ceiling, which is what a release pipeline should run.
#
# The measurement is the undefined (`UND`) versioned symbols in `.dynsym`, not
# `ldd` output. `ldd` answers "can this resolve *here*", which is a fact about
# the machine running it. Versioned undefined symbols answer "what must a host
# provide", which is a fact about the artifact and travels with it.
#
# A family whose definitions are themselves inside the tree — `ALSA_0.9` from a
# bundled libasound, say — is not a host requirement, and is folded away unless
# `--kul` asks for everything. Only the families with no bundled provider set
# the floor.
#
# WHAT THE SYMBOL ANALYSIS CANNOT SEE, AND WHY THERE IS A THIRD SECTION.
#
# `DT_NEEDED` is the link-time contract. A library reached by `dlopen` appears
# in no `DT_NEEDED` and in no relocation, so the first two sections are blind to
# it — and WebKit reaches `libGLESv2.so.2` exactly that way. Its absence does
# not degrade the process, it aborts it, which is the worst possible way for a
# missing dependency to be invisible to the tool that enumerates dependencies.
# The third section reads the string tables instead: a name that looks like a
# soname, carried inside an ELF, present nowhere in the tree and named by no
# `DT_NEEDED`, is something the code intends to open by hand at run time.
# It is evidence rather than proof — a soname in a string table may be a
# diagnostic message — so the section reports and never gates.
set -euo pipefail

SHAJARA="${1:?usage: qias_qaa.sh <dir> [--haddu-aqsa GLIBC_2.35] [--kul]}"
shift || true

HADD=""
KUL=0
while (( $# )); do
  case "$1" in
    --haddu-aqsa) HADD="${2:?--haddu-aqsa needs a value such as GLIBC_2.35}"; shift 2 ;;
    --kul) KUL=1; shift ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done

[[ -d "${SHAJARA}" ]] || { printf 'not a directory: %s\n' "${SHAJARA}" >&2; exit 2; }
command -v readelf >/dev/null || { printf 'readelf not found (install binutils)\n' >&2; exit 2; }
command -v strings >/dev/null || { printf 'strings not found (install binutils)\n' >&2; exit 2; }

MATLUB=$(mktemp)
MUWAFFAR=$(mktemp)
MAKTABAT=$(mktemp)
MAWJUD=$(mktemp)
MAFTUH=$(mktemp)
trap 'rm -f "${MATLUB}" "${MUWAFFAR}" "${MAKTABAT}" "${MAWJUD}" "${MAFTUH}"' EXIT

while IFS= read -r -d '' malaf; do
  # An ELF magic test rather than a name test: the tree holds icons, schemas and
  # shell wrappers, and the loaders under gdk-pixbuf/ and gtk-3.0/ carry no
  # extension pattern worth matching.
  [[ $(head -c 4 -- "${malaf}" 2>/dev/null || true) == $'\x7fELF' ]] || continue

  readelf --dyn-syms --wide -- "${malaf}" 2>/dev/null |
    awk -v F="${malaf}" '$0 ~ /UND/ && $0 ~ /@/ {
      for (i = 1; i <= NF; i++) if ($i ~ /@/) { print F "\t" $i; break }
    }' >> "${MATLUB}" || true

  # What the tree *provides* is read from `.gnu.version_d`, the version
  # definition table, and not from defined symbols in `.dynsym`. A copy
  # relocation — `stderr@GLIBC_2.2.5` in any executable that touches stderr —
  # is a defined entry in `.dynsym` while providing nothing, and reading it as
  # a definition would fold the whole GLIBC family away as "bundled" and hide
  # the one number this script exists to print.
  readelf --version-info --wide -- "${malaf}" 2>/dev/null |
    awk '/Version definition section/ { fi = 1; next }
         /Version needs section/      { fi = 0 }
         fi && /Name: / { for (i = 1; i <= NF; i++) if ($i == "Name:") print $(i + 1) }' \
    >> "${MUWAFFAR}" || true

  # `DT_NEEDED` against what the tree actually carries: the difference is the
  # set of shared objects the host has to own. `ldd` cannot answer this — it
  # resolves against the machine it runs on, and every one of these is present
  # on a build host by definition.
  readelf -d --wide -- "${malaf}" 2>/dev/null |
    sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' >> "${MAKTABAT}" || true
  basename -- "${malaf}" >> "${MAWJUD}"
  # A bundled library is found by its SONAME, which need not equal its filename.
  readelf -d --wide -- "${malaf}" 2>/dev/null |
    sed -n 's/.*(SONAME).*\[\(.*\)\]/\1/p' >> "${MAWJUD}" || true

  # Sonames spelled out in the string tables. A versioned suffix is required
  # because that is what a `dlopen` call site carries; `libfoo.so` with no
  # number is a development symlink and belongs to no runtime contract.
  strings -a -- "${malaf}" 2>/dev/null |
    grep -Eo '\blib[A-Za-z0-9_+.-]*\.so\.[0-9]+\b' |
    awk -v F="${malaf}" '{ print F "\t" $0 }' >> "${MAFTUH}" || true
done < <(find "${SHAJARA}" -type f -print0)

sort -u -o "${MAKTABAT}" "${MAKTABAT}"
sort -u -o "${MAWJUD}" "${MAWJUD}"

SHAJARA="${SHAJARA}" HADD="${HADD}" KUL="${KUL}" \
  python3 - "${MATLUB}" "${MUWAFFAR}" "${MAKTABAT}" "${MAWJUD}" "${MAFTUH}" <<'PY'
import collections
import os
import re
import sys

matlub, muwaffar, maktabat, mawjud, maftuh = sys.argv[1:6]
jidhr = os.environ["SHAJARA"].rstrip("/")
hadd = os.environ.get("HADD") or ""
kul = os.environ.get("KUL") == "1"

# `symbol@VER` or `symbol@@VER`, where VER is FAMILY_1.2.3.
namat = re.compile(r"^(?P<ramz>[^@]+)@@?(?P<usra>[A-Za-z_]+?)_(?P<isdar>\d[\d.]*)$")
# A bare version-definition name, as `.gnu.version_d` spells it.
tareef = re.compile(r"^(?P<usra>[A-Za-z_]+?)_(?P<isdar>\d[\d.]*)$")

# Families some bundled library defines for itself. Those never reach the host.
dakhili: set[str] = set()
with open(muwaffar, encoding="utf-8", errors="replace") as fh:
    for satr in fh:
        m = tareef.match(satr.strip())
        if m:
            dakhili.add(m["usra"])

usar: dict[str, list] = collections.defaultdict(list)
with open(matlub, encoding="utf-8", errors="replace") as fh:
    for satr in fh:
        malaf, _, tag = satr.rstrip("\n").partition("\t")
        m = namat.match(tag)
        if m:
            raqm = tuple(int(x) for x in m["isdar"].split("."))
            usar[m["usra"]].append((raqm, m["isdar"], m["ramz"], malaf))

if not usar:
    print(f"no versioned undefined symbols under {jidhr} — nothing to measure")
    sys.exit(2)


def qassir(masar: str) -> str:
    return masar[len(jidhr):] if masar.startswith(jidhr) else masar


mudif = sorted(u for u in usar if kul or u not in dakhili)

print(f"tree: {jidhr}")
print(f"{sum(len(v) for v in usar.values())} versioned imports across "
      f"{len({s[3] for v in usar.values() for s in v})} ELF files")
print()
print("REQUIRED FROM THE HOST — floor per version family"
      + ("  (--kul: bundled families included)" if kul else ""))
qimam = {}
for usra in mudif:
    sufuf = usar[usra]
    aala = max(s[0] for s in sufuf)
    ism = next(s[1] for s in sufuf if s[0] == aala)
    qimam[usra] = (aala, ism)
    mahall = sorted({(s[2], s[3]) for s in sufuf if s[0] == aala})
    marja = "  [also defined inside the tree]" if usra in dakhili else ""
    print(f"  {usra}_{ism}{marja}")
    for ramz, malaf in mahall[:8]:
        print(f"      {ramz:<30} {qassir(malaf)}")
    if len(mahall) > 8:
        print(f"      … and {len(mahall) - 8} further symbols at that version")

matwi = sorted(set(usar) - set(mudif))
if matwi:
    print()
    print("  satisfied inside the bundle, not a host requirement: "
          + ", ".join(matwi))

with open(maktabat, encoding="utf-8", errors="replace") as fh:
    talab = {s.strip() for s in fh if s.strip()}
with open(mawjud, encoding="utf-8", errors="replace") as fh:
    hadir = {s.strip() for s in fh if s.strip()}
naqis = sorted(talab - hadir)
print()
print("REQUIRED FROM THE HOST — shared objects with no copy in the tree")
if naqis:
    for soname in naqis:
        print(f"  {soname}")
else:
    print("  (none — every DT_NEEDED resolves inside the tree)")

def yahmiluhu(soname: str) -> bool:
    """Whether the tree carries this library under any longer version."""
    # `libbz2.so.1` is carried by a file whose SONAME is `libbz2.so.1.0`: the
    # same library, one more version component. Without this the section
    # reports every such pair as a runtime requirement.
    return soname in hadir or any(h.startswith(soname + ".") for h in hadir)


maftuhat: dict[str, list[str]] = collections.defaultdict(list)
with open(maftuh, encoding="utf-8", errors="replace") as fh:
    for satr in fh:
        malaf, _, soname = satr.rstrip("\n").partition("\t")
        # Already covered: carried in the tree, or already reported above as a
        # DT_NEEDED with no copy. What is left is reached by name at run time.
        if soname and soname not in talab and not yahmiluhu(soname):
            maftuhat[soname].append(malaf)

print()
print("REACHED BY NAME AT RUN TIME — a soname in a string table, in no DT_NEEDED")
if maftuhat:
    for soname in sorted(maftuhat):
        print(f"  {soname:<24} {qassir(sorted(maftuhat[soname])[0])}")
    print("  (evidence, not proof: a soname in a string table can also be a"
          " diagnostic message)")
else:
    print("  (none — no ELF names a library the tree neither carries nor links)")

print()
print("GLIBC, PER FILE, WHERE IT EXCEEDS 2.28 (the RHEL 8 / Debian 10 baseline)")
kull = collections.defaultdict(lambda: ((0,), "", ""))
for raqm, ism, ramz, malaf in usar.get("GLIBC", []):
    if raqm > kull[malaf][0]:
        kull[malaf] = (raqm, ism, ramz)
kharij = [(v[0], v[1], v[2], k) for k, v in kull.items() if v[0] > (2, 28)]
if kharij:
    for _, ism, ramz, malaf in sorted(kharij, reverse=True):
        print(f"  GLIBC_{ism:<7} {ramz:<28} {qassir(malaf)}")
else:
    print("  (nothing above GLIBC_2.28)")

if not hadd:
    sys.exit(0)

# The gate. Anything above the ceiling is a regression in the build
# environment, not in the code.
m = re.match(r"^([A-Za-z_]+)_(\d[\d.]*)$", hadd)
if not m:
    print(f"\n--haddu-aqsa must look like GLIBC_2.35, not {hadd!r}")
    sys.exit(2)
usra_h, isdar_h = m[1], tuple(int(x) for x in m[2].split("."))
print()
if usra_h not in qimam:
    print(f"CEILING {hadd}: family absent from the tree — nothing requires it")
    sys.exit(0)
aala, ism = qimam[usra_h]
if aala > isdar_h:
    print(f"CEILING {hadd} BREACHED: the tree requires {usra_h}_{ism}")
    sys.exit(1)
print(f"ceiling {hadd} held: the tree requires at most {usra_h}_{ism}")
PY
