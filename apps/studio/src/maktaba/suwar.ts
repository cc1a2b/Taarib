import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useRef, useState } from 'react';

import { HADATH_GHILAF_LUBA, nadi } from '@/hayat/jisr';
import type { GhilafHie } from '@/mustalahat/awamir';

/**
 * صور المكتبة — the cover art the library grid shows as it resolves.
 *
 * ## What the backend gives, and what that leaves for this module
 *
 * `hassil_suwar_maktaba` takes no arguments and walks the **whole** library,
 * emitting one {@link GhilafHie} per game on {@link HADATH_GHILAF_LUBA} as each
 * one settles — including for a game that resolved nothing, which is what lets
 * a card stop waiting instead of waiting forever. A cold pass over seventeen
 * games takes about 3.3 seconds and a warm one about 0.8, so artwork genuinely
 * arrives over seconds, one game at a time, onto rows that are already mounted.
 *
 * That shape settles two questions and leaves one.
 *
 * - **Which games** is not a question this module can ask. The command has no
 *   per-game argument, so the visible rows cannot be resolved ahead of the rest
 *   and the arrival order is the backend's walk order, not the user's scroll
 *   position. The report on this work says what a per-game argument would buy.
 * - **What to draw** is answered by the stream and by the library record
 *   together. A game whose record already carries a cover — the warm case, and
 *   most of a second visit — paints on the first frame with no event at all,
 *   and nothing here may overwrite it.
 * - **When to ask** is what is left, and it is what this module owns. The pass
 *   starts once the grid actually has rows without artwork, not on mount and
 *   not on every scroll frame, and it is not started again while one is
 *   running, nor while the games on screen are all accounted for.
 *
 * ## Three bounds, and what each one is for
 *
 * - **Settled once.** A game the stream has spoken about is settled, whether it
 *   came with artwork or with three nulls. A settled game never causes another
 *   pass. This is not an optimisation: the visible set is recomputed on every
 *   scroll frame, so without it every frame would consider asking again.
 * - **A trailing delay.** The decision is taken when the visible set holds
 *   still, not while it is moving, so the grid's own measurement churn on mount
 *   — the probe, the resize observer, the font set — resolves into one decision
 *   rather than three. The delay is capped by {@link AQSA_INTIZAR} so that a
 *   slow continuous drag, which never leaves a gap long enough to settle, still
 *   reaches a decision.
 * - **A cooldown.** A second pass is allowed only {@link MUHLAT_IAADA} after
 *   the last one ended, and only if something on screen is still unaccounted
 *   for. That is what turns "a rescan added games" into one more pass and a
 *   backend that silently drops a game into one retry rather than a pass per
 *   scroll.
 *
 * ## Nothing here can move the layout
 *
 * Every source this module produces ends up as the `src` of an image the card
 * has already positioned absolutely inside a well of a fixed size, over a plate
 * that is already occupying the same rectangle. See the governing rule in
 * `nizam/tokens.css`. The grid's own measurement of that claim is in the report.
 */

/* ==========================================================================
   Sources.
   ========================================================================== */

/**
 * A scheme, as opposed to a Windows volume letter.
 *
 * `C:\Games\...` and `https://...` both read as `<letters>:`, and the only
 * thing separating them is that a scheme is never one character long. Getting
 * this wrong in either direction is a cover that never appears: a bare path
 * handed to an `<img>`, or an `asset://` URL wrapped a second time.
 */
const NAMAT_MUKHATTAT = /^[a-z][a-z0-9+.-]+:/iu;

/** How many conversions are remembered before the oldest is dropped. */
const SAAT_TAHWIL = 4096;

/**
 * Conversions already performed, keyed by the raw value.
 *
 * The grid re-derives every visible card's source on every scroll frame, and
 * `convertFileSrc` percent-encodes a whole path each time it is called. Forty
 * cards at sixty frames is what this exists for.
 */
const TAHWILAT = new Map<string, string | null>();

/**
 * An absolute path from the backend, as something this document can load.
 *
 * Idempotent on purpose. Two different values reach it — the path on the
 * library record and the path on an artwork event — and either may already have
 * been converted by a caller upstream. A second conversion would percent-encode
 * the first one's URL and produce an image that cannot load.
 *
 * @param khaam an absolute path, an already-converted URL, or null
 */
export function masdarSalih(khaam: string | null): string | null {
  if (khaam === null) {
    return null;
  }
  const nass = khaam.trim();
  if (nass.length === 0) {
    return null;
  }
  const mukhazzan = TAHWILAT.get(nass);
  if (mukhazzan !== undefined) {
    return mukhazzan;
  }

  let natija: string | null;
  if (NAMAT_MUKHATTAT.test(nass)) {
    natija = nass;
  } else {
    try {
      natija = convertFileSrc(nass);
    } catch {
      // Outside a Tauri webview there is no asset protocol to convert against,
      // and a raw path in an `img` is a broken image rather than a missing one.
      natija = null;
    }
  }

  TAHWILAT.set(nass, natija);
  if (TAHWILAT.size > SAAT_TAHWIL) {
    const aqdam = TAHWILAT.keys().next();
    if (aqdam.done !== true) {
      TAHWILAT.delete(aqdam.value);
    }
  }
  return natija;
}

/* ==========================================================================
   The stream's payload.
   ========================================================================== */

/** One game's artwork, settled: a source the card can load, or null for none. */
export interface GhilafMuhall {
  /** Taarib's identity for the game, matching the library row's. */
  readonly muarrif: string;
  /** A source an `<img>` can load, already through {@link masdarSalih}. */
  readonly masdar: string | null;
}

/**
 * Which of the two images the card is drawn from.
 *
 * The banner first, because the card's well is 292 × 136 — a ratio of 2.147 —
 * and a Steam header is 460 × 215, which is 2.140. The picture is shown whole,
 * with nothing cropped away at all.
 *
 * The cover second, and it is the one that costs something: a 600 × 900
 * portrait under `object-fit: cover` in this well keeps a horizontal band
 * through its middle and throws away about three quarters of the image. On a
 * store cover that band is usually background art rather than the title, so
 * it is a poor picture of the game. It is still taken over a generated plate,
 * and it is only ever reached for a game whose launcher published a portrait
 * and no header.
 *
 * This order was the other way round when the card was portrait, and inverting
 * the card without inverting this would have cropped every game in the library.
 *
 * One line, so that reversing it is one line.
 */
export function ikhtarMasdar(ghilaf: string | null, batl: string | null): string | null {
  return batl ?? ghilaf;
}

/**
 * One event's payload, as a settled answer.
 *
 * Checked structurally rather than trusted: `listen` types its payload from the
 * generated bindings, and a backend one version ahead can send a shape this
 * build has never seen. An unrecognised payload settles nothing, which leaves
 * the card on its plate — correct, just uninformed.
 *
 * `lawn` is decoded by nobody here. It is the artwork's dominant colour, which
 * the per-game screen uses as that game's accent; a grid of ten thousand cards
 * has one accent and it is the product's.
 *
 * @param khaam one `taarib://ghilaf-luba` payload
 */
export function fukkGhilaf(khaam: unknown): GhilafMuhall | null {
  if (typeof khaam !== 'object' || khaam === null) {
    return null;
  }
  const kain = khaam as Record<string, unknown>;
  const muarrif = kain['muarrif'];
  if (typeof muarrif !== 'string' || muarrif.length === 0) {
    return null;
  }
  const ghilaf = kain['ghilaf'];
  const batl = kain['batl'];
  return {
    muarrif,
    masdar: masdarSalih(
      ikhtarMasdar(
        typeof ghilaf === 'string' ? ghilaf : null,
        typeof batl === 'string' ? batl : null,
      ),
    ),
  };
}

/* ==========================================================================
   The port.
   ========================================================================== */

/** A subscription to artwork as each game settles. */
export type MustamiSuwar = (
  ala_wusul: (dufa: readonly GhilafMuhall[]) => void,
) => Promise<() => void>;

/**
 * How the grid reaches cover art.
 *
 * Two halves, because the backend has two. {@link hassil} starts a pass and
 * resolves when the pass has ended; it carries no artwork, because the artwork
 * does not wait for it. {@link istami} is where every picture actually arrives.
 *
 * An interface rather than a direct call so that a harness can drive artwork
 * into a real grid with no backend behind it — which is the only way the claim
 * that a cover replacing a plate moves nothing can be measured rather than
 * asserted.
 */
export interface MinfathSuwar {
  /**
   * Runs one resolution pass over the whole library.
   *
   * Rejecting and resolving are both ordinary: a pass that failed is retried
   * once, after the cooldown, and then left alone.
   */
  readonly hassil: () => Promise<void>;
  /** Subscribes to artwork as it settles, or null for a port with no stream. */
  readonly istami: MustamiSuwar | null;
}

/** Whether this document is running inside a Tauri webview at all. */
function fiTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * The port every grid uses unless it is handed another one.
 *
 * The summary `hassil_suwar_maktaba` answers with — how many games were looked
 * at, how many ended with each kind of image, how long it took — is deliberately
 * dropped. It is a report about a pass, and this module's only question is
 * whether the pass ended; every fact the grid draws arrives on the stream. Its
 * `bila_suwar` field cannot stand in for that either: it holds game *names*,
 * and nothing may be settled by a name when identities exist.
 */
export const minfathTauri: MinfathSuwar = {
  hassil: async () => {
    if (!fiTauri()) {
      return;
    }
    await nadi('hassil_suwar_maktaba');
  },

  istami: async (ala_wusul) => {
    if (!fiTauri()) {
      return () => undefined;
    }
    try {
      return await listen<GhilafHie>(HADATH_GHILAF_LUBA, (hadath) => {
        const wahid = fukkGhilaf(hadath.payload);
        if (wahid !== null) {
          ala_wusul([wahid]);
        }
      });
    } catch {
      // A window that cannot subscribe still shows whatever the library record
      // carried; it just never learns about the games that resolved later.
      return () => undefined;
    }
  },
};

/* ==========================================================================
   The scheduler.
   ========================================================================== */

/** How long the visible set must hold still before a decision, in milliseconds. */
const MUHLAT_HUDU = 120;

/** How long a decision may be deferred by continuous scrolling, in milliseconds. */
const AQSA_INTIZAR = 500;

/** How long after a pass ends before another may start, in milliseconds. */
const MUHLAT_IAADA = 30_000;

/** How many passes may fail in a row before the grid stops asking. */
const AQSA_MUHAWALAT = 2;

/**
 * How many resolved sources are retained before the oldest is dropped.
 *
 * A pass speaks about the whole library, not about the window, so this fills to
 * the size of the library rather than to the size of the screen. The cap is far
 * above any plausible library because a source is an `asset://` URL of a couple
 * of hundred characters — ten thousand of them is a couple of megabytes of
 * strings, which is not a budget worth managing. It exists as a floor under a
 * pathological case, and a game past it falls back to its plate rather than
 * being asked for again: the command has no per-game argument, so re-fetching
 * one dropped source would mean re-walking the entire library.
 */
const SAAT_MAKHZAN = 20_000;

/**
 * Everything the scheduler owns, held in a ref rather than in state.
 *
 * None of it may cause a render: it changes on every scroll frame, and a grid
 * that re-rendered because its bookkeeping moved would be a grid that dropped
 * frames while scrolling. The one field that reaches React is the store, and it
 * reaches it through {@link anshur} only when artwork actually arrives.
 */
interface DakhilSuwar {
  /** The identities on screen that have no artwork yet, as of the last commit. */
  zahira: readonly string[];
  /** The port in force. Replaced when the prop changes. */
  minfath: MinfathSuwar;
  /** Hands a new snapshot of the store to React. */
  anshur: (sijill: ReadonlyMap<string, string>) => void;
  /** Every source resolved so far, in arrival order. */
  readonly sijill: Map<string, string>;
  /** Every game the stream has spoken about, with artwork or without. */
  readonly mustaqirra: Set<string>;
  /** Whether a pass is running. */
  jari: boolean;
  /** When the last pass ended, as epoch milliseconds, or zero for none. */
  masaha: number;
  /** How many passes have failed in a row. */
  ikhfaqat: number;
  /** The pending decision timer, or zero for none. */
  muaqqit: number;
  /** When the current deferral must end regardless, or zero for none. */
  hadd: number;
  /** False once the hook has unmounted, so a late answer does nothing. */
  hayy: boolean;
}

/** A fresh scheduler, with no port and nowhere to publish yet. */
function inshiDakhil(): DakhilSuwar {
  return {
    zahira: [],
    minfath: minfathTauri,
    anshur: () => undefined,
    sijill: new Map<string, string>(),
    mustaqirra: new Set<string>(),
    jari: false,
    masaha: 0,
    ikhfaqat: 0,
    muaqqit: 0,
    hadd: 0,
    hayy: true,
  };
}

/** Whether two visible lists hold the same identities in the same order. */
function mutasawiyaQaima(awwal: readonly string[], thani: readonly string[]): boolean {
  if (awwal.length !== thani.length) {
    return false;
  }
  for (const [martaba, muarrif] of awwal.entries()) {
    if (thani[martaba] !== muarrif) {
      return false;
    }
  }
  return true;
}

/**
 * Files what the stream said and publishes a new snapshot if anything changed.
 *
 * A game with no artwork is settled and stored nowhere, which is the whole
 * difference between "no cover" and "not asked yet": both draw the plate, and
 * only the second one is a reason to run a pass.
 *
 * The store is the scheduler's own map and React holds a copy. The other way
 * round — React holding the map and the scheduler reading it back — would make
 * this depend on a render having happened, and artwork arrives between renders.
 */
function adrijDufa(halat: DakhilSuwar, dufa: readonly GhilafMuhall[]): void {
  let taghayyar = false;

  for (const wahid of dufa) {
    halat.mustaqirra.add(wahid.muarrif);
    if (wahid.masdar === null || halat.sijill.get(wahid.muarrif) === wahid.masdar) {
      continue;
    }
    halat.sijill.set(wahid.muarrif, wahid.masdar);
    taghayyar = true;
  }

  while (halat.sijill.size > SAAT_MAKHZAN) {
    const aqdam = halat.sijill.keys().next();
    if (aqdam.done === true) {
      break;
    }
    // Dropped but still settled: there is no way to ask for one game's artwork
    // again, so unsettling it would buy a full re-walk of the library for one
    // card. It falls back to its plate, which is what it had before.
    halat.sijill.delete(aqdam.value);
    taghayyar = true;
  }

  if (taghayyar) {
    halat.anshur(new Map(halat.sijill));
  }
}

/**
 * Arranges for a decision, no sooner than the visible set holding still.
 *
 * The deadline is what keeps a slow, continuous scroll from starving: each new
 * visible set pushes the settle delay out again, and without a ceiling a user
 * who never quite stops moving never reaches a decision at all.
 */
function jadwil(halat: DakhilSuwar): void {
  if (!halat.hayy) {
    return;
  }
  const alaan = Date.now();
  if (halat.hadd === 0) {
    halat.hadd = alaan + AQSA_INTIZAR;
  }
  if (halat.muaqqit !== 0) {
    window.clearTimeout(halat.muaqqit);
  }
  const mutabaqqi = Math.max(0, Math.min(MUHLAT_HUDU, halat.hadd - alaan));
  halat.muaqqit = window.setTimeout(() => {
    qarrir(halat);
  }, mutabaqqi);
}

/**
 * Decides whether the library needs a resolution pass, and starts one if so.
 *
 * The visible set is read here, at the moment of deciding, rather than captured
 * when the decision was scheduled: everything the user scrolled past has
 * already left {@link DakhilSuwar.zahira} by the time this runs, so a grid
 * dragged through a thousand rows reaches one decision about the screenful the
 * user stopped on.
 */
function qarrir(halat: DakhilSuwar): void {
  halat.muaqqit = 0;
  halat.hadd = 0;
  if (!halat.hayy || halat.jari || halat.ikhfaqat >= AQSA_MUHAWALAT) {
    return;
  }

  let naqis = false;
  for (const muarrif of halat.zahira) {
    if (!halat.mustaqirra.has(muarrif)) {
      naqis = true;
      break;
    }
  }
  if (!naqis) {
    return;
  }

  const alaan = Date.now();
  if (halat.masaha !== 0 && alaan - halat.masaha < MUHLAT_IAADA) {
    return;
  }

  halat.jari = true;
  void halat.minfath.hassil().then(
    () => {
      if (!halat.hayy) {
        return;
      }
      halat.jari = false;
      halat.masaha = Date.now();
      halat.ikhfaqat = 0;
    },
    () => {
      if (!halat.hayy) {
        return;
      }
      halat.jari = false;
      halat.masaha = Date.now();
      // Retried once, because a pass can fail for a reason that passes — a busy
      // disk, a launcher rewriting its cache — and then left alone, because a
      // card with a plate is a finished card and not an error.
      halat.ikhfaqat += 1;
    },
  );
}

/* ==========================================================================
   The hook.
   ========================================================================== */

/** Nothing resolved yet. Shared, so an idle grid holds no map of its own. */
const SIJILL_FARIGH: ReadonlyMap<string, string> = new Map<string, string>();

/**
 * Cover art for a grid, driven by what that grid has on screen.
 *
 * @param zahira the games on screen whose record carries no artwork, in visual
 *   order; rebuilt on every scroll frame by the caller and compared by content
 *   here, so an unchanged window costs one length check and schedules nothing
 * @param minfath the port to resolve through; the Tauri one unless a caller —
 *   a test, a measurement harness — supplies its own
 * @returns every source resolved so far, keyed by game, ready for an `<img>`
 */
export function useSuwarMaktaba(
  zahira: readonly string[],
  minfath: MinfathSuwar = minfathTauri,
): ReadonlyMap<string, string> {
  const [sijill, haddidSijill] = useState<ReadonlyMap<string, string>>(SIJILL_FARIGH);

  const marja = useRef<DakhilSuwar | null>(null);
  const halat = (marja.current ??= inshiDakhil());

  // The port and the publisher are wired in an effect rather than during
  // render, because both are only ever read by a timer or by a settled promise,
  // and neither of those can run before the first commit.
  useEffect(() => {
    halat.minfath = minfath;
    halat.anshur = haddidSijill;
  }, [halat, minfath]);

  useEffect(() => {
    if (mutasawiyaQaima(halat.zahira, zahira)) {
      return;
    }
    halat.zahira = zahira;
    jadwil(halat);
  }, [halat, zahira]);

  useEffect(() => {
    const istami = minfath.istami;
    if (istami === null) {
      return undefined;
    }
    let mushtarik = true;
    let alghi: (() => void) | null = null;

    void istami((dufa) => {
      if (halat.hayy) {
        adrijDufa(halat, dufa);
      }
    }).then(
      (fak) => {
        // Unsubscribed before the subscription finished arriving, which is what
        // a remount inside one frame looks like from here.
        if (mushtarik) {
          alghi = fak;
        } else {
          fak();
        }
      },
      () => undefined,
    );

    return () => {
      mushtarik = false;
      alghi?.();
    };
  }, [halat, minfath]);

  useEffect(
    () => () => {
      halat.hayy = false;
      if (halat.muaqqit !== 0) {
        window.clearTimeout(halat.muaqqit);
        halat.muaqqit = 0;
      }
    },
    [halat],
  );

  return sijill;
}
