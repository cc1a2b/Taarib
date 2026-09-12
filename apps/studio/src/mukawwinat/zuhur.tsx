// الظهور — a thing that is sometimes there arrives and leaves on the panel tier instead of popping.

import type { HTMLMotionProps } from 'motion/react';
import { AnimatePresence, motion } from 'motion/react';
import type { JSX } from 'react';

import { HARAKAT_HALA, HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

/**
 * Where the thing comes from. A menu or a panel opens downward from the
 * control that owns it; a bar rises from the edge it is anchored to; a dialog
 * in the middle of nowhere simply resolves in place.
 */
export type AslZuhur = 'fawq' | 'taht' | 'mahall';

/** How far a thing travels on the way in: felt, not seen. */
const MASAFA = 6;

const MASAFAT: Readonly<Record<AslZuhur, number>> = {
  fawq: -MASAFA,
  taht: MASAFA,
  mahall: 0,
};

type KhasaisZuhur = Omit<HTMLMotionProps<'div'>, 'initial' | 'animate' | 'exit' | 'transition'> & {
  /** Whether the thing is there. The wrapper stays mounted; the thing does not. */
  readonly maftuh: boolean;
  readonly asl?: AslZuhur;
};

/**
 * A context menu, a filter panel, a confirmation, a selection bar: each used to
 * appear between two frames and vanish the same way, which reads as the
 * interface flickering rather than answering. One wrapper gives all of them
 * the same short arrival and the shorter leave, and reduced motion makes both
 * instant through the same tier every other transition reads.
 *
 * Everything else — the role, the label, the position, the handlers — passes
 * straight through to the element, so the thing keeps its own semantics.
 */
export function Zuhur({ maftuh, asl = 'fawq', children, ...baqi }: KhasaisZuhur): JSX.Element {
  return (
    <AnimatePresence>
      {maftuh ? (
        <motion.div
          {...baqi}
          initial={{ opacity: 0, y: MASAFAT[asl] }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: MASAFAT[asl] / 2, transition: haraka(HARAKAT_HALA) }}
          transition={haraka(HARAKAT_LAWHA)}
        >
          {children}
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
