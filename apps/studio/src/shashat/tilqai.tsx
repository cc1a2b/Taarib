import { useQuery } from '@tanstack/react-query';
import { Link, getRouteApi } from '@tanstack/react-router';
import { AnimatePresence, motion } from 'motion/react';
import type { CSSProperties, JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { mafatih } from '@/hayat/istifsar';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { ansha } from '@/hayat/tanbihat';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t, wasm } from '@/lugha/lugha';
import { khatarMin, maniAwwal, nassLugha } from '@/maktaba/aql';
import type { JahiziyaTashghil } from '@/maktaba/jahiziya';
import { jahiziyaMin, naqsJahiziya, tasil } from '@/maktaba/jahiziya';
import { IqrarKhatar, muarrifMatlub } from '@/mukawwinat/iqrar_khatar';
import { KutlatFashal, SatrRamz } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import { Zuhur } from '@/mukawwinat/zuhur';
import type { AqlLubaHie, Idadat, Lugha, NizamArqam, TafasilLuba } from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';
import type {
  HalatMarhala,
  HukmTilqai,
  KhataTilqai,
  LaqtatTilqai,
  MarhalatTilqai,
  MinfathTilqai,
  TaqaddumMarhala,
  TaqreerQira,
  WadTilqai,
} from '@/tilqai/aqd';
import { MARAHIL } from '@/tilqai/aqd';
import { useTilqai } from '@/tilqai/halat';

import './tilqai.css';

/**
 * شاشة التعريب التلقائي — one button, and everything the user watches while it
 * runs.
 *
 * The person this screen is for has never translated anything. They own a game
 * with no community patch, they press once, and some minutes later they play it
 * in Arabic. Three things follow from that and they are the whole design.
 *
 * **The verdict is the only decision.** Before anything runs the screen states
 * which tier applies, what the result will look like, what it will not do,
 * roughly how much text is involved and what it will cost against a ceiling it
 * cannot cross. Enough to say yes or no with understanding — not a wall of
 * technical detail, and not a cheerful lie.
 *
 * **Progress is real or it is words.** Every stage reports what it is actually
 * doing and how far through it is. A stage with a denominator draws a bar
 * against that denominator; a stage without one says what it is doing and shows
 * no proportion at all. There is no timer anywhere in this file and no
 * indeterminate spinner standing in for work that can be counted.
 *
 * **The quality is stated wherever the result appears.** This is machine
 * translation with no human review, it says so in the anchor, in the verdict and
 * in the completion panel, and the workshop and the registry are offered from
 * all three. That honesty is the feature.
 *
 * Everything about the backend lives behind {@link MinfathTilqai} in
 * `tilqai/aqd.ts`. This file names no command, no event and no wire field.
 */

const wajihat = getRouteApi('/tilqai/$muarrif');

/** The digit system each setting selects, as `Intl` spells it. */
const ANZIMAT_ARQAM: Readonly<Record<NizamArqam, 'latn' | 'arab' | 'arabext'>> = {
  latini: 'latn',
  arabi: 'arab',
  farisi: 'arabext',
};

const ASMA_MARAHIL: Readonly<Record<MarhalatTilqai, MiftahLugha>> = {
  istikhraj: 'tilqai.marhala.istikhraj',
  tarjama: 'tilqai.marhala.tarjama',
  takhtit: 'tilqai.marhala.takhtit',
  tajmee: 'tilqai.marhala.tajmee',
  tathbeet: 'tilqai.marhala.tathbeet',
};

/** What each stage is actually doing, for the stage that is doing it. */
const AAMAL_MARAHIL: Readonly<Record<MarhalatTilqai, MiftahLugha>> = {
  istikhraj: 'tilqai.amal.istikhraj',
  tarjama: 'tilqai.amal.tarjama',
  takhtit: 'tilqai.amal.takhtit',
  tajmee: 'tilqai.amal.tajmee',
  tathbeet: 'tilqai.amal.tathbeet',
};

/**
 * The element every withdrawn start button points at for its reason.
 *
 * A constant rather than four literals: `aria-describedby` names an id, and an
 * id spelled at five call sites is an id that will be spelled differently at
 * one of them and describe nothing at all.
 */
const MUARRIF_JAHIZIYA = 'tilqai-sabab-jahiziya';

/** The multiplayer acknowledgement, for the ids the component derives. */
const MUARRIF_SHABAKA = 'tilqai-iqrar-shabaka';

/**
 * The standing blocker, which every withdrawn start button points at.
 *
 * One id rather than one per kind, because at most one blocker panel draws:
 * they are all keyed on the first entry of `mawani`, and the core has already
 * decided which that is.
 */
const MUARRIF_MANI = 'tilqai-sabab-mani';

/* ---------------------------------------------------------------------------
   What stops a run, and where the screen learns it.

   **The blockers come from the core.** `aql_luba` holds every producer's answer
   about one game and returns them in one order — `NawMani::rutba` — so this
   screen reads `mawani[0]` and draws that. It applied a priority of its own
   until now: anti-cheat, then readiness, then the multiplayer question, spelled
   out here in a ternary. The game screen applied a different one, and the two
   named different refusals for the same game whenever more than one held. There
   is one order now and it is not in this file.

   Two of the core's own fields decide what the panel does, and neither is
   restated here:

   - `nitaq` says whether a blocker stops everything or only the automatic run.
   - `nihai` says whether it is a fact nothing will change. A blocker that stops
     everything **and** is final withdraws the button, because offering a press
     that will never be accepted is a lie about what the reader can do. One that
     is not final is stated and leaves the button, because the person can go and
     change what it names — set Steam's folder, finish the download, let the
     probe run — and has to be able to come back and press the same button.

   **The codes are what is left.** `ibda` still refuses at its own door, from a
   full walk rather than from a cached answer, and those refusals arrive as a
   rejected call rather than inside a snapshot. Two of the three the door raises
   are now stated before the press instead, so only the one that is a *question*
   is still matched here: **9129**, the multiplayer acknowledgement. The person
   is the only one who can answer it, so it reveals the acknowledgement and lets
   them press again. Matching is on the permanent code because that is the one
   part of a failure that does not move.
   --------------------------------------------------------------------------- */
const RAMZ_SHABAKA = 'TAARIB-E-9129';

const RUMUZ_BAB: ReadonlySet<string> = new Set([RAMZ_SHABAKA]);

const ASMA_HALAT: Readonly<Record<HalatMarhala, MiftahLugha>> = {
  muntazira: 'tilqai.hala.muntazira',
  jariya: 'tilqai.hala.jariya',
  tammat: 'tilqai.hala.tammat',
  fashilat: 'tilqai.hala.fashilat',
};

/**
 * Money, in the user's own digits.
 *
 * Built per currency because the ceiling and the spend are quoted in whatever
 * the provider bills in, and a formatter is not free to construct. An
 * unrecognised code makes `Intl` throw rather than answer, so the fallback
 * prints the number and the code beside it — a figure the user can still read
 * is worth more than a screen that stopped.
 */
function munassiqMablagh(lugha: Lugha, arqam: NizamArqam, umla: string): (qeema: number) => string {
  const mahalli = wasm(lugha);
  try {
    const munassiq = new Intl.NumberFormat(mahalli, {
      style: 'currency',
      currency: umla,
      numberingSystem: ANZIMAT_ARQAM[arqam],
      maximumFractionDigits: 2,
    });
    return (qeema) => munassiq.format(qeema);
  } catch {
    const ihtiyati = new Intl.NumberFormat(mahalli, {
      numberingSystem: ANZIMAT_ARQAM[arqam],
      maximumFractionDigits: 2,
    });
    return (qeema) => `${ihtiyati.format(qeema)} ${umla}`;
  }
}

/** A fraction of a whole, held inside 0..1 and defined when there is no whole. */
function nisbat(tamma: number, majmu: number | null): number {
  if (majmu === null || majmu <= 0) {
    return 0;
  }
  return Math.min(1, Math.max(0, tamma / majmu));
}

/** A width for a meter, as the percentage CSS wants. */
function ardMalu(kasr: number): CSSProperties {
  return { inlineSize: `${String(Math.round(kasr * 1000) / 10)}%` };
}

/* ==========================================================================
   The parts.
   ========================================================================== */

interface KhasaisQiyas {
  readonly tasmiya: string;
  readonly qeema: string;
  /** A word that qualifies the figure — "estimated", "known later" — or null. */
  readonly hamish?: string;
}

/**
 * One labelled figure.
 *
 * The label takes the free space and the value takes what it needs at the
 * trailing edge, so every number in the rail lands on one column and reads down
 * as a column of numbers rather than as five sentences that happen to end in
 * digits.
 */
function Qiyas({ tasmiya, qeema, hamish }: KhasaisQiyas): JSX.Element {
  return (
    <div className="tilqai__qiyas">
      <dt className="tilqai__qiyas-tasmiya">{tasmiya}</dt>
      <dd className="tilqai__qiyas-qeema">
        {qeema}
        {hamish === undefined ? null : <span className="tilqai__qiyas-hamish">{hamish}</span>}
      </dd>
    </div>
  );
}

interface KhasaisMiqyas {
  readonly kasr: number;
  readonly tasmiya: string;
  readonly nass: string;
}

/**
 * The cost meter: spent against the ceiling, and the two figures under it.
 *
 * Drawn only where there is a ceiling to draw against — the caller checks —
 * because a meter over a ceiling of zero is a proportion of nothing, which is
 * the same lie as a bar over an unknown denominator.
 */
function Miqyas({ kasr, tasmiya, nass }: KhasaisMiqyas): JSX.Element {
  return (
    <div className="tilqai__miqyas">
      <div
        className="tilqai__miqyas-masar"
        role="progressbar"
        aria-label={tasmiya}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(kasr * 100)}
      >
        <span className="tilqai__miqyas-malu" style={ardMalu(kasr)} />
      </div>
      <p className="tilqai__miqyas-nass">{nass}</p>
    </div>
  );
}

interface KhasaisSaffMarhala {
  readonly taqaddum: TaqaddumMarhala;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** Whether the run is actually moving right now. */
  readonly hayya: boolean;
  /** The last row draws no connecting rule below its marker. */
  readonly akhira: boolean;
}

/**
 * One stage.
 *
 * The count is the honest part. A stage that knows its denominator prints
 * "1 284 of 3 910" and draws a bar against exactly that; a stage that does not
 * prints the word for what it is doing and draws nothing, because a bar without
 * a denominator is a claim about a proportion nobody has measured.
 *
 * `hayya` is the second half of that honesty. A run the user came back to is
 * stopped on a stage, and that stage's state still reads `jariya` — so the row
 * keeps its count and its bar, which are true, and loses the sentence about
 * what it is doing and the file it is doing it to, which are not.
 */
function SaffMarhala({
  taqaddum,
  lugha,
  munassiq,
  hayya,
  akhira,
}: KhasaisSaffMarhala): JSX.Element {
  const { marhala, hala, tamma, majmu } = taqaddum;
  const jariya = hala === 'jariya';
  const nashita = jariya && hayya;
  const maadud = majmu !== null;
  const kasr = nisbat(tamma, majmu);
  const ism = t(ASMA_MARAHIL[marhala], lugha);

  const tabaqat = [
    'tilqai__marhala',
    `tilqai__marhala--${hala}`,
    akhira ? 'tilqai__marhala--akhira' : '',
  ]
    .filter((wahid) => wahid !== '')
    .join(' ');

  return (
    <li className={tabaqat}>
      <span className="tilqai__alama" aria-hidden="true" />
      <div className="tilqai__marhala-nass">
        <p className="tilqai__marhala-ism">{ism}</p>
        {nashita ? (
          <p className="tilqai__marhala-amal">{t(AAMAL_MARAHIL[marhala], lugha)}</p>
        ) : null}
        {nashita && taqaddum.tafsil !== null ? (
          <p className="tilqai__marhala-tafsil mono-ltr" title={taqaddum.tafsil}>
            {taqaddum.tafsil}
          </p>
        ) : null}
      </div>
      <p className="tilqai__marhala-adad">
        {maadud ? (
          t('tilqai.taqaddum.min', lugha, {
            tamma: munassiq.raqm(tamma),
            majmu: munassiq.raqm(majmu),
          })
        ) : (
          <span className="tilqai__marhala-hala">
            {t(
              nashita
                ? 'tilqai.taqaddum.jari'
                : jariya
                  ? 'tilqai.hala.mutawaqqifa'
                  : ASMA_HALAT[hala],
              lugha,
            )}
          </span>
        )}
      </p>
      {jariya && maadud ? (
        <div
          className="tilqai__shareet-taqaddum"
          role="progressbar"
          aria-label={ism}
          aria-valuemin={0}
          aria-valuemax={majmu}
          aria-valuenow={tamma}
        >
          <span className="tilqai__shareet-malu" style={ardMalu(kasr)} />
        </div>
      ) : null}
    </li>
  );
}

interface KhasaisQira {
  readonly qira: TaqreerQira;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

/** What extraction read, what it refused, and why — never hidden, only folded. */
function LawhatQira({ qira, lugha, munassiq }: KhasaisQira): JSX.Element {
  const [maftuh, haddidFath] = useState(false);
  return (
    <section className="tilqai__qism" aria-labelledby="tilqai-unwan-qira">
      <div className="tilqai__raas-qism">
        <h2 id="tilqai-unwan-qira" className="tilqai__unwan">
          {t('tilqai.qira.unwan', lugha)}
        </h2>
        <button
          type="button"
          className="zir"
          aria-expanded={maftuh}
          aria-controls="tilqai-asbab"
          onClick={() => {
            haddidFath((hali) => !hali);
          }}
        >
          {t(maftuh ? 'tilqai.qira.ikhfa' : 'tilqai.qira.izhar', lugha)}
        </button>
      </div>
      <dl className="tilqai__qiyasat tilqai__qiyasat--satri">
        <Qiyas tasmiya={t('tilqai.qira.maqru', lugha)} qeema={munassiq.raqm(qira.maqru)} />
        <Qiyas tasmiya={t('tilqai.qira.matruk', lugha)} qeema={munassiq.raqm(qira.matruk)} />
        <Qiyas tasmiya={t('tilqai.qira.nusus', lugha)} qeema={munassiq.raqm(qira.nusus)} />
      </dl>
      <AnimatePresence initial={false}>
        {maftuh ? (
          <motion.div
            key="asbab"
            className="tilqai__tawassu"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={haraka(HARAKAT_LAWHA)}
          >
            <div id="tilqai-asbab" className="tilqai__tawassu-dakhil">
              {qira.asbab.length === 0 ? (
                <p className="tilqai__nass">{t('tilqai.qira.la_matruk', lugha)}</p>
              ) : (
                <ul className="tilqai__asbab">
                  {qira.asbab.map((satr, martaba) => (
                    <li key={`${satr.mawdu}:${String(martaba)}`} className="tilqai__sabab">
                      <p className="tilqai__sabab-mawdu mono-ltr" title={satr.mawdu}>
                        {satr.mawdu}
                      </p>
                      <p className="tilqai__sabab-adad">
                        {satr.adad === null ? '' : munassiq.raqm(satr.adad)}
                      </p>
                      <p className="tilqai__sabab-nass">
                        {lugha === 'arabi' ? satr.sabab_arabi : satr.sabab_injilizi}
                      </p>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </section>
  );
}

interface KhasaisJawda {
  readonly lugha: Lugha;
  readonly muarrif: string;
}

/**
 * The quality statement, and the two ways out of it.
 *
 * In the rail rather than in the column, and in the rail in every state: the
 * user is being handed a translation nobody has read, and that fact has to be
 * on screen while they decide, while it runs and once it is installed — not
 * below whatever the column happens to be long enough to push it under. The
 * product's honesty about this is the thing that makes it usable rather than
 * the thing it has to apologise for.
 */
function LawhatJawda({ lugha, muarrif }: KhasaisJawda): JSX.Element {
  return (
    <section className="tilqai__lawha" aria-labelledby="tilqai-unwan-jawda">
      <h2 id="tilqai-unwan-jawda" className="tilqai__unwan-janib">
        {t('tilqai.jawda.unwan', lugha)}
      </h2>
      <p className="tilqai__jawda">{t('tilqai.jawda.jumla', lugha)}</p>
      <p className="tilqai__nass">{t('tilqai.jawda.sharh', lugha)}</p>
      <div className="tilqai__afal">
        <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
          {t('tilqai.jawda.warsha', lugha)}
        </Link>
        <Link to="/taqdeem/$muarrif" params={{ muarrif }} className="zir">
          {t('tilqai.jawda.taqdeem', lugha)}
        </Link>
      </div>
    </section>
  );
}

interface KhasaisKutla {
  readonly unwan: string;
  readonly khata: KhataTilqai;
  readonly lugha: Lugha;
  readonly children?: JSX.Element;
}

/**
 * A failure, in the product's own failure vocabulary.
 *
 * `KutlatKhata` cannot be reused directly: it takes a `KhataJisr`, and a run's
 * failure does not arrive as a rejected command — it arrives inside a snapshot.
 * The card underneath is the same one every other failure is drawn through, so
 * the two cannot drift apart.
 */
function KutlatTilqai({ unwan, khata, lugha, children }: KhasaisKutla): JSX.Element {
  return (
    <KutlatFashal
      unwan={unwan}
      nass={lugha === 'arabi' ? khata.arabi : khata.injilizi}
      ramz={khata.ramz}
      tafsil={khata.injilizi}
      lugha={lugha}
    >
      {children}
    </KutlatFashal>
  );
}

/** The panel-shaped placeholder, sized like the panels that replace it. */
function Haykal(): JSX.Element {
  return (
    <div className="tilqai__haykal zuhur-muakhkhar" aria-hidden="true">
      <div className="tilqai__haykal-mirsa">
        <span className="tilqai__haykal-ghilaf" />
        {/* Two lines under the title, not one: the anchor it stands in for holds
            a heading, the tier and the quality sentence, and a skeleton one line
            short of that is a skeleton the answer jumps past. */}
        <div className="tilqai__haykal-satr-mirsa">
          <span className="tilqai__haykal-unwan" />
          <span className="haykal__satr haykal__satr--mutawassit" />
          <span className="haykal__satr haykal__satr--qasir" />
        </div>
      </div>
      <div className="tilqai__haykal-amida">
        <div className="tilqai__haykal-qism">
          <span className="tilqai__haykal-unwan-qism" />
          <span className="haykal__satr haykal__satr--tawil" />
          <span className="haykal__satr haykal__satr--tawil" />
          <span className="haykal__satr haykal__satr--mutawassit" />
          <span className="haykal__satr haykal__satr--qasir" />
        </div>
        {/* The rail is a hairline and a margin, so its placeholder is too — a
            panel here would promise a box that never arrives. */}
        <div className="tilqai__haykal-lawha">
          <span className="tilqai__haykal-unwan-qism" />
          <span className="haykal__satr haykal__satr--mutawassit" />
          <span className="haykal__satr haykal__satr--qasir" />
        </div>
      </div>
    </div>
  );
}

/* ==========================================================================
   The screen.
   ========================================================================== */

/** Everything the body needs, so that nothing in it reads a global. */
export interface KhasaisShasha {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly arqam: NizamArqam;
  /** The port to drive; the Tauri one unless a harness supplies its own. */
  readonly minfath?: MinfathTilqai;
  /**
   * Whether Taarib can drive this game's engine at all in this build.
   *
   * Not part of the verdict, and deliberately not routed through
   * {@link MinfathTilqai}: the port is the contract of a *run*, and this is a
   * fact about the engine that is true before any run exists and is read by
   * three other surfaces from the same capability report. `null` both while the
   * report is being fetched and when it did not say, because the two want the
   * same behaviour: neither withdraws the button, since refusing on an answer
   * nobody has is refusing a game the backend may well support.
   */
  readonly jahiziya?: JahiziyaTashghil | null;
  /** What is unfinished, in the reader's language, when anything is. */
  readonly naqs?: string | null;
  /**
   * Everything the core knows about this game, or null while it is being
   * fetched and when it could not answer.
   *
   * The blockers, the risks and the promise all come from here, already in the
   * one order, so this screen names the same refusal the game screen names. It
   * is `null` rather than absent for the two cases that behave the same way:
   * the answer has not arrived, or the command failed. Neither withdraws
   * anything on its own — a refusal on an answer nobody has is a refusal of a
   * game the backend may well accept — and `ibda` still refuses at its own door
   * in both.
   */
  readonly aql?: AqlLubaHie | null;
}

/**
 * Which of the seven faces this screen is wearing.
 *
 * Derived once, from the verdict and the snapshot together, rather than tested
 * again at every branch — the states are mutually exclusive and a screen that
 * re-derives that at nine call sites is a screen that will eventually show two
 * of them at once.
 */
type WajhShasha =
  | 'tahmil'
  | 'khata'
  | 'marfud'
  | 'iltiqat'
  | 'hukm'
  | 'jariya'
  | 'mutawaqqifa'
  | 'jahiz'
  | 'mulgha'
  | 'fashal';

function wajhMin(
  yuhammil: boolean,
  khataHukm: KhataTilqai | null,
  hukm: HukmTilqai | null,
  laqta: LaqtatTilqai | null,
): WajhShasha {
  if (laqta !== null) {
    if (laqta.wad === 'jariya') return 'jariya';
    // An interrupted run is its own face, not the verdict again: the verdict
    // has already been taken, and what the user needs is the offer to resume
    // and the list showing how far it got before the window closed.
    if (laqta.wad === 'mutawaqqifa') return 'mutawaqqifa';
    if (laqta.wad === 'jahiz') return 'jahiz';
    if (laqta.wad === 'mulgha') return 'mulgha';
    if (laqta.wad === 'fashal') return 'fashal';
    if (laqta.wad === 'iltiqat') return 'iltiqat';
  }
  if (yuhammil && hukm === null) return 'tahmil';
  if (hukm === null) return khataHukm === null ? 'tahmil' : 'khata';
  if (hukm.masar === 'marfud') return 'marfud';
  if (hukm.masar === 'iltiqat') return 'iltiqat';
  return 'hukm';
}

/** The stage a resumable run stopped on, named, for the resume sentence. */
function ismMarhala(laqta: LaqtatTilqai, lugha: Lugha): string {
  const marhala = laqta.marhala ?? MARAHIL[0];
  return t(ASMA_MARAHIL[marhala ?? 'istikhraj'], lugha);
}

/** The stage list's own header count: which of the five is in progress. */
function raqmMarhala(laqta: LaqtatTilqai): number {
  if (laqta.marhala === null) {
    return MARAHIL.length;
  }
  return MARAHIL.indexOf(laqta.marhala) + 1;
}

export function ShashatTilqai(khasais: KhasaisShasha): JSX.Element {
  const { muarrif, lugha, arqam } = khasais;
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const halat = useTilqai(muarrif, khasais.minfath);
  const { hukm, laqta, khataHukm, khataAmal, yantazir } = halat;

  // Which control is waiting on the backend, so the arc turns on the button
  // that was pressed and not on its neighbour in the same band. Read only while
  // a call is in flight; the value left behind by the last one is masked.
  const [amalJari, setAmalJari] = useState<'ibda' | 'istinaf' | 'alghi' | null>(null);
  const mashghul = yantazir ? amalJari : null;

  const umla = laqta?.takalif.umla ?? hukm?.takalif.umla ?? 'USD';
  const nassMablagh = useMemo(
    () => munassiqMablagh(lugha, arqam, umla),
    [lugha, arqam, umla],
  );

  const wajh = wajhMin(halat.yuhammilHukm, khataHukm, hukm, laqta);
  const yajri = wajh === 'jariya';

  // Whether this build can put anything on screen inside this game at all. It
  // gates every affordance that *starts* work — the first press, the resume,
  // and the two re-runs — rather than only the first, because all four end in
  // the same install and the install is what would change nothing. It is not
  // the verdict's own refusal: `marfud` is the safety layer saying never, and
  // this is Taarib saying not yet.
  const mamnu = !tasil(khasais.jahiziya ?? null);
  const yabda = wajh === 'hukm' || wajh === 'mulgha' || wajh === 'fashal';
  const naqs = khasais.naqs ?? null;

  /* -------------------------------------------------------------------------
     The core's answer, which is what the refusals below are drawn from.

     `mani` is the single most serious blocker, already chosen by the one
     ordering; `khatarShabaka` is the multiplayer question with the scan's own
     wording and whether it has been answered. Neither is re-ranked here.
     ----------------------------------------------------------------------- */
  const aql = khasais.aql ?? null;
  const mani = aql === null ? null : maniAwwal(aql);
  const khatarShabaka = aql === null ? null : khatarMin(aql, 'laab_jamai');

  /* -------------------------------------------------------------------------
     The multiplayer acknowledgement.

     Three sources say the same thing and they are treated as one fact. The
     verdict reads the launcher's own catalogue entry, which is a hint; the core
     runs `kashf_shabaka` over the game's own files, which is the same walk the
     run makes; and `TAARIB-E-9129` coming back from a press is that walk having
     spoken after the fact. Any of them means the question is asked, which is why
     it can appear before a press as well as after one — and, now that the core
     answers before the button, usually before.

     The answer starts false and is only ever the user's. Nothing here defaults
     it, and it is cleared whenever the screen changes game: an acknowledgement
     given about one game is not an acknowledgement about the next.
     ----------------------------------------------------------------------- */
  const [iqrarShabaka, setIqrarShabaka] = useState(false);

  /*
   * That the walk found this game multiplayer, remembered.
   *
   * Latched rather than read live off `khataAmal`, because the next press clears
   * that error before the call goes out — and the walk behind the call is
   * seconds. Read live, the question would vanish the instant it was answered
   * and reappear when the answer came back: the panel the reader is looking at,
   * disappearing under their hand. And the fact does not stop being true because
   * an error object was cleared.
   *
   * Only this one needs a latch, and only for the press that gets as far as the
   * door. A final blocker withdraws every control that could clear its error, so
   * it cannot go stale; and the core's own answer is not read off `khataAmal` at
   * all, so it does not move when an error object is cleared.
   */
  const [kashafaShabaka, setKashafaShabaka] = useState(false);

  useEffect(() => {
    setIqrarShabaka(false);
    setKashafaShabaka(false);
  }, [muarrif]);

  const ramzBab = khataAmal !== null && RUMUZ_BAB.has(khataAmal.ramz) ? khataAmal.ramz : null;
  useEffect(() => {
    if (ramzBab === RAMZ_SHABAKA) {
      setKashafaShabaka(true);
    }
  }, [ramzBab]);

  const yalzamShabaka =
    khatarShabaka !== null || hukm?.yalzam_iqrar_shabaka === true || kashafaShabaka;
  // The core reports this risk as outstanding for every game it fires on, and
  // that is correct rather than a gap being papered over: nothing persists this
  // answer, because an acknowledgement is about one run of one game and one
  // carried over from another would be an answer nobody gave. The tick is the
  // answer and it lives here, in this screen's own state, for the length of the
  // decision it belongs to.
  const mamnuShabaka = yalzamShabaka && !iqrarShabaka;

  /*
   * Whether the standing blocker withdraws the button, from the core's own two
   * fields rather than from a list of codes kept here.
   *
   * `nitaq === 'kul'` is a blocker that stops everything; `nihai` is a blocker
   * nothing will change. Both together mean the press will never be accepted, so
   * offering it is a lie about what the reader can do — anti-cheat, a publisher's
   * own Arabic, an entry that is not a game. A blocker that stops everything and
   * is *not* final is stated and leaves the button, because the reader can go and
   * change what it names and has to be able to come back and press the same
   * button: an unread Steam catalogue, a download still running, a game nobody
   * has probed yet.
   */
  const maniQati = mani !== null && mani.nitaq === 'kul' && mani.nihai;

  /*
   * Which withdrawal a refused start button describes itself with.
   *
   * The order is the core's: a blocker outranks a risk by construction, because
   * a blocker is what no answer opens and a risk is a question the person can
   * answer, and `mawani` is already sorted among themselves. The one thing that
   * is decided here is readiness, which is a `tashghil`-scope blocker about this
   * screen's own run and has always had its own panel.
   *
   * On a game nothing can be done to, the multiplayer question is moot, and a
   * risk decision demanded for a run that cannot happen is how a person learns
   * to tick without reading.
   */
  const sababTawaqquf = maniQati
    ? MUARRIF_MANI
    : mamnu
      ? MUARRIF_JAHIZIYA
      : mamnuShabaka
        ? muarrifMatlub(MUARRIF_SHABAKA)
        : undefined;

  /** Whether any of the three gates is holding the start. */
  const mamnuBadi = mamnu || maniQati || mamnuShabaka;

  /**
   * The one place a run is started from.
   *
   * Every affordance that starts work goes through this — the first press, the
   * resume, both re-runs, and the command palette's own entry — so the gates are
   * stated once and cannot be true of a button and false of a keystroke. The
   * acknowledgement is read here rather than passed in, because what has to
   * reach the backend is the answer as it stands at the moment of the press.
   */
  const ibdaMahmi = (istinaf: boolean): void => {
    if (yantazir || mamnuBadi) {
      return;
    }
    setAmalJari(istinaf ? 'istinaf' : 'ibda');
    halat.ibda(istinaf, iqrarShabaka);
  };

  /** The one place a run is stopped from: the anchor's button and the palette's entry. */
  const alghiMahmi = (): void => {
    if (yantazir) {
      return;
    }
    setAmalJari('alghi');
    halat.alghi();
  };

  // The palette registers once per id set, so its actions reach the current
  // closures through a ref rather than through the registration.
  const afal = useRef({ ibda: ibdaMahmi, alghi: alghiMahmi });
  useEffect(() => {
    afal.current = { ibda: ibdaMahmi, alghi: alghiMahmi };
  });

  const awamir = useMemo<readonly AmrLawha[]>(() => {
    const majal = t('shasha.tilqai', lugha);
    return [
      {
        muarrif: `tilqai.${muarrif}.ibda`,
        unwan: t('tilqai.lawha.ibda', lugha),
        majal,
        nafidh: () => {
          afal.current.ibda(false);
        },
      },
      {
        muarrif: `tilqai.${muarrif}.alghi`,
        unwan: t('tilqai.lawha.alghi', lugha),
        majal,
        nafidh: () => {
          afal.current.alghi();
        },
      },
    ];
  }, [lugha, muarrif]);
  useSajjilAwamir(awamir);

  const ism = hukm?.ism ?? muarrif;
  const takalif = laqta?.takalif ?? hukm?.takalif ?? null;
  const kasrKulfa =
    takalif === null || takalif.saqf <= 0 ? 0 : nisbat(takalif.munfaq, takalif.saqf);

  /*
   * The run's end, said where the reader is looking.
   *
   * The completion panel stands in the column and the anchor's button changes,
   * but a person who pressed once and went on to read the reading report, or
   * scrolled the rail, is not looking there — and the notice stack floats over
   * the whole window. Every snapshot passes through here; only the one that
   * carries the same run from moving to stopped says anything, so a screen that
   * mounts onto a run already over, or a resumed run's first report, is silent.
   * An interruption is not an end and gets the resume band instead.
   */
  const wadSabiq = useRef<{ readonly tashghila: string; readonly wad: WadTilqai } | null>(null);
  useEffect(() => {
    const sabiq = wadSabiq.current;
    wadSabiq.current = laqta === null ? null : { tashghila: laqta.tashghila, wad: laqta.wad };
    if (
      laqta === null ||
      sabiq === null ||
      sabiq.tashghila !== laqta.tashghila ||
      sabiq.wad !== 'jariya' ||
      laqta.wad === 'jariya' ||
      laqta.wad === 'mutawaqqifa'
    ) {
      return;
    }
    switch (laqta.wad) {
      case 'jahiz':
        ansha({
          naw: 'najah',
          nass: t('tilqai.tanbih.jahiz', lugha, { ism }),
          tafsil: t('tilqai.jawda.jumla', lugha),
        });
        return;
      case 'mulgha':
        ansha({
          naw: 'najah',
          nass: t('tilqai.tanbih.mulgha', lugha, { ism }),
          tafsil: t('tilqai.tanbih.munfaq', lugha, { kulfa: nassMablagh(laqta.takalif.munfaq) }),
        });
        return;
      case 'fashal':
        ansha({
          naw: 'khatar',
          nass: t('tilqai.tanbih.fashal', lugha, { ism }),
          tafsil:
            laqta.khata === null ? null : lugha === 'arabi' ? laqta.khata.arabi : laqta.khata.injilizi,
          ramz: laqta.khata?.ramz ?? null,
        });
        return;
      case 'iltiqat':
        ansha({ naw: 'tanbeeh', nass: t('tilqai.tanbih.iltiqat', lugha, { ism }) });
        return;
    }
  }, [laqta, ism, lugha, nassMablagh]);

  /** The reason a refused start button gives under the pointer, matching what `aria-describedby` points at. */
  const sababNass =
    mani !== null && maniQati
      ? nassLugha(mani, lugha)
      : mamnu
        ? mani !== null && mani.naw === 'jahiziya_ghaiba'
          ? nassLugha(mani, lugha)
          : (naqs ?? t('tilqai.jahiziya.sharh', lugha))
        : mamnuShabaka
          ? t('tilqai.shabaka.matlub', lugha)
          : undefined;

  // Which single action the anchor offers. One place, in every state, so the
  // user never has to look for the button they pressed a minute ago.
  const zirRaisi = ((): JSX.Element | null => {
    if (wajh === 'hukm') {
      return (
        <button
          type="button"
          className="zir zir--tamyeez"
          aria-busy={yantazir}
          aria-disabled={mamnuBadi}
          aria-describedby={sababTawaqquf}
          title={sababNass}
          onClick={() => {
            ibdaMahmi(false);
          }}
        >
          {t(yantazir ? 'tilqai.hukm.jari_ibda' : 'tilqai.hukm.ibda', lugha)}
        </button>
      );
    }
    if (wajh === 'jariya') {
      return laqta !== null && laqta.muthabbata ? null : (
        <button
          type="button"
          className="zir zir--khatar"
          aria-busy={yantazir}
          onClick={alghiMahmi}
        >
          {t(yantazir ? 'tilqai.ilgha.jari' : 'tilqai.ilgha.zirr', lugha)}
        </button>
      );
    }
    if (wajh === 'jahiz') {
      return (
        <button
          type="button"
          className="zir zir--tamyeez"
          onClick={() => {
            halat.shaghghil();
          }}
        >
          {t('tilqai.jahiz.shaghghil', lugha)}
        </button>
      );
    }
    if (wajh === 'iltiqat') {
      return (
        <Link to="/tabaqa/$muarrif" params={{ muarrif }} className="zir zir--tamyeez">
          {t('tilqai.iltiqat.tabaqa', lugha)}
        </Link>
      );
    }
    if (wajh === 'mulgha' || wajh === 'fashal') {
      return (
        <button
          type="button"
          className="zir zir--tamyeez"
          aria-busy={yantazir}
          aria-disabled={mamnuBadi}
          aria-describedby={sababTawaqquf}
          title={sababNass}
          onClick={() => {
            ibdaMahmi(wajh === 'fashal');
          }}
        >
          {t(wajh === 'fashal' ? 'tilqai.istinaf.zirr' : 'tilqai.mulgha.iaada', lugha)}
        </button>
      );
    }
    return null;
  })();

  const marahil = laqta?.marahil ?? null;
  const mustanifa = wajh === 'mutawaqqifa';

  return (
    <div className="tilqai">
      <RaasShasha
        rujoo={{ ila: 'luba', muarrif }}
        nassRujoo={t('tilqai.raji', lugha)}
        unwan={t('shasha.tilqai', lugha)}
        mawdu={hukm?.ism ?? null}
        rawabit={
          <>
            <Link to="/" className="raas-shasha__rabt">
              {t('shasha.maktaba', lugha)}
            </Link>
            <Link to="/warsha/$muarrif" params={{ muarrif }} className="raas-shasha__rabt">
              {t('shasha.warsha', lugha)}
            </Link>
          </>
        }
      />

      <div className="tilqai__jism">
        <Mashhad
          miftah={wajh === 'tahmil' ? 'tahmil' : wajh === 'khata' ? 'khata' : 'jahiz'}
          className="tilqai__mashhad"
        >
        {wajh === 'tahmil' ? (
          <Haykal />
        ) : (
          <>
            <header className="tilqai__mirsa">
              {hukm?.ghilaf == null ? (
                <span className="tilqai__ghilaf tilqai__ghilaf--faragh" aria-hidden="true" />
              ) : (
                <img
                  className="tilqai__ghilaf"
                  src={hukm.ghilaf}
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                />
              )}
              <div className="tilqai__mirsa-nass">
                {/* Only while it moves. The top strip already says which screen
                    this is, and an eyebrow that repeats it is a line of text
                    that never changes above a line that does. */}
                <Zuhur maftuh={yajri}>
                  <p className="tilqai__fawq">{t('tilqai.jari.unwan', lugha)}</p>
                </Zuhur>
                <h2 className="tilqai__ism">{ism}</h2>
                {hukm === null ? null : (
                  <p className="tilqai__tabaqa">
                    {t('tilqai.hukm.tabaqa', lugha, {
                      raqm: munassiq.raqm(hukm.tabaqa_raqm),
                      ism: lugha === 'arabi' ? hukm.tabaqa_arabi : hukm.tabaqa_injilizi,
                    })}
                  </p>
                )}
                <p className="tilqai__jawda-satr">{t('tilqai.jawda.jumla', lugha)}</p>
              </div>
              {/* One slot, keyed on the face: the button the reader pressed a
                  minute ago is replaced where it stood rather than swapped
                  between two frames. */}
              <Mashhad miftah={wajh} className="tilqai__mirsa-afal">
                {zirRaisi}
              </Mashhad>
            </header>

            {/*
              Why the button above is off. Placed between the anchor and
              everything else because it governs everything else: the resume
              offer under it, the verdict panel below it and the cost figures in
              the rail are all descriptions of a run this build cannot deliver,
              and a reader who met them first would have read three screens of
              detail before learning the press does nothing.

              Amber and not red. The refusal panel this screen already has —
              `marfud` — is the safety layer saying never, in red, on a game the
              user must not touch. This is Taarib saying not yet, about itself,
              and an update lifts it.
            */}
            {/*
              The question, above the button it governs.

              It sits here rather than down in the verdict column for the reason
              the readiness notice below it does: the primary action is in the
              anchor directly above, and an acknowledgement a person reaches
              after scrolling past the button is an acknowledgement they meet
              once they have already pressed it. The `matlub` line inside carries
              the id every start button on this screen points at, so a refused
              button is not merely grey — it says which of the two things is
              holding it, in the reader's own language.

              Hidden while readiness withdraws the run, because the backend
              refuses in that order too: on a game this build cannot patch, the
              multiplayer question has no run to be about, and a risk decision
              demanded for nothing is how a person learns to tick without
              reading.
            */}
            <Zuhur
              maftuh={yalzamShabaka && !mamnu && !maniQati && (yabda || mustanifa)}
              className="tilqai__iqrar-shabaka"
            >
                {/* Revealing a panel is not an announcement. A press that was
                    refused has to say so to a reader who cannot see the panel
                    appear, and it has to say it in the run's own terms: nothing
                    started, nothing was spent. */}
                {ramzBab === RAMZ_SHABAKA ? (
                  <p className="khafi" role="status">
                    {t('tilqai.shabaka.marfud', lugha)}
                  </p>
                ) : null}
                <IqrarKhatar
                  muarrif={MUARRIF_SHABAKA}
                  unwan={t('tilqai.shabaka.unwan', lugha)}
                  tahdheer={t('tilqai.shabaka.tahdheer', lugha)}
                  // What the walk actually found, in the scan's own words. The
                  // core runs that walk before the button, so this is now here
                  // to be read while the decision is being taken rather than
                  // only after a press was refused — and the refused press is
                  // still the fallback for a game the core could not answer for.
                  tafsil={
                    khatarShabaka !== null
                      ? nassLugha(khatarShabaka, lugha)
                      : ramzBab === RAMZ_SHABAKA && khataAmal !== null
                        ? lugha === 'arabi'
                          ? khataAmal.arabi
                          : khataAmal.injilizi
                        : null
                  }
                  nassIqrar={t('tilqai.shabaka.iqrar', lugha)}
                  muqirr={iqrarShabaka}
                  alaTabdil={setIqrarShabaka}
                  matlub={t('tilqai.shabaka.matlub', lugha)}
                />
            </Zuhur>

            <Zuhur maftuh={mamnu && (yabda || mustanifa)}>
              <section
                className="tilqai__band tilqai__band--tanbeeh"
                aria-labelledby="tilqai-unwan-jahiziya"
              >
                <h2 id="tilqai-unwan-jahiziya" className="tilqai__band-unwan">
                  {t('tilqai.jahiziya.unwan', lugha)}
                </h2>
                {/* The capability report's own gap sentence, whichever way it
                    arrived: the core carries it on the `jahiziya_ghaiba` blocker
                    and the report carries it directly, and both are the same
                    `naqs`. The locale fallback under them covers a report stored
                    by a build that could not name its own gap — the one case
                    neither source has a sentence for. */}
                <p id={MUARRIF_JAHIZIYA} className="tilqai__nass" dir="auto">
                  {mani !== null && mani.naw === 'jahiziya_ghaiba'
                    ? nassLugha(mani, lugha)
                    : (naqs ?? t('tilqai.jahiziya.sharh', lugha))}
                </p>
                <p className="tilqai__nass">{t('tilqai.jahiziya.khutwa', lugha)}</p>
                <div className="tilqai__afal">
                  <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
                    {t('tilqai.jawda.warsha', lugha)}
                  </Link>
                </div>
              </section>
            </Zuhur>

            <Zuhur maftuh={mustanifa && laqta !== null}>
              {laqta === null ? null : (
              <section className="tilqai__band tilqai__band--tanbeeh">
                <h2 className="tilqai__band-unwan">{t('tilqai.istinaf.unwan', lugha)}</h2>
                <p className="tilqai__nass">
                  {t('tilqai.istinaf.sharh', lugha, { marhala: ismMarhala(laqta, lugha) })}
                </p>
                <div className="tilqai__afal">
                  <button
                    type="button"
                    className="zir zir--tamyeez"
                    aria-busy={mashghul === 'istinaf'}
                    aria-disabled={mamnuBadi || (yantazir && mashghul !== 'istinaf')}
                    aria-describedby={sababTawaqquf}
                    title={sababNass}
                    onClick={() => {
                      ibdaMahmi(true);
                    }}
                  >
                    {t('tilqai.istinaf.zirr', lugha)}
                  </button>
                  <button
                    type="button"
                    className="zir"
                    aria-busy={mashghul === 'ibda'}
                    aria-disabled={mamnuBadi || (yantazir && mashghul !== 'ibda')}
                    aria-describedby={sababTawaqquf}
                    title={sababNass}
                    onClick={() => {
                      ibdaMahmi(false);
                    }}
                  >
                    {t('tilqai.istinaf.jadeed', lugha)}
                  </button>
                </div>
              </section>
              )}
            </Zuhur>

            <div
              className={
                wajh === 'khata' ? 'tilqai__amida tilqai__amida--mufrad' : 'tilqai__amida'
              }
            >
              <div className="tilqai__raisi">
                {/* Shown in every face, not only its own: a verdict that failed
                    while an interrupted run is still resumable is a fact the
                    user is owed alongside the offer to resume it. */}
                <Zuhur maftuh={khataHukm !== null} asl="mahall">
                  {khataHukm === null ? null : (
                  <KutlatTilqai
                    unwan={t('tilqai.khata.hukm', lugha)}
                    khata={khataHukm}
                    lugha={lugha}
                  >
                    <button type="button" className="zir" onClick={halat.aidHukm}>
                      {t('amm.iaada', lugha)}
                    </button>
                  </KutlatTilqai>
                  )}
                </Zuhur>

                {/* Every failure except the one the door raises that is a
                    question. That one has a panel of its own above, because the
                    generic block says "something failed" about a decision the
                    person has not been asked for yet. */}
                <Zuhur maftuh={khataAmal !== null && ramzBab === null} asl="mahall">
                  {khataAmal === null ? null : (
                  <KutlatTilqai
                    unwan={t('tilqai.khata.amal', lugha)}
                    khata={khataAmal}
                    lugha={lugha}
                  />
                  )}
                </Zuhur>

                {/* The standing blocker, in the producer's own words.
                    `mawani[0]` and nothing else: the sentence is the safety
                    layer's for anti-cheat, the store adapter's for a catalogue
                    that would not open, discovery's for an entry that is not a
                    game, and the probe's own for a game nobody has looked at.
                    None of them is written here or in the string set.

                    Two of the core's fields shape the panel and neither is a
                    second opinion about the game. `nihai` decides the colour and
                    whether a button is offered at all — a final refusal gets the
                    refusal vocabulary and no control, because the product has no
                    override for anti-cheat or for a publisher's own Arabic
                    anywhere. A blocker that is not final is amber and keeps the
                    button, because the reader can go and change what it names.
                    `naw` decides only which one action to offer beside it.

                    Hidden while the run is actually moving, which is the gate
                    the verdict's own refusal panel below has always had: a
                    blocker that appeared after a run started — an anti-cheat
                    signature that landed in an update — is worth showing the
                    moment the run stops, and is noise across a progress list. */}
                <Zuhur maftuh={mani !== null && mani.nitaq === 'kul' && !yajri} asl="mahall">
                  {mani === null ? null : (
                  <section
                    className={
                      mani.nihai
                        ? 'tilqai__qism tilqai__qism--khatar'
                        : 'tilqai__qism tilqai__qism--tanbeeh'
                    }
                    role={mani.nihai ? 'alert' : undefined}
                    aria-labelledby="tilqai-unwan-mani"
                  >
                    <h2 id="tilqai-unwan-mani" className="tilqai__unwan">
                      {t(
                        mani.naw === 'himaya' || mani.naw === 'fahs_himaya_lam_yajri'
                          ? 'tilqai.himaya.unwan'
                          : 'tilqai.marfud.unwan',
                        lugha,
                      )}
                    </h2>
                    {/* `tilqai__daleel` rather than `tilqai__nass`: the safety
                        layer writes its refusal as a statement followed by the
                        evidence it rests on, one finding per line, and that
                        class is the one on this screen that keeps the breaks. */}
                    <p id={MUARRIF_MANI} className="tilqai__daleel" dir="auto">
                      {nassLugha(mani, lugha)}
                    </p>
                    {/* Only under a final refusal. That sentence says the run
                        does not start on a game the safety layer refuses and
                        that the workshop is still open, which is the right thing
                        to leave somebody with when nothing will change — and the
                        wrong thing when something will: every blocker that is not
                        final ends with its own next step, in the producer's own
                        words, and the button beside it goes there. */}
                    {mani.nihai ? (
                      <p className="tilqai__nass">{t('tilqai.marfud.khutwa', lugha)}</p>
                    ) : null}
                    <div className="tilqai__afal">
                      {mani.naw === 'fahs_himaya_lam_yajri' ? (
                        <Link to="/idadat" className="zir">
                          {t('tilqai.himaya.idadat', lugha)}
                        </Link>
                      ) : (
                        <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
                          {t('tilqai.jawda.warsha', lugha)}
                        </Link>
                      )}
                    </div>
                  </section>
                  )}
                </Zuhur>

                {/* The verdict's own refusal, for a game the core could not
                    answer for. It keeps the sentence `hukm_tilqai` computed,
                    which is the same producer text by another route. */}
                <Zuhur maftuh={mani === null && wajh === 'marfud' && hukm !== null} asl="mahall">
                  {hukm === null ? null : (
                  <section className="tilqai__qism tilqai__qism--khatar">
                    <h2 className="tilqai__unwan">{t('tilqai.marfud.unwan', lugha)}</h2>
                    <p className="tilqai__nass">
                      {lugha === 'arabi' ? hukm.sabab_arabi : hukm.sabab_injilizi}
                    </p>
                    <p className="tilqai__nass">{t('tilqai.marfud.khutwa', lugha)}</p>
                    <div className="tilqai__afal">
                      <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
                        {t('tilqai.jawda.warsha', lugha)}
                      </Link>
                    </div>
                  </section>
                  )}
                </Zuhur>

                <Zuhur maftuh={wajh === 'iltiqat'} asl="mahall">
                  <section className="tilqai__qism tilqai__qism--tanbeeh">
                    <h2 className="tilqai__unwan">{t('tilqai.iltiqat.unwan', lugha)}</h2>
                    <p className="tilqai__nass">
                      {hukm !== null && hukm.sabab_arabi !== ''
                        ? lugha === 'arabi'
                          ? hukm.sabab_arabi
                          : hukm.sabab_injilizi
                        : t('tilqai.iltiqat.sharh', lugha)}
                    </p>
                    <p className="tilqai__khutwa">{t('tilqai.iltiqat.khutwa', lugha)}</p>
                    <div className="tilqai__afal">
                      <button
                        type="button"
                        className="zir"
                        onClick={() => {
                          halat.shaghghil();
                        }}
                      >
                        {t('tilqai.jahiz.shaghghil', lugha)}
                      </button>
                    </div>
                  </section>
                </Zuhur>

                <Zuhur maftuh={wajh === 'jahiz'} asl="mahall">
                  <section className="tilqai__qism tilqai__qism--najah">
                    <h2 className="tilqai__unwan">{t('tilqai.jahiz.unwan', lugha)}</h2>
                    <p className="tilqai__nass">{t('tilqai.jahiz.sharh', lugha)}</p>
                    <p className="tilqai__jawda">{t('tilqai.jawda.jumla', lugha)}</p>
                    <div className="tilqai__afal">
                      <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
                        {t('tilqai.jawda.warsha', lugha)}
                      </Link>
                      <Link to="/taqdeem/$muarrif" params={{ muarrif }} className="zir">
                        {t('tilqai.jawda.taqdeem', lugha)}
                      </Link>
                    </div>
                  </section>
                </Zuhur>

                <Zuhur maftuh={wajh === 'mulgha' && takalif !== null} asl="mahall">
                  {takalif === null ? null : (
                  <section className="tilqai__qism">
                    <h2 className="tilqai__unwan">{t('tilqai.mulgha.unwan', lugha)}</h2>
                    <p className="tilqai__nass">
                      {t('tilqai.mulgha.sharh', lugha, { kulfa: nassMablagh(takalif.munfaq) })}
                    </p>
                  </section>
                  )}
                </Zuhur>

                <Zuhur maftuh={wajh === 'fashal'} asl="mahall">
                  <section className="tilqai__qism tilqai__qism--khatar">
                    <h2 className="tilqai__unwan">{t('tilqai.fashal.unwan', lugha)}</h2>
                    {laqta?.khata == null ? null : (
                      <p className="tilqai__nass">
                        {lugha === 'arabi' ? laqta.khata.arabi : laqta.khata.injilizi}
                      </p>
                    )}
                    <p className="tilqai__nass">{t('tilqai.fashal.sharh', lugha)}</p>
                    {laqta?.khata == null ? null : (
                      <SatrRamz
                        ramz={laqta.khata.ramz}
                        lugha={lugha}
                        tafsil={laqta.khata.injilizi}
                      />
                    )}
                  </section>
                </Zuhur>

                <Zuhur maftuh={wajh === 'hukm' && hukm !== null} asl="mahall">
                  {hukm === null ? null : (
                  <section className="tilqai__qism" aria-labelledby="tilqai-unwan-hukm">
                    <h2 id="tilqai-unwan-hukm" className="tilqai__unwan">
                      {t('tilqai.hukm.unwan', lugha)}
                    </h2>
                    <p className="tilqai__natija">
                      {lugha === 'arabi' ? hukm.sabab_arabi : hukm.sabab_injilizi}
                    </p>
                    <p className="tilqai__nass">
                      {hukm.nusus_taqribi === null
                        ? t('tilqai.hukm.hajm_majhul', lugha)
                        : t('tilqai.hukm.hajm', lugha, {
                            nusus: jam('tilqai.nusus', lugha, hukm.nusus_taqribi, munassiq),
                          })}
                    </p>
                    {/* A ceiling of zero is not a ceiling. `hukm` reports the
                        spend limit as 0 for two configurations — no provider is
                        elected, and an elected provider with no budget set — and
                        in both of them no run starts, so the unconditional
                        sentence promised a ceiling that does not exist and
                        implied the run was free. Which of the two it is, and
                        what to do about it, is stated in `hudud` immediately
                        below; this line only stops claiming the guarantee. */}
                    <p className="tilqai__nass">
                      {hukm.takalif.saqf > 0
                        ? t('tilqai.hukm.kulfa', lugha, {
                            munfaq: nassMablagh(hukm.takalif.munfaq),
                            saqf: nassMablagh(hukm.takalif.saqf),
                          })
                        : t('tilqai.hukm.kulfa_bila_saqf', lugha, {
                            munfaq: nassMablagh(hukm.takalif.munfaq),
                          })}
                    </p>
                    <h3 className="tilqai__unwan-farii">{t('tilqai.hukm.hudud', lugha)}</h3>
                    {(lugha === 'arabi' ? hukm.hudud_arabi : hukm.hudud_injilizi).length === 0 ? (
                      <p className="tilqai__nass">{t('tilqai.hukm.la_hudud', lugha)}</p>
                    ) : (
                      <ul className="tilqai__hudud">
                        {(lugha === 'arabi' ? hukm.hudud_arabi : hukm.hudud_injilizi).map(
                          (hadd, martaba) => (
                            <li key={`${hadd}:${String(martaba)}`}>{hadd}</li>
                          ),
                        )}
                      </ul>
                    )}
                    <p className="tilqai__qarar">{t('tilqai.hukm.qarar', lugha)}</p>
                  </section>
                  )}
                </Zuhur>

                <Zuhur maftuh={marahil !== null && wajh !== 'marfud' && wajh !== 'hukm'}>
                  {marahil === null ? null : (
                  <section className="tilqai__qism" aria-labelledby="tilqai-unwan-marahil">
                    <div className="tilqai__raas-qism">
                      <h2 id="tilqai-unwan-marahil" className="tilqai__unwan">
                        {t('tilqai.marahil.unwan', lugha)}
                      </h2>
                      {laqta === null ? null : (
                        <p className="tilqai__adad-marahil">
                          {t('tilqai.marhala.min', lugha, {
                            raqm: munassiq.raqm(raqmMarhala(laqta)),
                            kulli: munassiq.raqm(MARAHIL.length),
                          })}
                        </p>
                      )}
                    </div>
                    <ol
                      className={
                        yajri ? 'tilqai__marahil' : 'tilqai__marahil tilqai__marahil--waqifa'
                      }
                    >
                      {marahil.map((taqaddum, martaba) => (
                        <SaffMarhala
                          key={taqaddum.marhala}
                          taqaddum={taqaddum}
                          lugha={lugha}
                          munassiq={munassiq}
                          hayya={yajri}
                          akhira={martaba === marahil.length - 1}
                        />
                      ))}
                    </ol>
                    <Zuhur maftuh={yajri && laqta !== null} className="tilqai__ilgha">
                      {laqta === null ? null : (
                      <>
                        <p className="tilqai__ilgha-nass">
                          {t(laqta.muthabbata ? 'tilqai.ilgha.baad' : 'tilqai.ilgha.qabl', lugha)}
                        </p>
                        {laqta.muthabbata ? (
                          <div className="tilqai__afal">
                            <Link to="/luba/$muarrif" params={{ muarrif }} className="zir">
                              {t('tilqai.ilgha.luba', lugha)}
                            </Link>
                          </div>
                        ) : null}
                      </>
                      )}
                    </Zuhur>
                  </section>
                  )}
                </Zuhur>

                <Zuhur maftuh={laqta?.qira != null}>
                  {laqta?.qira == null ? null : (
                    <LawhatQira qira={laqta.qira} lugha={lugha} munassiq={munassiq} />
                  )}
                </Zuhur>
              </div>

              {/* Absent on the one face with nothing to put in it: a verdict
                  that could not be computed has no figures to state and no
                  result to qualify, and a rail of empty headings beside an
                  error is the product talking past the user. */}
              {wajh === 'khata' ? null : (
                <aside className="tilqai__janib">
                  <section className="tilqai__lawha" aria-labelledby="tilqai-unwan-arqam">
                    <h2 id="tilqai-unwan-arqam" className="tilqai__unwan-janib">
                      {t('tilqai.arqam.unwan', lugha)}
                    </h2>
                    <dl className="tilqai__qiyasat">
                      {hukm === null ? null : (
                        <Qiyas
                          tasmiya={t('tilqai.arqam.tabaqa', lugha)}
                          qeema={munassiq.raqm(hukm.tabaqa_raqm)}
                          hamish={lugha === 'arabi' ? hukm.tabaqa_arabi : hukm.tabaqa_injilizi}
                        />
                      )}
                      <Qiyas
                        tasmiya={t('tilqai.arqam.nusus', lugha)}
                        qeema={
                          laqta?.qira != null
                            ? munassiq.raqm(laqta.qira.nusus)
                            : hukm?.nusus_taqribi != null
                              ? munassiq.raqm(hukm.nusus_taqribi)
                              : '—'
                        }
                        {...(laqta?.qira == null
                          ? {
                              hamish: t(
                                hukm?.nusus_taqribi == null
                                  ? 'tilqai.arqam.majhul'
                                  : 'tilqai.arqam.taqribi',
                                lugha,
                              ),
                            }
                          : {})}
                      />
                      {takalif === null ? null : (
                        <>
                          <Qiyas
                            tasmiya={t('tilqai.arqam.kulfa', lugha)}
                            qeema={nassMablagh(takalif.munfaq)}
                          />
                          <Qiyas
                            tasmiya={t('tilqai.arqam.saqf', lugha)}
                            qeema={nassMablagh(takalif.saqf)}
                          />
                        </>
                      )}
                    </dl>
                    {takalif === null || takalif.saqf <= 0 ? null : (
                      <Miqyas
                        kasr={kasrKulfa}
                        tasmiya={t('tilqai.arqam.kulfa', lugha)}
                        nass={t('tilqai.arqam.min_saqf', lugha, {
                          munfaq: nassMablagh(takalif.munfaq),
                          saqf: nassMablagh(takalif.saqf),
                        })}
                      />
                    )}
                  </section>

                  <LawhatJawda lugha={lugha} muarrif={muarrif} />
                </aside>
              )}
            </div>
          </>
        )}
        </Mashhad>

        <p className="khafi" role="status">
          {yajri && laqta?.marhala != null
            ? ((): string => {
                const jariya = marahil?.find((wahid) => wahid.marhala === laqta.marhala) ?? null;
                const ismHali = t(ASMA_MARAHIL[laqta.marhala], lugha);
                if (jariya === null || jariya.majmu === null) {
                  return t('tilqai.taqaddum.itlaq_bila_adad', lugha, { marhala: ismHali });
                }
                return t('tilqai.taqaddum.itlaq', lugha, {
                  marhala: ismHali,
                  tamma: munassiq.raqm(jariya.tamma),
                  majmu: munassiq.raqm(jariya.majmu),
                });
              })()
            : ''}
        </p>
      </div>
    </div>
  );
}

/**
 * The route's component: the address, the session's display settings, the
 * engine's runtime readiness, and nothing else. Everything that draws is in
 * {@link ShashatTilqai}, which takes its whole world as props so a harness can
 * render every state it has.
 *
 * Readiness is fetched here rather than inside the body, and from the game's own
 * capability report rather than through {@link MinfathTilqai}, for one reason:
 * it is not a fact about a run. It is the same verdict the library card and the
 * game screen draw, it is answered by the same command under the same cache key
 * as the game screen's, so arriving here from that screen costs nothing, and
 * having one source for it is what stops this screen from disagreeing with the
 * one the user pressed the button on.
 *
 * The core's answer is fetched here for the same reason and one more. This is
 * the screen with a button that spends money and writes into somebody's game,
 * so the anti-cheat walk, the multiplayer walk and the proxy survey are paid for
 * *before* the button rather than at the door after a press — which is the
 * difference between a refusal a reader meets while deciding and one they meet
 * after pressing. It is under the same key the game screen uses, so the two
 * cannot be looking at two different answers about one game.
 */
export function Tilqai(): JSX.Element {
  const { muarrif } = wajihat.useParams();
  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const tafsil = useQuery<TafasilLuba, KhataJisr>({
    queryKey: mafatih.tafasil(muarrif),
    queryFn: () => nadi('tafasil_luba', { muarrif }),
  });
  const aql = useQuery<AqlLubaHie, KhataJisr>({
    queryKey: mafatih.aql(muarrif),
    queryFn: () => nadi('aql_luba', { muarrif }),
  });

  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const taqreer = tafsil.data?.taqreer;

  return (
    <ShashatTilqai
      muarrif={muarrif}
      lugha={lugha}
      arqam={idadat.data?.arqam ?? 'latini'}
      // Null until the report lands, which leaves the button offered: a start
      // withdrawn on an answer nobody has yet is a start withdrawn from a game
      // this build may well support.
      jahiziya={taqreer === undefined ? null : jahiziyaMin(taqreer.jahiziya)}
      naqs={taqreer === undefined ? null : naqsJahiziya(taqreer, lugha)}
      // The same rule, for the same reason: a scan that has not answered yet and
      // a scan that failed both leave the verdict's own refusal in charge.
      aql={aql.data ?? null}
    />
  );
}
