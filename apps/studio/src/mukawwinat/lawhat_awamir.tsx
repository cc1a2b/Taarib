// لوحة الأوامر — the chord opens a filtered, grouped action list; an undo is reported through the notices.

import { AnimatePresence, motion } from 'motion/react';
import type { ChangeEvent, JSX, KeyboardEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { rashshih, useAwamirLawha, useSajjilAwamir } from '@/hayat/awamir_lawha';
import { IKHTISAR_LAWHA, hallilAw, yutabiq } from '@/hayat/ikhtisarat';
import { ansha } from '@/hayat/tanbihat';
import { useMiftahTaraju } from '@/hayat/taraju';
import type { MiftahLugha } from '@/lugha/lugha';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './lawhat_awamir.css';

/** One rendered group: a majal header, its actions, and their flat start index. */
interface Majmua {
  readonly majal: string;
  readonly awamir: readonly AmrLawha[];
  readonly bidaya: number;
}

/** One key the open list answers to, and what pressing it does. */
interface MiftahQaima {
  readonly ramz: string;
  readonly wasf: MiftahLugha;
}

/**
 * The three keys the list answers to, shown where the list is: a palette that
 * only rewards a user who already knows it is a palette most people click
 * through.
 */
const MAFATIH_QAIMA: readonly MiftahQaima[] = [
  { ramz: '↑↓', wasf: 'lawha.miftah.tanaqqul' },
  { ramz: '↵', wasf: 'lawha.miftah.nafidh' },
  { ramz: 'Esc', wasf: 'lawha.miftah.ighlaq' },
];

function jammi(nataij: readonly AmrLawha[]): Majmua[] {
  const majmuat: { majal: string; awamir: AmrLawha[]; bidaya: number }[] = [];
  for (const [fihris, amr] of nataij.entries()) {
    const akhira = majmuat.at(-1);
    if (akhira !== undefined && akhira.majal === amr.majal) {
      akhira.awamir.push(amr);
    } else {
      majmuat.push({ majal: amr.majal, awamir: [amr], bidaya: fihris });
    }
  }
  return majmuat;
}

/**
 * The title with the run that matched marked inside it.
 *
 * The filter is a case-insensitive substring over the title *or* the group
 * name, so a title with no run of its own is returned untouched rather than
 * being marked at a position it does not hold.
 */
function Muabbar({ nass, ibra }: { readonly nass: string; readonly ibra: string }): JSX.Element {
  const mawqi = ibra === '' ? -1 : nass.toLowerCase().indexOf(ibra);
  if (mawqi < 0) {
    return <>{nass}</>;
  }
  return (
    <>
      {nass.slice(0, mawqi)}
      <mark className="lawha-awamir__mutabaqa">{nass.slice(mawqi, mawqi + ibra.length)}</mark>
      {nass.slice(mawqi + ibra.length)}
    </>
  );
}

interface KhasaisLawha {
  readonly lugha: Lugha;
  /** The stored chord that opens the palette; unreadable text falls back to Ctrl+K. */
  readonly ikhtisarLawha: string;
  /** The stored chord bound to undo; unreadable text falls back to Ctrl+Z. */
  readonly ikhtisarTaraju: string;
}

/** Mounted once, at the router root. */
export function LawhatAwamir({ lugha, ikhtisarLawha, ikhtisarTaraju }: KhasaisLawha): JSX.Element {
  const maftuha = useAwamirLawha((halat) => halat.maftuha);
  const aghliq = useAwamirLawha((halat) => halat.aghliq);
  const baddil = useAwamirLawha((halat) => halat.baddil);
  const awamir = useAwamirLawha((halat) => halat.awamir);
  const kul = useAwamirLawha((halat) => halat.kul);

  const [istifsar, setIstifsar] = useState('');
  const [fihris, setFihris] = useState(0);
  const qaima = useRef<HTMLUListElement | null>(null);

  // What was undone is said where every other outcome is said, in the notice
  // stack, rather than on a line of this palette's own.
  const alaTamamTaraju = useCallback(
    (wasf: string): void => {
      ansha({ naw: 'maluma', nass: t('taraju.tamma', lugha, { wasf }) });
    },
    [lugha],
  );
  const { shaghghil } = useMiftahTaraju(ikhtisarTaraju, alaTamamTaraju);

  // Undo as a palette action, so the chord is discoverable where actions live.
  const awamirDhatiya = useMemo<readonly AmrLawha[]>(
    () => [
      {
        muarrif: 'taraju.akhir',
        unwan: t('taraju.amr', lugha),
        majal: t('tatbiq.ism', lugha),
        ikhtisar: ikhtisarTaraju,
        nafidh: shaghghil,
      },
    ],
    [lugha, ikhtisarTaraju, shaghghil],
  );
  useSajjilAwamir(awamirDhatiya);

  // The map is the reactive source `kul` reads, so it is the memo's real key.
  const nataij = useMemo(() => rashshih(kul(), istifsar), [kul, awamir, istifsar]);
  const majmuat = useMemo(() => jammi(nataij), [nataij]);

  // The exact needle the filter matched on, so the mark lands where it matched.
  const ibra = istifsar.trim().toLowerCase();

  // The active index, clamped so a shrinking result set cannot leave it dangling.
  const faal = nataij.length === 0 ? 0 : Math.min(fihris, nataij.length - 1);

  useEffect(() => {
    const maqrua = hallilAw(ikhtisarLawha, IKHTISAR_LAWHA);
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      if (yutabiq(hadath, maqrua)) {
        hadath.preventDefault();
        baddil();
        return;
      }
      if (hadath.key === 'Escape' && maftuha) {
        aghliq();
      }
    };
    window.addEventListener('keydown', alaMiftah);
    return () => {
      window.removeEventListener('keydown', alaMiftah);
    };
  }, [maftuha, baddil, aghliq, ikhtisarLawha]);

  // Clearing on close rather than on open, so reopening never flashes the old query.
  useEffect(() => {
    if (!maftuha) {
      return undefined;
    }
    const sabiq = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => {
      setIstifsar('');
      setFihris(0);
      sabiq?.focus();
    };
  }, [maftuha]);

  useEffect(() => {
    qaima.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }, [faal, nataij]);

  const naffidh = (amr: AmrLawha | undefined): void => {
    if (amr === undefined) {
      return;
    }
    amr.nafidh();
    aghliq();
  };

  const alaMiftahHaql = (hadath: KeyboardEvent<HTMLInputElement>): void => {
    if (hadath.key === 'ArrowDown') {
      hadath.preventDefault();
      setFihris(Math.min(faal + 1, Math.max(nataij.length - 1, 0)));
      return;
    }
    if (hadath.key === 'ArrowUp') {
      hadath.preventDefault();
      setFihris(Math.max(faal - 1, 0));
      return;
    }
    if (hadath.key === 'Enter') {
      hadath.preventDefault();
      naffidh(nataij.at(faal));
    }
  };

  return (
    <AnimatePresence>
      {maftuha ? (
          <div key="lawha-awamir" className="lawha-awamir" role="presentation">
            <motion.div
              className="lawha-awamir__sitar"
              aria-hidden="true"
              onClick={aghliq}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={haraka(HARAKAT_LAWHA)}
            />
            <motion.div
              className="lawha-awamir__lawha"
              role="dialog"
              aria-modal="true"
              aria-label={t('lawha.tasmiya', lugha)}
              initial={{ opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={haraka(HARAKAT_LAWHA)}
            >
              <input
                className="lawha-awamir__haql"
                type="text"
                dir="auto"
                autoFocus
                role="combobox"
                aria-expanded="true"
                aria-controls="lawha-awamir-qaima"
                aria-activedescendant={
                  nataij.length > 0 ? `lawha-awamir-khiyar-${String(faal)}` : undefined
                }
                aria-label={t('lawha.tasmiya', lugha)}
                placeholder={t('lawha.mawdi', lugha)}
                value={istifsar}
                spellCheck={false}
                autoComplete="off"
                onChange={(hadath: ChangeEvent<HTMLInputElement>) => {
                  setIstifsar(hadath.target.value);
                  setFihris(0);
                }}
                onKeyDown={alaMiftahHaql}
              />
              {nataij.length === 0 ? (
                <p className="lawha-awamir__farigh">
                  {t('lawha.farigh', lugha)}
                  {ibra === '' ? null : (
                    <span className="lawha-awamir__farigh-ibra mono-ltr">{istifsar.trim()}</span>
                  )}
                </p>
              ) : (
                <ul
                  className="lawha-awamir__qaima"
                  id="lawha-awamir-qaima"
                  role="listbox"
                  aria-label={t('lawha.tasmiya', lugha)}
                  ref={qaima}
                >
                  {majmuat.map((majmua) => (
                    <li key={majmua.majal} className="lawha-awamir__majmua" role="presentation">
                      <span className="lawha-awamir__majal" aria-hidden="true">
                        {majmua.majal}
                      </span>
                      <ul role="group" aria-label={majmua.majal}>
                        {majmua.awamir.map((amr, dakhil) => {
                          const mawqi = majmua.bidaya + dakhil;
                          return (
                            <li
                              key={amr.muarrif}
                              id={`lawha-awamir-khiyar-${String(mawqi)}`}
                              className="lawha-awamir__khiyar"
                              role="option"
                              aria-selected={mawqi === faal}
                              onClick={() => {
                                naffidh(amr);
                              }}
                              onMouseEnter={() => {
                                setFihris(mawqi);
                              }}
                            >
                              <span className="lawha-awamir__unwan" dir="auto">
                                <Muabbar nass={amr.unwan} ibra={ibra} />
                              </span>
                              {amr.ikhtisar === undefined ? null : (
                                <kbd className="lawha-awamir__ikhtisar">{amr.ikhtisar}</kbd>
                              )}
                            </li>
                          );
                        })}
                      </ul>
                    </li>
                  ))}
                </ul>
              )}
              <footer className="lawha-awamir__qaida">
                <ul className="lawha-awamir__mafatih">
                  {MAFATIH_QAIMA.map((miftah) => (
                    <li key={miftah.ramz} className="lawha-awamir__miftah">
                      <kbd className="lawha-awamir__ikhtisar">{miftah.ramz}</kbd>
                      {t(miftah.wasf, lugha)}
                    </li>
                  ))}
                </ul>
                <p className="lawha-awamir__talmih">
                  {t('lawha.talmih', lugha, { lawha: ikhtisarLawha, taraju: ikhtisarTaraju })}
                </p>
              </footer>
            </motion.div>
          </div>
        ) : null}
    </AnimatePresence>
  );
}
