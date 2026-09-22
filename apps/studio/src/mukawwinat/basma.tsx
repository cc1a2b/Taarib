import type { JSX } from 'react';

import { majmuatBasma } from '@/mustalahat/khariji';

import './basma.css';

/**
 * البصمة — a sha256 set so that a person can actually compare two of them.
 *
 * A hash is sixty-four characters with no word boundaries, and a reader asked
 * to decide whether two of them are the same string will check the first four
 * characters, check the last four, and say yes. That is not carelessness; it is
 * what an unbroken run of hex does to a pair of eyes. The whole of the re-pin
 * decision in the review console rests on somebody seeing that two hashes
 * differ, so the difference has to be visible rather than merely present.
 *
 * So the hash is cut into eight-character groups — `majmuatBasma`, in the
 * vocabulary module, because the grouping is a fact about how this product
 * reads a hash rather than a decision this component is free to take — and the
 * groups that differ from the hash beside it are marked.
 *
 * **The mark is never colour alone.** A differing group takes the full text
 * colour, the medium weight, and a rule under it. Any one of those would be
 * enough for most readers and none of them is enough for all: the weight step
 * survives a monochrome display, the rule survives a colour-blind reader, and
 * both survive the light palette and the high-contrast one, because every value
 * is a token that re-points with the theme.
 *
 * The run is `mono-ltr` for the reason `qaida.css` gives at length: a hash is
 * mostly digits and letters with no strong direction, and inside an Arabic
 * paragraph the bidirectional algorithm will happily reorder it into something
 * that is not the hash the backend sent.
 */

interface Khasais {
  /** The hash, lowercase hex, exactly as it arrived. */
  readonly basma: string;
  /**
   * Which eight-character groups differ from the hash this one is set beside.
   *
   * Absent when there is nothing to compare against, which is every hash shown
   * on its own — an artifact's standing pin on the game screen, for instance.
   */
  readonly mukhtalifa?: ReadonlySet<number>;
  readonly className?: string;
}

/**
 * Which eight-character groups of two hashes differ.
 *
 * Compared group by group rather than character by character so that the mark
 * lands on a unit a reader can hold in their eye: a single altered character
 * inside a group marks that whole group, which is what makes a one-character
 * change as visible as a wholesale replacement. Two hashes of different lengths
 * — which should never happen, and would mean something is badly wrong — mark
 * every group past the shorter one rather than silently comparing nothing.
 */
export function majmuatMukhtalifa(awwal: string, thani: string): ReadonlySet<number> {
  const majmuatAwwal = majmuatBasma(awwal);
  const majmuatThani = majmuatBasma(thani);
  const tul = Math.max(majmuatAwwal.length, majmuatThani.length);
  const mukhtalifa = new Set<number>();
  for (let martaba = 0; martaba < tul; martaba += 1) {
    if (majmuatAwwal.at(martaba) !== majmuatThani.at(martaba)) {
      mukhtalifa.add(martaba);
    }
  }
  return mukhtalifa;
}

export function Basma({ basma, mukhtalifa, className }: Khasais): JSX.Element {
  const majmuat = majmuatBasma(basma);
  return (
    <span
      className={className === undefined ? 'basma mono-ltr' : `basma mono-ltr ${className}`}
      dir="ltr"
    >
      {majmuat.map((majmua, martaba) => (
        // Keyed by position as well as content, because two groups of one hash
        // can carry identical characters and the order is the whole meaning.
        <span
          key={`${String(martaba)}:${majmua}`}
          className={
            mukhtalifa !== undefined && mukhtalifa.has(martaba)
              ? 'basma__majmua basma__majmua--mukhtalifa'
              : 'basma__majmua'
          }
        >
          {majmua}
        </span>
      ))}
    </span>
  );
}
