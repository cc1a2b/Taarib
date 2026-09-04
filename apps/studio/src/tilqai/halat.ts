import { useCallback, useEffect, useRef, useState } from 'react';

import type { HukmTilqai, KhataTilqai, LaqtatTilqai, MinfathTilqai } from '@/tilqai/aqd';
import { khataMin, minfathTilqai } from '@/tilqai/aqd';

/**
 * حالة التعريب التلقائي — the run, as one screen's worth of state.
 *
 * Three things arrive independently and this is where they are reconciled: the
 * verdict, which is asked for once; the snapshot of any unfinished run, which
 * is asked for once; and the stream of snapshots, which arrives for as long as
 * the screen is mounted. Nothing here is cached in the query client, because a
 * run is not server state that can be re-fetched — it is a subscription, and
 * treating it as a query would mean a stale snapshot could be handed back to a
 * screen that has since watched the run finish.
 */

/** A run that has stopped, one way or another. */
function intahat(laqta: LaqtatTilqai | null): boolean {
  return laqta !== null && laqta.wad !== 'jariya';
}

/** Everything the screen reads, and everything it can ask for. */
export interface HalatShasha {
  /** The verdict, or null while it is being computed or after it failed. */
  readonly hukm: HukmTilqai | null;
  readonly yuhammilHukm: boolean;
  readonly khataHukm: KhataTilqai | null;
  /** The run, or null when this game has never had one in this session. */
  readonly laqta: LaqtatTilqai | null;
  /** A failure from starting or cancelling, as opposed to one inside the run. */
  readonly khataAmal: KhataTilqai | null;
  /** Whether a start or a cancel is in flight and the buttons must refuse. */
  readonly yantazir: boolean;
  /** Starts a run; `istinaf` resumes the unfinished one rather than replacing it. */
  readonly ibda: (istinaf: boolean) => void;
  readonly alghi: () => void;
  readonly aidHukm: () => void;
  readonly shaghghil: () => void;
}

/**
 * The run for one game.
 *
 * @param muarrif the game, as the library identifies it
 * @param minfath the port to drive; the Tauri one unless a caller supplies its
 *   own, which is how the preview harness renders every state with no backend
 */
export function useTilqai(
  muarrif: string,
  minfath: MinfathTilqai = minfathTilqai,
): HalatShasha {
  const [hukm, haddidHukm] = useState<HukmTilqai | null>(null);
  const [yuhammilHukm, haddidTahmil] = useState(true);
  const [khataHukm, haddidKhataHukm] = useState<KhataTilqai | null>(null);
  const [laqta, haddidLaqta] = useState<LaqtatTilqai | null>(null);
  const [khataAmal, haddidKhataAmal] = useState<KhataTilqai | null>(null);
  const [yantazir, haddidIntizar] = useState(false);

  // False the moment the screen unmounts or the game changes, so a settled
  // promise from the previous game cannot write into the new one's state.
  const hayy = useRef(true);
  useEffect(() => {
    hayy.current = true;
    return () => {
      hayy.current = false;
    };
  }, [muarrif]);

  // The snapshot the stream is judged against, held in a ref as well as in
  // state: the subscription is established once per game and its callback would
  // otherwise close over the snapshot as it stood at subscription time.
  const marjaLaqta = useRef<LaqtatTilqai | null>(null);

  const sajjilLaqta = useCallback((jadida: LaqtatTilqai) => {
    marjaLaqta.current = jadida;
    haddidLaqta(jadida);
  }, []);

  const aidHukm = useCallback(() => {
    haddidTahmil(true);
    haddidKhataHukm(null);
    minfath.hukm(muarrif).then(
      (jawab) => {
        if (!hayy.current) {
          return;
        }
        haddidHukm(jawab);
        haddidTahmil(false);
      },
      (khaam: unknown) => {
        if (!hayy.current) {
          return;
        }
        haddidKhataHukm(khataMin('hukm', khaam));
        haddidTahmil(false);
      },
    );
  }, [minfath, muarrif]);

  useEffect(() => {
    haddidHukm(null);
    haddidKhataHukm(null);
    haddidKhataAmal(null);
    haddidIntizar(false);
    marjaLaqta.current = null;
    haddidLaqta(null);
    aidHukm();
  }, [muarrif, aidHukm]);

  // The unfinished run, asked for once per game. A rejection is deliberately
  // silent: not knowing whether there is something to resume must not stop the
  // verdict from being shown, and the resume line simply does not appear.
  useEffect(() => {
    minfath.laqta(muarrif).then(
      (jawab) => {
        if (hayy.current && jawab !== null && jawab.muarrif === muarrif) {
          sajjilLaqta(jawab);
        }
      },
      () => undefined,
    );
  }, [minfath, muarrif, sajjilLaqta]);

  useEffect(() => {
    let mushtarik = true;
    let alghi: (() => void) | null = null;

    void minfath
      .istami((warida) => {
        if (!hayy.current || warida.muarrif !== muarrif) {
          return;
        }
        const haliya = marjaLaqta.current;
        // A report from a run this screen is not watching is dropped unless the
        // run it *is* watching has already ended — which is what a second run
        // started after a cancelled first one looks like from here.
        if (haliya !== null && haliya.tashghila !== warida.tashghila && !intahat(haliya)) {
          return;
        }
        sajjilLaqta(warida);
      })
      .then(
        (fak) => {
          if (mushtarik) {
            alghi = fak;
          } else {
            fak();
          }
        },
        () => undefined,
      );

    return () => {
      mushtarik = false;
      alghi?.();
    };
  }, [minfath, muarrif, sajjilLaqta]);

  const ibda = useCallback(
    (istinaf: boolean) => {
      if (!hayy.current) {
        return;
      }
      haddidIntizar(true);
      haddidKhataAmal(null);
      minfath.ibda(muarrif, istinaf).then(
        (jawab) => {
          if (!hayy.current) {
            return;
          }
          sajjilLaqta(jawab);
          haddidIntizar(false);
        },
        (khaam: unknown) => {
          if (!hayy.current) {
            return;
          }
          haddidKhataAmal(khataMin('ibda', khaam));
          haddidIntizar(false);
        },
      );
    },
    [minfath, muarrif, sajjilLaqta],
  );

  const alghi = useCallback(() => {
    if (!hayy.current) {
      return;
    }
    haddidIntizar(true);
    haddidKhataAmal(null);
    minfath.alghi(muarrif).then(
      (jawab) => {
        if (!hayy.current) {
          return;
        }
        sajjilLaqta(jawab);
        haddidIntizar(false);
      },
      (khaam: unknown) => {
        if (!hayy.current) {
          return;
        }
        haddidKhataAmal(khataMin('alghi', khaam));
        haddidIntizar(false);
      },
    );
  }, [minfath, muarrif, sajjilLaqta]);

  const shaghghil = useCallback(() => {
    minfath.shaghghil(muarrif).then(
      () => undefined,
      (khaam: unknown) => {
        if (hayy.current) {
          haddidKhataAmal(khataMin('shaghghil', khaam));
        }
      },
    );
  }, [minfath, muarrif]);

  return {
    hukm,
    yuhammilHukm,
    khataHukm,
    laqta,
    khataAmal,
    yantazir,
    ibda,
    alghi,
    aidHukm,
    shaghghil,
  };
}
