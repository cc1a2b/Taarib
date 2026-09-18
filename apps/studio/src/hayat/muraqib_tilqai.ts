// مراقب التلقائي — one subscription to every automatic run, for the person who is on another screen when it ends.

import { useNavigate, useRouterState } from '@tanstack/react-router';
import { useEffect, useRef } from 'react';

import { ansha } from '@/hayat/tanbihat';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';
import type { WadTilqai } from '@/tilqai/aqd';
import { minfathTilqai } from '@/tilqai/aqd';

/** The game identity in a route's params, or null on a screen that has none. */
function muarrifMin(params: unknown): string | null {
  if (typeof params !== 'object' || params === null) {
    return null;
  }
  const qeema = (params as Readonly<Record<string, unknown>>)['muarrif'];
  return typeof qeema === 'string' && qeema !== '' ? qeema : null;
}

/**
 * Says how an automatic run ended when its own screen is not the one open.
 *
 * The run's screen subscribes to the same event and says the same thing in
 * the same words, but it unmounts with the screen, and the run does not: a
 * person who pressed once and went back to the library would otherwise learn
 * that their game was ready — or that the run stopped — only by coming back
 * to look. Mounted once, in the shell, so it outlives every screen.
 *
 * Only the report that carries a run from moving to stopped says anything.
 * A report for a run this session never saw moving — a launch that finds a
 * finished run in the store — is not an ending that happened to this person,
 * and an interruption is not an end. The run's own screen is left to speak
 * for itself while it is the screen in front of the reader; the notice store
 * also folds an identical sentence into one, so the two can never stack.
 *
 * @param ismLuba the game's name for an identity, from whatever the shell knows
 */
export function useMuraqibTilqai(lugha: Lugha, ismLuba: (muarrif: string) => string): void {
  const intiqal = useNavigate();
  const shashaTilqai = useRouterState({
    select: (halat) => {
      const akhir = halat.matches.at(-1);
      return akhir?.routeId === '/tilqai/$muarrif' ? muarrifMin(akhir.params) : null;
    },
  });

  // Read at the moment a report arrives rather than captured when the
  // subscription was made: the language, the open screen and the library's
  // names all change while one subscription stands.
  const marja = useRef({ lugha, ismLuba, shashaTilqai });
  useEffect(() => {
    marja.current = { lugha, ismLuba, shashaTilqai };
  }, [lugha, ismLuba, shashaTilqai]);

  const awda = useRef(new Map<string, WadTilqai>());

  useEffect(() => {
    let mushtarik = true;
    let alghi: (() => void) | null = null;

    void minfathTilqai
      .istami((laqta) => {
        const sabiq = awda.current.get(laqta.tashghila);
        awda.current.set(laqta.tashghila, laqta.wad);
        if (sabiq !== 'jariya' || laqta.wad === 'jariya' || laqta.wad === 'mutawaqqifa') {
          return;
        }
        const hali = marja.current;
        if (hali.shashaTilqai === laqta.muarrif) {
          return;
        }
        const ism = hali.ismLuba(laqta.muarrif);
        const amal = {
          unwan: t('shasha.tilqai', hali.lugha),
          nafidh: () => {
            void intiqal({ to: '/tilqai/$muarrif', params: { muarrif: laqta.muarrif } });
          },
        };
        switch (laqta.wad) {
          case 'jahiz':
            ansha({
              naw: 'najah',
              nass: t('tilqai.tanbih.jahiz', hali.lugha, { ism }),
              // A run that succeeded and still left half the game in its
              // original language says so here as well as on its own screen: a
              // person who pressed once and walked away learns what happened
              // from this notice and from nothing else.
              tafsil:
                laqta.yanfa_iltiqat && !laqta.multaqat
                  ? t('tilqai.tanbih.jahiz_naqis', hali.lugha)
                  : null,
              amal,
            });
            return;
          case 'mulgha':
            ansha({ naw: 'najah', nass: t('tilqai.tanbih.mulgha', hali.lugha, { ism }), amal });
            return;
          case 'fashal':
            ansha({
              naw: 'khatar',
              nass: t('tilqai.tanbih.fashal', hali.lugha, { ism }),
              tafsil:
                laqta.khata === null
                  ? null
                  : hali.lugha === 'arabi'
                    ? laqta.khata.arabi
                    : laqta.khata.injilizi,
              ramz: laqta.khata?.ramz ?? null,
              amal,
            });
            return;
          case 'iltiqat':
            ansha({ naw: 'tanbeeh', nass: t('tilqai.tanbih.iltiqat', hali.lugha, { ism }), amal });
            return;
        }
      })
      .then(
        (fak) => {
          if (mushtarik) {
            alghi = fak;
          } else {
            fak();
          }
        },
        () => {
          // Outside Tauri there is no event channel, and nothing to report about it.
        },
      );

    return () => {
      mushtarik = false;
      alghi?.();
    };
  }, [intiqal]);
}
