// سجل لوحة الأوامر — actions registered by id, sorted by majal, filtered by substring.

import { useEffect } from 'react';
import { create } from 'zustand';

/** One palette action. */
export interface AmrLawha {
  readonly muarrif: string;
  readonly unwan: string;
  readonly majal: string;
  readonly ikhtisar?: string;
  readonly nafidh: () => void;
}

/** The registry, the open state, and everything that changes them. */
export interface MakhzanAwamirLawha {
  readonly awamir: ReadonlyMap<string, AmrLawha>;
  readonly maftuha: boolean;
  /** Registers a batch and returns the unregister that removes exactly those ids. */
  readonly sajjil: (awamir: readonly AmrLawha[]) => () => void;
  /** Every registered action, sorted by majal then unwan. */
  readonly kul: () => AmrLawha[];
  readonly iftah: () => void;
  readonly aghliq: () => void;
  readonly baddil: () => void;
}

export const useAwamirLawha = create<MakhzanAwamirLawha>()((haddid, iqra) => ({
  awamir: new Map<string, AmrLawha>(),
  maftuha: false,

  sajjil: (jadida) => {
    const damj = new Map(iqra().awamir);
    for (const amr of jadida) {
      damj.set(amr.muarrif, amr);
    }
    haddid({ awamir: damj });
    return () => {
      const baqiya = new Map(iqra().awamir);
      for (const amr of jadida) {
        baqiya.delete(amr.muarrif);
      }
      haddid({ awamir: baqiya });
    };
  },

  kul: () =>
    [...iqra().awamir.values()].sort(
      (awwal, thani) =>
        awwal.majal.localeCompare(thani.majal) || awwal.unwan.localeCompare(thani.unwan),
    ),

  iftah: () => {
    haddid({ maftuha: true });
  },

  aghliq: () => {
    haddid({ maftuha: false });
  },

  baddil: () => {
    haddid({ maftuha: !iqra().maftuha });
  },
}));

/** Case-insensitive substring over unwan+majal; an empty query returns all. */
export function rashshih(awamir: readonly AmrLawha[], istifsar: string): AmrLawha[] {
  const ibra = istifsar.trim().toLowerCase();
  if (ibra === '') {
    return [...awamir];
  }
  return awamir.filter(
    (amr) => amr.unwan.toLowerCase().includes(ibra) || amr.majal.toLowerCase().includes(ibra),
  );
}

/** Registers a screen's actions on mount and removes them on unmount. */
export function useSajjilAwamir(awamir: readonly AmrLawha[]): void {
  const sajjil = useAwamirLawha((halat) => halat.sajjil);
  // Invariant: a caller's muarrif set is fixed for its lifetime; new closures under
  // the same ids are not re-registered.
  const miftah = awamir.map((amr) => amr.muarrif).join('\u001f');
  useEffect(() => sajjil(awamir), [sajjil, miftah]);
}
