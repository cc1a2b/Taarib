/**
 * تعريب — the in-page runtime for Electron and NW.js games.
 *
 * This file is deliberately small, and the reason is the same one that keeps
 * `electron.rs` short: **the renderer is Chromium.** Blink lays the text out,
 * HarfBuzz shapes it, and the full Unicode bidirectional algorithm reorders it —
 * all three better than anything shipped here would. So for very nearly every
 * game this runtime touches, the whole job is three things:
 *
 *  1. **الاتجاه** — tell the document it is right-to-left, and keep telling
 *     containers the game creates afterwards.
 *  2. **الخط** — install an Arabic font the game did not ship, from a data URL.
 *  3. **الترجمة** — replace the strings the patch carries, including in text
 *     that appears after load.
 *
 * That is rung one (`Rutba.Idad`), and it is what the tier probe expects to
 * report. The text stays real text: selectable, searchable, copyable, and
 * legible to a screen reader. Nothing below that line is given up.
 *
 * ## The one exception, and why it is gated
 *
 * A game that paints its dialogue onto a `<canvas>` with `fillText` is *still
 * shaped* — canvas text goes through the same shaper the layout engine uses. A
 * game that draws each character as its own sprite, blits a bitmap font, or
 * bundles its own layout library gets no shaping at all, and that is the only
 * case that needs the takeover in `rakkibLawha`. It is gated on the rung the
 * patch recorded, never on what this file finds at runtime, because installing
 * a canvas takeover on a game whose canvas text was already correct is a
 * regression that produces plausible-looking output — the worst kind.
 *
 * ## Data never becomes code
 *
 * There is no `eval` in this file, no `new Function`, no
 * `setTimeout('...')`, and no assignment of patch content to `innerHTML`,
 * `outerHTML` or `insertAdjacentHTML`. Translations reach the document through
 * `textContent` and through attribute setters, both of which are inert. The
 * string table is parsed with `JSON.parse` and is never anything but data. This
 * is not a stylistic preference: a patch is a file a player downloaded, and a
 * patch format in which a translated string can execute is a patch format that
 * ships a code-execution bug to everyone who installs one.
 *
 * ## No network, ever
 *
 * The font arrives as a `data:` URL embedded in the payload. No `fetch`, no
 * `XMLHttpRequest`, no remote stylesheet, no font CDN. A patch that made the
 * game phone home would have changed the game's network behaviour, and that is
 * not a change a translation is entitled to make. It also means the font works
 * offline, on a `file://` origin, and under a content security policy that
 * forbids remote font sources — which many Electron games have.
 */

/** Which world this copy of the runtime is running in. */
export type Nitaq = 'maazul' | 'safha';

/** The rung the patch compiler recorded. Mirrors `tabaqa::Rutba`. */
export const RUTBA_IDAD = 1;
/** Corrected shaping: the engine shapes and lays out wrongly. */
export const RUTBA_TASHEEH = 2;
/** Full takeover: the engine cannot shape and Taarib draws. */
export const RUTBA_ISTILA = 3;

/** The font the patch ships, as a family name and a data URL. */
export interface KhattHamula {
  /** The family name this runtime registers the font under. */
  readonly ism: string;
  /** A `data:` URL. Never an `http(s):` or `file:` one — see the header. */
  readonly rabt: string;
}

/** Everything the patcher put in `taarib/hamula.json`. */
export interface Hamula {
  /** Payload format version. This build reads 1. */
  readonly isdar: number;
  /** The recorded rung: 1, 2 or 3. */
  readonly rutba: number;
  /** The base direction the patch was built for. */
  readonly ittijah: 'rtl' | 'ltr';
  /** Source text to translated text, keyed by the exact source string. */
  readonly jadwal: Readonly<Record<string, string>>;
  /** The bundled font, when the patch ships one. */
  readonly khatt?: KhattHamula;
}

/**
 * The slice of `taarib-wasm` this runtime uses.
 *
 * Declared structurally rather than imported so that this file stays a single
 * self-contained module the patcher can drop into an archive. The shapes mirror
 * `taarib-wasm`'s generated wrapper exactly — getters are properties, the rest
 * are methods — and a mismatch is a load-time failure with a message, not a
 * silent wrong result.
 */
export interface NawatTaarib {
  /** The layout engine. */
  readonly saff: SaffTaarib;
  /** The runtime glyph atlas. */
  readonly lawha: LawhaTaarib;
  /** The font chain the patch's fonts were registered into. */
  readonly silsila: SilsilaTaarib;
  /** Builds a layout request for one string at one size. */
  talab(nass: string, hajm: number): TalabTaarib;
}

/** A layout request. Mirrors `TaaribTalab`. */
export interface TalabTaarib {
  /** The text to lay out. */
  nass: string;
  /** The size in pixels. */
  hajm: number;
  /** The width to wrap at, or `undefined` for a single unwrapped line. */
  ard_mutah: number | undefined;
  /** Releases the WASM-side allocation. */
  free(): void;
}

/** The layout engine. Mirrors `TaaribSaff`. */
export interface SaffTaarib {
  /** Lays text out into positioned glyphs in visual order. */
  khattit(talab: TalabTaarib): TakhtitTaarib;
  /** Measures without positioning, from real shaped advances. */
  qis(talab: TalabTaarib): QiyasTaarib;
}

/** A measurement. Mirrors `TaaribQiyas`. */
export interface QiyasTaarib {
  /** The width of the widest line, in pixels. */
  readonly ard: number;
  /** The height of every line box together. */
  readonly irtifa: number;
  /** How far the first line rises above its baseline. */
  readonly suud: number;
  /** How far the last line falls below its baseline. */
  readonly hubut: number;
}

/** A finished layout. Mirrors `TaaribTakhtit`. */
export interface TakhtitTaarib {
  /** How many glyph rows `huruf` holds. */
  readonly adad_huruf: number;
  /** How many line rows `sutur` holds. */
  readonly adad_sutur: number;
  /** The packed glyph rows, stride `HAQL_HARF_ADAD`, in visual order. */
  readonly huruf: Float32Array;
  /** The packed line rows, stride `HAQL_SATR_ADAD`. */
  readonly sutur: Float32Array;
  /** The layout's total width. */
  readonly ard: number;
  /** The layout's total height. */
  readonly irtifa: number;
  /** Releases the WASM-side allocation. */
  free(): void;
}

/** The font chain. Mirrors `TaaribSilsila`. */
export interface SilsilaTaarib {
  /** How many fonts the chain holds. */
  readonly adad: number;
}

/** One glyph's place in the atlas. Mirrors `TaaribMawdi`. */
export interface MawdiTaarib {
  /** Which atlas page. */
  readonly safha: number;
  /** Left edge in the page, in texels. */
  readonly s: number;
  /** Top edge in the page, in texels. */
  readonly a: number;
  /** Width in texels. Zero for a glyph with no image. */
  readonly ard: number;
  /** Height in texels. Zero for a glyph with no image. */
  readonly irtifa: number;
  /** How far right of the pen the image starts. */
  readonly izaha_s: number;
  /** How far above the baseline the image's top edge sits. */
  readonly izaha_a: number;
  /** True when the glyph has no image at all — a space, for instance. */
  readonly khali: boolean;
}

/** The runtime atlas. Mirrors `TaaribLawha`. */
export interface LawhaTaarib {
  /** Starts a frame, releasing the previous frame's pins. */
  ibda_itar(): void;
  /** Rasterizes on a miss and returns where the glyph landed. */
  shakl(
    silsila: SilsilaTaarib,
    khatt: number,
    muarrif: number,
    hajm: number,
    bakat: number,
  ): MawdiTaarib;
  /** One page's coverage texels, one byte each, row-major. */
  safha(fahras: number): Uint8Array | undefined;
  /** That page's width in texels. */
  ard_safha(fahras: number): number | undefined;
  /** That page's height in texels. */
  irtifa_safha(fahras: number): number | undefined;
  /** How many pages the atlas holds. */
  readonly adad_safahat: number;
  /** Counters, used here only to know when a page's contents changed. */
  readonly ihsaat: { readonly ikhfaqat: number };
}

/** Stride of one glyph row in `TakhtitTaarib.huruf`. Mirrors `HaqlHarf.Adad`. */
export const HAQL_HARF_ADAD = 8;
/** Index of the glyph identifier within a row. */
export const HAQL_HARF_MUARRIF = 0;
/** Index of the glyph's x origin within a row. */
export const HAQL_HARF_S = 2;
/** Index of the glyph's y origin within a row. */
export const HAQL_HARF_A = 3;
/** Index of the font-chain index within a row. */
export const HAQL_HARF_KHATT = 6;

/** Stride of one line row in `TakhtitTaarib.sutur`. Mirrors `HaqlSatr.Adad`. */
export const HAQL_SATR_ADAD = 12;
/** Index of the baseline's y, from the layout's top, within a line row. */
export const HAQL_SATR_ASAS = 4;
/** Index of the line's ascent above its baseline. */
export const HAQL_SATR_SUUD = 8;
/** Index of the line's descent below its baseline. */
export const HAQL_SATR_HUBUT = 9;

/** The attribute this runtime stamps onto everything it creates. */
const SIMA = 'data-taarib';

/** Prefix for every message this runtime writes to the console. */
const WASM_SIJILL = '[taarib]';

/** Writes one line to the console, always prefixed so it is attributable. */
function sajjil(risala: string, tafsil?: unknown): void {
  if (tafsil === undefined) {
    console.warn(`${WASM_SIJILL} ${risala}`);
  } else {
    console.warn(`${WASM_SIJILL} ${risala}`, tafsil);
  }
}

// ---------------------------------------------------------------------------
// الخط — the font
// ---------------------------------------------------------------------------

/**
 * The Unicode ranges the patch's font is allowed to claim.
 *
 * Arabic, Arabic Supplement, Arabic Extended-A, the presentation forms, and the
 * two invisible marks that decide direction. Deliberately nothing else: a font
 * that claimed Latin as well would replace the game's own typography for every
 * word of English left in the interface, which is a visual change nobody asked
 * a translation to make.
 */
const NITAQAT_ARABIYA =
  'U+060C, U+061B-061F, U+0620-06FF, U+0750-077F, U+08A0-08FF, ' +
  'U+FB50-FDFF, U+FE70-FEFF, U+200F, U+061C, U+0640';

/** Family names that are CSS keywords and cannot have a face added to them. */
const AILAT_AAMMA = new Set([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  'math',
  'emoji',
  'fangsong',
  'inherit',
  'initial',
  'revert',
  'unset',
]);

/**
 * Whether a payload's font URL is a base64 `data:` URL and nothing else.
 *
 * Checked rather than trusted even though this runtime and the patcher that
 * writes the payload are the same project, because the payload is a file on
 * disk that anything can edit. The alternative is a string that goes into a CSS
 * rule, and a string that goes into a CSS rule without being checked is a way
 * to add an `@import` or a remote `src` to the document.
 */
export function rabtSalih(rabt: string): boolean {
  return /^data:[a-z0-9!#$&^_.+-]+\/[a-z0-9!#$&^_.+-]+;base64,[A-Za-z0-9+/]+={0,2}$/.test(
    rabt,
  );
}

/** Quotes a font family name for use inside a CSS declaration. */
function iqtibasAila(aila: string): string {
  return `"${aila.replace(/["\\]/g, '')}"`;
}

/**
 * Installs the patch's font and keeps it reachable from the game's own families.
 *
 * Two mechanisms, each with one job:
 *
 *  * A named `@font-face` under the payload's own family name, which is what the
 *    canvas path and any explicit styling use.
 *  * One extra `@font-face` **per family the game actually uses**, declaring the
 *    same font under *that* family name with `unicode-range` limited to Arabic.
 *    Adding a face to an existing family is how CSS composes fonts: the browser
 *    keeps the game's face for everything outside the range and uses ours inside
 *    it. The result is Arabic in a font that has Arabic and English in exactly
 *    the typography the game shipped, with no selector rewritten and no inline
 *    style added.
 *
 * A generic keyword — `sans-serif` and its neighbours — cannot have a face added
 * to it, so an element whose family resolves to one of those is given an inline
 * fallback instead. That is the only case where this runtime writes a style onto
 * a game's element.
 */
export class KhattMudmaj {
  private readonly khatt: KhattHamula;

  private readonly waraqa: CSSStyleSheet | null;

  private readonly maarufa = new Set<string>();

  private hadd = 32;

  /** Installs the base face. Does nothing when the URL is not a data URL. */
  constructor(khatt: KhattHamula) {
    this.khatt = khatt;
    this.waraqa = rabtSalih(khatt.rabt) ? KhattMudmaj.anshiWaraqa() : null;
    if (this.waraqa === null) {
      if (!rabtSalih(khatt.rabt)) {
        sajjil('the payload font is not a base64 data URL and was not installed');
      }
      return;
    }
    this.qaida(
      `@font-face{font-family:${iqtibasAila(khatt.ism)};` +
        `src:url(${khatt.rabt});font-display:block;}`,
    );
    this.maarufa.add(khatt.ism.toLowerCase());
  }

  /** The family name the font is registered under. */
  get ism(): string {
    return this.khatt.ism;
  }

  /** Whether anything was installed at all. */
  get mutah(): boolean {
    return this.waraqa !== null;
  }

  /**
   * Makes the patch's Arabic glyphs available under one of the game's own
   * family names.
   *
   * Idempotent and bounded: each family is aliased once, and no more than
   * `hadd` families are ever aliased, so a game that generates a new family
   * name per element cannot grow the stylesheet without limit.
   */
  daaAila(aila: string): void {
    const munaqqa = aila.trim().replace(/^["']|["']$/g, '');
    const miftah = munaqqa.toLowerCase();
    if (
      this.waraqa === null ||
      munaqqa === '' ||
      this.maarufa.has(miftah) ||
      AILAT_AAMMA.has(miftah) ||
      this.hadd <= 0
    ) {
      return;
    }
    this.maarufa.add(miftah);
    this.hadd -= 1;
    this.qaida(
      `@font-face{font-family:${iqtibasAila(munaqqa)};src:url(${this.khatt.rabt});` +
        `unicode-range:${NITAQAT_ARABIYA};font-display:block;}`,
    );
  }

  /** Whether a resolved family list is only generic keywords. */
  static aammaFaqat(qaima: string): boolean {
    const ajzaa = qaima
      .split(',')
      .map((juz) => juz.trim().replace(/^["']|["']$/g, '').toLowerCase())
      .filter((juz) => juz !== '');
    return ajzaa.length === 0 || ajzaa.every((juz) => AILAT_AAMMA.has(juz));
  }

  private qaida(nass: string): void {
    const waraqa = this.waraqa;
    if (waraqa === null) {
      return;
    }
    try {
      waraqa.insertRule(nass, waraqa.cssRules.length);
    } catch (khata) {
      sajjil('a font rule was refused by the stylesheet', khata);
    }
  }

  /**
   * A stylesheet this runtime owns.
   *
   * A constructed stylesheet when the engine has them, because it is not part
   * of the document tree and so cannot be found, reordered or removed by the
   * game's own DOM code. A `<style>` element otherwise, created empty and never
   * filled from patch content — the rules are built here and inserted through
   * the CSSOM, so no font name or URL is ever concatenated into markup.
   */
  private static anshiWaraqa(): CSSStyleSheet | null {
    try {
      const musannaa = new CSSStyleSheet();
      document.adoptedStyleSheets = [...document.adoptedStyleSheets, musannaa];
      return musannaa;
    } catch {
      // Older Chromium, or a document that refuses adopted sheets.
    }
    try {
      const unsur = document.createElement('style');
      unsur.setAttribute(SIMA, 'khatt');
      (document.head ?? document.documentElement).appendChild(unsur);
      return unsur.sheet;
    } catch (khata) {
      sajjil('no stylesheet could be created for the patch font', khata);
      return null;
    }
  }
}

// ---------------------------------------------------------------------------
// الاتجاه — direction
// ---------------------------------------------------------------------------

/**
 * Sets the document's base direction.
 *
 * `dir` on the root element and `direction` in a style, both, because they are
 * not the same statement: the attribute is what `dir="auto"` descendants and
 * the `:dir()` selector resolve against, and the property is what layout reads.
 * A game that sets one of them itself and not the other is common, and a patch
 * that set only one would be defeated by whichever the game set.
 */
export function atbiqIttijah(ittijah: 'rtl' | 'ltr'): void {
  const jidhr = document.documentElement;
  if (!jidhr) {
    return;
  }
  jidhr.setAttribute('dir', ittijah);
  jidhr.style.setProperty('direction', ittijah);
  if (document.body) {
    document.body.setAttribute('dir', ittijah);
  }
}

/**
 * Isolates one element's text from the text around it.
 *
 * `dir="auto"` rather than `dir="rtl"`, and the difference matters: `auto`
 * resolves the paragraph direction from the element's own first strong
 * character, so a translated Arabic label reads right-to-left and an English
 * one the patch did not cover still reads left-to-right, in the same list,
 * without either being reordered around the other. `unicode-bidi: isolate`
 * makes that resolution binding on the neighbours as well, which is what stops
 * a trailing colon or a number from jumping to the wrong end of the line.
 *
 * Elements the game already gave an explicit `dir` are left alone: the game
 * said something about that element's direction and this runtime is not
 * entitled to contradict it.
 */
export function azilUnsur(unsur: Element): void {
  if (!(unsur instanceof HTMLElement) || unsur.hasAttribute('dir')) {
    return;
  }
  unsur.setAttribute('dir', 'auto');
  unsur.style.setProperty('unicode-bidi', 'isolate');
}

// ---------------------------------------------------------------------------
// الترجمة — the string table and how it reaches the document
// ---------------------------------------------------------------------------

/** Elements whose text is never translated. */
const UNSUR_MAMNU = new Set([
  'SCRIPT',
  'STYLE',
  'NOSCRIPT',
  'TEMPLATE',
  'TEXTAREA',
  'CODE',
  'KBD',
  'SAMP',
  'VAR',
]);

/** Attributes that hold text a player reads. */
const SIMAT_NASS = ['title', 'placeholder', 'aria-label', 'alt', 'aria-placeholder'];

/** Input types whose `value` is a button label rather than user data. */
const ANWA_ZIRR = new Set(['button', 'submit', 'reset']);

/**
 * The patch's strings, keyed the way the document will present them.
 *
 * Markup puts newlines and indentation inside text nodes, so the string a
 * translator saw as `"Start Game"` arrives in the DOM as `"\n      Start
 * Game\n    "`. The table is therefore keyed on the *collapsed* form — outer
 * whitespace trimmed, inner runs reduced to one space — and the surrounding
 * whitespace of the node being replaced is put back around the translation, so
 * the document's own formatting survives being translated.
 */
export class JadwalTarjama {
  private readonly jadwal = new Map<string, string>();

  /** Builds the lookup from the payload's table. */
  constructor(jadwal: Readonly<Record<string, string>>) {
    for (const asl of Object.keys(jadwal)) {
      const tarjama = jadwal[asl];
      if (typeof tarjama !== 'string' || tarjama === '') {
        continue;
      }
      const miftah = JadwalTarjama.awhid(asl);
      if (miftah !== '' && !this.jadwal.has(miftah)) {
        this.jadwal.set(miftah, tarjama);
      }
    }
  }

  /** How many strings the patch carries. */
  get adad(): number {
    return this.jadwal.size;
  }

  /** Collapses a string to the form the table is keyed on. */
  static awhid(nass: string): string {
    return nass.replace(/\s+/g, ' ').trim();
  }

  /**
   * The translation for a piece of document text, with its whitespace kept.
   *
   * Returns `null` when there is no translation, which is the common answer and
   * is not a failure: a patch translates the strings it was given and leaves
   * everything else exactly as the game wrote it.
   */
  tarjim(khaam: string): string | null {
    const miftah = JadwalTarjama.awhid(khaam);
    if (miftah === '') {
      return null;
    }
    const tarjama = this.jadwal.get(miftah);
    if (tarjama === undefined) {
      return null;
    }
    const qabl = /^\s*/.exec(khaam)?.[0] ?? '';
    const baad = /\s*$/.exec(khaam)?.[0] ?? '';
    return `${qabl}${tarjama}${baad}`;
  }
}

/** How many nodes one observer drain will examine before yielding a frame. */
const AQSA_UQAD_LIL_DAFAA = 4000;

/** How many pending roots the observer queue holds before it gives up on them. */
const AQSA_TABUR = 512;

/**
 * Applies the patch's strings to the document, and keeps applying them.
 *
 * ## What the observer costs, and how that is bounded
 *
 * A `MutationObserver` is charged per *record*, not per document: the engine
 * already knows what changed and hands exactly that over. The cost that
 * actually bites is the one this class refuses to pay — observing `attributes`
 * without a filter. A game animating a health bar writes `style` sixty times a
 * second, and an unfiltered observer would wake for every one of them. So the
 * observation is `childList`, `characterData`, and an explicit
 * `attributeFilter` of five names, and nothing else.
 *
 * On top of that:
 *
 *  * Records are **queued and drained once per animation frame**, so a burst of
 *    a thousand insertions is one pass over one deduplicated set of roots
 *    rather than a thousand callbacks doing a thousand tree walks.
 *  * Each drain has a **node budget** (`AQSA_UQAD_LIL_DAFAA`). What it does not
 *    reach is rescheduled, so a drain never holds a frame, and a game that
 *    rebuilds ten thousand nodes at once stays at its frame rate while the
 *    translation catches up over the next few frames.
 *  * A translated text node is **remembered with what was written into it**, in
 *    a `WeakMap`. A node whose data still matches what this runtime wrote is
 *    skipped without a lookup, which is what stops the runtime reacting to its
 *    own writes; a node the game has since rewritten does not match, and is
 *    translated again, which is what makes a re-rendered message window work.
 *    A `WeakMap` and not a `Map`, so a node the game discards is collectable and
 *    a long session does not leak one entry per line of dialogue ever shown.
 *  * The queue itself is **capped** (`AQSA_TABUR`). Past that the queue is
 *    dropped for a single full walk of the document, because at that point a
 *    walk is cheaper than the bookkeeping, and unbounded queue growth is the
 *    one way an observer turns into a leak.
 */
export class Mutarjim {
  private readonly jadwal: JadwalTarjama;

  private readonly khatt: KhattMudmaj | null;

  private readonly maktub = new WeakMap<Text, string>();

  private readonly tabur = new Set<Node>();

  private muraqib: MutationObserver | null = null;

  private mujadwal = false;

  private kamil = false;

  /** How many replacements have been made. Reported by `ihsaat`. */
  private badalat = 0;

  /** Binds a table and a font to a document. */
  constructor(jadwal: JadwalTarjama, khatt: KhattMudmaj | null) {
    this.jadwal = jadwal;
    this.khatt = khatt;
  }

  /** Replacements made so far, for a log line. */
  get ihsaat(): number {
    return this.badalat;
  }

  /**
   * Translates the document as it stands and starts watching for more.
   *
   * Safe to call before the document has a body: the initial walk is repeated
   * on `DOMContentLoaded`, because a preload runs before the parser has built
   * anything and a single early walk would translate an empty document.
   */
  ibda(): void {
    this.imshi(document);
    if (document.readyState === 'loading') {
      document.addEventListener(
        'DOMContentLoaded',
        () => {
          this.imshi(document);
        },
        { once: true },
      );
    }
    this.muraqib = new MutationObserver((sijillat) => {
      this.iltaqit(sijillat);
    });
    this.muraqib.observe(document, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: SIMAT_NASS,
    });
    window.addEventListener(
      'pagehide',
      () => {
        this.awqif();
      },
      { once: true },
    );
  }

  /** Stops watching. The document keeps whatever was already translated. */
  awqif(): void {
    this.muraqib?.disconnect();
    this.muraqib = null;
    this.tabur.clear();
  }

  private iltaqit(sijillat: readonly MutationRecord[]): void {
    for (const sijill of sijillat) {
      if (this.kamil) {
        break;
      }
      if (sijill.type === 'characterData') {
        this.idaf(sijill.target);
      } else if (sijill.type === 'attributes') {
        this.idaf(sijill.target);
      } else {
        for (const uqda of sijill.addedNodes) {
          this.idaf(uqda);
        }
      }
    }
    this.jadwil();
  }

  private idaf(uqda: Node): void {
    if (this.tabur.size >= AQSA_TABUR) {
      // Past the cap the bookkeeping costs more than the walk it was avoiding.
      this.tabur.clear();
      this.kamil = true;
      return;
    }
    this.tabur.add(uqda);
  }

  private jadwil(): void {
    if (this.mujadwal) {
      return;
    }
    this.mujadwal = true;
    const dafaa = (): void => {
      this.mujadwal = false;
      this.ufrigh();
    };
    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(dafaa);
    } else {
      setTimeout(dafaa, 0);
    }
  }

  private ufrigh(): void {
    let mizaniya = AQSA_UQAD_LIL_DAFAA;
    if (this.kamil) {
      this.kamil = false;
      mizaniya = this.imshi(document, mizaniya);
      if (mizaniya <= 0) {
        this.kamil = true;
        this.jadwil();
      }
      return;
    }
    const juzur = [...this.tabur];
    this.tabur.clear();
    for (const jidhr of juzur) {
      if (mizaniya <= 0) {
        this.idaf(jidhr);
        continue;
      }
      mizaniya = this.imshi(jidhr, mizaniya);
    }
    if (this.tabur.size > 0) {
      this.jadwil();
    }
  }

  /**
   * Translates everything under one root, spending at most `mizaniya` nodes.
   *
   * Returns what is left of the budget, so a caller can walk several roots and
   * still stop at one total.
   */
  imshi(jidhr: Node, mizaniya = AQSA_UQAD_LIL_DAFAA): number {
    let baqi = mizaniya;
    if (jidhr.nodeType === Node.TEXT_NODE) {
      this.nass(jidhr as Text);
      return baqi - 1;
    }
    if (!(jidhr instanceof Element) && !(jidhr instanceof Document)) {
      return baqi;
    }
    if (jidhr instanceof Element) {
      if (Mutarjim.mamnu(jidhr)) {
        return baqi;
      }
      this.simat(jidhr);
    }
    const mashi = document.createTreeWalker(
      jidhr,
      NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT,
      {
        acceptNode: (uqda: Node): number => {
          if (uqda instanceof Element) {
            return Mutarjim.mamnu(uqda)
              ? NodeFilter.FILTER_REJECT
              : NodeFilter.FILTER_ACCEPT;
          }
          return NodeFilter.FILTER_ACCEPT;
        },
      },
    );
    let uqda = mashi.nextNode();
    while (uqda !== null && baqi > 0) {
      baqi -= 1;
      if (uqda instanceof Element) {
        this.simat(uqda);
      } else {
        this.nass(uqda as Text);
      }
      uqda = mashi.nextNode();
    }
    return baqi;
  }

  /** Whether an element and everything under it is off limits. */
  private static mamnu(unsur: Element): boolean {
    if (UNSUR_MAMNU.has(unsur.tagName) || unsur.hasAttribute(SIMA)) {
      return true;
    }
    return unsur instanceof HTMLElement && unsur.isContentEditable;
  }

  /** Translates one text node in place. */
  private nass(uqda: Text): void {
    const khaam = uqda.data;
    if (khaam.length === 0 || this.maktub.get(uqda) === khaam) {
      return;
    }
    const tarjama = this.jadwal.tarjim(khaam);
    if (tarjama === null || tarjama === khaam) {
      return;
    }
    // textContent, never innerHTML: a translation is data and stays data.
    uqda.data = tarjama;
    this.maktub.set(uqda, tarjama);
    this.badalat += 1;
    const walid = uqda.parentElement;
    if (walid !== null) {
      azilUnsur(walid);
      this.qayyidKhatt(walid);
    }
  }

  /** Translates the text-bearing attributes of one element. */
  private simat(unsur: Element): void {
    let ghayyar = false;
    for (const sima of SIMAT_NASS) {
      const khaam = unsur.getAttribute(sima);
      if (khaam === null || khaam === '') {
        continue;
      }
      const tarjama = this.jadwal.tarjim(khaam);
      if (tarjama !== null && tarjama !== khaam) {
        unsur.setAttribute(sima, tarjama);
        this.badalat += 1;
        ghayyar = true;
      }
    }
    if (unsur instanceof HTMLInputElement && ANWA_ZIRR.has(unsur.type)) {
      const tarjama = this.jadwal.tarjim(unsur.value);
      if (tarjama !== null && tarjama !== unsur.value) {
        unsur.value = tarjama;
        this.badalat += 1;
        ghayyar = true;
      }
    }
    if (ghayyar) {
      azilUnsur(unsur);
      this.qayyidKhatt(unsur);
    }
  }

  /**
   * Makes sure the patch's Arabic glyphs are reachable from this element's font.
   *
   * Reads the computed family once per translated element and hands it to the
   * font installer, which aliases each distinct family exactly once. When the
   * family resolves to a generic keyword — which cannot have a face added to it
   * — the element gets the patch family appended to its own list inline, which
   * is the only style this runtime writes onto a game's element.
   */
  private qayyidKhatt(unsur: Element): void {
    const khatt = this.khatt;
    if (khatt === null || !khatt.mutah || !(unsur instanceof HTMLElement)) {
      return;
    }
    const qaima = getComputedStyle(unsur).fontFamily;
    if (qaima === '') {
      return;
    }
    if (KhattMudmaj.aammaFaqat(qaima)) {
      if (!qaima.includes(khatt.ism)) {
        unsur.style.setProperty('font-family', `${iqtibasAila(khatt.ism)}, ${qaima}`);
      }
      return;
    }
    for (const juz of qaima.split(',')) {
      khatt.daaAila(juz);
    }
  }
}

// ---------------------------------------------------------------------------
// اللوحة — the canvas takeover, rung three only
// ---------------------------------------------------------------------------

/** The subpixel bucket this adapter asks the atlas for: whole pixels. */
const BAKAT_TAHAZZUZ = 0;

/** Marker property, so the hooks are never installed twice on one prototype. */
const SIMAT_LAWHA = '__taaribLawha';

/** Matches the pixel size in a CSS `font` shorthand. */
const NAMAT_HAJM = /(\d+(?:\.\d+)?)\s*px/;

/**
 * The atlas's pages, ready to blit, tinted per colour.
 *
 * The atlas hands out one byte of coverage per texel. A 2D canvas wants RGBA,
 * so each page is expanded once per colour into an offscreen canvas whose RGB
 * is the colour and whose alpha is the coverage. That is one expansion per page
 * per colour for as long as the atlas does not change, rather than one per
 * glyph per frame.
 *
 * The cache is keyed on the atlas's page count and miss count together, so any
 * rasterization — which is the only thing that can change a page's contents —
 * invalidates it. A stale page is a glyph drawn from where another glyph used
 * to be, which looks like a shaping bug and is not one.
 */
class SafahatLawha {
  private readonly lawha: LawhaTaarib;

  private readonly makhzan = new Map<string, HTMLCanvasElement>();

  private tanqih = -1;

  constructor(lawha: LawhaTaarib) {
    this.lawha = lawha;
  }

  /** A page tinted with one CSS colour, or `null` when the page is absent. */
  safha(fahras: number, lawn: string): HTMLCanvasElement | null {
    const tanqih = this.lawha.adad_safahat * 1_000_003 + this.lawha.ihsaat.ikhfaqat;
    if (tanqih !== this.tanqih) {
      this.makhzan.clear();
      this.tanqih = tanqih;
    }
    const miftah = `${fahras}|${lawn}`;
    const mukhazzan = this.makhzan.get(miftah);
    if (mukhazzan !== undefined) {
      return mukhazzan;
    }
    const bayt = this.lawha.safha(fahras);
    const ard = this.lawha.ard_safha(fahras);
    const irtifa = this.lawha.irtifa_safha(fahras);
    if (bayt === undefined || ard === undefined || irtifa === undefined) {
      return null;
    }
    const lawh = document.createElement('canvas');
    lawh.width = ard;
    lawh.height = irtifa;
    const siyaq = lawh.getContext('2d');
    if (siyaq === null) {
      return null;
    }
    const [ahmar, akhdar, azraq] = SafahatLawha.hallilLawn(lawn);
    const suura = siyaq.createImageData(ard, irtifa);
    const bayanat = suura.data;
    for (let i = 0; i < bayt.length; i += 1) {
      const j = i * 4;
      bayanat[j] = ahmar;
      bayanat[j + 1] = akhdar;
      bayanat[j + 2] = azraq;
      bayanat[j + 3] = bayt[i] ?? 0;
    }
    siyaq.putImageData(suura, 0, 0);
    this.makhzan.set(miftah, lawh);
    return lawh;
  }

  /**
   * Resolves any CSS colour to its three channels.
   *
   * Through a one-pixel canvas, because that is the only way to get the
   * engine's own answer for `hsl()`, `color-mix()`, a named colour or anything
   * else CSS grows next — and re-implementing colour parsing here would be a
   * second answer to a question the engine already answers.
   */
  private static hallilLawn(lawn: string): [number, number, number] {
    const mukhazzan = SafahatLawha.alwan.get(lawn);
    if (mukhazzan !== undefined) {
      return mukhazzan;
    }
    let natija: [number, number, number] = [255, 255, 255];
    try {
      const lawh = document.createElement('canvas');
      lawh.width = 1;
      lawh.height = 1;
      const siyaq = lawh.getContext('2d', { willReadFrequently: true });
      if (siyaq !== null) {
        siyaq.fillStyle = lawn;
        siyaq.fillRect(0, 0, 1, 1);
        const bayanat = siyaq.getImageData(0, 0, 1, 1).data;
        natija = [bayanat[0] ?? 255, bayanat[1] ?? 255, bayanat[2] ?? 255];
      }
    } catch (khata) {
      sajjil('a fill colour could not be resolved; white was used', khata);
    }
    if (SafahatLawha.alwan.size < 256) {
      SafahatLawha.alwan.set(lawn, natija);
    }
    return natija;
  }

  private static readonly alwan = new Map<string, [number, number, number]>();
}

/** The pixel size in a canvas `font` shorthand, defaulting to 10. */
function hajmMinKhatt(khatt: string): number {
  const mutabaqa = NAMAT_HAJM.exec(khatt);
  const hajm = mutabaqa === null ? Number.NaN : Number.parseFloat(mutabaqa[1] ?? '');
  return Number.isFinite(hajm) && hajm > 0 ? hajm : 10;
}

/**
 * Routes canvas text through the engine and draws it from Taarib's atlas.
 *
 * **Only installed on a rung-three game.** On any other game this is a
 * regression: `fillText` already shapes, already joins, already reorders, and
 * replacing it would trade a correct implementation for a newer one and lose
 * the browser's own text handling in the bargain. The gate is the rung the
 * patch recorded, not anything this function measures at runtime, because the
 * evidence that decides the rung is in the shipped scripts and not in the
 * running page.
 *
 * Three methods are replaced and each keeps its contract:
 *
 *  * `measureText` returns real shaped advances, so a game that reserves space
 *    before drawing reserves the right amount.
 *  * `fillText` lays out through `saff` and blits one atlas glyph per shaped
 *    glyph, honouring `textAlign`, `textBaseline`, `maxWidth`, the current
 *    transform and `globalAlpha`.
 *  * `strokeText` draws the same glyphs in `strokeStyle`. It is an
 *    approximation and is documented as one: an outline pass cannot be
 *    reproduced from coverage bitmaps, so the outline is lost. Drawing solidly
 *    in the stroke colour is chosen over skipping the call, because the usual
 *    idiom strokes and then fills over the top — so the fill still lands
 *    correctly — while a game that only strokes still gets readable text
 *    instead of nothing.
 *
 * Returns whether anything was installed.
 */
export function rakkibLawha(nawat: NawatTaarib, hamula: Hamula): boolean {
  if (hamula.rutba !== RUTBA_ISTILA) {
    sajjil(
      `the recorded rung is ${hamula.rutba}, so canvas text is left to the browser, ` +
        'which shapes and reorders it correctly on its own',
    );
    return false;
  }
  if (typeof CanvasRenderingContext2D !== 'function') {
    return false;
  }
  const namat = CanvasRenderingContext2D.prototype as unknown as Record<string, unknown>;
  if (namat[SIMAT_LAWHA] === true) {
    return true;
  }
  namat[SIMAT_LAWHA] = true;

  const safahat = new SafahatLawha(nawat.lawha);
  const asli = {
    fillText: CanvasRenderingContext2D.prototype.fillText,
    strokeText: CanvasRenderingContext2D.prototype.strokeText,
    measureText: CanvasRenderingContext2D.prototype.measureText,
  };

  const qis = (nass: string, hajm: number): QiyasTaarib | null => {
    const talab = nawat.talab(nass, hajm);
    try {
      return nawat.saff.qis(talab);
    } catch (khata) {
      sajjil('measurement failed; the browser was left to measure', khata);
      return null;
    } finally {
      talab.free();
    }
  };

  CanvasRenderingContext2D.prototype.measureText = function measureTextTaarib(
    this: CanvasRenderingContext2D,
    nass: string,
  ): TextMetrics {
    const qiyas = qis(nass, hajmMinKhatt(this.font));
    if (qiyas === null) {
      return asli.measureText.call(this, nass);
    }
    return {
      width: qiyas.ard,
      actualBoundingBoxLeft: 0,
      actualBoundingBoxRight: qiyas.ard,
      actualBoundingBoxAscent: qiyas.suud,
      actualBoundingBoxDescent: qiyas.hubut,
      fontBoundingBoxAscent: qiyas.suud,
      fontBoundingBoxDescent: qiyas.hubut,
      emHeightAscent: qiyas.suud,
      emHeightDescent: qiyas.hubut,
      hangingBaseline: qiyas.suud,
      alphabeticBaseline: 0,
      ideographicBaseline: -qiyas.hubut,
    } as unknown as TextMetrics;
  };

  const irsim = (
    siyaq: CanvasRenderingContext2D,
    lawn: string,
    nass: string,
    s: number,
    a: number,
    aqsaArd?: number,
  ): boolean => {
    if (nass === '') {
      return true;
    }
    const hajm = hajmMinKhatt(siyaq.font);
    const talab = nawat.talab(nass, hajm);
    let takhtit: TakhtitTaarib;
    try {
      nawat.lawha.ibda_itar();
      takhtit = nawat.saff.khattit(talab);
    } catch (khata) {
      talab.free();
      sajjil('layout failed; this string was left to the browser', khata);
      return false;
    }
    talab.free();
    try {
      rasmHuruf(siyaq, safahat, nawat, takhtit, lawn, hajm, s, a, aqsaArd);
    } finally {
      takhtit.free();
    }
    return true;
  };

  CanvasRenderingContext2D.prototype.fillText = function fillTextTaarib(
    this: CanvasRenderingContext2D,
    nass: string,
    s: number,
    a: number,
    aqsaArd?: number,
  ): void {
    const lawn = typeof this.fillStyle === 'string' ? this.fillStyle : '#ffffff';
    if (!irsim(this, lawn, nass, s, a, aqsaArd)) {
      asli.fillText.call(this, nass, s, a, aqsaArd);
    }
  };

  CanvasRenderingContext2D.prototype.strokeText = function strokeTextTaarib(
    this: CanvasRenderingContext2D,
    nass: string,
    s: number,
    a: number,
    aqsaArd?: number,
  ): void {
    const lawn = typeof this.strokeStyle === 'string' ? this.strokeStyle : '#000000';
    if (!irsim(this, lawn, nass, s, a, aqsaArd)) {
      asli.strokeText.call(this, nass, s, a, aqsaArd);
    }
  };

  return true;
}

/**
 * Blits one finished layout onto a canvas.
 *
 * Everything that decides *where* comes out of the layout: the glyph rows are
 * already in visual order with their x positions measured from the layout's own
 * left edge, so this function never reorders anything and never decides which
 * glyph is which. It resolves the three things the canvas API owns and the
 * layout cannot know — the alignment origin, the baseline, and `maxWidth` — and
 * then draws.
 *
 * The current transform, `globalAlpha` and any clip stay in force, because the
 * glyphs go through `drawImage` exactly as the game's own images do.
 */
function rasmHuruf(
  siyaq: CanvasRenderingContext2D,
  safahat: SafahatLawha,
  nawat: NawatTaarib,
  takhtit: TakhtitTaarib,
  lawn: string,
  hajm: number,
  s: number,
  a: number,
  aqsaArd?: number,
): void {
  const huruf = takhtit.huruf;
  const sutur = takhtit.sutur;
  const asas = takhtit.adad_sutur > 0 ? (sutur[HAQL_SATR_ASAS] ?? 0) : 0;
  const suud = takhtit.adad_sutur > 0 ? (sutur[HAQL_SATR_SUUD] ?? 0) : 0;
  const hubut = takhtit.adad_sutur > 0 ? (sutur[HAQL_SATR_HUBUT] ?? 0) : 0;

  let khatAsas = a;
  switch (siyaq.textBaseline) {
    case 'top':
    case 'hanging':
      khatAsas = a + suud;
      break;
    case 'middle':
      khatAsas = a + (suud - hubut) / 2;
      break;
    case 'bottom':
    case 'ideographic':
      khatAsas = a - hubut;
      break;
    default:
      khatAsas = a;
      break;
  }
  const qimma = khatAsas - asas;

  const yameen =
    siyaq.direction === 'rtl' ||
    (siyaq.direction === 'inherit' && document.documentElement?.dir === 'rtl');
  let muhadhaha: 'left' | 'right' | 'center';
  switch (siyaq.textAlign) {
    case 'right':
      muhadhaha = 'right';
      break;
    case 'center':
      muhadhaha = 'center';
      break;
    case 'start':
      muhadhaha = yameen ? 'right' : 'left';
      break;
    case 'end':
      muhadhaha = yameen ? 'left' : 'right';
      break;
    default:
      muhadhaha = 'left';
      break;
  }
  const ard = takhtit.ard;
  let badaS = s;
  if (muhadhaha === 'right') {
    badaS = s - ard;
  } else if (muhadhaha === 'center') {
    badaS = s - ard / 2;
  }

  const miqyas =
    typeof aqsaArd === 'number' && Number.isFinite(aqsaArd) && aqsaArd > 0 && ard > aqsaArd
      ? aqsaArd / ard
      : 1;

  siyaq.save();
  if (miqyas !== 1) {
    siyaq.translate(badaS, 0);
    siyaq.scale(miqyas, 1);
    badaS = 0;
  }
  let shakwa = false;
  for (let fahras = 0; fahras < takhtit.adad_huruf; fahras += 1) {
    const bidaya = fahras * HAQL_HARF_ADAD;
    const muarrif = Math.round(huruf[bidaya + HAQL_HARF_MUARRIF] ?? 0);
    const harfS = huruf[bidaya + HAQL_HARF_S] ?? 0;
    const harfA = huruf[bidaya + HAQL_HARF_A] ?? 0;
    const khatt = Math.round(huruf[bidaya + HAQL_HARF_KHATT] ?? 0);
    let mawdi: MawdiTaarib;
    try {
      mawdi = nawat.lawha.shakl(nawat.silsila, khatt, muarrif, hajm, BAKAT_TAHAZZUZ);
    } catch (khata) {
      if (!shakwa) {
        shakwa = true;
        sajjil('the atlas refused a glyph; the rest of the string was still drawn', khata);
      }
      continue;
    }
    if (mawdi.khali) {
      continue;
    }
    const lawh = safahat.safha(mawdi.safha, lawn);
    if (lawh === null) {
      continue;
    }
    siyaq.drawImage(
      lawh,
      mawdi.s,
      mawdi.a,
      mawdi.ard,
      mawdi.irtifa,
      badaS + harfS + mawdi.izaha_s,
      qimma + harfA - mawdi.izaha_a,
      mawdi.ard,
      mawdi.irtifa,
    );
  }
  siyaq.restore();
}

// ---------------------------------------------------------------------------
// التركيب — installation
// ---------------------------------------------------------------------------

/** The payload version this build reads. */
const ISDAR_HAMULA = 1;

/** The one live translator, so a second install is a no-op rather than a leak. */
let mutarjimHali: Mutarjim | null = null;

/** Whether a parsed payload is the shape this build reads. */
function hamulaSaliha(qeema: unknown): qeema is Hamula {
  if (typeof qeema !== 'object' || qeema === null) {
    return false;
  }
  const kain = qeema as Record<string, unknown>;
  return (
    kain['isdar'] === ISDAR_HAMULA &&
    typeof kain['rutba'] === 'number' &&
    (kain['ittijah'] === 'rtl' || kain['ittijah'] === 'ltr') &&
    typeof kain['jadwal'] === 'object' &&
    kain['jadwal'] !== null
  );
}

/**
 * The payload the isolated-world copy left in the document for the page world.
 *
 * Read from the text content of an inert `<script type="application/json">` and
 * parsed with `JSON.parse`. The element is inert by type, the parse produces
 * data, and neither step can execute anything the payload contains.
 */
function hamulaMinSafha(): Hamula | null {
  const unsur = document.getElementById('taarib-hamula');
  if (unsur === null) {
    return null;
  }
  try {
    const maqru: unknown = JSON.parse(unsur.textContent ?? '');
    return hamulaSaliha(maqru) ? maqru : null;
  } catch (khata) {
    sajjil('the in-page payload could not be parsed', khata);
    return null;
  }
}

/** The engine, when something has already put one on the global object. */
function nawatAlAam(): NawatTaarib | null {
  const aam = globalThis as unknown as Record<string, unknown>;
  const muhtamal = aam['taaribNawat'];
  if (typeof muhtamal !== 'object' || muhtamal === null) {
    return null;
  }
  const kain = muhtamal as Record<string, unknown>;
  return typeof kain['talab'] === 'function' && typeof kain['saff'] === 'object'
    ? (muhtamal as NawatTaarib)
    : null;
}

/**
 * Installs the runtime.
 *
 * Which half runs depends on which world this copy is in, and the split is not
 * arbitrary — it is what the DOM and the canvas prototype respectively are:
 *
 *  * **`maazul`** — a preload's isolated world. It shares the document with the
 *    page, so direction, the font and the translation all work from here, and
 *    doing them here keeps them out of reach of the game's own scripts.
 *  * **`safha`** — the page's own world, entered only on a rung-three game. The
 *    isolated copy has already done the document work; the only thing that
 *    genuinely needs this world is the canvas prototype, because the isolated
 *    world has a different one and hooking it would hook nothing.
 *
 * Returns whether anything was installed. A `false` here is not an error: it is
 * a payload this build does not read, or a rung whose branch does not apply.
 */
export function rakkib(hamula: Hamula, nitaq: Nitaq, nawat?: NawatTaarib): boolean {
  if (!hamulaSaliha(hamula)) {
    sajjil('the payload is not a shape this build reads; nothing was installed');
    return false;
  }
  if (typeof document === 'undefined') {
    return false;
  }

  if (nitaq === 'safha') {
    const muharrik = nawat ?? nawatAlAam();
    if (muharrik === null) {
      sajjil(
        'the page world has no Taarib engine, so canvas text is left to the game. ' +
          'Nothing was evaluated from a string in its place.',
      );
      return false;
    }
    return rakkibLawha(muharrik, hamula);
  }

  if (mutarjimHali !== null) {
    // A second install would run a second observer over the same document and
    // double every drain's cost for no benefit.
    return true;
  }

  atbiqIttijah(hamula.ittijah);

  const khatt = hamula.khatt === undefined ? null : new KhattMudmaj(hamula.khatt);
  const jadwal = new JadwalTarjama(hamula.jadwal);
  // Held at module scope, not in a local: the observer keeps its callback alive
  // and the callback keeps this object alive, but relying on that is relying on
  // a detail of how observers are collected. A named handle also gives `fakkik`
  // something to disconnect.
  const mutarjim = new Mutarjim(jadwal, khatt);
  mutarjim.ibda();
  mutarjimHali = mutarjim;

  const rutba = hamula.rutba;
  const wasf =
    rutba === RUTBA_ISTILA
      ? 'rung three: the canvas hook is installed separately in the page world'
      : rutba === RUTBA_TASHEEH
        ? 'rung two: direction and layout corrected, shaping left to the browser'
        : 'rung one: font, direction and translation, and nothing else';
  sajjil(
    `installed — ${wasf}. ${jadwal.adad} string(s) in the table, ` +
      `${mutarjim.ihsaat} applied on the first pass, ` +
      `${khatt !== null && khatt.mutah ? `font ${khatt.ism}` : 'no bundled font'}.`,
  );
  return true;
}

/**
 * Stops the observer and lets the runtime be collected.
 *
 * The document keeps whatever was already translated — undoing a translation
 * would mean remembering every original string for the life of the process,
 * which is a cost paid on every game to serve a case that only arises when
 * somebody is debugging this file.
 */
export function fakkik(): void {
  mutarjimHali?.awqif();
  mutarjimHali = null;
}

// The page-world copy is loaded as a plain `<script src=...>` with a marker on
// it, so it installs itself. `document.currentScript` is the element being
// executed, which is how this copy knows it is the page-world one rather than
// a module somebody required.
try {
  const hali = typeof document === 'undefined' ? null : document.currentScript;
  if (hali instanceof HTMLScriptElement && hali.dataset['taaribNitaq'] === 'safha') {
    const hamula = hamulaMinSafha();
    if (hamula !== null) {
      rakkib(hamula, 'safha');
    }
  }
} catch (khata) {
  sajjil('the page-world copy could not install itself', khata);
}




