import type {
  JahiziyatTashghil,
  Lugha,
  SijillMaktaba,
  TaqreerHie,
} from '@/mustalahat/awamir';

/**
 * جاهزية التشغيل — whether the half of Taarib that runs *inside* the game
 * exists yet for the engine a game turned out to be.
 *
 * ## Why this is not the tier, and why the interface must not conflate them
 *
 * A tier says what Taarib is **entitled** to do to a game. It does not say that
 * the adapter which does it has been written. For most engines in this build it
 * has not: the patch installs, the game starts, and it runs in its original
 * language. An interface that showed only the tier would therefore promise
 * Arabic that never appears — which is the one failure this module exists to
 * make impossible.
 *
 * The backend answers the question in `TaqreerHie.jahiziya` and names the gap
 * in `naqs_arabi` / `naqs_injilizi`. Nothing here re-derives either. There is no
 * engine list in this file and there must never be one: readiness is a fact
 * about the Rust adapters, it moves when Taarib is updated rather than when a
 * game is, and a second copy of it in TypeScript would be a copy that goes stale
 * silently in the direction that promises more than the product delivers.
 *
 * ## Why it lives beside the library rather than inside a screen
 *
 * Three surfaces read the same verdict — the library card, the game screen and
 * the automatic-run screen — and each of them draws a different thing from it.
 * Held inside any one of them, the narrowing below would be copied into the
 * other two, and three copies of a three-armed union is how a build ends up
 * treating an unknown fourth verdict as a warning on one screen and as silence
 * on another. `maktaba/` is the shelf the game-facing surfaces already share
 * for logic that is not a component and not a screen — `suwar`, `tanqiya` —
 * and this is that.
 */

/**
 * The three answers, re-exported from the generated bindings under this
 * module's own name.
 *
 * `mukammala` — everything the tier promises reaches the screen.
 * `naqisa` — part of it does and a named part does not.
 * `ghaiba` — none of it does.
 *
 * An **alias, not a copy**. This was three string literals written by hand,
 * beside a doc comment citing `JahiziyatTashghil` two lines above declaring
 * `JahiziyaTashghil` — one letter apart, so the two never collided and nothing
 * would have caught them drifting. A fourth verdict added in Rust would have
 * type-checked cleanly here and been silently narrowed to `null` on all three
 * surfaces, which is the direction that promises more than the product
 * delivers. Aliasing makes that a compile error instead.
 *
 * The local spelling is kept because six files import it; the generated name is
 * the one that is authoritative, and it is what this resolves to.
 */
export type JahiziyaTashghil = JahiziyatTashghil;

/**
 * The verdict, when it is one this build knows how to draw.
 *
 * `null` for anything else, and the null is deliberate in both directions: a
 * report from a backend one version ahead must not be turned into an alarm out
 * of a word this build does not understand, and a row from a scan that does not
 * carry the field at all must not be given a verdict it never made.
 *
 * Takes `unknown` rather than the generated union because what reaches it is
 * not always one: a stored scan or capability report written before the field
 * existed carries no value for it, and a backend one version ahead may carry a
 * word this build has never seen. Both arrive typed and neither is readable,
 * so the gate has to be a runtime one — and one gate for every caller is one
 * place to be wrong.
 */
export function jahiziyaMin(khaam: unknown): JahiziyaTashghil | null {
  return khaam === 'mukammala' || khaam === 'naqisa' || khaam === 'ghaiba' ? khaam : null;
}

/**
 * Whether anything the tier promises actually reaches the screen.
 *
 * An unknown verdict answers `true`. The interface refuses an action on this
 * value, and refusing on a word it could not read would block a game the
 * backend may well support.
 */
export function tasil(jahiziya: JahiziyaTashghil | null): boolean {
  return jahiziya !== 'ghaiba';
}

/** The three fields of the capability report this module reads, and no others. */
export type TaqreerJahiziya = Pick<TaqreerHie, 'jahiziya' | 'naqs_arabi' | 'naqs_injilizi'>;

/**
 * The sentence naming what is unfinished, in the reader's language, or null.
 *
 * Null when the engine is fully supported, when the verdict is unreadable, and
 * when the report is an old one stored before the field existed — a report that
 * cannot say what its build could not do is not made to say something.
 */
export function naqsJahiziya(taqreer: TaqreerJahiziya, lugha: Lugha): string | null {
  const hukm = jahiziyaMin(taqreer.jahiziya);
  if (hukm === null || hukm === 'mukammala') {
    return null;
  }
  const nass = lugha === 'arabi' ? taqreer.naqs_arabi : taqreer.naqs_injilizi;
  return nass === null || nass === '' ? null : nass;
}

/**
 * The verdict a library row carries.
 *
 * The row is assembled in Rust from the same cached capability report the game
 * screen reads, so the verdict is already computed for every game in the
 * library. It is now on the wire — `SijillMaktaba.jahiziya` — and this reads the
 * typed field rather than an optional `unknown`.
 *
 * It still goes through {@link jahiziyaMin} rather than returning the field
 * directly, because a row can also arrive from a stored scan written by a build
 * that predates the field. The narrowing is what turns that absence into `null`
 * instead of `undefined` leaking into a union that does not admit it.
 */
export function jahiziyaSaf(saf: Pick<SijillMaktaba, 'jahiziya'>): JahiziyaTashghil | null {
  return jahiziyaMin(saf.jahiziya);
}
