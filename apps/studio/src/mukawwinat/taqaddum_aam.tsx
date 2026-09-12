// التقدّم العام — one rule along the top of the content that runs while anything is being fetched or written.

import { useIsFetching, useIsMutating } from '@tanstack/react-query';
import type { JSX } from 'react';
import { useEffect, useState } from 'react';

import './taqaddum_aam.css';

/** Nothing, a wait in progress, or a wait that has just ended. */
type Hala = 'sakin' | 'jari' | 'tamm';

/**
 * A wait shorter than this is not shown at all. Most of this product's answers
 * come from a process on the same machine and land before the eye would find
 * the line; drawing it for those would make every click flicker.
 */
const MUHLAT_ZUHUR = 200;

/** How long the completed line stands before it goes. */
const MUDDAT_TAMAM = 360;

/**
 * The one place a person can see that the product is working when the screen
 * they are on has no skeleton of its own for it: a refetch after a return, a
 * write that has no bar, a second screen's data arriving. It reads the query
 * cache's own counters, so nothing has to announce itself to it.
 *
 * It never flashes. It appears only once a wait has outlived the threshold,
 * and it always finishes: the line fills to the end and then fades, so a wait
 * that ends is seen to end rather than the line simply vanishing.
 */
export function TaqaddumAam(): JSX.Element {
  const jari = useIsFetching() + useIsMutating() > 0;
  const [hala, setHala] = useState<Hala>('sakin');

  useEffect(() => {
    if (jari) {
      const muaqqit = window.setTimeout(() => {
        setHala('jari');
      }, MUHLAT_ZUHUR);
      return () => {
        window.clearTimeout(muaqqit);
      };
    }
    setHala((sabiq) => (sabiq === 'jari' ? 'tamm' : sabiq));
    return undefined;
  }, [jari]);

  useEffect(() => {
    if (hala !== 'tamm') {
      return undefined;
    }
    const muaqqit = window.setTimeout(() => {
      setHala('sakin');
    }, MUDDAT_TAMAM);
    return () => {
      window.clearTimeout(muaqqit);
    };
  }, [hala]);

  return (
    <div className="taqaddum-aam" data-hala={hala} aria-hidden="true">
      <span className="taqaddum-aam__khatt" />
    </div>
  );
}
