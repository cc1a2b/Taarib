import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, useNavigate, useRouterState } from '@tanstack/react-router';
import { motion } from 'motion/react';
import type { JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { useTaraju } from '@/hayat/taraju';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t } from '@/lugha/lugha';
import {
  AHJAM_BITAQA,
  HALAT,
  TAJMIAT,
  TARATIB,
  useKhiyaratMaktaba,
} from '@/maktaba/hifz_khiyarat';
import { jahiziyaSaf, tasil } from '@/maktaba/jahiziya';
import type { FahrasBahth, LubaGhaiba, SijillLuba, TalabMaktaba } from '@/maktaba/tanqiya';
import { ansha_fahrasat, nasseq } from '@/maktaba/tanqiya';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type { KhasaisRamz } from '@/mukawwinat/rumuz';
import {
  RamzIdadat,
  RamzMaktaba,
  RamzMuraja,
  RamzSahmAala,
  RamzSahmAsfal,
  RamzTabaqa,
  RamzTalabat,
  RamzTanbeeh,
  RamzTaqdeem,
  RamzTashkhis,
  RamzWarsha,
  ramz,
} from '@/mukawwinat/rumuz';
import { ShabakatMaktaba } from '@/mukawwinat/shabakat_maktaba';
import type {
  HalatLuba,
  HasilatMaktaba,
  Idadat,
  JalsaHie,
  Lugha,
  MaalumatTaarib,
  Manassa,
  NizamArqam,
  SijillGhaib,
  SijillMaktaba,
} from '@/mustalahat/awamir';
import { HARAKAT_MASAR, haraka } from '@/nizam/haraka';

import './maktaba.css';

/**
 * المكتبة — the Library, and the shell every other screen is mounted inside.
 *
 * The shell is a two-column grid: a fixed rail of screens on the leading edge
 * and one content region beside it. Both edges between them are hairlines, not
 * shadows, because a surface in this product declares its level by luminance
 * and a single-pixel border and never by floating above the one underneath it.
 */

/* ==========================================================================
   The five glyphs this screen needs and the family in `rumuz.tsx` does not
   carry yet. Built through that file's own `ramz` contract rather than as bare
   SVG, so the 24×24 grid, the single stroke weight, the round terminals and
   the decorative default cannot drift away from the eight glyphs beside them.
   They live here rather than in the family because the family is a shared file
   this phase does not own; if a second screen ever wants one, that is the
   moment they move.
   ========================================================================== */

/** The lens is 13 units across and the pupil 6, so the counter survives 16px. */
function RamzBahth(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <circle cx="10.5" cy="10.5" r="6.5" />
      <path d="M15.5 15.5L21 21" />
    </>,
  );
}

/** A chevron, not an arrow: the arrows already mean sort direction in this row. */
function RamzSuqut(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M5.5 9.5L12 16L18.5 9.5" />);
}

/** A game is a cover with a strip under it — the product's own card, at 24 units. */
function RamzLuba(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="5.5" y="2.5" width="13" height="19" rx="1" />
      <path d="M5.5 16.5H18.5" />
    </>,
  );
}

/** Handedness rather than direction, so it never mirrors. */
function RamzMuayana(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M2.5 12C5 7 8.5 5 12 5s7 2 9.5 7c-2.5 5-6 7-9.5 7s-7-2-9.5-7Z" />
      <circle cx="12" cy="12" r="3" />
    </>,
  );
}

/** The contributor, not the contribution: this screen lists people's work. */
function RamzMusahamat(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <circle cx="12" cy="6" r="3.5" />
      <path d="M4 20.5a8 8 0 0 1 16 0" />
    </>,
  );
}

/** One entry in the navigation rail. */
interface BandTanaqqul {
  /** Stable key, matching the screen's directory name. */
  readonly muarrif: string;
  /** The screen's name, in the string set. */
  readonly miftah: MiftahLugha;
  /** The route it navigates to, or null while the screen has no route. */
  readonly masar: '/' | '/talabat' | '/muraja' | '/idadat' | '/tashkhis' | null;
  /** Its glyph, at the rail's own size. */
  readonly ramz: (khasais: KhasaisRamz) => JSX.Element;
}

/** A titled run of rail entries. */
interface MajmuatTanaqqul {
  /** Stable key. */
  readonly muarrif: string;
  /** The run's heading, or null when the run is the landing screen alone. */
  readonly unwan: MiftahLugha | null;
  /** One sentence under the heading saying why the run reads the way it does. */
  readonly tanbih: MiftahLugha | null;
  readonly shashat: readonly BandTanaqqul[];
}

/**
 * The eleven screens, in three runs.
 *
 * The runs are not decoration. Six of these screens are scoped to a game and
 * have no route until one is open, so on the landing screen they are all
 * disabled at once — and a rail that opens with six greyed lines and no reason
 * reads as a broken product rather than as an honest one. Naming the run and
 * saying, once, what would enable it turns the same six lines into an answer.
 * Nothing is hidden and nothing is promised: the entries are still listed,
 * still disabled, and still in the order the product presents them.
 */
const MAJMUAT: readonly MajmuatTanaqqul[] = [
  {
    muarrif: 'jidhr',
    unwan: null,
    tanbih: null,
    shashat: [{ muarrif: 'maktaba', miftah: 'shasha.maktaba', masar: '/', ramz: RamzMaktaba }],
  },
  {
    muarrif: 'luba',
    unwan: 'maktaba.tanaqqul.luba',
    tanbih: 'maktaba.tanaqqul.bila_luba',
    shashat: [
      { muarrif: 'luba', miftah: 'shasha.luba', masar: null, ramz: RamzLuba },
      { muarrif: 'warsha', miftah: 'shasha.warsha', masar: null, ramz: RamzWarsha },
      { muarrif: 'muayana', miftah: 'shasha.muayana', masar: null, ramz: RamzMuayana },
      { muarrif: 'taqdeem', miftah: 'shasha.taqdeem', masar: null, ramz: RamzTaqdeem },
      { muarrif: 'musahamat', miftah: 'shasha.musahamat', masar: null, ramz: RamzMusahamat },
      { muarrif: 'tabaqa', miftah: 'shasha.tabaqa', masar: null, ramz: RamzTabaqa },
    ],
  },
  {
    muarrif: 'taarib',
    unwan: 'maktaba.tanaqqul.taarib',
    tanbih: null,
    shashat: [
      { muarrif: 'muraja', miftah: 'shasha.muraja', masar: '/muraja', ramz: RamzMuraja },
      { muarrif: 'talabat', miftah: 'shasha.talabat', masar: '/talabat', ramz: RamzTalabat },
      { muarrif: 'idadat', miftah: 'shasha.idadat', masar: '/idadat', ramz: RamzIdadat },
      { muarrif: 'tashkhis', miftah: 'shasha.tashkhis', masar: '/tashkhis', ramz: RamzTashkhis },
    ],
  },
];

/** The operating system's name, in the string set. */
function miftahNizam(nizam: MaalumatTaarib['nizam']): MiftahLugha {
  switch (nizam) {
    case 'windows':
      return 'nizam.windows';
    case 'linux':
      return 'nizam.linux';
    case 'mac':
      return 'nizam.mac';
  }
}

/** The architecture's name, in the string set. */
function miftahMimariya(mimariya: MaalumatTaarib['mimariya']): MiftahLugha {
  switch (mimariya) {
    case 'x86':
      return 'mimariya.x86';
    case 'x8664':
      return 'mimariya.x8664';
    case 'aarch64':
      return 'mimariya.aarch64';
  }
}

/** Every launcher family's Arabic name, for rows the backend names by slug. */
const ASMA_MANASSAT: Readonly<Record<Manassa, string>> = {
  steam: 'ستيم',
  epic: 'إيبك',
  gog: 'غوغ',
  ea: 'إي إيه',
  ubisoft: 'يوبيسوفت',
  battlenet: 'باتل نت',
  xbox: 'إكس بوكس',
  itch: 'إتش',
  amazon: 'أمازون',
  rockstar: 'روكستار',
  riot: 'رايوت',
  lutris: 'لوتريس',
  bottles: 'بوتلز',
  playnite: 'بلاي\u200cنايت',
  yadawi: 'مضافة يدويًا',
};

/** The same table, keyed by the wire spelling, for a list sent as bare slugs. */
const ASMA_BIL_SHIFRA: ReadonlyMap<string, string> = new Map(Object.entries(ASMA_MANASSAT));

/**
 * The launchers a scan searched, named for a person.
 *
 * `HasilatMaktaba.manassat` is a list of slugs rather than the `Manassa` union
 * because it names what the scan *looked at*, and a backend newer than this
 * build can look at a launcher this build has no name for. An unrecognised slug
 * is passed through as it came: the wire spelling still tells the user which
 * launcher it was, and folding it into `yadawi` would tell them Taarib searched
 * "added by hand", which is not a launcher and not what happened.
 *
 * Deduplicated, because two unrecognised slugs must not print one name twice in
 * a sentence whose whole job is to list them.
 *
 * @param shifrat the slugs the scan reported, in the order it reported them
 */
function asmaManassat(shifrat: readonly string[]): string[] {
  const asma: string[] = [];
  for (const shifra of shifrat) {
    const ism = ASMA_BIL_SHIFRA.get(shifra) ?? shifra;
    if (!asma.includes(ism)) {
      asma.push(ism);
    }
  }
  return asma;
}

function milli(waqt: string | null): number | null {
  if (waqt === null) {
    return null;
  }
  const qeema = Date.parse(waqt);
  return Number.isNaN(qeema) ? null : qeema;
}

function sijillLuba(saf: SijillMaktaba): SijillLuba {
  return {
    muarrif: saf.muarrif,
    ism: saf.ism,
    ism_arabi: null,
    asma_badila: [],
    manassa: saf.manassa,
    // The shard family's name, never `saf.ism_manassa`. That field is the
    // *primary launcher's* name, which is a different fact: a game published
    // under the Steam shard and reported by Playnite arrives with
    // `manassa: 'steam'` and a Playnite display name. The grid keys its
    // sections off `manassa` and titles them from this field, so taking the two
    // from different facts is how a section headed "Playnite" fills up with
    // Steam games — and how ticking "Steam" in the filters empties a section
    // that is visibly full of Steam.
    ism_manassa: ASMA_MANASSAT[saf.manassa],
    muharrik: saf.muharrik,
    tabaqa: saf.tabaqa,
    hala: saf.hala,
    nass: saf.nass === 'la_shay' ? 'la-shay' : saf.nass,
    sawt: saf.sawt === 'la_shay' ? 'la-shay' : saf.sawt,
    lugha_rasmiya: saf.lugha_rasmiya,
    // The scan assembles every row from the same cached capability report the
    // game screen reads, so this verdict is already decided for every game in
    // the library — it is read here rather than re-derived, and a row from a
    // build that does not send it yet answers null and the card draws no mark.
    jahiziya: jahiziyaSaf(saf),
    hajm: saf.hajm,
    akhir_laab: milli(saf.akhir_laab),
    akhir_tathbeet: milli(saf.akhir_tathbeet),
    ghilaf: saf.ghilaf,
    batl: saf.batl,
    lawha: saf.lawha,
  };
}

/**
 * A physical viewport x, as a logical inline offset.
 *
 * `clientX` and `getBoundingClientRect()` measure from the viewport's left
 * edge, always. `inset-inline-start` on a fixed element measures from the
 * *reading* start edge — the right edge in Arabic. Handing one to the other
 * mirrors the position across the window: right-click a card at the left of
 * the screen and the menu opens at the right. The flip has to happen here,
 * because the browser will not do it for a value it did not measure.
 *
 * ## Where the flip lives, and why it is only here
 *
 * `bitaqa.tsx` reports the pointer as `clientX` and the keyboard menu key as
 * `getBoundingClientRect().left + width / 2`. Both are physical, both cross
 * `shabakat_maktaba.tsx` untouched, and this screen is the first place either
 * becomes a style. So the flip belongs here, and it is applied at the moment
 * the position is *stored* rather than while the menu renders: that gives
 * {@link QaimatSiyaq.s} exactly one meaning — an inline offset — instead of a
 * number whose meaning depends on which side of the render it is read from.
 *
 * Flipping twice is worse than not flipping at all: not flipping puts the menu
 * on the wrong side, which is visible immediately, and flipping twice puts it
 * back on the right side for a centred click and wrong for every other one. If
 * the emitter is ever changed to report a logical offset, this call is what has
 * to be deleted — not matched with a second one.
 */
function mawqiMantiqi(s: number): number {
  return document.documentElement.dir === 'rtl'
    ? Math.max(0, document.documentElement.clientWidth - s)
    : s;
}

function lubaGhaiba(saf: SijillGhaib): LubaGhaiba {
  return {
    muarrif: saf.muarrif,
    ism: saf.ism,
    manassa: saf.manassa,
    ism_manassa: ASMA_MANASSAT[saf.manassa],
    sabab: saf.sabab,
  };
}

/** An open context menu: which game, and where — in logical coordinates. */
interface QaimatSiyaq {
  readonly muarrif: string;
  /**
   * The offset from the reading start edge. Already through
   * {@link mawqiMantiqi}, so nothing downstream may flip it again.
   */
  readonly s: number;
  /** The offset from the top. Never flipped: the block axis does not mirror. */
  readonly a: number;
}

/* ==========================================================================
   The header's controls.

   Three of them choose one value out of a short closed list, and one of them
   filters. They are built as two different things for that reason: a chooser
   always reads back the value it holds, and a filter reads back its own name
   plus how many values are ticked, because "الحالة" with a 2 beside it is the
   only label that stays true when two states are selected at once.
   ========================================================================== */

/** One chooser: a native select wearing the product's chrome. */
interface KhasaisMihwar<T extends string> {
  /** The control's accessible name; there is no visible label in this row. */
  readonly tasmiya: string;
  readonly qeema: T;
  readonly khiyarat: readonly T[];
  readonly ism: (qeema: T) => string;
  readonly ala_ikhtiyar: (qeema: T) => void;
}

/**
 * The select stays native — its popup is the platform's, its keyboard is the
 * platform's, and a hand-built listbox would be a worse one of both. Only the
 * closed state is restyled, and the chevron is the product's own glyph rather
 * than the engine's, which is the single detail that made four of these read
 * as browser furniture instead of as one row of controls.
 */
function Mihwar<T extends string>({
  tasmiya,
  qeema,
  khiyarat,
  ism,
  ala_ikhtiyar,
}: KhasaisMihwar<T>): JSX.Element {
  return (
    <span className="raas__mihwar">
      <select
        className="raas__ikhtiyar"
        aria-label={tasmiya}
        value={qeema}
        onChange={(hadath) => {
          ala_ikhtiyar(hadath.target.value as T);
        }}
      >
        {khiyarat.map((khiyar) => (
          <option key={khiyar} value={khiyar}>
            {ism(khiyar)}
          </option>
        ))}
      </select>
      <RamzSuqut className="raas__sahm" />
    </span>
  );
}

/** The Arabization-status filter. */
interface KhasaisMurashshihHala {
  readonly mukhtara: readonly HalatLuba[];
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly ala_tabdeel: (hala: HalatLuba) => void;
  readonly ala_imsah: () => void;
}

/**
 * Six tick boxes behind one chip, because the model behind them is a set.
 *
 * `TasfiyatMaktaba.halat` accepts any subset and ORs within the category, so a
 * single-choice control would have to answer for a stored set of two by
 * showing one of them or by showing "all" — the first hides a constraint that
 * is really applied and the second states the opposite of what is happening.
 * A panel of tick boxes is the only control that can be read back honestly.
 *
 * Dismissal is on the document rather than on the panel: a `blur` handler
 * closes it while the pointer is travelling between two tick boxes, and a
 * `mouseleave` handler leaves it open for ever for a keyboard user.
 */
function MurashshihHala({
  mukhtara,
  lugha,
  munassiq,
  ala_tabdeel,
  ala_imsah,
}: KhasaisMurashshihHala): JSX.Element {
  const [maftuh, setMaftuh] = useState(false);
  const marja = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!maftuh) {
      return undefined;
    }
    const alaIshara = (hadath: PointerEvent): void => {
      const hadaf = hadath.target;
      if (marja.current !== null && hadaf instanceof Node && !marja.current.contains(hadaf)) {
        setMaftuh(false);
      }
    };
    const alaMiftah = (hadath: KeyboardEvent): void => {
      if (hadath.key === 'Escape') {
        setMaftuh(false);
      }
    };
    document.addEventListener('pointerdown', alaIshara);
    document.addEventListener('keydown', alaMiftah);
    return () => {
      document.removeEventListener('pointerdown', alaIshara);
      document.removeEventListener('keydown', alaMiftah);
    };
  }, [maftuh]);

  const unwan = t('maktaba.hala.unwan', lugha);
  const nashit = mukhtara.length > 0;

  return (
    <div className="raas__murashshih" ref={marja}>
      <button
        type="button"
        className={nashit ? 'raas__mihwar-zir raas__mihwar-zir--nashit' : 'raas__mihwar-zir'}
        aria-haspopup="true"
        aria-expanded={maftuh}
        onClick={() => {
          setMaftuh(!maftuh);
        }}
      >
        {unwan}
        {nashit ? <span className="raas__wasm">{munassiq.raqm(mukhtara.length)}</span> : null}
        <RamzSuqut className="raas__ramz" />
      </button>
      {maftuh ? (
        <div className="raas__lawha" role="group" aria-label={unwan}>
          {HALAT.map((hala) => (
            <label className="raas__khiyar" key={hala}>
              <input
                type="checkbox"
                checked={mukhtara.includes(hala)}
                onChange={() => {
                  ala_tabdeel(hala);
                }}
              />
              {t(`maktaba.hala.${hala}`, lugha)}
            </label>
          ))}
          {nashit ? (
            <button type="button" className="raas__imsah" onClick={ala_imsah}>
              {t('maktaba.faragh.tasfiya.imsah', lugha)}
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

export function Maktaba(): JSX.Element {
  const makhzanIstifsar = useQueryClient();
  const intiqal = useNavigate();

  const maalumat = useQuery<MaalumatTaarib, KhataJisr>({
    queryKey: mafatih.maalumat,
    queryFn: () => nadi('maalumat_taarib'),
  });

  const jalsa = useQuery<JalsaHie, KhataJisr>({
    queryKey: mafatih.jalsa,
    queryFn: () => nadi('jalsati'),
  });

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });

  const maktaba = useQuery<HasilatMaktaba, KhataJisr>({
    queryKey: mafatih.maktaba,
    queryFn: () => nadi('maktaba'),
    // A launcher scan walks catalogues on disk; twice a minute would be noise.
    staleTime: 5 * 60_000,
  });

  // Arabic and Latin digits until the backend says otherwise, which is what
  // the settings default to anyway: the first frame is never a different
  // language from the second.
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);


  const masarHali = useRouterState({ select: (halat) => halat.location.pathname });

  const hajmBitaqa = useKhiyaratMaktaba((hali) => hali.hajm_bitaqa);
  const tajmi = useKhiyaratMaktaba((hali) => hali.tajmi);
  const tartib = useKhiyaratMaktaba((hali) => hali.tartib);
  const ittijahFarz = useKhiyaratMaktaba((hali) => hali.ittijah);
  const murashshih = useKhiyaratMaktaba((hali) => hali.murashshih);
  const ghiyabMatwi = useKhiyaratMaktaba((hali) => hali.ghiyab_matwi);
  // In the store rather than in this component's own state, so that opening a
  // game and coming back returns the user to their search and not to the whole
  // library. It is still not persisted — see the store's own note.
  const bahth = useKhiyaratMaktaba((hali) => hali.bahth);
  const haddidHajmBitaqa = useKhiyaratMaktaba((hali) => hali.haddidHajmBitaqa);
  const haddidBahth = useKhiyaratMaktaba((hali) => hali.haddidBahth);
  const haddidTajmi = useKhiyaratMaktaba((hali) => hali.haddidTajmi);
  const haddidTartib = useKhiyaratMaktaba((hali) => hali.haddidTartib);
  const aksIttijah = useKhiyaratMaktaba((hali) => hali.aksIttijah);
  const baddilMurashshih = useKhiyaratMaktaba((hali) => hali.baddilMurashshih);
  const amsahMurashshih = useKhiyaratMaktaba((hali) => hali.amsahMurashshih);
  const baddilGhiyab = useKhiyaratMaktaba((hali) => hali.baddilGhiyab);

  const [qaimatSiyaq, setQaimatSiyaq] = useState<QaimatSiyaq | null>(null);
  const [masarIdafa, setMasarIdafa] = useState<string | null>(null);

  /** The search field, so the palette's focus command can reach it. */
  const marjaBahth = useRef<HTMLInputElement>(null);

  const sajjilTaraju = useTaraju((hali) => hali.sajjil);

  const sijillat = useMemo(
    () => (maktaba.data?.alaab ?? []).map(sijillLuba),
    [maktaba.data],
  );
  const ghaiba = useMemo(
    () => (maktaba.data?.ghaiba ?? []).map(lubaGhaiba),
    [maktaba.data],
  );
  /**
   * The readiness verdict for the game the context menu is open on.
   *
   * Read off the same rows the grid draws rather than fetched, so opening a
   * menu costs no command. A game that has left the library between the click
   * and the render answers `null`, which reads as not ready — the safe way
   * round, since the alternative offers a run for a game that is gone.
   */
  const jahiziyatQaima = useMemo(
    () =>
      qaimatSiyaq === null
        ? null
        : (sijillat.find((saf) => saf.muarrif === qaimatSiyaq.muarrif)?.jahiziya ?? null),
    [qaimatSiyaq, sijillat],
  );
  /**
   * The index that was in force before this scan.
   *
   * `ansha_fahrasat` reuses the entry of every game whose searchable text is
   * unchanged, and it can only do that if it is handed the previous index —
   * without this ref the library screen would rebuild four normalizations and
   * two consonant skeletons for every game on every refresh, including the
   * refresh that changed nothing but one game's play time.
   *
   * Written during render, which is safe here because the derivation is
   * idempotent: feeding a run its own output back reuses every entry and
   * produces an equivalent map, so a double invocation under StrictMode and a
   * discarded memo both land in the same place.
   */
  const marjaFahrasat = useRef<ReadonlyMap<string, FahrasBahth>>(new Map());
  const fahrasat = useMemo(() => {
    const jadeed = ansha_fahrasat(sijillat, marjaFahrasat.current);
    marjaFahrasat.current = jadeed;
    return jadeed;
  }, [sijillat]);

  const talab: TalabMaktaba = useMemo(
    () => ({ bahth, murashshih, tartib, ittijah: ittijahFarz, tajmi }),
    [bahth, murashshih, tartib, ittijahFarz, tajmi],
  );
  const natija = useMemo(
    () => nasseq(sijillat, fahrasat, talab, lugha),
    [sijillat, fahrasat, talab, lugha],
  );

  /**
   * Hides a set of games, and records one undo step for the whole set.
   *
   * Takes a list rather than one identity because both callers are really
   * asking the same question — the context menu asks it about one game and the
   * selection bar about forty — and answering them with the same mutation is
   * what makes the undo honest. Firing a single-game mutation in a loop instead
   * pushes forty steps onto the stack, so reversing one gesture costs forty
   * presses of the undo chord, and only the last failure of the forty is
   * anywhere on screen.
   *
   * Sequential, because the backend writes one store and forty concurrent
   * writes are forty attempts at the same lock for no gain at this size.
   */
  const ikhfa = useMutation<readonly string[], KhataJisr, readonly string[]>({
    mutationFn: async (muarrifat) => {
      const mukhfat: string[] = [];
      try {
        for (const muarrif of muarrifat) {
          // eslint-disable-next-line no-await-in-loop
          await nadi('ikhfa_luba', { muarrif, mukhfiya: true });
          mukhfat.push(muarrif);
        }
      } finally {
        // Registered here rather than in `onSuccess` so that a run which failed
        // half way is still reversible. A hide is reversible one game at a
        // time; a step covering only the games that were actually hidden is the
        // only step that tells the truth about what happened.
        if (mukhfat.length > 0) {
          sajjilTaraju({
            wasf: t('maktaba.taraju.ikhfa', lugha),
            taraju: async () => {
              for (const muarrif of mukhfat) {
                // eslint-disable-next-line no-await-in-loop
                await nadi('ikhfa_luba', { muarrif, mukhfiya: false });
              }
              await makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
            },
          });
        }
      }
      return mukhfat;
    },
    // Either way: a partial run hid something, and the grid is showing games
    // that are no longer in it.
    onSettled: () => {
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
    },
  });

  const iftahMujallad = useMutation<boolean, KhataJisr, string>({
    mutationFn: (muarrif) => nadi('iftah_manassa', { muarrif }),
  });

  const [jamaiHala, setJamaiHala] = useState<string | null>(null);
  const jamai = useMutation<number, KhataJisr, readonly string[]>({
    mutationFn: async (muarrifat) => {
      let adad = 0;
      for (const muarrif of muarrifat) {
        setJamaiHala(t('maktaba.jamai.jari', lugha, { adad: munassiq.raqm(adad + 1) }));
        // Sequential on purpose: two installs writing one store would race.
        // eslint-disable-next-line no-await-in-loop
        const ruqaa = await nadi('ruqaa_luba', { muarrif });
        const awwal = ruqaa.mudkhalat.find((mudkhal) => mudkhal.qabila_lil_tathbeet);
        if (awwal === undefined) {
          continue;
        }
        // eslint-disable-next-line no-await-in-loop
        const hasila = await nadi('nazzil_ruqaa', { muarrif, ruqaa: String(awwal.id) });
        // eslint-disable-next-line no-await-in-loop
        await nadi('thabbit_ruqaa', {
          muarrif,
          masarMalaf: hasila.masar,
          iqrarShabaka: awwal.yahtaj_iqrar,
          iqrarTaqribi: awwal.yahtaj_iqrar,
        });
        adad += 1;
      }
      return adad;
    },
    onSettled: () => {
      setJamaiHala(null);
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
    },
  });

  const idafa = useMutation<number, KhataJisr, string>({
    mutationFn: (masar) => nadi('adif_mujallad_fahs', { masar }),
    onSuccess: () => {
      setMasarIdafa(null);
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.idadat });
    },
  });

  /** Stable per observer, so the palette's rescan closure never goes stale. */
  const aadaFahsMaktaba = maktaba.refetch;

  /** The library's palette entries: three screens, the rescan, and the search field. */
  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    const majal = t('shasha.maktaba', lugha);
    return [
      {
        muarrif: 'maktaba.iftah_idadat',
        unwan: t('maktaba.lawha.iftah_idadat', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/idadat' });
        },
      },
      {
        muarrif: 'maktaba.iftah_tashkhis',
        unwan: t('maktaba.lawha.iftah_tashkhis', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/tashkhis' });
        },
      },
      {
        muarrif: 'maktaba.iftah_talabat',
        unwan: t('maktaba.lawha.iftah_talabat', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/talabat' });
        },
      },
      {
        muarrif: 'maktaba.aada_fahs',
        unwan: t('maktaba.lawha.aada_fahs', lugha),
        majal,
        nafidh: () => {
          void aadaFahsMaktaba();
        },
      },
      {
        muarrif: 'maktaba.rakkiz_bahth',
        unwan: t('maktaba.lawha.rakkiz_bahth', lugha),
        majal,
        nafidh: () => {
          marjaBahth.current?.focus();
        },
      },
    ];
  }, [lugha, intiqal, aadaFahsMaktaba]);
  useSajjilAwamir(awamirShasha);

  /**
   * The sentence behind a probe that failed, or null when it did not.
   *
   * Every one of these used to be dropped on the floor. The cost was not a
   * missing error block — it was that something disappeared and there was no
   * way to find out why: the review console's entry left the rail, the status
   * bar shimmered for ever, and the interface offered the user nothing to read.
   * A failure that only removes something must still be answerable.
   */
  const sabab = (fashal: KhataJisr | null): string | null =>
    fashal === null ? null : (fashal.nass(lugha) ?? t('faragh.jisr', lugha));

  const sababJalsa = sabab(jalsa.error);
  const sababMaalumat = sabab(maalumat.error);
  const sababIdadat = sabab(idadat.error);

  // The language has to be known before the first character is painted, or a
  // session set to English opens in Arabic and swaps a frame later. Nothing
  // else gates the screen: the grid draws its own skeleton from `yafhas`, and
  // the status bar draws its own from the absence of its data.
  const yuhammil = idadat.isPending;

  // Only the library query feeds the grid, so only the library query may take
  // it away. This used to be all three, which meant a failed build-info probe —
  // four rows of version text in the status bar — replaced the user's whole
  // library with an error block.
  const khata = maktaba.error;

  const aidIstifsar = (): void => {
    void maalumat.refetch();
    void idadat.refetch();
    void maktaba.refetch();
  };

  const alaFath = (muarrif: string): void => {
    void intiqal({ to: '/luba/$muarrif', params: { muarrif } });
  };

  const iftahIdafa = (): void => {
    setMasarIdafa('');
  };

  /** One rail entry, or nothing when the session genuinely has no such screen. */
  const bandShasha = (shasha: BandTanaqqul): JSX.Element | null => {
    const ism = t(shasha.miftah, lugha);
    const Ramz = shasha.ramz;
    if (shasha.muarrif === 'muraja' && jalsa.data?.malik !== true) {
      // The console's route exists only in an owner session's table, so a
      // session that is not one has nothing to link to and the entry is
      // genuinely absent. A session whose probe *failed* is a third case and
      // not that one: it is not "not an owner", it is "not known yet", and a
      // genuine owner watching the console vanish from the rail concludes it
      // was taken away. So it stays, disabled, carrying the reason it cannot be
      // opened. A `title` on an element that already has text becomes its
      // accessible description, so the reason is announced as well as hovered.
      if (sababJalsa === null) {
        return null;
      }
      return (
        <li key={shasha.muarrif}>
          <span className="band band--matfi" aria-disabled="true" title={sababJalsa}>
            <Ramz className="band__ramz" />
            {ism}
          </span>
        </li>
      );
    }
    if (shasha.masar === null) {
      return (
        <li key={shasha.muarrif}>
          <span className="band band--matfi" aria-disabled="true">
            <Ramz className="band__ramz" />
            {ism}
          </span>
        </li>
      );
    }
    const nashit = shasha.masar === masarHali;
    return (
      <li key={shasha.muarrif}>
        <Link
          to={shasha.masar}
          className={nashit ? 'band band--nashit' : 'band'}
          {...(nashit ? ({ 'aria-current': 'page' } as const) : {})}
        >
          {nashit ? (
            <motion.span
              layoutId="mushir-tanaqqul"
              className="band__mushir"
              transition={haraka(HARAKAT_MASAR)}
            />
          ) : null}
          <Ramz className="band__ramz" />
          {ism}
        </Link>
      </li>
    );
  };

  return (
    <div className="hikal">
      <aside className="janib">
        <div className="janib__tarwisa">
          <span className="janib__ism">{t('tatbiq.ism', lugha)}</span>
        </div>

        <nav className="janib__tanaqqul" aria-label={t('tanaqqul.unwan', lugha)}>
          {MAJMUAT.map((majmua) => {
            const bunud = majmua.shashat
              .map(bandShasha)
              .filter((band): band is JSX.Element => band !== null);
            if (bunud.length === 0) {
              return null;
            }
            return (
              <div className="janib__majmua" key={majmua.muarrif}>
                {majmua.unwan === null ? null : (
                  <h2 className="janib__fasl">{t(majmua.unwan, lugha)}</h2>
                )}
                {majmua.tanbih === null ? null : (
                  <p className="janib__tanbih">{t(majmua.tanbih, lugha)}</p>
                )}
                <ul>{bunud}</ul>
              </div>
            );
          })}
        </nav>
      </aside>

      <main className="mutawa">
        <header className="raas">
          <h1 className="raas__unwan">{t('shasha.maktaba', lugha)}</h1>
          <div
            className="raas__adawat"
            role="group"
            aria-label={t('maktaba.adawat.tasmiya', lugha)}
          >
            <div className="raas__bahth">
              <input
                ref={marjaBahth}
                className="raas__bahth-haql"
                type="search"
                dir="auto"
                placeholder={t('maktaba.bahth.mawdi', lugha)}
                aria-label={t('maktaba.bahth.tasmiya', lugha)}
                value={bahth}
                onChange={(hadath) => {
                  haddidBahth(hadath.target.value);
                }}
              />
              <RamzBahth className="raas__ramz" />
            </div>

            <MurashshihHala
              mukhtara={murashshih.halat}
              lugha={lugha}
              munassiq={munassiq}
              ala_tabdeel={(hala) => {
                baddilMurashshih('halat', hala);
              }}
              ala_imsah={() => {
                amsahMurashshih('halat');
              }}
            />

            {/*
              The order and its direction are one decision shown as two
              controls, so they are drawn as one control with a seam: choosing
              "size" and then wanting "largest first" is a single thought, and
              two separate chips a gap apart do not say that they modify each
              other.
            */}
            <div className="raas__majmua raas__majmua--multasiqa">
              <Mihwar
                tasmiya={t('maktaba.tartib.unwan', lugha)}
                qeema={tartib}
                khiyarat={TARATIB}
                ism={(qeema) => t(`maktaba.tartib.${qeema}` as MiftahLugha, lugha)}
                ala_ikhtiyar={haddidTartib}
              />
              <button
                type="button"
                className="raas__ittijah"
                aria-label={t('maktaba.tartib.aks', lugha)}
                title={t(
                  ittijahFarz === 'tasaudi' ? 'maktaba.tartib.tasaudi' : 'maktaba.tartib.tanazuli',
                  lugha,
                )}
                onClick={aksIttijah}
              >
                {ittijahFarz === 'tasaudi' ? <RamzSahmAala /> : <RamzSahmAsfal />}
              </button>
            </div>

            {/*
              Both lists come from the store's own declarations rather than
              from options written out here, which is what keeps the control and
              the validator honest with each other: a size the validator would
              reject cannot be offered, and a size that is offered cannot fail
              to survive a restart.
            */}
            <div className="raas__majmua">
              <Mihwar
                tasmiya={t('maktaba.kathafa.unwan', lugha)}
                qeema={hajmBitaqa}
                khiyarat={AHJAM_BITAQA}
                ism={(qeema) => t(`maktaba.kathafa.${qeema}`, lugha)}
                ala_ikhtiyar={haddidHajmBitaqa}
              />
              <Mihwar
                tasmiya={t('maktaba.tajmi.unwan', lugha)}
                qeema={tajmi}
                khiyarat={TAJMIAT}
                ism={(qeema) => t(`maktaba.tajmi.${qeema}`, lugha)}
                ala_ikhtiyar={haddidTajmi}
              />
            </div>
          </div>
        </header>

        <section className="jism">
          {/*
            The count belongs to the result and not to the chrome, so it sits
            under the header's rule rather than beside the title: it is the
            first line of the answer, and it changes every time a filter does.
            The line keeps its height while the scan is still running, so the
            first row of artwork does not step down the screen when the number
            arrives.
          */}
          <p className="maktaba__adad">
            {maktaba.data === undefined
              ? null
              : natija.munaqqa
                ? t('maktaba.adad_zahir', lugha, {
                    adad: munassiq.raqm(natija.adad),
                    kulli: munassiq.raqm(natija.adadKulli),
                  })
                : t('maktaba.adad', lugha, { adad: munassiq.raqm(natija.adadKulli) })}
          </p>
          {yuhammil ? (
            <div className="haykal" aria-hidden="true">
              <span className="haykal__satr haykal__satr--tawil" />
              <span className="haykal__satr haykal__satr--mutawassit" />
              <span className="haykal__satr haykal__satr--qasir" />
            </div>
          ) : khata !== null ? (
            <KutlatKhata
              unwan={t('amm.khata', lugha)}
              khata={khata}
              lugha={lugha}
              aada={aidIstifsar}
            />
          ) : (
            <ShabakatMaktaba
              majmuat={natija.majmuat}
              adadKulli={natija.adadKulli}
              munaqqa={natija.munaqqa}
              ghaiba={ghaiba}
              mutaadhira={maktaba.data?.mutaadhira ?? []}
              manassat={asmaManassat(maktaba.data?.manassat ?? [])}
              yafhas={maktaba.isPending}
              kathafa={hajmBitaqa}
              ghiyabMatwi={ghiyabMatwi}
              lugha={lugha}
              munassiq={munassiq}
              ala_fath={alaFath}
              ala_qaima={(muarrif, s, a) => {
                // The one place the physical measurement becomes a logical one.
                setQaimatSiyaq({ muarrif, s: mawqiMantiqi(s), a });
              }}
              ala_amal_jamai={(naw, muarrifat) => {
                if (naw === 'ikhfa') {
                  if (!ikhfa.isPending) {
                    ikhfa.mutate(muarrifat);
                  }
                } else if (!jamai.isPending) {
                  jamai.mutate(muarrifat);
                }
              }}
              ala_tabdeel_ghiyab={baddilGhiyab}
              ala_fath_manassa={(_manassa, muarrif) => {
                iftahMujallad.mutate(muarrif);
              }}
              ala_idafa_yadawiya={iftahIdafa}
              ala_tarshih_mujallad={iftahIdafa}
              ala_imsah_tasfiya={() => {
                haddidBahth('');
                amsahMurashshih();
              }}
            />
          )}

          {qaimatSiyaq !== null ? (
            <div
              className="maktaba__qaima-siyaq"
              role="menu"
              style={{ insetInlineStart: qaimatSiyaq.s, insetBlockStart: qaimatSiyaq.a }}
              onMouseLeave={() => {
                setQaimatSiyaq(null);
              }}
            >
              <button
                type="button"
                role="menuitem"
                className="maktaba__band-qaima"
                onClick={() => {
                  alaFath(qaimatSiyaq.muarrif);
                  setQaimatSiyaq(null);
                }}
              >
                {t('maktaba.qaima.iftah', lugha)}
              </button>
              <button
                type="button"
                role="menuitem"
                className="maktaba__band-qaima"
                onClick={() => {
                  iftahMujallad.mutate(qaimatSiyaq.muarrif);
                  setQaimatSiyaq(null);
                }}
              >
                {t('maktaba.qaima.mujallad', lugha)}
              </button>
              {/*
                The second entry point to the automatic run. The game screen
                carries the other one, and both reach `/tilqai/$muarrif`, so an
                engine this build cannot drive has to be refused in both places
                or the card menu becomes the way around the gate.

                Refused in place rather than omitted: an absent entry is
                indistinguishable from a broken one, and this menu is where a
                reader looks to find out whether the action exists at all.
              */}
              <button
                type="button"
                role="menuitem"
                className="maktaba__band-qaima"
                aria-disabled={!tasil(jahiziyatQaima)}
                onClick={() => {
                  if (!tasil(jahiziyatQaima)) {
                    return;
                  }
                  void intiqal({
                    to: '/tilqai/$muarrif',
                    params: { muarrif: qaimatSiyaq.muarrif },
                  });
                  setQaimatSiyaq(null);
                }}
              >
                {t('tilqai.luba.zirr', lugha)}
              </button>
              {tasil(jahiziyatQaima) ? null : (
                <p className="maktaba__sabab-qaima">{t('luba.tilqai.mamnu', lugha)}</p>
              )}
              <button
                type="button"
                role="menuitem"
                className="maktaba__band-qaima"
                onClick={() => {
                  ikhfa.mutate([qaimatSiyaq.muarrif]);
                  setQaimatSiyaq(null);
                }}
              >
                {t('maktaba.ikhtiyar.ikhfa', lugha)}
              </button>
            </div>
          ) : null}

          {masarIdafa !== null ? (
            <div
              className="maktaba__idafa"
              role="dialog"
              aria-label={t('maktaba.faragh.fahs', lugha)}
            >
              <label className="maktaba__idafa-tasmiya" htmlFor="maktaba-masar-idafa">
                {t('maktaba.idafa.masar', lugha)}
              </label>
              <input
                id="maktaba-masar-idafa"
                className="maktaba__haql mono-ltr"
                dir="ltr"
                value={masarIdafa}
                onChange={(hadath) => {
                  setMasarIdafa(hadath.target.value);
                }}
              />
              <div className="maktaba__idafa-afal">
                <button
                  type="button"
                  className="zir zir--tamyeez"
                  aria-disabled={idafa.isPending || masarIdafa.trim() === ''}
                  onClick={() => {
                    if (!idafa.isPending && masarIdafa.trim() !== '') {
                      idafa.mutate(masarIdafa.trim());
                    }
                  }}
                >
                  {t('maktaba.idafa.thabbit', lugha)}
                </button>
                <button
                  type="button"
                  className="zir"
                  onClick={() => {
                    setMasarIdafa(null);
                  }}
                >
                  {t('luba.taakid.ilgha', lugha)}
                </button>
              </div>
              {idafa.error !== null ? (
                <p className="halat__nass">{idafa.error.nass(lugha) ?? t('faragh.jisr', lugha)}</p>
              ) : null}
            </div>
          ) : null}

          <div aria-live="polite">
            {jamaiHala !== null ? <p className="maktaba__jamai">{jamaiHala}</p> : null}
            {jamai.data !== undefined ? (
              <p className="maktaba__jamai" role="status">
                {jam('maktaba.jamai.tamma', lugha, jamai.data, munassiq)}
              </p>
            ) : null}
          </div>

          {ikhfa.error !== null || iftahMujallad.error !== null || jamai.error !== null ? (
            <div className="maktaba__akhta">
              {ikhfa.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={ikhfa.error}
                  lugha={lugha}
                />
              ) : null}
              {iftahMujallad.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={iftahMujallad.error}
                  lugha={lugha}
                />
              ) : null}
              {jamai.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={jamai.error}
                  lugha={lugha}
                />
              ) : null}
            </div>
          ) : null}

          {/*
            The settings read is the one failure with nowhere visible to land:
            the screen carries on in Arabic with Latin digits, which is what a
            fresh install would have shown anyway, so there is nothing on screen
            that looks wrong. Announced here so that it is at least reachable —
            a user reading in English whose language did not take needs to be
            able to find out that it was the settings and not the interface.
          */}
          <p className="khafi" role="status">
            {yuhammil ? t('amm.tahmil', lugha) : (sababIdadat ?? '')}
          </p>
        </section>

        <footer className="shareet">
          {sababMaalumat !== null ? (
            /*
              A placeholder is a promise that something is coming. This probe is
              not coming back on its own, so leaving the skeleton in place would
              have the status bar shimmering for the rest of the session over a
              failure nobody was told about. The reason takes the row instead,
              truncated like the data path's own long value and readable in full
              on hover.
            */
            <dl className="shareet__qaima">
              <div className="shareet__band shareet__band--masar">
                <dd title={sababMaalumat}>{sababMaalumat}</dd>
              </div>
            </dl>
          ) : maalumat.data === undefined ? (
            <div className="shareet__haykal" aria-hidden="true">
              <span className="haykal__satr haykal__satr--qasir" />
            </div>
          ) : (
            /*
              Five facts at one weight is five things to read and no reason to
              read any of them. Only one of the five is about this session — a
              build that trusts the published development key rather than the
              release key — so that one leads, carries the warning colour and
              its glyph, and states itself in three words with the full sentence
              behind them. The other four are provenance: what is running and
              where it keeps its files. They stay, because nothing else in the
              product displays them yet, but they lose their labels — a status
              bar that names every value it shows says each thing twice — and
              they sit at the far end in the quietest text the scale has.
            */
            <dl className="shareet__qaima">
              {maalumat.data.hawiyat_thiqa === 'tatwir' ? (
                <div className="shareet__band">
                  <dt className="khafi">{t('maalumat.tatwir', lugha)}</dt>
                  <dd className="shareet__wasm-tatwir">
                    <RamzTanbeeh />
                    {t('maktaba.shareet.tatwir', lugha)}
                    <span className="khafi">{t('maalumat.tatwir', lugha)}</span>
                  </dd>
                </div>
              ) : null}
              <div className="shareet__band shareet__band--masar">
                <dt className="khafi">{t('maalumat.bayanat', lugha)}</dt>
                <dd className="mono-ltr" title={maalumat.data.jidhr_bayanat}>
                  {maalumat.data.jidhr_bayanat}
                </dd>
              </div>
              <div className="shareet__band shareet__band--tarif">
                <dt className="khafi">{t('maalumat.isdar', lugha)}</dt>
                <dd className="mono-ltr" title={t('maalumat.isdar', lugha)}>
                  {maalumat.data.isdar}
                </dd>
              </div>
              <div className="shareet__band shareet__band--tarif">
                <dt className="khafi">{t('maalumat.nizam', lugha)}</dt>
                <dd title={t('maalumat.nizam', lugha)}>
                  {t(miftahNizam(maalumat.data.nizam), lugha)}
                </dd>
              </div>
              <div className="shareet__band shareet__band--tarif">
                <dt className="khafi">{t('maalumat.mimariya', lugha)}</dt>
                <dd className="mono-ltr" title={t('maalumat.mimariya', lugha)}>
                  {t(miftahMimariya(maalumat.data.mimariya), lugha)}
                </dd>
              </div>
            </dl>
          )}
        </footer>
      </main>
    </div>
  );
}
