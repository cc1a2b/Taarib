import { motion } from 'motion/react';
import type { CSSProperties, JSX, MouseEvent, KeyboardEvent } from 'react';
import { memo, useCallback, useState } from 'react';

import type { MiftahLugha } from '@/lugha/lugha';
import type { JahiziyaTashghil } from '@/maktaba/jahiziya';

import type {
  HalatLughaRasmiya,
  LawhaBadila,
  LawnBariz,
  Lugha,
  SatrUnwan,
} from '@/mustalahat/awamir';
import { t } from '@/lugha/lugha';
import { HARAKAT_HALA, haraka } from '@/nizam/haraka';

import './bitaqa.css';

/**
 * البطاقة — one game, and the product's visual signature.
 *
 * ## The governing rule
 *
 * **Every card occupies identical outer dimensions and shows identically sized
 * artwork, in every state.** A game with nothing, a game with a text patch, a
 * game with a voice pack, and a game with both all present the same 314×192
 * footprint and the same 292×136 of artwork inside it. (The figure said 240×394
 * here long after the card became landscape, which is the shape `tokens.css`
 * declares and the shape every number in `bitaqa.css` is derived from.)
 *
 * The status frames are taken from *inside* the card — out of a reserve that is
 * always present in the artwork well — and never from the layout around it. The
 * consequence is the point: a library grid never reflows, never jitters, and
 * never shifts as patch availability resolves in from the registry. A state
 * change is colour appearing in space that was already there.
 *
 * This is why {@link BitaqaLuba} has no conditional sizing anywhere and why the
 * reserve is drawn even when both rings are transparent. A component that
 * collapsed the reserve when unused would be a component that reflowed the grid
 * the moment a registry lookup returned, which for a ten-thousand-game library
 * is every card moving while the user is trying to click one.
 *
 * ## The caption
 *
 * The band under the artwork is the card's caption, not a plinth for the tabs:
 * the game's name at the leading edge, the status pair at the trailing edge.
 * It used to hold the tabs alone, which meant that a library with nothing
 * patched yet — every library on first run — drew a row of cards each with an
 * empty strip along the bottom.
 *
 * It is also the only place the name survives. Set on the generated plate it
 * disappeared the moment real artwork decoded, so a game whose cover does not
 * spell its own title became a picture with no name under it.
 *
 * The name's measure is the one thing a patch state changes, and it changes
 * nothing outside the card: the tab slot is as wide as the widest tab actually
 * drawn, so a game with nothing gives its whole band to its name. The card's
 * box, the artwork and the band's height are identical in all four states.
 *
 * ## The two rings
 *
 * Concentric, inside the reserve. The **outer** ring is voice and is a muted
 * steel blue; the **inner** ring is text and is a muted signal green. Outer for
 * voice is not arbitrary — it is why a card with both shows the blue *outside*
 * the green, which is the layering the direction specifies.
 *
 * ## Availability is not installation
 *
 * The two are different facts and must be distinguishable without a legend:
 *
 * - **available, not installed** — the ring is a hairline outline in its colour,
 *   with no fill weight, and the tab is present but unfilled with its colour
 *   carried only in the label and border;
 * - **installed** — the ring is drawn at full weight and the tab is solid with
 *   its label in the tab's own contrast colour;
 * - **an update is available** — full weight, plus one small mark on the tab.
 *   Not an animation, not a badge circle, not a dot floating outside the card.
 *
 * ## The two marks, and why they are not a third patch state
 *
 * A strip across the top of the artwork carries what the card knows about the
 * *game* rather than about its patches: at the trailing edge, that the
 * publisher already ships Arabic; at the leading edge, that Taarib cannot yet
 * drive this game's engine, so a patch installed today would change the files
 * and nothing on screen. Both are absolutely positioned inside the reserve and
 * cost the card no space, and neither is allowed near the tabs — the band's
 * trailing edge means patch state, and anything put beside the tabs is read as
 * one.
 *
 * ## The calm state
 *
 * A game with no translation in the registry has no rings and no tabs — artwork
 * and the card, nothing else. Most of a large library is in this state and the
 * grid has to look good full of them, so it is deliberately unremarkable rather
 * than marked as missing something.
 *
 * ## Two coordinate spaces, and which one this component speaks
 *
 * {@link KhasaisBitaqa.ala_qaima} reports a **physical** viewport point, and
 * says so in its own documentation, because that is the only thing this
 * component can honestly measure: `clientX` and `getBoundingClientRect()` are
 * both distances from the viewport's left edge in every direction, and a card
 * has no way to know what box its consumer will position a menu inside. Turning
 * that into an `inset-inline-start` — which measures from the right edge under
 * `dir="rtl"` — needs the width of the *containing block* of the element being
 * placed, and only the consumer knows which block that is. `shashat/maktaba.tsx`
 * owns the conversion for the one menu it anchors to the viewport.
 */

/** Whether a patch of one kind exists, and how far along the user is with it. */
export type HalatRuqaa = 'la-shay' | 'mutaha' | 'mutabbaqa' | 'tahdith';

/** How large the grid is drawing cards right now. */
export type KathafatBitaqa = 'mudmaj' | 'qiyasi' | 'kabir';

/** Everything one card needs to draw itself. */
export interface KhasaisBitaqa {
  /** Taarib's identity for the game, used as the key and the hit target's id. */
  readonly muarrif: string;
  /** The game's display name, exactly as its launcher gives it. */
  readonly ism: string;
  /**
   * The cover, as a source this document can already load, or null.
   *
   * Already converted — `convertFileSrc` on a Tauri path, or an `https:` URL —
   * because the conversion depends on the shell the interface is running in and
   * a card is not the part of the product that knows which shell that is.
   */
  readonly ghilaf: string | null;
  /**
   * The generated plate, present whenever {@link ghilaf} is null.
   *
   * Computed in Rust, in `taarib-mustalahat::lawha_badila`, because the
   * wrapping and size-step rules have to produce one answer and three
   * interfaces each deciding when to drop a step would be three different
   * grids.
   */
  readonly lawha: LawhaBadila | null;
  /**
   * The cover's dominant colour, already extracted and clamped for legibility,
   * or absent.
   *
   * Optional because the artwork cascade supplies it and the card must draw
   * correctly before it does — every rule that reads it carries the neutral
   * hairline as its fallback, so a card that is never given one is not a card
   * that looks unfinished. Used for one thing only: the mount around the
   * artwork takes the artwork's own hue while the pointer is on it. At rest
   * the grid stays neutral, because a wall of per-game accents is a wall with
   * no accent in it.
   */
  readonly lawn_ghilaf?: LawnBariz | null;
  /** The text patch's state. */
  readonly nass: HalatRuqaa;
  /** The voice pack's state. */
  readonly sawt: HalatRuqaa;
  /**
   * What the publisher already ships, when the library scan established it.
   *
   * Optional because it arrives from the same row as everything else and a
   * consumer that has not been updated to pass it should draw a correct card
   * rather than an empty badge. `'ghaib'` and absence are the same card: no
   * badge. Most of a library is in that state and the grid has to look good
   * full of it, which is why the badge marks the exception rather than
   * labelling the rule.
   */
  readonly lugha_rasmiya?: HalatLughaRasmiya | null;
  /**
   * Whether Taarib can actually patch this game's engine in this build.
   *
   * A fact about Taarib rather than about the game, and the reason the card
   * carries it at all: without it a grid of cards presents a game nothing will
   * change exactly like a game that will be translated end to end, and the
   * person finds out after they have run the whole flow. Optional and nullable
   * on the same terms as {@link lugha_rasmiya} — a consumer that has not been
   * given the verdict draws a card with no mark rather than a mark with no
   * verdict, because a card that guessed would be the same lie in a new place.
   */
  readonly jahiziya?: JahiziyaTashghil | null;
  /** Whether this card is in the current selection. */
  readonly mukhtara: boolean;
  /** The grid's current density. */
  readonly kathafa: KathafatBitaqa;
  /** The language of the current session, for the accessible name and tab labels. */
  readonly lugha: Lugha;
  /** Opens the game. */
  readonly ala_fath: (muarrif: string) => void;
  /** Toggles or extends the selection. */
  readonly ala_ikhtiyar: (muarrif: string, hadath: MouseEvent | KeyboardEvent) => void;
  /**
   * Opens the context menu at a point.
   *
   * `s` and `a` are **physical viewport coordinates** — distances from the
   * viewport's left and top edges, in both reading directions, exactly as the
   * pointer event and `getBoundingClientRect` report them. A consumer that
   * places the menu with `inset-inline-start` must mirror `s` itself under
   * `dir="rtl"`; see the note on coordinate spaces at the top of this file for
   * why the card cannot do that for it.
   */
  readonly ala_qaima: (muarrif: string, s: number, a: number) => void;
}

/** Whether a state paints its ring at all. */
function yarsim(hala: HalatRuqaa): boolean {
  return hala !== 'la-shay';
}

/**
 * The badge's wording, per verdict.
 *
 * `ghaib` has no entry on purpose: a game with no official Arabic is the normal
 * case, and a grid where every card carries a "No Arabic" chip is a grid that
 * has labelled its own baseline. The badge exists for the exception.
 */
const WASM_LUGHA: Readonly<Partial<Record<HalatLughaRasmiya, MiftahLugha>>> = {
  wajiha_faqat: 'bitaqa.lugha.wajiha',
  nusus_faqat: 'bitaqa.lugha.nusus',
  kamila: 'bitaqa.lugha.kamila',
  mubhama: 'bitaqa.lugha.mubhama',
};

/**
 * The stamp on a game that already speaks Arabic.
 *
 * Drawn inside the artwork, in the trailing top corner, and absolutely
 * positioned like the rings: it takes no space from the caption, so the game's
 * name keeps its whole measure and the card's box, artwork and band are
 * identical in every state — the rule this file exists to enforce.
 *
 * It is set in the corner of the picture rather than in the status band because
 * of what the band means. The trailing edge of the band is where patch state
 * lives, and a third chip beside the two tabs would be read as a third patch
 * state. This is not a patch state; it is a fact about the game.
 *
 * Neutral ground and a hairline, with the accent carried on one edge. Not
 * `--najah`, which would claim an outcome, and not `--tanbeeh` or `--khatar`,
 * which would tell an Arabic-speaking player that a game they can already read
 * is a problem.
 */
function WasmLugha(khasais: {
  readonly hala: HalatLughaRasmiya;
  readonly lugha: Lugha;
}): JSX.Element | null {
  const miftah = WASM_LUGHA[khasais.hala];
  if (miftah === undefined) return null;
  return (
    <span className="bitaqa__wasm-lugha" aria-hidden="true">
      {t(miftah, khasais.lugha)}
    </span>
  );
}

/**
 * The mark's wording, per verdict — the short form on the picture and the long
 * form the card is announced with.
 *
 * `mukammala` has no entry, and that is the whole restraint of this mark: a
 * card whose engine works says nothing about it, exactly as a card with no
 * official Arabic says nothing about that. The mark is for the exception even
 * though the exception is, in this build, most of the library — because the day
 * an adapter lands, every card it covers must go quiet on its own.
 */
const WASM_JAHIZIYA: Readonly<
  Partial<Record<JahiziyaTashghil, { readonly wasm: MiftahLugha; readonly wasf: MiftahLugha }>>
> = {
  naqisa: { wasm: 'bitaqa.jahiziya.naqisa', wasf: 'bitaqa.jahiziya.naqisa_wasf' },
  ghaiba: { wasm: 'bitaqa.jahiziya.ghaiba', wasf: 'bitaqa.jahiziya.ghaiba_wasf' },
};

/**
 * The mark on a game whose engine Taarib cannot fully drive yet.
 *
 * Set on the artwork rather than in the caption band, for the reason
 * {@link WasmLugha} is: the band's trailing edge is where *patch* state lives,
 * and a third chip beside the two tabs is read as a third patch state. This is
 * not one. It is a fact about what this build of Taarib can do, and it belongs
 * on the picture of the game beside the other such fact.
 *
 * Amber and not red, and a hairline rather than a fill. `--khatar` on a hundred
 * cards would tell a user their library is broken; it is not — the games are
 * fine and the adapter is unwritten, which is a caution and is temporary. The
 * weight is deliberately below {@link WasmLugha}'s: the label sits in
 * `--nass-2` where that one sits in `--nass-1`, so a grid where every card
 * carries this one still reads as a grid of covers rather than as a wall of
 * warnings.
 */
function WasmJahiziya(khasais: {
  readonly hala: JahiziyaTashghil;
  readonly lugha: Lugha;
}): JSX.Element | null {
  const wasm = WASM_JAHIZIYA[khasais.hala];
  if (wasm === undefined) return null;
  return (
    <span className="bitaqa__wasm-jahiziya" aria-hidden="true">
      {t(wasm.wasm, khasais.lugha)}
    </span>
  );
}

/** Whether a state paints at full weight. */
function murakkaba(hala: HalatRuqaa): boolean {
  return hala === 'mutabbaqa' || hala === 'tahdith';
}

/**
 * The class list for one ring.
 *
 * Three states, three classes, and no inline style: the ring's colour and
 * weight are both token-driven, and a component that computed a shadow string
 * would be a component the light theme could not re-point.
 */
function tabaqatHalqa(asas: string, hala: HalatRuqaa): string {
  if (!yarsim(hala)) return `${asas} ${asas}--khafiya`;
  return murakkaba(hala) ? `${asas} ${asas}--murakkaba` : `${asas} ${asas}--mutaha`;
}

/**
 * One status tab.
 *
 * Absent, not empty, when its state is `la-shay`. The band's height is declared
 * on the band itself rather than produced by whatever it contains, so a tab
 * that stopped occupying space costs nothing vertically: the band is the same
 * height whether it holds two tabs, one, or none. Horizontally the width it
 * gives up goes to the game's name, which is inside the same card and visible
 * to nothing outside it.
 */
function Lisan(khasais: {
  readonly naw: 'nass' | 'sawt';
  readonly hala: HalatRuqaa;
  readonly unwan: string;
}): JSX.Element | null {
  const { naw, hala, unwan } = khasais;
  if (!yarsim(hala)) return null;

  const asas = `bitaqa__lisan bitaqa__lisan--${naw}`;
  const wazn = murakkaba(hala) ? `${asas} ${asas}-musmat` : `${asas} ${asas}-mafrugh`;

  return (
    <span className={wazn} aria-hidden="true">
      <span className="bitaqa__lisan-nass">{unwan}</span>
      {hala === 'tahdith' ? <span className="bitaqa__alamat-tahdith" /> : null}
    </span>
  );
}

/**
 * Three channels as `#rrggbb`.
 *
 * The wire form of a colour is three integers, because that is what the rest of
 * the product does arithmetic on. CSS wants a string, and building it here is
 * the whole conversion — sending a second, redundant hex field from Rust would
 * be two representations of one value that could disagree.
 */
function sittasi(lawn: LawnBariz): string {
  const qanat = (q: number): string => q.toString(16).padStart(2, '0');
  return `#${qanat(lawn.ahmar)}${qanat(lawn.akhdar)}${qanat(lawn.azraq)}`;
}

/**
 * The plate's drawn width, as a `calc` term over the card's own tokens.
 *
 * The plate fills the artwork well, and the well is the card's declared width
 * less its own hairline on each side and less the reserve on each side.
 * Written as the expression rather than as the pixel value it evaluates to,
 * for the reason `bitaqa.css` states about every other number in the card: a
 * literal would be correct at exactly one of the three card sizes. A term and
 * not a whole value, so it composes into one `calc()` below rather than
 * nesting a second one inside it.
 */
const ARD_MARSUM = '(var(--ard-bitaqa) - 2 * var(--samk-hadd) - 2 * var(--ihtiyat-halqa))';

/**
 * The drawn width the title was fitted to on the way here.
 *
 * `taarib-mustalahat::lawha_badila::saat_al_satr` wraps against 220 device
 * pixels, which is the well at the standard density and nowhere else. Setting
 * the returned point size verbatim therefore overflowed the compact card by
 * half — a 28pt title laid out for 292px, set into 198px — and left the large
 * one a third emptier than it was designed to be. The size travels as a ratio
 * of the width it was chosen for, so the plate is the same picture at all three.
 */
const ARD_MARSUM_ASAS = 292;

/**
 * The title's size at the current density, as CSS.
 *
 * Cached because `KHUTUWAT_HAJM` has four entries and this is called once per
 * visible card on every scroll frame: the same four strings, rebuilt forty
 * times a frame, is work with no result.
 */
const AHJAM_MARSUMA = new Map<number, string>();

function hajmMarsum(hajm: number): string {
  const mukhazzan = AHJAM_MARSUMA.get(hajm);
  if (mukhazzan !== undefined) {
    return mukhazzan;
  }
  const qeema = `calc(${ARD_MARSUM} * ${String(hajm)} / ${String(ARD_MARSUM_ASAS)})`;
  AHJAM_MARSUMA.set(hajm, qeema);
  return qeema;
}

/**
 * The style object a card with no extracted cover colour carries.
 *
 * One shared frozen instance rather than a fresh `{}` per render: the grid
 * rebuilds its whole window on every scroll frame, and a new object identity
 * on a prop React compares by reference is a diff on every card, every frame,
 * for a value that is empty.
 */
const BILA_LAWN: CSSProperties = Object.freeze({});

const HARF = /\p{L}/u;
const HARF_ARABI = /\p{Script=Arabic}/u;

/**
 * Which script the title is set in, taken from its first letter.
 *
 * Not cosmetic bookkeeping: the two scripts want opposite settings and one of
 * them is destructive. Latin display sizes want a little negative tracking, and
 * the same tracking applied to Arabic breaks the joins between letters — the
 * word comes apart into disconnected glyphs. So the plate has to know, and the
 * first letter is the honest answer for a title that is one language with the
 * occasional Latin numeral in it, which is what game titles are.
 */
function khattUnwan(sutur: readonly SatrUnwan[]): 'arabi' | 'latini' {
  for (const satr of sutur) {
    for (const harf of satr.nass) {
      if (!HARF.test(harf)) {
        continue;
      }
      return HARF_ARABI.test(harf) ? 'arabi' : 'latini';
    }
  }
  return 'latini';
}

/**
 * Leading and tracking per script.
 *
 * Arabic is set looser and never tracked: the face carries its marks above and
 * below the line, and at 1.14 they touch the line above at the two larger size
 * steps. Latin is set tight, which is what a title reads as at display size
 * rather than as a paragraph that happens to be three lines long.
 */
const DABT_KHATT = {
  arabi: { irtifa: 1.32, tabaud: 'normal' },
  latini: { irtifa: 1.14, tabaud: '-0.015em' },
} as const;

/**
 * The generated plate, drawn when no artwork resolved.
 *
 * The title's lines and its point size arrive already decided from Rust, and
 * this component does not wrap, does not measure and does not choose a size:
 * doing any of those here would be a second layout rule that could disagree
 * with the one in `taarib-mustalahat::lawha_badila`. What it does own is how
 * the decided lines are *set* — the size scaled to the well it is actually
 * being drawn into, the leading and tracking the title's own script needs, and
 * the direction each line resolves in.
 *
 * Hidden from assistive technology in full. The plate stands in for cover art,
 * cover art is `alt=""`, and every word on it — the game's name, the launcher's
 * name — is already in the card's accessible name or in the section heading
 * above it. Announced, it would be the card read twice.
 */
function LawhatBadila(khasais: { readonly lawha: LawhaBadila }): JSX.Element {
  const { lawha } = khasais;
  const lawn = sittasi(lawha.lawn);
  const khatt = khattUnwan(lawha.sutur);
  const dabt = DABT_KHATT[khatt];

  return (
    <div
      className="bitaqa__lawha"
      aria-hidden="true"
      data-khatt={khatt}
      data-sutur={String(lawha.sutur.length)}
      style={
        {
          // The legitimate inline styles in this component, and the reason each
          // one is here: the field colour is derived per game from its identity
          // and cannot be a token, and the title's size is derived per title
          // from a layout Rust already performed. Both are data, not styling.
          // The field is also published as a custom property so the stylesheet
          // can mix a companion tone out of it — a hairline, a rule — which is
          // the only way CSS can reach a colour it did not declare.
          backgroundColor: lawn,
          color: lawha.nass_fatih ? 'var(--nass-1)' : 'var(--sath-0)',
          '--lawn-lawha': lawn,
          fontSize: hajmMarsum(lawha.hajm),
        } as CSSProperties
      }
    >
      {lawha.sutur.length === 0 ? null : (
        <span
          className="bitaqa__lawha-unwan"
          style={{ lineHeight: dabt.irtifa, letterSpacing: dabt.tabaud }}
        >
          {lawha.sutur.map((satr, martaba) => (
            // Keyed by position, because the lines are a layout and never
            // reorder — and two lines of one title can be the same text.
            //
            // `dir="auto"` per line rather than on the block: it makes each
            // line resolve from its own first strong character, which is what
            // puts a Latin title the right way round inside an Arabic document
            // and — the case that matters — puts the ellipsis on a truncated
            // line at the end the reader actually finishes at, on the left for
            // Latin and on the right for Arabic. Rust appends U+2026 to the
            // string; where it lands is decided here.
            <span
              key={`${String(martaba)}:${satr.nass}`}
              className="bitaqa__lawha-satr"
              dir="auto"
            >
              {satr.nass}
            </span>
          ))}
        </span>
      )}
      <span className="bitaqa__lawha-matjar" dir="auto">
        {lawha.alamat_matjar}
      </span>
    </div>
  );
}

/**
 * One game card.
 *
 * The whole card is a single hit target. The tabs are **status, not controls**,
 * and carry `aria-hidden` because their meaning is already in the card's
 * accessible name — a screen reader that announced "Arabic, voice" as two
 * unlabelled buttons after the game's name would be announcing two controls
 * that do not exist.
 */
function BitaqaLubaBila(khasais: KhasaisBitaqa): JSX.Element {
  const {
    muarrif,
    ism,
    ghilaf,
    lawha,
    lawn_ghilaf,
    nass,
    sawt,
    lugha_rasmiya,
    jahiziya,
    mukhtara,
    kathafa,
    lugha,
    ala_fath,
    ala_ikhtiyar,
    ala_qaima,
  } = khasais;

  // Which cover has decoded, and which one the engine refused — both held as
  // the source itself rather than as booleans. `ghilaf` changes under a mounted
  // card when the artwork cascade resolves, and a boolean would still be
  // describing the previous image: the card would either cross-fade a source it
  // has not loaded yet, or go on suppressing one that has since been replaced.
  const [ghilafZahir, haddidZahir] = useState<string | null>(null);
  const [ghilafFashil, haddidFashil] = useState<string | null>(null);

  const alaHiml = useCallback(() => {
    haddidZahir(ghilaf);
  }, [ghilaf]);

  const alaKhataGhilaf = useCallback(() => {
    haddidFashil(ghilaf);
    haddidZahir((sabiq) => (sabiq === ghilaf ? null : sabiq));
  }, [ghilaf]);

  const alaNaqr = useCallback(
    (hadath: MouseEvent<HTMLElement>) => {
      // A modifier means the user is building a selection, not opening a game.
      if (hadath.shiftKey || hadath.ctrlKey || hadath.metaKey) {
        ala_ikhtiyar(muarrif, hadath);
        return;
      }
      ala_fath(muarrif);
    },
    [muarrif, ala_fath, ala_ikhtiyar],
  );

  const alaMiftah = useCallback(
    (hadath: KeyboardEvent<HTMLElement>) => {
      if (hadath.key === 'Enter') {
        hadath.preventDefault();
        ala_fath(muarrif);
        return;
      }
      if (hadath.key === ' ') {
        hadath.preventDefault();
        ala_ikhtiyar(muarrif, hadath);
        return;
      }
      // The dedicated context-menu key, and Shift+F10, which is the same
      // affordance on keyboards that lack it.
      if (hadath.key === 'ContextMenu' || (hadath.shiftKey && hadath.key === 'F10')) {
        hadath.preventDefault();
        // The card's centre, which is the one anchor that needs no direction to
        // compute: an edge would have to be chosen as leading or trailing, and
        // this component deliberately reports physical coordinates only.
        const sunduq = hadath.currentTarget.getBoundingClientRect();
        ala_qaima(muarrif, sunduq.left + sunduq.width / 2, sunduq.top + sunduq.height / 2);
      }
    },
    [muarrif, ala_fath, ala_ikhtiyar, ala_qaima],
  );

  const alaQaima = useCallback(
    (hadath: MouseEvent<HTMLElement>) => {
      hadath.preventDefault();
      ala_qaima(muarrif, hadath.clientX, hadath.clientY);
    },
    [muarrif, ala_qaima],
  );

  // Announced as one name. The two patch states are appended as words rather
  // than left to the rings, which convey nothing to a screen reader.
  const halat: string[] = [];
  // The publisher's own Arabic first: it is the fact that decides whether the
  // rest of this card's states are even offered.
  const miftahLugha =
    lugha_rasmiya === undefined || lugha_rasmiya === null ? undefined : WASM_LUGHA[lugha_rasmiya];
  if (miftahLugha !== undefined) {
    halat.push(t(miftahLugha, lugha));
  }
  if (yarsim(nass)) {
    halat.push(t(murakkaba(nass) ? 'bitaqa.nass.mutabbaqa' : 'bitaqa.nass.mutaha', lugha));
  }
  if (yarsim(sawt)) {
    halat.push(t(murakkaba(sawt) ? 'bitaqa.sawt.mutabbaqa' : 'bitaqa.sawt.mutaha', lugha));
  }
  // Last, and in its long form. It qualifies everything said before it — a
  // patch can be available for a game and still change nothing on screen — so
  // it has to be the phrase the listener finishes on, and it has to be a whole
  // clause: "not running yet" after two patch states would be heard as a third
  // patch state, which is the confusion the mark is placed away from the tabs
  // to avoid in the first place.
  const wasmJahiziya =
    jahiziya === undefined || jahiziya === null ? undefined : WASM_JAHIZIYA[jahiziya];
  if (wasmJahiziya !== undefined) {
    halat.push(t(wasmJahiziya.wasf, lugha));
  }
  const wasf = halat.length > 0 ? `${ism} — ${halat.join('، ')}` : ism;

  // Only when the plate had to cut the title. A card that carried a tooltip
  // repeating a name already in front of the user would put one under the
  // pointer on every hover in a grid of ten thousand.
  const maqsus = lawha !== null && lawha.sutur.some((satr) => satr.maqsus);

  // Both tabs at once, which is the only state where the pair has to share the
  // band. The stacking is the stylesheet's; what it needs from here is the one
  // fact it cannot read off a single tab.
  const muzdawaj = yarsim(nass) && yarsim(sawt);

  // Published as a custom property rather than applied to a rule, because the
  // element that consumes it is the seat line — a pseudo-element — and CSS is
  // the only thing that can reach one. Empty when no colour arrived, so the
  // stylesheet's own fallback answers instead of a null being stringified. It
  // is set on the well rather than on the card because the well is a plain
  // element: Motion's `style` is a different type that models every property
  // as animatable, and a custom property cast through it would be a cast
  // fighting a type rather than describing a value.
  const tabaqa =
    lawn_ghilaf === undefined || lawn_ghilaf === null
      ? BILA_LAWN
      : ({ '--lawn-ghilaf': sittasi(lawn_ghilaf) } as CSSProperties);

  return (
    <motion.article
      className={`bitaqa bitaqa--${kathafa}${mukhtara ? ' bitaqa--mukhtara' : ''}`}
      // The raise is translate-only. A scale would resample the artwork every
      // frame of the transition and a cover two hundred pixels wide would
      // visibly soften. It is also the only part of the hover that is motion:
      // the surface and the hairline step by luminance and the seat line takes
      // the cover's own hue, and the lift is what makes those three reach the
      // eye as one change rather than as a flicker.
      whileHover={{ y: -3 }}
      // Pressing puts the card back down. With the shadows gone, a press that
      // did nothing left the pointer with no acknowledgement at all before the
      // game window appears, which on a slow launcher is a second of silence.
      whileTap={{ y: 0 }}
      transition={haraka(HARAKAT_HALA)}
      tabIndex={0}
      role="button"
      aria-label={wasf}
      aria-pressed={mukhtara}
      title={maqsus ? ism : undefined}
      data-muarrif={muarrif}
      onClick={alaNaqr}
      onKeyDown={alaMiftah}
      onContextMenu={alaQaima}
    >
      <div className="bitaqa__bir" style={tabaqa}>
        {/* The reserve. Always present, in every state, at every density. */}
        <span className={tabaqatHalqa('bitaqa__halqa-sawt', sawt)} aria-hidden="true" />
        <span className={tabaqatHalqa('bitaqa__halqa-nass', nass)} aria-hidden="true" />

        <div className="bitaqa__sura">
          {/*
            The plate is drawn whenever one exists, and stays underneath the
            artwork rather than being swapped out for it. That is what makes the
            cross-fade possible without a layout step: the image fades in over a
            plate that was already occupying the same rectangle, so there is no
            frame in which the well is empty and nothing moves when the image
            arrives.
          */}
          {lawha === null ? null : <LawhatBadila lawha={lawha} />}
          {ghilaf === null || ghilaf === ghilafFashil ? null : (
            <img
              className={
                ghilaf === ghilafZahir
                  ? 'bitaqa__ghilaf bitaqa__ghilaf--zahira'
                  : 'bitaqa__ghilaf'
              }
              src={ghilaf}
              alt=""
              loading="lazy"
              // Decoded off the main thread. A grid scrolling past forty covers
              // that each decode synchronously drops frames on the thread that
              // is also handling the scroll.
              decoding="async"
              draggable={false}
              // No `width`/`height`: the element is absolute at `inset: 0` in a
              // well whose size the card's tokens already fix, so an intrinsic
              // hint would change nothing and would state the standard
              // density's numbers on a card drawn at one of the other two.
              onLoad={alaHiml}
              // A cover the engine refused is taken out of the tree rather than
              // left in it at zero opacity. That is what makes a failed cover
              // indistinguishable from a game that never had one: the plate
              // below is the fallback in both cases, and it only reads as the
              // fallback while nothing at all is drawn over it — a rejected
              // `img` still paints its own broken-source chrome in some
              // engines. Removed by source, so a later cover from the cascade
              // is still tried.
              onError={alaKhataGhilaf}
            />
          )}
        </div>

        {/*
          The two marks that are facts about the game rather than patch states.
          The strip is absent, not empty, when neither mark is drawn: it is one
          element per card and most of a large library carries neither, which in
          a ten-thousand-card grid is ten thousand nodes that exist to hold
          nothing. Its two children answer the same conditions the accessible
          name was already built from, so the strip and the announcement cannot
          disagree about what this card is showing.

          Over the artwork rather than in the caption, and after it in the tree
          so they stack above without a z-index. The strip is absolutely
          positioned inside the reserve and holds them apart — readiness at the
          leading edge, the publisher's Arabic at the trailing edge — so a card
          carrying one, the other, both or neither is the same box in all four
          cases; see the governing rule at the top of this file. A strip rather
          than two independently anchored marks because two of them on a compact
          card were free to overlap, and the one thing worse than a mark nobody
          reads is two marks printed on top of each other.
        */}
        {miftahLugha === undefined && wasmJahiziya === undefined ? null : (
          <div className="bitaqa__wusum" aria-hidden="true">
            {jahiziya === undefined || jahiziya === null ? null : (
              <WasmJahiziya hala={jahiziya} lugha={lugha} />
            )}
            {lugha_rasmiya === undefined || lugha_rasmiya === null ? null : (
              <WasmLugha hala={lugha_rasmiya} lugha={lugha} />
            )}
          </div>
        )}
      </div>

      {/*
        The caption band. Fixed height whether it holds two tabs, one, or
        none — and never empty, because the name holds it open.

        `dir="auto"` on the inner span and not on the outer one: the outer box
        keeps the card's direction so every name in the grid starts at the same
        edge, and the inner one takes the title's own, which is what puts the
        ellipsis on a cut title at the end the reader finishes at. Two elements
        because one cannot do both; `bitaqa.css` carries the full argument.

        Hidden from assistive technology, like the plate and the tabs: every
        word of it is already in the card's accessible name, and announced here
        as well it would be the game read twice.
      */}
      <div className="bitaqa__shareet">
        <span className="bitaqa__unwan" aria-hidden="true">
          <span className="bitaqa__unwan-nass" dir="auto">
            {ism}
          </span>
        </span>
        <span className={muzdawaj ? 'bitaqa__alsina bitaqa__alsina--muzdawaj' : 'bitaqa__alsina'}>
          <Lisan naw="sawt" hala={sawt} unwan={t('bitaqa.lisan.sawt', lugha)} />
          <Lisan naw="nass" hala={nass} unwan={t('bitaqa.lisan.nass', lugha)} />
        </span>
      </div>
    </motion.article>
  );
}

/**
 * Memoized on every prop, because the grid re-renders its whole window on
 * scroll and a ten-thousand-card library cannot afford to rebuild a card whose
 * data has not changed.
 */
export const BitaqaLuba = memo(BitaqaLubaBila);
