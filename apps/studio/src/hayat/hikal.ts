// الهيكل — how the shell was left: the rail's width, and the game it was last holding.

import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

/** The game the rail's second run points at when no game screen is open. */
export interface LubaAkhira {
  readonly muarrif: string;
  readonly ism: string;
}

/** Everything about the shell that survives a restart. */
export interface HalatHikal {
  /** Whether the rail is folded to its glyphs. */
  readonly janib_matwi: boolean;
  /**
   * The last game a screen was opened for.
   *
   * Kept so that the rail's game run stays live on the library: a person who
   * opened a game, went back for another look, and wants its workshop should
   * not have to open the game screen a second time to reach it. The name is
   * stored beside the identity because the rail draws it before any query for
   * that game has been asked on this launch.
   */
  readonly akhir_luba: LubaAkhira | null;
}

export interface MakhzanHikal extends HalatHikal {
  readonly baddilJanib: () => void;
  readonly haddidJanib: (matwi: boolean) => void;
  readonly sajjilLuba: (luba: LubaAkhira) => void;
}

const HALAT_IFTIRADIYA: HalatHikal = {
  janib_matwi: false,
  akhir_luba: null,
};

const MIFTAH_TAKHZIN = 'taarib.hikal';
const ISDAR_TAKHZIN = 1;

/** A stored object as a record, or an empty one when it is anything else. */
function kaSijill(khaam: unknown): Readonly<Record<string, unknown>> {
  if (typeof khaam !== 'object' || khaam === null || Array.isArray(khaam)) {
    return {};
  }
  return khaam as Readonly<Record<string, unknown>>;
}

/**
 * Rebuilds a valid shell state out of whatever was in storage, field by field,
 * for the reason the library's own store gives: a blob written by another
 * build must cost the user nothing they did not lose.
 */
export function naqqiHikal(khaam: unknown): HalatHikal {
  const sijill = kaSijill(khaam);
  const luba = kaSijill(sijill['akhir_luba']);
  const muarrif = luba['muarrif'];
  const ism = luba['ism'];
  return {
    janib_matwi:
      typeof sijill['janib_matwi'] === 'boolean'
        ? sijill['janib_matwi']
        : HALAT_IFTIRADIYA.janib_matwi,
    akhir_luba:
      typeof muarrif === 'string' && muarrif !== '' && typeof ism === 'string' && ism !== ''
        ? { muarrif, ism }
        : null,
  };
}

/** An in-memory stand-in for `localStorage`, for a document that has none. */
function takhzinDhakira(): Storage {
  const bayanat = new Map<string, string>();
  return {
    get length(): number {
      return bayanat.size;
    },
    clear: (): void => {
      bayanat.clear();
    },
    getItem: (miftah: string): string | null => bayanat.get(miftah) ?? null,
    key: (martaba: number): string | null => [...bayanat.keys()].at(martaba) ?? null,
    removeItem: (miftah: string): void => {
      bayanat.delete(miftah);
    },
    setItem: (miftah: string, qeema: string): void => {
      bayanat.set(miftah, qeema);
    },
  };
}

function takhzin(): Storage {
  try {
    const mawjud = globalThis.localStorage;
    if (typeof mawjud === 'object') {
      return mawjud;
    }
  } catch {
    // Reading `localStorage` throws rather than answering undefined when site
    // data is disabled, so the guard has to be a try block.
  }
  return takhzinDhakira();
}

export const useHikal = create<MakhzanHikal>()(
  persist(
    (haddid, iqra) => ({
      ...HALAT_IFTIRADIYA,

      baddilJanib: () => {
        haddid({ janib_matwi: !iqra().janib_matwi });
      },

      haddidJanib: (matwi) => {
        haddid({ janib_matwi: matwi });
      },

      sajjilLuba: (luba) => {
        const hali = iqra().akhir_luba;
        if (hali !== null && hali.muarrif === luba.muarrif && hali.ism === luba.ism) {
          return;
        }
        haddid({ akhir_luba: luba });
      },
    }),
    {
      name: MIFTAH_TAKHZIN,
      version: ISDAR_TAKHZIN,
      storage: createJSONStorage(takhzin),
      partialize: (halat): HalatHikal => ({
        janib_matwi: halat.janib_matwi,
        akhir_luba: halat.akhir_luba,
      }),
      migrate: (mahfuz: unknown): HalatHikal => naqqiHikal(mahfuz),
      merge: (mahfuz: unknown, hali: MakhzanHikal): MakhzanHikal => ({
        ...hali,
        ...naqqiHikal(mahfuz),
      }),
    },
  ),
);
