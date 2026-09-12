// التنبيهات — the short notices that float over every screen: what just happened, and one thing to do about it.

import { create } from 'zustand';

/** What kind of thing a notice reports, which decides its rule colour, its glyph and how long it stays. */
export type NawTanbih = 'najah' | 'maluma' | 'tanbeeh' | 'khatar';

/** The one control a notice may carry: an undo, a way to the screen that finishes the job. */
export interface AmalTanbih {
  readonly unwan: string;
  readonly nafidh: () => void;
}

/** One notice on screen. */
export interface Tanbih {
  readonly muarrif: number;
  readonly naw: NawTanbih;
  readonly nass: string;
  /** A second, quieter line: a path, a count, the machine's own words. */
  readonly tafsil: string | null;
  /** The permanent code, when the notice reports a failure that has one. */
  readonly ramz: string | null;
  readonly amal: AmalTanbih | null;
  /** How long it stays, in milliseconds, or null to stay until dismissed. */
  readonly mudda: number | null;
}

/** What a caller says; everything it leaves out takes the kind's own default. */
export interface TalabTanbih {
  readonly naw: NawTanbih;
  readonly nass: string;
  readonly tafsil?: string | null;
  readonly ramz?: string | null;
  readonly amal?: AmalTanbih | null;
  readonly mudda?: number | null;
}

/**
 * How long each kind stays.
 *
 * A confirmation is read in a glance and gone before it is in the way. A
 * warning stays longer because it asks the reader to weigh something. A
 * failure stays until it is dismissed, because a failure that disappears on
 * its own is a failure the reader can never be sure they saw.
 */
const MUDAD: Readonly<Record<NawTanbih, number | null>> = {
  najah: 5000,
  maluma: 5000,
  tanbeeh: 8000,
  khatar: null,
};

/**
 * How many notices stand at once. Beyond this the oldest leaves: a column of
 * eight confirmations is a column nobody reads, and the newest is the one that
 * answers what the reader just did.
 */
const HADD = 4;

export interface MakhzanTanbihat {
  readonly qaima: readonly Tanbih[];
  /** Adds a notice and answers with its identity, for a caller that will dismiss it early. */
  readonly ansha: (talab: TalabTanbih) => number;
  readonly aghliq: (muarrif: number) => void;
  readonly amsah: () => void;
}

export const useTanbihat = create<MakhzanTanbihat>()((haddid, iqra) => {
  let adad = 0;
  return {
    qaima: [],

    ansha: (talab) => {
      adad += 1;
      const jadeed: Tanbih = {
        muarrif: adad,
        naw: talab.naw,
        nass: talab.nass,
        tafsil: talab.tafsil ?? null,
        ramz: talab.ramz ?? null,
        amal: talab.amal ?? null,
        mudda: talab.mudda === undefined ? MUDAD[talab.naw] : talab.mudda,
      };
      // The same sentence twice is one sentence said again: the earlier copy
      // leaves and the new one takes its place at the end, so a repeated
      // action refreshes its notice rather than stacking it.
      const baqiya = iqra().qaima.filter(
        (tanbih) => !(tanbih.naw === jadeed.naw && tanbih.nass === jadeed.nass),
      );
      haddid({ qaima: [...baqiya, jadeed].slice(-HADD) });
      return adad;
    },

    aghliq: (muarrif) => {
      haddid({ qaima: iqra().qaima.filter((tanbih) => tanbih.muarrif !== muarrif) });
    },

    amsah: () => {
      haddid({ qaima: [] });
    },
  };
});

/** Adds a notice from outside a component: a store, a hook, a callback that has no render of its own. */
export function ansha(talab: TalabTanbih): number {
  return useTanbihat.getState().ansha(talab);
}
