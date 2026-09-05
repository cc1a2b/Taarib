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
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t, wasm } from '@/lugha/lugha';
import type { JahiziyaTashghil } from '@/maktaba/jahiziya';
import { jahiziyaMin, naqsJahiziya, tasil } from '@/maktaba/jahiziya';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type {
  BinaHie,
  DaleelHie,
  HalatIqrar,
  HalatTashghil,
  HasilatIzala,
  HimayaHie,
  Idadat,
  LawnBariz,
  Lugha,
  LughaRasmiyaHie,
  MatlabIzala,
  MuharrikHie,
  NatijatTathbeetHie,
  NizamArqam,
  RuqaaLuba,
  TafasilLuba,
  TaqreerHie,
  TaqreerTahaqquqHie,
} from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './luba.css';

/** شاشة اللعبة — one game's real facts, its patches, its safety verdict, and the actions the backend answers for. */

const wajihat = getRouteApi('/luba/$muarrif');

const MIFTAH_TAAKID: Readonly<Record<MatlabIzala, MiftahLugha>> = {
  nass: 'luba.taakid.izalat_nass',
  sawt: 'luba.taakid.izalat_sawt',
  kul: 'luba.taakid.istiada',
};

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
      {qaima.map((band) => (
        <li key={band} className="luba__riqaqa">
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

/** The engine, as the readiness notice names it: family, and version if known. */
function ismMuharrik(muharrik: MuharrikHie): string {
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
      <dl className="luba__jadwal">
        <Saff unwan={t('luba.muharrik.aila', lugha)}>{muharrik.aila}</Saff>
        {muharrik.isdar === null ? null : (
          <Saff unwan={t('luba.muharrik.isdar', lugha)}>
            <span className="mono-ltr">{muharrik.isdar}</span>
          </Saff>
        )}
        <Saff unwan={t('luba.muharrik.khalfiya', lugha)}>{muharrik.khalfiya}</Saff>
        {muharrik.rusum.length > 0 ? (
          <Saff unwan={t('luba.muharrik.rusum', lugha)}>
            <Riqaqat qaima={muharrik.rusum} />
          </Saff>
        ) : null}
        <Saff unwan={t('luba.muharrik.tabaqa', lugha)}>
          {t('luba.muharrik.tabaqa_qeema', lugha, {
            raqm: munassiq.raqm(taqreer.tabaqa_raqm),
            ism: taqreer.tabaqa_arabi,
          })}
          {/* The qualification belongs on the claim, not three lines under it:
              a tier names what Taarib may do, and this row is where a reader
              decides it is what Taarib will do. Read off the verdict and not
              off the sentence beneath it: an older report can carry the verdict
              without the sentence, and the tier would then be stated flat. */}
          {hukmJahiziya === null || hukmJahiziya === 'mukammala' ? null : (
            <span className="luba__tabaqa-muallaqa">
              {t('luba.jahiziya.ghayr_faal', lugha)}
            </span>
          )}
        </Saff>
        <Saff unwan={t('luba.muharrik.thiqa', lugha)}>{munassiq.nisba(muharrik.thiqa)}</Saff>
      </dl>
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
      <div className="luba__imkaniyat">
        <h3 className="luba__unwan-farii">{t('luba.imkaniyat.sabab', lugha)}</h3>
        <p className="luba__sabab">
          {lugha === 'arabi' ? taqreer.sabab_arabi : taqreer.sabab_injilizi}
        </p>
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
              {hudud.map((hadd, martaba) => (
                <li key={`${String(martaba)}:${hadd}`}>{hadd}</li>
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
          <button type="button" className="zir" aria-disabled={yajriFahs} onClick={alaAadaFahs}>
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
        {dalailZahira ? (
          dalail.isPending ? (
            <p className="luba__jari">{t('luba.dalail.jari', lugha)}</p>
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
            <p className="luba__nass-hadi">{t('luba.dalail.la_shay', lugha)}</p>
          ) : (
            <ul id="luba-dalail" className="luba__adilla">
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
          )
        ) : null}
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

interface KhasaisRuqaa {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** The capability report, for the readiness notice above the install button. */
  readonly taqreer: TaqreerHie;
  readonly mahmiya: boolean;
  /** Actions are locked while the game runs or the first-run statement is unacknowledged. */
  readonly muqfal: boolean;
  readonly yashtaghil: boolean;
  /** Whether {@link yashtaghil} is a precaution rather than an observation. */
  readonly tashghilMajhul: boolean;
  readonly amaliya: string | null;
  readonly yahtajIqrar: boolean;
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
  yashtaghil,
  tashghilMajhul,
  amaliya,
  yahtajIqrar,
  hukmLugha,
  yajriFahsLugha,
  khataLugha,
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
    },
  });

  const [marhala, setMarhala] = useState<string | null>(null);
  useEffect(() => {
    const ilgha = listen<string>(HADATH_MARHALAT_TATHBEET, (hadath) => {
      setMarhala(hadath.payload);
    });
    const ilghaTanzeel = listen<{ marhala_arabi: string }>(HADATH_TAQADDUM_TANZEEL, (hadath) => {
      setMarhala(hadath.payload.marhala_arabi);
    });
    return () => {
      void ilgha.then((f) => { f(); });
      void ilghaTanzeel.then((f) => { f(); });
    };
  }, []);

  const tathbeet = useMutation<NatijatTathbeetHie, KhataJisr, { ruqaa: string; iqrar: boolean }>({
    mutationFn: async ({ ruqaa: idRuqaa, iqrar }) => {
      const hasila = await nadi('nazzil_ruqaa', { muarrif, ruqaa: idRuqaa });
      return nadi('thabbit_ruqaa', {
        muarrif,
        masarMalaf: hasila.masar,
        iqrarShabaka: iqrar,
        iqrarTaqribi: iqrar,
      });
    },
    onSettled: () => {
      setMarhala(null);
    },
    onSuccess: () => {
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
    },
  });

  const naqs = naqsJahiziya(taqreer, lugha);
  const hukmJahiziya = jahiziyaMin(taqreer.jahiziya);

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
        <p className="luba__jari">{t('luba.lugha.jari', lugha)}</p>
      ) : null}
      {khataLugha === null ? null : (
        <KutlatKhata
          unwan={t('luba.lugha.taadhur', lugha)}
          khata={khataLugha}
          lugha={lugha}
          muarrif={muarrif}
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
          {yashtaghil ? (
            <p className="luba__nass-hadi luba__tahdheer">
              {tashghilMajhul
                ? t('luba.tashghil.majhul', lugha)
                : t('luba.tashghil.tahdheer', lugha, { amaliya: amaliya ?? '' })}
            </p>
          ) : yahtajIqrar ? (
            <p className="luba__nass-hadi luba__tahdheer">{t('luba.iqrar.qabl', lugha)}</p>
          ) : null}
          {ruqaa.isPending ? (
            <p className="luba__jari">{t('luba.ruqaa.jari', lugha)}</p>
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
              <p className="luba__nass-hadi">{t('luba.ruqaa.la_shay', lugha)}</p>
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
              <button
                type="button"
                className="zir"
                aria-disabled={talab.isPending}
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
              {ruqaa.data.mudkhalat.map((mudkhal) => (
                <li
                  key={`${mudkhal.id}-${String(mudkhal.murajaa)}`}
                  className="luba__lawhat-tathbeet"
                >
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
                    <Saff unwan={t('luba.muharrik.tabaqa', lugha)}>
                      {t('luba.muharrik.tabaqa_qeema', lugha, {
                        raqm: munassiq.raqm(mudkhal.tabaqa_raqm),
                        ism: mudkhal.tabaqa_arabi,
                      })}
                    </Saff>
                  </dl>
                  {mahmiya ? null : (
                    <div className="luba__saff-afal">
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={tathbeet.isPending || muqfal}
                        onClick={() => {
                          if (!tathbeet.isPending && !muqfal) {
                            tathbeet.mutate({
                              ruqaa: String(mudkhal.id),
                              iqrar: mudkhal.yahtaj_iqrar,
                            });
                          }
                        }}
                      >
                        {t('luba.ruqaa.tathbeet', lugha)}
                      </button>
                      {mudkhal.mutabaqa_arabi === null ? null : (
                        <span className="luba__nass-hadi">{mudkhal.mutabaqa_arabi}</span>
                      )}
                    </div>
                  )}
                </li>
              ))}
            </ul>
          )}
          <div className="luba__mintaqa" aria-live="polite">
            {tathbeet.isPending ? (
              <p className="luba__jari">{marhala ?? t('luba.ruqaa.jari_tathbeet', lugha)}</p>
            ) : null}
            {tathbeet.error !== null ? (
              <KutlatKhata
                unwan={t('luba.khata.amal', lugha)}
                khata={tathbeet.error}
                lugha={lugha}
                muarrif={muarrif}
                aada={() => {
                  const akhira = tathbeet.variables;
                  if (akhira !== undefined && !tathbeet.isPending && !muqfal) {
                    tathbeet.mutate(akhira);
                  }
                }}
              />
            ) : null}
            {tathbeet.data === undefined ? null : (
              <div className="luba__natija">
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
              </div>
            )}
          </div>
        </>
      )}
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
  });

  const tahaqquq = useMutation<TaqreerTahaqquqHie[], KhataJisr, void>({
    mutationFn: () => nadi('tahaqquq_ruqaa', { muarrif }),
  });

  const izala = useMutation<HasilatIzala, KhataJisr, MatlabIzala>({
    mutationFn: (matlab) => nadi('azil_ruqaa', { muarrif, matlab, sarim: false }),
    onSuccess: () => {
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
    },
  });

  const tashghil = useQuery<HalatTashghil, KhataJisr>({
    queryKey: mafatih.tashghil(muarrif),
    queryFn: () => nadi('hal_tashtaghil', { muarrif }),
  });

  const iqrar = useQuery<HalatIqrar, KhataJisr>({
    queryKey: mafatih.iqrar,
    queryFn: () => nadi('iqrar_aman'),
  });

  const sajjilIqrar = useMutation<HalatIqrar, KhataJisr, void>({
    mutationFn: () => nadi('sajjil_iqrar_aman'),
    onSuccess: (hala) => {
      makhzan.setQueryData(mafatih.iqrar, hala);
    },
  });

  const fahsMuharrik = useMutation<TaqreerHie, KhataJisr, void>({
    mutationFn: () => nadi('afhas_muharrik', { muarrif }),
    // The command overwrote the stored report, so a fresh detail read is the
    // report it just wrote.
    onSuccess: () => {
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
  const yahtajIqrar = iqrar.data?.yahtaj !== false;
  const muqfal = yashtaghil || yahtajIqrar;

  const [taakid, setTaakid] = useState<MatlabIzala | null>(null);
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
  const aidSajjilIqrar = sajjilIqrar.reset;
  useEffect(() => {
    setTaakid(null);
    setSuturZahira(false);
    aidTahaqquq();
    aidIzala();
    aidHimaya();
    aidFahsMuharrik();
    aidSajjilIqrar();
  }, [muarrif, aidTahaqquq, aidIzala, aidHimaya, aidFahsMuharrik, aidSajjilIqrar]);

  useEffect(() => {
    if (taakid !== null) {
      zirIlghaRef.current?.focus();
    }
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
    izala.mutate(taakid);
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

  return (
    <div className="luba" style={uslubLuba}>
      <header className="luba__shareet-alawi">
        <Link to="/" className="luba__raji">
          {t('luba.raji', lugha)}
        </Link>
        <span className="luba__fasl">{t('shasha.luba', lugha)}</span>
        <Link to="/warsha/$muarrif" params={{ muarrif }} className="luba__raji">
          {t('shasha.warsha', lugha)}
        </Link>
        <Link to="/taqdeem/$muarrif" params={{ muarrif }} className="luba__raji">
          {t('shasha.taqdeem', lugha)}
        </Link>
        <Link to="/tabaqa/$muarrif" params={{ muarrif }} className="luba__raji">
          {t('shasha.tabaqa', lugha)}
        </Link>
      </header>

      <div className="luba__jism">
        {yuhammil ? (
          <div className="luba__haykal" aria-hidden="true">
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
              {bayanat.ghilaf === null ? (
                <span className="luba__ghilaf luba__ghilaf--faragh" aria-hidden="true" />
              ) : (
                <img
                  className="luba__ghilaf"
                  src={convertFileSrc(bayanat.ghilaf)}
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                />
              )}
              <div className="luba__mirsa-nass">
                <h1 className="luba__ism">{bayanat.ism}</h1>
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

                {iqrar.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={iqrar.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      void iqrar.refetch();
                    }}
                  />
                ) : yahtajIqrar && iqrar.data !== undefined ? (
                  <section
                    className="luba__qism luba__qism--iqrar"
                    aria-labelledby="luba-unwan-iqrar"
                  >
                    <h2 id="luba-unwan-iqrar" className="luba__unwan-qism">
                      {t('luba.iqrar.unwan', lugha)}
                    </h2>
                    <p className="luba__sabab" dir="rtl" lang="ar">
                      {iqrar.data.nass_arabi}
                    </p>
                    <div className="luba__saff-afal">
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={sajjilIqrar.isPending}
                        onClick={() => {
                          if (!sajjilIqrar.isPending) {
                            sajjilIqrar.mutate();
                          }
                        }}
                      >
                        {t(sajjilIqrar.isPending ? 'luba.iqrar.jari' : 'luba.iqrar.zirr', lugha)}
                      </button>
                    </div>
                    {sajjilIqrar.error !== null ? (
                      <KutlatKhata
                        unwan={t('luba.khata.amal', lugha)}
                        khata={sajjilIqrar.error}
                        lugha={lugha}
                        muarrif={muarrif}
                        aada={() => {
                          if (!sajjilIqrar.isPending) {
                            sajjilIqrar.mutate();
                          }
                        }}
                      />
                    ) : null}
                  </section>
                ) : null}

                <QismRuqaa
                  muarrif={muarrif}
                  lugha={lugha}
                  munassiq={munassiq}
                  taqreer={bayanat.taqreer}
                  mahmiya={mahmiya}
                  muqfal={muqfal}
                  yashtaghil={yashtaghil}
                  tashghilMajhul={tashghilMajhul}
                  amaliya={tashghil.data?.amaliya ?? null}
                  yahtajIqrar={yahtajIqrar}
                  hukmLugha={lughaRasmiya.data ?? null}
                  yajriFahsLugha={lughaRasmiya.isPending}
                  khataLugha={lughaRasmiya.error}
                  istibdal={idadat.data?.istibdal_lugha_rasmiya === true}
                />

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
                            aria-disabled={tahaqquq.isPending}
                            onClick={alaTahaqquq}
                          >
                            {t('luba.afal.tahaqquq', lugha)}
                          </button>
                        </div>

                        {yashtaghil ? (
                          <p className="luba__nass-hadi luba__tahdheer">
                            {tashghilMajhul
                              ? t('luba.tashghil.majhul', lugha)
                              : t('luba.tashghil.tahdheer', lugha, {
                                  amaliya: tashghil.data?.amaliya ?? '',
                                })}
                          </p>
                        ) : yahtajIqrar ? (
                          <p className="luba__nass-hadi luba__tahdheer">
                            {t('luba.iqrar.qabl', lugha)}
                          </p>
                        ) : null}
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
                              aria-disabled={izala.isPending || muqfal}
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
                              aria-disabled={izala.isPending || muqfal}
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
                            aria-disabled={izala.isPending || muqfal}
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
                            const matlab = izala.variables;
                            if (matlab !== undefined && !izala.isPending) {
                              izala.mutate(matlab);
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

        <p className="khafi" role="status">
          {yuhammil ? t('amm.tahmil', lugha) : ''}
        </p>
      </div>
    </div>
  );
}
