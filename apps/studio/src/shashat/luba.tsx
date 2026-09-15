import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { Link, getRouteApi, useNavigate } from '@tanstack/react-router';
import { AnimatePresence, motion } from 'motion/react';
import type { CSSProperties, JSX, KeyboardEvent, ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { HADATH_MARHALAT_TATHBEET, HADATH_TAQADDUM_TANZEEL, KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { ansha } from '@/hayat/tanbihat';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t, wasm } from '@/lugha/lugha';
import type { HalatHimaya } from '@/maktaba/aql';
import { halatHimayaMin, nassLugha } from '@/maktaba/aql';
import type { JahiziyaTashghil } from '@/maktaba/jahiziya';
import { jahiziyaMin, naqsJahiziya, tasil } from '@/maktaba/jahiziya';
import type { Tabaqa } from '@/maktaba/tabaqat';
import {
  ISM_TABAQA,
  KULFAT_TABAQA,
  WASM_TABAQA,
  tabaqaMudkhal,
  tabaqaTaqreer,
} from '@/maktaba/tabaqat';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { IqrarKhatar, muarrifMatlub } from '@/mukawwinat/iqrar_khatar';
import { QismIqrar, useIqrarAwwal } from '@/mukawwinat/qism_iqrar';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import { Zuhur } from '@/mukawwinat/zuhur';
import type {
  AqlLubaHie,
  BinaHie,
  DaleelHie,
  HalatTarjamaMujtama,
  HalatTashghil,
  HasilatIzala,
  HimayaHie,
  Idadat,
  KhuttatIzalaHie,
  KhuttatTathbeetHie,
  LawnBariz,
  Lugha,
  LughaRasmiyaHie,
  MatlabIzala,
  MudifTarjama,
  MudkhalRuqaaHie,
  MuharrikHie,
  NatijatTathbeetHie,
  NawRukhsa,
  NawTanzeelat,
  NizamArqam,
  RuqaaLuba,
  ShahidHie,
  SiyasatIzala,
  TafasilLuba,
  TaghtiyaMujtama,
  TaqaddumTanzeel,
  TaqreerHie,
  TaqreerTahaqquqHie,
  TareeqaMujtama,
  TarjamaMujtamaHie,
  TawzeeTarjama,
} from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka, ismIntiqalGhilaf } from '@/nizam/haraka';

import './luba.css';

/** شاشة اللعبة — one game's real facts, its patches, its safety verdict, and the actions the backend answers for. */

const wajihat = getRouteApi('/luba/$muarrif');

const MIFTAH_TAAKID: Readonly<Record<MatlabIzala, MiftahLugha>> = {
  nass: 'luba.taakid.izalat_nass',
  sawt: 'luba.taakid.izalat_sawt',
  kul: 'luba.taakid.istiada',
};

/**
 * ما تطلبه إزالة واحدة — what one removal asks for.
 *
 * The policy travels with the request rather than in its own state read at send
 * time, so a retry repeats the answer the user actually gave.
 */
interface TalabIzala {
  readonly matlab: MatlabIzala;
  readonly siyasa: SiyasatIzala;
}

/** What a removal that came off whole is confirmed with, per request. */
const MIFTAH_TANBIH_IZALA: Readonly<Record<MatlabIzala, MiftahLugha>> = {
  nass: 'luba.tanbih.izalat_nass',
  sawt: 'luba.tanbih.izalat_sawt',
  kul: 'luba.istiada.najah_kul',
};

/**
 * The registry's verdict on a patch against this build, in the reader's language.
 *
 * Both sentences are computed in Rust from the same `MutabaqatRuqaa` and cross
 * the wire together, so this only picks. It is a function rather than two
 * inline ternaries because the two places that show this verdict — the row's
 * summary line and the approximate-match acknowledgement — were picking
 * differently: the summary rendered Arabic to everyone, and the prompt showed
 * an English reader nothing at all, which is the worse half of the same bug.
 */
function wasfMutabaqa(mudkhal: MudkhalRuqaaHie, lugha: Lugha): string | null {
  return lugha === 'arabi' ? mudkhal.mutabaqa_arabi : mudkhal.mutabaqa_injilizi;
}

/**
 * Which of the three products one patch in the listing installs, and its tier
 * number, as one cell.
 *
 * Every row in the listing carries its own install button, so every row has to
 * name what pressing it produces — and the listing renders its tier name in
 * Arabic alone, so this cell read `طبقة ترجمة` to an English session at the
 * exact moment it was deciding whether to install. `MudkhalRuqaaHie.tabaqa` is
 * the same fact as a discriminant, so the product is named from the string set
 * and the row falls back to the listing's own name only for a tier this build
 * cannot name.
 *
 * The number stays beside it. The registry, the capability report and the
 * review console all label a tier with it, and a reader comparing this row
 * against one of those needs it to still be here.
 */
function tabaqatMudkhal(mudkhal: MudkhalRuqaaHie, lugha: Lugha, munassiq: Munassiqat): string {
  const tabaqa = tabaqaMudkhal(mudkhal);
  return t('luba.muharrik.tabaqa_qeema', lugha, {
    raqm: munassiq.raqm(mudkhal.tabaqa_raqm),
    ism: tabaqa === null ? mudkhal.tabaqa_arabi : t(WASM_TABAQA[tabaqa], lugha),
  });
}

function miftahNaw(naw: MatlabIzala): MiftahLugha {
  if (naw === 'nass') return 'luba.naw.nass';
  if (naw === 'sawt') return 'luba.naw.sawt';
  return 'luba.naw.kul';
}

const ANZIMAT_ARQAM: Readonly<Record<NizamArqam, 'latn' | 'arab' | 'arabext'>> = {
  latini: 'latn',
  arabi: 'arab',
  farisi: 'arabext',
};

function munassiqWaqt(lugha: Lugha, arqam: NizamArqam): Intl.DateTimeFormat {
  return new Intl.DateTimeFormat(wasm(lugha), {
    dateStyle: 'medium',
    timeStyle: 'short',
    numberingSystem: ANZIMAT_ARQAM[arqam],
  });
}

/**
 * Which of a query's faces is showing, as the key `Mashhad` switches on.
 *
 * Empty and filled are one face: the two never replace each other for the same
 * answer, and a refetch that changes the count is a data update, which nothing
 * on this screen animates.
 */
function wajhIstifsar(istifsar: {
  readonly isPending: boolean;
  readonly error: unknown;
}): 'tahmil' | 'khata' | 'jahiz' {
  return istifsar.isPending ? 'tahmil' : istifsar.error !== null ? 'khata' : 'jahiz';
}

/* ---------------------------------------------------------------------------
   لون اللعبة — the per-game accent.

   Every accented rule in `luba.css` resolves through `--lawn-luba`, which reads
   `--tamyeez-luba` and falls back to the product teal. Nothing anywhere set
   `--tamyeez-luba`, so the fallback was the only value the screen ever had: the
   feature did not look broken, it simply did not exist.

   The dominant colour the importer extracts (`SuwarLuba::lawn`) does not cross
   the command boundary — `TafasilLuba` carries the cover's path and not its
   colour — so the screen derives it from the artwork it is already showing, and
   sets it on its own root element only. Scoped there rather than on `:root`
   because two games opened in sequence must not tint each other.
   --------------------------------------------------------------------------- */

/** The sample the histogram runs over. A colour count does not care about
    proportion, and 1024 pixels is a read instead of a megapixel walk. */
const HAJM_AYNA = 32;

/** Below this chroma the artwork has no colour to lend, and the screen keeps
    the product accent rather than painting a grey rule and calling it identity. */
const HADD_ISHBA = 0.12;

/** The same floor stated absolutely. A near-black two levels off neutral passes
    the ratio test on arithmetic alone, and lifting that into the band invents a
    colour out of encoder noise. */
const HADD_MADA = 12;

/** The band the accent is held inside; see {@link fiNitaq} for why these two. */
const ADNA_DAW = 0.18;
const AQSA_DAW = 0.28;

/** Cover art is often near-neutral or fully blown out; either extreme becomes a
    saturation of 1 once its lightness is moved, which vibrates on near-black. */
const AQSA_ISHBA = 0.8;

/** Hue, saturation and lightness, each 0..1. */
interface LawnHsl {
  readonly sabgha: number;
  readonly ishba: number;
  readonly idaa: number;
}

function qanatKhattiya(qanat: number): number {
  const nisba = qanat / 255;
  return nisba <= 0.04045 ? nisba / 12.92 : ((nisba + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance, which is what legibility against a surface is a
    function of — not HSL lightness, which weights every hue the same. */
function daw(lawn: LawnBariz): number {
  return (
    0.2126 * qanatKhattiya(lawn.ahmar) +
    0.7152 * qanatKhattiya(lawn.akhdar) +
    0.0722 * qanatKhattiya(lawn.azraq)
  );
}

function ilaHsl(lawn: LawnBariz): LawnHsl {
  const ahmar = lawn.ahmar / 255;
  const akhdar = lawn.akhdar / 255;
  const azraq = lawn.azraq / 255;
  const aqsa = Math.max(ahmar, akhdar, azraq);
  const adna = Math.min(ahmar, akhdar, azraq);
  const mada = aqsa - adna;
  const idaa = (aqsa + adna) / 2;
  if (mada === 0) {
    return { sabgha: 0, ishba: 0, idaa };
  }
  const ishba = mada / (1 - Math.abs(2 * idaa - 1));
  let sitta: number;
  if (aqsa === ahmar) {
    sitta = (akhdar - azraq) / mada;
  } else if (aqsa === akhdar) {
    sitta = (azraq - ahmar) / mada + 2;
  } else {
    sitta = (ahmar - akhdar) / mada + 4;
  }
  return { sabgha: (sitta / 6 + 1) % 1, ishba, idaa };
}

function minHsl(hsl: LawnHsl): LawnBariz {
  const mada = hsl.ishba * Math.min(hsl.idaa, 1 - hsl.idaa);
  const qanat = (izaha: number): number => {
    const dawra = (izaha + hsl.sabgha * 12) % 12;
    const qeema = hsl.idaa - mada * Math.max(-1, Math.min(dawra - 3, 9 - dawra, 1));
    return Math.min(255, Math.max(0, Math.round(qeema * 255)));
  };
  return { ahmar: qanat(0), akhdar: qanat(8), azraq: qanat(4) };
}

/** Three channels as `#rrggbb`. The same conversion the card makes for its
    generated plate; the two cannot share one while it lives inside a component. */
function sittasi(lawn: LawnBariz): string {
  const qanat = (q: number): string => q.toString(16).padStart(2, '0');
  return `#${qanat(lawn.ahmar)}${qanat(lawn.akhdar)}${qanat(lawn.azraq)}`;
}

/**
 * The extracted colour, moved into a band the interface can actually show.
 *
 * A dominant colour lifted from cover art is whatever the art is: a near-black
 * key art, a blown-out sky, a fully saturated red, a muddy brown. The screen
 * spends it on 2px rules, hairline borders and small labels against
 * `--sath-0..2`, all of which vanish at one end of that range and glare at the
 * other, so the value has to be pulled into a usable band before it is set.
 *
 * The band is stated in relative luminance and brackets the product's own
 * accent: `--tamyeez` (#2a9d8f) measures 0.266 and `--tamyeez-qawi` 0.356.
 * Holding a game's colour to 0.18–0.28 puts it in the same legibility class the
 * product already ships — 4.0:1 to 5.8:1 against the near-black `--sath-1`, and
 * still 3.2:1 to 4.6:1 against the light theme's white, which is where a
 * brighter ceiling would fail. One band has to serve both palettes because the
 * value is computed once and CSS carries it into whichever theme is live.
 *
 * Hue is carried through untouched — the whole point is that the screen reads
 * as *that game's* colour — and only HSL lightness moves. It moves by bisection
 * rather than by a multiplier because luminance is monotonic in lightness but
 * badly non-linear in it: a factor that lands a dark blue in the band sends a
 * yellow far past it.
 *
 * An achromatic source returns null, and the caller then leaves
 * `--tamyeez-luba` unset so `var(--tamyeez-luba, var(--tamyeez))` in the
 * stylesheet does the fallback on its own.
 */
function fiNitaq(lawn: LawnBariz): LawnBariz | null {
  const aqsa = Math.max(lawn.ahmar, lawn.akhdar, lawn.azraq);
  const adna = Math.min(lawn.ahmar, lawn.akhdar, lawn.azraq);
  if (aqsa === 0 || aqsa - adna < HADD_MADA || (aqsa - adna) / aqsa < HADD_ISHBA) {
    return null;
  }

  const asli = ilaHsl(lawn);
  const hsl: LawnHsl = {
    sabgha: asli.sabgha,
    ishba: Math.min(asli.ishba, AQSA_ISHBA),
    idaa: asli.idaa,
  };
  const mabdai = minHsl(hsl);
  const halli = daw(mabdai);
  if (halli >= ADNA_DAW && halli <= AQSA_DAW) {
    return mabdai;
  }

  const hadaf = halli < ADNA_DAW ? ADNA_DAW : AQSA_DAW;
  let asfal = 0;
  let aala = 1;
  let natija = mabdai;
  for (let dawra = 0; dawra < 16; dawra += 1) {
    const wasat = (asfal + aala) / 2;
    natija = minHsl({ sabgha: hsl.sabgha, ishba: hsl.ishba, idaa: wasat });
    if (daw(natija) < hadaf) {
      asfal = wasat;
    } else {
      aala = wasat;
    }
  }
  return natija;
}

/**
 * The cover's dominant colour, by weighted histogram.
 *
 * Pixels are counted into 4096 coarse buckets rather than compared exactly,
 * because the question is which family of colours the art is mostly made of and
 * an exact count over sixteen million values answers a different one. Each vote
 * is weighted by its own chroma and by how far it sits from both ends of the
 * tonal range: cover art is mostly black matte and white type, and an unweighted
 * count returns one of those for every game in the library.
 *
 * Returns null when the canvas cannot be read — the asset response carries the
 * window's origin, so it normally can, and a screen without an accent is the
 * right outcome the one time it does not.
 */
function lawnGhalib(sura: HTMLImageElement): LawnBariz | null {
  if (sura.naturalWidth === 0 || sura.naturalHeight === 0) {
    return null;
  }

  const lawha = document.createElement('canvas');
  lawha.width = HAJM_AYNA;
  lawha.height = HAJM_AYNA;
  const siyagh = lawha.getContext('2d');
  if (siyagh === null) {
    return null;
  }

  let biksilat: Uint8ClampedArray;
  try {
    siyagh.drawImage(sura, 0, 0, HAJM_AYNA, HAJM_AYNA);
    biksilat = siyagh.getImageData(0, 0, HAJM_AYNA, HAJM_AYNA).data;
  } catch {
    return null;
  }

  const awzan = new Float64Array(4096);
  const majamee = new Float64Array(4096 * 3);
  let aqwa = -1;
  let wazenAqwa = 0;

  for (let mawqi = 0; mawqi + 3 < biksilat.length; mawqi += 4) {
    if ((biksilat[mawqi + 3] ?? 0) < 128) {
      continue;
    }
    const ahmar = biksilat[mawqi] ?? 0;
    const akhdar = biksilat[mawqi + 1] ?? 0;
    const azraq = biksilat[mawqi + 2] ?? 0;
    const aqsa = Math.max(ahmar, akhdar, azraq);
    const adna = Math.min(ahmar, akhdar, azraq);
    if (aqsa === 0) {
      continue;
    }

    const idaa = (aqsa + adna) / 510;
    const wazn = ((aqsa - adna) / aqsa) * (1 - Math.abs(2 * idaa - 1));
    if (wazn <= 0) {
      continue;
    }

    const khana = ((ahmar >> 4) << 8) | ((akhdar >> 4) << 4) | (azraq >> 4);
    const majmu = (awzan[khana] ?? 0) + wazn;
    awzan[khana] = majmu;
    majamee[khana * 3] = (majamee[khana * 3] ?? 0) + ahmar * wazn;
    majamee[khana * 3 + 1] = (majamee[khana * 3 + 1] ?? 0) + akhdar * wazn;
    majamee[khana * 3 + 2] = (majamee[khana * 3 + 2] ?? 0) + azraq * wazn;
    if (majmu > wazenAqwa) {
      wazenAqwa = majmu;
      aqwa = khana;
    }
  }

  if (aqwa < 0 || wazenAqwa <= 0) {
    return null;
  }

  return {
    ahmar: Math.round((majamee[aqwa * 3] ?? 0) / wazenAqwa),
    akhdar: Math.round((majamee[aqwa * 3 + 1] ?? 0) / wazenAqwa),
    azraq: Math.round((majamee[aqwa * 3 + 2] ?? 0) / wazenAqwa),
  };
}

/**
 * The game's accent as a CSS colour, or null when nothing can supply one.
 *
 * Two sources, in order. The importer stores a colour beside every cover it
 * fetches — clustered in Oklab over the whole image, with the status hues
 * rejected so a red cover cannot dress the screen as an error — and that
 * arrives on the wire as a field read, before the artwork has decoded. It
 * still passes through `fiNitaq`, because the importer kept the dominant
 * colour and this screen needs one that stays legible on its own surfaces.
 *
 * The probe below is the fallback for a row that carries a cover but no
 * colour, which a store written before the field existed can do. It is a
 * detached image rather than the one already on screen: it has to ask for the
 * asset with CORS for the canvas it is drawn into to stay readable, and
 * putting that attribute on the visible cover would mean a refused request
 * blanks the artwork the user opened the screen to look at. Here the same
 * refusal costs an accent and nothing else.
 */
function useLawnLuba(masdar: string | null, muqaddam: LawnBariz | null): string | null {
  const [lawn, setLawn] = useState<string | null>(null);

  // Keyed on the channels, not the object: the query hands back a fresh
  // object on every refetch and the colour has not changed.
  const ahmar = muqaddam?.ahmar ?? null;
  const akhdar = muqaddam?.akhdar ?? null;
  const azraq = muqaddam?.azraq ?? null;
  const minWire = useMemo(() => {
    if (ahmar === null || akhdar === null || azraq === null) {
      return null;
    }
    const munaqqa = fiNitaq({ ahmar, akhdar, azraq });
    return munaqqa === null ? null : sittasi(munaqqa);
  }, [ahmar, akhdar, azraq]);

  useEffect(() => {
    // Cleared before the next cover decodes, so the previous game's colour is
    // never the one a newly opened game is painted in.
    setLawn(null);
    if (masdar === null || ahmar !== null) {
      return undefined;
    }

    let mulgha = false;
    const sura = new Image();
    sura.crossOrigin = 'anonymous';
    sura.decoding = 'async';
    sura.onload = () => {
      if (mulgha) {
        return;
      }
      const khaam = lawnGhalib(sura);
      const munaqqa = khaam === null ? null : fiNitaq(khaam);
      setLawn(munaqqa === null ? null : sittasi(munaqqa));
    };
    sura.onerror = () => {
      if (!mulgha) {
        setLawn(null);
      }
    };
    sura.src = masdar;

    return () => {
      mulgha = true;
      sura.onload = null;
      sura.onerror = null;
    };
  }, [masdar, ahmar]);

  return ahmar !== null ? minWire : lawn;
}

interface KhasaisSaff {
  readonly unwan: string;
  readonly children: ReactNode;
}

function Saff({ unwan, children }: KhasaisSaff): JSX.Element {
  return (
    <div className="luba__saff">
      <dt>{unwan}</dt>
      <dd>{children}</dd>
    </div>
  );
}

function Riqaqat({ qaima }: { readonly qaima: readonly string[] }): JSX.Element {
  return (
    <ul className="luba__riqaq">
      {/* `dir="auto"` because these are the backend's own words and it writes
          some of them in Arabic whatever the session's language is — a graphics
          API it could not determine is `غير محدَّدة`. Laid out left to right
          inside an English panel, such a chip puts its first word last. */}
      {qaima.map((band) => (
        <li key={band} className="luba__riqaqa" dir="auto">
          {band}
        </li>
      ))}
    </ul>
  );
}

/* ---------------------------------------------------------------------------
   جاهزية التشغيل — whether the tier the report names actually runs.

   A tier says what Taarib is entitled to do to a game. It does not say that the
   half which does it exists yet, and for an engine whose adapter stops at an
   unfinished seam the screen would otherwise promise Arabic that never appears.
   The report answers this in `jahiziya` and names the gap in `naqs`.

   The narrowing itself moved to `maktaba/jahiziya`, because three surfaces now
   read the same verdict — this screen, the library card and the automatic-run
   screen — and three copies of one three-armed union is how a build ends up
   treating an unknown fourth verdict as a warning on one screen and as silence
   on another.
   --------------------------------------------------------------------------- */

/**
 * The engine family, in the reader's language.
 *
 * `aila` is `AilatMuharrik::ism` and that function answers the unidentified
 * engine with the Arabic literal `غير معروف` whatever the session's language
 * is — so this row, which is the most consequential one on the screen for the
 * large part of a real library that probes as unknown, was the row an English
 * reader could not read. `aila_ramz` is the same fact as a discriminant, so the
 * miss is named from the string set and every family Taarib does recognise
 * keeps its own name, which is not a translatable thing.
 */
function ailaMuharrik(muharrik: MuharrikHie, lugha: Lugha): string {
  return muharrik.aila_ramz === 'majhul' ? t('luba.muharrik.majhul', lugha) : muharrik.aila;
}

/**
 * The engine, as the readiness notice names it: family, and version if known.
 *
 * `null` when nothing was identified, and the notice then names no engine at
 * all. "Engine detected: not identified" is a sentence that contradicts itself,
 * and the readiness verdict it introduces is about this build of Taarib rather
 * than about the engine, so it stands perfectly well without the line.
 */
function ismMuharrik(muharrik: MuharrikHie): string | null {
  if (muharrik.aila_ramz === 'majhul') {
    return null;
  }
  return muharrik.isdar === null ? muharrik.aila : `${muharrik.aila} ${muharrik.isdar}`;
}

/**
 * The named limitations, in the reader's own language.
 *
 * This list was drawn from the Arabic field in both languages, which put a
 * paragraph of Arabic under four English headings — in the one part of the
 * report that says what will *not* work, which is the last part a reader can be
 * asked to guess at. A limit named in only one language still stands, so an
 * empty English list falls back to the Arabic rather than to silence.
 */
function hududQira(taqreer: TaqreerHie, lugha: Lugha): readonly string[] {
  if (lugha === 'arabi') {
    return taqreer.hudud;
  }
  return taqreer.hudud_injilizi.length > 0 ? taqreer.hudud_injilizi : taqreer.hudud;
}

/**
 * The tier's own name as the report renders it, in the reader's language.
 *
 * Only reached for a tier this build cannot name — a backend one version ahead
 * of it — and it exists so that case degrades to the backend's own words rather
 * than to a blank heading. It fixes a live bug on the way: the report used to
 * send its tier name in Arabic alone, so the single line on this screen that
 * says which of the three products a game gets read `طبقة ترجمة` to an English
 * session that had just read the tier number, the reason and the systems in
 * English. Falls back to the Arabic rather than to silence, on the same terms
 * as {@link hududQira}: a tier named in one language is still named.
 */
function ismTabaqaTaqreer(taqreer: TaqreerHie, lugha: Lugha): string {
  if (lugha === 'arabi') {
    return taqreer.tabaqa_arabi;
  }
  return taqreer.tabaqa_injilizi.length > 0 ? taqreer.tabaqa_injilizi : taqreer.tabaqa_arabi;
}

/**
 * What the tier does, as the backend writes it, in the reader's language.
 *
 * `Tabaqa` decides what a tier means and now says so on the wire, so this
 * screen prints that sentence instead of one the interface kept for itself.
 * The three locale paragraphs it replaces were deleted rather than left in
 * place: two copies of the same explanation drift, and the copy that drifts is
 * the one that is not recompiled when the tier's behaviour changes.
 */
function sharhTabaqa(taqreer: TaqreerHie, lugha: Lugha): string {
  if (lugha === 'arabi') {
    return taqreer.sharh_arabi;
  }
  return taqreer.sharh_injilizi.length > 0 ? taqreer.sharh_injilizi : taqreer.sharh_arabi;
}

/**
 * What an unfinished adapter means for the person reading, per verdict.
 *
 * The one sentence the report cannot write for itself. `naqs` states, in the
 * backend's own words, *what* is missing; this states what follows from it — the
 * patch installs, the game starts, and the text does not change — and that the
 * gap is in this build of Taarib rather than in the game the reader owns, which
 * is the difference between a limitation and a wait.
 */
const ATHAR_JAHIZIYA: Readonly<Record<'naqisa' | 'ghaiba', MiftahLugha>> = {
  naqisa: 'luba.jahiziya.athar_juzii',
  ghaiba: 'luba.jahiziya.athar',
};

interface KhasaisJahiziya {
  readonly unwan: string;
  /** The engine the verdict is about, or null where the panel already names it. */
  readonly muharrik: string | null;
  /**
   * What is unfinished, in the backend's words, or null.
   *
   * Null for a report stored by a build from before the gap was named. The
   * notice still draws: the verdict is known even when the sentence explaining
   * it is not, and a screen that went silent because it was one sentence short
   * would be back to promising a tier nothing delivers.
   */
  readonly nass: string | null;
  /** What that means for the reader. */
  readonly athar: string;
  readonly lugha: Lugha;
}

/**
 * The runtime-readiness notice.
 *
 * Three lines and a label, in the order a reader needs them: which engine this
 * is about, what does not work, and what that means for them. It used to be the
 * label and the middle line alone, which told a user that "the Unity adapter
 * stops at the font atlas" without ever telling them that installing anyway
 * leaves the game in English — the fact they were actually about to act on.
 *
 * `role="note"` rather than `alert`: it is true of the screen from the moment it
 * opens rather than the result of anything the reader did, and an alert would
 * interrupt a screen reader mid-sentence on every game in the library.
 */
function TanbeehJahiziya({ unwan, muharrik, nass, athar, lugha }: KhasaisJahiziya): JSX.Element {
  return (
    <div className="luba__jahiziya" role="note">
      <p className="luba__jahiziya-unwan">{unwan}</p>
      {muharrik === null ? null : (
        <p className="luba__jahiziya-muharrik">
          {t('luba.jahiziya.muharrik', lugha, { muharrik })}
        </p>
      )}
      {nass === null ? null : <p className="luba__jahiziya-nass">{nass}</p>}
      <p className="luba__jahiziya-athar">{athar}</p>
    </div>
  );
}

/* ---------------------------------------------------------------------------
   ما الذي تحصل عليه — which of the three products this game gets.

   The compatibility panel used to open with a table of engine internals and
   state the product as row five of it: "Tier 3 — طبقة ترجمة". Two things are
   wrong with that and neither is cosmetic.

   A tier number is a rank, and a rank invites the reading that three is a worse
   two — the same thing, slightly degraded. It is not. Tier 1 replaces the game's
   text inside its engine and the result is the game's own text; tier 2 has
   Taarib draw the words itself where the engine's words were; tier 3 does not
   touch the game at all and paints Arabic over the picture, which disappears
   when Taarib is closed and was never inside the game to begin with. Three
   products, three sets of consequences, and nobody can infer any of it from a
   digit.

   And the Arabic name was printed verbatim in both languages, so an English
   reader got `طبقة ترجمة` — the one row on the screen that says which product
   they are about to install.

   So the panel now opens with the product, says what it does and what it costs,
   carries the report's own reason for landing there, and demotes the number to
   a footnote. The number stays because it is what the capability report, the
   registry and the patch listing all label a tier with, and a reader comparing
   this screen against one of those needs it to still be here.

   WHAT IS NOT DECIDED HERE. Which tier a game is on is decided in Rust and read
   off the wire: `TaqreerHie.tabaqa` is the discriminant, `sharh_arabi` and
   `sharh_injilizi` are the tier's own explanation of itself, and both are
   printed rather than paraphrased. The tier number is deliberately NOT inverted
   into a tier for the one case the discriminant does not cover — a backend one
   version ahead of this build — because that would be a second copy of
   `Tabaqa::raqm` in TypeScript, and a copy of a taxonomy mislabels rather than
   fails the day the taxonomy moves. That case falls back to the report's own
   rendered tier name, now sent in both languages.

   The one paragraph that is still the interface's own is what the tier COSTS.
   It has no field, it is true of every game on a tier, and it is in
   `maktaba/tabaqat` with the rest of the vocabulary.
   --------------------------------------------------------------------------- */

interface KhasaisMuntaj {
  /** The product, off the wire; null only for a tier this build cannot name. */
  readonly tabaqa: Tabaqa | null;
  readonly taqreer: TaqreerHie;
  /** Whether the product actually runs in this build, for the qualification. */
  readonly jahiziya: JahiziyaTashghil | null;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

function QismMuntaj({ tabaqa, taqreer, jahiziya, lugha, munassiq }: KhasaisMuntaj): JSX.Element {
  const marfud = taqreer.marfuda;
  // Read off the verdict rather than off the sentence under it: an older report
  // carries the verdict without the sentence, and the product would then be
  // stated flat for a game nothing will change.
  const muallaq = jahiziya !== null && jahiziya !== 'mukammala';
  const sabab = lugha === 'arabi' ? taqreer.sabab_arabi : taqreer.sabab_injilizi;

  return (
    <div className={marfud ? 'luba__muntaj luba__muntaj--marfud' : 'luba__muntaj'}>
      <p className="luba__muntaj-unwan">
        {marfud ? (
          t('luba.muntaj.marfuda', lugha)
        ) : tabaqa === null ? (
          // A tier this build has no name for, stated in the report's own
          // words. Inline `dir="auto"` rather than on the paragraph: the report
          // sends both languages now, but a build that fell back to the Arabic
          // gets the bidi right without flipping a whole heading to the trailing
          // edge of an otherwise left-to-right panel.
          <span dir="auto">{ismTabaqaTaqreer(taqreer, lugha)}</span>
        ) : (
          t(ISM_TABAQA[tabaqa], lugha)
        )}
        {/* The qualification belongs on the claim rather than three lines under
            it. A refused game does not get one: the refusal is permanent, and
            "does not run yet in this build" beside it would offer a wait that is
            not coming. */}
        {marfud || !muallaq ? null : (
          <span className="luba__tabaqa-muallaqa">{t('luba.jahiziya.ghayr_faal', lugha)}</span>
        )}
      </p>
      {marfud ? null : (
        <>
          {/* `Tabaqa`'s own explanation of itself, printed and not paraphrased.
              Not conditional on the discriminant, because the sentence and the
              verdict come from the same place: a tier this build cannot name
              still says what it does. `dir="auto"` for the one case that puts
              Arabic in an English panel — a report whose English half is empty,
              which falls back rather than going silent. */}
          <p className="luba__muntaj-sharh" dir="auto">
            {sharhTabaqa(taqreer, lugha)}
          </p>
          {/* What the product costs, as opposed to what this game's engine
              costs. The per-game limits are the report's own list further down;
              this is the price of the tier itself and is true of every game on
              it, which is why the report has no reason to repeat it per game.
              The one paragraph here still keyed on the discriminant: it has no
              field, so an unnameable tier gets no cost line rather than the
              wrong one. */}
          {tabaqa === null ? null : (
            <p className="luba__muntaj-kulfa">{t(KULFAT_TABAQA[tabaqa], lugha)}</p>
          )}
        </>
      )}
      <p className="luba__sabab">{sabab}</p>
      <p className="luba__muntaj-raqm">
        {t('luba.muntaj.tabaqa_raqm', lugha, { raqm: munassiq.raqm(taqreer.tabaqa_raqm) })}
      </p>
    </div>
  );
}

const TULAT_SATR = ['tawil', 'mutawassit', 'qasir'] as const;

/**
 * One panel-shaped placeholder.
 *
 * Both columns are replaced by bordered `.luba__qism` sections, so the skeleton
 * has to be a panel too: loose lines on the page ground reserve neither the
 * padding nor the hairline, and the column steps the moment the answer lands.
 */
function HaykalQism({ sutur }: { readonly sutur: number }): JSX.Element {
  return (
    <div className="luba__haykal-qism">
      <span className="luba__haykal-unwan-qism" />
      {Array.from({ length: sutur }, (_, fihris) => (
        <span
          key={fihris}
          className={`haykal__satr haykal__satr--${TULAT_SATR[fihris % 3] ?? 'tawil'}`}
        />
      ))}
    </div>
  );
}

/**
 * The lines a panel's own query reserves while it runs.
 *
 * Inside a panel that is already drawn, so no second border: a few lines where
 * the answer will stand, after the same delay every skeleton waits so a warm
 * answer never flashes them. The sentence a screen reader hears is the one the
 * text line used to show; the shapes are for the eye alone.
 */
function HaykalSutur({
  sutur,
  nass,
}: {
  readonly sutur: number;
  readonly nass: string;
}): JSX.Element {
  return (
    <div className="luba__haykal-sutur zuhur-muakhkhar">
      <span className="khafi" role="status">
        {nass}
      </span>
      {Array.from({ length: sutur }, (_, fihris) => (
        <span
          key={fihris}
          aria-hidden="true"
          className={`haykal__satr haykal__satr--${TULAT_SATR[fihris % 3] ?? 'tawil'}`}
        />
      ))}
    </div>
  );
}

interface KhasaisTaqreer {
  readonly muarrif: string;
  readonly muharrik: MuharrikHie;
  readonly taqreer: TaqreerHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly yajriFahs: boolean;
  readonly alaAadaFahs: () => void;
  readonly khataFahs: KhataJisr | null;
}

function QismMuharrik({
  muarrif,
  muharrik,
  taqreer,
  lugha,
  munassiq,
  yajriFahs,
  alaAadaFahs,
  khataFahs,
}: KhasaisTaqreer): JSX.Element {
  const [dalailZahira, setDalailZahira] = useState(false);
  useEffect(() => {
    setDalailZahira(false);
  }, [muarrif]);

  const dalail = useQuery<DaleelHie[], KhataJisr>({
    queryKey: mafatih.dalail(muarrif),
    queryFn: () => nadi('dalail_muharrik', { muarrif }),
    enabled: dalailZahira,
  });

  const naqs = naqsJahiziya(taqreer, lugha);
  const hukmJahiziya = jahiziyaMin(taqreer.jahiziya);
  const hudud = hududQira(taqreer, lugha);

  return (
    <section className="luba__qism" aria-labelledby="luba-unwan-tawafuq">
      <h2 id="luba-unwan-tawafuq" className="luba__unwan-qism">
        {t('luba.tawafuq.unwan', lugha)}
      </h2>
      {/* The product first, before the engine's internals. What this game gets
          is the question the reader opened the screen with; the scripting
          backend and the graphics API are the evidence for the answer, and
          evidence goes under a finding rather than in front of it. */}
      <h3 className="luba__unwan-farii">{t('luba.muntaj.unwan', lugha)}</h3>
      <QismMuntaj
        tabaqa={tabaqaTaqreer(taqreer)}
        taqreer={taqreer}
        jahiziya={hukmJahiziya}
        lugha={lugha}
        munassiq={munassiq}
      />
      {hukmJahiziya === null || hukmJahiziya === 'mukammala' ? null : (
        <TanbeehJahiziya
          unwan={t(
            hukmJahiziya === 'naqisa' ? 'luba.jahiziya.unwan_juzii' : 'luba.jahiziya.unwan',
            lugha,
          )}
          muharrik={ismMuharrik(muharrik)}
          nass={naqs}
          athar={t(ATHAR_JAHIZIYA[hukmJahiziya], lugha)}
          lugha={lugha}
        />
      )}
      <h3 className="luba__unwan-farii">{t('luba.muharrik.unwan', lugha)}</h3>
      <dl className="luba__jadwal">
        {/* The engine family, off `aila_ramz` rather than off the rendered
            name. `AilatMuharrik::ism` answers the unidentified engine with the
            Arabic literal `غير معروف` in both languages, and that is the single
            most important row on this screen for the large part of a real
            library that probes as unknown — so it is the row that has to be
            readable. The rendered name survives for every family Taarib does
            recognise, because "Unity" is not a translatable thing.

            `dir="auto"` stays for the scripting backend below, which the
            backend still answers in Arabic — `غير معروفة` — whatever the
            session's language, and which laid out left to right reads back to
            front. */}
        <Saff unwan={t('luba.muharrik.aila', lugha)}>
          <span dir="auto">{ailaMuharrik(muharrik, lugha)}</span>
        </Saff>
        {muharrik.isdar === null ? null : (
          <Saff unwan={t('luba.muharrik.isdar', lugha)}>
            <span className="mono-ltr">{muharrik.isdar}</span>
          </Saff>
        )}
        <Saff unwan={t('luba.muharrik.khalfiya', lugha)}>
          <span dir="auto">{muharrik.khalfiya}</span>
        </Saff>
        {muharrik.rusum.length > 0 ? (
          <Saff unwan={t('luba.muharrik.rusum', lugha)}>
            <Riqaqat qaima={muharrik.rusum} />
          </Saff>
        ) : null}
        <Saff unwan={t('luba.muharrik.thiqa', lugha)}>{munassiq.nisba(muharrik.thiqa)}</Saff>
      </dl>
      <div className="luba__imkaniyat">
        {taqreer.anzimat.length > 0 ? (
          <>
            <h3 className="luba__unwan-farii">{t('luba.imkaniyat.anzima', lugha)}</h3>
            <Riqaqat qaima={taqreer.anzimat} />
          </>
        ) : null}
        {hudud.length > 0 ? (
          <>
            <h3 className="luba__unwan-farii">{t('luba.imkaniyat.hudud', lugha)}</h3>
            <ul className="luba__hudud">
              {/* The backend writes a limit in one language and not always in
                  both, and `hududQira` falls back to the Arabic rather than to
                  silence — a limit named in one language still stands. So the
                  list has to be able to typeset a right-to-left sentence inside
                  a left-to-right panel. */}
              {hudud.map((hadd, martaba) => (
                <li key={`${String(martaba)}:${hadd}`} dir="auto">
                  {hadd}
                </li>
              ))}
            </ul>
          </>
        ) : null}
        {taqreer.marfuda ? (
          <p className="luba__marfuda">{t('luba.imkaniyat.marfuda', lugha)}</p>
        ) : null}
      </div>
      <div className="luba__fahs luba__fahs--dhayl">
        <div className="luba__saff-afal">
          <button type="button" className="zir" aria-busy={yajriFahs} onClick={alaAadaFahs}>
            {t(yajriFahs ? 'luba.muharrik.jari_fahs' : 'luba.muharrik.aada_fahs', lugha)}
          </button>
          <button
            type="button"
            className="zir"
            aria-expanded={dalailZahira}
            aria-controls="luba-dalail"
            onClick={() => {
              setDalailZahira((hali) => !hali);
            }}
          >
            {t(dalailZahira ? 'luba.dalail.ikhfa' : 'luba.dalail.zirr', lugha)}
          </button>
        </div>
        {khataFahs !== null ? (
          <KutlatKhata
            unwan={t('luba.khata.amal', lugha)}
            khata={khataFahs}
            lugha={lugha}
            muarrif={muarrif}
            aada={alaAadaFahs}
          />
        ) : null}
        <Zuhur maftuh={dalailZahira} id="luba-dalail" className="luba__kashf">
          <Mashhad miftah={wajhIstifsar(dalail)}>
            {dalail.isPending ? (
              <HaykalSutur sutur={3} nass={t('luba.dalail.jari', lugha)} />
            ) : dalail.error !== null ? (
              <KutlatKhata
                unwan={t('luba.dalail.taadhur', lugha)}
                khata={dalail.error}
                lugha={lugha}
                muarrif={muarrif}
                aada={() => {
                  void dalail.refetch();
                }}
              />
            ) : dalail.data === undefined || dalail.data.length === 0 ? (
              <HalatFarigha unwan={t('luba.dalail.la_shay', lugha)} />
            ) : (
              <ul className="luba__adilla">
                {dalail.data.map((daleel, fihris) => (
                  <li
                    key={`${daleel.naw}-${String(fihris)}`}
                    className="luba__daleel luba__daleel--muharrik"
                  >
                    <p className="luba__daleel-nass">{daleel.wasf}</p>
                    {daleel.mawqi === null ? null : (
                      <p className="mono-ltr luba__daleel-masar luba__qat" title={daleel.mawqi}>
                        {daleel.mawqi}
                      </p>
                    )}
                    <p className="luba__daleel-wazn">
                      {t('luba.dalail.wazn', lugha, { adad: munassiq.raqm(daleel.wazn) })}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </Mashhad>
        </Zuhur>
      </div>
    </section>
  );
}

interface KhasaisHimaya {
  readonly muarrif: string;
  readonly himaya: HimayaHie | null;
  readonly lugha: Lugha;
  readonly yajri: boolean;
  readonly alaFahs: () => void;
  readonly khata: KhataJisr | null;
}

function QismHimaya({ muarrif, himaya, lugha, yajri, alaFahs, khata }: KhasaisHimaya): JSX.Element {
  const mahmiya = himaya?.mahmiya === true;
  return (
    <section
      className={mahmiya ? 'luba__qism luba__qism--khatar' : 'luba__qism'}
      aria-labelledby="luba-unwan-aman"
    >
      <h2 id="luba-unwan-aman" className="luba__unwan-qism">
        {t('luba.aman.unwan', lugha)}
      </h2>
      {himaya === null ? (
        <div className="luba__fahs">
          <p className="luba__nass-hadi">{t('luba.aman.lam_yufhas', lugha)}</p>
          <button type="button" className="zir" onClick={alaFahs} aria-busy={yajri}>
            {t(yajri ? 'luba.aman.jari' : 'luba.aman.ifhas', lugha)}
          </button>
        </div>
      ) : mahmiya ? (
        <>
          <p className="luba__rafd">{t('luba.aman.mahmiya', lugha)}</p>
          {himaya.anwa.length > 0 ? (
            <Riqaqat qaima={himaya.anwa} />
          ) : null}
          {himaya.adilla.length > 0 ? (
            <>
              <p className="luba__nass-hadi">{t('luba.aman.adilla', lugha)}</p>
              <ul className="luba__adilla">
                {himaya.adilla.map((daleel, fihris) => (
                  <li key={`${daleel}-${String(fihris)}`} className="mono-ltr luba__satr-masar">
                    {daleel}
                  </li>
                ))}
              </ul>
            </>
          ) : null}
        </>
      ) : (
        <>
          <p className="luba__nass-hadi">
            <span className="luba__nuqta luba__nuqta--najah" aria-hidden="true" />
            {t('luba.aman.salima', lugha)}
          </p>
          {himaya.mabtur || himaya.thughrat.length > 0 ? (
            <p className="luba__nass-hadi luba__tahdheer">{t('luba.aman.juzii', lugha)}</p>
          ) : null}
        </>
      )}
      {khata !== null ? (
        <KutlatKhata
          unwan={t('luba.khata.amal', lugha)}
          khata={khata}
          lugha={lugha}
          muarrif={muarrif}
          aada={alaFahs}
        />
      ) : null}
    </section>
  );
}

/* ---------------------------------------------------------------------------
   لماذا تقول هذه اللعبة ذلك — the evidence chain, behind a disclosure.

   Every answer `taarib-aql` gives comes back as a verdict together with the
   producers behind it: which module said what, and where it saw it. That is
   what makes "why does this game say that" answerable — a verdict that cannot
   name its inputs is a cache, and a wrong verdict that can is traceable to a
   wrong *input* rather than to a guess somebody has to go and find.

   It is here rather than nowhere because a chain nobody can reach is a chain
   that stops being maintained. It is behind a disclosure rather than open
   because `aql_luba` walks the game directory — the anti-cheat scan, the
   multiplayer scan and the proxy survey all read files — and this screen
   deliberately does not pay for that on every open, which is the same reason
   the protection panel above has its own button.

   Nothing in here is phrased by the interface. Every sentence is a producer's
   own, in both languages, and the observations are kept in whatever language
   their producer wrote them: translating an observation would put a line in the
   trail that nothing ever said.
   --------------------------------------------------------------------------- */

/** The three answers the anti-cheat question actually has, named. */
const ISM_HALAT_HIMAYA: Readonly<Record<HalatHimaya, MiftahLugha>> = {
  mahmiya: 'luba.aql.himaya_mahmiya',
  lam_yajri: 'luba.aql.himaya_lam_yajri',
  la_tawqee: 'luba.aql.himaya_la_tawqee',
};

interface KhasaisShawahid {
  readonly shawahid: readonly ShahidHie[];
  readonly lugha: Lugha;
}

/**
 * One answer's chain.
 *
 * An empty chain is drawn as an empty chain rather than omitted. `Musnad` says
 * why: "nobody looked" is a legitimate answer and the one state a reader must
 * never be shown as a finding, so a verdict standing on nothing says so.
 */
function Shawahid({ shawahid, lugha }: KhasaisShawahid): JSX.Element {
  if (shawahid.length === 0) {
    return <p className="luba__nass-hadi">{t('luba.aql.la_shawahid', lugha)}</p>;
  }
  return (
    <ul className="luba__shawahid">
      {shawahid.map((shahid, fihris) => (
        <li key={`${shahid.masdar}-${String(fihris)}`} className="luba__shahid">
          <p className="mono-ltr luba__shahid-muntij">{shahid.muntij}</p>
          <p className="luba__shahid-wasf" dir="auto">
            {shahid.wasf}
          </p>
          {shahid.mawqi === null ? null : (
            <p className="mono-ltr luba__daleel-masar luba__qat" title={shahid.mawqi}>
              {shahid.mawqi}
            </p>
          )}
        </li>
      ))}
    </ul>
  );
}

interface KhasaisAqlQism {
  readonly muarrif: string;
  readonly lugha: Lugha;
}

function QismAql({ muarrif, lugha }: KhasaisAqlQism): JSX.Element {
  const [zahir, setZahir] = useState(false);
  useEffect(() => {
    setZahir(false);
  }, [muarrif]);

  // The same key the automatic-run screen asks under, so arriving there from
  // here costs nothing and — the point — the two cannot hold two different
  // answers about one game.
  const aql = useQuery<AqlLubaHie, KhataJisr>({
    queryKey: mafatih.aql(muarrif),
    queryFn: () => nadi('aql_luba', { muarrif }),
    enabled: zahir,
  });
  const bayanat = aql.data;
  const halaHimaya = bayanat === undefined ? null : halatHimayaMin(bayanat.hala_himaya);

  return (
    <section className="luba__qism" aria-labelledby="luba-unwan-aql">
      <h2 id="luba-unwan-aql" className="luba__unwan-qism">
        {t('luba.aql.unwan', lugha)}
      </h2>
      <p className="luba__nass-hadi">{t('luba.aql.sharh', lugha)}</p>
      <div className="luba__saff-afal">
        <button
          type="button"
          className="zir"
          aria-expanded={zahir}
          aria-controls="luba-aql"
          onClick={() => {
            setZahir((hali) => !hali);
          }}
        >
          {t(zahir ? 'luba.aql.ikhfa' : 'luba.aql.zirr', lugha)}
        </button>
      </div>
      <Zuhur maftuh={zahir} id="luba-aql" className="luba__kashf">
        <Mashhad miftah={wajhIstifsar(aql)}>
          {aql.isPending ? (
            <HaykalSutur sutur={5} nass={t('luba.aql.jari', lugha)} />
          ) : aql.error !== null ? (
            <KutlatKhata
              unwan={t('luba.aql.taadhur', lugha)}
              khata={aql.error}
              lugha={lugha}
              muarrif={muarrif}
              aada={() => {
                void aql.refetch();
              }}
            />
          ) : bayanat === undefined ? null : (
            <div className="luba__aql">
              <h3 className="luba__unwan-farii">{t('luba.aql.muntaj', lugha)}</h3>
              <p className="luba__aql-jumla" dir="auto">
                {nassLugha({ arabi: bayanat.ism_arabi, injilizi: bayanat.ism_injilizi }, lugha)}
              </p>
              <p className="luba__sabab" dir="auto">
                {nassLugha({ arabi: bayanat.sabab_arabi, injilizi: bayanat.sabab_injilizi }, lugha)}
              </p>
              <Shawahid shawahid={bayanat.shawahid_muntaj} lugha={lugha} />

              <h3 className="luba__unwan-farii">{t('luba.aql.mawani', lugha)}</h3>
              {bayanat.mawani.length === 0 ? (
                <p className="luba__nass-hadi">{t('luba.aql.la_mawani', lugha)}</p>
              ) : (
                // An ordered list, because the order is the answer: this is the one
                // ranking in the product and every surface reads it rather than
                // making one.
                <ol className="luba__aql-qaima">
                  {bayanat.mawani.map((mani) => (
                    <li key={mani.naw} className="luba__aql-madkhal">
                      <p className="luba__aql-jumla" dir="auto">
                        {nassLugha(mani, lugha)}
                      </p>
                      <p className="luba__aql-wusum">
                        <span className="luba__riqaqa">
                          {t(
                            mani.nitaq === 'kul' ? 'luba.aql.nitaq_kul' : 'luba.aql.nitaq_tashghil',
                            lugha,
                          )}
                        </span>
                        <span className="luba__riqaqa">
                          {t(mani.nihai ? 'luba.aql.nihai' : 'luba.aql.ghayr_nihai', lugha)}
                        </span>
                      </p>
                      <Shawahid shawahid={mani.shawahid} lugha={lugha} />
                    </li>
                  ))}
                </ol>
              )}

              <h3 className="luba__unwan-farii">{t('luba.aql.makhatir', lugha)}</h3>
              {bayanat.makhatir.length === 0 ? (
                <p className="luba__nass-hadi">{t('luba.aql.la_makhatir', lugha)}</p>
              ) : (
                <ol className="luba__aql-qaima">
                  {bayanat.makhatir.map((khatar) => (
                    <li key={khatar.naw} className="luba__aql-madkhal">
                      <p className="luba__aql-jumla" dir="auto">
                        {nassLugha(khatar, lugha)}
                      </p>
                      <p className="luba__aql-wusum">
                        <span className="luba__riqaqa">
                          {t(khatar.muqarr ? 'luba.aql.muqarr' : 'luba.aql.muallaq', lugha)}
                        </span>
                      </p>
                      <Shawahid shawahid={khatar.shawahid} lugha={lugha} />
                    </li>
                  ))}
                </ol>
              )}

              <h3 className="luba__unwan-farii">{t('luba.aql.hudud', lugha)}</h3>
              {bayanat.hudud.length === 0 ? (
                <p className="luba__nass-hadi">{t('luba.aql.la_hudud', lugha)}</p>
              ) : (
                <ul className="luba__aql-qaima">
                  {bayanat.hudud.map((hadd, martaba) => (
                    <li key={`${String(martaba)}:${hadd.arabi}`} className="luba__aql-madkhal">
                      <p className="luba__aql-jumla" dir="auto">
                        {nassLugha(hadd, lugha)}
                      </p>
                      <Shawahid shawahid={hadd.shawahid} lugha={lugha} />
                    </li>
                  ))}
                </ul>
              )}

              <h3 className="luba__unwan-farii">{t('luba.aql.himaya', lugha)}</h3>
              {/* Named from the verdict rather than from the evidence list, because
                  an empty list is what a clean game and an unread catalogue both
                  produce and the difference is the whole point of the third answer.
                  A verdict this build cannot name falls through to its chain, which
                  still says what was looked at. */}
              {halaHimaya === null ? null : (
                <p className="luba__nass-hadi">{t(ISM_HALAT_HIMAYA[halaHimaya], lugha)}</p>
              )}
              <Shawahid shawahid={bayanat.shawahid_himaya} lugha={lugha} />
              {bayanat.ikhtilaf_himaya === null ? null : (
                <div className="luba__aql-ikhtilaf">
                  <p className="luba__nass-hadi luba__tahdheer">{t('luba.aql.ikhtilaf', lugha)}</p>
                  <dl className="luba__jadwal">
                    <Saff unwan={t('luba.aql.min_kashf', lugha)}>
                      <Riqaqat qaima={bayanat.ikhtilaf_himaya.min_kashf} />
                    </Saff>
                    <Saff unwan={t('luba.aql.min_iktishaf', lugha)}>
                      <Riqaqat qaima={bayanat.ikhtilaf_himaya.min_iktishaf} />
                    </Saff>
                  </dl>
                </div>
              )}
            </div>
          )}
        </Mashhad>
      </Zuhur>
    </section>
  );
}

interface KhasaisBinaQism {
  readonly bina: BinaHie | null;
  readonly muthabbat: boolean;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly nassWaqt: (khaam: string) => string;
}

function QismBina({ bina, muthabbat, lugha, munassiq, nassWaqt }: KhasaisBinaQism): JSX.Element {
  return (
    <section className="luba__qism" aria-labelledby="luba-unwan-bina">
      <h2 id="luba-unwan-bina" className="luba__unwan-qism">
        {t('luba.bina.unwan', lugha)}
      </h2>
      {bina === null ? (
        <p className="luba__nass-hadi">
          {t(muthabbat ? 'luba.bina.la_shay' : 'luba.bina.qabl_fahs', lugha)}
        </p>
      ) : (
        <dl className="luba__jadwal">
          {bina.manassa === null ? null : (
            <Saff unwan={t('luba.bina.manassa', lugha)}>{bina.manassa}</Saff>
          )}
          <Saff unwan={t('luba.bina.wasm', lugha)}>
            <span className="mono-ltr">{bina.wasm}</span>
          </Saff>
          <Saff unwan={t('luba.bina.basma', lugha)}>
            <span className="mono-ltr luba__qat" title={bina.basma_mukhtasara}>
              {bina.basma_mukhtasara}
            </span>
          </Saff>
          <Saff unwan={t('luba.bina.malaffat', lugha)}>{munassiq.raqm(bina.adad_malaffat)}</Saff>
          <Saff unwan={t('luba.bina.waqt', lugha)}>
            <span dir="auto">{nassWaqt(bina.waqt)}</span>
          </Saff>
        </dl>
      )}
    </section>
  );
}

const MIFTAH_LUGHA_RASMIYA: Readonly<Record<LughaRasmiyaHie['hala'], MiftahLugha>> = {
  ghaib: 'luba.lugha.ghaib',
  wajiha_faqat: 'luba.lugha.wajiha',
  nusus_faqat: 'luba.lugha.nusus',
  kamila: 'luba.lugha.kamila',
  mubhama: 'luba.lugha.mubhama',
};

/**
 * What the publisher already ships, and everything that says so.
 *
 * Shown whenever the verdict is that the game speaks Arabic, whether or not the
 * override is on: the reason a surface is missing has to be visible from the
 * place it is missing from, and a user who turns the override on is owed the
 * same evidence they would have had to argue with it.
 *
 * The evidence list is not decoration. A verdict nobody can argue with is a
 * verdict nobody can correct, and this is the screen where a user who believes
 * Taarib is wrong finds out exactly what it read.
 */
function QismLughaRasmiya({
  hukm,
  lugha,
  munassiq,
}: {
  readonly hukm: LughaRasmiyaHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}): JSX.Element {
  return (
    <div className="luba__lugha">
      <p className="luba__lugha-hukm">
        <span className="luba__lugha-wasm">{t(MIFTAH_LUGHA_RASMIYA[hukm.hala], lugha)}</span>
        <span className="luba__nass-hadi">
          {t('luba.lugha.thiqa', lugha, { nisba: munassiq.raqm(hukm.thiqa) })}
        </span>
      </p>
      {hukm.dalail.length === 0 ? null : (
        <ul className="luba__lugha-dalail">
          {hukm.dalail.map((daleel) => (
            <li key={`${daleel.naw}-${String(daleel.wazn)}-${daleel.wasf}`}>
              <span className="luba__lugha-naw">{daleel.naw}</span>
              <span className="luba__lugha-wasf" dir="auto">
                {daleel.wasf}
              </span>
              {daleel.mawqi === null ? null : (
                <span className="mono-ltr luba__lugha-mawqi" dir="ltr">
                  {daleel.mawqi}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
      {hukm.majhul.length === 0 ? null : (
        <>
          <p className="luba__unwan-farii">{t('luba.lugha.majhul', lugha)}</p>
          <ul className="luba__lugha-majhul">
            {hukm.majhul.map((sabab) => (
              <li key={sabab} dir="auto">
                {sabab}
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

interface KhasaisTilqaiMutah {
  readonly muarrif: string;
  readonly lugha: Lugha;
  /** The engine's runtime readiness, or null when the report did not say. */
  readonly jahiziya: JahiziyaTashghil | null;
  /** What is unfinished, when anything is — the reason the offer is withdrawn. */
  readonly naqs: string | null;
}

/**
 * The offer to translate a game nobody has published a patch for.
 *
 * ## Why the button is disabled rather than removed
 *
 * An engine whose adapter is unwritten gets the button, greyed, with the reason
 * beside it. Removing it would say the automatic route does not exist for this
 * game, and this screen already has a vocabulary for *that*: an anti-cheat
 * refusal, which is permanent and is drawn in red. An unfinished adapter is
 * neither — it is a wait, it is a fact about this build of Taarib rather than
 * about the game, and it ends with an update. A button that is present and off
 * says exactly that; an absent one says the wrong thing in the one direction
 * this whole stage exists to correct, because a person who never sees the offer
 * cannot know it is coming.
 *
 * The refusal is `aria-disabled` and not `disabled`, like every other refused
 * control in this product: a `disabled` button cannot be focused, so a keyboard
 * user tabbing through the panel would never reach the control whose
 * description carries the reason.
 */
function TilqaiMutah({ muarrif, lugha, jahiziya, naqs }: KhasaisTilqaiMutah): JSX.Element {
  const intiqal = useNavigate();
  const mutah = tasil(jahiziya);
  const muarrifSabab = 'luba-sabab-tilqai';

  return (
    <div className="luba__tilqai">
      <p className="luba__nass-hadi">{t('tilqai.luba.sharh', lugha)}</p>
      <div className="luba__saff-afal">
        <button
          type="button"
          className="zir zir--tamyeez"
          aria-disabled={!mutah}
          aria-describedby={mutah ? undefined : muarrifSabab}
          onClick={() => {
            if (mutah) {
              void intiqal({ to: '/tilqai/$muarrif', params: { muarrif } });
            }
          }}
        >
          {t('tilqai.luba.zirr', lugha)}
        </button>
      </div>
      {mutah ? null : (
        <p id={muarrifSabab} className="luba__nass-hadi luba__tahdheer">
          {t('luba.tilqai.mamnu', lugha)}
          {/* The specific gap, when the report named one. The sentence above is
              the rule; this is what is missing for this particular engine, and a
              reader who is told only the rule cannot tell whether it will ever
              stop applying to them. */}
          {naqs === null ? null : ` ${naqs}`}
        </p>
      )}
    </div>
  );
}

/* ---------------------------------------------------------------------------
   ما الذي سيُكتب في لعبتك — the install plan, beside the button that runs it.

   `khuttat_tathbeet` is the same `tarkib::khutta` the install itself is handed,
   over the same description of the game, so this is not a description of the
   install: it is the install's own plan, read early. Nothing in here is phrased
   twice. The reason no framework is needed, each launch requirement and each
   loader already sitting in the game are the installer's own sentences in both
   languages, and the paths are the plan's own.

   That now includes the last two that were not. The store-verify note and the
   launcher note used to exist in `KhuttatTarkib` in English only, so this screen
   — whose first language is Arabic — held its own Arabic for both and keyed each
   off a bare `bool`. Nothing tied the two wordings together, and this panel is
   the one place a user reads what is about to happen to a game they own.
   `malhuzat_tahaqquq_arabi` and `MalhuzatManassa::wasf_arabi` mean the plan says
   both sentences itself, in both languages; the screen chooses a language and
   renders. Nothing in this panel is phrased twice any more.

   The shape is deliberate. Findings are never behind a disclosure: a mod already
   in the game, a store verify that would undo half of this, a launcher that will
   overwrite what is being edited. The inventory — which file goes where — is,
   because it is long, it is read once, and it is not the part a decision turns
   on. `IqrarKhatar` was considered and rejected: that component is for the two
   risks that carry a permanent consequence and need a tick, and a plan is not a
   risk. It is a statement of what will happen, and it is owed whether or not
   anybody agrees to anything.
   --------------------------------------------------------------------------- */

interface KhasaisKhutta {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /**
   * Whether building the plan is worth it on this screen at all.
   *
   * The plan reads the game directory — one listing of the executable's own
   * directory, and the modules in it that occupy a loader slot — so it is not
   * asked for on a refused game or on a game with nothing to install.
   */
  readonly mumakkan: boolean;
}

/** One labelled list of paths, drawn only when it has entries. */
function QaimatMasarat({
  unwan,
  masarat,
}: {
  readonly unwan: string;
  readonly masarat: readonly string[];
}): JSX.Element | null {
  if (masarat.length === 0) {
    return null;
  }
  return (
    <>
      <h4 className="luba__unwan-farii">{unwan}</h4>
      <ul className="luba__sutur">
        {masarat.map((masar) => (
          <li key={masar} className="mono-ltr luba__satr-masar" title={masar}>
            {masar}
          </li>
        ))}
      </ul>
    </>
  );
}

function QismKhutta({ muarrif, lugha, munassiq, mumakkan }: KhasaisKhutta): JSX.Element {
  const khutta = useQuery<KhuttatTathbeetHie, KhataJisr>({
    queryKey: mafatih.khutta(muarrif),
    queryFn: () => nadi('khuttat_tathbeet', { muarrif }),
    enabled: mumakkan,
  });

  return (
    <Zuhur maftuh={mumakkan} className="luba__khutta-zuhur">
      <Mashhad miftah={wajhIstifsar(khutta)}>
        {khutta.isPending ? (
          <HaykalSutur sutur={4} nass={t('luba.khutta.jari', lugha)} />
        ) : khutta.error !== null ? (
          // A plan that cannot be built is an install that would not have run.
          // The component that is missing, the compatibility prefix that was
          // never built, the report the safety layer refused — each of those is
          // the install's own refusal, said before a backup is taken instead of
          // half way through one.
          <KutlatKhata
            unwan={t('luba.khutta.taadhur', lugha)}
            khata={khutta.error}
            lugha={lugha}
            muarrif={muarrif}
            aada={() => {
              void khutta.refetch();
            }}
          />
        ) : khutta.data === undefined ? null : (
          <KhuttaJahiza bayanat={khutta.data} lugha={lugha} munassiq={munassiq} />
        )}
      </Mashhad>
    </Zuhur>
  );
}

/** The plan once the installer has written it: the counts, the findings, the inventory. */
function KhuttaJahiza({
  bayanat,
  lugha,
  munassiq,
}: {
  readonly bayanat: KhuttatTathbeetHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}): JSX.Element {
  const sababFaragh =
    lugha === 'arabi' ? bayanat.sabab_faragh_arabi : bayanat.sabab_faragh_injilizi;
  const talabat = lugha === 'arabi' ? bayanat.talabat_arabi : bayanat.talabat_injilizi;
  const mudafa = bayanat.mudkhalat.filter((m) => !m.tadeel).map((m) => m.nisbi);
  const muaddala = bayanat.mudkhalat.filter((m) => m.tadeel).map((m) => m.nisbi);
  const manassa = bayanat.manassa;
  const malhuzatManassa =
    manassa === null ? null : lugha === 'arabi' ? manassa.wasf_arabi : manassa.wasf_injilizi;
  const malhuzatTahaqquq =
    lugha === 'arabi' ? bayanat.malhuzat_tahaqquq_arabi : bayanat.malhuzat_tahaqquq_injilizi;

  return (
    <section className="luba__khutta" aria-labelledby="luba-unwan-khutta">
      <h3 id="luba-unwan-khutta" className="luba__unwan-farii">
        {t('luba.khutta.unwan', lugha)}
      </h3>
      {/* Two sentences and not one count, because the plan counts two different
          things. `mudkhalat` is the additive layer only — the framework's own
          files are copied out of the component store as it is deployed and are
          never enumerated in the plan — so a Unity game, whose whole install is
          the framework, has an additive layer of exactly nothing. Summarising it
          as "0 files added, 0 modified" would be the screen telling somebody
          that pressing install writes nothing into their game. */}
      {bayanat.faragha ? (
        <p className="luba__khutta-hasila">{t('luba.khutta.la_shay', lugha)}</p>
      ) : (
        <>
          {bayanat.itar === null ? null : (
            <p className="luba__khutta-hasila">{t('luba.khutta.hasila_itar', lugha)}</p>
          )}
          {bayanat.adad_idafat === 0 && bayanat.adad_tadeelat === 0 ? null : (
            <p className="luba__khutta-hasila">
              {t('luba.khutta.hasila', lugha, {
                idafat: munassiq.raqm(bayanat.adad_idafat),
                tadeelat: munassiq.raqm(bayanat.adad_tadeelat),
              })}
            </p>
          )}
        </>
      )}
      {sababFaragh === null ? null : (
        <p className="luba__nass-hadi" dir="auto">
          {sababFaragh}
        </p>
      )}

      {/* The findings, and never behind a disclosure. Somebody agreeing to
          "install Arabic into this game" is agreeing to a different thing than
          they think if nobody tells them what else is already loaded in it. */}
      {bayanat.huqn_qaim.length === 0 ? null : (
        <div className="luba__khutta-tanbeeh">
          <p className="luba__khutta-tanbeeh-unwan">{t('luba.khutta.huqn', lugha)}</p>
          <ul className="luba__khutta-huqn">
            {bayanat.huqn_qaim.map((wakeel) => (
              <li key={wakeel.ism} dir="auto">
                {lugha === 'arabi' ? wakeel.arabi : wakeel.injilizi}
              </li>
            ))}
          </ul>
          <p className="luba__nass-hadi">{t('luba.khutta.huqn_sharh', lugha)}</p>
        </div>
      )}
      {malhuzatTahaqquq === null ? null : (
        <p className="luba__nass-hadi luba__tahdheer" dir="auto">
          {malhuzatTahaqquq}
        </p>
      )}
      {malhuzatManassa === null ? null : (
        <p className="luba__nass-hadi luba__tahdheer" dir="auto">
          {malhuzatManassa}
        </p>
      )}
      {/* Outside the disclosure, because a launch requirement is a change to
          something the user owns and did not come here to change: Taarib writes
          the launch options at install and puts them back at removal, and that
          is not a detail about file layout. */}
      {talabat.length === 0 ? null : (
        <>
          <h4 className="luba__unwan-farii">{t('luba.khutta.talabat', lugha)}</h4>
          <ul className="luba__khutta-talabat">
            {talabat.map((talab) => (
              <li key={talab} dir="auto">
                {talab}
              </li>
            ))}
          </ul>
        </>
      )}

      {/* The inventory. Long, read once, and not the part the decision turns on
          — so it takes the same disclosure shape the per-game limits above it
          already use. */}
      {bayanat.faragha ? null : (
        <details className="luba__kulfa">
          <summary className="luba__kulfa-unwan">{t('luba.khutta.tafsil', lugha)}</summary>
          {bayanat.itar === null ? null : (
            <dl className="luba__jadwal">
              <Saff unwan={t('luba.khutta.itar', lugha)}>
                <span dir="auto">{bayanat.itar}</span>
              </Saff>
              {bayanat.jidhr_muhammil === null ? null : (
                <Saff unwan={t('luba.khutta.mawqi_muhammil', lugha)}>
                  <span className="mono-ltr">{bayanat.jidhr_muhammil}</span>
                </Saff>
              )}
            </dl>
          )}
          <QaimatMasarat
            unwan={t('luba.khutta.mujalladat', lugha)}
            masarat={bayanat.mujalladat}
          />
          <QaimatMasarat unwan={t('luba.khutta.mudafa', lugha)} masarat={mudafa} />
          <QaimatMasarat unwan={t('luba.khutta.muaddala', lugha)} masarat={muaddala} />
          {bayanat.khatt_renpy === null ? null : (
            <QaimatMasarat
              unwan={t('luba.khutta.khatt', lugha)}
              masarat={[bayanat.khatt_renpy]}
            />
          )}
          {/* The plan's own report text, unedited: the thing to paste into a bug
              report, and the same lines the install log carries. */}
          <details className="luba__khutta-nass">
            <summary className="luba__kulfa-unwan">{t('luba.khutta.sutur', lugha)}</summary>
            <ol className="luba__sutur">
              {bayanat.sutur.map((satr, martaba) => (
                <li key={`${String(martaba)}:${satr}`} className="mono-ltr luba__satr-khutta">
                  {satr}
                </li>
              ))}
            </ol>
          </details>
        </details>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------------------
   ما الذي ستفعله الإزالة — the removal's dry run, inside its confirmation.

   `taraju::khutta` has always computed this and nobody has ever been shown it.
   It belongs here and nowhere else: the removal is the one control on this
   screen that can destroy something the user did not put there, and the
   residue is now named rather than counted, so a decision about it is possible.
   --------------------------------------------------------------------------- */

interface KhasaisKhuttatIzala {
  readonly muarrif: string;
  readonly matlab: MatlabIzala;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

function KhuttatIzala({
  muarrif,
  matlab,
  lugha,
  munassiq,
}: KhasaisKhuttatIzala): JSX.Element {
  const khutta = useQuery<KhuttatIzalaHie[], KhataJisr>({
    queryKey: mafatih.khuttat_izala(muarrif, matlab),
    queryFn: () => nadi('khuttat_izala', { muarrif, matlab }),
  });
  const khutat = khutta.data;

  return (
    <Mashhad miftah={wajhIstifsar(khutta)} className="luba__khuttat-izala-mashhad">
      {khutta.isPending ? (
        <HaykalSutur sutur={3} nass={t('luba.izala.khutta_jari', lugha)} />
      ) : khutta.error !== null ? (
        <KutlatKhata
          unwan={t('luba.izala.khutta_taadhur', lugha)}
          khata={khutta.error}
          lugha={lugha}
          muarrif={muarrif}
          aada={() => {
            void khutta.refetch();
          }}
        />
      ) : khutat === undefined ? null : khutat.length === 0 ? (
        <HalatFarigha unwan={t('luba.izala.khutta_la_shay', lugha)} />
      ) : (
        <div className="luba__khuttat-izala">
          <h3 className="luba__unwan-farii">{t('luba.izala.khutta_unwan', lugha)}</h3>
          {khutat.map((khutwa) => (
            <div key={khutwa.naw} className="luba__natija-band">
              <p className="luba__natija-nass">
                <span
                  className={
                    khutwa.nazif ? 'luba__nuqta luba__nuqta--najah' : 'luba__nuqta luba__nuqta--khatar'
                  }
                  aria-hidden="true"
                />
                {t(miftahNaw(khutwa.naw === 'nass' ? 'nass' : 'sawt'), lugha)}
                {' — '}
                {t(khutwa.nazif ? 'luba.izala.nazif' : 'luba.izala.ghayr_nazif', lugha)}
              </p>
              <ul className="luba__adad-natija">
                <li>{t('luba.izala.li_istiada', lugha, { adad: munassiq.raqm(khutwa.li_istiada) })}</li>
                <li>{t('luba.izala.li_hadhf', lugha, { adad: munassiq.raqm(khutwa.li_hadhf) })}</li>
                <li>{t('luba.izala.mujalladat', lugha, { adad: munassiq.raqm(khutwa.mujalladat) })}</li>
                <li>{t('luba.izala.hajm', lugha, { hajm: khutwa.hajm_nusakh_maqru })}</li>
              </ul>
              <QaimatMasarat
                unwan={t('luba.izala.mustabdala', lugha)}
                masarat={khutwa.mustabdala}
              />
              <QaimatMasarat unwan={t('luba.izala.mafquda', lugha)} masarat={khutwa.mafquda} />
              {khutwa.baqaya.length === 0 ? null : (
                <>
                  <h4 className="luba__unwan-farii">{t('luba.izala.baqaya', lugha)}</h4>
                  <ul className="luba__baqaya">
                    {khutwa.baqaya.map((baqiya) => (
                      <li key={baqiya.mujallad}>
                        <p className="luba__nass-hadi" dir="auto">
                          {baqiya.adad === 0
                            ? t('luba.izala.baqaya_dakhil', lugha, { mujallad: baqiya.mujallad })
                            : t('luba.izala.baqaya_mujallad', lugha, {
                                mujallad: baqiya.mujallad,
                                adad: munassiq.raqm(baqiya.adad),
                              })}
                        </p>
                        <ul className="luba__sutur">
                          {baqiya.madakhil.map((madkhal) => (
                            <li key={madkhal} className="mono-ltr luba__satr-masar" title={madkhal}>
                              {madkhal}
                            </li>
                          ))}
                          {baqiya.adad > baqiya.madakhil.length ? (
                            <li className="luba__nass-hadi">
                              {t('luba.izala.baqaya_mazid', lugha, {
                                adad: munassiq.raqm(baqiya.adad - baqiya.madakhil.length),
                              })}
                            </li>
                          ) : null}
                        </ul>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </div>
          ))}
        </div>
      )}
    </Mashhad>
  );
}

/**
 * What one install was authorised with, and by whom.
 *
 * Two acknowledgements travel here, not one. `MudkhalRuqaaHie::yahtaj_iqrar` is
 * the registry's verdict on the **build match** and nothing else — it is true
 * exactly when `MutabaqaBina` is `Nitaq` — and this screen used to spend that
 * single value on both `iqrarShabaka` and `iqrarTaqribi`. So the answer to "do
 * you accept that an anti-cheat may notice this on a game you play with other
 * people" was being read off a fact about build fingerprints, and it was read as
 * *yes* precisely on the patches that were furthest from matching. The two are
 * separate fields because they are separate decisions, and both of them now
 * carry what the user actually said.
 *
 * `mifta` identifies the row rather than the lineage: a failed install has to be
 * able to reopen the question on the row it was pressed on, and the registry may
 * list two revisions of one lineage.
 */
interface TalabTathbeet {
  /** The patch lineage, as `thabbit_ruqaa` wants it. */
  readonly ruqaa: string;
  /** The row that asked, as this screen identifies rows. */
  readonly mifta: string;
  /** The user's answer to the multiplayer risk. */
  readonly iqrarShabaka: boolean;
  /** The user's answer to the approximate build match. */
  readonly iqrarTaqribi: boolean;
}

/** One patch row's identity, stable across a refetch that reorders the list. */
function miftahMudkhal(mudkhal: MudkhalRuqaaHie): string {
  return `${mudkhal.id}-${String(mudkhal.murajaa)}`;
}

interface KhasaisRuqaa {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** The capability report, for the readiness notice above the install button. */
  readonly taqreer: TaqreerHie;
  readonly mahmiya: boolean;
  /** Actions are locked while the game runs or the first-run statement is unacknowledged. */
  readonly muqfal: boolean;
  /** Why they are locked, in the reader's language, or null when they are not. */
  readonly sababQafl: string | null;
  /**
   * Whether the publisher already ships Arabic, once that has been decided.
   *
   * `null` while the probe is still walking the game's containers. The section
   * shows nothing but the check itself until then, and that order is the whole
   * point: a patch offered and withdrawn a second later is worse than a patch
   * not offered yet.
   */
  readonly hukmLugha: LughaRasmiyaHie | null;
  /** Whether the probe is still running. */
  readonly yajriFahsLugha: boolean;
  /** Why it could not run, when it could not. */
  readonly khataLugha: KhataJisr | null;
  /** Runs the probe again after it could not. */
  readonly alaAadaLugha: () => void;
  /** The user's own override, from settings. */
  readonly istibdal: boolean;
}

function QismRuqaa({
  muarrif,
  lugha,
  munassiq,
  taqreer,
  mahmiya,
  muqfal,
  sababQafl,
  hukmLugha,
  yajriFahsLugha,
  khataLugha,
  alaAadaLugha,
  istibdal,
}: KhasaisRuqaa): JSX.Element {
  const makhzan = useQueryClient();
  const ruqaa = useQuery<RuqaaLuba, KhataJisr>({
    queryKey: mafatih.ruqaa(muarrif),
    queryFn: () => nadi('ruqaa_luba', { muarrif }),
  });

  const talabat = useQuery<number, KhataJisr>({
    queryKey: mafatih.talabat_luba(muarrif),
    queryFn: () => nadi('adad_talabat', { muarrif }),
  });
  const talab = useMutation<number, KhataJisr, void>({
    mutationFn: () => nadi('utlub_tarjama', { muarrif, mulahaza: null }),
    onSuccess: (adad) => {
      makhzan.setQueryData(mafatih.talabat_luba(muarrif), adad);
      ansha({
        naw: 'najah',
        nass: t('luba.talabat.sujjil', lugha, { adad: munassiq.raqm(adad) }),
      });
    },
  });

  const [marhala, setMarhala] = useState<string | null>(null);
  // The download is the one countable stretch of an install: the listing
  // declares the bytes, so the bar draws against them and stops at the last
  // report. The install stages after it are named, not counted.
  const [tanzil, setTanzil] = useState<TaqaddumTanzeel | null>(null);
  useEffect(() => {
    const ilgha = listen<string>(HADATH_MARHALAT_TATHBEET, (hadath) => {
      setMarhala(hadath.payload);
      setTanzil(null);
    });
    const ilghaTanzeel = listen<TaqaddumTanzeel>(HADATH_TAQADDUM_TANZEEL, (hadath) => {
      setMarhala(hadath.payload.marhala_arabi);
      setTanzil(hadath.payload.tamma ? null : hadath.payload);
    });
    return () => {
      void ilgha.then((f) => { f(); });
      void ilghaTanzeel.then((f) => { f(); });
    };
  }, []);

  const tathbeet = useMutation<NatijatTathbeetHie, KhataJisr, TalabTathbeet>({
    mutationFn: async ({ ruqaa: idRuqaa, iqrarShabaka, iqrarTaqribi }) => {
      const hasila = await nadi('nazzil_ruqaa', { muarrif, ruqaa: idRuqaa });
      // The one countable stretch is over and the named stages begin; a reader
      // who scrolled away from the meter is told so where they are.
      ansha({
        naw: 'najah',
        nass: t('luba.tanbih.tanzil_tamma', lugha),
        tafsil: munassiq.hajm(hasila.hajm),
      });
      return nadi('thabbit_ruqaa', {
        muarrif,
        masarMalaf: hasila.masar,
        iqrarShabaka,
        iqrarTaqribi,
      });
    },
    onSettled: () => {
      setMarhala(null);
      setTanzil(null);
    },
    onSuccess: (natija) => {
      // The result block stands at the foot of a long section. A write that
      // verified is confirmed; one that did not is not dressed as a success,
      // and the notice sends the reader to the block that says what happened.
      ansha(
        natija.tahaqquq_salim
          ? { naw: 'najah', nass: t('luba.tanbih.tathbeet_tamma', lugha) }
          : { naw: 'tanbeeh', nass: t('luba.tanbih.tathbeet_ghayr_salim', lugha) },
      );
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
      // The plan described the game before this install. Its loader slot is now
      // taken, its files are now there, and a plan still saying otherwise would
      // be describing a game that no longer exists.
      void makhzan.invalidateQueries({ queryKey: mafatih.khutta(muarrif) });
      void makhzan.invalidateQueries({ queryKey: ['khuttat_izala', muarrif] });
    },
  });

  /* -------------------------------------------------------------------------
     The question, and the two answers to it.

     `sual` is the row whose acknowledgements are open, so at most one is asked
     at a time and the panel is physically inside the patch it is about — a
     dialog over a list would take the patch's own title, coverage and match
     verdict off screen at the exact moment they are what the decision is being
     made on.

     Both answers start false on every opening. An answer given about one patch
     is not an answer about the next one, and a tick remembered across rows is a
     tick the user did not give here.
     ----------------------------------------------------------------------- */
  const [sual, setSual] = useState<string | null>(null);
  const [iqrarTaqribi, setIqrarTaqribi] = useState(false);
  const [iqrarShabaka, setIqrarShabaka] = useState(false);

  /** Each row's install button, so cancelling returns focus where it started. */
  const azrarTathbeet = useRef(new Map<string, HTMLButtonElement | null>());
  const zirIlghaIqrarRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    setSual(null);
    setIqrarTaqribi(false);
    setIqrarShabaka(false);
  }, [muarrif]);

  // The safe control, exactly as the removal strip below does it: opening a
  // panel that can write into a game directory and landing focus on the button
  // that writes is an accident waiting for one keystroke.
  useEffect(() => {
    if (sual !== null) {
      zirIlghaIqrarRef.current?.focus();
    }
  }, [sual]);

  const alaIlghaSual = (): void => {
    if (sual !== null) {
      azrarTathbeet.current.get(sual)?.focus();
    }
    setSual(null);
  };

  /**
   * The install button on a row.
   *
   * A patch the registry judged an exact or a fingerprint match asks nothing and
   * installs with **both** acknowledgements withheld. That is not the old
   * behaviour renamed: withholding a grant nobody gave is the correct value, and
   * the backend refuses if the multiplayer one turns out to matter — which is a
   * refusal the user can then answer, through {@link alaMurajaatIqrar}, rather
   * than an authorisation this screen invented on their behalf.
   */
  const alaTalabTathbeet = (mudkhal: MudkhalRuqaaHie): void => {
    if (tathbeet.isPending || muqfal) {
      return;
    }
    const mifta = miftahMudkhal(mudkhal);
    if (!mudkhal.yahtaj_iqrar && sual !== mifta) {
      tathbeet.mutate({
        ruqaa: String(mudkhal.id),
        mifta,
        iqrarShabaka: false,
        iqrarTaqribi: false,
      });
      return;
    }
    // A second press on a row whose question is already open closes it, which is
    // what `aria-expanded` on that button promises. Reopening it instead would
    // silently clear two answers the reader had already given.
    if (sual === mifta) {
      alaIlghaSual();
      return;
    }
    setIqrarTaqribi(false);
    setIqrarShabaka(false);
    setSual(mifta);
  };

  const alaTanfidhSual = (mudkhal: MudkhalRuqaaHie): void => {
    if (tathbeet.isPending || muqfal || (mudkhal.yahtaj_iqrar && !iqrarTaqribi)) {
      return;
    }
    tathbeet.mutate({
      ruqaa: String(mudkhal.id),
      mifta: miftahMudkhal(mudkhal),
      iqrarShabaka,
      iqrarTaqribi,
    });
    setSual(null);
  };

  /**
   * Reopens the question after a refused install.
   *
   * The install pipeline flattens every safety refusal into one code, so this
   * screen cannot tell "the game is multiplayer and you did not say yes" from
   * any other refusal — and re-firing the same call with the same two answers,
   * which is what the block's own retry would do, cannot change the outcome of a
   * refusal that is a *question*. So the answer to a refused install is the
   * question again, with the backend's own sentence sitting above it, and the
   * previous ticks kept so the reader changes only what they mean to change.
   */
  const alaMurajaatIqrar = (): void => {
    const akhira = tathbeet.variables;
    if (akhira !== undefined && !tathbeet.isPending && !muqfal) {
      setSual(akhira.mifta);
    }
  };

  const naqs = naqsJahiziya(taqreer, lugha);
  const hukmJahiziya = jahiziyaMin(taqreer.jahiziya);
  // What installing costs, at the point where it is spent. Two lists, and the
  // difference between them is what makes both worth showing: `hududTathbeet`
  // is what will not work in *this* game, in the report's own words, and
  // `kulfatTabaqa` is the price of the tier itself — true of every game on it,
  // which is why the per-game report has no reason to repeat it. The second is
  // `null` until `TaqreerHie` carries the tier discriminant; see
  // `maktaba/tabaqat`.
  const hududTathbeet = hududQira(taqreer, lugha);
  const tabaqatTathbeet = tabaqaTaqreer(taqreer);
  const kulfatTabaqa =
    tabaqatTathbeet === null ? null : t(KULFAT_TABAQA[tabaqatTathbeet], lugha);

  return (
    <section className="luba__qism" aria-labelledby="luba-unwan-ruqaa">
      <h2 id="luba-unwan-ruqaa" className="luba__unwan-qism">
        {t('luba.ruqaa.unwan', lugha)}
      </h2>
      {/*
        The check that decides whether any of this is offered at all, and the
        verdict it produced. It comes before the listing because that is the
        order the decision is made in: a section that drew a patch and then
        withdrew it would have offered one.
      */}
      {yajriFahsLugha && hukmLugha === null ? (
        <p className="luba__jari zuhur-muakhkhar">{t('luba.lugha.jari', lugha)}</p>
      ) : null}
      {khataLugha === null ? null : (
        <KutlatKhata
          unwan={t('luba.lugha.taadhur', lugha)}
          khata={khataLugha}
          lugha={lugha}
          muarrif={muarrif}
          aada={alaAadaLugha}
        />
      )}
      {hukmLugha !== null && hukmLugha.yatakallam_arabi ? (
        <>
          <QismLughaRasmiya hukm={hukmLugha} lugha={lugha} munassiq={munassiq} />
          {istibdal ? (
            <p className="luba__athar">{t('luba.lugha.athar', lugha)}</p>
          ) : (
            <p className="luba__nass-hadi">{t('luba.lugha.mahjub', lugha)}</p>
          )}
        </>
      ) : null}
      {hukmLugha !== null && !hukmLugha.yatakallam_arabi && !hukmLugha.hasim ? (
        <p className="luba__nass-hadi">{t('luba.lugha.ghayr_hasim', lugha)}</p>
      ) : null}

      {hukmLugha === null || (hukmLugha.yatakallam_arabi && !istibdal) ? null : (
        <>
          {/* Above the install button, because this is where the decision is made
              and a warning read after the fact is a warning that was not given. */}
          {mahmiya || hukmJahiziya === null || hukmJahiziya === 'mukammala' ? null : (
            <TanbeehJahiziya
              unwan={t('luba.jahiziya.qabl_tathbeet', lugha)}
              // The engine is named in the compatibility panel directly above
              // this one, and naming it twice in one column reads as two
              // findings rather than one.
              muharrik={null}
              nass={naqs}
              athar={t(ATHAR_JAHIZIYA[hukmJahiziya], lugha)}
              lugha={lugha}
            />
          )}
          {/*
            What this costs, beside the button that spends it.

            The compatibility panel already carries the full list, and this is
            deliberately not a second copy of it on the page: it is the same
            list, one press away from the control it applies to. A limit that
            lives only in the panel above is a limit read after the decision or
            not at all, and every sentence in here is the backend's own — the
            report writes them per game, in both languages, and the interface
            has no business paraphrasing what will not work.

            Closed by default and never for a refused game: `mahmiya` means the
            install controls are not drawn at all, and a disclosure about what a
            patch will not translate is noise beside a refusal to install one.
          */}
          {mahmiya || hududTathbeet.length === 0 ? null : (
            <details className="luba__kulfa">
              <summary className="luba__kulfa-unwan">{t('luba.hudud.tafsil', lugha)}</summary>
              {kulfatTabaqa === null ? null : (
                <p className="luba__muntaj-kulfa luba__kulfa-tabaqa">{kulfatTabaqa}</p>
              )}
              <ul className="luba__hudud">
                {hududTathbeet.map((hadd, martaba) => (
                  <li key={`${String(martaba)}:${hadd}`} dir="auto">
                    {hadd}
                  </li>
                ))}
              </ul>
            </details>
          )}
          {/*
            What pressing install actually does, above the button that does it.

            Asked for as soon as there is something to install and this game is
            not refused, rather than on a press: the answer is the reason to
            press or not to press, and a plan nobody opened is a plan nobody was
            shown. It is the cheapest of the three directory reads this screen
            can make — one listing of the executable's own directory — which is
            why it is not behind a button the way the evidence chain is.
          */}
          <QismKhutta
            muarrif={muarrif}
            lugha={lugha}
            munassiq={munassiq}
            mumakkan={!mahmiya && (ruqaa.data?.mudkhalat.length ?? 0) > 0}
          />
          {sababQafl === null ? null : (
            <p className="luba__nass-hadi luba__tahdheer">{sababQafl}</p>
          )}
          <Mashhad miftah={wajhIstifsar(ruqaa)}>
            {ruqaa.isPending ? (
              <HaykalSutur sutur={5} nass={t('luba.ruqaa.jari', lugha)} />
            ) : ruqaa.error !== null ? (
              <KutlatKhata
                unwan={t('luba.ruqaa.taadhur', lugha)}
                khata={ruqaa.error}
                lugha={lugha}
                muarrif={muarrif}
                aada={() => {
                  void ruqaa.refetch();
                }}
              />
            ) : ruqaa.data === undefined || ruqaa.data.mudkhalat.length === 0 ? (
              <div className="luba__talabat">
                <HalatFarigha
                  unwan={t('luba.ruqaa.la_shay_unwan', lugha)}
                  nass={t('luba.ruqaa.la_shay', lugha)}
                />
                <TilqaiMutah
                  muarrif={muarrif}
                  lugha={lugha}
                  jahiziya={hukmJahiziya}
                  naqs={naqs}
                />
                {talabat.data !== undefined && talabat.data > 0 ? (
                  <p className="luba__nass-hadi">
                    {jam('luba.talabat.adad', lugha, talabat.data, munassiq)}
                  </p>
                ) : null}
                {talabat.error === null ? null : (
                  <KutlatKhata
                    unwan={t('luba.talabat.taadhur', lugha)}
                    khata={talabat.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      void talabat.refetch();
                    }}
                  />
                )}
                <button
                  type="button"
                  className="zir"
                  aria-busy={talab.isPending}
                  onClick={() => {
                    if (!talab.isPending) {
                      talab.mutate();
                    }
                  }}
                >
                  {t('luba.talabat.utlub', lugha)}
                </button>
                {talab.data !== undefined ? (
                  <p className="luba__nass-hadi" role="status">
                    {t('luba.talabat.sujjil', lugha, { adad: munassiq.raqm(talab.data) })}
                  </p>
                ) : null}
                {talab.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={talab.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      if (!talab.isPending) {
                        talab.mutate();
                      }
                    }}
                  />
                ) : null}
              </div>
            ) : (
              <ul className="luba__ruqaa">
                {ruqaa.data.mudkhalat.map((mudkhal) => {
                  const mifta = miftahMudkhal(mudkhal);
                  // Open for this row either because the patch matches only
                  // approximately, or because an install of it was refused and the
                  // reader asked to see the questions again.
                  const maftuh = sual === mifta;
                  const yasal = mudkhal.yahtaj_iqrar || maftuh;
                  // Busy on the row that was pressed alone. The other rows are
                  // refused for the duration, not busy: nothing is happening to
                  // them, and an arc turning on every row says otherwise.
                  const yuthabbat = tathbeet.isPending && tathbeet.variables?.mifta === mifta;
                  return (
                    <li key={mifta} className="luba__lawhat-tathbeet">
                      <div className="luba__raas-lawha">
                        <span className="luba__ruqaa-unwan">{mudkhal.unwan}</span>
                        <span className="luba__ruqaa-musahim">{mudkhal.musahim}</span>
                      </div>
                      <dl className="luba__ruqaa-tafsil">
                        <Saff unwan={t('luba.ruqaa.taghtiya', lugha)}>
                          {munassiq.nisba(kasr(mudkhal.nisbat_taghtiya))}
                        </Saff>
                        <Saff unwan={t('luba.ruqaa.nusus', lugha)}>
                          {munassiq.raqm(mudkhal.adad_nusus)}
                        </Saff>
                        <Saff unwan={t('luba.ruqaa.hajm', lugha)}>
                          <span dir="auto">{mudkhal.hajm_maqru}</span>
                        </Saff>
                        <Saff unwan={t('luba.ruqaa.tareeqa', lugha)}>{mudkhal.tareeqa_arabi}</Saff>
                        {/* What pressing this row's own install button produces,
                            in the reader's language; see `tabaqatMudkhal`. */}
                        <Saff unwan={t('luba.muharrik.tabaqa', lugha)}>
                          <span dir="auto">{tabaqatMudkhal(mudkhal, lugha, munassiq)}</span>
                        </Saff>
                      </dl>
                      {mahmiya ? null : (
                        <>
                          <div className="luba__saff-afal">
                            <button
                              type="button"
                              className="zir zir--tamyeez"
                              ref={(uqda) => {
                                azrarTathbeet.current.set(mifta, uqda);
                              }}
                              aria-disabled={muqfal || (tathbeet.isPending && !yuthabbat)}
                              aria-busy={yuthabbat}
                              title={sababQafl ?? undefined}
                              aria-expanded={yasal ? maftuh : undefined}
                              onClick={() => {
                                alaTalabTathbeet(mudkhal);
                              }}
                            >
                              {t('luba.ruqaa.tathbeet', lugha)}
                            </button>
                            {wasfMutabaqa(mudkhal, lugha) === null ? null : (
                              <span className="luba__nass-hadi">{wasfMutabaqa(mudkhal, lugha)}</span>
                            )}
                          </div>
                          <AnimatePresence initial={false}>
                            {maftuh ? (
                              <motion.div
                                key="iqrar"
                                className="luba__tawassu"
                                initial={{ height: 0, opacity: 0 }}
                                animate={{ height: 'auto', opacity: 1 }}
                                exit={{ height: 0, opacity: 0 }}
                                transition={haraka(HARAKAT_LAWHA)}
                              >
                                <div
                                  className="luba__tawassu-dakhil luba__iqrarat"
                                  onKeyDown={(hadath: KeyboardEvent<HTMLDivElement>) => {
                                    if (hadath.key === 'Escape') {
                                      hadath.stopPropagation();
                                      alaIlghaSual();
                                    }
                                  }}
                                >
                                  {/* Only for the patches it is true of. Shown beside
                                      an exact match it would be a sentence about a
                                      risk that is not there, which is the fastest way
                                      to teach somebody that these boxes are furniture. */}
                                  {mudkhal.yahtaj_iqrar ? (
                                    <IqrarKhatar
                                      muarrif={`luba-taqribi-${mifta}`}
                                      unwan={t('luba.khatar.taqribi.unwan', lugha)}
                                      tahdheer={t('luba.khatar.taqribi.tahdheer', lugha)}
                                      // The registry's own verdict on this patch
                                      // against this build, in the reader's language.
                                      tafsil={wasfMutabaqa(mudkhal, lugha)}
                                      nassIqrar={t('luba.khatar.taqribi.iqrar', lugha)}
                                      muqirr={iqrarTaqribi}
                                      alaTabdil={setIqrarTaqribi}
                                      matlub={t('luba.khatar.taqribi.matlub', lugha)}
                                    />
                                  ) : null}
                                  {/* Never required, because nothing on this screen
                                      knows whether this game is played with other
                                      people — the patch listing carries no such
                                      field. An untouched box is a "no" the backend
                                      acts on, which is the correct value; what it
                                      must never be is a yes nobody said. */}
                                  <IqrarKhatar
                                    muarrif={`luba-shabaka-${mifta}`}
                                    unwan={t('luba.khatar.shabaka.unwan', lugha)}
                                    tahdheer={t('luba.khatar.shabaka.tahdheer', lugha)}
                                    nassIqrar={t('luba.khatar.shabaka.iqrar', lugha)}
                                    muqirr={iqrarShabaka}
                                    alaTabdil={setIqrarShabaka}
                                  />
                                  <div className="luba__saff-afal">
                                    <button
                                      type="button"
                                      className="zir zir--khatar"
                                      aria-disabled={
                                        tathbeet.isPending ||
                                        muqfal ||
                                        (mudkhal.yahtaj_iqrar && !iqrarTaqribi)
                                      }
                                      aria-describedby={
                                        mudkhal.yahtaj_iqrar && !iqrarTaqribi
                                          ? muarrifMatlub(`luba-taqribi-${mifta}`)
                                          : undefined
                                      }
                                      onClick={() => {
                                        alaTanfidhSual(mudkhal);
                                      }}
                                    >
                                      {t('luba.khatar.tathbeet', lugha)}
                                    </button>
                                    <button
                                      type="button"
                                      className="zir"
                                      ref={zirIlghaIqrarRef}
                                      onClick={alaIlghaSual}
                                    >
                                      {t('luba.khatar.ilgha', lugha)}
                                    </button>
                                  </div>
                                </div>
                              </motion.div>
                            ) : null}
                          </AnimatePresence>
                        </>
                      )}
                    </li>
                  );
                })}
              </ul>
            )}
          </Mashhad>
          <div className="luba__mintaqa" aria-live="polite">
            {tathbeet.isPending ? (
              <p className="luba__jari">{marhala ?? t('luba.ruqaa.jari_tathbeet', lugha)}</p>
            ) : null}
            <Zuhur
              maftuh={tathbeet.isPending && tanzil !== null && tanzil.majmu > 0}
              className="luba__miqyas"
            >
              {tanzil === null ? null : (
                <>
                  <div
                    className="luba__miqyas-masar"
                    role="progressbar"
                    aria-label={t('luba.ruqaa.tanzil', lugha)}
                    aria-valuemin={0}
                    aria-valuemax={tanzil.majmu}
                    aria-valuenow={Math.min(tanzil.manqul, tanzil.majmu)}
                  >
                    <span
                      className="luba__miqyas-malu"
                      style={{
                        inlineSize: `${String(Math.min(100, (tanzil.manqul / tanzil.majmu) * 100))}%`,
                      }}
                    />
                  </div>
                  <p className="luba__miqyas-nass">
                    {t('amm.taqaddum.min', lugha, {
                      tamma: tanzil.manqul_maqru,
                      majmu: tanzil.majmu_maqru,
                    })}
                  </p>
                </>
              )}
            </Zuhur>
            {tathbeet.error !== null ? (
              <KutlatKhata
                unwan={t('luba.khata.amal', lugha)}
                khata={tathbeet.error}
                lugha={lugha}
                muarrif={muarrif}
              >
                {/* Not `aada`. The install pipeline answers every refusal with
                    one code, so the sentence above may be a question — "this
                    game is multiplayer and you have not accepted that" — and a
                    retry that sends the same two answers again is guaranteed to
                    earn the same refusal. This reopens the acknowledgements on
                    the row that was pressed instead, with the refusal still on
                    screen above them. */}
                {tathbeet.variables === undefined ? null : (
                  <button type="button" className="zir" onClick={alaMurajaatIqrar}>
                    {t('luba.khatar.muraja', lugha)}
                  </button>
                )}
              </KutlatKhata>
            ) : null}
            <Zuhur maftuh={tathbeet.data !== undefined} className="luba__natija">
              {tathbeet.data === undefined ? null : (
                <>
                  <p className="luba__natija-nass">
                    <span
                      className={
                        tathbeet.data.tahaqquq_salim
                          ? 'luba__nuqta luba__nuqta--najah'
                          : 'luba__nuqta luba__nuqta--khatar'
                      }
                      aria-hidden="true"
                    />
                    {jam('luba.ruqaa.tamma', lugha, tathbeet.data.adad_muhtawa, munassiq)}
                  </p>
                  <p className="luba__nass-hadi">{tathbeet.data.tawafuq_arabi}</p>
                  <p className="luba__nass-hadi">{tathbeet.data.tahaqquq_arabi}</p>
                </>
              )}
            </Zuhur>
          </div>
        </>
      )}
    </section>
  );
}

/* ---------------------------------------------------------------------------
   تعريبات المجتمع — what other teams already made for this game.

   A list of credits and addresses, read from the registry's community index
   and nothing else. Taarib hosts none of it, verifies none of it and installs
   none of it: every row names who made the work, where it lives and what its
   own page says about coverage, method and terms, and the one control on the
   row opens that page in the browser. The maker is the first thing on every
   row because the whole panel exists to point at other people's work.

   Three states, like every other panel's own query: the skeleton, the list or
   its empty state, and the failure with its retry. The empty state is only
   drawn when the index was actually read — a machine that could not fetch it
   and has no cached copy gets the failure, not "no known translation", because
   the backend refuses rather than answering an empty list in that case.

   The page opens through `iftah_rabt`, never through an anchor: the webview
   has no browser to open a link in, and the command is where the address is
   checked against the index it came from before anything is launched.
   --------------------------------------------------------------------------- */

/** The registry's own repository, where a translation the index lacks is added. */
const RABT_MUSAHAMA = 'https://github.com/cc1a2b/taarib-registry';

const MIFTAH_MUDIF: Readonly<Record<MudifTarjama, MiftahLugha>> = {
  steam_workshop: 'luba.mujtama.mudif.steam_workshop',
  nexusmods: 'luba.mujtama.mudif.nexusmods',
  thunderstore: 'luba.mujtama.mudif.thunderstore',
  gamebanana: 'luba.mujtama.mudif.gamebanana',
  github: 'luba.mujtama.mudif.github',
  itch: 'luba.mujtama.mudif.itch',
  outerwildsmods: 'luba.mujtama.mudif.outerwildsmods',
  mawqi_alfariq: 'luba.mujtama.mudif.mawqi_alfariq',
  mudawwana: 'luba.mujtama.mudif.mudawwana',
  majhul: 'luba.mujtama.mudif.majhul',
};

/**
 * The hosts that are somebody's own site rather than a platform, where the
 * kind alone names nothing and the host itself is added beside it.
 */
const MUDIFUN_KHASSA: ReadonlySet<MudifTarjama> = new Set<MudifTarjama>([
  'mawqi_alfariq',
  'mudawwana',
  'majhul',
]);

const MIFTAH_TAGHTIYA_MUJTAMA: Readonly<Record<TaghtiyaMujtama, MiftahLugha>> = {
  kamila: 'luba.mujtama.taghtiya.kamila',
  wajiha: 'luba.mujtama.taghtiya.wajiha',
  hiwar: 'luba.mujtama.taghtiya.hiwar',
  juziya: 'luba.mujtama.taghtiya.juziya',
  ghayr_musarraha: 'luba.mujtama.taghtiya.ghayr_musarraha',
  majhul: 'luba.mujtama.taghtiya.majhul',
};

const MIFTAH_TAREEQA_MUJTAMA: Readonly<Record<TareeqaMujtama, MiftahLugha>> = {
  bashariya_kamila: 'luba.mujtama.tareeqa.bashariya_kamila',
  aaliya_thum_bashariya: 'luba.mujtama.tareeqa.aaliya_thum_bashariya',
  aaliya_faqat: 'luba.mujtama.tareeqa.aaliya_faqat',
  ghayr_musarraha: 'luba.mujtama.tareeqa.ghayr_musarraha',
  majhul: 'luba.mujtama.tareeqa.majhul',
};

const MIFTAH_RUKHSA: Readonly<Record<NawRukhsa, MiftahLugha>> = {
  ghayr_musarraha: 'luba.mujtama.rukhsa.ghayr_musarraha',
  musarraha: 'luba.mujtama.rukhsa.musarraha',
  spdx: 'luba.mujtama.rukhsa.spdx',
  majhul: 'luba.mujtama.rukhsa.majhul',
};

const MIFTAH_TAWZEE: Readonly<Record<TawzeeTarjama, MiftahLugha>> = {
  majjani: 'luba.mujtama.tawzee.majjani',
  madfua: 'luba.mujtama.tawzee.madfua',
  vip: 'luba.mujtama.tawzee.vip',
  majhul: 'luba.mujtama.tawzee.majhul',
};

const MIFTAH_HALAT_MUJTAMA: Readonly<Record<HalatTarjamaMujtama, MiftahLugha>> = {
  nashita: 'luba.mujtama.hala.nashita',
  awwaliya: 'luba.mujtama.hala.awwaliya',
  qayd_altatwir: 'luba.mujtama.hala.qayd_altatwir',
  mahjura: 'luba.mujtama.hala.mahjura',
  majhul: 'luba.mujtama.hala.majhul',
};

/** The label over a page's count, per what the page counted. */
const MIFTAH_TANZEELAT: Readonly<Record<NawTanzeelat, MiftahLugha>> = {
  tanzeelat: 'luba.mujtama.tanzeelat',
  mushtarikun: 'luba.mujtama.mushtarikun',
  majhul: 'luba.mujtama.tanzeelat',
};

/**
 * A `YYYY-MM-DD` from the index as the reader's calendar writes it.
 *
 * Built from its three parts rather than parsed as a string: `new Date('2026-05-27')`
 * is midnight UTC, which west of Greenwich is the evening of the 26th, and a
 * publication date is a day and not an instant. A value that is not three
 * numbers is shown as it came rather than as "Invalid Date".
 */
function tareekhMujtama(nass: string, munassiq: Intl.DateTimeFormat): string {
  const ajza = nass.split('-').map(Number);
  const [sana, shahr, yawm] = ajza;
  if (
    ajza.length !== 3 ||
    sana === undefined ||
    shahr === undefined ||
    yawm === undefined ||
    !Number.isInteger(sana) ||
    !Number.isInteger(shahr) ||
    !Number.isInteger(yawm)
  ) {
    return nass;
  }
  return munassiq.format(new Date(sana, shahr - 1, yawm));
}

interface KhasaisMadkhalMujtama {
  readonly tarjama: TarjamaMujtamaHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly munTareekh: Intl.DateTimeFormat;
  /** Whether this row's page is the one being opened right now. */
  readonly yaftah: boolean;
  readonly iftah: () => void;
}

/** One community translation: who made it first, then what its page says. */
function MadkhalMujtama({
  tarjama,
  lugha,
  munassiq,
  munTareekh,
  yaftah,
  iftah,
}: KhasaisMadkhalMujtama): JSX.Element {
  // The team in the reader's script when it has a name in it, the other script
  // otherwise. A team whose credited author is itself is named once, not as
  // "X — X".
  const ismFariq =
    lugha === 'arabi'
      ? (tarjama.fariq_arabi ?? tarjama.fariq)
      : (tarjama.fariq ?? tarjama.fariq_arabi);
  const itiman =
    ismFariq === null || ismFariq === tarjama.muallif
      ? t('luba.mujtama.min_amal_muallif', lugha, { muallif: tarjama.muallif })
      : t('luba.mujtama.min_amal', lugha, { fariq: ismFariq, muallif: tarjama.muallif });
  const mudif = MUDIFUN_KHASSA.has(tarjama.mudif)
    ? `${t(MIFTAH_MUDIF[tarjama.mudif], lugha)} (${tarjama.mudif_ism})`
    : t(MIFTAH_MUDIF[tarjama.mudif], lugha);
  // A recognised licence is named by its identifier, which is the one word a
  // reader can look up; the other kinds are named by how the page stated them.
  const rukhsa =
    tarjama.rukhsa === 'spdx' && tarjama.rukhsa_muarrif !== null
      ? tarjama.rukhsa_muarrif
      : t(MIFTAH_RUKHSA[tarjama.rukhsa], lugha);

  return (
    <li className="luba__lawhat-tathbeet luba__mujtama-madkhal">
      <div className="luba__raas-lawha">
        <span className="luba__ruqaa-unwan" dir="auto">
          {itiman}
        </span>
        <span className="luba__ruqaa-musahim">
          {t(MIFTAH_HALAT_MUJTAMA[tarjama.hala], lugha)}
          {' · '}
          {t(MIFTAH_TAWZEE[tarjama.tawzee], lugha)}
        </span>
      </div>
      <dl className="luba__ruqaa-tafsil">
        <Saff unwan={t('luba.mujtama.mudif', lugha)}>
          <span dir="auto">{mudif}</span>
        </Saff>
        <Saff unwan={t('luba.mujtama.taghtiya', lugha)}>
          {t(MIFTAH_TAGHTIYA_MUJTAMA[tarjama.taghtiya], lugha)}
          {tarjama.taghtiya_nass === null ? null : (
            <span className="luba__mujtama-nass" dir="auto">
              {tarjama.taghtiya_nass}
            </span>
          )}
        </Saff>
        <Saff unwan={t('luba.mujtama.tareeqa', lugha)}>
          {t(MIFTAH_TAREEQA_MUJTAMA[tarjama.tareeqa], lugha)}
        </Saff>
        <Saff unwan={t('luba.mujtama.rukhsa', lugha)}>
          <span dir="auto">{rukhsa}</span>
          {tarjama.rukhsa_nass === null ? null : (
            <span className="luba__mujtama-nass" dir="auto">
              {tarjama.rukhsa_nass}
            </span>
          )}
        </Saff>
        {tarjama.isdar === null ? null : (
          <Saff unwan={t('luba.mujtama.isdar', lugha)}>
            <span dir="auto">{tarjama.isdar}</span>
          </Saff>
        )}
        {tarjama.waqt_alnashr === null ? null : (
          <Saff unwan={t('luba.mujtama.nashr', lugha)}>
            {tareekhMujtama(tarjama.waqt_alnashr, munTareekh)}
          </Saff>
        )}
        {tarjama.akhir_tahdith === null ? null : (
          <Saff unwan={t('luba.mujtama.tahdith', lugha)}>
            {tareekhMujtama(tarjama.akhir_tahdith, munTareekh)}
          </Saff>
        )}
        {tarjama.tanzeelat === null ? null : (
          <Saff unwan={t(MIFTAH_TANZEELAT[tarjama.tanzeelat_naw ?? 'tanzeelat'], lugha)}>
            {munassiq.raqm(tarjama.tanzeelat)}
          </Saff>
        )}
      </dl>
      <p className="luba__nass-hadi luba__mujtama-tahaqquq">
        {t('luba.mujtama.tahaqquq', lugha, {
          waqt: tareekhMujtama(tarjama.waqt_tahaqquq, munTareekh),
        })}
      </p>
      <div className="luba__saff-afal luba__mujtama-afal">
        <button
          type="button"
          className="zir zir--tamyeez"
          aria-busy={yaftah}
          onClick={iftah}
        >
          {t(yaftah ? 'luba.mujtama.jari_fath' : 'luba.mujtama.iftah', lugha)}
        </button>
        <span className="mono-ltr luba__mujtama-rabt" dir="ltr" title={tarjama.rabt}>
          {tarjama.rabt}
        </span>
      </div>
    </li>
  );
}

interface KhasaisMujtama {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly arqam: NizamArqam;
  readonly munassiq: Munassiqat;
}

function QismMujtama({ muarrif, lugha, arqam, munassiq }: KhasaisMujtama): JSX.Element {
  const mujtama = useQuery<TarjamaMujtamaHie[], KhataJisr>({
    queryKey: mafatih.mujtama(muarrif),
    queryFn: () => nadi('tarjamat_mujtama', { muarrif }),
    // The index changes when a team publishes, weeks apart, and the backend
    // holds it for a day; re-asking on every visit would re-read the same
    // cache for the same answer.
    staleTime: 5 * 60_000,
  });
  const munTareekh = useMemo(
    () =>
      new Intl.DateTimeFormat(wasm(lugha), {
        dateStyle: 'medium',
        numberingSystem: ANZIMAT_ARQAM[arqam],
      }),
    [lugha, arqam],
  );

  // One mutation for every page on the panel, the empty state's included: the
  // row that was pressed is the one that is busy, and the others stay
  // controls. A refusal is said where the reader is, with the backend's own
  // sentence and code, because a browser that did not open leaves no other
  // trace on the screen.
  const fath = useMutation<boolean, KhataJisr, string>({
    mutationFn: (rabt) => nadi('iftah_rabt', { rabt }),
    onError: (khata) => {
      ansha({
        naw: 'khatar',
        nass: t('luba.mujtama.khata_fath', lugha),
        tafsil: khata.nass(lugha) ?? khata.message,
        ramz: khata.khata?.ramz ?? null,
      });
    },
  });
  const iftah = (rabt: string): void => {
    if (!fath.isPending) {
      fath.mutate(rabt);
    }
  };
  const yaftah = (rabt: string): boolean => fath.isPending && fath.variables === rabt;

  return (
    <section
      className="luba__qism"
      aria-labelledby="luba-unwan-mujtama"
      aria-busy={fath.isPending}
    >
      <h2 id="luba-unwan-mujtama" className="luba__unwan-qism">
        {t('luba.mujtama.unwan', lugha)}
      </h2>
      <p className="luba__nass-hadi luba__mujtama-sharh">{t('luba.mujtama.sharh', lugha)}</p>
      <Mashhad miftah={wajhIstifsar(mujtama)}>
        {mujtama.isPending ? (
          <HaykalSutur sutur={4} nass={t('luba.mujtama.jari', lugha)} />
        ) : mujtama.error !== null ? (
          <KutlatKhata
            unwan={t('luba.mujtama.taadhur', lugha)}
            khata={mujtama.error}
            lugha={lugha}
            muarrif={muarrif}
            aada={() => {
              void mujtama.refetch();
            }}
          />
        ) : mujtama.data === undefined || mujtama.data.length === 0 ? (
          <HalatFarigha
            unwan={t('luba.mujtama.la_shay_unwan', lugha)}
            nass={t('luba.mujtama.la_shay', lugha)}
          >
            <button
              type="button"
              className="zir"
              aria-busy={yaftah(RABT_MUSAHAMA)}
              onClick={() => {
                iftah(RABT_MUSAHAMA);
              }}
            >
              {t('luba.mujtama.musahama', lugha)}
            </button>
          </HalatFarigha>
        ) : (
          <ul className="luba__mujtama">
            {mujtama.data.map((tarjama) => (
              <MadkhalMujtama
                key={tarjama.muarrif}
                tarjama={tarjama}
                lugha={lugha}
                munassiq={munassiq}
                munTareekh={munTareekh}
                yaftah={yaftah(tarjama.rabt)}
                iftah={() => {
                  iftah(tarjama.rabt);
                }}
              />
            ))}
          </ul>
        )}
      </Mashhad>
    </section>
  );
}

export function Luba(): JSX.Element {
  const { muarrif } = wajihat.useParams();
  const makhzan = useQueryClient();
  const intiqal = useNavigate();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);
  const munWaqt = useMemo(() => munassiqWaqt(lugha, arqam), [lugha, arqam]);

  const nassWaqt = useCallback(
    (khaam: string): string => {
      const tarikh = new Date(khaam);
      return Number.isNaN(tarikh.getTime()) ? khaam : munWaqt.format(tarikh);
    },
    [munWaqt],
  );

  const tafsil = useQuery<TafasilLuba, KhataJisr>({
    queryKey: mafatih.tafasil(muarrif),
    queryFn: () => nadi('tafasil_luba', { muarrif }),
  });

  // Its own query, because it opens the game's containers and that is seconds
  // on a large game. The rest of the screen draws immediately; only the
  // arabization section waits, and it waits visibly.
  const lughaRasmiya = useQuery<LughaRasmiyaHie, KhataJisr>({
    queryKey: mafatih.lugha_rasmiya(muarrif),
    queryFn: () => nadi('lugha_rasmiya', { muarrif }),
    // A verdict is a fact about files on disk that only a game update moves,
    // and the walk behind it is expensive: re-asking every thirty seconds
    // because the user moved between screens would pay for it again and again.
    staleTime: Infinity,
  });

  const himaya = useMutation<HimayaHie, KhataJisr, void>({
    mutationFn: () => nadi('fahs_himaya', { muarrif }),
    // The palette can start this from anywhere on the screen, so the verdict
    // is said where the reader is as well as in the panel that shows it.
    onSuccess: (hukm) => {
      ansha(
        hukm.mahmiya
          ? { naw: 'tanbeeh', nass: t('luba.tanbih.aman_mahmiya', lugha) }
          : { naw: 'najah', nass: t('luba.tanbih.aman_salima', lugha) },
      );
    },
  });

  const tahaqquq = useMutation<TaqreerTahaqquqHie[], KhataJisr, void>({
    mutationFn: () => nadi('tahaqquq_ruqaa', { muarrif }),
  });

  const izala = useMutation<HasilatIzala, KhataJisr, TalabIzala>({
    mutationFn: ({ matlab, siyasa }) => nadi('azil_ruqaa', { muarrif, matlab, siyasa }),
    onSuccess: (hasila, { matlab }) => {
      // The result block stands at the foot of the screen. A removal that came
      // off whole is confirmed; one that did not is not dressed as a success,
      // and the notice sends the reader to the block that lists why.
      ansha(
        hasila.najahat
          ? { naw: 'najah', nass: t(MIFTAH_TANBIH_IZALA[matlab], lugha) }
          : { naw: 'tanbeeh', nass: t('luba.tanbih.izala_naqisa', lugha) },
      );
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
      // Both plans described the game as it was a moment ago. The removal put
      // files back and deleted others, so what an install would write and what a
      // second removal would find are now different answers.
      void makhzan.invalidateQueries({ queryKey: mafatih.khutta(muarrif) });
      void makhzan.invalidateQueries({ queryKey: ['khuttat_izala', muarrif] });
    },
  });

  const tashghil = useQuery<HalatTashghil, KhataJisr>({
    queryKey: mafatih.tashghil(muarrif),
    queryFn: () => nadi('hal_tashtaghil', { muarrif }),
  });

  const fahsMuharrik = useMutation<TaqreerHie, KhataJisr, void>({
    mutationFn: () => nadi('afhas_muharrik', { muarrif }),
    // The command overwrote the stored report, so a fresh detail read is the
    // report it just wrote. A probe that lands on the same verdict changes
    // nothing visible, which is why the notice says it ran.
    onSuccess: () => {
      ansha({ naw: 'najah', nass: t('luba.tanbih.fahs_muharrik_tamma', lugha) });
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
    },
  });

  // Locked while the game runs — and locked while we do not know whether it
  // runs. A failed check leaves `data` undefined, and reading that as "not
  // running" would unlock installing into and deleting files out of a game
  // that may have them open. The one safe reading of "we could not tell" is
  // the one that refuses.
  const yashtaghil = tashghil.data?.tashtaghil !== false;
  // Distinguished only for the sentence. A sandboxed build sees its own process
  // table rather than the host's, so it cannot observe a running game at all —
  // and telling somebody "close the game" when nothing was seen running sends
  // them to close a window that may not be open. The refusal is the same either
  // way; only the explanation differs.
  const tashghilMajhul = tashghil.data?.majhul === true || tashghil.isError;
  const { yahtaj: yahtajIqrar } = useIqrarAwwal();
  const muqfal = yashtaghil || yahtajIqrar;
  // The reason, once, for every control the lock refuses: the sentence under
  // the buttons and the title on each of them are the same words.
  const sababQafl = yashtaghil
    ? tashghilMajhul
      ? t('luba.tashghil.majhul', lugha)
      : t('luba.tashghil.tahdheer', lugha, { amaliya: tashghil.data?.amaliya ?? '' })
    : yahtajIqrar
      ? t('luba.iqrar.qabl', lugha)
      : null;

  const [taakid, setTaakid] = useState<MatlabIzala | null>(null);
  const [yaknus, setYaknus] = useState(false);
  const [suturZahira, setSuturZahira] = useState(false);

  const zirIzalatNassRef = useRef<HTMLButtonElement | null>(null);
  const zirIzalatSawtRef = useRef<HTMLButtonElement | null>(null);
  const zirIstiadaRef = useRef<HTMLButtonElement | null>(null);
  const zirIlghaRef = useRef<HTMLButtonElement | null>(null);
  const mintaqatIzalaRef = useRef<HTMLDivElement | null>(null);

  const aidTahaqquq = tahaqquq.reset;
  const aidIzala = izala.reset;
  const aidHimaya = himaya.reset;
  const aidFahsMuharrik = fahsMuharrik.reset;
  useEffect(() => {
    setTaakid(null);
    setSuturZahira(false);
    aidTahaqquq();
    aidIzala();
    aidHimaya();
    aidFahsMuharrik();
  }, [muarrif, aidTahaqquq, aidIzala, aidHimaya, aidFahsMuharrik]);

  useEffect(() => {
    if (taakid !== null) {
      zirIlghaRef.current?.focus();
    }
    // Opening a confirmation always starts from the answer that destroys
    // nothing, whichever way the last one was answered.
    setYaknus(false);
  }, [taakid]);

  useEffect(() => {
    if (izala.isPending) {
      mintaqatIzalaRef.current?.focus();
    }
  }, [izala.isPending]);

  const alaFahsMuharrik = useCallback(() => {
    if (!fahsMuharrik.isPending) {
      fahsMuharrik.mutate();
    }
  }, [fahsMuharrik]);

  const alaFahsHimaya = useCallback(() => {
    if (!himaya.isPending) {
      himaya.mutate();
    }
  }, [himaya]);

  // The palette registers once per id set, so its actions read the handlers
  // through a ref that always holds the current closures.
  const afalHaliya = useRef({ muharrik: alaFahsMuharrik, aman: alaFahsHimaya });
  useEffect(() => {
    afalHaliya.current = { muharrik: alaFahsMuharrik, aman: alaFahsHimaya };
  });

  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    if (idadat.data === undefined) {
      return [];
    }
    const majal = t('shasha.luba', lugha);
    return [
      {
        muarrif: `luba.${muarrif}.warsha`,
        unwan: t('luba.lawha.warsha', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/warsha/$muarrif', params: { muarrif } });
        },
      },
      {
        muarrif: `luba.${muarrif}.taqdeem`,
        unwan: t('luba.lawha.taqdeem', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/taqdeem/$muarrif', params: { muarrif } });
        },
      },
      {
        muarrif: `luba.${muarrif}.tabaqa`,
        unwan: t('luba.lawha.tabaqa', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/tabaqa/$muarrif', params: { muarrif } });
        },
      },
      {
        muarrif: `luba.${muarrif}.fahs-muharrik`,
        unwan: t('luba.lawha.fahs_muharrik', lugha),
        majal,
        nafidh: () => {
          afalHaliya.current.muharrik();
        },
      },
      {
        muarrif: `luba.${muarrif}.fahs-aman`,
        unwan: t('luba.lawha.fahs_aman', lugha),
        majal,
        nafidh: () => {
          afalHaliya.current.aman();
        },
      },
    ];
  }, [idadat.data, lugha, muarrif, intiqal]);
  useSajjilAwamir(awamirShasha);

  // The same query the confirmation's plan draws, asked here for one fact: is
  // there anything a sweep would take. Sharing the key means one call answers
  // both, and the control cannot offer to remove what the list does not show.
  const khuttatIzala = useQuery<KhuttatIzalaHie[], KhataJisr>({
    queryKey: mafatih.khuttat_izala(muarrif, taakid ?? 'kul'),
    queryFn: () => nadi('khuttat_izala', { muarrif, matlab: taakid ?? 'kul' }),
    enabled: taakid !== null,
  });
  const baqaya = useMemo(
    () => (khuttatIzala.data ?? []).flatMap((khutwa) => khutwa.baqaya),
    [khuttatIzala.data],
  );
  const lahaBaqaya = baqaya.length > 0;
  const adadBaqaya = baqaya.reduce((majmu, baqiya) => majmu + baqiya.adad, 0);

  const alaTalabTaakid = (matlab: MatlabIzala): void => {
    if (izala.isPending || muqfal) {
      return;
    }
    setTaakid(matlab);
  };

  const alaIlghaTaakid = (): void => {
    if (taakid === 'nass') {
      zirIzalatNassRef.current?.focus();
    } else if (taakid === 'sawt') {
      zirIzalatSawtRef.current?.focus();
    } else if (taakid === 'kul') {
      zirIstiadaRef.current?.focus();
    }
    setTaakid(null);
  };

  const alaTanfidhTaakid = (): void => {
    if (taakid === null || izala.isPending || muqfal) {
      return;
    }
    // The sweep is only sendable while the plan in front of the reader still
    // names residue: the query below refreshes after every removal, so a stale
    // tick cannot authorise a deletion nobody is looking at.
    izala.mutate({
      matlab: taakid,
      siyasa: yaknus && lahaBaqaya ? 'kanasa' : 'muhafiza',
    });
    setTaakid(null);
  };

  const alaTahaqquq = (): void => {
    if (!tahaqquq.isPending) {
      tahaqquq.mutate();
    }
  };

  const yuhammil = tafsil.isPending || idadat.isPending;
  const khata = tafsil.error ?? idadat.error;
  const bayanat = tafsil.data;
  const himayaHali = himaya.data ?? bayanat?.himaya ?? null;
  const mahmiya = himayaHali?.mahmiya === true;

  const ghilaf = bayanat?.ghilaf ?? null;
  const masdarGhilaf = useMemo(
    () => (ghilaf === null ? null : convertFileSrc(ghilaf)),
    [ghilaf],
  );
  const lawnLuba = useLawnLuba(masdarGhilaf, bayanat?.lawn ?? null);
  const uslubLuba = useMemo<CSSProperties | undefined>(
    () =>
      lawnLuba === null
        ? undefined
        : // A custom property is not part of `CSSProperties`, and the style
          // attribute is the only way to scope one to a single element.
          ({ '--tamyeez-luba': lawnLuba } as CSSProperties),
    [lawnLuba],
  );

  const aidIstifsar = (): void => {
    void tafsil.refetch();
    void idadat.refetch();
  };

  const wajh = yuhammil ? 'tahmil' : khata !== null ? 'khata' : bayanat === undefined ? 'tahmil' : 'jahiz';

  return (
    <div className="luba" style={uslubLuba}>
      <RaasShasha
        rujoo={{ ila: 'maktaba' }}
        nassRujoo={t('luba.raji', lugha)}
        unwan={t('shasha.luba', lugha)}
        mawdu={bayanat?.ism ?? null}
        rawabit={
          <>
            <Link to="/warsha/$muarrif" params={{ muarrif }} className="raas-shasha__rabt">
              {t('shasha.warsha', lugha)}
            </Link>
            <Link to="/taqdeem/$muarrif" params={{ muarrif }} className="raas-shasha__rabt">
              {t('shasha.taqdeem', lugha)}
            </Link>
            <Link to="/tabaqa/$muarrif" params={{ muarrif }} className="raas-shasha__rabt">
              {t('shasha.tabaqa', lugha)}
            </Link>
          </>
        }
      />

      <Mashhad miftah={wajh} className="luba__jism">
        {yuhammil ? (
          <div className="luba__haykal zuhur-muakhkhar" aria-hidden="true">
            <div className="luba__haykal-mirsa">
              <span className="luba__haykal-ghilaf" />
              <div className="luba__haykal-mirsa-nass">
                <span className="luba__haykal-unwan" />
                <span className="haykal__satr haykal__satr--tawil" />
                <span className="haykal__satr haykal__satr--mutawassit" />
                <span className="haykal__satr haykal__satr--qasir" />
              </div>
            </div>
            <div className="luba__haykal-amida">
              <div className="luba__haykal-amud">
                <HaykalQism sutur={6} />
                <HaykalQism sutur={4} />
                <HaykalQism sutur={5} />
                <HaykalQism sutur={4} />
              </div>
              <div className="luba__haykal-amud">
                <HaykalQism sutur={5} />
              </div>
            </div>
          </div>
        ) : khata !== null ? (
          <KutlatKhata unwan={t('amm.khata', lugha)} khata={khata} lugha={lugha} muarrif={muarrif}>
            <button type="button" className="zir" onClick={aidIstifsar}>
              {t('amm.iaada', lugha)}
            </button>
          </KutlatKhata>
        ) : bayanat === undefined ? null : (
          <>
            <header className="luba__mirsa">
              {/*
                Named for the document's view transition under the same name
                the library gives this game's card, so the cover travels from
                the grid to here on the way in and back on the way out.
              */}
              {bayanat.ghilaf === null ? (
                <span
                  className="luba__ghilaf luba__ghilaf--faragh"
                  aria-hidden="true"
                  style={{ viewTransitionName: ismIntiqalGhilaf(muarrif) }}
                />
              ) : (
                <img
                  className="luba__ghilaf"
                  src={convertFileSrc(bayanat.ghilaf)}
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                  style={{ viewTransitionName: ismIntiqalGhilaf(muarrif) }}
                />
              )}
              <div className="luba__mirsa-nass">
                <h2 className="luba__ism">{bayanat.ism}</h2>
                <dl className="luba__jadwal">
                  <Saff unwan={t('luba.mirsa.masar', lugha)}>
                    <span className="mono-ltr luba__qat" title={bayanat.jidhr}>
                      {bayanat.jidhr}
                    </span>
                  </Saff>
                  <Saff unwan={t('luba.mirsa.masadir', lugha)}>
                    {bayanat.manassat.length > 0 ? (
                      <Riqaqat qaima={bayanat.manassat} />
                    ) : (
                      <span className="luba__nass-hadi">{t('luba.mirsa.la_masadir', lugha)}</span>
                    )}
                  </Saff>
                </dl>
              </div>
            </header>

            <div className="luba__amida">
              <div className="luba__raisi">
                <QismMuharrik
                  muarrif={muarrif}
                  muharrik={bayanat.muharrik}
                  taqreer={bayanat.taqreer}
                  lugha={lugha}
                  munassiq={munassiq}
                  yajriFahs={fahsMuharrik.isPending}
                  alaAadaFahs={alaFahsMuharrik}
                  khataFahs={fahsMuharrik.error}
                />

                <QismHimaya
                  muarrif={muarrif}
                  himaya={himayaHali}
                  lugha={lugha}
                  yajri={himaya.isPending}
                  alaFahs={alaFahsHimaya}
                  khata={himaya.error}
                />

                {/* Under the protection panel on purpose: it is the panel whose
                    verdict a reader is most likely to want the workings of, and
                    the chain restates that verdict beside the four others rather
                    than in a place they have to go looking for. */}
                <QismAql muarrif={muarrif} lugha={lugha} />

                <QismIqrar
                  lugha={lugha}
                  luba={muarrif}
                  matlub={yahtajIqrar ? t('luba.iqrar.qabl', lugha) : null}
                />

                <QismRuqaa
                  muarrif={muarrif}
                  lugha={lugha}
                  munassiq={munassiq}
                  taqreer={bayanat.taqreer}
                  mahmiya={mahmiya}
                  muqfal={muqfal}
                  sababQafl={sababQafl}
                  hukmLugha={lughaRasmiya.data ?? null}
                  yajriFahsLugha={lughaRasmiya.isPending}
                  khataLugha={lughaRasmiya.error}
                  alaAadaLugha={() => {
                    void lughaRasmiya.refetch();
                  }}
                  istibdal={idadat.data?.istibdal_lugha_rasmiya === true}
                />

                {/* Under the registry's own listing, because it answers the
                    same question from the other direction: what Arabic exists
                    for this game that Taarib did not make and will not install. */}
                <QismMujtama muarrif={muarrif} lugha={lugha} arqam={arqam} munassiq={munassiq} />

                <section
                  className="luba__qism"
                  aria-labelledby="luba-unwan-afal"
                  aria-busy={izala.isPending || tahaqquq.isPending}
                >
                  <h2 id="luba-unwan-afal" className="luba__unwan-qism">
                    {t('luba.afal.unwan', lugha)}
                  </h2>

                  <div className="luba__afal">
                    {!bayanat.muthabbat_nass && !bayanat.muthabbat_sawt ? (
                      <p className="luba__nass-hadi">{t('luba.afal.la_tathbeet', lugha)}</p>
                    ) : (
                      <>
                        <div
                          className="luba__saff-afal"
                          role="group"
                          aria-label={t('luba.afal.majmuat_tahaqquq', lugha)}
                        >
                          <button
                            type="button"
                            className="zir"
                            aria-busy={tahaqquq.isPending}
                            onClick={alaTahaqquq}
                          >
                            {t('luba.afal.tahaqquq', lugha)}
                          </button>
                        </div>

                        {sababQafl === null ? null : (
                          <p className="luba__nass-hadi luba__tahdheer">{sababQafl}</p>
                        )}
                        <div
                          className="luba__saff-afal"
                          role="group"
                          aria-label={t('luba.afal.majmuat_izala', lugha)}
                        >
                          {bayanat.muthabbat_nass ? (
                            <button
                              type="button"
                              className="zir"
                              ref={zirIzalatNassRef}
                              aria-disabled={muqfal || (izala.isPending && izala.variables?.matlab !== 'nass')}
                              aria-busy={izala.isPending && izala.variables?.matlab === 'nass'}
                              title={sababQafl ?? undefined}
                              onClick={() => {
                                alaTalabTaakid('nass');
                              }}
                            >
                              {t('luba.afal.izalat_nass', lugha)}
                            </button>
                          ) : null}
                          {bayanat.muthabbat_sawt ? (
                            <button
                              type="button"
                              className="zir"
                              ref={zirIzalatSawtRef}
                              aria-disabled={muqfal || (izala.isPending && izala.variables?.matlab !== 'sawt')}
                              aria-busy={izala.isPending && izala.variables?.matlab === 'sawt'}
                              title={sababQafl ?? undefined}
                              onClick={() => {
                                alaTalabTaakid('sawt');
                              }}
                            >
                              {t('luba.afal.izalat_sawt', lugha)}
                            </button>
                          ) : null}
                          <button
                            type="button"
                            className="zir zir--khatar"
                            ref={zirIstiadaRef}
                            aria-disabled={muqfal || (izala.isPending && izala.variables?.matlab !== 'kul')}
                            aria-busy={izala.isPending && izala.variables?.matlab === 'kul'}
                            title={sababQafl ?? undefined}
                            onClick={() => {
                              alaTalabTaakid('kul');
                            }}
                          >
                            {t('luba.afal.istiada', lugha)}
                          </button>
                        </div>
                      </>
                    )}

                    <AnimatePresence initial={false}>
                      {taakid !== null ? (
                        <motion.div
                          key="taakid"
                          className="luba__tawassu"
                          initial={{ height: 0, opacity: 0 }}
                          animate={{ height: 'auto', opacity: 1 }}
                          exit={{ height: 0, opacity: 0 }}
                          transition={haraka(HARAKAT_LAWHA)}
                        >
                          <div className="luba__tawassu-dakhil">
                            <div
                              className="luba__taakid"
                              role="group"
                              aria-labelledby="luba-nass-taakid"
                              onKeyDown={(hadath: KeyboardEvent<HTMLDivElement>) => {
                                if (hadath.key === 'Escape') {
                                  hadath.stopPropagation();
                                  alaIlghaTaakid();
                                }
                              }}
                            >
                              <p id="luba-nass-taakid" className="luba__taakid-nass">
                                {t(MIFTAH_TAAKID[taakid], lugha)}
                              </p>
                              {/* The removal's own dry run, between the question
                                  and the button that answers it. The sweep this
                                  confirmation authorises can delete files
                                  Taarib never wrote, and those files are named
                                  here — a count cannot be consented to. */}
                              <KhuttatIzala
                                muarrif={muarrif}
                                matlab={taakid}
                                lugha={lugha}
                                munassiq={munassiq}
                              />
                              {lahaBaqaya ? (
                                <label className="luba__kans">
                                  <input
                                    type="checkbox"
                                    checked={yaknus}
                                    onChange={(hadath) => {
                                      setYaknus(hadath.currentTarget.checked);
                                    }}
                                  />
                                  <span>
                                    {t('luba.izala.iknis', lugha, {
                                      adad: munassiq.raqm(adadBaqaya),
                                    })}
                                  </span>
                                </label>
                              ) : null}
                              {lahaBaqaya && yaknus ? (
                                <p className="luba__kans-tahdheer" role="alert">
                                  {t('luba.izala.iknis_tahdheer', lugha)}
                                </p>
                              ) : null}
                              <div className="luba__taakid-azrar">
                                <button
                                  type="button"
                                  className="zir zir--khatar"
                                  onClick={alaTanfidhTaakid}
                                >
                                  {t('luba.taakid.tanfidh', lugha)}
                                </button>
                                <button
                                  type="button"
                                  className="zir"
                                  ref={zirIlghaRef}
                                  onClick={alaIlghaTaakid}
                                >
                                  {t('luba.taakid.ilgha', lugha)}
                                </button>
                              </div>
                            </div>
                          </div>
                        </motion.div>
                      ) : null}
                    </AnimatePresence>

                    <div className="luba__mintaqa" aria-live="polite">
                      {tahaqquq.isPending ? (
                        <p className="luba__jari">{t('luba.tahaqquq.jari', lugha)}</p>
                      ) : null}
                      {tahaqquq.error !== null ? (
                        <KutlatKhata
                          unwan={t('luba.khata.amal', lugha)}
                          khata={tahaqquq.error}
                          lugha={lugha}
                          muarrif={muarrif}
                          aada={alaTahaqquq}
                        />
                      ) : null}
                      <AnimatePresence initial={false}>
                        {tahaqquq.data !== undefined ? (
                          <motion.section
                            key="natijat-tahaqquq"
                            className="luba__natija"
                            aria-labelledby="luba-unwan-natijat-tahaqquq"
                            initial={{ opacity: 0, y: 6 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0 }}
                            transition={haraka(HARAKAT_LAWHA)}
                          >
                            <h3 id="luba-unwan-natijat-tahaqquq" className="luba__unwan-natija">
                              {t('luba.tahaqquq.unwan', lugha)}
                            </h3>
                            {tahaqquq.data.length === 0 ? (
                              <p className="luba__nass-hadi">{t('luba.tahaqquq.la_shay', lugha)}</p>
                            ) : (
                              tahaqquq.data.map((taqreer) => (
                                <div key={taqreer.naw} className="luba__natija-band">
                                  <p className="luba__natija-nass">
                                    <span
                                      className={
                                        taqreer.salim
                                          ? 'luba__nuqta luba__nuqta--najah'
                                          : 'luba__nuqta luba__nuqta--khatar'
                                      }
                                      aria-hidden="true"
                                    />
                                    {t(miftahNaw(taqreer.naw === 'nass' ? 'nass' : 'sawt'), lugha)}
                                    {' — '}
                                    {lugha === 'arabi'
                                      ? taqreer.natija_arabi
                                      : taqreer.natija_injilizi}
                                  </p>
                                  <ul className="luba__adad-natija">
                                    <li>
                                      {t('luba.tahaqquq.masarat', lugha, {
                                        adad: munassiq.raqm(taqreer.adad_masarat),
                                      })}
                                    </li>
                                    <li>
                                      {t('luba.tahaqquq.munharif', lugha, {
                                        adad: munassiq.raqm(taqreer.adad_munharif),
                                      })}
                                    </li>
                                    <li>
                                      {t('luba.tahaqquq.mafqud', lugha, {
                                        adad: munassiq.raqm(taqreer.adad_mafqud),
                                      })}
                                    </li>
                                  </ul>
                                  {taqreer.munharifa.length > 0 ? (
                                    <ul className="luba__sutur">
                                      {taqreer.munharifa.map((masar, fihris) => (
                                        <li
                                          key={`${masar}-${String(fihris)}`}
                                          className="mono-ltr luba__satr-masar"
                                          title={masar}
                                        >
                                          {masar}
                                        </li>
                                      ))}
                                    </ul>
                                  ) : null}
                                </div>
                              ))
                            )}
                          </motion.section>
                        ) : null}
                      </AnimatePresence>
                    </div>

                    <div
                      className="luba__mintaqa"
                      ref={mintaqatIzalaRef}
                      tabIndex={-1}
                      aria-live="polite"
                    >
                      {izala.isPending ? (
                        <p className="luba__jari">{t('luba.istiada.jari', lugha)}</p>
                      ) : null}
                      {izala.error !== null ? (
                        <KutlatKhata
                          unwan={t('luba.khata.amal', lugha)}
                          khata={izala.error}
                          lugha={lugha}
                          muarrif={muarrif}
                          aada={() => {
                            const talab = izala.variables;
                            if (talab !== undefined && !izala.isPending) {
                              izala.mutate(talab);
                            }
                          }}
                        />
                      ) : null}
                      <AnimatePresence initial={false}>
                        {izala.data !== undefined ? (
                          <motion.section
                            key="natijat-izala"
                            className="luba__natija"
                            aria-labelledby="luba-unwan-natijat-izala"
                            initial={{ opacity: 0, y: 6 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0 }}
                            transition={haraka(HARAKAT_LAWHA)}
                          >
                            <h3 id="luba-unwan-natijat-izala" className="luba__unwan-natija">
                              {t('luba.istiada.unwan', lugha)}
                            </h3>
                            <p className="luba__natija-nass">
                              <span
                                className={
                                  izala.data.najahat
                                    ? 'luba__nuqta luba__nuqta--najah'
                                    : 'luba__nuqta luba__nuqta--khatar'
                                }
                                aria-hidden="true"
                              />
                              {t(
                                izala.data.najahat
                                  ? 'luba.istiada.najah_kul'
                                  : 'luba.istiada.naqis',
                                lugha,
                              )}
                            </p>
                            {izala.data.sutur.length > 0 ? (
                              <>
                                <div className="luba__saff-afal">
                                  <button
                                    type="button"
                                    className="zir"
                                    aria-expanded={suturZahira}
                                    aria-controls="luba-sutur-izala"
                                    onClick={() => {
                                      setSuturZahira((hali) => !hali);
                                    }}
                                  >
                                    {t(
                                      suturZahira
                                        ? 'luba.tahaqquq.ikhfa_sutur'
                                        : 'luba.tahaqquq.izhar_sutur',
                                      lugha,
                                    )}
                                  </button>
                                </div>
                                <AnimatePresence initial={false}>
                                  {suturZahira ? (
                                    <motion.ul
                                      key="sutur-izala"
                                      id="luba-sutur-izala"
                                      className="luba__sutur"
                                      initial={{ height: 0, opacity: 0 }}
                                      animate={{ height: 'auto', opacity: 1 }}
                                      exit={{ height: 0, opacity: 0 }}
                                      transition={haraka(HARAKAT_LAWHA)}
                                    >
                                      {izala.data.sutur.map((satr, fihris) => (
                                        <li
                                          key={`${satr}-${String(fihris)}`}
                                          className="luba__satr-tahaqquq"
                                        >
                                          {satr}
                                        </li>
                                      ))}
                                    </motion.ul>
                                  ) : null}
                                </AnimatePresence>
                              </>
                            ) : null}
                            {izala.data.akhta.length > 0 ? (
                              <div className="luba__mustabdala">
                                <p className="luba__nass-hadi">{t('luba.istiada.akhta', lugha)}</p>
                                <ul className="luba__sutur">
                                  {izala.data.akhta.map((satr, fihris) => (
                                    <li
                                      key={`${satr}-${String(fihris)}`}
                                      className="luba__satr-tahaqquq"
                                    >
                                      {satr}
                                    </li>
                                  ))}
                                </ul>
                              </div>
                            ) : null}
                          </motion.section>
                        ) : null}
                      </AnimatePresence>
                    </div>
                  </div>
                </section>
              </div>

              <div className="luba__janib">
                <QismBina
                  bina={bayanat.bina}
                  muthabbat={bayanat.muthabbat_nass || bayanat.muthabbat_sawt}
                  lugha={lugha}
                  munassiq={munassiq}
                  nassWaqt={nassWaqt}
                />
              </div>
            </div>
          </>
        )}
      </Mashhad>

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
