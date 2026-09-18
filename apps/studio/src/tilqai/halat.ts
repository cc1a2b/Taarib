import { useCallback, useEffect, useRef, useState } from 'react';

import type {
  HalatIltiqat,
  HukmTilqai,
  KhataTilqai,
  LaqtatTilqai,
  MinfathTilqai,
} from '@/tilqai/aqd';
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
  /**
   * Whether a pass can be recorded for this game, and what is waiting.
   *
   * Null until the first answer arrives, and null again for a game whose state
   * could not be read — the two behave the same way, which is that the recorder
   * panel states the situation and offers nothing it cannot deliver.
   */
  readonly iltiqat: HalatIltiqat | null;
  /** Whether the recorder switch is in flight. */
  readonly yantazirIltiqat: boolean;
  /**
   * Starts a run; `istinaf` resumes the unfinished one rather than replacing it.
   *
   * `iqrarShabaka` is the user's answer to the multiplayer warning, passed
   * through untouched. It is a required parameter rather than an optional one on
   * purpose: every caller has to have obtained an answer, and a default here
   * would be this layer answering for them.
   */
  readonly ibda: (istinaf: boolean, iqrarShabaka: boolean, dammIltiqat: boolean) => void;
  readonly alghi: () => void;
  readonly aidHukm: () => void;
  readonly shaghghil: () => void;
  /** Turns the in-game recorder on or off for the next launch. */
  readonly sajjil: (mufaal: boolean) => void;
  /** Reads the recorder's state again, after a launch or a run. */
  readonly aidIltiqat: () => void;
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
  const [iltiqat, haddidIltiqat] = useState<HalatIltiqat | null>(null);
  const [yantazirIltiqat, haddidIntizarIltiqat] = useState(false);

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

  /**
   * The recorder's state, asked for once per game and again after anything that
   * could have changed it.
   *
   * A rejection is deliberately silent and leaves the answer null: not knowing
   * whether a pass is waiting must not stop the run's own screen from drawing,
   * and the panel that reads this states the situation rather than offering a
   * control it cannot drive.
   */
  const aidIltiqat = useCallback(() => {
    minfath.iltiqat(muarrif).then(
      (jawab) => {
        if (hayy.current) {
          haddidIltiqat(jawab);
        }
      },
      () => undefined,
    );
  }, [minfath, muarrif]);

  useEffect(() => {
    haddidIltiqat(null);
    haddidIntizarIltiqat(false);
    aidIltiqat();
  }, [muarrif, aidIltiqat]);

  const sajjil = useCallback(
    (mufaal: boolean) => {
      if (!hayy.current) {
        return;
      }
      haddidIntizarIltiqat(true);
      haddidKhataAmal(null);
      minfath.sajjil(muarrif, mufaal).then(
        (jawab) => {
          if (!hayy.current) {
            return;
          }
          haddidIltiqat(jawab);
          haddidIntizarIltiqat(false);
        },
        (khaam: unknown) => {
          if (!hayy.current) {
            return;
          }
          haddidKhataAmal(khataMin('sajjil', khaam));
          haddidIntizarIltiqat(false);
        },
      );
    },
    [minfath, muarrif],
  );

  const ibda = useCallback(
    (istinaf: boolean, iqrarShabaka: boolean, dammIltiqat: boolean) => {
      if (!hayy.current) {
        return;
      }
      haddidIntizar(true);
      haddidKhataAmal(null);
      minfath.ibda(muarrif, istinaf, iqrarShabaka, dammIltiqat).then(
        (jawab) => {
          if (!hayy.current) {
            return;
          }
          sajjilLaqta(jawab);
          haddidIntizar(false);
          // A run that was handed the recording switched the recorder off and
          // is about to consume the pass, so the panel's own state is stale the
          // moment the call returns.
          if (dammIltiqat) {
            aidIltiqat();
          }
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
    [minfath, muarrif, sajjilLaqta, aidIltiqat],
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
      () => {
        // The launch is what produces a pass, so what is on disk is re-read the
        // moment the person comes back to this window — which is the next time
        // anything on this screen is touched, not when the game exits.
        aidIltiqat();
      },
      (khaam: unknown) => {
        if (hayy.current) {
          haddidKhataAmal(khataMin('shaghghil', khaam));
        }
      },
    );
  }, [minfath, muarrif, aidIltiqat]);

  return {
    hukm,
    yuhammilHukm,
    khataHukm,
    laqta,
    khataAmal,
    yantazir,
    iltiqat,
    yantazirIltiqat,
    ibda,
    alghi,
    aidHukm,
    shaghghil,
    sajjil,
    aidIltiqat,
  };
}
