import type { JSX } from 'react';

import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';

/**
 * ائتمان الصانع — whose work this is, in one sentence, wherever it is listed.
 *
 * Taarib lists and installs third-party patches and builds none of them, and
 * the single rule that governs how they are presented is that Taarib never
 * presents one as its own. That rule is only kept if the credit travels with
 * the entry to every surface it appears on — the card in the game screen's
 * catalogue, the same card's detail, the re-pin rows in the owner's console,
 * and the notice raised when an install lands.
 *
 * Which is exactly why the sentence is here rather than written out at each of
 * those places. Four copies of one sentence in two languages is eight strings,
 * and eight strings drift: one grows the team name, another loses "installed by
 * Taarib", and the promise is quietly broken in the place nobody reread.
 *
 * Two forms, and the choice between them is not cosmetic. A patch made by one
 * person under a team's name credits both; a patch with no team, or whose team
 * name *is* the maker's name, credits the person once. "Emad Adel — Emad Adel"
 * is worse than no team at all.
 */

/**
 * The credit sentence, as a string, for a place that needs one — a notice, a
 * title attribute, an accessible name.
 *
 * @param muallif the maker, in their own spelling of their own name
 * @param fariq the team, when the work is a team's
 * @param lugha the language of the current session
 */
export function nassItiman(muallif: string, fariq: string | null, lugha: Lugha): string {
  return fariq === null || fariq.trim() === '' || fariq === muallif
    ? t('khariji.itiman_muallif', lugha, { muallif })
    : t('khariji.itiman', lugha, { muallif, fariq });
}

interface Khasais {
  readonly muallif: string;
  readonly fariq: string | null;
  readonly lugha: Lugha;
  /** The surface's own dress for the line; the sentence itself is the same everywhere. */
  readonly className?: string;
}

/**
 * The credit as an element.
 *
 * `dir="auto"` because a maker's name is written in their own script and the
 * two around it are not: an Arabic team name inside an English sentence, or a
 * Latin one inside an Arabic sentence, resolves from its own first strong
 * character and lands the right way round either way.
 */
export function ItimanKhariji({ muallif, fariq, lugha, className }: Khasais): JSX.Element {
  return (
    <p className={className} dir="auto">
      {nassItiman(muallif, fariq, lugha)}
    </p>
  );
}
