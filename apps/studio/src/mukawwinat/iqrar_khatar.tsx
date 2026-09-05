import type { JSX, ReactNode } from 'react';

import { RamzTanbeeh } from '@/mukawwinat/rumuz';

import './iqrar_khatar.css';

/**
 * إقرار الخطر — the one shape in which this product asks somebody to take a
 * risk on knowingly.
 *
 * ## Why this is a component and not two hand-built panels
 *
 * There are exactly two risks a user can accept in Taarib — a multiplayer
 * game's anti-cheat noticing a modified file, and a patch whose build match is
 * only approximate — and they are asked for on two different screens. Every
 * other refusal in the product has no override at all: `Khutwa::LaShay`,
 * deliberately, because a VAC ban attaches to an account and no release lifts
 * it. So these two prompts are the entire surface on which a person can hand
 * themselves a permanent consequence, and two independently drawn versions of
 * that surface would drift — one growing a tick, the other a plain confirm
 * button; one naming the specific game, the other saying "are you sure".
 *
 * ## What the shape enforces
 *
 * **The question is named.** `unwan` says what is being accepted and `tahdheer`
 * says what the consequence is; `tafsil` carries the fact about *this* game or
 * *this* patch that made the question apply. A prompt that does not say what it
 * is asking about is worse than no prompt, because it teaches people that the
 * box is a door and the sentence above it is furniture.
 *
 * **The tick is the answer.** `muqirr` is the caller's state and the caller
 * passes it to the backend; nothing here defaults it to true, and nothing here
 * decides anything on the user's behalf. An untouched box is a "no" that the
 * backend then acts on, not a value the interface filled in.
 *
 * **The refusal explains itself.** While the box is untouched, `matlub` says
 * why the action beside it will not run. It carries a stable id — see
 * {@link muarrifMatlub} — so a button that lives elsewhere on the screen, such
 * as the automatic run's primary action up in the header, can point
 * `aria-describedby` at the same sentence rather than being merely grey.
 *
 * The leading rule is `--khatar` in every instance. This is not a warning about
 * quality or a notice that something is unfinished; both of the things asked
 * here put an account at stake, and the product's amber is spoken for by "read
 * this before you act".
 */

interface Khasais {
  /**
   * A document-unique fragment. The heading, the tick and the refusal sentence
   * are all keyed off it, so two of these on one screen — a patch list can draw
   * one per row — do not share a label.
   */
  readonly muarrif: string;
  /** What is being accepted, as a heading. */
  readonly unwan: string;
  /** The consequence, in full, in the reader's own language. */
  readonly tahdheer: string;
  /**
   * The fact about this particular game or patch that made the question apply —
   * the match verdict, the evidence a scan found — or null when the general
   * statement is the whole of it.
   */
  readonly tafsil?: string | null;
  /** The sentence the tick stands for, written in the first person. */
  readonly nassIqrar: string;
  /** The user's answer. Never defaulted, never inferred. */
  readonly muqirr: boolean;
  readonly alaTabdil: (muqirr: boolean) => void;
  /**
   * Why the action is refused while the box is untouched. Shown only while it
   * is, because a requirement already met is noise.
   */
  readonly matlub?: string | null;
  /** The controls the decision is taken with, when the caller puts them here. */
  readonly children?: ReactNode;
}

/**
 * The id of the refusal sentence for one acknowledgement.
 *
 * Exported because the control the sentence governs is not always inside this
 * block: on the automatic screen the primary action sits in the page header,
 * far above the question, and a grey button with no reason attached is the
 * failure this whole component exists to correct.
 */
export function muarrifMatlub(muarrif: string): string {
  return `${muarrif}-matlub`;
}

export function IqrarKhatar({
  muarrif,
  unwan,
  tahdheer,
  tafsil = null,
  nassIqrar,
  muqirr,
  alaTabdil,
  matlub = null,
  children,
}: Khasais): JSX.Element {
  const muarrifUnwan = `${muarrif}-unwan`;
  const muarrifHaql = `${muarrif}-haql`;
  const muarrifSabab = muarrifMatlub(muarrif);
  const yulzim = matlub !== null && !muqirr;

  return (
    <section className="iqrar" aria-labelledby={muarrifUnwan}>
      <h3 id={muarrifUnwan} className="iqrar__unwan">
        <RamzTanbeeh className="iqrar__ramz" aria-hidden />
        {unwan}
      </h3>
      <p className="iqrar__nass">{tahdheer}</p>
      {/* `dir="auto"` because this line is the backend's own words, and the
          backend writes some of them in Arabic only: a right-to-left sentence
          laid out left-to-right puts its full stop at the wrong end. */}
      {tafsil === null ? null : (
        <p className="iqrar__tafsil" dir="auto">
          {tafsil}
        </p>
      )}
      <label
        className={muqirr ? 'iqrar__khiyar iqrar__khiyar--muqirra' : 'iqrar__khiyar'}
        htmlFor={muarrifHaql}
      >
        <input
          id={muarrifHaql}
          className="iqrar__murabba"
          type="checkbox"
          checked={muqirr}
          aria-describedby={yulzim ? muarrifSabab : undefined}
          onChange={(hadath) => {
            alaTabdil(hadath.currentTarget.checked);
          }}
        />
        <span className="iqrar__khiyar-nass">{nassIqrar}</span>
      </label>
      {yulzim ? (
        <p id={muarrifSabab} className="iqrar__matlub">
          {matlub}
        </p>
      ) : null}
      {children === undefined ? null : <div className="iqrar__afal">{children}</div>}
    </section>
  );
}
