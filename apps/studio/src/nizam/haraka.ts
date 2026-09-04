import type { Transition } from 'motion/react';

/**
 * الحركة — the motion constants.
 *
 * Three tiers, one spring family, no exceptions. Motion in Taarib exists to
 * show that a thing moved from one state to another; it never announces
 * itself, never bounces, and never plays on a data update. A table that
 * re-sorts itself with a flourish is a table nobody can read.
 *
 * The spring is what actually moves position, size and transform. The
 * duration on each tier drives the tween used for the values a spring reads
 * badly — opacity and colour — so a single tier is one coherent movement
 * rather than a spring racing a fade.
 */

/**
 * The one spring in the product. Critically damped enough that nothing
 * overshoots: `damping` of 38 against `stiffness` of 420 at `mass` 0.9 settles
 * without a visible return swing.
 */
export const NABD = {
  type: 'spring',
  stiffness: 420,
  damping: 38,
  mass: 0.9,
  restDelta: 0.001,
  restSpeed: 0.01,
} as const satisfies Transition;

/** The three tier durations, in seconds, as Motion expects them. */
export const MUDDA = {
  /** Hover, press, and any single-element state change. */
  hala: 0.12,
  /** A panel, a pane, or a disclosure opening or closing. */
  lawha: 0.18,
  /** A route change or a shared element moving between two screens. */
  masar: 0.24,
} as const;

/**
 * The easing for the tween half of a tier. It leaves fast and arrives slowly,
 * matching the spring's own profile, and never returns a value above 1 — an
 * overshooting curve on opacity reads as a flicker.
 *
 * Kept identical to `--munhana` in the token layer so a CSS transition and a
 * Motion transition of the same tier are indistinguishable.
 */
const MUNHANA: [number, number, number, number] = [0.22, 0.61, 0.36, 1];

/**
 * Hover, press, and state. Must land within a frame or two of the pointer
 * arriving: a hover state with a delay reads as an unresponsive application.
 */
export const HARAKAT_HALA = {
  ...NABD,
  opacity: { type: 'tween', duration: MUDDA.hala, ease: MUNHANA },
} as const satisfies Transition;

/** A panel, a pane, or a disclosure. The same spring over a longer distance. */
export const HARAKAT_LAWHA = {
  ...NABD,
  opacity: { type: 'tween', duration: MUDDA.lawha, ease: MUNHANA },
} as const satisfies Transition;

/**
 * A route change or a shared element. Position and size only — the content
 * area never fades, because fading it would mean re-rendering it, and the
 * content area is exactly the thing that must survive navigation intact.
 */
export const HARAKAT_MASAR = {
  ...NABD,
  layout: { ...NABD },
  opacity: { type: 'tween', duration: MUDDA.masar, ease: MUNHANA },
} as const satisfies Transition;

/** A transition that has already finished. Not a fast one — a finished one. */
export const FAWRI = { duration: 0 } as const satisfies Transition;

/**
 * Whether this session has asked for less motion.
 *
 * Read at call time rather than cached, because the preference can change
 * while the application is open and the next transition has to honour it.
 * `data-haraka='hurr'` on the root is the settings override for a user who
 * turned the honouring off, matching the token layer's guard. Guarded for the
 * case where `matchMedia` is unavailable, which is what a stripped webview or
 * a server-side render looks like.
 */
export function yufaddilTaqleelHaraka(): boolean {
  if (typeof globalThis.matchMedia !== 'function') {
    return false;
  }
  if (globalThis.document.documentElement.dataset['haraka'] === 'hurr') {
    return false;
  }
  return globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * Wraps a tier so it is honoured, or made instant, according to the session's
 * reduced-motion preference.
 *
 * Instant, never slower: a user who asks for less motion is not asking for
 * lazier motion, and stretching a transition out is the most common way that
 * request gets misread.
 *
 * @param intiqal the tier to use when motion is welcome
 */
export function haraka(intiqal: Transition): Transition {
  return yufaddilTaqleelHaraka() ? FAWRI : intiqal;
}
