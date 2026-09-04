import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

import type { AilatMuharrik, HalatLuba, Manassa, Tabaqa } from '@/mustalahat/awamir';
import type { HalatRuqaa, KathafatBitaqa } from '@/mukawwinat/bitaqa';

import type {
  IttijahFarz,
  TajmiMaktaba,
  TartibMaktaba,
  TasfiyatMaktaba,
} from '@/maktaba/tanqiya';
import { ITTIJAH_IFTIRADI, TASFIYA_FARIGHA } from '@/maktaba/tanqiya';

/**
 * حفظ الخيارات — how the user left the library, kept across restarts.
 *
 * Card size, grouping, sort, filters and the collapsed state of the unavailable
 * section are all *view* state: they belong to this machine and this person,
 * they are meaningless to the backend, and a round trip to ask for them would
 * mean the grid renders once at the default and again at the user's choice.
 * That second render is a full re-layout of the whole grid, visible as a jump,
 * on every single launch. So they live here, in local storage, read
 * synchronously before the first paint.
 *
 * ## Why every field is validated separately
 *
 * The stored shape is written by whichever build the user ran last, and Taarib
 * ships filters that gain values, sorts that gain orders and a card-size list
 * that may one day lose one. Zustand's own behaviour on a shape it cannot use
 * is to keep whatever it parsed, which puts `"recently_played"` — a sort order
 * from an older build that no longer exists — straight into a `switch` that has
 * no case for it.
 *
 * The alternative usually reached for is to version the store and throw the
 * whole thing away on a mismatch. That is worse than it sounds: a user who had
 * six launcher filters and a large card size loses all six and the size because
 * the sort order gained a value. So {@link naqqi} validates field by field and
 * falls back per field. A stored blob with one unrecognised sort and five good
 * filters yields the default sort and five good filters.
 *
 * ## Card size is not density
 *
 * Taarib has two size settings and they are genuinely two, not one leaking:
 *
 * - **الكثافة**, `Kathafa` — `mudmaj | murih | kabir`. Application-wide, owned
 *   by the backend's settings tree, applied by the root layout as
 *   `data-kathafa` on the document element. It sets the root font size and
 *   every row height, so it scales the whole interface: the rail, the console's
 *   tables, the workspace, this screen's own header.
 * - **حجم البطاقة**, {@link KathafatBitaqa} — `mudmaj | qiyasi | kabir`. This
 *   store's {@link KhiyaratMaktaba.hajm_bitaqa}. Local to this machine, and it
 *   sizes exactly one thing: the artwork on a library card.
 *
 * They are kept apart because the answers genuinely differ. A user on a large
 * display who wants text they can read without leaning in and a library that
 * shows forty covers at once wants `kabir` for the first and `mudmaj` for the
 * second, and one control cannot give them both. The settings screen's own copy
 * has always said "الكثافة" for the first and this screen's has always said
 * "حجم البطاقة" for the second; what made them look like one setting was the
 * *code* — this field was called `kathafa`, its type is called `KathafatBitaqa`,
 * and two of the three values are spelled the same in both vocabularies. So the
 * field is named for what it sizes.
 */

/* ==========================================================================
   The accepted values, as arrays, so that validation and the interface's own
   option lists come from one declaration.
   ========================================================================== */

/**
 * The three card sizes the grid can draw.
 *
 * Not the application's three densities, which are `mudmaj | murih | kabir`.
 * The middle value is the difference and it is not a typo: a card has a
 * standard size, and the interface around it has a comfortable one.
 */
export const AHJAM_BITAQA: readonly KathafatBitaqa[] = ['mudmaj', 'qiyasi', 'kabir'];

/** The two ways the grid can be sectioned. */
export const TAJMIAT: readonly TajmiMaktaba[] = ['mudmaj', 'manassa'];

/** The five orders the library can be read in. */
export const TARATIB: readonly TartibMaktaba[] = [
  'ism',
  'akhir_laab',
  'akhir_tathbeet',
  'hajm',
  'ruqaa',
];

/** The two directions any order can run in. */
export const ITTIJAHAT: readonly IttijahFarz[] = ['tasaudi', 'tanazuli'];

/** Every launcher family a filter may name. */
export const MANASSAT: readonly Manassa[] = [
  'steam',
  'epic',
  'gog',
  'ea',
  'ubisoft',
  'battlenet',
  'xbox',
  'itch',
  'amazon',
  'rockstar',
  'riot',
  'lutris',
  'bottles',
  'playnite',
  'yadawi',
];

/** Every engine family a filter may name. */
export const MUHARRIKAT: readonly AilatMuharrik[] = [
  'unity',
  'unreal',
  'godot',
  'rpg_maker_mv',
  'rpg_maker_mz',
  'rpg_maker_vx_ace',
  'renpy',
  'game_maker',
  'electron',
  'majhul',
];

/** Every Arabization state a filter may name. */
export const HALAT: readonly HalatLuba[] = [
  'mutabbaqa',
  'mutaha',
  'madum_bila_ruqaa',
  'mutaha_ghayr_mutabiqa',
  'tabaqa_faqat',
  'marfuda',
];

/** Every voice-pack state a filter may name. */
export const HALAT_SAWT: readonly HalatRuqaa[] = ['la-shay', 'mutaha', 'mutabbaqa', 'tahdith'];

/** Every injection tier a filter may name. */
export const TABAQAT: readonly Tabaqa[] = ['kamil', 'rasm_mubashir', 'tarjama_fawqiya'];

/* ==========================================================================
   The state.
   ========================================================================== */

/** Everything about the library grid that survives a restart. */
export interface KhiyaratMaktaba {
  /**
   * How large the grid draws cards.
   *
   * The card's size and nothing else. The application's own density is
   * `Idadat.kathafa`, lives in the backend's settings tree, and is applied by
   * the root layout — see this file's header for why they are two settings.
   */
  readonly hajm_bitaqa: KathafatBitaqa;
  /** Whether the grid is sectioned by launcher. */
  readonly tajmi: TajmiMaktaba;
  /** The chosen order. */
  readonly tartib: TartibMaktaba;
  /** Which way that order runs. */
  readonly ittijah: IttijahFarz;
  /** The active filters. */
  readonly murashshih: TasfiyatMaktaba;
  /** Whether the unavailable-games section is collapsed. */
  readonly ghiyab_matwi: boolean;
}

/** Which filter category an operation applies to. */
export type FiatTasfiya = keyof TasfiyatMaktaba;

/** The state plus everything that changes it. */
export interface MakhzanKhiyarat extends KhiyaratMaktaba {
  /**
   * What is in the search field, for as long as the window is open.
   *
   * Held here rather than in the screen's own `useState` so that it survives a
   * route change: a user who searches for a game, opens it, and comes back
   * expects to be looking at their search rather than at the whole library
   * again — the filters beside it already behave that way, and a search field
   * that empties itself while the launcher chips stay ticked is a screen that
   * half-remembers.
   *
   * Deliberately absent from {@link KhiyaratMaktaba}, which is the persisted
   * shape, so `partialize` cannot write it. A query is a thing the user is
   * doing right now, not a preference: an application that reopened three days
   * later still showing `dark sou` would be an application that had lost the
   * user's library, and they would have to work out what to clear.
   */
  readonly bahth: string;
  /** Sets the card size. */
  readonly haddidHajmBitaqa: (hajm: KathafatBitaqa) => void;
  /** Sets what is in the search field. */
  readonly haddidBahth: (bahth: string) => void;
  /** Sets the grouping. */
  readonly haddidTajmi: (tajmi: TajmiMaktaba) => void;
  /**
   * Sets the order, and its direction.
   *
   * Choosing the order the grid is already in reverses it, which is what a user
   * clicking a sort control twice means, and choosing a different one adopts
   * that order's own default direction rather than carrying the previous one
   * across — "size, oldest first" is not what somebody who was reading names
   * A-to-Z asked for when they switched to size.
   */
  readonly haddidTartib: (tartib: TartibMaktaba) => void;
  /** Reverses the current order without changing which order it is. */
  readonly aksIttijah: () => void;
  /** Adds or removes one value from one filter category. */
  readonly baddilMurashshih: (fia: FiatTasfiya, qeema: string) => void;
  /** Clears one category, or every category when given nothing. */
  readonly amsahMurashshih: (fia?: FiatTasfiya) => void;
  /** Opens or closes the unavailable-games section. */
  readonly baddilGhiyab: () => void;
  /**
   * Returns every preference to its default.
   *
   * The search is left alone: it is not a preference, and a control labelled
   * "reset the view" that also wiped what somebody was in the middle of typing
   * would be doing two things under one name.
   */
  readonly sifr: () => void;
}

/**
 * What a fresh install looks like.
 *
 * The standard density, one merged library, alphabetical, no filters, and the
 * unavailable section closed. Closed because it is the answer to a question
 * most users never ask, and a library that opens with a list of things that are
 * wrong is a library that looks broken on first run.
 */
export const KHIYARAT_IFTIRADIYA: KhiyaratMaktaba = {
  hajm_bitaqa: 'qiyasi',
  tajmi: 'mudmaj',
  tartib: 'ism',
  ittijah: ITTIJAH_IFTIRADI.ism,
  murashshih: TASFIYA_FARIGHA,
  ghiyab_matwi: true,
};

/* ==========================================================================
   Validation.
   ========================================================================== */

/** Whether a stored value is one of a union's members. */
function wahidMin<T extends string>(qeema: unknown, maqbula: readonly T[]): qeema is T {
  return typeof qeema === 'string' && (maqbula as readonly string[]).includes(qeema);
}

/**
 * A stored array, reduced to the values this build still recognises.
 *
 * Unknown members are dropped rather than failing the whole category, because
 * the case this exists for is a filter that *lost* a value between builds — a
 * launcher that stopped being supported — and losing the other four selections
 * over it would be losing work the user did.
 *
 * Duplicates are dropped too. Nothing in the interface can create one, but a
 * hand-edited storage entry can, and a duplicated launcher would show a filter
 * count of six where five chips are visible.
 */
function naqqiQaima<T extends string>(khaam: unknown, maqbula: readonly T[]): T[] {
  if (!Array.isArray(khaam)) {
    return [];
  }
  const makhraj: T[] = [];
  for (const qeema of khaam as readonly unknown[]) {
    if (wahidMin(qeema, maqbula) && !makhraj.includes(qeema)) {
      makhraj.push(qeema);
    }
  }
  return makhraj;
}

/** A stored object as a record, or an empty one when it is anything else. */
function kaSijill(khaam: unknown): Readonly<Record<string, unknown>> {
  if (typeof khaam !== 'object' || khaam === null || Array.isArray(khaam)) {
    return {};
  }
  return khaam as Readonly<Record<string, unknown>>;
}

/**
 * Rebuilds a valid preference set out of whatever was in storage.
 *
 * Never throws and never returns a partial object: every field is either the
 * stored value, proven to be one this build accepts, or the default. That
 * guarantee is what lets the rest of the product treat the store's fields as
 * exhaustive unions and write `switch` statements over them with no default
 * case.
 *
 * @param khaam the parsed contents of the storage entry, of unknown shape
 */
export function naqqi(khaam: unknown): KhiyaratMaktaba {
  const sijill = kaSijill(khaam);
  const murashshihKhaam = kaSijill(sijill['murashshih']);

  const tartib = wahidMin(sijill['tartib'], TARATIB)
    ? sijill['tartib']
    : KHIYARAT_IFTIRADIYA.tartib;

  // `hajm_bitaqa` was written as `kathafa` until the card's size was named
  // apart from the application's density. An install from a build before that
  // still spells it the old way and its owner still chose a size; reading both
  // is what makes the rename cost them nothing. The stored version is not
  // bumped for it, because the field did not change meaning — it changed name,
  // which is exactly what this validator absorbs without a migration.
  const hajmKhaam =
    sijill['hajm_bitaqa'] === undefined ? sijill['kathafa'] : sijill['hajm_bitaqa'];

  return {
    hajm_bitaqa: wahidMin(hajmKhaam, AHJAM_BITAQA)
      ? hajmKhaam
      : KHIYARAT_IFTIRADIYA.hajm_bitaqa,
    tajmi: wahidMin(sijill['tajmi'], TAJMIAT) ? sijill['tajmi'] : KHIYARAT_IFTIRADIYA.tajmi,
    tartib,
    // Falls back to the *order's* default rather than the store's, so a stored
    // sort that survived alongside a corrupt direction still reads the way that
    // sort is meant to read.
    ittijah: wahidMin(sijill['ittijah'], ITTIJAHAT)
      ? sijill['ittijah']
      : ITTIJAH_IFTIRADI[tartib],
    murashshih: {
      manassat: naqqiQaima(murashshihKhaam['manassat'], MANASSAT),
      muharrikat: naqqiQaima(murashshihKhaam['muharrikat'], MUHARRIKAT),
      halat: naqqiQaima(murashshihKhaam['halat'], HALAT),
      sawt: naqqiQaima(murashshihKhaam['sawt'], HALAT_SAWT),
      tabaqat: naqqiQaima(murashshihKhaam['tabaqat'], TABAQAT),
    },
    ghiyab_matwi:
      typeof sijill['ghiyab_matwi'] === 'boolean'
        ? sijill['ghiyab_matwi']
        : KHIYARAT_IFTIRADIYA.ghiyab_matwi,
  };
}

/* ==========================================================================
   Filter mutation.
   ========================================================================== */

/** Adds a value to a category's list, or removes it when it is already there. */
function qalib<T extends string>(hali: readonly T[], qeema: T): T[] {
  return hali.includes(qeema) ? hali.filter((mawjud) => mawjud !== qeema) : [...hali, qeema];
}

/**
 * Adds or removes one value in one category, returning a new filter set.
 *
 * Takes the value as a `string` and narrows it against the category's own list
 * rather than taking a union, because the caller is a click handler on a
 * rendered list of options: typing it precisely at the call site would mean
 * five handlers and a five-way discriminated payload to express one toggle.
 *
 * An unrecognised value is a no-op that returns the *same* object, so a stale
 * event cannot make the grid re-derive its whole visible set for nothing.
 *
 * Written as one branch per category, with no cast anywhere. A computed key
 * would need `jadeed as readonly Manassa[]`, and a cast is exactly how a filter
 * category ends up holding an engine name.
 */
function baddil(murashshih: TasfiyatMaktaba, fia: FiatTasfiya, qeema: string): TasfiyatMaktaba {
  switch (fia) {
    case 'manassat':
      return wahidMin(qeema, MANASSAT)
        ? { ...murashshih, manassat: qalib(murashshih.manassat, qeema) }
        : murashshih;
    case 'muharrikat':
      return wahidMin(qeema, MUHARRIKAT)
        ? { ...murashshih, muharrikat: qalib(murashshih.muharrikat, qeema) }
        : murashshih;
    case 'halat':
      return wahidMin(qeema, HALAT)
        ? { ...murashshih, halat: qalib(murashshih.halat, qeema) }
        : murashshih;
    case 'sawt':
      return wahidMin(qeema, HALAT_SAWT)
        ? { ...murashshih, sawt: qalib(murashshih.sawt, qeema) }
        : murashshih;
    case 'tabaqat':
      return wahidMin(qeema, TABAQAT)
        ? { ...murashshih, tabaqat: qalib(murashshih.tabaqat, qeema) }
        : murashshih;
  }
}

/** Empties one category, or every category. */
function imsah(murashshih: TasfiyatMaktaba, fia: FiatTasfiya | undefined): TasfiyatMaktaba {
  if (fia === undefined) {
    return TASFIYA_FARIGHA;
  }
  switch (fia) {
    case 'manassat':
      return { ...murashshih, manassat: [] };
    case 'muharrikat':
      return { ...murashshih, muharrikat: [] };
    case 'halat':
      return { ...murashshih, halat: [] };
    case 'sawt':
      return { ...murashshih, sawt: [] };
    case 'tabaqat':
      return { ...murashshih, tabaqat: [] };
  }
}

/* ==========================================================================
   Storage.
   ========================================================================== */

/** The storage key. Namespaced, because this origin is the whole product. */
const MIFTAH_TAKHZIN = 'taarib.maktaba.khiyarat';

/**
 * The current shape's version.
 *
 * Bumped only when a field changes *meaning* rather than when one gains a
 * value, because {@link naqqi} already absorbs new and missing values without a
 * migration. A version bump here is a statement that the old data is wrong, not
 * that it is old.
 */
const ISDAR_TAKHZIN = 1;

/**
 * An in-memory stand-in for `localStorage`.
 *
 * The Tauri webview always has real storage, but a unit test and a stripped
 * webview do not, and a store that threw on construction would take the whole
 * library screen down with it over a preference.
 */
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

/** Real storage where there is some, memory where there is not. */
function takhzin(): Storage {
  try {
    const mawjud = globalThis.localStorage;
    if (typeof mawjud === 'object') {
      return mawjud;
    }
  } catch {
    // Accessing `localStorage` throws rather than returning undefined when the
    // embedder has disabled site data, so the guard has to be a try block.
  }
  return takhzinDhakira();
}

/**
 * The library's preferences, persisted.
 *
 * Subscribe to the fields a component actually reads — `useKhiyaratMaktaba((h)
 * => h.hajm_bitaqa)` — rather than to the whole store. The grid re-lays-out on
 * a card-size change and must not re-render on a filter change it does not use.
 */
export const useKhiyaratMaktaba = create<MakhzanKhiyarat>()(
  persist(
    (haddid, iqra) => ({
      ...KHIYARAT_IFTIRADIYA,
      bahth: '',

      haddidHajmBitaqa: (hajm) => {
        haddid({ hajm_bitaqa: hajm });
      },

      haddidBahth: (bahth) => {
        haddid({ bahth });
      },

      haddidTajmi: (tajmi) => {
        haddid({ tajmi });
      },

      haddidTartib: (tartib) => {
        const hali = iqra();
        if (hali.tartib === tartib) {
          haddid({ ittijah: hali.ittijah === 'tasaudi' ? 'tanazuli' : 'tasaudi' });
          return;
        }
        haddid({ tartib, ittijah: ITTIJAH_IFTIRADI[tartib] });
      },

      aksIttijah: () => {
        haddid({ ittijah: iqra().ittijah === 'tasaudi' ? 'tanazuli' : 'tasaudi' });
      },

      baddilMurashshih: (fia, qeema) => {
        const hali = iqra();
        const jadeed = baddil(hali.murashshih, fia, qeema);
        if (jadeed !== hali.murashshih) {
          haddid({ murashshih: jadeed });
        }
      },

      amsahMurashshih: (fia) => {
        haddid({ murashshih: imsah(iqra().murashshih, fia) });
      },

      baddilGhiyab: () => {
        haddid({ ghiyab_matwi: !iqra().ghiyab_matwi });
      },

      sifr: () => {
        haddid({ ...KHIYARAT_IFTIRADIYA });
      },
    }),
    {
      name: MIFTAH_TAKHZIN,
      version: ISDAR_TAKHZIN,
      storage: createJSONStorage(takhzin),

      // Only the values, and only the ones that are preferences. Persisting the
      // actions would write function keys that serialize to nothing and then
      // read back as `undefined`, replacing live handlers with holes on the
      // second launch; persisting `bahth` would reopen the application into a
      // library filtered by a query the user typed days ago. The return type is
      // the guard for both — `KhiyaratMaktaba` is the persisted shape, and a
      // field that must not be written is a field that is not on it.
      partialize: (halat): KhiyaratMaktaba => ({
        hajm_bitaqa: halat.hajm_bitaqa,
        tajmi: halat.tajmi,
        tartib: halat.tartib,
        ittijah: halat.ittijah,
        murashshih: halat.murashshih,
        ghiyab_matwi: halat.ghiyab_matwi,
      }),

      // Every stored version goes through the same validator, so a version bump
      // needs no per-version migration function: a field that gained or lost a
      // value is already absorbed. Only a field whose *meaning* changed would
      // need a step added here.
      migrate: (mahfuz: unknown): KhiyaratMaktaba => naqqi(mahfuz),

      // The single point where stored data becomes state. Sanitising here
      // rather than in `onRehydrateStorage` means invalid data is never in the
      // store at all, not even for the tick between hydration and the callback.
      //
      // `naqqi` answers with the persisted shape, so the spread cannot reach
      // `bahth` or any action: whatever was in storage decides the preferences
      // and nothing else.
      merge: (mahfuz: unknown, hali: MakhzanKhiyarat): MakhzanKhiyarat => ({
        ...hali,
        ...naqqi(mahfuz),
      }),
    },
  ),
);
