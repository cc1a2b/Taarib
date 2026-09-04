// الاختصارات — parsing stored `ctrl+k` chords and matching them against keyboard events.

/** One parsed chord. Ctrl and the platform command key count as one modifier. */
export interface Ikhtisar {
  readonly ctrl: boolean;
  readonly shift: boolean;
  readonly alt: boolean;
  /** The final key, lowercased, as `KeyboardEvent.key` reports it. */
  readonly miftah: string;
}

/** Spelled-out tokens for keys whose `KeyboardEvent.key` is not their name. */
const ASMA_MAFATIH: Readonly<Record<string, string>> = {
  space: ' ',
  esc: 'escape',
};

/**
 * Parses a stored chord.
 *
 * Null when the text names no final key, names two, or carries no ctrl or alt
 * modifier — a bare letter as a global shortcut would swallow typing.
 */
export function hallil(nass: string): Ikhtisar | null {
  const ajza = nass
    .toLowerCase()
    .split('+')
    .map((juz) => juz.trim())
    .filter((juz) => juz !== '');
  if (ajza.length === 0) {
    return null;
  }
  let ctrl = false;
  let shift = false;
  let alt = false;
  let miftah: string | null = null;
  for (const juz of ajza) {
    if (juz === 'ctrl' || juz === 'meta' || juz === 'cmd') {
      ctrl = true;
    } else if (juz === 'shift') {
      shift = true;
    } else if (juz === 'alt') {
      alt = true;
    } else if (miftah === null) {
      miftah = ASMA_MAFATIH[juz] ?? juz;
    } else {
      return null;
    }
  }
  if (miftah === null || (!ctrl && !alt)) {
    return null;
  }
  return { ctrl, shift, alt, miftah };
}

/** Whether the event is exactly this chord. */
export function yutabiq(hadath: KeyboardEvent, ikhtisar: Ikhtisar): boolean {
  return (
    (hadath.ctrlKey || hadath.metaKey) === ikhtisar.ctrl &&
    hadath.shiftKey === ikhtisar.shift &&
    hadath.altKey === ikhtisar.alt &&
    hadath.key.toLowerCase() === ikhtisar.miftah
  );
}

/** The chord a stored text resolves to, falling back when it does not parse. */
export function hallilAw(nass: string, badeel: Ikhtisar): Ikhtisar {
  return hallil(nass) ?? badeel;
}

/** The parsed defaults, for fallback when a stored chord is unreadable. */
export const IKHTISAR_LAWHA: Ikhtisar = { ctrl: true, shift: false, alt: false, miftah: 'k' };
export const IKHTISAR_TARAJU: Ikhtisar = { ctrl: true, shift: false, alt: false, miftah: 'z' };
