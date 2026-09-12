/**
 * رموز — the product's icon family: one grid, one stroke, one rule about direction.
 *
 * THE GRID. Every glyph is a 24×24 viewBox with a 20×20 live area inside a 2-unit margin,
 * and every point sits on a whole or half unit. The family is drawn for 16px, where one
 * grid unit renders as 0.667px: a counter narrower than 3 units closes into a blob and a
 * detail thinner than 2 units disappears, so nothing here stacks two parallel strokes
 * closer than 3 units and nothing carries interior ornament. Rectangles take rx="1"; free
 * paths get their corners from the round join rather than from arcs, because arc tangents
 * on a triangle or a chevron land on irrational coordinates and break the grid.
 *
 * THE STROKE. fill="none", stroke="currentColor", round caps, round joins — and no
 * stroke-width anywhere. qaida.css declares `svg { stroke-width: var(--samk-ramz) }` once
 * for the document, so a component physically cannot introduce a second weight, which is
 * the fastest way an icon set stops reading as a set. Colour is currentColor only: an icon
 * is the colour of the text beside it, never a colour of its own.
 *
 * THE MIRROR. The interface is right-to-left by default. A glyph that means "back" has to
 * point the other way in Arabic; a glyph that means "done" must not. The directional ones
 * — the four arrows, and the request list whose markers lead a column — draw inside a <g>
 * carrying translate(24 0) scale(-1 1) whenever document.documentElement.dir is "rtl".
 * They read that attribute through one shared MutationObserver, so the flip is live if the
 * language changes and no caller can forget to ask for it. Nothing else ever flips: a
 * check, a magnifier, a clock, a warning triangle, a play head, a lock, a spinner arc and
 * a folded page corner all carry handedness that is not direction, and mirroring them
 * yields a wrong object rather than a right-to-left one.
 *
 * WHAT IS DELIBERATELY ABSENT. No gear — a cogwheel is a dozen teeth of sub-2-unit detail
 * that fuse into a grey disc at 16px, and it promises machinery rather than the handful of
 * things this product actually lets you set; a slider pair says that and survives the
 * render. No globe — the language control switches a language, and a globe says "the
 * world", which is the wrong claim from an Arabic-first tool. No speech bubble — nothing
 * in this product is a conversation.
 */

import type { JSX, ReactNode } from 'react';
import { useSyncExternalStore } from 'react';

/** What every glyph in the family accepts, and the only thing a caller may pass. */
export type KhasaisRamz = {
  readonly hajm?: number;
  readonly className?: string;
  readonly 'aria-hidden'?: boolean;
};

/**
 * The inline size, restated here because an SVG needs a number before CSS resolves.
 * It matches --hajm-ramz-satri; a navigation-sized icon passes hajm, and a caller that
 * wants the token itself sizes the svg through className, which overrides the attribute.
 */
const HAJM_SATRI = 16;

/** Flip about the grid's own centre line: scale alone would push the glyph off canvas. */
const MIRAT = 'translate(24 0) scale(-1 1)';

const mustamioon = new Set<() => void>();
let muraqib: MutationObserver | null = null;

/**
 * One observer for the whole family, not one per icon.
 *
 * A virtualised library grid mounts thousands of glyphs at once, and a MutationObserver
 * per instance would put thousands of observers on the same single attribute.
 */
function ishtirakIttijah(tanbih: () => void): () => void {
  mustamioon.add(tanbih);
  if (muraqib === null) {
    muraqib = new MutationObserver(() => {
      for (const mustami of mustamioon) {
        mustami();
      }
    });
    muraqib.observe(document.documentElement, { attributes: true, attributeFilter: ['dir'] });
  }
  return () => {
    mustamioon.delete(tanbih);
    if (mustamioon.size === 0 && muraqib !== null) {
      muraqib.disconnect();
      muraqib = null;
    }
  };
}

function laqtatIttijah(): boolean {
  return document.documentElement.dir === 'rtl';
}

/** The group the directional glyphs draw inside; it is a no-op group in a left-to-right
 *  document so the element tree does not change shape when the language does. */
function Maqloob({ children }: { readonly children: ReactNode }): JSX.Element {
  const maqloob = useSyncExternalStore(ishtirakIttijah, laqtatIttijah);
  return <g transform={maqloob ? MIRAT : undefined}>{children}</g>;
}

/**
 * The family's contract in one place: every glyph, here and in any file that extends this
 * set, is built through this so the viewBox, the fills, the terminals and the default
 * decorative role cannot drift apart from one icon to the next.
 */
export function ramz(khasais: KhasaisRamz, rasm: ReactNode): JSX.Element {
  const hajm = khasais.hajm ?? HAJM_SATRI;
  return (
    <svg
      width={hajm}
      height={hajm}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={khasais.className}
      aria-hidden={khasais['aria-hidden'] ?? true}
    >
      {rasm}
    </svg>
  );
}

/** The same contract for a glyph that carries direction: the flip is built in, not asked
 *  for, because a mirroring decision left to the call site is a mirroring decision that
 *  will be missed at some of the call sites. */
export function ramzIttijahi(khasais: KhasaisRamz, rasm: ReactNode): JSX.Element {
  return ramz(khasais, <Maqloob>{rasm}</Maqloob>);
}

/* ==========================================================================
   التنقل والبنية — navigation and structure.
   ========================================================================== */

/** A shelf of covers rather than a book: the library holds games, not volumes, and the
 *  cells are square because a portrait cell at this size leaves a 2-unit gutter. */
export function RamzMaktaba(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="2.5" y="2.5" width="7.5" height="7.5" rx="1" />
      <rect x="14" y="2.5" width="7.5" height="7.5" rx="1" />
      <rect x="2.5" y="14" width="7.5" height="7.5" rx="1" />
      <rect x="14" y="14" width="7.5" height="7.5" rx="1" />
    </>,
  );
}

/** Each track breaks either side of its handle instead of running behind it: a line
 *  crossing a 5-unit circle closes the counter at 16px and the handle stops reading. */
export function RamzIdadat(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M3 7H6.5M11.5 7H21" />
      <circle cx="9" cy="7" r="2.5" />
      <path d="M3 17H12.5M17.5 17H21" />
      <circle cx="15" cy="17" r="2.5" />
    </>,
  );
}

export function RamzTashkhis(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M2.5 12H6.5L9 6L12.5 18L15.5 12H21.5" />);
}

/** Mirrored, unlike the rest of this group: the markers are a leading column, and a column
 *  flushed to the left edge of an Arabic list is the same mistake as a left-pointing back
 *  arrow. The dots are filled because a 3-unit ring is a smudge at 16px. */
export function RamzTalabat(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(
    khasais,
    <>
      <circle cx="4" cy="5.5" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="4" cy="12" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="4" cy="18.5" r="1.5" fill="currentColor" stroke="none" />
      <path d="M8.5 5.5H21.5M8.5 12H21.5M8.5 18.5H21.5" />
    </>,
  );
}

/** The cut corner and its fold are what separate a document from a checkbox once the check
 *  is inside it. The fold is handedness, not direction, so it never mirrors. */
export function RamzMuraja(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M5 2.5H14L19 7.5V21.5H5Z" />
      <path d="M14 2.5V7.5H19" />
      <path d="M8 14.5L10.5 17L15.5 12" />
    </>,
  );
}

/** The divider sits on the centre line rather than off to one side: an off-centre split
 *  would assert which pane is the sidebar, and that answer changes with the direction. */
export function RamzWarsha(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="2.5" y="5" width="19" height="14" rx="1" />
      <path d="M12 5V19" />
    </>,
  );
}

export function RamzTaqdeem(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M12 3V14M8 7L12 3L16 7" />
      <path d="M3 14.5v5a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1v-5" />
    </>,
  );
}

/** The rear sheet is an open path, not a second rectangle: two full rectangles cross
 *  inside the overlap and the crossing is four extra strokes the 16px render cannot hold. */
export function RamzTabaqa(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M6 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v2" />
      <rect x="9" y="9" width="12" height="12" rx="1" />
    </>,
  );
}

/** A game is a cover with a strip under it — the product's own card, at 24 units. */
export function RamzLuba(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="5.5" y="2.5" width="13" height="19" rx="1" />
      <path d="M5.5 16.5H18.5" />
    </>,
  );
}

/** The play head: the automatic run starts here and nowhere else in the rail. */
export function RamzTilqai(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M7 4.5L18.5 12L7 19.5Z" />);
}

/** The rail itself: a frame with its divider on the leading side, so it mirrors. */
export function RamzJanib(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(
    khasais,
    <>
      <rect x="2.5" y="4" width="19" height="16" rx="1" />
      <path d="M9 4V20" />
    </>,
  );
}

/* ==========================================================================
   النافذة — the three window controls, drawn as the platform draws its own:
   a line, a frame, and two frames one behind the other. The restore glyph's
   rear frame is an open path so its corner does not cross the front one.
   ========================================================================== */

export function RamzTasgheer(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M5 12.5H19" />);
}

export function RamzTakbeer(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <rect x="5" y="5" width="14" height="14" rx="1" />);
}

export function RamzIstiada(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="4.5" y="8.5" width="11" height="11" rx="1" />
      <path d="M8.5 8.5V5.5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-3" />
    </>,
  );
}

/* ==========================================================================
   الاتجاه — the directional set. Every glyph below mirrors itself.
   ========================================================================== */

export function RamzRujoo(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(khasais, <path d="M20.5 12H3.5M10 5.5L3.5 12L10 18.5" />);
}

export function RamzTaqaddum(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(khasais, <path d="M3.5 12H20.5M14 5.5L20.5 12L14 18.5" />);
}

/** The vertical pair is symmetric about x = 12, so today the flip changes nothing. They
 *  still go through the directional builder so the group has one behaviour, and so a later
 *  edit that gives either one an asymmetric tail cannot silently lose the mirror. */
export function RamzSahmAsfal(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(khasais, <path d="M12 3.5V20.5M5.5 14L12 20.5L18.5 14" />);
}

export function RamzSahmAala(khasais: KhasaisRamz): JSX.Element {
  return ramzIttijahi(khasais, <path d="M12 20.5V3.5M5.5 10L12 3.5L18.5 10" />);
}

/* ==========================================================================
   الحالة — state. Nothing here mirrors.
   ========================================================================== */

export function RamzTahaqquq(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M4 12L9.5 17.5L20 7" />);
}

export function RamzKhata(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M5 5L19 19M19 5L5 19" />);
}

/** The triangle keeps sharp vertices on half units and takes its softening from the round
 *  join: rounding the corners with arcs would move all six tangent points off the grid. */
export function RamzTanbeeh(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <path d="M12 3.5L21.5 20.5H2.5Z" />
      <path d="M12 9.5V14" />
      <circle cx="12" cy="17" r="1" fill="currentColor" stroke="none" />
    </>,
  );
}

export function RamzMaluma(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <circle cx="12" cy="12" r="9.5" />
      <circle cx="12" cy="8" r="1" fill="currentColor" stroke="none" />
      <path d="M12 11.5V16.5" />
    </>,
  );
}

/** Three quarters of a ring, ending on the cardinal points so the gap is unmistakable at
 *  16px. It is static: the rotation belongs to the caller's transition, and the arc's
 *  handedness is rotation rather than reading order, so it never mirrors. */
export function RamzTahmil(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M12 2.5A9.5 9.5 0 1 1 2.5 12" />);
}

/** No keyhole: a 2-unit dot inside a 10-unit body leaves under 3 units of clear counter on
 *  either side at 16px, and the shackle already says lock without it. */
export function RamzQufl(khasais: KhasaisRamz): JSX.Element {
  return ramz(
    khasais,
    <>
      <rect x="4.5" y="10.5" width="15" height="10" rx="1" />
      <path d="M8 10.5V7.5a4 4 0 0 1 8 0v3" />
    </>,
  );
}
