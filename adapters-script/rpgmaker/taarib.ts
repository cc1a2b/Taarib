/*:
 * @target MZ
 * @plugindesc تعريب — Arabic text for RPG Maker MV and MZ. Installed by Taarib.
 * @author Taarib
 * @url https://github.com/cc1a2b/taarib
 * @help
 * This plugin is installed by Taarib's patcher and configured by it. Every
 * parameter below was decided when the patch was built, from the game's own
 * shipped files, and none of it should be edited by hand: the rung in
 * particular is the answer to a question the patcher asked of this specific
 * game, and changing it either takes over an engine that was already drawing
 * Arabic correctly or leaves Arabic unjoined.
 *
 * To uninstall: delete this file, delete js/plugins/taarib/, and remove the
 * "taarib" entry from js/plugins.js. The plugin adds nothing anywhere else.
 *
 * @param rutba
 * @text Rung
 * @desc 1 configuration only, 2 corrected shaping, 3 full takeover. Set by the patcher from the game's own files.
 * @type select
 * @option 1
 * @option 2
 * @option 3
 * @default 2
 *
 * @param khatt
 * @text Font file
 * @desc File name inside js/plugins/taarib/. Letters, digits, dot, dash and underscore only.
 * @default Taarib.ttf
 *
 * @param bayanat
 * @text Patch data
 * @desc File name inside js/plugins/taarib/ holding this patch's runtime decisions, as JSON.
 * @default taarib.json
 *
 * @param lugha
 * @text Language
 * @desc 0 detect, 1 Arabic, 2 Persian, 3 Urdu, 4 Latin.
 * @default 1
 *
 * @param arqam
 * @text Digits
 * @desc 0 as written, 1 European, 2 Arabic-Indic, 3 Eastern Arabic-Indic.
 * @default 0
 *
 * @param tashkeel
 * @text Diacritics
 * @desc 0 keep, 1 remove, 2 keep in dialogue only.
 * @default 0
 *
 * @param dabt
 * @text Justification
 * @desc 0 none, 1 spaces, 2 kashida, 3 kashida then spaces.
 * @default 0
 *
 * @param nawafidhYameen
 * @text Mirror windows
 * @desc Mirror selectable window layouts right to left.
 * @type boolean
 * @default true
 */

// -----------------------------------------------------------------------------
// تعريب — the RPG Maker MV and MZ runtime.
//
// ## What this file is allowed to do, and what it is not
//
// It integrates. It does not implement. No line here shapes Arabic, reorders a
// bidirectional line, chooses a kashida, positions a diacritic or maps a
// codepoint to a glyph. On the takeover rung all of that happens inside
// `taarib_core.wasm`, which is the same Rust that draws the preview in Taarib
// Studio and the text in a Unity game. A convenience reimplementation here —
// however small, however "just for this one case" — would fork the one
// guarantee the whole product rests on.
//
// It also never evaluates patch content. There is no `eval`, no `new Function`,
// no `innerHTML`, no `setTimeout` with a string, and no `JSON.parse` result that
// reaches a code path expecting code. A patch is data. Data never becomes code.
//
// ## The rung decides almost everything, and this file does not decide the rung
//
// The patcher established the rung from the game's shipped files — which
// Chromium the shell carries, whether the core script's text chain buffers whole
// runs or draws one character at a time, and whether an enabled plugin replaced
// that chain — and wrote it into `@param rutba`. This file reads it and branches.
// It deliberately does not re-derive it: the files the probe read are not in the
// same state once a patch is installed, and two answers to one question is one
// answer too many.
//
//   * **Rung 1 and 2 — the canvas shapes.** `Bitmap.prototype.drawText` ends in
//     `fillText`, and Chromium's `fillText` runs the same HarfBuzz its layout
//     engine does. So Arabic already joins. What is wrong is direction,
//     alignment and window layout, and this file corrects exactly those: it
//     installs the font, forces the base direction of runs that need it, flips
//     leading-edge alignment, turns on MZ's own right-to-left text state, and
//     mirrors the window layouts the patch marked. The text stays real text —
//     selectable in a devtools inspector, searchable, and legible to anything
//     that reads the canvas.
//
//   * **Rung 3 — the canvas is never asked.** The engine draws one character at
//     a time and no shaper can join a single character to neighbours it was
//     never shown. So the text pipeline is replaced: layout comes from
//     `taarib_core.wasm`, glyphs come from Taarib's atlas, and the message
//     window's typewriter reveal is re-implemented over shaped glyph clusters
//     rather than over characters. This costs the player selectable text and the
//     patcher said so before installing.
//
// ## Which JavaScript this compiles to, and what that forbids
//
// The output is ES5, because the oldest runtime this has to boot in is RPG Maker
// MV 1.5's NW.js 0.12 — Chromium 41, from 2015. TypeScript down-levels syntax
// but it does not polyfill library functions, so the source avoids everything
// that is a runtime feature rather than a syntactic one:
//
//   * No `Map`, `Set`, `WeakMap`, `Symbol`, `Proxy`, `Reflect`.
//   * No `Object.assign`, `Object.entries`, `Object.values`.
//   * No `Array.prototype.includes`, `find`, `findIndex`, `fill`, `from`, `of`.
//   * No `String.prototype.includes`, `padStart`, `padEnd`, `repeat`,
//     `codePointAt`, `normalize`.
//   * No `Number.isInteger`, `Number.parseFloat`, `Number.EPSILON`.
//   * No `fetch`, no `TextDecoder`, no `URL`, no `async`/`await`, no generators.
//   * No `for...of` over anything but a real array, and no spread of anything.
//
// `XMLHttpRequest` loads every file, because these games run from `file://` and
// `fetch` refuses that scheme even where it exists. `Promise` is used and is
// safe: it shipped in Chromium 32, before every runtime in scope.
//
// `WebAssembly` is the one thing that genuinely is not there on Chromium 41 — it
// arrived in Chromium 57. So the takeover rung feature-detects it, and a game
// too old to load the core descends to the corrected-shaping rung at run time
// and says so in the console rather than booting into a broken text pipeline.
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// The engine, as much of it as this file touches
//
// Declared rather than imported: RPG Maker has no module system and every one of
// these is a global `var` the core scripts defined before this plugin loaded.
// The shapes are deliberately narrow — only the members actually used appear —
// so that a change in the engine breaks compilation here instead of silently
// drawing nothing.
// -----------------------------------------------------------------------------

/** The plugin manager, which is how a plugin reads the parameters the list holds. */
declare const PluginManager: {
    parameters(ism: string): { [miftah: string]: string };
};

/** The engine's own environment queries. Present in both MV and MZ. */
declare const Utils: {
    RPGMAKER_NAME: string;
    RPGMAKER_VERSION: string;
};

/** The screen and, in MV, the font loader. */
declare const Graphics: {
    width: number;
    height: number;
    loadFont?: (ism: string, masar: string) => void;
    isFontLoaded?: (ism: string) => boolean;
};

/** MZ's font loader. Absent in MV, which is how the two are told apart. */
declare const FontManager:
    | {
          load(aila: string, malaf: string): void;
          isReady(): boolean;
      }
    | undefined;

/**
 * Taarib's own share of a text state, on the takeover rung.
 *
 * A sub-line: the stretch of raw text from where the pen was last anchored to
 * the end of the current line, at the font size in force when it was anchored.
 * A mid-line size change starts a new one, which is how `\{` and `\}` keep
 * working when the layout is no longer the engine's.
 */
interface HalatTaarib {
    /** Where this sub-line starts in the raw text, as a UTF-16 index. */
    bidaya: number;
    /** The left edge the sub-line is drawn from. */
    s: number;
    /** The top of the line box. */
    a: number;
    /** The font size the sub-line was prepared at. */
    hajm: number;
    /** How much width the revealed part covered on the last draw. */
    ard: number;
}

/** One text state as the window text chain passes it around. */
interface HalatNass {
    text: string;
    index: number;
    x: number;
    y: number;
    left?: number;
    height: number;
    /** MZ only: the run being accumulated before a flush. */
    buffer?: string;
    /** MZ only: whether this run is right to left. */
    rtl?: boolean;
    /** MZ only: whether this pass draws or only measures. */
    drawing?: boolean;
    /** Taarib's own state, attached on the takeover rung. */
    taarib?: HalatTaarib;
}

/** A drawable surface. Both MV and MZ expose the same members this file uses. */
interface BitmapRpg {
    width: number;
    height: number;
    fontFace: string;
    fontSize: number;
    textColor: string;
    paintOpacity: number;
    context?: CanvasRenderingContext2D;
    _context?: CanvasRenderingContext2D;
    _canvas?: HTMLCanvasElement;
    drawText(
        nass: string,
        s: number,
        a: number,
        ard: number,
        irtifa: number,
        muhadhaha?: string,
    ): void;
    measureTextWidth(nass: string): number;
    clearRect(s: number, a: number, ard: number, irtifa: number): void;
    blt(
        masdar: BitmapRpg,
        ms: number,
        ma: number,
        mard: number,
        mirtifa: number,
        hs: number,
        ha: number,
        hard?: number,
        hirtifa?: number,
    ): void;
    _setDirty?: () => void;
    _baseTexture?: { update(): void };
}

/** The window base class, as far as the text chain goes. */
interface NafidhaRpg {
    contents: BitmapRpg;
    innerWidth: number;
    innerHeight: number;
    width: number;
    height: number;
    padding: number;
    constructor: { name: string };
    textWidth(nass: string): number;
    drawTextEx(nass: string, s: number, a: number, ard?: number): number;
    calcTextHeight(halat: HalatNass, kul?: boolean): number;
    processCharacter(halat: HalatNass): void;
    processControlCharacter(halat: HalatNass, ramz: string): void;
    flushTextState?: (halat: HalatNass) => void;
    createTextState?: (
        nass: string,
        s: number,
        a: number,
        ard: number,
    ) => HalatNass;
    resetFontSettings?: () => void;
    contentsWidth?: () => number;
    obtainEscapeCode?: (halat: HalatNass) => string;
    processEscapeCharacter?: (ramz: string, halat: HalatNass) => void;
    processNewLine?: (halat: HalatNass) => void;
    processDrawIcon?: (fahras: number, halat: HalatNass) => void;
    textColor?: (fahras: number) => string;
    changeTextColor?: (lawn: string) => void;
    resetTextColor?: () => void;
    itemRect?: (fahras: number) => MustatilRpg;
    itemLineRect?: (fahras: number) => MustatilRpg;
    lineHeight(): number;
}

/** A rectangle as the engine's selectable windows hand them out. */
interface MustatilRpg {
    x: number;
    y: number;
    width: number;
    height: number;
}

/** A constructor with a prototype this file extends. */
interface SinfRpg<T> {
    prototype: T;
    new (...wusut: never[]): T;
}

declare const Bitmap: SinfRpg<BitmapRpg>;
declare const Window_Base: SinfRpg<NafidhaRpg>;
declare const Window_Selectable: SinfRpg<NafidhaRpg> | undefined;
declare const SceneManager: { renderScene?: () => void };

/** The image cache, which is where the icon sheet comes from. */
declare const ImageManager:
    | {
          loadSystem(ism: string): BitmapRpg;
          iconWidth?: number;
          iconHeight?: number;
      }
    | undefined;

// -----------------------------------------------------------------------------
// `taarib_core.wasm`, through the `wasm-bindgen` no-modules glue
//
// Only the members this file calls are declared. The field-index enums are read
// off the module rather than written out here, because the row layouts belong to
// `taarib-wasm` and an adapter that counted offsets by hand would be a second
// place they are recorded and the first place they would go stale.
// -----------------------------------------------------------------------------

/** A finished layout of text that still carried its markup. */
interface TakhtitKhaam {
    readonly nass: string;
    readonly adad_huruf: number;
    readonly adad_sutur: number;
    readonly huruf: Float32Array;
    readonly sutur: Float32Array;
    readonly ard: number;
    readonly irtifa: number;
    readonly hajm: number;
    readonly yameen: boolean;
    readonly adad_dharrat: number;
    dharra_khaam(fahras: number): string | undefined;
    dharra_nitaq(fahras: number): number | undefined;
    dharra_naw(fahras: number): number | undefined;
    dharra_tarteeb(fahras: number): number | undefined;
    free(): void;
}

/** A measurement: the same pipeline, none of the output. */
interface Qiyas {
    readonly ard: number;
    readonly irtifa: number;
    readonly suud: number;
    readonly hubut: number;
    readonly adad_sutur: number;
    free(): void;
}

/** Where one glyph lives in the atlas, and how to draw it. */
interface MawdiShakl {
    readonly taqaddum: number;
    readonly safha: number;
    readonly s: number;
    readonly a: number;
    readonly ard: number;
    readonly irtifa: number;
    readonly izaha_s: number;
    readonly izaha_a: number;
    readonly khali: boolean;
    free(): void;
}

/** A font's own metrics at one pixel size. */
interface QiyasatKhatt {
    readonly suud: number;
    readonly hubut: number;
    readonly fajwa: number;
    readonly irtifa_satr: number;
    free(): void;
}

/** The recorded layout decisions a patch carries. */
interface KhiyaratTaarib {
    ittijah: number;
    lugha: number;
    dabt: number;
    muhadhaha: number;
    tashkeel: number;
    arqam: number;
    tajawuz: number;
    hajm_adna: number;
    irtifa_satr: number;
    tabaud_ahruf: number;
    tabaud_kalimat: number;
    hiwar: boolean;
    satr_wahid: boolean;
    free(): void;
}

/** The shaping and layout engine. One per game, alive for its whole run. */
interface Saff {
    khattit_khaam_bi(
        khaam: string,
        lahja: number,
        silsila: Silsila,
        hajm: number,
        ard: number | undefined,
        irtifa: number | undefined,
        khiyarat: KhiyaratTaarib,
    ): TakhtitKhaam;
    qis(talab: Talab): Qiyas;
    free(): void;
}

/** A layout request over clean text. */
interface Talab {
    nass: string;
    hajm: number;
    ard_mutah: number | undefined;
    bi_khiyarat(khiyarat: KhiyaratTaarib): void;
    free(): void;
}

/** A loaded, validated font. */
interface Khatt {
    readonly aila: string;
    qiyasat(hajm: number): QiyasatKhatt;
    istinsakh(): Khatt;
    free(): void;
}

/** An ordered chain of fonts, tried in order per character. */
interface Silsila {
    readonly adad: number;
    qiyasat(hajm: number): QiyasatKhatt;
    free(): void;
}

/** The runtime atlas. */
interface Lawha {
    readonly adad_safahat: number;
    ibda_itar(): void;
    shakl(
        silsila: Silsila,
        khatt: number,
        muarrif: number,
        hajm: number,
        bakat: number,
    ): MawdiShakl;
    safha(fahras: number): Uint8Array | undefined;
    ard_safha(fahras: number): number | undefined;
    irtifa_safha(fahras: number): number | undefined;
    free(): void;
}

/** How the atlas is sized, padded and budgeted. */
interface KhiyaratLawha {
    aqsa_ard: number;
    aqsa_irtifa: number;
    hashw: number;
    quwwat_ithnayn: boolean;
    mizaniyat_bayt: number;
    free(): void;
}

/** The module's exports, as the no-modules glue installs them. */
interface NawatTaarib {
    (mawrid?: ArrayBuffer | string): Promise<unknown>;
    hayyi(): void;
    isdar(): string;
    bakat_tahazzuz(s: number): number;
    HaqlHarf: { [ism: string]: number };
    HaqlSatr: { [ism: string]: number };
    AlamHarf: { [ism: string]: number };
    AlamSatr: { [ism: string]: number };
    NawDharra: { [ism: string]: number };
    Lahja: { [ism: string]: number };
    TaaribSaff: { new (): Saff };
    TaaribKhatt: { min_bayt(bayt: Uint8Array, fahras: number): Khatt };
    TaaribSilsila: { new (khutut: Khatt[]): Silsila };
    TaaribLawha: { new (khiyarat: KhiyaratLawha): Lawha };
    TaaribKhiyaratLawha: { new (): KhiyaratLawha };
    TaaribKhiyarat: { new (): KhiyaratTaarib };
    TaaribTalab: { new (nass: string, silsila: Silsila, hajm: number): Talab };
}

declare const wasm_bindgen: NawatTaarib | undefined;

(function taarib(): void {
    "use strict";

    /** The plugin's name in the engine's plugin list. */
    const ISM = "taarib";

    /** Where the plugin's own files live, relative to the game's index.html. */
    const MUJALLAD = "js/plugins/taarib/";

    /**
     * The `wasm-bindgen` glue and the core, by fixed name.
     *
     * Fixed rather than parameterised on purpose. Every other file this plugin
     * opens is named by a parameter and validated as a bare file name; these two
     * are the only files whose contents become executable code in the page, and
     * a name that came from patch data would be patch data choosing what to
     * execute.
     */
    const GLUE = MUJALLAD + "taarib_core.js";

    /** The core module's bytes, by the same fixed name and for the same reason. */
    const WASM = MUJALLAD + "taarib_core_bg.wasm";

    /** Writes one diagnostic line. The only channel a game process offers. */
    function sajjil(risala: string): void {
        if (typeof console !== "undefined" && console && console.log) {
            console.log("[taarib] " + risala);
        }
    }

    /** Writes one diagnostic line for something that went wrong. */
    function anthir(risala: string): void {
        if (typeof console !== "undefined" && console && console.warn) {
            console.warn("[taarib] " + risala);
        }
    }

    // -------------------------------------------------------------------------
    // Settings
    // -------------------------------------------------------------------------

    /** The decisions the patcher recorded into the plugin list entry. */
    interface Idadat {
        rutba: number;
        khatt: string;
        bayanat: string;
        lugha: number;
        arqam: number;
        tashkeel: number;
        dabt: number;
        nawafidhYameen: boolean;
    }

    /**
     * Whether a name is a bare file name this plugin will open.
     *
     * Letters, digits, dot, dash and underscore, and no `..` anywhere. The
     * parameters come out of a patch a stranger produced, and a file name is the
     * one field in a patch that decides which bytes on somebody's disk get read.
     */
    function ism_amin(ism: string): boolean {
        if (!ism || ism.length > 128) {
            return false;
        }
        if (ism.indexOf("..") >= 0) {
            return false;
        }
        return /^[A-Za-z0-9._-]+$/.test(ism);
    }

    /** Reads a whole number from a parameter, clamped, with a documented default. */
    function raqm(nass: string | undefined, adna: number, aqsa: number, iftiradi: number): number {
        if (typeof nass !== "string" || nass.length === 0) {
            return iftiradi;
        }
        const qeema = parseInt(nass, 10);
        if (isNaN(qeema) || qeema < adna || qeema > aqsa) {
            return iftiradi;
        }
        return qeema;
    }

    /** Reads the plugin list entry's parameters, validating every one of them. */
    function qira_idadat(): Idadat {
        let khaam: { [miftah: string]: string } = {};
        try {
            khaam = PluginManager.parameters(ISM) || {};
        } catch (khata) {
            anthir("the plugin list holds no parameters for " + ISM);
        }
        const khatt = khaam["khatt"];
        const bayanat = khaam["bayanat"];
        return {
            rutba: raqm(khaam["rutba"], 1, 3, 2),
            khatt: ism_amin(khatt) ? khatt : "",
            bayanat: ism_amin(bayanat) ? bayanat : "",
            lugha: raqm(khaam["lugha"], 0, 4, 1),
            arqam: raqm(khaam["arqam"], 0, 3, 0),
            tashkeel: raqm(khaam["tashkeel"], 0, 2, 0),
            dabt: raqm(khaam["dabt"], 0, 3, 0),
            nawafidhYameen: khaam["nawafidhYameen"] !== "false",
        };
    }

    // -------------------------------------------------------------------------
    // The patch's own runtime data
    //
    // Read with `JSON.parse` and then checked field by field. Nothing in it is
    // ever evaluated, nothing in it names a code path, and every field that is
    // absent or of the wrong type takes the documented default rather than
    // making the plugin fail — a patch built by an older compiler must still run.
    // -------------------------------------------------------------------------

    /** What the patch tells the runtime about how to draw this game. */
    interface Bayanat {
        /** The size in pixels to draw at, or zero to keep the engine's own. */
        hajmKhatt: number;
        /** Whether selectable window layouts are mirrored. */
        yameen: boolean;
        /** Window class names that are never mirrored, however marked. */
        mustathnayat: string[];
        /** Alignment, as `Muhadhaha` numbers it. */
        muhadhaha: number;
        /** The widest atlas page, in texels. */
        aqsaArd: number;
        /** The tallest atlas page, in texels. */
        aqsaIrtifa: number;
        /** The gutter around every glyph, in texels. */
        hashw: number;
        /** The atlas byte budget. */
        mizaniyatBayt: number;
    }

    /** The defaults, which are the conservative profile the atlas documents. */
    function bayanat_iftiradiya(): Bayanat {
        return {
            hajmKhatt: 0,
            yameen: true,
            mustathnayat: [],
            muhadhaha: 0,
            aqsaArd: 2048,
            aqsaIrtifa: 2048,
            hashw: 1,
            mizaniyatBayt: 32 * 1024 * 1024,
        };
    }

    /** A number field of a parsed patch, clamped, or the default. */
    function haql_raqm(
        jidhr: { [miftah: string]: unknown },
        ism: string,
        adna: number,
        aqsa: number,
        iftiradi: number,
    ): number {
        const qeema = jidhr[ism];
        if (typeof qeema !== "number" || !isFinite(qeema) || qeema < adna || qeema > aqsa) {
            return iftiradi;
        }
        return Math.floor(qeema);
    }

    /** A boolean field of a parsed patch, or the default. */
    function haql_mantiqi(
        jidhr: { [miftah: string]: unknown },
        ism: string,
        iftiradi: boolean,
    ): boolean {
        const qeema = jidhr[ism];
        return typeof qeema === "boolean" ? qeema : iftiradi;
    }

    /** A list-of-names field of a parsed patch, filtered to plain identifiers. */
    function haql_asmaa(jidhr: { [miftah: string]: unknown }, ism: string): string[] {
        const qeema = jidhr[ism];
        const mabni: string[] = [];
        if (!(qeema instanceof Array)) {
            return mabni;
        }
        for (let khana = 0; khana < qeema.length; khana += 1) {
            const wahid: unknown = qeema[khana];
            if (typeof wahid === "string" && /^[A-Za-z0-9_]{1,64}$/.test(wahid)) {
                mabni.push(wahid);
            }
        }
        return mabni;
    }

    /** Validates a parsed patch document into the shape the runtime uses. */
    function qira_bayanat(khaam: string): Bayanat {
        const bayanat = bayanat_iftiradiya();
        let jidhr: unknown;
        try {
            jidhr = JSON.parse(khaam);
        } catch (khata) {
            anthir("the patch data file is not valid JSON; defaults are used");
            return bayanat;
        }
        if (!jidhr || typeof jidhr !== "object" || jidhr instanceof Array) {
            anthir("the patch data file is not an object; defaults are used");
            return bayanat;
        }
        const kutla = jidhr as { [miftah: string]: unknown };
        bayanat.hajmKhatt = haql_raqm(kutla, "hajmKhatt", 0, 256, 0);
        bayanat.yameen = haql_mantiqi(kutla, "yameen", true);
        bayanat.mustathnayat = haql_asmaa(kutla, "mustathnayat");
        bayanat.muhadhaha = haql_raqm(kutla, "muhadhaha", 0, 3, 0);
        bayanat.aqsaArd = haql_raqm(kutla, "aqsaArd", 64, 8192, 2048);
        bayanat.aqsaIrtifa = haql_raqm(kutla, "aqsaIrtifa", 64, 8192, 2048);
        bayanat.hashw = haql_raqm(kutla, "hashw", 0, 16, 1);
        bayanat.mizaniyatBayt = haql_raqm(
            kutla,
            "mizaniyatBayt",
            1024 * 1024,
            512 * 1024 * 1024,
            32 * 1024 * 1024,
        );
        return bayanat;
    }

    // -------------------------------------------------------------------------
    // Loading
    //
    // `XMLHttpRequest` everywhere. These games run from `file://`, which `fetch`
    // refuses on every browser that has it, and NW.js allows for XHR because the
    // engine's own data loader depends on that allowance.
    // -------------------------------------------------------------------------

    /** Reads a file as bytes. */
    function hammil_bayt(
        masar: string,
        thumma: (bayt: Uint8Array) => void,
        akhfaq: (sabab: string) => void,
    ): void {
        const talab = new XMLHttpRequest();
        talab.open("GET", masar, true);
        talab.responseType = "arraybuffer";
        talab.onload = function (): void {
            const nataja = talab.status;
            if ((nataja >= 200 && nataja < 300) || nataja === 0) {
                const jism: unknown = talab.response;
                if (jism instanceof ArrayBuffer) {
                    thumma(new Uint8Array(jism));
                    return;
                }
            }
            akhfaq(masar + " responded " + String(nataja));
        };
        talab.onerror = function (): void {
            akhfaq(masar + " could not be read");
        };
        talab.send();
    }

    /** Reads a file as text. */
    function hammil_nass(
        masar: string,
        thumma: (nass: string) => void,
        akhfaq: (sabab: string) => void,
    ): void {
        const talab = new XMLHttpRequest();
        talab.open("GET", masar, true);
        talab.onload = function (): void {
            const nataja = talab.status;
            if ((nataja >= 200 && nataja < 300) || nataja === 0) {
                thumma(talab.responseText || "");
                return;
            }
            akhfaq(masar + " responded " + String(nataja));
        };
        talab.onerror = function (): void {
            akhfaq(masar + " could not be read");
        };
        talab.send();
    }

    /**
     * Loads the one script this plugin adds to the page.
     *
     * A `<script src>` element and nothing else: no `eval`, no `new Function`,
     * no `innerHTML`. The source is [`GLUE`], a fixed relative path this file
     * spells out, never a value that came from patch data.
     */
    function hammil_barnamaj(thumma: () => void, akhfaq: (sabab: string) => void): void {
        const unsur = document.createElement("script");
        unsur.type = "text/javascript";
        unsur.async = false;
        unsur.src = GLUE;
        unsur.onload = function (): void {
            thumma();
        };
        unsur.onerror = function (): void {
            akhfaq(GLUE + " could not be loaded");
        };
        document.body.appendChild(unsur);
    }

    // -------------------------------------------------------------------------
    // Shared helpers
    // -------------------------------------------------------------------------

    /** The family name the patch's font is registered under. */
    const AILA_KHATT = "TaaribArabic";

    /**
     * Whether a string holds Arabic-script text.
     *
     * The four blocks plus the two presentation-form blocks. The presentation
     * forms are checked because a game may already contain text some other tool
     * mangled into them, and such a string still needs right-to-left treatment
     * even though Taarib will never produce one.
     */
    const ARABI = new RegExp(
        "[\\u0600-\\u06FF\\u0750-\\u077F\\u08A0-\\u08FF\\uFB50-\\uFDFF\\uFE70-\\uFEFF]",
    );

    /** Whether a string needs right-to-left treatment. */
    function fiha_arabi(nass: string): boolean {
        return ARABI.test(nass);
    }

    /** U+2067 RIGHT-TO-LEFT ISOLATE, spelled by code point so it stays visible. */
    const RLI = String.fromCharCode(0x2067);

    /** U+2069 POP DIRECTIONAL ISOLATE. */
    const PDI = String.fromCharCode(0x2069);

    /**
     * Forces a run's base direction without touching the canvas state.
     *
     * `context.direction` would be the direct way and arrived in Chromium 63,
     * which is later than the oldest runtime here. An isolate pair is portable
     * to every runtime in scope, costs no advance width because both characters
     * are zero-width, and is what the bidirectional algorithm defines for
     * exactly this purpose.
     */
    function itar_ittijah(nass: string): string {
        return RLI + nass + PDI;
    }

    /** The 2D context behind a bitmap, under either engine's field name. */
    function siyaq_bitmap(lawha: BitmapRpg): CanvasRenderingContext2D | undefined {
        if (lawha.context) {
            return lawha.context;
        }
        return lawha._context;
    }

    /** The width a window has for content, under either engine's field name. */
    function ard_dakhili(nafidha: NafidhaRpg): number {
        if (typeof nafidha.innerWidth === "number" && nafidha.innerWidth > 0) {
            return nafidha.innerWidth;
        }
        if (nafidha.contentsWidth) {
            return nafidha.contentsWidth();
        }
        return nafidha.contents ? nafidha.contents.width : 0;
    }

    /** The class name of a window, or the empty string where it was minified away. */
    function ism_nafidha(nafidha: NafidhaRpg): string {
        const sinf = nafidha.constructor;
        return sinf && typeof sinf.name === "string" ? sinf.name : "";
    }

    // -------------------------------------------------------------------------
    // The font
    // -------------------------------------------------------------------------

    /**
     * Registers the patch's font under [`AILA_KHATT`].
     *
     * Through a `@font-face` rule rather than through either engine's own font
     * loader, because MZ's `FontManager.load` resolves names against `fonts/`
     * and MV's `Graphics.loadFont` against wherever the caller says. One rule
     * built here works identically under both and puts the file where the patch
     * put it. The rule's only variable part is a file name this plugin already
     * validated as letters, digits, dot, dash and underscore.
     */
    function rakkib_khatt(idadat: Idadat): boolean {
        if (!idadat.khatt) {
            anthir("no font file is configured; the game's own font is kept");
            return false;
        }
        const masar = MUJALLAD + idadat.khatt;
        if (Graphics.loadFont) {
            Graphics.loadFont(AILA_KHATT, masar);
            return true;
        }
        const namat = document.createElement("style");
        namat.type = "text/css";
        const qaida =
            '@font-face { font-family: "' +
            AILA_KHATT +
            '"; src: url("' +
            masar +
            '"); font-display: block; }';
        namat.appendChild(document.createTextNode(qaida));
        document.head.appendChild(namat);
        return true;
    }

    /** Makes every window draw in the patch's font. */
    function atbiq_khatt(): void {
        const asli = Window_Base.prototype.resetFontSettings;
        if (!asli) {
            return;
        }
        Window_Base.prototype.resetFontSettings = function (this: NafidhaRpg): void {
            asli.call(this);
            if (this.contents) {
                this.contents.fontFace = AILA_KHATT;
            }
        };
    }

    // -------------------------------------------------------------------------
    // Rungs one and two — the canvas shapes; direction and layout are corrected
    //
    // Nothing here draws a glyph. Every character still goes through the
    // engine's own `fillText`, which means the text stays real text: it is the
    // same string the game stored, in the same canvas, shaped by the same
    // HarfBuzz the browser uses everywhere else. What is corrected is the three
    // things the engine gets wrong about right-to-left text and would get wrong
    // no matter how good its shaper was, because they are layout decisions and
    // not shaping ones.
    // -------------------------------------------------------------------------

    /**
     * Flips leading-edge alignment for a right-to-left string.
     *
     * Only for a string that actually holds Arabic. A number drawn with
     * `"right"` — an item count, a gold total — keeps its alignment, because a
     * numeric column sits at the same edge of a mirrored window that it sat at
     * before, and flipping it would move it twice.
     */
    function muhadhaha_maqluba(muhadhaha: string | undefined, aksis: boolean): string | undefined {
        if (!aksis) {
            return muhadhaha;
        }
        if (muhadhaha === "left" || muhadhaha === undefined) {
            return "right";
        }
        if (muhadhaha === "right") {
            return "left";
        }
        return muhadhaha;
    }

    /**
     * Wraps `Bitmap.prototype.drawText` so a right-to-left run is drawn as one.
     *
     * Two corrections, both of which leave the shaping to the browser:
     *
     *   * the run is wrapped in an isolate pair so its base direction is
     *     right-to-left regardless of what the canvas element's CSS direction
     *     happens to be, and regardless of whether a leading digit or bracket
     *     would otherwise have decided it;
     *   * `context.direction` is set as well where the runtime has it, because
     *     it is what the alignment inside `fillText`'s own max-width handling
     *     reads, and the isolate alone does not reach that.
     *
     * Everything not holding Arabic goes through untouched, so a game's English
     * menus and its numbers are byte-for-byte the calls the engine made.
     */
    function sahhih_ittijah(bayanat: Bayanat): void {
        const asli = Bitmap.prototype.drawText;
        Bitmap.prototype.drawText = function (
            this: BitmapRpg,
            nass: string,
            s: number,
            a: number,
            ard: number,
            irtifa: number,
            muhadhaha?: string,
        ): void {
            const makhtut = nass === undefined || nass === null ? "" : String(nass);
            if (!fiha_arabi(makhtut)) {
                asli.call(this, makhtut, s, a, ard, irtifa, muhadhaha);
                return;
            }
            const siyaq = siyaq_bitmap(this);
            // `CanvasDirection`, not `string`: what comes off the context is what
            // has to go back onto it, and a widened `string` makes the restore
            // below unassignable — which is the compiler noticing that a
            // restore is only a restore if it round-trips the same type.
            let sabiq: CanvasDirection | undefined;
            if (siyaq && typeof siyaq.direction === "string") {
                sabiq = siyaq.direction;
                siyaq.direction = "rtl";
            }
            asli.call(
                this,
                itar_ittijah(makhtut),
                s,
                a,
                ard,
                irtifa,
                muhadhaha_maqluba(muhadhaha, bayanat.yameen),
            );
            if (siyaq && sabiq !== undefined) {
                siyaq.direction = sabiq;
            }
        };
    }

    /**
     * Turns on MZ's own right-to-left text state.
     *
     * MZ's window text chain already buffers characters and flushes whole runs,
     * and `flushTextState` already knows how to advance the pen leftwards and
     * how to seed a buffer with an embedding when `rtl` is set. The engine
     * simply never sets it. Setting it is the whole correction: the run order,
     * the pen advance and the wrap point all come out right, from the engine's
     * own code, with nothing replaced.
     *
     * Absent under MV, where there is no `createTextState` to wrap. That is not
     * a gap — an MV game reaching this rung is an MV game whose text chain a
     * plugin already replaced, and that plugin owns the run order.
     */
    function sahhih_halat_nass(): void {
        const asli = Window_Base.prototype.createTextState;
        if (!asli) {
            sajjil("this engine has no createTextState; run direction is left to the canvas");
            return;
        }
        Window_Base.prototype.createTextState = function (
            this: NafidhaRpg,
            nass: string,
            s: number,
            a: number,
            ard: number,
        ): HalatNass {
            const halat = asli.call(this, nass, s, a, ard);
            if (fiha_arabi(halat.text || "")) {
                halat.rtl = true;
                if (this.flushTextState && typeof halat.buffer === "string") {
                    // The buffer was seeded before `rtl` was known, so it is
                    // seeded again now. `createTextBuffer` is the engine's own
                    // function and produces the embedding the engine expects.
                    const munshi = (this as unknown as {
                        createTextBuffer?: (yameen: boolean) => string;
                    }).createTextBuffer;
                    if (munshi) {
                        halat.buffer = munshi.call(this, true);
                    }
                }
            }
            return halat;
        };
    }

    /**
     * Mirrors selectable window layouts.
     *
     * One override, on the rectangle every selectable window computes for its
     * items. Mirroring there moves the items, their icons, their labels and
     * their cursors together, because all four are positioned from it.
     *
     * A window whose class name is in the patch's exclusion list is left alone,
     * and so is every window when the class name could not be read at all —
     * which happens in a game whose core scripts were minified. Mirroring the
     * wrong window is a visible defect; declining to mirror is not.
     */
    function iqlib_nawafidh(idadat: Idadat, bayanat: Bayanat): void {
        if (!idadat.nawafidhYameen || !bayanat.yameen) {
            return;
        }
        if (typeof Window_Selectable === "undefined" || !Window_Selectable) {
            sajjil("this engine has no Window_Selectable; window layouts are left alone");
            return;
        }
        const mustathnayat = bayanat.mustathnayat;
        const yumkin = function (nafidha: NafidhaRpg): boolean {
            const ism = ism_nafidha(nafidha);
            if (!ism) {
                return false;
            }
            for (let khana = 0; khana < mustathnayat.length; khana += 1) {
                if (mustathnayat[khana] === ism) {
                    return false;
                }
            }
            return true;
        };
        const iqlib = function (nafidha: NafidhaRpg, mustatil: MustatilRpg): MustatilRpg {
            const ard = ard_dakhili(nafidha);
            if (ard <= 0 || !yumkin(nafidha)) {
                return mustatil;
            }
            mustatil.x = ard - mustatil.x - mustatil.width;
            return mustatil;
        };
        const asli = Window_Selectable.prototype.itemRect;
        if (asli) {
            Window_Selectable.prototype.itemRect = function (
                this: NafidhaRpg,
                fahras: number,
            ): MustatilRpg {
                return iqlib(this, asli.call(this, fahras));
            };
        }
        const asli_satr = Window_Selectable.prototype.itemLineRect;
        if (asli_satr) {
            Window_Selectable.prototype.itemLineRect = function (
                this: NafidhaRpg,
                fahras: number,
            ): MustatilRpg {
                // MZ derives the line rectangle from `itemRect`, which is
                // already mirrored above; mirroring again would put it back.
                return asli_satr.call(this, fahras);
            };
        }
    }

    /** Installs everything rungs one and two need. */
    function rakkib_tasheeh(idadat: Idadat, bayanat: Bayanat): void {
        atbiq_khatt();
        sahhih_ittijah(bayanat);
        sahhih_halat_nass();
        iqlib_nawafidh(idadat, bayanat);
        sajjil("rung " + String(idadat.rutba) + ": the engine shapes, direction corrected");
    }

    // -------------------------------------------------------------------------
    // Rung three — the takeover
    //
    // Reached only when the patcher established that this game draws one
    // character at a time. Everything below replaces the text pipeline with a
    // layout from `taarib_core.wasm` and glyph images from Taarib's atlas. It is
    // the expensive rung and the one that costs the player selectable text, and
    // it exists because a shaper handed one character has no neighbours to join
    // to however good it is.
    // -------------------------------------------------------------------------

    /** The one object replacement character `nasq` leaves per lifted atom. */
    const BADEEL_DHARRA = 0xfffc;

    /** `NawDharra::Sura` — an inline image. */
    const DHARRA_SURA = 1;

    /** `NawDharra::Amr` — a command the engine acts on and never draws. */
    const DHARRA_AMR = 3;

    /** The loaded core and everything built on top of it. */
    interface Nawat {
        wasm: NawatTaarib;
        saff: Saff;
        silsila: Silsila;
        lawha: Lawha;
        khiyarat: KhiyaratTaarib;
        harf: { [ism: string]: number };
        satr: { [ism: string]: number };
        lahja: number;
        safahat: HTMLCanvasElement[];
        adadSafahat: number;
        khadsh: HTMLCanvasElement;
    }

    /**
     * Turns one atlas page into a canvas the engine can `drawImage` from.
     *
     * Pages cross as one byte of coverage per texel. A canvas wants four, so the
     * expansion happens here, once per page and again only when the page grows:
     * white with the coverage as alpha, which is what lets a single page serve
     * every colour through one composite operation instead of one rasterization
     * per colour.
     */
    function safha_lawha(hala: Nawat, fahras: number): HTMLCanvasElement | undefined {
        const mawjud = hala.safahat[fahras];
        if (mawjud) {
            return mawjud;
        }
        const bayt = hala.lawha.safha(fahras);
        const ard = hala.lawha.ard_safha(fahras);
        const irtifa = hala.lawha.irtifa_safha(fahras);
        if (!bayt || !ard || !irtifa) {
            return undefined;
        }
        const lawh = document.createElement("canvas");
        lawh.width = ard;
        lawh.height = irtifa;
        const siyaq = lawh.getContext("2d");
        if (!siyaq) {
            return undefined;
        }
        const suura = siyaq.createImageData(ard, irtifa);
        const hadaf = suura.data;
        for (let khana = 0; khana < bayt.length; khana += 1) {
            const asas = khana * 4;
            hadaf[asas] = 255;
            hadaf[asas + 1] = 255;
            hadaf[asas + 2] = 255;
            hadaf[asas + 3] = bayt[khana];
        }
        siyaq.putImageData(suura, 0, 0);
        hala.safahat[fahras] = lawh;
        return lawh;
    }

    /**
     * Drops the cached page canvases when the atlas has opened a new page.
     *
     * A page that grew or was repacked is a page whose cached expansion is
     * stale, and a stale page draws the wrong letter. The atlas reports its page
     * count, and a change in it is the one observable that says so.
     */
    function jaddid_safahat(hala: Nawat): void {
        const hali = hala.lawha.adad_safahat;
        if (hali !== hala.adadSafahat) {
            hala.safahat = [];
            hala.adadSafahat = hali;
        }
    }

    /** The scratch canvas every tinted blit goes through, grown as needed. */
    function khadsh_bi(hala: Nawat, ard: number, irtifa: number): CanvasRenderingContext2D | undefined {
        const lawh = hala.khadsh;
        if (lawh.width < ard) {
            lawh.width = Math.ceil(ard);
        }
        if (lawh.height < irtifa) {
            lawh.height = Math.ceil(irtifa);
        }
        const siyaq = lawh.getContext("2d");
        if (!siyaq) {
            return undefined;
        }
        siyaq.clearRect(0, 0, lawh.width, lawh.height);
        return siyaq;
    }

    /** The UTF-8 byte length of a string, which is what a cluster offset counts in. */
    function tul_utf8(nass: string): number {
        let tul = 0;
        for (let khana = 0; khana < nass.length; khana += 1) {
            const wahda = nass.charCodeAt(khana);
            if (wahda < 0x80) {
                tul += 1;
            } else if (wahda < 0x800) {
                tul += 2;
            } else if (wahda >= 0xd800 && wahda < 0xdc00 && khana + 1 < nass.length) {
                tul += 4;
                khana += 1;
            } else {
                tul += 3;
            }
        }
        return tul;
    }

    // -------------------------------------------------------------------------
    // Preparing a line
    //
    // A raw string becomes a sequence of parts: laid-out text, and inline icons
    // between them. The split exists because `khattit_khaam_bi` lifts `\I[n]`
    // into a zero-width atom — correct for shaping, since an icon is not a
    // letter and must never join to one — and something has to reserve the room
    // the icon occupies. Splitting at the icon does that, and an icon is a
    // legitimate run boundary anyway: no font ligates across a picture.
    //
    // The raw offsets of the atoms are found from the atom table the layout
    // itself returned, by searching for each atom's own recorded source in
    // order. Nothing here parses an escape code. Reading the grammar is
    // `taarib_saff::nasq`'s job and it has already done it.
    // -------------------------------------------------------------------------

    /** A colour change, at a byte offset into a part's clean text. */
    interface NitaqLawn {
        mawqi: number;
        ramz: number;
    }

    /** One piece of a prepared line: laid-out text, or one icon. */
    interface JuzMarsum {
        /** The icon index, or -1 when this piece is text. */
        ayqouna: number;
        /** Where this piece starts in the raw string, as a UTF-16 index. */
        khaamBidaya: number;
        /** One past where it ends. */
        khaamNihaya: number;
        /** The layout, for a text piece. */
        takhtit?: TakhtitKhaam;
        /** The colour changes inside it, by clean-text byte offset. */
        alwan: NitaqLawn[];
        /** Where the lifted atoms sat in this piece's own raw text. */
        dharrat: { mawqi: number; tul: number }[];
        /** How much horizontal room the piece takes. */
        ard: number;
    }

    /** A raw string, prepared for drawing and for a progressive reveal. */
    interface SatrMuhaddad {
        miftah: string;
        khaam: string;
        ajzaa: JuzMarsum[];
        ard: number;
        irtifa: number;
        suud: number;
        yameen: boolean;
    }

    /** How many prepared lines are kept. A message window shows four at a time. */
    const HIFZ_SUTUR = 32;

    /** The prepared-line cache, most recently used first. */
    const makhzan: SatrMuhaddad[] = [];

    /** Frees a prepared line's WebAssembly-side layouts. */
    function atliq_satr(satr: SatrMuhaddad): void {
        for (let khana = 0; khana < satr.ajzaa.length; khana += 1) {
            const juz = satr.ajzaa[khana];
            if (juz.takhtit) {
                juz.takhtit.free();
                juz.takhtit = undefined;
            }
        }
    }

    /** Looks a prepared line up, moving it to the front when found. */
    function min_makhzan(miftah: string): SatrMuhaddad | undefined {
        for (let khana = 0; khana < makhzan.length; khana += 1) {
            const satr = makhzan[khana];
            if (satr.miftah === miftah) {
                makhzan.splice(khana, 1);
                makhzan.unshift(satr);
                return satr;
            }
        }
        return undefined;
    }

    /** Stores a prepared line, evicting the least recently used. */
    function ila_makhzan(satr: SatrMuhaddad): void {
        makhzan.unshift(satr);
        while (makhzan.length > HIFZ_SUTUR) {
            const qadeem = makhzan.pop();
            if (qadeem) {
                atliq_satr(qadeem);
            }
        }
    }

    /** The byte offsets, in a clean text, of each lifted atom's placeholder. */
    function mawaqi_dharrat(naqi: string): number[] {
        const mawaqi: number[] = [];
        let bayt = 0;
        for (let khana = 0; khana < naqi.length; khana += 1) {
            const wahda = naqi.charCodeAt(khana);
            if (wahda === BADEEL_DHARRA) {
                mawaqi.push(bayt);
            }
            if (wahda < 0x80) {
                bayt += 1;
            } else if (wahda < 0x800) {
                bayt += 2;
            } else if (wahda >= 0xd800 && wahda < 0xdc00 && khana + 1 < naqi.length) {
                bayt += 4;
                khana += 1;
            } else {
                bayt += 3;
            }
        }
        return mawaqi;
    }

    /** The number inside a bracketed escape's recorded source, or -1. */
    function raqm_dharra(khaam: string): number {
        const mutabaqa = /^\\[A-Za-z]\[(\d{1,9})\]$/.exec(khaam);
        if (!mutabaqa) {
            return -1;
        }
        const qeema = parseInt(mutabaqa[1], 10);
        return isNaN(qeema) ? -1 : qeema;
    }

    /** Whether a recorded atom source is a colour change. */
    function huwa_lawn(khaam: string): boolean {
        return khaam.length > 2 && khaam.charAt(0) === "\\" && /^[Cc]$/.test(khaam.charAt(1));
    }

    /** The colour changes a layout's atom table records, by clean-text offset. */
    function alwan_takhtit(takhtit: TakhtitKhaam): NitaqLawn[] {
        const mawaqi = mawaqi_dharrat(takhtit.nass);
        const alwan: NitaqLawn[] = [];
        for (let khana = 0; khana < takhtit.adad_dharrat; khana += 1) {
            if (takhtit.dharra_naw(khana) !== DHARRA_AMR) {
                continue;
            }
            const masdar = takhtit.dharra_khaam(khana);
            if (!masdar || !huwa_lawn(masdar)) {
                continue;
            }
            const ramz = raqm_dharra(masdar);
            if (ramz >= 0 && khana < mawaqi.length) {
                alwan.push({ mawqi: mawaqi[khana], ramz: ramz });
            }
        }
        return alwan;
    }

    /**
     * Where every lifted atom sat in the raw text this layout was made from.
     *
     * Found by searching for each atom's own recorded source, left to right,
     * from where the previous one ended. Exact, because `nasq` lifts atoms in
     * order and their sources do not overlap — and it stays exact without this
     * file knowing one thing about what an escape code looks like.
     */
    function mawaqi_khaam(takhtit: TakhtitKhaam, khaam: string): { mawqi: number; tul: number }[] {
        const mawaqi: { mawqi: number; tul: number }[] = [];
        let mashi = 0;
        for (let khana = 0; khana < takhtit.adad_dharrat; khana += 1) {
            const masdar = takhtit.dharra_khaam(khana);
            if (!masdar) {
                continue;
            }
            const mawqi = khaam.indexOf(masdar, mashi);
            if (mawqi < 0) {
                continue;
            }
            mawaqi.push({ mawqi: mawqi, tul: masdar.length });
            mashi = mawqi + masdar.length;
        }
        return mawaqi;
    }

    /** Lays one text piece out and reads its colour changes. */
    function juz_nass(
        hala: Nawat,
        khaam: string,
        bidaya: number,
        nihaya: number,
        hajm: number,
    ): JuzMarsum {
        const juzKhaam = khaam.substring(bidaya, nihaya);
        const takhtit = hala.saff.khattit_khaam_bi(
            juzKhaam,
            hala.lahja,
            hala.silsila,
            hajm,
            undefined,
            undefined,
            hala.khiyarat,
        );
        return {
            ayqouna: -1,
            khaamBidaya: bidaya,
            khaamNihaya: nihaya,
            takhtit: takhtit,
            alwan: alwan_takhtit(takhtit),
            dharrat: mawaqi_khaam(takhtit, juzKhaam),
            ard: takhtit.ard,
        };
    }

    /** The icon sheet's cell size, under either engine's spelling. */
    function hajm_ayqouna(): number {
        if (typeof ImageManager !== "undefined" && ImageManager && ImageManager.iconWidth) {
            return ImageManager.iconWidth;
        }
        const qadeem = (Window_Base as unknown as { _iconWidth?: number })._iconWidth;
        return typeof qadeem === "number" && qadeem > 0 ? qadeem : 32;
    }

    /**
     * Prepares a raw string for drawing, reusing a prepared one where possible.
     *
     * The whole string is laid out once to obtain its atom table. When it holds
     * no inline icon — which is most strings in most games — that one layout is
     * the answer and nothing is laid out twice. When it does, the layout is
     * discarded and each stretch between icons is laid out on its own, so the
     * icons get the room they occupy and no run is shaped across a picture.
     */
    function hayyi_satr(hala: Nawat, khaam: string, hajm: number): SatrMuhaddad {
        const miftah = khaam + String.fromCharCode(1) + String(hajm);
        const mahfuz = min_makhzan(miftah);
        if (mahfuz) {
            return mahfuz;
        }

        const kamil = hala.saff.khattit_khaam_bi(
            khaam,
            hala.lahja,
            hala.silsila,
            hajm,
            undefined,
            undefined,
            hala.khiyarat,
        );
        const dharrat = mawaqi_khaam(kamil, khaam);
        const ayqunat: { mawqi: number; tul: number; fahras: number }[] = [];
        let mashi = 0;
        for (let khana = 0; khana < kamil.adad_dharrat; khana += 1) {
            const masdar = kamil.dharra_khaam(khana);
            if (!masdar) {
                continue;
            }
            const mawqi = khaam.indexOf(masdar, mashi);
            if (mawqi < 0) {
                continue;
            }
            mashi = mawqi + masdar.length;
            if (kamil.dharra_naw(khana) === DHARRA_SURA) {
                const fahras = raqm_dharra(masdar);
                if (fahras >= 0) {
                    ayqunat.push({ mawqi: mawqi, tul: masdar.length, fahras: fahras });
                }
            }
        }

        const qiyasat = hala.silsila.qiyasat(hajm);
        const suud = qiyasat.suud;
        const irtifaSatr = qiyasat.irtifa_satr;
        qiyasat.free();

        const ajzaa: JuzMarsum[] = [];
        let yameen = kamil.yameen;
        if (ayqunat.length === 0) {
            ajzaa.push({
                ayqouna: -1,
                khaamBidaya: 0,
                khaamNihaya: khaam.length,
                takhtit: kamil,
                alwan: alwan_takhtit(kamil),
                dharrat: dharrat,
                ard: kamil.ard,
            });
        } else {
            kamil.free();
            const hajmAyqouna = hajm_ayqouna();
            let sabiq = 0;
            for (let khana = 0; khana < ayqunat.length; khana += 1) {
                const ayqouna = ayqunat[khana];
                if (ayqouna.mawqi > sabiq) {
                    const juz = juz_nass(hala, khaam, sabiq, ayqouna.mawqi, hajm);
                    yameen = yameen || (juz.takhtit ? juz.takhtit.yameen : false);
                    ajzaa.push(juz);
                }
                ajzaa.push({
                    ayqouna: ayqouna.fahras,
                    khaamBidaya: ayqouna.mawqi,
                    khaamNihaya: ayqouna.mawqi + ayqouna.tul,
                    alwan: [],
                    dharrat: [],
                    ard: hajmAyqouna + 4,
                });
                sabiq = ayqouna.mawqi + ayqouna.tul;
            }
            if (sabiq < khaam.length) {
                const juz = juz_nass(hala, khaam, sabiq, khaam.length, hajm);
                yameen = yameen || (juz.takhtit ? juz.takhtit.yameen : false);
                ajzaa.push(juz);
            }
        }

        let ard = 0;
        let irtifa = irtifaSatr;
        for (let khana = 0; khana < ajzaa.length; khana += 1) {
            ard += ajzaa[khana].ard;
            const takhtit = ajzaa[khana].takhtit;
            if (takhtit && takhtit.irtifa > irtifa) {
                irtifa = takhtit.irtifa;
            }
        }

        const satr: SatrMuhaddad = {
            miftah: miftah,
            khaam: khaam,
            ajzaa: ajzaa,
            ard: ard,
            irtifa: irtifa,
            suud: suud,
            yameen: yameen,
        };
        ila_makhzan(satr);
        return satr;
    }

    /**
     * How many bytes of a piece's clean text a raw reveal count reaches.
     *
     * Between atoms the two texts run character for character, and every atom
     * that has been fully revealed contributes exactly one three-byte object
     * replacement character in place of its own source. An atom the reveal has
     * only half-consumed counts for nothing, because half an escape code is not
     * a thing the reveal can show.
     */
    function kashf_naqi(satr: SatrMuhaddad, juz: JuzMarsum, kashf: number): number {
        const tulJuz = juz.khaamNihaya - juz.khaamBidaya;
        let raqmKhaam = kashf - juz.khaamBidaya;
        if (raqmKhaam <= 0) {
            return 0;
        }
        if (raqmKhaam > tulJuz) {
            raqmKhaam = tulJuz;
        }
        const juzKhaam = satr.khaam.substring(juz.khaamBidaya, juz.khaamBidaya + raqmKhaam);
        let bayt = tul_utf8(juzKhaam);
        for (let khana = 0; khana < juz.dharrat.length; khana += 1) {
            const dharra = juz.dharrat[khana];
            if (dharra.mawqi + dharra.tul <= raqmKhaam) {
                // Every escape code is ASCII, so its UTF-16 length and its
                // UTF-8 length are the same number.
                bayt = bayt - dharra.tul + 3;
            }
        }
        return bayt;
    }

    // -------------------------------------------------------------------------
    // Drawing
    // -------------------------------------------------------------------------

    /** The margin the scratch canvas keeps around a run for bearings and marks. */
    const HAMISH = 8;

    /** Tells a bitmap its pixels changed, under either engine's mechanism. */
    function alim_bi_taghyeer(hadaf: BitmapRpg): void {
        if (hadaf._setDirty) {
            hadaf._setDirty();
            return;
        }
        if (hadaf._baseTexture) {
            hadaf._baseTexture.update();
        }
    }

    /** The palette entry a colour code names, or the bitmap's current colour. */
    function lawn_min_ramz(
        nafidha: NafidhaRpg | undefined,
        ramz: number,
        asasi: string,
    ): string {
        if (ramz < 0 || !nafidha || !nafidha.textColor) {
            return asasi;
        }
        const lawn = nafidha.textColor(ramz);
        return typeof lawn === "string" && lawn.length > 0 ? lawn : asasi;
    }

    /**
     * Blits one text piece, one colour run at a time.
     *
     * Every glyph image in the atlas is white with its coverage in the alpha
     * channel, so colour is applied by compositing rather than by rasterizing
     * again: the run's glyphs go onto a scratch canvas, one `source-in` fill
     * tints all of them at once, and the tinted result is drawn onto the
     * destination. That is one extra composite per colour run rather than two
     * per glyph, which is the difference between a message window that keeps
     * sixty frames and one that does not.
     *
     * Returns how far the revealed glyphs reached, which is what the pause
     * cursor and the wait icon are positioned from.
     */
    function arsim_juz(
        hala: Nawat,
        hadaf: BitmapRpg,
        satr: SatrMuhaddad,
        juz: JuzMarsum,
        s: number,
        a: number,
        kashf: number,
        nafidha: NafidhaRpg | undefined,
    ): number {
        const takhtit = juz.takhtit;
        if (!takhtit) {
            return 0;
        }
        const hadd = kashf_naqi(satr, juz, kashf);
        if (hadd <= 0) {
            return 0;
        }
        const siyaqHadaf = siyaq_bitmap(hadaf);
        const lawh = hala.khadsh;
        if (!siyaqHadaf) {
            return 0;
        }
        const khatwa = hala.harf["Adad"];
        const huruf = takhtit.huruf;
        const adad = takhtit.adad_huruf;
        const hudud: NitaqLawn[] = [{ mawqi: 0, ramz: -1 }];
        for (let khana = 0; khana < juz.alwan.length; khana += 1) {
            hudud.push(juz.alwan[khana]);
        }
        const ardKhadsh = Math.ceil(juz.ard) + HAMISH * 2;
        const irtifaKhadsh = Math.ceil(satr.irtifa) + HAMISH * 2;
        const shaffafiya =
            typeof hadaf.paintOpacity === "number" ? hadaf.paintOpacity / 255 : 1;
        let balagh = 0;

        for (let nitaq = 0; nitaq < hudud.length; nitaq += 1) {
            const bidaya = hudud[nitaq].mawqi;
            const nihaya = nitaq + 1 < hudud.length ? hudud[nitaq + 1].mawqi : hadd;
            const mukhtara: number[] = [];
            for (let ghurfa = 0; ghurfa < adad; ghurfa += 1) {
                const asas = ghurfa * khatwa;
                const anqud = huruf[asas + hala.harf["Anqud"]];
                if (anqud >= bidaya && anqud < nihaya && anqud < hadd) {
                    mukhtara.push(asas);
                }
            }
            if (mukhtara.length === 0) {
                continue;
            }
            const siyaqKhadsh = khadsh_bi(hala, ardKhadsh, irtifaKhadsh);
            if (!siyaqKhadsh) {
                continue;
            }
            for (let khana = 0; khana < mukhtara.length; khana += 1) {
                const asas = mukhtara[khana];
                const gs = huruf[asas + hala.harf["S"]];
                const ga = huruf[asas + hala.harf["A"]];
                const mawdi = hala.lawha.shakl(
                    hala.silsila,
                    huruf[asas + hala.harf["Khatt"]],
                    huruf[asas + hala.harf["Muarrif"]],
                    takhtit.hajm,
                    0,
                );
                const nihayatHarf = gs + huruf[asas + hala.harf["Taqaddum"]];
                if (nihayatHarf > balagh) {
                    balagh = nihayatHarf;
                }
                if (mawdi.khali) {
                    mawdi.free();
                    continue;
                }
                const safha = safha_lawha(hala, mawdi.safha);
                if (safha) {
                    siyaqKhadsh.drawImage(
                        safha,
                        mawdi.s,
                        mawdi.a,
                        mawdi.ard,
                        mawdi.irtifa,
                        Math.round(gs + mawdi.izaha_s) + HAMISH,
                        Math.round(ga - mawdi.izaha_a) + HAMISH,
                        mawdi.ard,
                        mawdi.irtifa,
                    );
                }
                mawdi.free();
            }
            siyaqKhadsh.globalCompositeOperation = "source-in";
            siyaqKhadsh.fillStyle = lawn_min_ramz(nafidha, hudud[nitaq].ramz, hadaf.textColor);
            siyaqKhadsh.fillRect(0, 0, ardKhadsh, irtifaKhadsh);
            siyaqKhadsh.globalCompositeOperation = "source-over";

            siyaqHadaf.save();
            siyaqHadaf.globalAlpha = shaffafiya;
            siyaqHadaf.drawImage(
                lawh,
                0,
                0,
                ardKhadsh,
                irtifaKhadsh,
                Math.round(s) - HAMISH,
                Math.round(a) - HAMISH,
                ardKhadsh,
                irtifaKhadsh,
            );
            siyaqHadaf.restore();
        }
        return balagh;
    }

    /** Draws one inline icon from the engine's own icon sheet. */
    function arsim_ayqouna(hadaf: BitmapRpg, fahras: number, s: number, a: number): void {
        if (typeof ImageManager === "undefined" || !ImageManager) {
            return;
        }
        const waraq = ImageManager.loadSystem("IconSet");
        if (!waraq) {
            return;
        }
        const ard = hajm_ayqouna();
        const irtifa = ImageManager.iconHeight ? ImageManager.iconHeight : ard;
        const saf = Math.floor(fahras / 16);
        const amud = fahras - saf * 16;
        hadaf.blt(waraq, amud * ard, saf * irtifa, ard, irtifa, Math.round(s), Math.round(a));
    }

    /**
     * Draws a prepared line, revealing the first `kashf` code units of it.
     *
     * `s` is the left edge of the line box and `a` its top, whichever direction
     * the line runs in. Pieces are placed in logical order and the pen runs in
     * the line's own direction — leftwards from the right edge for a
     * right-to-left line — so an icon between two Arabic words lands between
     * them on screen and not at the far end.
     *
     * Returns the width actually covered, which the caller uses to place the
     * pause cursor and to anchor whatever it draws next.
     */
    function arsim_satr(
        hala: Nawat,
        hadaf: BitmapRpg,
        satr: SatrMuhaddad,
        s: number,
        a: number,
        kashf: number,
        nafidha: NafidhaRpg | undefined,
    ): number {
        jaddid_safahat(hala);
        const hajmAyqouna = hajm_ayqouna();
        let qalam = satr.yameen ? s + satr.ard : s;
        let mughatta = 0;
        for (let khana = 0; khana < satr.ajzaa.length; khana += 1) {
            const juz = satr.ajzaa[khana];
            const juzS = satr.yameen ? qalam - juz.ard : qalam;
            if (juz.ayqouna >= 0) {
                if (kashf > juz.khaamBidaya) {
                    arsim_ayqouna(
                        hadaf,
                        juz.ayqouna,
                        juzS + 2,
                        a + (satr.irtifa - hajmAyqouna) / 2,
                    );
                    mughatta = mughatta + juz.ard;
                }
            } else {
                const balagh = arsim_juz(hala, hadaf, satr, juz, juzS, a, kashf, nafidha);
                if (balagh > 0) {
                    mughatta = mughatta + Math.min(balagh, juz.ard);
                }
            }
            qalam = satr.yameen ? qalam - juz.ard : qalam + juz.ard;
        }
        alim_bi_taghyeer(hadaf);
        return mughatta;
    }

    // -------------------------------------------------------------------------
    // The six overrides
    // -------------------------------------------------------------------------

    /** The size a window is currently drawing at. */
    function hajm_nafidha(nafidha: NafidhaRpg): number {
        if (nafidha.contents && nafidha.contents.fontSize > 0) {
            return nafidha.contents.fontSize;
        }
        return 28;
    }

    /** The rest of the current line, from an index, without its newline. */
    function nass_far(nass: string, bidaya: number): string {
        const baqi = nass.substring(bidaya);
        const nihaya = baqi.indexOf("\n");
        return nihaya < 0 ? baqi : baqi.substring(0, nihaya);
    }

    /** The baseline the engine would have used inside a line box. */
    function asas_satr(a: number, irtifa: number, hajm: number): number {
        return a + irtifa / 2 + hajm * 0.35;
    }

    /**
     * Anchors a sub-line: prepares it, and decides which edge it starts from.
     *
     * A fresh right-to-left line starts at the trailing edge of the window's
     * content area, which is where a reader looks first. A sub-line opened
     * part-way through a line by a size change continues from wherever the pen
     * already was.
     */
    function athbit_halat(
        hala: Nawat,
        nafidha: NafidhaRpg,
        halat: HalatNass,
        bayanat: Bayanat,
    ): HalatTaarib | undefined {
        const hajm = hajm_nafidha(nafidha);
        const khaam = nass_far(halat.text, halat.index);
        if (khaam.length === 0) {
            return undefined;
        }
        const satr = hayyi_satr(hala, khaam, hajm);
        const yasar = typeof halat.left === "number" ? halat.left : halat.x;
        const mutah = ard_dakhili(nafidha);
        let s = halat.x;
        if (satr.yameen && bayanat.yameen && mutah > 0 && halat.x <= yasar) {
            s = yasar + Math.max(0, mutah - satr.ard);
        }
        const taarib: HalatTaarib = { bidaya: halat.index, s: s, a: halat.y, hajm: hajm, ard: 0 };
        halat.taarib = taarib;
        return taarib;
    }

    /**
     * Redraws the current sub-line up to the reveal point.
     *
     * The whole sub-line's box is cleared and drawn again on every step rather
     * than appended to, because a shaped line is not the concatenation of its
     * prefixes: revealing one more letter can change the form of the letter
     * before it, and appending would leave the old form behind. Clearing costs
     * one rectangle per revealed character in one window, which is what the
     * takeover rung buys correctness with.
     */
    function arsim_halat(
        hala: Nawat,
        nafidha: NafidhaRpg,
        halat: HalatNass,
        taarib: HalatTaarib,
    ): void {
        const khaam = nass_far(halat.text, taarib.bidaya);
        if (khaam.length === 0) {
            return;
        }
        const satr = hayyi_satr(hala, khaam, taarib.hajm);
        const irtifaSatr = halat.height > 0 ? halat.height : nafidha.lineHeight();
        const aAla = asas_satr(taarib.a, irtifaSatr, taarib.hajm) - satr.suud;
        const hadaf = nafidha.contents;
        hadaf.clearRect(
            Math.floor(taarib.s) - HAMISH,
            Math.floor(aAla) - HAMISH,
            Math.ceil(satr.ard) + HAMISH * 2,
            Math.ceil(satr.irtifa) + HAMISH * 2,
        );
        const kashf = halat.index - taarib.bidaya;
        const mughatta = arsim_satr(hala, hadaf, satr, taarib.s, aAla, kashf, nafidha);
        taarib.ard = mughatta;
        halat.x = satr.yameen ? taarib.s + satr.ard - mughatta : taarib.s + mughatta;
    }

    /** Installs everything the takeover rung needs. */
    function rakkib_istila(hala: Nawat, idadat: Idadat, bayanat: Bayanat): void {
        atbiq_khatt();

        // 1. The primitive. Every menu, gauge, button and item name goes here.
        Bitmap.prototype.drawText = function (
            this: BitmapRpg,
            nass: string,
            s: number,
            a: number,
            ard: number,
            irtifa: number,
            muhadhaha?: string,
        ): void {
            const makhtut = nass === undefined || nass === null ? "" : String(nass);
            if (makhtut.length === 0) {
                return;
            }
            const hajm = this.fontSize > 0 ? this.fontSize : 28;
            const satr = hayyi_satr(hala, makhtut, hajm);
            const mutah = ard > 0 ? ard : satr.ard;
            const marsum = Math.min(satr.ard, mutah);
            const mhd = muhadhaha_maqluba(muhadhaha, bayanat.yameen && satr.yameen);
            let sx = s;
            if (mhd === "center") {
                sx = s + (mutah - marsum) / 2;
            } else if (mhd === "right") {
                sx = s + mutah - marsum;
            }
            const aAla = asas_satr(a, irtifa, hajm) - satr.suud;
            arsim_satr(hala, this, satr, sx, aAla, Number.MAX_VALUE, undefined);
        };

        // 2. Measurement, from real shaped advances rather than from a character
        //    count, so the room a caller reserves is the room the text takes.
        Bitmap.prototype.measureTextWidth = function (this: BitmapRpg, nass: string): number {
            const makhtut = nass === undefined || nass === null ? "" : String(nass);
            if (makhtut.length === 0) {
                return 0;
            }
            const hajm = this.fontSize > 0 ? this.fontSize : 28;
            return hayyi_satr(hala, makhtut, hajm).ard;
        };

        Window_Base.prototype.textWidth = function (this: NafidhaRpg, nass: string): number {
            const makhtut = nass === undefined || nass === null ? "" : String(nass);
            if (makhtut.length === 0) {
                return 0;
            }
            return hayyi_satr(hala, makhtut, hajm_nafidha(this)).ard;
        };

        // 3. Line height, from the font's own metrics, never below the
        //    engine's own answer so that nothing already laid out moves.
        const irtifaAsli = Window_Base.prototype.calcTextHeight;
        Window_Base.prototype.calcTextHeight = function (
            this: NafidhaRpg,
            halat: HalatNass,
            kul?: boolean,
        ): number {
            const asasi = irtifaAsli ? irtifaAsli.call(this, halat, kul) : this.lineHeight();
            const khaam = nass_far(halat.text || "", halat.index);
            if (khaam.length === 0) {
                return asasi;
            }
            const satr = hayyi_satr(hala, khaam, hajm_nafidha(this));
            return Math.max(asasi, Math.ceil(satr.irtifa));
        };

        // 4. Rich text: the whole string, markup and all, laid out and blitted.
        Window_Base.prototype.drawTextEx = function (
            this: NafidhaRpg,
            nass: string,
            s: number,
            a: number,
            ard?: number,
        ): number {
            const makhtut = nass === undefined || nass === null ? "" : String(nass);
            if (this.resetFontSettings) {
                this.resetFontSettings();
            }
            const hajm = hajm_nafidha(this);
            const sutur = makhtut.split("\n");
            const irtifaSatr = this.lineHeight();
            const mutah = ard && ard > 0 ? ard : ard_dakhili(this);
            let aqsa = 0;
            let ay = a;
            for (let khana = 0; khana < sutur.length; khana += 1) {
                const satr = hayyi_satr(hala, sutur[khana], hajm);
                let sx = s;
                if (satr.yameen && bayanat.yameen && mutah > 0) {
                    sx = s + Math.max(0, mutah - satr.ard);
                }
                const aAla = asas_satr(ay, irtifaSatr, hajm) - satr.suud;
                arsim_satr(hala, this.contents, satr, sx, aAla, Number.MAX_VALUE, this);
                if (satr.ard > aqsa) {
                    aqsa = satr.ard;
                }
                ay += irtifaSatr;
            }
            return aqsa;
        };

        // 5. The reveal. The engine's own escape parser still runs, so waits,
        //    the gold window and colour changes behave exactly as they did; only
        //    the drawing is ours, and the icon drawing is suppressed below
        //    because the prepared line already places icons.
        const harfAsli = Window_Base.prototype.processCharacter;
        Window_Base.prototype.processCharacter = function (
            this: NafidhaRpg,
            halat: HalatNass,
        ): void {
            const harf = halat.text.charAt(halat.index);
            if (harf === "") {
                return;
            }
            if (harf.charCodeAt(0) < 0x20) {
                halat.index += 1;
                halat.taarib = undefined;
                this.processControlCharacter(halat, harf);
                return;
            }
            let taarib = halat.taarib;
            if (!taarib || taarib.hajm !== hajm_nafidha(this)) {
                taarib = athbit_halat(hala, this, halat, bayanat);
            }
            if (!taarib) {
                harfAsli.call(this, halat);
                return;
            }
            if (harf === "\\" && this.obtainEscapeCode && this.processEscapeCharacter) {
                halat.index += 1;
                const ramz = this.obtainEscapeCode(halat);
                this.processEscapeCharacter(ramz, halat);
            } else {
                halat.index += 1;
            }
            arsim_halat(hala, this, halat, taarib);
        };

        Window_Base.prototype.processDrawIcon = function (): void {
            // The prepared line draws icons where shaping put them. Letting the
            // engine draw a second one at its own pen position would put the
            // same icon on screen twice, in two places.
        };

        // 6. One frame boundary per rendered frame, which is what makes the
        //    atlas's eviction safe: everything this frame referenced is pinned
        //    until the next call.
        const mashhadAsli = SceneManager.renderScene;
        if (mashhadAsli) {
            SceneManager.renderScene = function (): void {
                hala.lawha.ibda_itar();
                mashhadAsli.call(SceneManager);
            };
        } else {
            anthir("SceneManager.renderScene is absent; the atlas cannot pin per frame");
        }

        iqlib_nawafidh(idadat, bayanat);
        sajjil("rung 3: text is drawn from Taarib's atlas, core " + hala.wasm.isdar());
    }

    // -------------------------------------------------------------------------
    // Bringing the core up
    // -------------------------------------------------------------------------

    /** Builds the engine, the font chain, the atlas and the recorded decisions. */
    function jahhiz(
        wasm: NawatTaarib,
        khattBayt: Uint8Array,
        idadat: Idadat,
        bayanat: Bayanat,
    ): Nawat {
        wasm.hayyi();
        const saff = new wasm.TaaribSaff();
        const khatt = wasm.TaaribKhatt.min_bayt(khattBayt, 0);
        const silsila = new wasm.TaaribSilsila([khatt]);

        const khiyaratLawha = new wasm.TaaribKhiyaratLawha();
        khiyaratLawha.aqsa_ard = bayanat.aqsaArd;
        khiyaratLawha.aqsa_irtifa = bayanat.aqsaIrtifa;
        khiyaratLawha.hashw = bayanat.hashw;
        khiyaratLawha.mizaniyat_bayt = bayanat.mizaniyatBayt;
        const lawha = new wasm.TaaribLawha(khiyaratLawha);
        khiyaratLawha.free();

        const khiyarat = new wasm.TaaribKhiyarat();
        khiyarat.lugha = idadat.lugha;
        khiyarat.arqam = idadat.arqam;
        khiyarat.tashkeel = idadat.tashkeel;
        khiyarat.dabt = idadat.dabt;
        khiyarat.muhadhaha = bayanat.muhadhaha;
        khiyarat.hiwar = true;

        return {
            wasm: wasm,
            saff: saff,
            silsila: silsila,
            lawha: lawha,
            khiyarat: khiyarat,
            harf: wasm.HaqlHarf,
            satr: wasm.HaqlSatr,
            lahja: wasm.Lahja["RpgMaker"],
            safahat: [],
            adadSafahat: lawha.adad_safahat,
            khadsh: document.createElement("canvas"),
        };
    }

    /**
     * Loads the core, or reports why it could not be loaded.
     *
     * Every failure ends in `tamam(undefined)` and a console line naming the
     * cause, never in an exception that leaves the game with half a text
     * pipeline. The commonest cause by far is a runtime with no `WebAssembly` at
     * all — RPG Maker MV 1.5 ships NW.js 0.12, whose Chromium predates it by two
     * years — and that is a descent, not a defect.
     */
    function hammil_nawat(
        idadat: Idadat,
        bayanat: Bayanat,
        tamam: (hala: Nawat | undefined) => void,
    ): void {
        if (typeof WebAssembly === "undefined") {
            anthir("this runtime has no WebAssembly; the takeover rung cannot start here");
            tamam(undefined);
            return;
        }
        if (!idadat.khatt) {
            anthir("the takeover rung needs a font file and none is configured");
            tamam(undefined);
            return;
        }
        const fashal = function (sabab: string): void {
            anthir(sabab);
            tamam(undefined);
        };
        hammil_barnamaj(function (): void {
            if (typeof wasm_bindgen === "undefined" || !wasm_bindgen) {
                fashal(GLUE + " loaded and defined no core");
                return;
            }
            const wasm = wasm_bindgen;
            hammil_bayt(
                WASM,
                function (bayt: Uint8Array): void {
                    const jism = bayt.buffer;
                    wasm(jism as ArrayBuffer).then(
                        function (): void {
                            hammil_bayt(
                                MUJALLAD + idadat.khatt,
                                function (khattBayt: Uint8Array): void {
                                    try {
                                        tamam(jahhiz(wasm, khattBayt, idadat, bayanat));
                                    } catch (khata) {
                                        fashal("the core refused its inputs: " + String(khata));
                                    }
                                },
                                fashal,
                            );
                        },
                        function (khata: unknown): void {
                            fashal("the core module did not instantiate: " + String(khata));
                        },
                    );
                },
                fashal,
            );
        }, fashal);
    }

    // -------------------------------------------------------------------------
    // Boot
    // -------------------------------------------------------------------------

    /** A copy of the settings with the rung lowered, for a run-time descent. */
    function bi_rutba(idadat: Idadat, rutba: number): Idadat {
        return {
            rutba: rutba,
            khatt: idadat.khatt,
            bayanat: idadat.bayanat,
            lugha: idadat.lugha,
            arqam: idadat.arqam,
            tashkeel: idadat.tashkeel,
            dabt: idadat.dabt,
            nawafidhYameen: idadat.nawafidhYameen,
        };
    }

    /** Reads the settings and the patch data, then installs the right rung. */
    function iqla(): void {
        const idadat = qira_idadat();
        const rakkib = function (bayanat: Bayanat): void {
            rakkib_khatt(idadat);
            if (idadat.rutba < 3) {
                rakkib_tasheeh(idadat, bayanat);
                return;
            }
            hammil_nawat(idadat, bayanat, function (hala: Nawat | undefined): void {
                if (!hala) {
                    anthir(
                        "the takeover rung could not start; the corrected-shaping rung is " +
                            "installed instead, which leaves the engine's own drawing in place",
                    );
                    rakkib_tasheeh(bi_rutba(idadat, 2), bayanat);
                    return;
                }
                rakkib_istila(hala, idadat, bayanat);
            });
        };
        if (idadat.bayanat) {
            hammil_nass(
                MUJALLAD + idadat.bayanat,
                function (khaam: string): void {
                    rakkib(qira_bayanat(khaam));
                },
                function (sabab: string): void {
                    anthir(sabab + "; the patch's runtime defaults are used");
                    rakkib(bayanat_iftiradiya());
                },
            );
            return;
        }
        rakkib(bayanat_iftiradiya());
    }

    iqla();
})();








