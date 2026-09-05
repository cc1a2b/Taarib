import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { HADATH_TILQAI, nadi } from '@/hayat/jisr';
import { masdarSalih } from '@/maktaba/suwar';

/**
 * عقد التعريب التلقائي — the whole surface between this screen and the run.
 *
 * ## Why this file exists at all
 *
 * The orchestrator (`crates/taarib-tilqai`) and the Tauri layer over it were
 * built by other people at the same time as this screen, so the command names,
 * the event name and the payload field names were not decided when it was
 * written. Two bad answers were available: guess them and rename twenty files
 * later, or leave the screen unbuilt. This is the third — **the screen is
 * written against the interface below and against nothing else**, and every fact
 * about the real backend is confined to the two blocks marked as the adaptation
 * point. The contract has since landed and the names below were the ones taken;
 * only {@link HADATH_TILQAI} moved, to `hayat/jisr.ts` where it belonged.
 *
 * ## Why `invoke` and not `nadi`
 *
 * `hayat/jisr.ts` normalises every rejection into one `KhataJisr`, whose message
 * is a single English string. What the four commands below actually reject with
 * is the backend's own `Khata` — a permanent code and one sentence in *each*
 * language — and {@link fukkKhata} reads that object directly. Going through
 * `nadi` would flatten an Arabic sentence into an English one on the one screen
 * whose whole job is to explain a long, expensive operation. The launch action
 * still goes through `nadi`, because it has no failure worth translating.
 *
 * ## Why everything is decoded rather than trusted
 *
 * Same reason `maktaba/suwar.ts` decodes artwork events: a backend one build
 * ahead can send a shape this build has never seen, and a progress screen that
 * threw on an unfamiliar payload would take down the window in the middle of a
 * run the user cannot restart cheaply. An unrecognised snapshot is ignored and
 * the screen keeps drawing the last one it understood.
 */

/* ==========================================================================
   The adaptation point, part one: the names.
   ========================================================================== */

/**
 * The registered command names, exactly as `apps/studio/src-tauri` spells them.
 *
 * They are collected here rather than written at their call sites so that
 * reconciling them with the backend is one edit to one object. The same four
 * names are listed in `TartibWusata` in `hayat/jisr.ts`, which is what makes a
 * command that disappears from Rust a build error rather than a rejection.
 */
const ASMA_AWAMIR = {
  /** The verdict for one game, computed without running anything. */
  hukm: 'hukm_tilqai',
  /** The unfinished run for one game, or nothing. */
  laqta: 'laqtat_tilqai',
  /** Starts a run, or resumes the unfinished one. */
  ibda: 'ibda_tilqai',
  /** Asks the running job to stop. */
  alghi: 'alghi_tilqai',
} as const;

/**
 * The window event every snapshot arrives on.
 *
 * Now declared in `hayat/jisr.ts` beside the other event names, which is where
 * this file always said it belonged, and re-exported here so that everything
 * this screen needs is still reachable from one import.
 */
export { HADATH_TILQAI };

/* ==========================================================================
   The vocabulary.
   ========================================================================== */

/** The five stages a run passes through, in the order it passes through them. */
export type MarhalatTilqai = 'istikhraj' | 'tarjama' | 'takhtit' | 'tajmee' | 'tathbeet';

/**
 * The stage order, which is also the order the interface draws.
 *
 * Stated here rather than derived from whatever the backend sent, so a payload
 * that omits a stage or reorders them still produces the same five rows in the
 * same places. A progress list that reorders itself mid-run is a progress list
 * nobody can follow.
 */
export const MARAHIL: readonly MarhalatTilqai[] = [
  'istikhraj',
  'tarjama',
  'takhtit',
  'tajmee',
  'tathbeet',
];

const MARAHIL_MAQBULA = new Set<string>(MARAHIL);

/** Where one stage stands. */
export type HalatMarhala = 'muntazira' | 'jariya' | 'tammat' | 'fashilat';

/** Where the run as a whole stands. */
export type WadTilqai =
  /** Running right now. */
  | 'jariya'
  /** Interrupted — the window closed, the machine slept — and resumable. */
  | 'mutawaqqifa'
  /** Installed, verified, and ready to play. */
  | 'jahiz'
  /** Stopped because the user asked. */
  | 'mulgha'
  /** Stopped because something failed. */
  | 'fashal'
  /** Text cannot be read out of the files; the run routes to runtime capture. */
  | 'iltiqat';

const AWDA_MAQBULA = new Set<string>([
  'jariya',
  'mutawaqqifa',
  'jahiz',
  'mulgha',
  'fashal',
  'iltiqat',
]);

/** Which of the three routes a game takes, decided before anything runs. */
export type MasarTilqai =
  /** Taarib can do this end to end. */
  | 'tilqai'
  /** Nothing readable in the files; the game must be played once with capture on. */
  | 'iltiqat'
  /** The safety layer refuses this game outright. */
  | 'marfud';

const MASARAT_MAQBULA = new Set<string>(['tilqai', 'iltiqat', 'marfud']);

/**
 * A failure, in the shape the backend already uses everywhere else.
 *
 * The permanent code and the two sentences, and nothing more: the `khutwa`
 * field on the backend's own `Khata` routes to a screen, and every route this
 * run can offer is already on this screen.
 */
export interface KhataTilqai {
  readonly ramz: string;
  readonly arabi: string;
  readonly injilizi: string;
}

/**
 * How far one stage has got.
 *
 * `majmu` is nullable and the null is load-bearing: it means the stage
 * genuinely does not know how many units it has, and the interface must then
 * say what the stage is doing in words rather than draw a bar against a
 * denominator it invented. Every stage that *can* count — strings extracted,
 * strings translated, glyphs rasterised, files written — reports one.
 */
export interface TaqaddumMarhala {
  readonly marhala: MarhalatTilqai;
  readonly hala: HalatMarhala;
  /** Units finished. */
  readonly tamma: number;
  /** How many units there are, or null when the stage has no denominator. */
  readonly majmu: number | null;
  /** The machine's own detail — a file, a batch, a font — or null for none. */
  readonly tafsil: string | null;
}

/** One thing extraction refused, and why. */
export interface SatrRafd {
  /** What was refused: a file, a container, a class of string. */
  readonly mawdu: string;
  readonly sabab_arabi: string;
  readonly sabab_injilizi: string;
  /** How many strings or files it accounts for, or null when it is one thing. */
  readonly adad: number | null;
}

/** What extraction read, what it skipped, and why it skipped it. */
export interface TaqreerQira {
  /** Containers and files read. */
  readonly maqru: number;
  /** Containers and files skipped. */
  readonly matruk: number;
  /** User-facing strings found. */
  readonly nusus: number;
  readonly asbab: readonly SatrRafd[];
}

/**
 * Money, against the ceiling it may not cross.
 *
 * The ceiling travels with the amount rather than being read from settings,
 * because the number the user agreed to is the number that must be shown back
 * to them — a ceiling changed in settings mid-run would silently redraw the
 * meter the run was authorised against.
 */
export interface Takalif {
  /** Spent so far, or estimated in the verdict. */
  readonly munfaq: number;
  /** The ceiling this run may not cross. */
  readonly saqf: number;
  /** ISO 4217, for `Intl.NumberFormat`. */
  readonly umla: string;
}

/**
 * The verdict: everything the user needs in order to say yes or no, once.
 *
 * This is the only moment there is a decision to take, so it carries the tier,
 * what the result will look like, what it will not do, roughly how much text is
 * involved and what it will cost — and nothing else, because a wall of detail
 * at the one moment a decision is being taken is the same failure as a cheerful
 * lie about it.
 */
export interface HukmTilqai {
  readonly muarrif: string;
  readonly ism: string;
  readonly masar: MasarTilqai;
  /** 1 to 3, matching `Tabaqa` in the generated bindings. */
  readonly tabaqa_raqm: number;
  readonly tabaqa_arabi: string;
  readonly tabaqa_injilizi: string;
  /** What the result will look like, and why this tier and not a better one. */
  readonly sabab_arabi: string;
  readonly sabab_injilizi: string;
  /** Everything that will not work, named specifically. */
  readonly hudud_arabi: readonly string[];
  readonly hudud_injilizi: readonly string[];
  /** Roughly how many strings are involved, or null when only a run can tell. */
  readonly nusus_taqribi: number | null;
  /**
   * Whether the run will ask for the multiplayer acknowledgement.
   *
   * Read off the launcher's own catalogue entry, which the library scan already
   * stored — the only source a verdict may consult, because deciding it properly
   * means walking the game directory and this command is answered on mount. The
   * run decides it again from that walk and refuses at the door when the two
   * disagree, so `false` here is "the launcher did not say so", never "you will
   * not be asked". The screen therefore treats a `TAARIB-E-9129` refusal as this
   * field having been true all along.
   */
  readonly yalzam_iqrar_shabaka: boolean;
  readonly takalif: Takalif;
  /** The cover, as a source this document can load, or null. */
  readonly ghilaf: string | null;
}

/**
 * One run, as it stands at one instant.
 *
 * Every report is a whole snapshot rather than a delta, which is what makes the
 * screen correct after a remount: a subscriber that joined halfway through gets
 * the complete picture on the next report instead of a stream of increments it
 * has no base to apply.
 */
export interface LaqtatTilqai {
  readonly muarrif: string;
  /** This run's own identity, so a late report from a previous run is dropped. */
  readonly tashghila: string;
  readonly wad: WadTilqai;
  /** The stage in progress, or null once the run is terminal. */
  readonly marhala: MarhalatTilqai | null;
  /** Always all five, always in {@link MARAHIL} order. */
  readonly marahil: readonly TaqaddumMarhala[];
  /** What extraction read and refused, or null before extraction reports. */
  readonly qira: TaqreerQira | null;
  readonly takalif: Takalif;
  /** The failure, when the run ended in one. */
  readonly khata: KhataTilqai | null;
  /**
   * Whether anything has been written into the game yet.
   *
   * It is what the cancel affordance is phrased against: before installation,
   * cancelling costs the work; after it, there is nothing left to cancel and
   * the way back is removal on the game screen.
   */
  readonly muthabbata: boolean;
  /** When the run last moved, RFC 3339, for the resume line. */
  readonly waqt: string;
}

/**
 * How the screen reaches a run.
 *
 * An interface rather than five direct calls, for the reason `MinfathSuwar` in
 * `maktaba/suwar.ts` is one: it is the seam a preview harness drives the whole
 * screen through with no backend behind it, which is the only way every state
 * this screen has can actually be looked at.
 */
export interface MinfathTilqai {
  /** The verdict, computed without running anything. */
  readonly hukm: (muarrif: string) => Promise<HukmTilqai>;
  /** The unfinished run for this game, or null when there is none. */
  readonly laqta: (muarrif: string) => Promise<LaqtatTilqai | null>;
  /**
   * Starts a run, or resumes the unfinished one, and answers with a snapshot.
   *
   * `iqrarShabaka` is the user's own answer to the multiplayer warning and
   * nothing else — not a default, not a value derived from the verdict. The run
   * refuses with `TAARIB-E-9129` when it is false and the game turns out to be
   * played with other people, before a single string is sent for translation and
   * before anything is spent.
   */
  readonly ibda: (
    muarrif: string,
    istinaf: boolean,
    iqrarShabaka: boolean,
  ) => Promise<LaqtatTilqai>;
  /** Asks the run to stop, and answers with the snapshot that resulted. */
  readonly alghi: (muarrif: string) => Promise<LaqtatTilqai>;
  /** Every snapshot the backend publishes, until the returned function runs. */
  readonly istami: (ala_wusul: (laqta: LaqtatTilqai) => void) => Promise<() => void>;
  /** Launches the game through its own launcher. */
  readonly shaghghil: (muarrif: string) => Promise<void>;
}

/* ==========================================================================
   The adaptation point, part two: the decoders.
   ========================================================================== */

/** A plain object, as opposed to null, an array or a primitive. */
function kain(khaam: unknown): Readonly<Record<string, unknown>> | null {
  if (typeof khaam !== 'object' || khaam === null || Array.isArray(khaam)) {
    return null;
  }
  return khaam as Readonly<Record<string, unknown>>;
}

function nass(khaam: unknown, badil: string): string {
  return typeof khaam === 'string' ? khaam : badil;
}

function nassAwLaShay(khaam: unknown): string | null {
  return typeof khaam === 'string' && khaam.length > 0 ? khaam : null;
}

/** A finite, non-negative count. Anything else is the caller's neutral value. */
function adad(khaam: unknown, badil: number): number {
  if (typeof khaam !== 'number' || !Number.isFinite(khaam) || khaam < 0) {
    return badil;
  }
  return Math.trunc(khaam);
}

/** The same, keeping the fraction: money is not a count. */
function mablagh(khaam: unknown, badil: number): number {
  if (typeof khaam !== 'number' || !Number.isFinite(khaam) || khaam < 0) {
    return badil;
  }
  return khaam;
}

/** A count that is allowed to be genuinely unknown. */
function adadAwLaShay(khaam: unknown): number | null {
  if (typeof khaam !== 'number' || !Number.isFinite(khaam) || khaam < 0) {
    return null;
  }
  return Math.trunc(khaam);
}

function qaimatNusus(khaam: unknown): readonly string[] {
  if (!Array.isArray(khaam)) {
    return [];
  }
  return khaam.filter((wahid): wahid is string => typeof wahid === 'string');
}

/** Zero cost, in the one currency the product bills in when nothing said. */
const TAKALIF_SIFR: Takalif = { munfaq: 0, saqf: 0, umla: 'USD' };

export function fukkTakalif(khaam: unknown): Takalif {
  const sijill = kain(khaam);
  if (sijill === null) {
    return TAKALIF_SIFR;
  }
  return {
    munfaq: mablagh(sijill['munfaq'], 0),
    saqf: mablagh(sijill['saqf'], 0),
    umla: nass(sijill['umla'], TAKALIF_SIFR.umla),
  };
}

function fukkKhata(khaam: unknown): KhataTilqai | null {
  const sijill = kain(khaam);
  if (sijill === null) {
    return null;
  }
  const ramz = sijill['ramz'];
  const arabi = sijill['arabi'];
  const injilizi = sijill['injilizi'];
  if (typeof ramz !== 'string' || typeof arabi !== 'string' || typeof injilizi !== 'string') {
    return null;
  }
  return { ramz, arabi, injilizi };
}

/**
 * Whatever a rejected call produced, as a failure with a code and two
 * sentences.
 *
 * The backend's own error already carries all three, and a rejection that never
 * reached a command carries neither — so the command's own name stands in for
 * the code, exactly as `KutlatKhata` does for a bridge failure elsewhere.
 */
export function khataMin(amr: string, khaam: unknown): KhataTilqai {
  const mabniyya = fukkKhata(khaam);
  if (mabniyya !== null) {
    return mabniyya;
  }
  const matn = khaam instanceof Error ? khaam.message : String(khaam);
  return { ramz: amr, arabi: matn, injilizi: matn };
}

function fukkRafd(khaam: unknown): SatrRafd | null {
  const sijill = kain(khaam);
  if (sijill === null) {
    return null;
  }
  const mawdu = nassAwLaShay(sijill['mawdu']);
  if (mawdu === null) {
    return null;
  }
  const arabi = nass(sijill['sabab_arabi'], '');
  return {
    mawdu,
    sabab_arabi: arabi,
    sabab_injilizi: nass(sijill['sabab_injilizi'], arabi),
    adad: adadAwLaShay(sijill['adad']),
  };
}

function fukkQira(khaam: unknown): TaqreerQira | null {
  const sijill = kain(khaam);
  if (sijill === null) {
    return null;
  }
  const khaamAsbab = sijill['asbab'];
  const asbab: SatrRafd[] = [];
  if (Array.isArray(khaamAsbab)) {
    for (const wahid of khaamAsbab) {
      const satr = fukkRafd(wahid);
      if (satr !== null) {
        asbab.push(satr);
      }
    }
  }
  return {
    maqru: adad(sijill['maqru'], 0),
    matruk: adad(sijill['matruk'], 0),
    nusus: adad(sijill['nusus'], 0),
    asbab,
  };
}

function fukkHalatMarhala(khaam: unknown): HalatMarhala | null {
  if (
    khaam === 'muntazira' ||
    khaam === 'jariya' ||
    khaam === 'tammat' ||
    khaam === 'fashilat'
  ) {
    return khaam;
  }
  return null;
}

/**
 * A stage's state when the payload did not carry one, derived from position.
 *
 * Derived rather than defaulted to "waiting", because a run that reports only
 * its current stage would otherwise draw four untouched rows and one busy one,
 * losing the fact that everything above the current stage has finished.
 */
function halatMushtaqqa(marhala: MarhalatTilqai, haliya: MarhalatTilqai | null): HalatMarhala {
  if (haliya === null) {
    return 'muntazira';
  }
  const mawqi = MARAHIL.indexOf(marhala);
  const mawqiHali = MARAHIL.indexOf(haliya);
  if (mawqi < mawqiHali) {
    return 'tammat';
  }
  return mawqi === mawqiHali ? 'jariya' : 'muntazira';
}

/** A stage with nothing reported about it yet. */
function marhalaFarigha(marhala: MarhalatTilqai, hala: HalatMarhala): TaqaddumMarhala {
  return { marhala, hala, tamma: 0, majmu: null, tafsil: null };
}

/**
 * The five stages, always all five, always in order.
 *
 * The backend's list is indexed by stage and then read in {@link MARAHIL}'s
 * order, so a payload that sends three stages, or sends them shuffled, still
 * draws the same five rows in the same places.
 */
function fukkMarahil(khaam: unknown, haliya: MarhalatTilqai | null): TaqaddumMarhala[] {
  const wasala = new Map<MarhalatTilqai, Readonly<Record<string, unknown>>>();
  if (Array.isArray(khaam)) {
    for (const wahid of khaam) {
      const sijill = kain(wahid);
      if (sijill === null) {
        continue;
      }
      const ism = sijill['marhala'];
      if (typeof ism === 'string' && MARAHIL_MAQBULA.has(ism)) {
        wasala.set(ism as MarhalatTilqai, sijill);
      }
    }
  }

  return MARAHIL.map((marhala) => {
    const sijill = wasala.get(marhala);
    const mushtaqqa = halatMushtaqqa(marhala, haliya);
    if (sijill === undefined) {
      return marhalaFarigha(marhala, mushtaqqa);
    }
    const majmu = adadAwLaShay(sijill['majmu']);
    const tamma = adad(sijill['tamma'], 0);
    return {
      marhala,
      hala: fukkHalatMarhala(sijill['hala']) ?? mushtaqqa,
      // Clamped rather than trusted: a stage that reports 4 001 of 4 000 would
      // otherwise draw a bar past the end of its own track.
      tamma: majmu === null ? tamma : Math.min(tamma, majmu),
      majmu,
      tafsil: nassAwLaShay(sijill['tafsil']),
    };
  });
}

/**
 * One snapshot off the wire.
 *
 * Returns null for a payload this build does not recognise, and the caller then
 * keeps the last snapshot it did understand — an uninformed screen rather than
 * a broken one.
 */
export function fukkLaqta(khaam: unknown): LaqtatTilqai | null {
  const sijill = kain(khaam);
  if (sijill === null) {
    return null;
  }
  const muarrif = nassAwLaShay(sijill['muarrif']);
  if (muarrif === null) {
    return null;
  }
  const khaamWad = sijill['wad'];
  if (typeof khaamWad !== 'string' || !AWDA_MAQBULA.has(khaamWad)) {
    return null;
  }
  const wad = khaamWad as WadTilqai;

  const khaamMarhala = sijill['marhala'];
  const marhala =
    typeof khaamMarhala === 'string' && MARAHIL_MAQBULA.has(khaamMarhala)
      ? (khaamMarhala as MarhalatTilqai)
      : null;

  return {
    muarrif,
    tashghila: nass(sijill['tashghila'], muarrif),
    wad,
    marhala,
    marahil: fukkMarahil(sijill['marahil'], marhala),
    qira: fukkQira(sijill['qira']),
    takalif: fukkTakalif(sijill['takalif']),
    khata: fukkKhata(sijill['khata']),
    muthabbata: sijill['muthabbata'] === true,
    waqt: nass(sijill['waqt'], ''),
  };
}

/** One verdict off the wire. Null for anything this build cannot read. */
export function fukkHukm(khaam: unknown): HukmTilqai | null {
  const sijill = kain(khaam);
  if (sijill === null) {
    return null;
  }
  const muarrif = nassAwLaShay(sijill['muarrif']);
  if (muarrif === null) {
    return null;
  }
  const khaamMasar = sijill['masar'];
  const masar =
    typeof khaamMasar === 'string' && MASARAT_MAQBULA.has(khaamMasar)
      ? (khaamMasar as MasarTilqai)
      : 'tilqai';

  const sababArabi = nass(sijill['sabab_arabi'], '');
  const tabaqaArabi = nass(sijill['tabaqa_arabi'], '');
  const hududArabi = qaimatNusus(sijill['hudud_arabi']);
  const hududInjilizi = qaimatNusus(sijill['hudud_injilizi']);

  return {
    muarrif,
    ism: nass(sijill['ism'], muarrif),
    masar,
    // Clamped into the three tiers the product actually has, so an unknown
    // number cannot be printed next to a tier name that contradicts it.
    tabaqa_raqm: Math.min(3, Math.max(1, adad(sijill['tabaqa_raqm'], 1))),
    tabaqa_arabi: tabaqaArabi,
    tabaqa_injilizi: nass(sijill['tabaqa_injilizi'], tabaqaArabi),
    sabab_arabi: sababArabi,
    sabab_injilizi: nass(sijill['sabab_injilizi'], sababArabi),
    hudud_arabi: hududArabi,
    // A limit named in one language only is still a limit, so the other
    // language shows the list it has rather than an empty section.
    hudud_injilizi: hududInjilizi.length > 0 ? hududInjilizi : hududArabi,
    nusus_taqribi: adadAwLaShay(sijill['nusus_taqribi']),
    // Strictly `=== true`, like `muthabbata` above: a build one version behind
    // the backend sends no such key at all, and the safe reading of a missing
    // acknowledgement flag is that the question was not asked — which leaves the
    // answer false, which is what the run refuses on rather than proceeds on.
    yalzam_iqrar_shabaka: sijill['yalzam_iqrar_shabaka'] === true,
    takalif: fukkTakalif(sijill['takalif']),
    ghilaf: nassAwLaShay(sijill['ghilaf']),
  };
}

/* ==========================================================================
   The port in force.
   ========================================================================== */

/** Whether this document is running inside a Tauri webview at all. */
function fiTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/** A rejection with a code and two sentences, whatever the call produced. */
async function nadiKhaam(amr: string, wusata: Record<string, unknown>): Promise<unknown> {
  try {
    return await invoke<unknown>(amr, wusata);
  } catch (khaam) {
    throw khataMin(amr, khaam);
  }
}

/** A payload this build could not read, phrased as the failure it is. */
function khataHamula(amr: string): KhataTilqai {
  return {
    ramz: amr,
    arabi: 'ردّت الخدمة بشكل لا تعرفه هذه النسخة من تعريب.',
    injilizi: 'The service answered in a shape this build of Taarib does not know.',
  };
}

/**
 * The port every run uses unless a caller hands the screen another one.
 *
 * `shaghghil` is the one method already wired to a real command:
 * `iftah_manassa` exists today and goes through `nadi`, so it is checked
 * against the generated bindings like every other call in the product.
 */
export const minfathTilqai: MinfathTilqai = {
  hukm: async (muarrif) => {
    const khaam = await nadiKhaam(ASMA_AWAMIR.hukm, { muarrif });
    const mafkuk = fukkHukm(khaam);
    if (mafkuk === null) {
      throw khataHamula(ASMA_AWAMIR.hukm);
    }
    // The cover arrives as an absolute path, and only the shell knows how a
    // document in it loads one. Done here rather than in the decoder so that
    // decoding stays a pure function of its input and a harness can drive the
    // screen with sources it already holds.
    return { ...mafkuk, ghilaf: masdarSalih(mafkuk.ghilaf) };
  },

  laqta: async (muarrif) => {
    const khaam = await nadiKhaam(ASMA_AWAMIR.laqta, { muarrif });
    // Null is an answer here rather than a failure: most games have no
    // unfinished run, and that is the ordinary case.
    return khaam === null || khaam === undefined ? null : fukkLaqta(khaam);
  },

  ibda: async (muarrif, istinaf, iqrarShabaka) => {
    const khaam = await nadiKhaam(ASMA_AWAMIR.ibda, { muarrif, istinaf, iqrarShabaka });
    const mafkuk = fukkLaqta(khaam);
    if (mafkuk === null) {
      throw khataHamula(ASMA_AWAMIR.ibda);
    }
    return mafkuk;
  },

  alghi: async (muarrif) => {
    const khaam = await nadiKhaam(ASMA_AWAMIR.alghi, { muarrif });
    const mafkuk = fukkLaqta(khaam);
    if (mafkuk === null) {
      throw khataHamula(ASMA_AWAMIR.alghi);
    }
    return mafkuk;
  },

  istami: async (ala_wusul) => {
    if (!fiTauri()) {
      return () => undefined;
    }
    try {
      return await listen<unknown>(HADATH_TILQAI, (hadath) => {
        const laqta = fukkLaqta(hadath.payload);
        if (laqta !== null) {
          ala_wusul(laqta);
        }
      });
    } catch {
      // A window that cannot subscribe still shows whatever the snapshot call
      // returned; it just stops learning about the stages after it.
      return () => undefined;
    }
  },

  shaghghil: async (muarrif) => {
    await nadi('iftah_manassa', { muarrif });
  },
};
