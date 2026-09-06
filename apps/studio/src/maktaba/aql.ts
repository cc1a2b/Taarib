import type { AqlLubaHie, KhatarHie, Lugha, ManiHie, ShahidHie } from '@/mustalahat/awamir';

/**
 * العقل — the core's answers about one game, narrowed for the surfaces that
 * read them.
 *
 * ## What this file does and does not decide
 *
 * Nothing here decides anything about a game. `crates/taarib-aql` holds every
 * producer's answer for one game and answers five questions from it — which
 * product, what is promised, what the limits are, what risks need consent, and
 * what blocks it outright — and `aql_luba` sends all five with the evidence
 * chain behind each. This file narrows the four discriminants that cross the
 * wire as strings and reads entries out of the lists. It contains **no
 * ordering**: {@link AqlLubaHie.mawani} arrives sorted by the core's own
 * `NawMani::rutba`, and a surface that needs one reason takes the first entry.
 *
 * That is the point of the whole exercise. The automatic-run screen and the
 * game screen each used to apply a priority of their own over anti-cheat,
 * readiness and the multiplayer question, and the two named different refusals
 * for the same game. There is one order now and it is not in TypeScript.
 *
 * ## Why the discriminants arrive as strings
 *
 * The core does not derive `specta::Type` — it is a domain crate with no
 * interface dependency, and it is out of the Studio's write set — so the Tauri
 * layer sends each variant's own stable machine name instead of serializing the
 * enum. Those names are byte-identical to the snake_case `serde` spelling,
 * asserted variant by variant in `aql_awamir::ikhtibarat::al_asma_hiya_asma_serde`,
 * so the unions below are the real discriminants rather than a paraphrase of
 * them.
 *
 * They are still narrowed at runtime rather than cast, for the reason
 * `maktaba/jahiziya.ts` gives: a value this build has never seen must degrade to
 * `null` and be ignored, not be asserted into a union that does not admit it.
 */

/** Which of Taarib's products a game gets, or the two answers that are not one. */
export type Muntaj =
  /** The game's own text is replaced inside the engine and looks native. */
  | 'istibdal'
  /** Taarib draws the text itself over the engine's own text objects. */
  | 'rasm_mubashir'
  /** The game is not modified; Arabic is shown over it as a reading aid. */
  | 'tabaqa_fawqiya'
  /** Nothing. A blocker stands, and it is final until what it names changes. */
  | 'la_shay'
  /** Not yet known: this game has not been examined. */
  | 'majhul';

const MUNTAJAT: ReadonlySet<string> = new Set<Muntaj>([
  'istibdal',
  'rasm_mubashir',
  'tabaqa_fawqiya',
  'la_shay',
  'majhul',
]);

/** Why Taarib will not act on a game. The order is the core's, never this one's. */
export type NawMani =
  /** Anti-cheat, from evidence on disk or in the store's own catalogue. */
  | 'himaya'
  /** The anti-cheat check could not be completed, so its silence proves nothing. */
  | 'fahs_himaya_lam_yajri'
  /** The publisher already ships Arabic and the user has not asked otherwise. */
  | 'lugha_rasmiya'
  /** The launcher marks this entry as something other than a game. */
  | 'laysat_luba'
  /** The entry is a directory of ROM or disc images for an emulator. */
  | 'muhakat_rum'
  /** The game is not on this disk, or not all of it is. */
  | 'ghayr_hadira'
  /** Nobody has examined this game, so nothing can be decided about it yet. */
  | 'lam_yufhas'
  /** The part of Taarib that delivers this game's tier is unfinished here. */
  | 'jahiziya_ghaiba';

const ANWA_MAWANI: ReadonlySet<string> = new Set<NawMani>([
  'himaya',
  'fahs_himaya_lam_yajri',
  'lugha_rasmiya',
  'laysat_luba',
  'muhakat_rum',
  'ghayr_hadira',
  'lam_yufhas',
  'jahiziya_ghaiba',
]);

/** A risk the user can take, as against a blocker no answer opens. */
export type NawKhatar =
  /** Taarib's first-run statement has not been acknowledged. */
  | 'bayan_awwal'
  /** The publisher ships Arabic and the user turned the exclusion off. */
  | 'lugha_rasmiya_mutajawaza'
  /** Something that is not Taarib already holds a loader slot in the folder. */
  | 'wakeel_ghareeb'
  /** The best published patch matches the installed build only approximately. */
  | 'mutabaqa_taqribiya'
  /** The game has online play and no anti-cheat was found in it. */
  | 'laab_jamai';

const ANWA_MAKHATIR: ReadonlySet<string> = new Set<NawKhatar>([
  'bayan_awwal',
  'lugha_rasmiya_mutajawaza',
  'wakeel_ghareeb',
  'mutabaqa_taqribiya',
  'laab_jamai',
]);

/**
 * What the anti-cheat question was actually answered with.
 *
 * Three values, and the third is why this is not a boolean. `la_tawqee` is
 * *no signature matched* — a claim about Taarib's own list, not about the game —
 * and `lam_yajri` is the check never having run at all, which produces exactly
 * the empty evidence a clean game produces and must never be drawn as one.
 */
export type HalatHimaya = 'mahmiya' | 'lam_yajri' | 'la_tawqee';

/** How far a blocker reaches. */
export type NitaqMani =
  /** Nothing at all happens to this game: no patch, no overlay, no run. */
  | 'kul'
  /** Only the automatic run is refused; a published patch may still be installed. */
  | 'tashghil';

/** The product, when it is one this build knows how to draw. */
export function muntajMin(khaam: string): Muntaj | null {
  return MUNTAJAT.has(khaam) ? (khaam as Muntaj) : null;
}

/** One blocker's kind, when it is one this build knows how to draw. */
export function nawManiMin(khaam: string): NawMani | null {
  return ANWA_MAWANI.has(khaam) ? (khaam as NawMani) : null;
}

/** One risk's kind, when it is one this build knows how to draw. */
export function nawKhatarMin(khaam: string): NawKhatar | null {
  return ANWA_MAKHATIR.has(khaam) ? (khaam as NawKhatar) : null;
}

/** The protection verdict, when it is one this build knows how to draw. */
export function halatHimayaMin(khaam: string): HalatHimaya | null {
  return khaam === 'mahmiya' || khaam === 'lam_yajri' || khaam === 'la_tawqee' ? khaam : null;
}

/**
 * The single most serious blocker, which is the one a surface with room for one
 * sentence shows.
 *
 * The first entry, because the list arrives sorted. This function exists so that
 * `mawani[0]` is written once — `noUncheckedIndexedAccess` makes every call site
 * that indexes it deal with `undefined`, and four call sites dealing with it
 * four ways is how one of them ends up not dealing with it.
 */
export function maniAwwal(aql: Pick<AqlLubaHie, 'mawani'>): ManiHie | null {
  return aql.mawani[0] ?? null;
}

/** One named blocker, when it is standing. */
export function maniMin(aql: Pick<AqlLubaHie, 'mawani'>, naw: NawMani): ManiHie | null {
  return aql.mawani.find((mani) => mani.naw === naw) ?? null;
}

/** One named risk, whether or not it has been answered. */
export function khatarMin(aql: Pick<AqlLubaHie, 'makhatir'>, naw: NawKhatar): KhatarHie | null {
  return aql.makhatir.find((khatar) => khatar.naw === naw) ?? null;
}

/**
 * The producer's own sentence, in the reader's language.
 *
 * Falls back to the Arabic rather than to silence, on the terms the rest of the
 * product uses for the same crossing: a refusal written in one language is still
 * a refusal, and an empty paragraph where a reason should be is worse than a
 * paragraph the reader has to work at.
 */
export function nassLugha(
  jumla: Readonly<{ arabi: string; injilizi: string }>,
  lugha: Lugha,
): string {
  if (lugha === 'arabi') {
    return jumla.arabi;
  }
  return jumla.injilizi.length > 0 ? jumla.injilizi : jumla.arabi;
}

/** One line of a chain, as a maintainer reads it: producer, observation, place. */
export function satrShahid(shahid: ShahidHie): string {
  return shahid.mawqi === null ? shahid.wasf : `${shahid.wasf} — ${shahid.mawqi}`;
}
