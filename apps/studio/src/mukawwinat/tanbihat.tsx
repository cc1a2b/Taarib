// التنبيهات — the notice stack at the foot of the window: what just happened, and the one thing to do about it.

import { AnimatePresence, motion } from 'motion/react';
import type { JSX } from 'react';
import { useCallback, useEffect, useRef } from 'react';

import type { NawTanbih, Tanbih } from '@/hayat/tanbihat';
import { useTanbihat } from '@/hayat/tanbihat';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';
import { RamzKhata, RamzMaluma, RamzTahaqquq, RamzTanbeeh } from '@/mukawwinat/rumuz';
import { HARAKAT_HALA, HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './tanbihat.css';

/** How far a notice rises on the way in, and how far it sinks on the way out. */
const MASAFA = 12;

function RamzNaw({ naw }: { readonly naw: NawTanbih }): JSX.Element {
  switch (naw) {
    case 'najah':
      return <RamzTahaqquq className="tanbih__ramz" />;
    case 'maluma':
      return <RamzMaluma className="tanbih__ramz" />;
    case 'tanbeeh':
    case 'khatar':
      return <RamzTanbeeh className="tanbih__ramz" />;
  }
}

interface KhasaisBand {
  readonly tanbih: Tanbih;
  readonly lugha: Lugha;
  readonly ala_ighlaq: () => void;
}

/**
 * One notice.
 *
 * The clock stops while the pointer or the keyboard is on it: a person who
 * has moved to read the second line, or to reach the undo, must not lose it
 * under their hand. It restarts from the full duration when they leave, which
 * is simpler than a resumed remainder and never shorter than they expect.
 */
function BandTanbih({ tanbih, lugha, ala_ighlaq }: KhasaisBand): JSX.Element {
  const muaqqit = useRef<number | null>(null);
  const { mudda } = tanbih;

  const awqif = useCallback((): void => {
    if (muaqqit.current !== null) {
      window.clearTimeout(muaqqit.current);
      muaqqit.current = null;
    }
  }, []);

  const ibda = useCallback((): void => {
    awqif();
    if (mudda === null) {
      return;
    }
    muaqqit.current = window.setTimeout(ala_ighlaq, mudda);
  }, [awqif, mudda, ala_ighlaq]);

  useEffect(() => {
    ibda();
    return awqif;
  }, [ibda, awqif]);

  const amal = tanbih.amal;

  return (
    <motion.div
      layout
      className={`tanbih tanbih--${tanbih.naw}`}
      role={tanbih.naw === 'khatar' ? 'alert' : 'status'}
      initial={{ opacity: 0, y: MASAFA }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: MASAFA / 2, transition: haraka(HARAKAT_HALA) }}
      transition={haraka(HARAKAT_LAWHA)}
      onPointerEnter={awqif}
      onPointerLeave={ibda}
      onFocusCapture={awqif}
      onBlurCapture={ibda}
    >
      <RamzNaw naw={tanbih.naw} />
      <div className="tanbih__matn">
        <p className="tanbih__jumla">{tanbih.nass}</p>
        {tanbih.tafsil === null ? null : (
          <p className="tanbih__tafsil" dir="auto">
            {tanbih.tafsil}
          </p>
        )}
        {tanbih.ramz === null ? null : (
          <span className="tanbih__ramz-khata mono-ltr">{tanbih.ramz}</span>
        )}
      </div>
      {amal === null ? null : (
        <button
          type="button"
          className="zir tanbih__amal"
          onClick={() => {
            amal.nafidh();
            ala_ighlaq();
          }}
        >
          {amal.unwan}
        </button>
      )}
      <button
        type="button"
        className="tanbih__ighlaq"
        aria-label={t('tanbih.ighlaq', lugha)}
        title={t('tanbih.ighlaq', lugha)}
        onClick={ala_ighlaq}
      >
        <RamzKhata />
      </button>
    </motion.div>
  );
}

/**
 * The stack. Mounted once, in the shell, over every screen.
 *
 * Each notice is its own live region rather than the stack being one, so a
 * failure interrupts and a confirmation waits its turn, and so a notice that
 * leaves does not re-announce the ones still standing.
 */
export function Tanbihat({ lugha }: { readonly lugha: Lugha }): JSX.Element {
  const qaima = useTanbihat((halat) => halat.qaima);
  const aghliq = useTanbihat((halat) => halat.aghliq);

  return (
    <div className="tanbihat">
      <AnimatePresence initial={false}>
        {qaima.map((tanbih) => (
          <BandTanbih
            key={tanbih.muarrif}
            tanbih={tanbih}
            lugha={lugha}
            ala_ighlaq={() => {
              aghliq(tanbih.muarrif);
            }}
          />
        ))}
      </AnimatePresence>
    </div>
  );
}
