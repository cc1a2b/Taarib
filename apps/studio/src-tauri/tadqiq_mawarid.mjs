// تدقيق الموارد — the packaging gate for the staged resource tree.
//
// `bundle.resources` ships `mawarid/`, and that directory is committed empty:
// `crates/taarib-tajmee` fills it on a build machine. Nothing in the bundler
// notices when it was not filled, so a release built without that step produced
// an installer whose `mawarid/` held one README — and every font operation in
// the product then failed on a user's machine with "no usable font", after
// download, install and first launch had all appeared to succeed.
//
// This runs as `build.beforeBundleCommand`, between the compile and the
// bundler, and refuses to let such a build become an installer.
//
// Fonts, and only fonts, are fatal here. A missing plugin component is already
// handled: `zamin_mukawwinat` names it at startup and every install refuses that
// component by name. A missing font is handled by nothing — `taarib-saff` has no
// Arabic fallback shaper, so the product cannot draw a single Arabic word.
//
// Node rather than Python: `build.beforeBuildCommand` already makes Node a build
// prerequisite on every platform, and the Windows build machine has no Python.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Resolved from this file rather than from `process.cwd()`: Tauri runs hooks
// from the app directory, and a path built on that assumption breaks silently
// the first time somebody invokes the CLI from the workspace root.
const HUNA = dirname(fileURLToPath(import.meta.url));
const JIDHR = resolve(HUNA, "..", "..", "..");
const MAWARID = join(HUNA, "mawarid");
const KHUTUT = join(MAWARID, "khutut");

/// The GSUB features an Arabic font must carry, mirroring
/// `SIFAT_GSUB_MATLUBA` in `crates/taarib-saff/src/khatt.rs`.
///
/// `isol` is deliberately absent, and this list must stay four long. A GSUB
/// feature replaces the nominal glyph, and for an Arabic letter the nominal
/// glyph — the one `cmap` yields — already is the isolated form, so a font
/// defines `isol` only when its isolated form differs from that default.
/// Requiring the tag tests an encoding choice, not a capability: IBM Plex Sans
/// Arabic and Dubai both omit it and shape correctly, and one of them is the
/// face this product draws its own interface in.
const SIFAT_GSUB = ["init", "medi", "fina", "rlig"];

/// The GPOS feature an Arabic font must carry. Without it marks do not float
/// slightly wrong; they land on the baseline.
const SIFAT_GPOS = ["mark"];

/// The characters a font must cover, mirroring `HURUF_MATLUBA` in
/// `crates/taarib-saff/src/khatt.rs`: the twenty-eight letters, hamza and the
/// three alef forms lam ligates with, teh marbuta, alef maqsura, the tatweel
/// kashida justification elongates, and the five marks vocalised text needs.
const HURUF = [
  0x0627, 0x0628, 0x062a, 0x062b, 0x062c, 0x062d, 0x062e, 0x062f,
  0x0630, 0x0631, 0x0632, 0x0633, 0x0634, 0x0635, 0x0636, 0x0637,
  0x0638, 0x0639, 0x063a, 0x0641, 0x0642, 0x0643, 0x0644, 0x0645,
  0x0646, 0x0647, 0x0648, 0x064a, 0x0621, 0x0622, 0x0623, 0x0625,
  0x0629, 0x0649, 0x0640, 0x064e, 0x064f, 0x0650, 0x0651, 0x0652,
];

/// The manifest range that marks a family as one the Arabic rule applies to.
/// Read from the declaration rather than guessed from the file name, so a Latin
/// companion is not asked for joining forms it was never meant to have.
const NITAQ_ARABI = "U+0600-06FF";

const shakawa = [];

function ishtaki(satr) {
  shakawa.push(satr);
}

/// Reads a font's table directory. Returns a map of tag to `{ mawqi, tul }`.
///
/// Collections are refused rather than mishandled: nothing in the manifest
/// stages one, and guessing which face inside it was meant would be a way to
/// validate a different font from the one that ships.
function jadwal_sfnt(bayt, ism) {
  if (bayt.length < 12) {
    ishtaki(`${ism}: too short to be a font (${bayt.length} bytes)`);
    return null;
  }
  const naw = bayt.readUInt32BE(0);
  if (naw === 0x74746366) {
    ishtaki(`${ism}: is a TrueType collection; the manifest stages single faces`);
    return null;
  }
  if (naw !== 0x00010000 && naw !== 0x4f54544f && naw !== 0x74727565) {
    ishtaki(`${ism}: not a TrueType or OpenType font (sfnt tag 0x${naw.toString(16)})`);
    return null;
  }
  const adad = bayt.readUInt16BE(4);
  const jadwal = new Map();
  for (let i = 0; i < adad; i += 1) {
    const sijill = 12 + i * 16;
    if (sijill + 16 > bayt.length) {
      ishtaki(`${ism}: table directory runs past the end of the file`);
      return null;
    }
    const wasm = bayt.toString("latin1", sijill, sijill + 4);
    const mawqi = bayt.readUInt32BE(sijill + 8);
    const tul = bayt.readUInt32BE(sijill + 12);
    if (mawqi + tul > bayt.length) {
      ishtaki(`${ism}: table ${wasm.trim()} runs past the end of the file`);
      return null;
    }
    jadwal.set(wasm, { mawqi, tul });
  }
  return jadwal;
}

/// Every feature tag in a GSUB or GPOS `FeatureList`.
///
/// The whole list, not the tags reachable from any one script: a font that
/// registers `init` under `arab` and a font that registers it under `DFLT` both
/// shape, and the runtime check reads the same flat record list.
function sifat_qaima(bayt, mawqi, ism, wasm_jadwal) {
  if (mawqi + 10 > bayt.length) {
    ishtaki(`${ism}: ${wasm_jadwal} header is truncated`);
    return new Set();
  }
  const mawqi_qaima = mawqi + bayt.readUInt16BE(mawqi + 6);
  if (mawqi_qaima + 2 > bayt.length) {
    ishtaki(`${ism}: ${wasm_jadwal} feature list is out of range`);
    return new Set();
  }
  const adad = bayt.readUInt16BE(mawqi_qaima);
  const sifat = new Set();
  for (let i = 0; i < adad; i += 1) {
    const sijill = mawqi_qaima + 2 + i * 6;
    if (sijill + 6 > bayt.length) {
      ishtaki(`${ism}: ${wasm_jadwal} feature record ${i} is out of range`);
      break;
    }
    sifat.add(bayt.toString("latin1", sijill, sijill + 4));
  }
  return sifat;
}

/// The best Unicode `cmap` subtable offset: a full-repertoire format 12 first,
/// then the BMP format 4 every font still carries.
function ikhtar_cmap(bayt, mawqi, ism) {
  if (mawqi + 4 > bayt.length) {
    ishtaki(`${ism}: cmap header is truncated`);
    return null;
  }
  const adad = bayt.readUInt16BE(mawqi + 2);
  let mumtad = null;
  let asasi = null;
  for (let i = 0; i < adad; i += 1) {
    const sijill = mawqi + 4 + i * 8;
    if (sijill + 8 > bayt.length) {
      break;
    }
    const manassa = bayt.readUInt16BE(sijill);
    const tarmiz = bayt.readUInt16BE(sijill + 2);
    const dakhili = mawqi + bayt.readUInt32BE(sijill + 4);
    if (dakhili + 2 > bayt.length) {
      continue;
    }
    const shakl = bayt.readUInt16BE(dakhili);
    const unicode =
      manassa === 0 || (manassa === 3 && (tarmiz === 1 || tarmiz === 10));
    if (!unicode) {
      continue;
    }
    if (shakl === 12 && mumtad === null) {
      mumtad = dakhili;
    } else if (shakl === 4 && asasi === null) {
      asasi = dakhili;
    }
  }
  const mukhtar = mumtad ?? asasi;
  if (mukhtar === null) {
    ishtaki(`${ism}: no Unicode cmap subtable in format 4 or 12`);
  }
  return mukhtar;
}

/// The glyph a codepoint maps to, or `0` for `.notdef` — which is what an
/// uncovered character yields and is therefore a coverage hole, not a glyph.
function muarrif_harf(bayt, mawqi, harf) {
  const shakl = bayt.readUInt16BE(mawqi);
  if (shakl === 12) {
    const majmuat = bayt.readUInt32BE(mawqi + 12);
    for (let i = 0; i < majmuat; i += 1) {
      const sijill = mawqi + 16 + i * 12;
      if (sijill + 12 > bayt.length) {
        break;
      }
      const min = bayt.readUInt32BE(sijill);
      const ila = bayt.readUInt32BE(sijill + 4);
      if (harf >= min && harf <= ila) {
        return (bayt.readUInt32BE(sijill + 8) + (harf - min)) & 0xffff;
      }
    }
    return 0;
  }

  // Format 4: the segmented BMP mapping, read exactly as the specification
  // lays it out — the `idRangeOffset` arithmetic is a byte offset from the
  // slot it was read out of, not an index.
  if (harf > 0xffff) {
    return 0;
  }
  const adad_nisf = bayt.readUInt16BE(mawqi + 6) / 2;
  const nihayat = mawqi + 14;
  const bidayat = nihayat + adad_nisf * 2 + 2;
  const furuq = bidayat + adad_nisf * 2;
  const izahat = furuq + adad_nisf * 2;
  for (let i = 0; i < adad_nisf; i += 1) {
    if (bayt.readUInt16BE(nihayat + i * 2) < harf) {
      continue;
    }
    if (bayt.readUInt16BE(bidayat + i * 2) > harf) {
      return 0;
    }
    const izaha = bayt.readUInt16BE(izahat + i * 2);
    if (izaha === 0) {
      return (harf + bayt.readInt16BE(furuq + i * 2)) & 0xffff;
    }
    const mawqi_muarrif = izahat + i * 2 + izaha + (harf - bayt.readUInt16BE(bidayat + i * 2)) * 2;
    if (mawqi_muarrif + 2 > bayt.length) {
      return 0;
    }
    const muarrif = bayt.readUInt16BE(mawqi_muarrif);
    return muarrif === 0 ? 0 : (muarrif + bayt.readInt16BE(furuq + i * 2)) & 0xffff;
  }
  return 0;
}

/// Applies the Arabic rule to one staged face.
function daqqiq_khatt(bayt, ism, sifat_muallana, arabi) {
  const jadwal = jadwal_sfnt(bayt, ism);
  if (jadwal === null) {
    return;
  }
  for (const wasm of ["cmap", "head", "hhea", "hmtx", "maxp", "name"]) {
    if (!jadwal.has(wasm)) {
      ishtaki(`${ism}: missing the ${wasm} table`);
    }
  }

  const gsub = jadwal.get("GSUB");
  const gpos = jadwal.get("GPOS");
  const sifat = new Set();
  if (gsub) {
    for (const sifa of sifat_qaima(bayt, gsub.mawqi, ism, "GSUB")) {
      sifat.add(sifa);
    }
  }
  if (gpos) {
    for (const sifa of sifat_qaima(bayt, gpos.mawqi, ism, "GPOS")) {
      sifat.add(sifa);
    }
  }

  // A family's own declaration is checked for every face, Arabic or not: a
  // manifest that has drifted from the file is the failure mode this catches.
  for (const sifa of sifat_muallana) {
    if (!sifat.has(sifa)) {
      ishtaki(`${ism}: khutut.json declares ${sifa}, and the font does not carry it`);
    }
  }

  if (!arabi) {
    return;
  }

  if (!gsub) {
    ishtaki(`${ism}: no GSUB table, so Arabic letters would never join`);
  }
  if (!gpos) {
    ishtaki(`${ism}: no GPOS table, so marks would land on the baseline`);
  }
  for (const sifa of SIFAT_GSUB) {
    if (!sifat.has(sifa)) {
      ishtaki(`${ism}: GSUB is missing ${sifa}, which taarib-saff requires`);
    }
  }
  for (const sifa of SIFAT_GPOS) {
    if (!sifat.has(sifa)) {
      ishtaki(`${ism}: GPOS is missing ${sifa}, which taarib-saff requires`);
    }
  }

  const cmap = jadwal.get("cmap");
  if (!cmap) {
    return;
  }
  const dakhili = ikhtar_cmap(bayt, cmap.mawqi, ism);
  if (dakhili === null) {
    return;
  }
  const naqisa = HURUF.filter((harf) => muarrif_harf(bayt, dakhili, harf) === 0);
  if (naqisa.length > 0) {
    const awwal = naqisa.map((harf) => `U+${harf.toString(16).toUpperCase().padStart(4, "0")}`);
    ishtaki(`${ism}: covers none of ${naqisa.length} required character(s): ${awwal.join(" ")}`);
  }
}

function main() {
  const qufl = JSON.parse(readFileSync(join(JIDHR, "assets/aqfal/qufl_khutut.json"), "utf8"));
  const bayan = JSON.parse(readFileSync(join(JIDHR, "assets/fonts/khutut.json"), "utf8"));

  // The lock is the authority on bytes: every staged file must be the exact
  // upstream release the manifest pins, so a face swapped by hand on a build
  // machine cannot reach an installer.
  const mahmula = new Map();
  for (const madkhal of qufl.madakhil) {
    const dhayl = madkhal.wajha ?? madkhal.muarrif;
    const masar = join(KHUTUT, dhayl);
    let bayt;
    try {
      bayt = readFileSync(masar);
    } catch {
      ishtaki(`khutut/${dhayl}: not staged`);
      continue;
    }
    if (bayt.length !== madkhal.hajm) {
      ishtaki(`khutut/${dhayl}: ${bayt.length} bytes, the lock says ${madkhal.hajm}`);
      continue;
    }
    const basma = createHash("sha256").update(bayt).digest("hex");
    if (basma !== madkhal.sha256) {
      ishtaki(`khutut/${dhayl}: sha256 ${basma} does not match the lock`);
      continue;
    }
    mahmula.set(dhayl, bayt);
  }

  // The declaration is the authority on capability: every face `khutut.json`
  // names has to be one of the staged files, and has to shape.
  for (const aila of bayan.khutut) {
    const arabi = (aila.nitaqat ?? []).includes(NITAQ_ARABI);
    for (const malaf of aila.milaffat) {
      const bayt = mahmula.get(malaf.malaf);
      if (bayt === undefined) {
        ishtaki(`${aila.muarrif}: khutut.json names ${malaf.malaf}, which is not staged`);
        continue;
      }
      daqqiq_khatt(bayt, malaf.malaf, aila.sifat ?? [], arabi);
    }
    if (aila.malaf_rukhsa && !mahmula.has(aila.malaf_rukhsa)) {
      ishtaki(`${aila.muarrif}: its licence file ${aila.malaf_rukhsa} is not staged`);
    }
  }

  if (shakawa.length > 0) {
    process.stderr.write("tadqiq_mawarid: the staged resource tree is not shippable\n\n");
    for (const satr of shakawa) {
      process.stderr.write(`  ${satr}\n`);
    }
    process.stderr.write(
      "\nStage it on this machine, then bundle again:\n" +
        "  cargo run -p taarib-tajmee -- --hadaf <target-triple> --jalb\n",
    );
    process.exit(1);
  }

  process.stdout.write(
    `tadqiq_mawarid: ${mahmula.size} staged font file(s) verified against the lock ` +
      `and the Arabic requirement\n`,
  );
}

main();
