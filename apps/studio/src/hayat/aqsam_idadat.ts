import type { QismIdadat } from '@/mustalahat/awamir';

/**
 * أقسام الإعدادات — the settings screen's eleven sections, named once.
 *
 * The screen folds each section independently and the backend can name one in
 * a failure's next step, so the key has to exist somewhere both the screen and
 * the failure block can read. It lives here rather than in either of them
 * because the screen imports the failure block and the failure block would
 * then have to import the screen back.
 *
 * Two sections have no counterpart in the backend's own list and are reachable
 * only by scrolling — the overlay's appearance and the keyboard shortcuts —
 * because no failure anywhere names them.
 */
export type MiftahQism =
  | 'ard'
  | 'manassat'
  | 'taareeb'
  | 'takhzin'
  | 'khutut'
  | 'muzawwidun'
  | 'masadir'
  | 'tahdith'
  | 'tashkhis'
  | 'tabaqa'
  | 'ikhtisarat';

/** Every section key, in the order the document draws them. */
export const AQSAM_IDADAT: readonly MiftahQism[] = [
  'ard',
  'manassat',
  'taareeb',
  'takhzin',
  'khutut',
  'muzawwidun',
  'masadir',
  'tahdith',
  'tashkhis',
  'tabaqa',
  'ikhtisarat',
];

/**
 * Whether a value carried in the address names a section this build draws.
 *
 * The address survives a reload and an older build's link, so it is validated
 * rather than trusted: an unknown section opens the screen at the top, which
 * is what it did before it could be deep-linked at all.
 */
export function huwaQism(khaam: unknown): khaam is MiftahQism {
  return typeof khaam === 'string' && (AQSAM_IDADAT as readonly string[]).includes(khaam);
}

/**
 * Where each section the backend can name is drawn.
 *
 * Total over `QismIdadat`, so a section added to the Rust vocabulary fails to
 * compile here until the screen that draws it is named — which is the whole
 * reason a failure may carry a section at all.
 */
export const QISM_SHASHA: Readonly<Record<QismIdadat, MiftahQism>> = {
  manassat: 'manassat',
  khutut: 'khutut',
  muzawwidun: 'muzawwidun',
  takhzin: 'takhzin',
  masadir: 'masadir',
  tahdith: 'tahdith',
  // The interface's own language and digits are the first rows of the display
  // section; the backend names the subject, not the heading it sits under.
  lugha: 'ard',
  tashkhis: 'tashkhis',
  taareeb: 'taareeb',
};
