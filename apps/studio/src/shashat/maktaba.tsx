import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { listen } from '@tauri-apps/api/event';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { HADATH_FAHS_MUHARRIK, HADATH_JAWLA_MUHARRIK, KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { ansha } from '@/hayat/tanbihat';
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
import { RamzSahmAala, RamzSahmAsfal, ramz } from '@/mukawwinat/rumuz';
import { ShabakatMaktaba } from '@/mukawwinat/shabakat_maktaba';
import { Zuhur } from '@/mukawwinat/zuhur';
import type {
  FahsMuharrikHie,
  HalatJawlaHie,
  HalatLuba,
  HasilatMaktaba,
  Idadat,
  Lugha,
  Manassa,
  NizamArqam,
  SijillGhaib,
  SijillMaktaba,
} from '@/mustalahat/awamir';

import './maktaba.css';

/**
 * المكتبة — the Library: the grid of every game on this machine, and the row
 * of controls that decides which of them are on screen and in what order. The
 * rail, the window strip and the status strip around it belong to the shell
 * in `mukawwinat/hikal.tsx`, which every screen shares.
 */

/* ==========================================================================
   The two glyphs this screen needs and the family in `rumuz.tsx` does not
   carry. Built through that file's own `ramz` contract rather than as bare
   SVG, so the 24×24 grid, the single stroke weight, the round terminals and
   the decorative default cannot drift away from the glyphs beside them.
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
    // Whether the three fields below it are answers or defaults. The scan fills
    // a row for a game whose report was never written exactly as it fills one
    // for a game examined and not recognised — same engine, same tier, same
    // readiness — so this is the only thing that separates "we looked and found
    // nothing", which is final, from "nothing has been looked at", which is not.
    mafhusa: saf.mafhusa,
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
    tarjamat_mujtama: saf.tarjamat_mujtama,
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
      <Zuhur maftuh={maftuh} className="raas__lawha" role="group" aria-label={unwan}>
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
      </Zuhur>
    </div>
  );
}

/**
 * What one bulk install did: what it installed, and what it deliberately did not.
 *
 * The second number exists because this path used to install everything by
 * answering the risk questions itself. It read `MudkhalRuqaaHie::yahtaj_iqrar`,
 * which is the registry's verdict on the **build match** and nothing else, and
 * sent that one value as the user's answer to that question *and* to the
 * multiplayer one — so a patch that matched only approximately silently carried
 * an acknowledgement of an anti-cheat risk nobody had been shown.
 *
 * A game selected in a grid is not a place either question can be asked. There
 * is no room to say which patch, against which build, at what risk, and a prompt
 * per game across a selection of forty is a prompt nobody reads. So the games
 * that need an answer are left for the screen that can ask for one properly, and
 * how many were left is reported rather than quietly folded into the total.
 */
interface HasilatJamai {
  /** Games a patch was actually written into. */
  readonly muthabbat: number;
  /** Games whose only installable patch needs an answer this screen cannot take. */
  readonly matruk: number;
}

export function Maktaba(): JSX.Element {
  const makhzanIstifsar = useQueryClient();
  const intiqal = useNavigate();

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

  /** Where the background engine sweep stands, or null until the backend has said. */
  const [jawla, setJawla] = useState<HalatJawlaHie | null>(null);

  /**
   * The session's language and formatters, as the sweep's listeners read them.
   *
   * The listeners below are registered once and live as long as the screen, so
   * they cannot close over `lugha` without going stale the moment the setting
   * changes — a notice raised half an hour into a session would come out in the
   * language the screen opened in. Kept current from an effect, never written
   * during render.
   */
  const marjaSiyaq = useRef({ lugha, munassiq });
  useEffect(() => {
    marjaSiyaq.current = { lugha, munassiq };
  }, [lugha, munassiq]);

  // The library probes itself after every scan, on the backend's own task, and
  // that task is usually already running by the time the grid has drawn: the
  // scan's answer is what starts it. So the standing is read once on mount, for
  // a screen that arrives mid-sweep, and every change after that arrives on two
  // events. A finished game patches its own row in the cache in place — the row
  // keeps its shape, nothing else in the answer is touched, and no rescan is
  // asked for — so one card's badges redraw and no card's box changes.
  useEffect(() => {
    let hayy = true;
    void nadi('halat_jawla').then(
      (hala) => {
        if (hayy) {
          setJawla(hala);
        }
      },
      (khata: unknown) => {
        const { lugha: lughaHaliya } = marjaSiyaq.current;
        ansha({
          naw: 'tanbeeh',
          nass: t('maktaba.jawla.khata', lughaHaliya),
          tafsil: khata instanceof KhataJisr ? khata.nass(lughaHaliya) : null,
        });
      },
    );
    const ilghaFahs = listen<FahsMuharrikHie>(HADATH_FAHS_MUHARRIK, (hadath) => {
      const fahs = hadath.payload;
      makhzanIstifsar.setQueryData<HasilatMaktaba>(mafatih.maktaba, (qadeem) =>
        qadeem === undefined
          ? qadeem
          : {
              ...qadeem,
              alaab: qadeem.alaab.map((saf) =>
                saf.muarrif === fahs.muarrif
                  ? {
                      ...saf,
                      mafhusa: fahs.mafhusa,
                      muharrik: fahs.muharrik,
                      tabaqa: fahs.tabaqa,
                      jahiziya: fahs.jahiziya,
                      hala: fahs.hala,
                    }
                  : saf,
              ),
            },
      );
    });
    const ilghaJawla = listen<HalatJawlaHie>(HADATH_JAWLA_MUHARRIK, (hadath) => {
      const hala = hadath.payload;
      setJawla(hala);
      if (hala.jariya) {
        return;
      }
      // The sweep is over. Its failures are the backend's to log and this
      // screen's to say: a probe that was refused leaves a card reading "not
      // probed yet" for a reason, and the reason has to reach the person.
      const { lugha: lughaHaliya, munassiq: munassiqHali } = marjaSiyaq.current;
      if (hala.khata !== null) {
        ansha({
          naw: 'tanbeeh',
          nass: t('maktaba.jawla.khata', lughaHaliya),
          tafsil: lughaHaliya === 'arabi' ? hala.khata.arabi : hala.khata.injilizi,
        });
      }
      if (hala.fashila > 0) {
        const awwal = hala.akhta[0];
        ansha({
          naw: 'tanbeeh',
          nass: jam('maktaba.jawla.fashila', lughaHaliya, hala.fashila, munassiqHali),
          tafsil:
            awwal === undefined
              ? null
              : `${awwal.ism}: ${lughaHaliya === 'arabi' ? awwal.khata.arabi : awwal.khata.injilizi}`,
        });
      }
    });
    return () => {
      hayy = false;
      void ilghaFahs.then((f) => {
        f();
      });
      void ilghaJawla.then((f) => {
        f();
      });
    };
  }, [makhzanIstifsar]);

  // A menu opened from the keyboard has to close from it: the pointer leaving
  // is the only other way out, and a keyboard has no pointer to leave with.
  useEffect(() => {
    if (qaimatSiyaq === null) {
      return undefined;
    }
    const alaMiftah = (hadath: KeyboardEvent): void => {
      if (hadath.key === 'Escape') {
        setQaimatSiyaq(null);
      }
    };
    document.addEventListener('keydown', alaMiftah);
    return () => {
      document.removeEventListener('keydown', alaMiftah);
    };
  }, [qaimatSiyaq]);

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
  const ikhfa = useMutation<
    readonly string[],
    KhataJisr,
    readonly string[],
    { sabiqa: HasilatMaktaba | undefined }
  >({
    // The cards leave the grid at the press, not at the answer: a hide is the
    // user's own decision about their own library, and the grid the backend
    // sends back is the same grid minus the same cards. A refused write puts
    // the grid back exactly as it stood.
    onMutate: async (muarrifat) => {
      await makhzanIstifsar.cancelQueries({ queryKey: mafatih.maktaba });
      const sabiqa = makhzanIstifsar.getQueryData<HasilatMaktaba>(mafatih.maktaba);
      makhzanIstifsar.setQueryData<HasilatMaktaba>(mafatih.maktaba, (qadeem) =>
        qadeem === undefined
          ? qadeem
          : { ...qadeem, alaab: qadeem.alaab.filter((saf) => !muarrifat.includes(saf.muarrif)) },
      );
      return { sabiqa };
    },
    onError: (_khata, _muarrifat, siyaq) => {
      if (siyaq?.sabiqa !== undefined) {
        makhzanIstifsar.setQueryData(mafatih.maktaba, siyaq.sabiqa);
      }
    },
    // The cards are already gone from the grid, so the confirmation is the
    // only thing on screen that says what happened — and it carries the way
    // back, which pops the step `mutationFn` registered a moment ago.
    onSuccess: (mukhfat) => {
      if (mukhfat.length === 0) {
        return;
      }
      ansha({
        naw: 'najah',
        nass: jam('maktaba.ikhfa.tamma', lugha, mukhfat.length, munassiq),
        amal: {
          unwan: t('taraju.zirr', lugha),
          nafidh: () => {
            void useTaraju.getState().taraju();
          },
        },
      });
    },
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
  // The selection is the denominator: the bar draws against the games chosen,
  // and moves as each one is either patched or left for its own screen.
  const [taqaddumJamai, setTaqaddumJamai] = useState<{ tamma: number; majmu: number } | null>(
    null,
  );

  const jamai = useMutation<HasilatJamai, KhataJisr, readonly string[]>({
    mutationFn: async (muarrifat) => {
      let muthabbat = 0;
      let matruk = 0;
      let jari = 0;
      setTaqaddumJamai({ tamma: 0, majmu: muarrifat.length });
      for (const muarrif of muarrifat) {
        jari += 1;
        setTaqaddumJamai({ tamma: jari - 1, majmu: muarrifat.length });
        setJamaiHala(t('maktaba.jamai.jari', lugha, { adad: munassiq.raqm(jari) }));
        // Sequential on purpose: two installs writing one store would race.
        // eslint-disable-next-line no-await-in-loop
        const ruqaa = await nadi('ruqaa_luba', { muarrif });
        const mutah = ruqaa.mudkhalat.filter((mudkhal) => mudkhal.qabila_lil_tathbeet);
        const awwal = mutah.find((mudkhal) => !mudkhal.yahtaj_iqrar);
        if (awwal === undefined) {
          if (mutah.length > 0) {
            matruk += 1;
          }
          continue;
        }
        // eslint-disable-next-line no-await-in-loop
        const hasila = await nadi('nazzil_ruqaa', { muarrif, ruqaa: String(awwal.id) });
        // Both withheld, and withheld is the correct value rather than a
        // cautious one: neither question has been put to anybody here, so there
        // is no answer to send. The install gate refuses if either turns out to
        // matter, which is the game's own screen's cue to ask.
        // eslint-disable-next-line no-await-in-loop
        await nadi('thabbit_ruqaa', {
          muarrif,
          masarMalaf: hasila.masar,
          iqrarShabaka: false,
          iqrarTaqribi: false,
        });
        muthabbat += 1;
      }
      setTaqaddumJamai({ tamma: muarrifat.length, majmu: muarrifat.length });
      return { muthabbat, matruk };
    },
    // Said in a notice rather than under the grid, because the person who
    // selected forty games has usually scrolled away from where the bar was
    // by the time the fortieth finishes. The games left for their own screens
    // are the second line: a selection of ten that installs eight is
    // otherwise indistinguishable from one that installed all of them.
    onSuccess: (hasila) => {
      ansha({
        naw: 'najah',
        nass: jam('maktaba.jamai.tamma', lugha, hasila.muthabbat, munassiq),
        tafsil:
          hasila.matruk > 0
            ? t('maktaba.jamai.matruk', lugha, { adad: munassiq.raqm(hasila.matruk) })
            : null,
      });
    },
    onSettled: () => {
      setJamaiHala(null);
      setTaqaddumJamai(null);
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
    },
  });

  const idafa = useMutation<number, KhataJisr, string>({
    mutationFn: (masar) => nadi('adif_mujallad_fahs', { masar }),
    onSuccess: () => {
      setMasarIdafa(null);
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.maktaba });
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.fahs_akhir });
      void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.idadat });
    },
  });

  /** Stable per observer, so the palette's rescan closure never goes stale. */
  const aadaFahsMaktaba = maktaba.refetch;

  /** The library's palette entries: the rescan and the search field. The screens are the shell's. */
  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    const majal = t('shasha.maktaba', lugha);
    return [
      {
        muarrif: 'maktaba.aada_fahs',
        unwan: t('maktaba.lawha.aada_fahs', lugha),
        majal,
        nafidh: () => {
          void aadaFahsMaktaba();
          // The diagnostics screen's record of the last scan is about to be
          // out of date, and it is not this screen's to redraw.
          void makhzanIstifsar.invalidateQueries({ queryKey: mafatih.fahs_akhir });
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
  }, [lugha, aadaFahsMaktaba, makhzanIstifsar]);
  useSajjilAwamir(awamirShasha);

  /**
   * The sentence behind the settings read failing, or null when it did not.
   *
   * It used to be dropped on the floor, and the cost was that the screen
   * carried on in the defaults with nothing on it that looked wrong. A failure
   * that only removes something must still be answerable.
   */
  const sababIdadat =
    idadat.error === null ? null : (idadat.error.nass(lugha) ?? t('faragh.jisr', lugha));

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
    void idadat.refetch();
    void maktaba.refetch();
  };

  const alaFath = (muarrif: string): void => {
    void intiqal({ to: '/luba/$muarrif', params: { muarrif } });
  };

  const iftahIdafa = (): void => {
    setMasarIdafa('');
  };

  // The menu's place, as a style, only while it is open: the wrapper keeps
  // the last style it was given for the frames the leaving menu takes.
  const uslubQaima =
    qaimatSiyaq === null
      ? undefined
      : { insetInlineStart: qaimatSiyaq.s, insetBlockStart: qaimatSiyaq.a };

  return (
    <div className="maktaba">
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
            {yuhammil || maktaba.data === undefined
              ? null
              : natija.munaqqa
                ? t('maktaba.adad_zahir', lugha, {
                    adad: munassiq.raqm(natija.adad),
                    kulli: munassiq.raqm(natija.adadKulli),
                  })
                : t('maktaba.adad', lugha, { adad: munassiq.raqm(natija.adadKulli) })}
            {/*
              The sweep's own line, beside the count and only while it runs:
              the grid is filling in which product each game gets, and this is
              the one place that says so. It leaves with the sweep, so a library
              whose every game is probed reads exactly as it always did. A
              refused game counts as examined here — it was looked at — and is
              reported by name when the sweep ends.
            */}
            {jawla !== null && jawla.jariya && jawla.majmu > 0 ? (
              <span className="maktaba__jawla">
                {t('maktaba.jawla.jariya', lugha, {
                  tamma: munassiq.raqm(jawla.tamma + jawla.fashila),
                  majmu: munassiq.raqm(jawla.majmu),
                })}
              </span>
            ) : null}
          </p>
          {khata !== null ? (
            <div className="jism__mutadahrij">
              <KutlatKhata
                unwan={t('amm.khata', lugha)}
                khata={khata}
                lugha={lugha}
                aada={aidIstifsar}
              />
            </div>
          ) : (
            /* While the settings are still unread the grid draws its own
               placeholder — rows of card-shaped boxes on the grid's own
               arithmetic — rather than a second, unmeasured skeleton: the two
               used to disagree by a whole row, and the first real artwork
               stepped down the screen the moment the scan landed. Handing the
               grid no rows keeps the language gate intact, since a placeholder
               has no text to paint in the wrong language. */
            <ShabakatMaktaba
              majmuat={yuhammil ? [] : natija.majmuat}
              adadKulli={natija.adadKulli}
              munaqqa={natija.munaqqa}
              ghaiba={ghaiba}
              mutaadhira={maktaba.data?.mutaadhira ?? []}
              manassat={asmaManassat(maktaba.data?.manassat ?? [])}
              yafhas={yuhammil || maktaba.isPending}
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

          <Zuhur
            maftuh={qaimatSiyaq !== null}
            className="maktaba__qaima-siyaq"
            role="menu"
            {...(uslubQaima === undefined ? {} : { style: uslubQaima })}
            onMouseLeave={() => {
              setQaimatSiyaq(null);
            }}
          >
            {qaimatSiyaq === null ? null : (
            <>
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
            </>
            )}
          </Zuhur>

          <Zuhur
            maftuh={masarIdafa !== null}
            asl="taht"
            className="maktaba__idafa"
            role="dialog"
            aria-label={t('maktaba.faragh.fahs', lugha)}
          >
            {masarIdafa === null ? null : (
            <>
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
            </>
            )}
          </Zuhur>

          <div aria-live="polite">
            {jamaiHala !== null ? <p className="maktaba__jamai">{jamaiHala}</p> : null}
            {jamai.isPending && taqaddumJamai !== null && taqaddumJamai.majmu > 0 ? (
              <div className="maktaba__miqyas">
                <div
                  className="maktaba__miqyas-masar"
                  role="progressbar"
                  aria-label={t('maktaba.ikhtiyar.tathbeet', lugha)}
                  aria-valuemin={0}
                  aria-valuemax={taqaddumJamai.majmu}
                  aria-valuenow={taqaddumJamai.tamma}
                >
                  <span
                    className="maktaba__miqyas-malu"
                    style={{
                      inlineSize: `${String((taqaddumJamai.tamma / taqaddumJamai.majmu) * 100)}%`,
                    }}
                  />
                </div>
                <p className="maktaba__miqyas-nass">
                  {t('amm.taqaddum.min', lugha, {
                    tamma: munassiq.raqm(taqaddumJamai.tamma),
                    majmu: munassiq.raqm(taqaddumJamai.majmu),
                  })}
                </p>
              </div>
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
    </div>
  );
}
