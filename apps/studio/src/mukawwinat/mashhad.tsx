// المشهد — one state of a screen at a time, arriving on the panel spring instead of snapping.

import { AnimatePresence, motion } from 'motion/react';
import type { JSX, ReactNode } from 'react';

import { HARAKAT_HALA, HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

/** How far a state travels on the way in: felt, not seen. */
const MASAFA = 4;

interface Khasais {
  /** Which state is showing. A change of key is the only thing that moves. */
  readonly miftah: string;
  readonly className?: string;
  readonly children: ReactNode;
}

/**
 * A screen's body is a skeleton, then a result, an empty state or a failure,
 * and each used to replace the last between two frames. `mode="wait"` is what
 * keeps two of them from painting at once: the leaving state fades on the
 * state tier before the arriving one rises on the panel tier. Data updates
 * inside a state never pass through here, so nothing re-animates on a refetch.
 */
export function Mashhad({ miftah, className, children }: Khasais): JSX.Element {
  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={miftah}
        className={className}
        initial={{ opacity: 0, y: MASAFA }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, transition: haraka(HARAKAT_HALA) }}
        transition={haraka(HARAKAT_LAWHA)}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}
