// رأس الشاشة — the one strip every sub-screen opens with: the way back, the screen's name, its subject, its tools.

import { Link } from '@tanstack/react-router';
import type { JSX, ReactNode } from 'react';

import { RamzRujoo } from '@/mukawwinat/rumuz';

import './raas_shasha.css';

/** Where the back link goes: the library, or the game the screen belongs to. */
export type Rujoo =
  | { readonly ila: 'maktaba' }
  | { readonly ila: 'luba'; readonly muarrif: string };

interface Khasais {
  readonly rujoo: Rujoo;
  /** The back link's label, from the screen's own string set. */
  readonly nassRujoo: string;
  /** The screen's name. */
  readonly unwan: string;
  /** What the screen is about — a game's name — when it is about one thing. */
  readonly mawdu?: string | null;
  /** A quiet fact beside the subject: a count, a session identity. */
  readonly tafasil?: ReactNode;
  /** Sibling screens of the same subject, as links. */
  readonly rawabit?: ReactNode;
  /** The screen's controls, at the trailing edge. */
  readonly adawat?: ReactNode;
}

/**
 * Nine screens carried nine copies of this strip, each with its own flex
 * alignment, and the one with `align-items: baseline` put the screen's name on
 * the strip's top edge beside a centred back link. One strip, one alignment:
 * every part is a flex item that either keeps its width or ellipsises, and the
 * strip itself clips, so nothing in it can ever overlap or leave the window.
 */
export function RaasShasha({
  rujoo,
  nassRujoo,
  unwan,
  mawdu,
  tafasil,
  rawabit,
  adawat,
}: Khasais): JSX.Element {
  const dakhil = (
    <>
      <RamzRujoo className="raas-shasha__ramz" />
      <span className="raas-shasha__rujoo-nass">{nassRujoo}</span>
    </>
  );
  return (
    <header className="raas-shasha">
      {rujoo.ila === 'maktaba' ? (
        <Link to="/" className="raas-shasha__rujoo">
          {dakhil}
        </Link>
      ) : (
        <Link to="/luba/$muarrif" params={{ muarrif: rujoo.muarrif }} className="raas-shasha__rujoo">
          {dakhil}
        </Link>
      )}
      <span className="raas-shasha__fasil" aria-hidden="true" />
      <div className="raas-shasha__hawiya">
        <h1 className="raas-shasha__unwan">{unwan}</h1>
        {mawdu === undefined || mawdu === null ? null : (
          <span className="raas-shasha__mawdu" dir="auto" title={mawdu}>
            {mawdu}
          </span>
        )}
        {tafasil === undefined || tafasil === null ? null : (
          <span className="raas-shasha__tafasil">{tafasil}</span>
        )}
      </div>
      {rawabit === undefined || rawabit === null ? null : (
        <nav className="raas-shasha__rawabit">{rawabit}</nav>
      )}
      {adawat === undefined || adawat === null ? null : (
        <div className="raas-shasha__adawat">{adawat}</div>
      )}
    </header>
  );
}
