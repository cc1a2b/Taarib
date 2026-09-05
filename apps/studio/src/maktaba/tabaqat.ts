import type { MiftahLugha } from '@/lugha/lugha';

import type {
  MudkhalRuqaaHie,
  SijillMaktaba,
  Tabaqa as TabaqaSilkiya,
  TaqreerHie,
} from '@/mustalahat/awamir';

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
 * Nor does it hold the tier's **explanation**. What a tier does to a game is a
 * fact about `Tabaqa`, the backend sends it as prose in both languages, and the
 * three paragraphs that used to live here as locale keys were deleted the day it
 * arrived. What is left are the names — a heading, a plate, and the sentence
 * about what the tier *costs*, which is the interface's own and has no field.
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
 * Takes `unknown` rather than the generated union because the union is what
 * this build was compiled against, not what a running backend sends: every
 * caller below passes a field the bindings type as `Tabaqa`, and the whole
 * value of the check is the case where that is untrue.
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
 * The tier the capability report names.
 *
 * The report carries the discriminant beside the number and the two rendered
 * names, so the game screen states which of the three products a game gets
 * rather than printing `tabaqa_raqm` and hoping the reader knows what a three
 * is. It still goes through {@link tabaqaMin} for the one case the type cannot
 * describe: a backend one version ahead naming a fourth tier, which must fall
 * through to the report's own rendered name rather than be labelled with a
 * description written for a different product.
 *
 * It is deliberately not derived from `tabaqa_raqm`. Inverting the tier number
 * here would be a second copy of `Tabaqa::raqm`, and a copy of a taxonomy is a
 * copy that mislabels rather than fails the day the taxonomy moves.
 */
export function tabaqaTaqreer(taqreer: Pick<TaqreerHie, 'tabaqa'>): Tabaqa | null {
  return tabaqaMin(taqreer.tabaqa);
}

/**
 * The tier one patch in the registry listing installs.
 *
 * Every row in that listing carries its own install button, so every row has to
 * say what pressing it produces — and the listing's only rendered name is
 * Arabic, which put `طبقة ترجمة` in front of an English reader at the exact
 * point they were deciding whether to press. Narrowed on the same terms as
 * {@link tabaqaTaqreer}.
 */
export function tabaqaMudkhal(mudkhal: Pick<MudkhalRuqaaHie, 'tabaqa'>): Tabaqa | null {
  return tabaqaMin(mudkhal.tabaqa);
}

/**
 * The short form: the plate on a library card, and the value in a patch row.
 *
 * Every game has a tier, so this plate is on every card in the grid — which is
 * the constraint the wording is written against. Each is a noun phrase naming
 * *where the Arabic ends up*, because that is the one difference between the
 * three that a person can act on at a glance: inside the game's own text, drawn
 * over it, or drawn over the picture.
 *
 * The patch listing reads it for the same reason the grid does and not because
 * the two surfaces happen to be adjacent: a row in that listing is a cell in a
 * definition list beside a coverage figure and a file size, so it has a plate's
 * worth of room and not a paragraph's, and the noun phrase is what fits.
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
 * The tier's name, in full, for the heading of the game screen's product panel.
 *
 * Separate from {@link WASM_TABAQA} on room rather than on surface: this is the
 * one place in the product that has a whole heading to state the product in, and
 * abbreviating there to fit a constraint the grid has and it does not would be
 * abbreviating for nobody. The same screen's patch rows take the short form,
 * because a cell in a definition list is measured in pixels exactly as a plate
 * on a card is.
 */
export const ISM_TABAQA: Readonly<Record<Tabaqa, MiftahLugha>> = {
  kamil: 'luba.muntaj.kamil',
  rasm_mubashir: 'luba.muntaj.rasm',
  tarjama_fawqiya: 'luba.muntaj.tabaqa',
};

/*
 * WHAT THE TIER DOES IS NOT A TABLE HERE, AND USED TO BE.
 *
 * `SHARH_TABAQA` pointed at three hand-written paragraphs per language, written
 * only because the explanation was not on the wire. It is now:
 * `TaqreerHie.sharh_arabi` and `sharh_injilizi` carry `Tabaqa`'s own words, and
 * the game screen prints them verbatim. The locale keys those three pointed at
 * were deleted with the table — two copies of one sentence drift, and the copy
 * that drifts is always the one nobody recompiles.
 *
 * The paragraph is therefore no longer conditional on the discriminant. A tier
 * this build cannot name still explains itself, because the sentence and the
 * verdict now come from the same place.
 */

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
