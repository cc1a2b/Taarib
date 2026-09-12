// التراجع — a stack bounded to the last fifty steps, and its window-level chord binding.

import { useCallback, useEffect } from 'react';
import { create } from 'zustand';

import { IKHTISAR_TARAJU, hallilAw, yutabiq } from '@/hayat/ikhtisarat';

/** One undoable step. */
export interface KhatwaTaraju {
  readonly wasf: string;
  readonly taraju: () => Promise<void>;
}

const HADD_MAKDAS = 50;

/** The stack, whether an undo is in flight, and everything that changes them. */
export interface MakhzanTaraju {
  readonly makdas: readonly KhatwaTaraju[];
  readonly jari: boolean;
  readonly sajjil: (khatwa: KhatwaTaraju) => void;
  /** Pops, awaits, and returns the wasf; null when the stack is empty or an undo is running. */
  readonly taraju: () => Promise<string | null>;
  readonly akhir: () => string | null;
}

export const useTaraju = create<MakhzanTaraju>()((haddid, iqra) => ({
  makdas: [],
  jari: false,

  sajjil: (khatwa) => {
    haddid({ makdas: [...iqra().makdas, khatwa].slice(-HADD_MAKDAS) });
  },

  taraju: async () => {
    const { makdas, jari } = iqra();
    const khatwa = makdas.at(-1);
    if (khatwa === undefined || jari) {
      return null;
    }
    haddid({ makdas: makdas.slice(0, -1), jari: true });
    try {
      await khatwa.taraju();
      return khatwa.wasf;
    } catch (khata) {
      // The step was not undone, so it goes back on top and stays retryable.
      haddid({ makdas: [...iqra().makdas, khatwa].slice(-HADD_MAKDAS) });
      throw khata;
    } finally {
      haddid({ jari: false });
    }
  },

  akhir: () => iqra().makdas.at(-1)?.wasf ?? null,
}));

/** What the caller runs: the busy flag, and the trigger. */
export interface HalatMiftahTaraju {
  readonly jari: boolean;
  /** Pops one step and reports it — what the chord and the palette action both run. */
  readonly shaghghil: () => void;
}

/**
 * Binds the undo chord, window-level, to the stack. Mount once — every
 * consumer adds its own listener.
 *
 * @param ikhtisar the stored chord text; an unreadable one falls back to Ctrl+Z
 * @param alaTamam told what was undone, so the caller can say so in its own words
 */
export function useMiftahTaraju(
  ikhtisar: string,
  alaTamam: (wasf: string) => void,
): HalatMiftahTaraju {
  const taraju = useTaraju((halat) => halat.taraju);
  const jari = useTaraju((halat) => halat.jari);

  const shaghghil = useCallback((): void => {
    void taraju()
      .then((wasf) => {
        if (wasf !== null) {
          alaTamam(wasf);
        }
      })
      .catch(() => {
        // The failed step is already back on the stack; there is nothing to announce.
      });
  }, [taraju, alaTamam]);

  useEffect(() => {
    const maqrua = hallilAw(ikhtisar, IKHTISAR_TARAJU);
    const alaMiftah = (hadath: KeyboardEvent): void => {
      if (!yutabiq(hadath, maqrua)) {
        return;
      }
      // Inside an editable element the native text undo must stay native.
      const hadaf = hadath.target;
      if (
        hadaf instanceof HTMLElement &&
        (hadaf.tagName === 'INPUT' || hadaf.tagName === 'TEXTAREA' || hadaf.isContentEditable)
      ) {
        return;
      }
      hadath.preventDefault();
      shaghghil();
    };
    window.addEventListener('keydown', alaMiftah);
    return () => {
      window.removeEventListener('keydown', alaMiftah);
    };
  }, [shaghghil, ikhtisar]);

  return { jari, shaghghil };
}
