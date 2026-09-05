import type { MiftahLugha } from '@/lugha/lugha';

import type { SijillMaktaba, Tabaqa as TabaqaSilkiya, TaqreerHie } from '@/mustalahat/awamir';

/**
 * الطبقات — which of the three products a game gets, said in words.
 *
 * ## Why a tier number is not an answer
 *
 * The card and the game screen both used to state a tier as a digit: "Tier 3".
 * A digit is a rank, and a rank invites the reading that three is a worse two —
 * that the same thing arrives, slightly degraded. It does not. The three tiers
 * are three different products with three different failure modes, and the one
 * a person is about to install decides whether their translation lives inside
 * the game, is painted over the game's own text, or is drawn on top of the
 * picture and disappears when Taarib is closed. Nobody can infer that from a
 * digit, and nobody should have to.
 *
 * ## Why the vocabulary is here and not in a screen
 *
 * Three surfaces answer the same question — the library card, the game screen
 * and the automatic-run screen — and each draws a different length of the same
 * answer. Held inside any one of them, the tables below would be copied into
 * the other two, and three copies of a three-armed union is how a build ends up
 * calling the overlay "a reading aid" on one screen and "Tier 3" on another.
 * `maktaba/` is the shelf the game-facing surfaces already share for logic that
 * is neither a component nor a screen — `jahiziya`, `suwar`, `tanqiya` — and
 * this is that.
 *
 * ## What this file is allowed to hold, and what it is not
 *
 * It holds **labels for a verdict the wire already made**. It does not hold the
 * verdict. There is no engine list here, no tier-number arithmetic, and nothing
 * that decides which tier a game is on — for the same reason `jahiziya.ts`
 * refuses an engine list: a second copy of a Rust decision in TypeScript goes
 * stale silently, and it goes stale in the direction that promises more than
 * the product delivers.
 *
 * In particular, **the tier number is never inverted into a tier**. `Tabaqa` is
 * an enum in Rust and `Tabaqa::raqm` renders it as 1, 2 or 3; rebuilding the
 * reverse map here would be a second definition of the taxonomy, and the day a
 * fourth tier or a renumbering lands it would mislabel every card rather than
 * fail. The discriminant is read off the wire or it is not read at all.
 *
 * @see jahiziya — the sibling verdict: whether the tier named here actually
 * runs in this build. A tier says what Taarib is entitled to do; that says
 * whether the code that does it has been written. Both are needed and neither
 * substitutes for the other.
 */

/**
 * The three products, re-exported from the generated bindings under this
 * module's own name.
 *
 * `kamil` — the game's text is replaced inside the engine.
 * `rasm_mubashir` — Taarib draws the text itself over the engine's text objects.
 * `tarjama_fawqiya` — the game is untouched and Arabic is drawn over the picture.
 *
 * An **alias, not a copy**, on the same terms as `JahiziyaTashghil`: a fourth
 * tier added in Rust must be a compile error in every table below rather than a
 * value that quietly narrows to `null` on three surfaces at once.
 */
export type Tabaqa = TabaqaSilkiya;

/**
 * The tier, when it is one this build knows how to describe.
 *
 * `null` for anything else, and the null is deliberate in both directions: a
 * row from a backend one version ahead must not be given a description meant
 * for a different product, and a row from a stored scan that predates the field
 * must not be given a tier it never carried.
 *
 * Takes `unknown` rather than the generated union because what reaches it is
 * not always one — see {@link tabaqaTaqreer}, which reads a field the capability
 * report does not carry yet.
 */
export function tabaqaMin(khaam: unknown): Tabaqa | null {
  return khaam === 'kamil' || khaam === 'rasm_mubashir' || khaam === 'tarjama_fawqiya'
    ? khaam
    : null;
}

/**
 * The tier a library row carries.
 *
 * The row is assembled in Rust from the cached capability report, so the tier is
 * already decided for every game in the library and arrives typed on
 * `SijillMaktaba.tabaqa`. It still goes through {@link tabaqaMin} rather than
 * being returned directly, because a row can also arrive from a scan stored by a
 * build that predates the field, and the narrowing is what turns that absence
 * into `null` instead of `undefined` leaking into a union that does not admit
 * one.
 */
export function tabaqaSaf(saf: Pick<SijillMaktaba, 'tabaqa'>): Tabaqa | null {
  return tabaqaMin(saf.tabaqa);
}

/**
 * The two fields of the capability report the game screen reads for the tier,
 * plus the discriminant it needs and does not yet get.
 *
 * `tabaqa` is optional and `unknown` because **`TaqreerHie` does not carry it**.
 * The report renders the tier as a number and an Arabic name and drops the enum
 * on the way out of `luba_awamir::taqreer_hie`, so the game screen can print
 * "Tier 3 — طبقة ترجمة" and cannot say which of the three products that is in
 * the reader's own language. `SijillMaktaba` carries the discriminant; this does
 * not, and the two are built from the same `TaqreerImkaniyat` three functions
 * apart.
 *
 * The shape is written for the field rather than around its absence, so that
 * adding `pub tabaqa: Tabaqa` to `TaqreerHie` is the whole of the change: this
 * module starts answering, and every surface that reads it starts speaking,
 * with no TypeScript edit at all. Until then {@link tabaqaTaqreer} answers
 * `null` and each surface falls back to what the report does carry.
 */
export type TaqreerTabaqa = Pick<TaqreerHie, 'tabaqa_raqm' | 'tabaqa_arabi'> & {
  readonly tabaqa?: unknown;
};

/**
 * The tier the capability report names, when it names one.
 *
 * `null` in this build for every game — see {@link TaqreerTabaqa}. It is not a
 * guess and it is deliberately not derived from `tabaqa_raqm`: inverting the
 * tier number here would be a second copy of `Tabaqa::raqm`, and a copy of a
 * taxonomy is a copy that mislabels rather than fails the day the taxonomy
 * moves.
 */
export function tabaqaTaqreer(taqreer: TaqreerTabaqa): Tabaqa | null {
  return tabaqaMin(taqreer.tabaqa);
}

/**
 * The short form, for the plate on a library card.
 *
 * Every game has a tier, so this plate is on every card in the grid — which is
 * the constraint the wording is written against. Each is a noun phrase naming
 * *where the Arabic ends up*, because that is the one difference between the
 * three that a person can act on at a glance: inside the game's own text, drawn
 * over it, or drawn over the picture.
 */
export const WASM_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'bitaqa.muntaj.kamil',
  rasm_mubashir: 'bitaqa.muntaj.rasm',
  tarjama_fawqiya: 'bitaqa.muntaj.tabaqa',
};

/**
 * The clause the card is announced with.
 *
 * A whole sentence rather than the plate's noun phrase, because a screen reader
 * hears the card's states as a list and "screen overlay" arriving after two
 * patch states is heard as a third patch state. It is also the only place the
 * card can afford to say what the tier *means* rather than what it is called.
 */
export const WASF_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'bitaqa.muntaj.kamil_wasf',
  rasm_mubashir: 'bitaqa.muntaj.rasm_wasf',
  tarjama_fawqiya: 'bitaqa.muntaj.tabaqa_wasf',
};

/**
 * The tier's name, in full, for the game screen.
 *
 * Separate from {@link WASM_TABAQA} because the card's plate is measured in
 * pixels and this one is not: the game screen has a column to state the product
 * in, and abbreviating there to fit a constraint that only the grid has would be
 * abbreviating for nobody.
 */
export const ISM_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'luba.muntaj.kamil',
  rasm_mubashir: 'luba.muntaj.rasm',
  tarjama_fawqiya: 'luba.muntaj.tabaqa',
};

/**
 * What the tier does, as one paragraph, on the game screen.
 *
 * These are the interface's own wording and not the backend's, which is a
 * deliberate and temporary state: `Tabaqa::sharh_arabi` in
 * `taarib-mustalahat::muharrik` already says this in Arabic and is the sentence
 * these should become, verbatim, the moment it reaches the wire beside the
 * discriminant. Until it does, a screen that said nothing would be a screen
 * that still states the product as a digit.
 */
export const SHARH_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'luba.muntaj.kamil_sharh',
  rasm_mubashir: 'luba.muntaj.rasm_sharh',
  tarjama_fawqiya: 'luba.muntaj.tabaqa_sharh',
};

/**
 * What the tier costs, as one paragraph, on the game screen.
 *
 * The half a reader is owed before they install and the half documentation
 * usually keeps. Every one of these is a capability the player had before and
 * will not have after, stated as such rather than as a hedge — the overlay's
 * text is not in the game and does not survive Taarib closing; a takeover's text
 * looks right and has stopped being text.
 *
 * The per-game specifics stay where they already are: `TaqreerHie.hudud` names
 * what will not work in *this* game, in the backend's own words, and the game
 * screen draws that list unchanged. This is the cost of the product itself,
 * which is true of every game on the tier and is therefore the part the report
 * has no reason to repeat per game.
 */
export const KULFAT_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'luba.muntaj.kamil_kulfa',
  rasm_mubashir: 'luba.muntaj.rasm_kulfa',
  tarjama_fawqiya: 'luba.muntaj.tabaqa_kulfa',
};
