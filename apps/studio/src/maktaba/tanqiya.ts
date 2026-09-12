import type {
  AilatMuharrik,
  FiatGhiyab,
  HalatLuba,
  HalatLughaRasmiya,
  LawhaBadila,
  Lugha,
  Manassa,
  SababGhiyab,
  Tabaqa,
} from '@/mustalahat/awamir';
import { wasm } from '@/lugha/lugha';
import type { JahiziyaTashghil } from '@/maktaba/jahiziya';
import type { HalatRuqaa } from '@/mukawwinat/bitaqa';

/**
 * التنقية — search, sort, filter and grouping for the library, as pure
 * functions over plain records.
 *
 * Nothing here touches React, the query cache, or the DOM. That is deliberate
 * and load-bearing: the library grid re-derives its visible set on every
 * keystroke, and a hook that re-derived it *inside* a component would tie the
 * cost of the derivation to the render it happens to be sitting in. Here it is
 * a function the caller memoizes once and calls again only when its inputs
 * actually changed.
 *
 * ## Why there is no debounce
 *
 * The direction is explicit that search has no perceptible delay. A debounce is
 * how a slow filter hides from the user, and it hides badly — the list arrives
 * a beat after the letter, which reads as the application thinking. So the
 * filter is made fast enough not to need one instead.
 *
 * The whole trick is {@link FahrasBahth}: every string comparison a query needs
 * is precomputed per game, once, when the record enters the library. Matching
 * ten thousand games against a query is then ten thousand `indexOf` calls over
 * strings that are already normalized, already folded, already lower-cased —
 * roughly a millisecond on the hardware this product targets, and no allocation
 * per candidate. Normalizing inside the loop instead would allocate four
 * strings per game per keystroke, which is where a search like this actually
 * goes slow.
 *
 * ## Why matching is not `String.includes`
 *
 * The library holds a game whose name is `تعويذة القلب` and a user who types
 * `qalb`. It holds `Final Fantasy Ⅳ` and a user who types `final fantasy 4`. It
 * holds `Pokémon` and a user with no way to type `é`. None of those match by
 * substring, and all three are the ordinary case rather than the exotic one in
 * an Arabic-first product whose library is full of Latin titles.
 *
 * So a query is matched through three progressively looser lenses — the
 * normalized form, the folded Latin form, and the consonant skeleton that
 * bridges the two scripts — and the *quality* of the match is scored rather
 * than thrown away. A prefix hit outranks a word-boundary hit outranks a
 * substring hit outranks a subsequence hit, which is what makes typing `ff`
 * put `FF Tactics` above `Stuff Goes Here`.
 */

/* ==========================================================================
   The record the library is made of.
   ========================================================================== */

/**
 * One game, flattened into exactly what the grid and this module need.
 *
 * A view model rather than the backend's `Luba`: the grid needs the launcher's
 * display name, not its identifier union; it needs two patch states, not the
 * registry rows they were computed from. Flattening once at the edge keeps the
 * comparison functions below free of `switch` statements over wire types, and
 * keeps this module compilable without the registry.
 */
export interface SijillLuba {
  /** Taarib's identity for the game. Stable across rescans and reinstalls. */
  readonly muarrif: string;
  /** The name the launcher gives, verbatim, which is what the card shows. */
  readonly ism: string;
  /** The Arabic title, when the registry knows one, otherwise null. */
  readonly ism_arabi: string | null;
  /** Every alternate title worth matching: series names, abbreviations. */
  readonly asma_badila: readonly string[];
  /** The launcher family this game shards under. */
  readonly manassa: Manassa;
  /**
   * The name of {@link manassa}, already localized.
   *
   * The name of the *shard family*, and never the name of whichever launcher
   * happened to report the game: {@link jammi} keys its sections off `manassa`
   * and titles them from this field, so taking the two from different facts is
   * how a section headed with one launcher ends up holding another's games. A
   * game reported by Playnite and published under the Steam shard belongs to
   * the Steam section and has to say so.
   */
  readonly ism_manassa: string;
  /**
   * Whether the probe has ever examined this game.
   *
   * The qualifier on the three fields under it. {@link muharrik},
   * {@link tabaqa} and {@link jahiziya} are all non-null for every row in the
   * library, including the rows nothing has ever looked at: those carry a
   * pessimistic default — `majhul`, the overlay tier — which is byte-identical
   * to what a game the probe examined and did not recognise carries. Without
   * this the two are the same record, and the card tells a user a queue
   * position is a verdict.
   */
  readonly mafhusa: boolean;
  /** The identified engine family, when {@link mafhusa}; `majhul` until then. */
  readonly muharrik: AilatMuharrik;
  /** The injection tier Taarib can reach, when {@link mafhusa}; the floor until then. */
  readonly tabaqa: Tabaqa;
  /** The Arabization status the card badges. */
  readonly hala: HalatLuba;
  /** The text patch's state, as the card draws it. */
  readonly nass: HalatRuqaa;
  /** The voice pack's state, as the card draws it. */
  readonly sawt: HalatRuqaa;
  /**
   * What the publisher already ships, from the library scan's shallow pass.
   *
   * A fact about the game rather than a patch state, which is why it sits
   * beside them rather than among them: it decides whether the two above are
   * offered at all.
   */
  readonly lugha_rasmiya: HalatLughaRasmiya;
  /**
   * Whether Taarib can drive this game's engine in this build, or null when the
   * library scan did not say.
   *
   * A fact about Taarib and not about the game, which is why it sits apart from
   * the tier beside it: {@link tabaqa} is what Taarib is entitled to do here,
   * and this is whether the code that does it has been written. The card draws
   * the difference, because a grid that showed only the tier would present a
   * game nothing will change exactly like a game that will be translated whole.
   */
  readonly jahiziya: JahiziyaTashghil | null;
  /**
   * How many Arabic translations other teams have published for this game,
   * as the cached community index lists them; zero when it lists none or no
   * index is cached yet.
   *
   * A fact about the game on the same footing as {@link lugha_rasmiya}: it
   * decides one mark on the card and nothing about the patch states beside it.
   */
  readonly tarjamat_mujtama: number;
  /** Size on disk in bytes, or zero when the launcher reports nothing. */
  readonly hajm: number;
  /** Last played, as epoch milliseconds, or null when never or unrecorded. */
  readonly akhir_laab: number | null;
  /** Last installed or updated by the launcher, as epoch milliseconds. */
  readonly akhir_tathbeet: number | null;
  /** The resolved portrait cover, or null when none is cached yet. */
  readonly ghilaf: string | null;
  /**
   * The resolved landscape banner, or null when none is cached yet.
   *
   * Carried beside the cover rather than instead of it because the card's well
   * is landscape and the two crop very differently in it: a banner fills it
   * whole, a portrait keeps a band through its middle. Which one the card draws
   * is `maktaba/suwar`'s `ikhtarMasdar`, so that the choice is made once for
   * both the artwork already on the record and the artwork that streams in
   * afterwards.
   */
  readonly batl: string | null;
  /**
   * The generated plate, present whenever neither picture is.
   *
   * Carried on the record rather than derived in the grid because the wrapping
   * and the point size are decided in `taarib-mustalahat::lawha_badila`, and a
   * second layout rule in the interface would be a second grid.
   */
  readonly lawha: LawhaBadila | null;
}

/* ==========================================================================
   Normalization.
   ========================================================================== */

/**
 * Arabic marks that carry no identity for matching purposes.
 *
 * The fatha through sukun run plus the superscript alef, which is the one
 * combining mark outside that range that appears in ordinary game titles —
 * `هٰذا`. Stripped rather than decomposed because Arabic diacritics are not
 * canonical decompositions of the letters they sit on, so NFD leaves every one
 * of them exactly where it was.
 */
const TASHKEEL = /[ً-ْٰ]/gu;

/** The Arabic elongation dash. Decorative in a title, noise in an index. */
const TATWEEL = /ـ/gu;

/** Every combining mark, for the Latin side, after NFD has separated them. */
const ALAMAT_MURAKKABA = /[̀-ͯ]/gu;

/** Anything that is not a letter or a digit, collapsed to a single space. */
const GHAYR_ABJADI = /[^\p{L}\p{N}]+/gu;

/**
 * The alef forms, unified to bare alef.
 *
 * A user typing a game's name types `الأسطورة` or `الاسطورة` with equal
 * likelihood and neither is wrong; a library that matched only one of them
 * would look broken to whichever half of its users guessed the other.
 */
const ASHKAL_ALEF = /[أإآٱٲٳٵ]/gu;

/**
 * Normalizes Arabic text for matching.
 *
 * Strips tashkeel and tatweel, unifies the alef forms, folds ta marbuta to ha
 * and alef maqsura to ya, and unifies the two hamza-carrying ya/waw forms. What
 * survives is the consonantal skeleton a reader would recognise, which is what
 * a search over titles somebody half-remembers has to compare.
 *
 * @param nass any Arabic or mixed-script string
 */
export function wahhidArabi(nass: string): string {
  return nass
    .replace(TASHKEEL, '')
    .replace(TATWEEL, '')
    .replace(ASHKAL_ALEF, 'ا')
    .replace(/ة/gu, 'ه')
    .replace(/ى/gu, 'ي')
    .replace(/ؤ/gu, 'و')
    .replace(/ئ/gu, 'ي')
    .replace(/ء/gu, '');
}

/**
 * Arabic-Indic and extended Arabic-Indic digits, mapped to ASCII.
 *
 * A title written `فاينل فانتسي ٧` and a user typing `7` are the same query.
 * Done as a lookup rather than by arithmetic on code points because the two
 * digit blocks are not contiguous with each other.
 */
const ARQAM_ARABIYA: ReadonlyMap<string, string> = new Map([
  ['٠', '0'],
  ['١', '1'],
  ['٢', '2'],
  ['٣', '3'],
  ['٤', '4'],
  ['٥', '5'],
  ['٦', '6'],
  ['٧', '7'],
  ['٨', '8'],
  ['٩', '9'],
  ['۰', '0'],
  ['۱', '1'],
  ['۲', '2'],
  ['۳', '3'],
  ['۴', '4'],
  ['۵', '5'],
  ['۶', '6'],
  ['۷', '7'],
  ['۸', '8'],
  ['۹', '9'],
]);

/** Rewrites every non-ASCII digit in a string to its ASCII equivalent. */
function wahhidArqam(nass: string): string {
  let makhraj = '';
  for (const harf of nass) {
    makhraj += ARQAM_ARABIYA.get(harf) ?? harf;
  }
  return makhraj;
}

/**
 * The Unicode Roman numeral characters, U+2160 to U+217F, as their ASCII
 * spellings.
 *
 * Steam and GOG both ship titles containing the real characters — `Ⅳ` is one
 * code point, not two letters — and a user has no way to type them. Expanded to
 * ASCII first so that the numeral folding below sees one representation.
 */
const HURUF_RUMANIYA: ReadonlyMap<string, string> = new Map([
  ['Ⅰ', 'i'],
  ['Ⅱ', 'ii'],
  ['Ⅲ', 'iii'],
  ['Ⅳ', 'iv'],
  ['Ⅴ', 'v'],
  ['Ⅵ', 'vi'],
  ['Ⅶ', 'vii'],
  ['Ⅷ', 'viii'],
  ['Ⅸ', 'ix'],
  ['Ⅹ', 'x'],
  ['Ⅺ', 'xi'],
  ['Ⅻ', 'xii'],
  ['Ⅼ', 'l'],
  ['Ⅽ', 'c'],
  ['Ⅾ', 'd'],
  ['Ⅿ', 'm'],
  ['ⅰ', 'i'],
  ['ⅱ', 'ii'],
  ['ⅲ', 'iii'],
  ['ⅳ', 'iv'],
  ['ⅴ', 'v'],
  ['ⅵ', 'vi'],
  ['ⅶ', 'vii'],
  ['ⅷ', 'viii'],
  ['ⅸ', 'ix'],
  ['ⅹ', 'x'],
  ['ⅺ', 'xi'],
  ['ⅻ', 'xii'],
  ['ⅼ', 'l'],
  ['ⅽ', 'c'],
  ['ⅾ', 'd'],
  ['ⅿ', 'm'],
]);

/** Expands the precomposed Roman numeral characters into ASCII letters. */
function fukkRumaniya(nass: string): string {
  let makhraj = '';
  for (const harf of nass) {
    makhraj += HURUF_RUMANIYA.get(harf) ?? harf;
  }
  return makhraj;
}

/** The value of each Roman numeral letter, for the token conversion below. */
const QEEM_RUMANIYA: ReadonlyMap<string, number> = new Map([
  ['i', 1],
  ['v', 5],
  ['x', 10],
  ['l', 50],
  ['c', 100],
  ['d', 500],
  ['m', 1000],
]);

/** A token made entirely of Roman numeral letters, up to a plausible length. */
const NAMAT_RUMANI = /^[ivxlcdm]{1,7}$/u;

/**
 * A Roman numeral token as a number, or null when the token is not one.
 *
 * Bounded at 3999 and rejected when it does not round-trip, so that `mix`,
 * `civil` and `did` — all of which are made only of numeral letters — are not
 * silently rewritten into numbers in the middle of a title.
 *
 * @param kalima one whitespace-delimited token, already lower-cased
 */
function qeematRumaniya(kalima: string): number | null {
  if (!NAMAT_RUMANI.test(kalima)) {
    return null;
  }
  let majmu = 0;
  let sabiq = 0;
  for (let mawqi = kalima.length - 1; mawqi >= 0; mawqi -= 1) {
    const qeema = QEEM_RUMANIYA.get(kalima.charAt(mawqi));
    if (qeema === undefined) {
      return null;
    }
    majmu += qeema < sabiq ? -qeema : qeema;
    sabiq = Math.max(sabiq, qeema);
  }
  if (majmu <= 0 || majmu > 3999) {
    return null;
  }
  // `iiii` sums to 4 and is not how 4 is written; re-spelling the sum and
  // comparing is the cheapest way to reject every malformed numeral at once.
  return ishtaqqRumani(majmu) === kalima ? majmu : null;
}

/** The canonical Roman spelling of a number from 1 to 3999. */
function ishtaqqRumani(raqm: number): string {
  const jadwal: readonly (readonly [number, string])[] = [
    [1000, 'm'],
    [900, 'cm'],
    [500, 'd'],
    [400, 'cd'],
    [100, 'c'],
    [90, 'xc'],
    [50, 'l'],
    [40, 'xl'],
    [10, 'x'],
    [9, 'ix'],
    [5, 'v'],
    [4, 'iv'],
    [1, 'i'],
  ];
  let baqi = raqm;
  let makhraj = '';
  for (const zawj of jadwal) {
    const [qeema, ramz] = zawj;
    while (baqi >= qeema) {
      makhraj += ramz;
      baqi -= qeema;
    }
  }
  return makhraj;
}

/**
 * Rewrites every Roman numeral token in a normalized string as a decimal.
 *
 * Applied to the title and to the query as a *second* form, never as a
 * replacement for the first. `Final Fantasy IV`, `Final Fantasy Ⅳ` and `final
 * fantasy 4` all reduce to the same folded entry, so any of the three finds the
 * others.
 *
 * The reason it cannot simply replace the normalized form is `civ`. It is made
 * only of numeral letters, it is a well-formed numeral, and it is worth exactly
 * 104 — so folding the query would turn somebody typing the first three letters
 * of *Civilization* into somebody searching for the number 104. Keeping both
 * forms and scoring against both costs one extra `indexOf` per field and makes
 * that whole class of collision impossible.
 *
 * @param nass a string already lower-cased and space-separated
 */
export function wahhidAdad(nass: string): string {
  const kalimat = nass.split(' ');
  const makhraj: string[] = [];
  for (const kalima of kalimat) {
    if (kalima.length === 0) {
      continue;
    }
    const raqm = qeematRumaniya(kalima);
    makhraj.push(raqm === null ? kalima : String(raqm));
  }
  return makhraj.join(' ');
}

/**
 * Digraphs folded to one letter, so that the many spellings of one Arabic
 * consonant collapse before the skeleton is taken.
 *
 * `khalid`, `halid` and `kalid` are the same name written by three people; a
 * bridge that treated them as three different consonants would match none of
 * them against `خالد`. Ordered longest-first because `sh` must be consumed
 * before `s`, and applied as one pass rather than as chained `replace` calls so
 * that an output letter cannot be re-consumed as the start of the next digraph.
 */
const THUNAIYAT: readonly (readonly [string, string])[] = [
  ['tch', 'j'],
  ['sch', 's'],
  ['kh', 'k'],
  ['gh', 'g'],
  ['sh', 's'],
  ['ch', 's'],
  ['th', 't'],
  ['dh', 'd'],
  ['ph', 'f'],
  ['ck', 'k'],
  ['qu', 'k'],
  ['ll', 'l'],
  ['ss', 's'],
  ['tt', 't'],
  ['nn', 'n'],
  ['mm', 'm'],
  ['pp', 'p'],
  ['ff', 'f'],
  ['rr', 'r'],
  ['dd', 'd'],
];

/**
 * Normalizes Latin text for matching.
 *
 * NFD, drop the combining marks, lower-case, then collapse everything that is
 * not a letter or a digit to a single space. `Pokémon` and `pokemon` end up
 * identical, which matters because a user on an Arabic keyboard layout has no
 * comfortable way to type the acute.
 *
 * The decomposition is done before the mark strip rather than trusting NFKD to
 * do both, because NFKD also rewrites ligatures and superscripts — it would
 * turn `№` into `no` inside a title and change what the user sees matched.
 *
 * @param nass any Latin or mixed-script string
 */
export function wahhidLatini(nass: string): string {
  return nass
    .normalize('NFD')
    .replace(ALAMAT_MURAKKABA, '')
    .toLowerCase()
    .replace(/ß/gu, 'ss')
    .replace(/[æœ]/gu, 'e')
    .replace(/ø/gu, 'o')
    .replace(/đ/gu, 'd')
    .replace(/ł/gu, 'l');
}

/**
 * Both scripts, one normalized form: the string the index compares against.
 *
 * Runs the Arabic rules and the Latin rules over the same text rather than
 * choosing between them, because a game title is routinely both — `فاينل
 * فانتسي VII Remake` is one string in one field — and a normalizer that had to
 * pick a script would mangle whichever half it guessed wrong.
 *
 * @param nass a raw title as a launcher or the registry gives it
 */
export function wahhid(nass: string): string {
  const marhala = wahhidLatini(wahhidArabi(wahhidArqam(fukkRumaniya(nass))));
  return marhala.replace(GHAYR_ABJADI, ' ').trim();
}

/* ==========================================================================
   The transliteration bridge.
   ========================================================================== */

/**
 * Each Arabic letter as the romanization people actually type.
 *
 * Deliberately the *conventional* spelling rather than a one-letter code, so
 * that the digraph folding below can run over Arabic-derived and Latin-typed
 * text with one table. `ش` becomes `sh` here and `sh` becomes `s` there, which
 * is exactly what happens to a user typing `sh` — one rule, applied twice,
 * instead of two rules that can drift apart.
 *
 * `ع` and `ء` map to nothing: no transliteration convention agrees on them,
 * users type nothing for them, and a letter that produces a consonant in the
 * index and nothing in the query is a letter that breaks every match it appears
 * in.
 */
const NAQL_HARFI: ReadonlyMap<string, string> = new Map([
  ['ا', 'a'],
  ['ب', 'b'],
  ['ت', 't'],
  ['ث', 'th'],
  ['ج', 'j'],
  ['ح', 'h'],
  ['خ', 'kh'],
  ['د', 'd'],
  ['ذ', 'dh'],
  ['ر', 'r'],
  ['ز', 'z'],
  ['س', 's'],
  ['ش', 'sh'],
  ['ص', 's'],
  ['ض', 'd'],
  ['ط', 't'],
  ['ظ', 'z'],
  ['ع', ''],
  ['غ', 'gh'],
  ['ف', 'f'],
  ['ق', 'q'],
  ['ك', 'k'],
  ['ل', 'l'],
  ['م', 'm'],
  ['ن', 'n'],
  ['ه', 'h'],
  ['و', 'w'],
  ['ي', 'y'],
  ['ء', ''],
  ['پ', 'p'],
  ['چ', 'ch'],
  ['ژ', 'zh'],
  ['گ', 'g'],
  ['ڤ', 'v'],
  ['ک', 'k'],
  ['ی', 'y'],
]);

/** Rewrites every Arabic letter as its romanization, leaving the rest alone. */
function romanize(nass: string): string {
  let makhraj = '';
  for (const harf of nass) {
    makhraj += NAQL_HARFI.get(harf) ?? harf;
  }
  return makhraj;
}

/**
 * Collapses the digraphs in one pass.
 *
 * A single left-to-right scan rather than a chain of `String.replace` calls,
 * because chaining lets an earlier rule's output feed a later rule's input:
 * `sh` → `s` followed by `ss` → `s` turns `mishshi` into something no query
 * produces. One pass consumes each input span exactly once.
 */
function idghamThunaiyat(nass: string): string {
  let makhraj = '';
  let mawqi = 0;
  while (mawqi < nass.length) {
    let mudgham = false;
    for (const zawj of THUNAIYAT) {
      const [namat, badil] = zawj;
      if (nass.startsWith(namat, mawqi)) {
        makhraj += badil;
        mawqi += namat.length;
        mudgham = true;
        break;
      }
    }
    if (!mudgham) {
      makhraj += nass.charAt(mawqi);
      mawqi += 1;
    }
  }
  return makhraj;
}

/** The five Latin vowels, which carry no identity in an Arabic skeleton. */
const HARAKAT_LATINIYA = new Set(['a', 'e', 'i', 'o', 'u']);

/**
 * The two Arabic letters that are a consonant at the start of a word and a long
 * vowel everywhere else.
 *
 * `سيف` is written `sayf` by one person and `saif` by another; both are three
 * consonants and one of them spells the `ي` and one of them does not. Dropping
 * `w` and `y` away from a word's first position makes the two spellings agree,
 * and keeping them at the first position preserves `yakuza` and `witcher`,
 * where the letter genuinely starts the word.
 */
const SHIBH_SAKIN = new Set(['w', 'y']);

/**
 * The consonant skeleton of a normalized string: the bridge between scripts.
 *
 * `قلب` and `qalb` both reduce to `qlb`, which is the whole point — a user who
 * only has a Latin keyboard can find an Arabic-titled game, and a user typing
 * Arabic can find a Latin-titled one whose Arabic name the registry knows.
 *
 * Digits survive untouched, so `4` in a skeleton is still `4` and the Roman
 * numeral folding upstream still pays off here.
 *
 * @param muwahhad a string already through {@link wahhid}
 */
export function hikalHarfi(muwahhad: string): string {
  const mudgham = idghamThunaiyat(romanize(muwahhad));
  const kalimat = mudgham.split(' ');
  const makhraj: string[] = [];
  for (const kalima of kalimat) {
    let asas = '';
    for (let mawqi = 0; mawqi < kalima.length; mawqi += 1) {
      const harf = kalima.charAt(mawqi);
      if (HARAKAT_LATINIYA.has(harf)) {
        continue;
      }
      if (SHIBH_SAKIN.has(harf) && asas.length > 0) {
        continue;
      }
      asas += harf;
    }
    if (asas.length > 0) {
      makhraj.push(asas);
    }
  }
  return makhraj.join(' ');
}

/**
 * The first letter of each word, joined: `final fantasy tactics` → `fft`.
 *
 * Users abbreviate long titles and expect the abbreviation to work. Without
 * this, `fft` reaches the right game only as a subsequence match, ranked below
 * every game that happens to contain an `f`, an `f` and a `t` in that order.
 */
function awailKalimat(muwahhad: string): string {
  let makhraj = '';
  for (const kalima of muwahhad.split(' ')) {
    const awwal = kalima.charAt(0);
    if (awwal.length > 0) {
      makhraj += awwal;
    }
  }
  return makhraj;
}

/* ==========================================================================
   The index.
   ========================================================================== */

/** How much a match in one field is worth relative to the primary title. */
const WAZN_HAQL = {
  /** The launcher's own name for the game, which is what the card shows. */
  asli: 1,
  /** The registry's Arabic title. */
  arabi: 0.97,
  /** A series name, an abbreviation, a former title. */
  badil: 0.88,
} as const;

/** One searchable form of one field. */
interface HaqlFahras {
  /** The normalized text. */
  readonly nass: string;
  /** The same text with its Roman numerals written as decimals. */
  readonly adad: string;
  /** The consonant skeleton of the folded form. */
  readonly hikal: string;
  /** Its word initials. */
  readonly awail: string;
  /** What a match here is worth. */
  readonly wazn: number;
}

/**
 * Everything about one game that a query is compared against, computed once.
 *
 * This is the reason the search needs no debounce. Building it costs four
 * normalizations and two skeletons per game — real work, done once when the
 * library loads — and every keystroke afterwards is `indexOf` over strings that
 * are already in their final form. Recomputing per keystroke instead would
 * allocate roughly forty thousand strings for a ten-thousand-game library
 * between one letter and the next, which is where a search like this stops
 * feeling instant.
 */
export interface FahrasBahth {
  /** The game this index belongs to. */
  readonly muarrif: string;
  /** Every field, in the order they are scored. */
  readonly huqul: readonly HaqlFahras[];
}

/**
 * Builds one field's four forms.
 *
 * The skeleton is taken from the numeral-folded text rather than the raw
 * normalized text, so that a digit is a digit on both sides of the bridge: a
 * skeleton built before folding would hold `v` for a title's `IV` and `4` for
 * the query's, and the two would never meet.
 */
function ansha_haql(khaam: string, wazn: number): HaqlFahras | null {
  const nass = wahhid(khaam);
  if (nass.length === 0) {
    return null;
  }
  const adad = wahhidAdad(nass);
  return { nass, adad, hikal: hikalHarfi(adad), awail: awailKalimat(nass), wazn };
}

/**
 * Builds the search index for one game.
 *
 * @param sijill the game record
 */
export function ansha_fahras(sijill: SijillLuba): FahrasBahth {
  const huqul: HaqlFahras[] = [];

  const asli = ansha_haql(sijill.ism, WAZN_HAQL.asli);
  if (asli !== null) {
    huqul.push(asli);
  }

  if (sijill.ism_arabi !== null) {
    const arabi = ansha_haql(sijill.ism_arabi, WAZN_HAQL.arabi);
    // Skipped when it normalizes to the same text as the launcher's name, which
    // happens for every Arabic-titled game the registry did not rename. Scoring
    // one string twice would rank those games above equally good matches for no
    // reason a user could see.
    if (arabi !== null && (asli === null || arabi.nass !== asli.nass)) {
      huqul.push(arabi);
    }
  }

  for (const badil of sijill.asma_badila) {
    const haql = ansha_haql(badil, WAZN_HAQL.badil);
    if (haql !== null && !huqul.some((mawjud) => mawjud.nass === haql.nass)) {
      huqul.push(haql);
    }
  }

  return { muarrif: sijill.muarrif, huqul };
}

/**
 * The index for a whole library, keyed by game identity.
 *
 * A `Map` rather than a field on the record because the records come from the
 * query cache and are replaced wholesale on every refresh: attaching the index
 * to them would rebuild it on every refresh, including the ones that changed
 * nothing but a play time.
 *
 * @param sijillat every game in the library
 * @param sabiq the previous index, whose entries are reused where the game is
 *   unchanged
 */
export function ansha_fahrasat(
  sijillat: readonly SijillLuba[],
  sabiq?: ReadonlyMap<string, FahrasBahth>,
): Map<string, FahrasBahth> {
  const jadeed = new Map<string, FahrasBahth>();
  for (const sijill of sijillat) {
    const mawjud = sabiq?.get(sijill.muarrif);
    // Reused only when every searchable field is identical. A game whose size
    // or play time changed keeps its index; a game that was renamed does not.
    if (mawjud !== undefined && yatatabaqFahras(mawjud, sijill)) {
      jadeed.set(sijill.muarrif, mawjud);
      continue;
    }
    jadeed.set(sijill.muarrif, ansha_fahras(sijill));
  }
  return jadeed;
}

/** Whether an existing index still describes this record's searchable text. */
function yatatabaqFahras(fahras: FahrasBahth, sijill: SijillLuba): boolean {
  // Normalized once: this runs per game per rescan, and `wahhid` is four passes
  // over the string plus an allocation for each of them.
  const ism = wahhid(sijill.ism);
  const awwal = fahras.huqul.at(0);
  if (awwal === undefined) {
    return ism.length === 0;
  }
  if (awwal.nass !== ism) {
    return false;
  }
  const adad = 1 + (sijill.ism_arabi === null ? 0 : 1) + sijill.asma_badila.length;
  // A loose count check rather than a full re-derivation: the expensive part is
  // the skeleton, and a title that kept its primary name while gaining an
  // alternate is rare enough that rebuilding it is cheaper than proving it.
  return fahras.huqul.length <= adad;
}

/* ==========================================================================
   Scoring.
   ========================================================================== */

/**
 * The five match qualities, as scores rather than as an enum.
 *
 * Numbers because the field weight and the lens weight multiply into them and
 * the length bonus adds to them: two games can both match by prefix and the
 * shorter one has to win. The gaps between the tiers are wide enough that no
 * combination of weights lets a lower tier overtake a higher one — a substring
 * hit at full weight is 480 and a word-boundary hit at the weakest possible
 * weight is 640 × 0.88 × 0.8 = 450, which is the tightest pair and still
 * ordered correctly against a same-lens comparison.
 */
export const DARAJAT = {
  /** The field is the query, exactly. */
  mutabiq: 1000,
  /** The field starts with the query. */
  bidaya: 820,
  /** The query is the field's word initials: `fft` for Final Fantasy Tactics. */
  ikhtisar: 760,
  /** The query starts at a word boundary inside the field. */
  hadd_kalima: 640,
  /** The query appears somewhere in the field. */
  tadmeen: 480,
  /** The query's letters appear in order, with other letters between them. */
  mutatabi: 260,
} as const;

/** What a match through the skeleton is worth against a direct one. */
const WAZN_HIKAL = 0.8;

/** The position of a subsequence match, and how tightly packed it was. */
interface TatabuMutatabi {
  /** Where the first letter landed. */
  readonly bidaya: number;
  /** How many characters the whole match spanned. */
  readonly imtidad: number;
}

/**
 * Whether every character of the query appears in the haystack in order.
 *
 * Greedy from the left rather than optimal, because the optimal answer needs
 * dynamic programming over both strings and this runs once per field per game
 * per keystroke. The greedy span is a slight over-estimate on pathological
 * inputs and identical on everything a game title actually contains.
 */
function tatabbu(bahth: string, kawm: string): TatabuMutatabi | null {
  if (bahth.length === 0) {
    return null;
  }
  let mawqiBahth = 0;
  let bidaya = -1;
  for (let mawqi = 0; mawqi < kawm.length; mawqi += 1) {
    if (kawm.charAt(mawqi) !== bahth.charAt(mawqiBahth)) {
      continue;
    }
    if (bidaya < 0) {
      bidaya = mawqi;
    }
    mawqiBahth += 1;
    if (mawqiBahth === bahth.length) {
      return { bidaya, imtidad: mawqi - bidaya + 1 };
    }
  }
  return null;
}

/**
 * The raw quality of one query term against one haystack, before weighting.
 *
 * Zero when the term does not match at all, which is what lets the caller treat
 * a single unmatched term as disqualifying the whole record.
 */
function darajatKawm(bahth: string, kawm: string, awail: string): number {
  if (kawm.length === 0 || bahth.length === 0) {
    return 0;
  }
  if (kawm === bahth) {
    return DARAJAT.mutabiq;
  }
  if (kawm.startsWith(bahth)) {
    // Longer titles are penalised so that `dark` puts `Dark Souls` above
    // `Darkest Dungeon: The Colour of Madness`, which is the ordering a user
    // typing four letters expects.
    return DARAJAT.bidaya + qismatTul(bahth.length, kawm.length) * 60;
  }
  if (awail.length > 1 && awail === bahth) {
    return DARAJAT.ikhtisar;
  }
  if (awail.length > 1 && awail.startsWith(bahth) && bahth.length > 1) {
    return DARAJAT.ikhtisar - 40;
  }

  const mawqi = kawm.indexOf(bahth);
  if (mawqi > 0) {
    const sabiq = kawm.charAt(mawqi - 1);
    if (sabiq === ' ') {
      return DARAJAT.hadd_kalima + qismatTul(bahth.length, kawm.length) * 40;
    }
    return DARAJAT.tadmeen + qismatTul(bahth.length, kawm.length) * 30;
  }

  const mutatabi = tatabbu(bahth, kawm);
  if (mutatabi === null) {
    return 0;
  }
  // A subsequence is only weak evidence, so it is scored by how compact it was:
  // `ff` inside `final fantasy` spans seven characters and is worth far more
  // than the same two letters spread across `Stuff Goes Off`.
  const kathafa = bahth.length / Math.max(1, mutatabi.imtidad);
  const qurb = 1 - Math.min(1, mutatabi.bidaya / Math.max(1, kawm.length));
  return DARAJAT.mutatabi * (0.5 + 0.35 * kathafa + 0.15 * qurb);
}

/** How much of the haystack the query accounts for, clamped to 0…1. */
function qismatTul(tulBahth: number, tulKawm: number): number {
  if (tulKawm <= 0) {
    return 0;
  }
  return Math.min(1, tulBahth / tulKawm);
}

/** The three forms one query term is compared in. */
interface HadTalab {
  /** The normalized term. */
  readonly nass: string;
  /** The same term with its Roman numerals written as decimals. */
  readonly adad: string;
  /** The folded term's consonant skeleton. */
  readonly hikal: string;
}

/** Builds the three comparison forms of one already-normalized term. */
function hadTalab(nass: string): HadTalab {
  const adad = wahhidAdad(nass);
  return { nass, adad, hikal: hikalHarfi(adad) };
}

/**
 * The best score one query term reaches against one field.
 *
 * All three lenses are tried and the best one wins, rather than falling through
 * to the next only when the previous failed. A query can match a title weakly
 * as a subsequence and strongly through the skeleton — `qlb` against `قلب` is
 * exactly that — and taking the first non-zero answer would return the
 * subsequence score and bury the game.
 *
 * The numeral-folded lens carries full weight because it is an exact identity
 * rather than an approximation: `IV` and `4` are the same number, and a title
 * found through them is not a fuzzier match than one found by its letters. The
 * skeleton is the only weakened lens.
 */
function darajatHaql(hadd: HadTalab, haql: HaqlFahras): number {
  let afdal = darajatKawm(hadd.nass, haql.nass, haql.awail);

  if (hadd.adad !== hadd.nass || haql.adad !== haql.nass) {
    afdal = Math.max(afdal, darajatKawm(hadd.adad, haql.adad, haql.awail));
  }

  if (hadd.hikal.length > 0) {
    afdal = Math.max(afdal, darajatKawm(hadd.hikal, haql.hikal, haql.awail) * WAZN_HIKAL);
  }

  return afdal * haql.wazn;
}

/** A query, prepared once per keystroke rather than once per game. */
interface IstifsarMuhaddar {
  /** The whole query, in its three forms, for the phrase score. */
  readonly kamil: HadTalab;
  /** Each whitespace-separated term, in its three forms. */
  readonly hudud: readonly HadTalab[];
}

/**
 * Normalizes a raw query into the forms the scorer compares against.
 *
 * Exported because the grid holds the prepared query in state: preparing it
 * inside the search would redo four normalizations for every call, and the
 * search is called again whenever a filter changes even though the query did
 * not.
 *
 * @param khaam whatever is in the search field right now
 */
export function haddirIstifsar(khaam: string): IstifsarMuhaddar | null {
  const kamil = wahhid(khaam);
  if (kamil.length === 0) {
    return null;
  }
  const hudud = kamil
    .split(' ')
    .filter((hadd) => hadd.length > 0)
    .map((hadd) => hadTalab(hadd));
  return { kamil: hadTalab(kamil), hudud };
}

/**
 * How well one game matches a prepared query. Zero means it does not.
 *
 * Every term must match something, and the record's score is the mean of the
 * per-term bests. `AND` across terms rather than `OR` because a user typing two
 * words is narrowing, not widening: `final 4` should return one game, not every
 * game with a four in it.
 *
 * The whole query is scored as a phrase too, and the better of the two answers
 * wins, so an exact title match is never beaten by the average of its own
 * words.
 */
export function qayyim(istifsar: IstifsarMuhaddar, fahras: FahrasBahth): number {
  if (fahras.huqul.length === 0) {
    return 0;
  }

  let jumla = 0;
  for (const haql of fahras.huqul) {
    jumla = Math.max(jumla, darajatHaql(istifsar.kamil, haql));
  }

  if (istifsar.hudud.length <= 1) {
    return jumla;
  }

  let majmu = 0;
  for (const hadd of istifsar.hudud) {
    let afdal = 0;
    for (const haql of fahras.huqul) {
      afdal = Math.max(afdal, darajatHaql(hadd, haql));
    }
    if (afdal === 0) {
      // One term nobody matched disqualifies the record outright, even if the
      // phrase scored: a game that matches `final` and not `4` is not what the
      // user asked for.
      return 0;
    }
    majmu += afdal;
  }

  return Math.max(jumla, majmu / istifsar.hudud.length);
}

/** A game and how well it matched. */
export interface NatijatBahth {
  /** The matching game. */
  readonly sijill: SijillLuba;
  /** Its score, higher being better. */
  readonly daraja: number;
}

/**
 * Every game matching a query, best first.
 *
 * @param sijillat the library, already filtered
 * @param fahrasat the index, keyed by game identity
 * @param istifsar the prepared query, or null when the field is empty
 */
export function bahth(
  sijillat: readonly SijillLuba[],
  fahrasat: ReadonlyMap<string, FahrasBahth>,
  istifsar: IstifsarMuhaddar | null,
): NatijatBahth[] {
  if (istifsar === null) {
    return sijillat.map((sijill) => ({ sijill, daraja: 0 }));
  }
  const nataij: NatijatBahth[] = [];
  for (const sijill of sijillat) {
    const fahras = fahrasat.get(sijill.muarrif);
    if (fahras === undefined) {
      continue;
    }
    const daraja = qayyim(istifsar, fahras);
    if (daraja > 0) {
      nataij.push({ sijill, daraja });
    }
  }
  return nataij;
}

/* ==========================================================================
   Filtering.
   ========================================================================== */

/**
 * The five filter categories, each a set of accepted values.
 *
 * An empty category is not a filter that rejects everything — it is the absence
 * of a constraint. Modelled that way because the interface's "no launcher
 * selected" and "every launcher selected" are the same library, and a user who
 * unticks the last launcher expects their games back rather than an empty grid.
 */
export interface TasfiyatMaktaba {
  /** Accepted launcher families. */
  readonly manassat: readonly Manassa[];
  /** Accepted engine families. */
  readonly muharrikat: readonly AilatMuharrik[];
  /** Accepted Arabization states. */
  readonly halat: readonly HalatLuba[];
  /** Accepted voice-pack states. */
  readonly sawt: readonly HalatRuqaa[];
  /** Accepted injection tiers. */
  readonly tabaqat: readonly Tabaqa[];
}

/** No constraint in any category: the whole library. */
export const TASFIYA_FARIGHA: TasfiyatMaktaba = {
  manassat: [],
  muharrikat: [],
  halat: [],
  sawt: [],
  tabaqat: [],
};

/** Whether any category constrains anything at all. */
export function tasfiyaNashita(murashshih: TasfiyatMaktaba): boolean {
  return (
    murashshih.manassat.length > 0 ||
    murashshih.muharrikat.length > 0 ||
    murashshih.halat.length > 0 ||
    murashshih.sawt.length > 0 ||
    murashshih.tabaqat.length > 0
  );
}

/** How many individual values are selected across every category. */
export function adadTasfiya(murashshih: TasfiyatMaktaba): number {
  return (
    murashshih.manassat.length +
    murashshih.muharrikat.length +
    murashshih.halat.length +
    murashshih.sawt.length +
    murashshih.tabaqat.length
  );
}

/** An empty category accepts everything; a non-empty one accepts its members. */
function yaqbal<T>(maqbula: readonly T[], qeema: T): boolean {
  return maqbula.length === 0 || maqbula.includes(qeema);
}

/**
 * Applies the filters: `AND` across categories, `OR` within one.
 *
 * Steam **and** Unity means both; Steam **or** GOG means either. That is the
 * only combination that reads correctly from a panel of tick boxes, and it is
 * the one every user of one has already learned somewhere else.
 *
 * Rebuilt as a new array rather than mutated, because the grid's virtualizer
 * keys off identity to know that its measurement cache is stale.
 *
 * @param sijillat the library
 * @param murashshih the active filters
 */
export function tasfiya(
  sijillat: readonly SijillLuba[],
  murashshih: TasfiyatMaktaba,
): SijillLuba[] {
  if (!tasfiyaNashita(murashshih)) {
    return [...sijillat];
  }
  return sijillat.filter(
    (sijill) =>
      yaqbal(murashshih.manassat, sijill.manassa) &&
      yaqbal(murashshih.muharrikat, sijill.muharrik) &&
      yaqbal(murashshih.halat, sijill.hala) &&
      yaqbal(murashshih.sawt, sijill.sawt) &&
      yaqbal(murashshih.tabaqat, sijill.tabaqa),
  );
}

/* ==========================================================================
   Sorting.
   ========================================================================== */

/** The five orders the library can be read in. */
export type TartibMaktaba = 'ism' | 'akhir_laab' | 'akhir_tathbeet' | 'hajm' | 'ruqaa';

/** Which way an order runs. */
export type IttijahFarz = 'tasaudi' | 'tanazuli';

/**
 * The direction each order defaults to.
 *
 * Names read A-to-Z and everything else reads newest, largest or most
 * actionable first, because those are the answers the order was chosen to get.
 * A "recently played" list that started at the game somebody played once in
 * 2019 would be a list nobody chose.
 */
export const ITTIJAH_IFTIRADI: Readonly<Record<TartibMaktaba, IttijahFarz>> = {
  ism: 'tasaudi',
  akhir_laab: 'tanazuli',
  akhir_tathbeet: 'tanazuli',
  hajm: 'tanazuli',
  ruqaa: 'tanazuli',
};

/**
 * How actionable each Arabization state is, for the patch-availability order.
 *
 * `mutaha` outranks `mutabbaqa` deliberately: a patch waiting to be installed
 * is something the user can act on right now, and a patch already installed is
 * not. The order exists to surface work, not to celebrate finished work.
 */
const RUTBAT_HALA: Readonly<Record<HalatLuba, number>> = {
  mutaha: 5,
  mutaha_ghayr_mutabiqa: 4,
  mutabbaqa: 3,
  madum_bila_ruqaa: 2,
  tabaqa_faqat: 1,
  marfuda: 0,
};

/**
 * The collators, one per locale, built once.
 *
 * `Intl.Collator` construction is expensive enough to show up in a profile when
 * it happens inside a comparison function, and a ten-thousand-game sort calls
 * its comparator well over a hundred thousand times.
 *
 * `numeric` is on so `Rogue 2` sorts before `Rogue 10`, and sensitivity is
 * `base` so that a title differing only in case or in an accent does not sort
 * into a different place than the user reads it.
 */
const MURATTIBAT = new Map<string, Intl.Collator>();

/** The collator for a language, built on first use. */
function murattib(lugha: Lugha): Intl.Collator {
  const mahalli = wasm(lugha);
  const mawjud = MURATTIBAT.get(mahalli);
  if (mawjud !== undefined) {
    return mawjud;
  }
  const jadeed = new Intl.Collator(mahalli, {
    numeric: true,
    sensitivity: 'base',
    ignorePunctuation: true,
  });
  MURATTIBAT.set(mahalli, jadeed);
  return jadeed;
}

/**
 * Compares two nullable timestamps, with null always last.
 *
 * Null last in *both* directions, which is not what a naive numeric compare
 * does. A game never played has no place in a "recently played" order at
 * either end, and putting it first when the order is reversed would fill the
 * top of the list with games the user has never opened.
 */
function qaranWaqt(awwal: number | null, thani: number | null, ittijahFarz: IttijahFarz): number {
  if (awwal === null && thani === null) {
    return 0;
  }
  if (awwal === null) {
    return 1;
  }
  if (thani === null) {
    return -1;
  }
  return ittijahFarz === 'tasaudi' ? awwal - thani : thani - awwal;
}

/**
 * Two games' identities, compared by code unit.
 *
 * The last tiebreak of every order, and the reason the order is *total* rather
 * than merely stable. `Array.prototype.sort` preserves the input order of equal
 * elements, and the input order is a fresh array built from a fresh scan — a
 * launcher that enumerated its catalogue in a different order this time hands
 * the comparator the same two games the other way round, and a stable sort
 * faithfully preserves the new order. The library visibly reshuffles.
 *
 * Two games can compare equal on every visible key: the name comparison is a
 * collator at `base` sensitivity, so a title differing only in case or in an
 * accent scores zero, and the same game owned on two launchers has the same
 * name on both. The identity is the only key that is unique, stable across
 * rescans, and independent of the response's order.
 *
 * Compared by code unit rather than through the collator on purpose. An
 * identity is an opaque key, not text somebody reads, and the collator's
 * `ignorePunctuation` and `base` sensitivity are both ways for two distinct
 * keys to compare equal — which is the one thing a final tiebreak may not do.
 *
 * Never reversed with the chosen direction, because it is not part of the
 * order: it exists to pin rows the user cannot tell apart, and reversing it
 * would make the pinning itself depend on the direction.
 */
function bilHawiya(awwal: SijillLuba, thani: SijillLuba): number {
  if (awwal.muarrif < thani.muarrif) {
    return -1;
  }
  return awwal.muarrif > thani.muarrif ? 1 : 0;
}

/**
 * Sorts the library.
 *
 * Every order falls back to the name, through the same collator, and then to
 * the identity, so the result is total: two games of identical size, identical
 * patch state or identical name must not swap places between one render and the
 * next, and `Array.prototype.sort` being stable is not enough when the input
 * array itself is rebuilt from a new query response in a different order.
 *
 * @param sijillat the games to order
 * @param tartib which order
 * @param ittijahFarz which way it runs
 * @param lugha the session's language, for collation
 */
export function farz(
  sijillat: readonly SijillLuba[],
  tartib: TartibMaktaba,
  ittijahFarz: IttijahFarz,
  lugha: Lugha,
): SijillLuba[] {
  const muqaran = murattib(lugha);
  const bilIsm = (awwal: SijillLuba, thani: SijillLuba): number => {
    const natija = muqaran.compare(awwal.ism, thani.ism);
    return natija === 0 ? bilHawiya(awwal, thani) : natija;
  };

  const nusakh = [...sijillat];
  nusakh.sort((awwal, thani) => {
    switch (tartib) {
      case 'ism': {
        const natija = muqaran.compare(awwal.ism, thani.ism);
        // The identity breaks the tie outside the reversal, so two games with
        // the same name keep one relative order in both directions.
        if (natija === 0) {
          return bilHawiya(awwal, thani);
        }
        return ittijahFarz === 'tasaudi' ? natija : -natija;
      }
      case 'akhir_laab': {
        const natija = qaranWaqt(awwal.akhir_laab, thani.akhir_laab, ittijahFarz);
        return natija === 0 ? bilIsm(awwal, thani) : natija;
      }
      case 'akhir_tathbeet': {
        const natija = qaranWaqt(awwal.akhir_tathbeet, thani.akhir_tathbeet, ittijahFarz);
        return natija === 0 ? bilIsm(awwal, thani) : natija;
      }
      case 'hajm': {
        const farq = ittijahFarz === 'tasaudi' ? awwal.hajm - thani.hajm : thani.hajm - awwal.hajm;
        return farq === 0 ? bilIsm(awwal, thani) : farq;
      }
      case 'ruqaa': {
        const alif = RUTBAT_HALA[awwal.hala];
        const ba = RUTBAT_HALA[thani.hala];
        const farq = ittijahFarz === 'tasaudi' ? alif - ba : ba - alif;
        return farq === 0 ? bilIsm(awwal, thani) : farq;
      }
    }
  });
  return nusakh;
}

/* ==========================================================================
   Grouping.
   ========================================================================== */

/** Either one merged library, or one section per launcher. */
export type TajmiMaktaba = 'mudmaj' | 'manassa';

/** One section of the grid: a heading and the games under it. */
export interface MajmuatMaktaba {
  /**
   * A stable key for the section. `''` for the single merged section.
   *
   * The grouping key itself — a {@link Manassa} when the grid is sectioned by
   * launcher — so the heading beside it is always a name for *this* key.
   */
  readonly muarrif: string;
  /** The heading, already localized, or null for the merged view. */
  readonly unwan: string | null;
  /** The games in this section, in the order they were sorted into. */
  readonly sijillat: readonly SijillLuba[];
}

/**
 * Splits the ordered library into sections.
 *
 * The merged view returns a single section with a null heading rather than a
 * bare array, so the grid renders one code path instead of two. A grid with a
 * separate "grouped" branch is a grid whose keyboard navigation is written
 * twice and gets fixed once.
 *
 * Sections keep the order their first game appeared in, which means the section
 * order follows the sort: a library ordered by recency puts the launcher the
 * user played on last at the top, and that is the right answer without a second
 * rule for section ordering.
 *
 * The heading is taken from the first game's {@link SijillLuba.ism_manassa},
 * which that field's contract requires to be the name of `manassa` rather than
 * of whichever launcher reported that particular game. It is the only field
 * carrying a localized launcher name and the sections are keyed by `manassa`,
 * so the two agree by construction — but only as far as the contract holds,
 * which is why it is stated on the field and repeated here.
 *
 * @param sijillat the games, already filtered, searched and sorted
 * @param tajmi which grouping
 */
export function jammi(
  sijillat: readonly SijillLuba[],
  tajmi: TajmiMaktaba,
): MajmuatMaktaba[] {
  if (tajmi === 'mudmaj') {
    return [{ muarrif: '', unwan: null, sijillat: [...sijillat] }];
  }

  const aqsam = new Map<Manassa, { unwan: string; sijillat: SijillLuba[] }>();
  for (const sijill of sijillat) {
    const mawjud = aqsam.get(sijill.manassa);
    if (mawjud === undefined) {
      aqsam.set(sijill.manassa, { unwan: sijill.ism_manassa, sijillat: [sijill] });
      continue;
    }
    mawjud.sijillat.push(sijill);
  }

  const makhraj: MajmuatMaktaba[] = [];
  for (const [muarrif, qism] of aqsam) {
    makhraj.push({ muarrif, unwan: qism.unwan, sijillat: qism.sijillat });
  }
  return makhraj;
}

/* ==========================================================================
   The whole pipeline.
   ========================================================================== */

/** Everything that decides what the grid shows. */
export interface TalabMaktaba {
  /** The raw contents of the search field. */
  readonly bahth: string;
  /** The active filters. */
  readonly murashshih: TasfiyatMaktaba;
  /** The chosen order. */
  readonly tartib: TartibMaktaba;
  /** Which way that order runs. */
  readonly ittijah: IttijahFarz;
  /** Whether the grid is sectioned by launcher. */
  readonly tajmi: TajmiMaktaba;
}

/** What the grid renders, and the numbers the header reports. */
export interface NatijatMaktaba {
  /** The sections, in order. */
  readonly majmuat: readonly MajmuatMaktaba[];
  /** How many games survived, across every section. */
  readonly adad: number;
  /** How many games the library holds before any filter or query. */
  readonly adadKulli: number;
  /**
   * Whether a query or a filter is narrowing a library that has something in
   * it.
   *
   * The second half is what makes it usable as the grid's empty-state
   * discriminator. A machine with no games at all is the library's most-seen
   * state, and a filter left ticked from a previous session survives a restart
   * — so "a filter is set" alone is true on a first run with nothing installed,
   * and the grid would answer an empty machine with "nothing matches what you
   * chose". Nothing was chosen away. There was nothing there.
   */
  readonly munaqqa: boolean;
}

/**
 * Filter, then search, then sort, then group — in that order, for a reason.
 *
 * Filtering first is what keeps the search cheap: the scorer is the expensive
 * step and running it over a set a launcher filter has already cut to a tenth
 * is a tenth of the work. Sorting last is what lets a query override the
 * chosen order without the order having to know about the query.
 *
 * When a query is active the score is the primary key and the user's chosen
 * order is the tiebreak. Sorting purely by score would shuffle the ties on
 * every keystroke — hundreds of games can share a subsequence score — and
 * sorting purely by the chosen order would bury the exact match the user is
 * looking straight at.
 *
 * @param sijillat the whole library
 * @param fahrasat the search index
 * @param talab everything the user has selected
 * @param lugha the session's language, for collation
 */
export function nasseq(
  sijillat: readonly SijillLuba[],
  fahrasat: ReadonlyMap<string, FahrasBahth>,
  talab: TalabMaktaba,
  lugha: Lugha,
): NatijatMaktaba {
  const istifsar = haddirIstifsar(talab.bahth);
  const munaqqa =
    sijillat.length > 0 && (istifsar !== null || tasfiyaNashita(talab.murashshih));

  const baada_tasfiya = tasfiya(sijillat, talab.murashshih);
  const nataij = bahth(baada_tasfiya, fahrasat, istifsar);

  let murattaba: SijillLuba[];
  if (istifsar === null) {
    murattaba = farz(
      nataij.map((natija) => natija.sijill),
      talab.tartib,
      talab.ittijah,
      lugha,
    );
  } else {
    const rutab = new Map<string, number>();
    for (const natija of nataij) {
      rutab.set(natija.sijill.muarrif, natija.daraja);
    }
    const bilTartib = farz(
      nataij.map((natija) => natija.sijill),
      talab.tartib,
      talab.ittijah,
      lugha,
    );
    // Stable, so the sort above survives inside each score band.
    murattaba = bilTartib
      .map((sijill, martaba) => ({ sijill, martaba }))
      .sort((awwal, thani) => {
        const alif = rutab.get(awwal.sijill.muarrif) ?? 0;
        const ba = rutab.get(thani.sijill.muarrif) ?? 0;
        return ba === alif ? awwal.martaba - thani.martaba : ba - alif;
      })
      .map((zawj) => zawj.sijill);
  }

  return {
    majmuat: jammi(murattaba, talab.tajmi),
    adad: murattaba.length,
    adadKulli: sijillat.length,
    munaqqa,
  };
}

/**
 * Every launcher family present in the library, in first-seen order.
 *
 * The empty state names the launchers that were searched, and it has to name
 * the ones this machine actually has rather than the fifteen the product
 * supports: telling a user with Steam and GOG installed that Taarib searched
 * Battle.net and Riot is telling them about launchers they do not have, which
 * makes the sentence read as boilerplate.
 *
 * @param sijillat the whole library, unfiltered
 */
export function manassatHadira(
  sijillat: readonly SijillLuba[],
): { readonly manassa: Manassa; readonly ism: string }[] {
  const maruf = new Map<Manassa, string>();
  for (const sijill of sijillat) {
    if (!maruf.has(sijill.manassa)) {
      maruf.set(sijill.manassa, sijill.ism_manassa);
    }
  }
  return [...maruf].map(([manassa, ism]) => ({ manassa, ism }));
}

/* ==========================================================================
   The games that did not make it into the library.
   ========================================================================== */

/**
 * One game a launcher lists that the existence gate rejected.
 *
 * Carries the launcher's own name for it rather than Taarib's, because Taarib
 * never got far enough to have one, and the user is going to look for this
 * entry in the launcher's list.
 */
export interface LubaGhaiba {
  /** A stable key: the launcher identifier the scan rejected. */
  readonly muarrif: string;
  /** The name the launcher gives it. */
  readonly ism: string;
  /** The launcher family, for the button that opens it. */
  readonly manassa: Manassa;
  /** The launcher's name, already localized. */
  readonly ism_manassa: string;
  /** Why it is not in the library. */
  readonly sabab: SababGhiyab;
}

/** One heading of the unavailable section and the rows under it. */
export interface FiatGhaiba {
  /** Which of the four groups. */
  readonly fia: FiatGhiyab;
  /** The rows, in the order the scan produced them. */
  readonly alab: readonly LubaGhaiba[];
}

/**
 * Which heading each reason belongs under.
 *
 * A direct port of `SababGhiyab::fia` in
 * `crates/taarib-mustalahat/src/ghiyab.rs`. Duplicated rather than sent over the
 * wire because it is a pure function of the discriminant, and a `fia` field on
 * every row would be a second copy of the same fact that a future reason could
 * be added without updating.
 *
 * Written as a table over the discriminant rather than as the `match`'s
 * `switch`, because the two fail differently when Rust gains a reason. A
 * `switch` with no `default` does fail — it falls off its end and the declared
 * return type does not admit `undefined` — but it reports that the *function*
 * can return nothing, which is a sentence about this file rather than about the
 * variant somebody just added. A table reports the missing property by name, in
 * the wire spelling, which is the spelling to search the Rust crate for.
 *
 * The arms of the Rust `match` are kept as the groups below, in its order, so
 * the two can be read side by side.
 */
const FIAT_SABAB: Readonly<Record<SababGhiyab['naw'], FiatGhiyab>> = {
  // The launcher is already fixing it; nothing is asked of the user.
  qayd_tanzil: 'jariya',
  qayd_tahdith: 'jariya',

  // It is here and incomplete, and the user can finish it.
  tanzil_mutawaqqif: 'naqis',
  tathbeet_naqis: 'naqis',
  himl_sahabi: 'naqis',

  // It is not where the launcher says it is, and that needs a decision.
  mujallad_mafqud: 'mafquda',
  mujallad_mamnu: 'mafquda',
  tanfidhi_mafqud: 'mafquda',
  tanfidhi_mukhtalif: 'mafquda',
  beea_mafquda: 'mafquda',
  tanfidhi_kharij_beea: 'mafquda',

  // Nothing is wrong. The volume or the host is simply not here right now.
  qurs_ghayr_muttasil: 'ghayr_muttasila',
  shabaka_ghayr_mutaha: 'ghayr_muttasila',
};

/**
 * Which heading one reason belongs under.
 *
 * @param sabab the reason the gate recorded
 */
export function fiatSabab(sabab: SababGhiyab): FiatGhiyab {
  return FIAT_SABAB[sabab.naw];
}

/**
 * The four headings, in the order the Rust enum declares them.
 *
 * Ordered by how much the user can do about the group: what the launcher is
 * already fixing, then what they can fix, then what needs a decision. Written
 * as a constant rather than derived from the data so that an empty group keeps
 * its place and the section does not reorder itself as a download finishes.
 */
export const TARTEEB_FIAT = [
  'jariya',
  'naqis',
  'mafquda',
  'ghayr_muttasila',
] as const satisfies readonly FiatGhiyab[];

/** A type parameter no non-empty union can be passed as. */
type Mustanfad<T extends never> = T;

/**
 * A compile-time proof that {@link TARTEEB_FIAT} names every group.
 *
 * {@link jammiGhiyab} builds its output by walking that list, so a group
 * missing from it is a group whose rows are silently dropped from the
 * unavailable section — the rejected games would simply not be shown, which is
 * the one thing that section exists to prevent. The `satisfies` above proves
 * every entry is a real group; this proves the converse. While the list is
 * complete the `Exclude` is `never` and this resolves to `never`; the moment a
 * group is added in Rust and not listed, it resolves to that group's own name
 * and fails the constraint with it in the message.
 */
export type ShumulFiatGhiyab = Mustanfad<
  Exclude<FiatGhiyab, (typeof TARTEEB_FIAT)[number]>
>;

/**
 * Groups the rejected games under their four headings, dropping empty ones.
 *
 * Mirrors `SijillWujud::ghaiba` on the Rust side, which produces the same four
 * buckets for the diagnostics report. The interface does it again rather than
 * asking for the grouped form because the grouping is one table lookup and the
 * wire shape is a flat list the store already has.
 *
 * Walks {@link TARTEEB_FIAT} rather than the buckets it built, so an empty
 * group keeps its place in the order. {@link ShumulFiatGhiyab} is what keeps
 * that safe: a group absent from the list would be a group of rejected games
 * that never reached the screen.
 *
 * @param alab every rejected game
 */
export function jammiGhiyab(alab: readonly LubaGhaiba[]): FiatGhaiba[] {
  const dilaa = new Map<FiatGhiyab, LubaGhaiba[]>();
  for (const luba of alab) {
    const fia = fiatSabab(luba.sabab);
    const mawjud = dilaa.get(fia);
    if (mawjud === undefined) {
      dilaa.set(fia, [luba]);
      continue;
    }
    mawjud.push(luba);
  }

  const makhraj: FiatGhaiba[] = [];
  for (const fia of TARTEEB_FIAT) {
    const alabFia = dilaa.get(fia);
    if (alabFia !== undefined && alabFia.length > 0) {
      makhraj.push({ fia, alab: alabFia });
    }
  }
  return makhraj;
}
