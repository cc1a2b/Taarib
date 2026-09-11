// حالة فارغة — nothing here yet: a heading, one sentence, and the one thing to do next.

import type { JSX, ReactNode } from 'react';

interface Khasais {
  readonly unwan: string;
  readonly nass?: string | null;
  /** Whether this state stands for a whole screen rather than one panel of it. */
  readonly shasha?: boolean;
  readonly children?: ReactNode;
}

/**
 * An empty state is not a failure and must not be dressed as one: no box, no
 * glyph, no code. It is set on the ground in the library's own vocabulary —
 * the treatment `shabakat_maktaba.css` gives an empty library — so the same
 * sentence reads the same on every screen that has nothing to show.
 */
export function HalatFarigha({ unwan, nass, shasha = false, children }: Khasais): JSX.Element {
  return (
    <div className={shasha ? 'halat halat--farigh halat--shasha' : 'halat halat--farigh'} role="status">
      <p className="halat__unwan">{unwan}</p>
      {nass === undefined || nass === null ? null : <p className="halat__nass">{nass}</p>}
      {children === undefined || children === null ? null : (
        <div className="halat__afal">{children}</div>
      )}
    </div>
  );
}
