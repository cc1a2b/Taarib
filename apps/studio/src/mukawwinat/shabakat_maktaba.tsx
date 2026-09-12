import { useVirtualizer } from '@tanstack/react-virtual';
import { motion } from 'motion/react';
import type { FocusEvent, JSX, KeyboardEvent, MouseEvent, RefObject } from 'react';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { flushSync } from 'react-dom';

import { useHikal } from '@/hayat/hikal';
import type { Munassiqat } from '@/lugha/lugha';
import { ittijah, jam, t } from '@/lugha/lugha';
import type { MinfathSuwar } from '@/maktaba/suwar';
import { ikhtarMasdar, masdarSalih, useSuwarMaktaba } from '@/maktaba/suwar';
import type { FiatGhaiba, LubaGhaiba, MajmuatMaktaba, SijillLuba } from '@/maktaba/tanqiya';
import { jammiGhiyab } from '@/maktaba/tanqiya';
import type { KathafatBitaqa } from '@/mukawwinat/bitaqa';
import { BitaqaLuba } from '@/mukawwinat/bitaqa';
import type {
  FiatGhiyab,
  Lugha,
  Manassa,
  SababGhiyab,
  SijillMutaadhir,
} from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './shabakat_maktaba.css';

/**
 * شبكة المكتبة — the library grid.
 *
 * Ten thousand games, at full frame rate, with the scroll position surviving
 * every refresh. Three decisions carry that, and each one is the opposite of
 * the obvious approach.
 *
 * ## Rows are virtualized, not cells
 *
 * The obvious design virtualizes a two-dimensional grid: a windowing library
 * that knows about columns and rows and renders the visible rectangle. It costs
 * more than it gives. A card is a fixed 240px wide and the container is a few
 * thousand pixels at most, so the horizontal axis is never more than a dozen
 * cells — there is nothing to save there — while a two-dimensional windower has
 * to re-measure both axes on every resize and gives up cheap `position: static`
 * row layout for absolute positioning per cell.
 *
 * So the column count is computed from the container's width and the card's
 * width at the current density, the library is cut into rows of that many
 * cards, and {@link useVirtualizer} windows the rows. Each row is one flex
 * container the browser lays out itself.
 *
 * ## The card scales in steps, the grid in columns
 *
 * The card has three sizes and no others, declared in `bitaqa.css` as three
 * density classes. Nothing here interpolates between them and nothing here
 * scales a card by a fraction: a card at 87% is a card whose 220×340 artwork is
 * resampled to 191×296 and visibly softened, on every card, permanently.
 *
 * What adapts is the column count and the gap. The gap absorbs the remainder so
 * that `columns × cardWidth + (columns − 1) × gap` lands exactly on the
 * available width, and every number is floored, because a card at a fractional
 * offset is a card whose artwork straddles two device pixels.
 *
 * ## Every dimension is measured, none is written here
 *
 * The virtualizer needs numbers and the card's size lives in CSS custom
 * properties that three density classes re-point. Hard-coding 240 and 394 here
 * would mean the grid and the card each own a copy of the same five numbers,
 * and the copies would diverge the first time somebody adjusted the card.
 *
 * So {@link useAbaadShabaka} renders a hidden probe carrying the same density
 * class the cards carry, and measures it. The card's tokens stay the single
 * source of the card's size.
 *
 * ## The window is what decides that artwork is wanted
 *
 * The grid is the only thing in the product that knows which rows are mounted —
 * that is what the virtualizer is for — so it is the thing that says when cover
 * art is worth resolving. The identities in the current window that have no
 * artwork on their record go to {@link useSuwarMaktaba}, which turns a stream
 * of them into one decision: run a pass, or do not.
 *
 * It is one decision and not one request per game, because the backend's
 * `hassil_suwar_maktaba` takes no arguments and walks the whole library. So the
 * window cannot be resolved *first*; what it can do is keep the pass from
 * starting before there is anything to resolve, keep it from starting twice,
 * and keep a fast scroll from asking again on every frame. See `maktaba/suwar.ts`
 * for the bounds and for what a per-game argument would change.
 *
 * Only games whose record carries no artwork are counted. A record that already
 * has a cover — the warm case, and most of a second visit — paints on the first
 * frame and is never overwritten by the stream: it is what the scan produced,
 * and swapping it later would be a picture changing under a pointer.
 *
 * Nothing about this can move the layout. The card reserves its artwork well at
 * a fixed size in every state, and a resolved cover changes the `src` of an
 * image that is already absolutely positioned inside that well — see the
 * governing rule in `nizam/tokens.css` and the reserve in `bitaqa.css`.
 */

/* ==========================================================================
   Measurement.
   ========================================================================== */

/** Every measurement the layout needs, in device-independent pixels. */
interface AbaadShabaka {
  /** One card's width at the current density. */
  readonly ardBitaqa: number;
  /** One card's full height, artwork plus status strip. */
  readonly irtifaBitaqa: number;
  /** The gap the design asks for, before the remainder is distributed. */
  readonly fajwaAsas: number;
  /** The grid's own inset from the edges of its scroll region. */
  readonly hashiya: number;
  /** The height of a launcher section's heading row. */
  readonly irtifaUnwan: number;
}

/** What to assume for one frame, before the probe has been measured. */
const ABAAD_MABDAIYA: AbaadShabaka = {
  ardBitaqa: 0,
  irtifaBitaqa: 0,
  fajwaAsas: 0,
  hashiya: 0,
  irtifaUnwan: 0,
};

/** Reads one probe child's box, floored, with zero for a missing child. */
function qisBoxAmudi(unsur: Element | null): number {
  if (unsur === null) {
    return 0;
  }
  return Math.floor(unsur.getBoundingClientRect().height);
}

/** Reads one probe child's width, floored, with zero for a missing child. */
function qisBoxUfuqi(unsur: Element | null): number {
  if (unsur === null) {
    return 0;
  }
  return Math.floor(unsur.getBoundingClientRect().width);
}

/**
 * Measures the card and the grid's spacing out of the token layer.
 *
 * Re-measured when the density changes, and again whenever the document's own
 * density or theme attribute changes: the card's five numbers are pixels and do
 * not move, but the grid's gap and inset are `rem`, and the interface density
 * setting changes the root font size under them. A grid that measured once
 * would keep a comfortable-density gap in a compact-density interface until the
 * next resize.
 *
 * @param kathafa the density the cards are being drawn at
 * @param miqyas the probe element
 */
function useAbaadShabaka(
  kathafa: KathafatBitaqa,
  miqyas: RefObject<HTMLDivElement | null>,
): AbaadShabaka {
  const [abaad, haddidAbaad] = useState<AbaadShabaka>(ABAAD_MABDAIYA);

  useLayoutEffect(() => {
    const unsur = miqyas.current;
    if (unsur === null) {
      return undefined;
    }

    let hayy = true;

    const qis = (): void => {
      if (!hayy) {
        return;
      }
      const bitaqa = unsur.querySelector('.shabaka__miqyas-bitaqa');
      const unwan = unsur.querySelector('.shabaka__miqyas-unwan');
      const fajwa = unsur.querySelector('.shabaka__miqyas-fajwa');
      const jadeed: AbaadShabaka = {
        ardBitaqa: qisBoxUfuqi(bitaqa),
        irtifaBitaqa: qisBoxAmudi(bitaqa),
        fajwaAsas: qisBoxUfuqi(fajwa),
        hashiya: qisBoxAmudi(fajwa),
        irtifaUnwan: qisBoxAmudi(unwan),
      };
      haddidAbaad((sabiq) => (mutasawiyaAbaad(sabiq, jadeed) ? sabiq : jadeed));
    };

    qis();

    const jidhr = document.documentElement;
    const raqib = new MutationObserver(qis);
    raqib.observe(jidhr, { attributes: true, attributeFilter: ['data-kathafa', 'data-sima'] });

    // The webview can finish loading the interface face after the first paint,
    // and a font swap changes what a rem measures. One more read once the font
    // set settles is cheaper than a grid laid out against a fallback face. The
    // flag is what keeps that late callback from measuring a probe this effect
    // has already torn down.
    const khutut = document.fonts as FontFaceSet | undefined;
    if (khutut !== undefined) {
      void khutut.ready.then(qis).catch(() => undefined);
    }

    return () => {
      hayy = false;
      raqib.disconnect();
    };
  }, [kathafa, miqyas]);

  return abaad;
}

/** Whether two measurements are the same, so state is not replaced for nothing. */
function mutasawiyaAbaad(awwal: AbaadShabaka, thani: AbaadShabaka): boolean {
  return (
    awwal.ardBitaqa === thani.ardBitaqa &&
    awwal.irtifaBitaqa === thani.irtifaBitaqa &&
    awwal.fajwaAsas === thani.fajwaAsas &&
    awwal.hashiya === thani.hashiya &&
    awwal.irtifaUnwan === thani.irtifaUnwan
  );
}

/** A scroll region's content box, floored to whole pixels. */
interface QiyasMutawa {
  /** The width available to the columns. */
  readonly ard: number;
  /** The height, which decides how many skeletons a screenful is. */
  readonly irtifa: number;
}

/**
 * Watches the scroll region's content box.
 *
 * `ResizeObserver` rather than a window resize listener, because the grid's
 * width changes when the sidebar collapses and when a detail pane opens, and
 * neither of those resizes the window.
 */
function useQiyasMutawa(marja: RefObject<HTMLElement | null>): QiyasMutawa {
  const [qiyas, haddidQiyas] = useState<QiyasMutawa>({ ard: 0, irtifa: 0 });

  useLayoutEffect(() => {
    const unsur = marja.current;
    if (unsur === null) {
      return undefined;
    }
    const sajjil = (ard: number, irtifa: number): void => {
      const jadeed: QiyasMutawa = { ard: Math.floor(ard), irtifa: Math.floor(irtifa) };
      haddidQiyas((sabiq) =>
        sabiq.ard === jadeed.ard && sabiq.irtifa === jadeed.irtifa ? sabiq : jadeed,
      );
    };
    const raqib = new ResizeObserver((madakhil) => {
      const madkhal = madakhil.at(0);
      if (madkhal === undefined) {
        return;
      }
      sajjil(madkhal.contentRect.width, madkhal.contentRect.height);
    });
    raqib.observe(unsur);
    // `clientWidth`, not the border box: the region scrolls, so its border box
    // is wider than its content by the scrollbar, and the column count is
    // computed from this number. A scrollbar's worth of phantom width is one
    // column too many on a container sitting just under a step, for the one
    // frame before the observer delivers its own first measurement — which is a
    // whole grid laid out and thrown away. The block padding is still counted
    // in the height, and is corrected by that same first delivery.
    sajjil(unsur.clientWidth, unsur.clientHeight);
    return () => {
      raqib.disconnect();
    };
  }, [marja]);

  return qiyas;
}

/* ==========================================================================
   Layout arithmetic.
   ========================================================================== */

/** How the cards are placed across the available width. */
interface QiyasSufuf {
  /** How many cards fit on one row. Never below one. */
  readonly aamida: number;
  /** The gap between two cards, after the remainder is distributed. */
  readonly fajwa: number;
  /** The leading inset that centres the block of columns. */
  readonly bidaya: number;
  /** One row's full height, card plus the vertical gap under it. */
  readonly irtifaSaf: number;
}

/** Nothing measured yet: one column, no gap, no row height. */
const QIYAS_MABDAI: QiyasSufuf = { aamida: 1, fajwa: 0, bidaya: 0, irtifaSaf: 1 };

/**
 * How far the gap is allowed to grow while absorbing the remainder.
 *
 * Twice the design's gap. Without a ceiling, a container just short of fitting
 * a third card hands the whole leftover — most of a card's width — to a single
 * gap, and two cards end up pinned to opposite edges of an empty row. Past the
 * ceiling the leftover becomes a centring inset instead, which reads as margin
 * rather than as a layout mistake.
 */
const AQSA_TAMDID_FAJWA = 2;

/**
 * Works out the column count, the gap and the centring inset.
 *
 * Everything is floored before it is used, not rounded at the end. A gap of
 * 16.4px repeated across nine columns puts the last card 3.6px off the whole
 * pixel, which on a 220px cover is a visibly resampled row of artwork next to a
 * crisp one.
 *
 * @param ard the scroll region's content width
 * @param abaad the measured card and spacing sizes
 */
function ihsibSufuf(ard: number, abaad: AbaadShabaka): QiyasSufuf {
  if (ard <= 0 || abaad.ardBitaqa <= 0 || abaad.irtifaBitaqa <= 0) {
    return QIYAS_MABDAI;
  }

  const mutah = Math.max(0, Math.floor(ard) - abaad.hashiya * 2);
  const khutwa = abaad.ardBitaqa + abaad.fajwaAsas;
  const aamida = Math.max(1, Math.floor((mutah + abaad.fajwaAsas) / khutwa));

  if (aamida === 1) {
    const baqi = Math.max(0, mutah - abaad.ardBitaqa);
    return {
      aamida: 1,
      fajwa: abaad.fajwaAsas,
      bidaya: abaad.hashiya + Math.floor(baqi / 2),
      irtifaSaf: abaad.irtifaBitaqa + abaad.fajwaAsas,
    };
  }

  const baqi = mutah - aamida * abaad.ardBitaqa;
  const muwazzaa = Math.floor(baqi / (aamida - 1));
  const saqf = abaad.fajwaAsas * AQSA_TAMDID_FAJWA;
  const fajwa = Math.max(abaad.fajwaAsas, Math.min(saqf, muwazzaa));
  const mustahlak = aamida * abaad.ardBitaqa + (aamida - 1) * fajwa;

  return {
    aamida,
    fajwa,
    bidaya: abaad.hashiya + Math.max(0, Math.floor((mutah - mustahlak) / 2)),
    irtifaSaf: abaad.irtifaBitaqa + abaad.fajwaAsas,
  };
}

/* ==========================================================================
   The flattened row model.
   ========================================================================== */

/** A launcher's heading, occupying a whole row. */
interface SafUnwan {
  readonly naw: 'unwan';
  /** A key that survives a data refresh. */
  readonly miftah: string;
  /** The heading text, already localized. */
  readonly unwan: string;
  /** How many games are under it. */
  readonly adad: number;
}

/** One row of cards. */
interface SafBitaqat {
  readonly naw: 'bitaqat';
  /** A key that survives a data refresh. */
  readonly miftah: string;
  /** The games on this row, at most one row's worth. */
  readonly sijillat: readonly SijillLuba[];
  /** The index of the first card, into the flat card order. */
  readonly awwal: number;
}

/** Everything the virtualizer windows. */
type SafShabaka = SafUnwan | SafBitaqat;

/** The flat rows, plus the flat card order the keyboard navigates. */
interface BinyatShabaka {
  /** Every row, in visual order. */
  readonly sufuf: readonly SafShabaka[];
  /** Every card, in visual order, as identities. */
  readonly tasalsul: readonly string[];
  /** Which row each card sits on, parallel to {@link tasalsul}. */
  readonly sufufBitaqat: readonly number[];
}

/** Nothing to show. */
const BINYA_FARIGHA: BinyatShabaka = { sufuf: [], tasalsul: [], sufufBitaqat: [] };

/**
 * Cuts the sections into rows of `aamida` cards.
 *
 * A section always starts on a fresh row: a launcher heading followed by three
 * of the previous launcher's games and then two of this one's would be a
 * heading that does not describe the row under it.
 *
 * The row key is the identity of its first card rather than its position,
 * which is what keeps the virtualizer's cache aligned across a refresh — see
 * the note on `getItemKey` in {@link ShabakatMaktaba}.
 *
 * @param majmuat the sorted, grouped library
 * @param aamida how many cards fit on a row
 */
function ibniSufuf(majmuat: readonly MajmuatMaktaba[], aamida: number): BinyatShabaka {
  if (aamida < 1) {
    return BINYA_FARIGHA;
  }

  const sufuf: SafShabaka[] = [];
  const tasalsul: string[] = [];
  const sufufBitaqat: number[] = [];

  for (const majmua of majmuat) {
    if (majmua.sijillat.length === 0) {
      continue;
    }
    if (majmua.unwan !== null) {
      sufuf.push({
        naw: 'unwan',
        miftah: `unwan:${majmua.muarrif}`,
        unwan: majmua.unwan,
        adad: majmua.sijillat.length,
      });
    }
    for (let bidaya = 0; bidaya < majmua.sijillat.length; bidaya += aamida) {
      const shariha = majmua.sijillat.slice(bidaya, bidaya + aamida);
      const raid = shariha.at(0);
      if (raid === undefined) {
        continue;
      }
      const martabatSaf = sufuf.length;
      sufuf.push({
        naw: 'bitaqat',
        miftah: `saf:${majmua.muarrif}:${raid.muarrif}`,
        sijillat: shariha,
        awwal: tasalsul.length,
      });
      for (const sijill of shariha) {
        tasalsul.push(sijill.muarrif);
        sufufBitaqat.push(martabatSaf);
      }
    }
  }

  return { sufuf, tasalsul, sufufBitaqat };
}

/* ==========================================================================
   The unavailable section.
   ========================================================================== */

/**
 * Wraps a run of text in the bidirectional isolate characters.
 *
 * A Windows path, a volume letter or a byte count dropped into an Arabic
 * sentence is a run of neutral and left-to-right characters inside a
 * right-to-left paragraph, and the bidirectional algorithm resolves the
 * slashes, colons and spaces around it against the paragraph. The user then
 * reads `C:\Games\...` reordered into a path that does not exist on their disk,
 * and retypes what they saw.
 *
 * The `.mono-ltr` class solves this for a whole element. It cannot be used
 * here, because these runs are *inside* a sentence that comes back from `t` as
 * one string. U+2068 and U+2069 do the same job without markup.
 *
 * @param nass the run to isolate
 */
function aazil(nass: string): string {
  return `\u2068${nass}\u2069`;
}

/**
 * The sentence explaining why one game is not in the library.
 *
 * A `switch` over the discriminant rather than a sentence sent from the
 * backend, because `SababGhiyab` carries the *facts* — a path, a percentage,
 * two byte counts — and the numbers in them have to be formatted with the digit
 * system and the units the user chose. A pre-formatted sentence from Rust would
 * arrive with Latin digits in an Arabic-Indic session.
 *
 * @param sabab the gate's reason
 * @param lugha the session's language
 * @param munassiq the session's formatters
 */
export function jumlatGhiyab(
  sabab: SababGhiyab,
  lugha: Lugha,
  munassiq: Munassiqat,
): string {
  switch (sabab.naw) {
    case 'qayd_tanzil':
      return sabab.nisba === null
        ? t('ghiyab.qayd_tanzil', lugha)
        : t('ghiyab.qayd_tanzil.nisba', lugha, {
            nisba: aazil(munassiq.nisba(sabab.nisba / 100)),
          });
    case 'qayd_tahdith':
      return t('ghiyab.qayd_tahdith', lugha);
    case 'tanzil_mutawaqqif':
      return sabab.nisba === null
        ? t('ghiyab.tanzil_mutawaqqif', lugha)
        : t('ghiyab.tanzil_mutawaqqif.nisba', lugha, {
            nisba: aazil(munassiq.nisba(sabab.nisba / 100)),
          });
    case 'tathbeet_naqis':
      return t('ghiyab.tathbeet_naqis', lugha, {
        hajm: aazil(munassiq.hajm(sabab.hajm)),
        adna: aazil(munassiq.hajm(sabab.adna)),
      });
    case 'mujallad_mafqud':
      return t('ghiyab.mujallad_mafqud', lugha, { masar: aazil(sabab.masar) });
    case 'mujallad_mamnu':
      return t('ghiyab.mujallad_mamnu', lugha, {
        masar: aazil(sabab.masar),
        sabab: sabab.sabab,
      });
    case 'tanfidhi_mafqud':
      return t('ghiyab.tanfidhi_mafqud', lugha, { masar: aazil(sabab.masar) });
    case 'tanfidhi_mukhtalif':
      return t('ghiyab.tanfidhi_mukhtalif', lugha, {
        mawjud: aazil(munassiq.hajm(sabab.mawjud)),
        musajjal: aazil(munassiq.hajm(sabab.musajjal)),
      });
    case 'himl_sahabi':
      return t('ghiyab.himl_sahabi', lugha);
    case 'beea_mafquda':
      return t('ghiyab.beea_mafquda', lugha, { masar: aazil(sabab.masar) });
    case 'tanfidhi_kharij_beea':
      return t('ghiyab.tanfidhi_kharij_beea', lugha, {
        masar: aazil(sabab.masar_windows),
        beea: aazil(sabab.beea),
      });
    case 'qurs_ghayr_muttasil':
      return t('ghiyab.qurs_ghayr_muttasil', lugha, { qurs: aazil(sabab.qurs) });
    case 'shabaka_ghayr_mutaha':
      return t('ghiyab.shabaka_ghayr_mutaha', lugha, { masar: aazil(sabab.masar) });
  }
}

/** The heading of one of the four groups. */
function unwanFia(fia: FiatGhiyab, lugha: Lugha): string {
  switch (fia) {
    case 'jariya':
      return t('ghiyab.fia.jariya', lugha);
    case 'naqis':
      return t('ghiyab.fia.naqis', lugha);
    case 'mafquda':
      return t('ghiyab.fia.mafquda', lugha);
    case 'ghayr_muttasila':
      return t('ghiyab.fia.ghayr_muttasila', lugha);
  }
}

/** One rejected game: its name, its launcher, why, and the way back to it. */
function SatrGhaiba(khasais: {
  readonly luba: LubaGhaiba;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly ala_fath_manassa: (manassa: Manassa, muarrif: string) => void;
}): JSX.Element {
  const { luba, lugha, munassiq, ala_fath_manassa } = khasais;

  const alaNaqr = useCallback(() => {
    ala_fath_manassa(luba.manassa, luba.muarrif);
  }, [ala_fath_manassa, luba.manassa, luba.muarrif]);

  return (
    <li className="ghaiba__satr">
      <div className="ghaiba__tarif">
        <span className="ghaiba__ism">{luba.ism}</span>
        <span className="ghaiba__manassa">{luba.ism_manassa}</span>
      </div>
      <p className="ghaiba__sabab">{jumlatGhiyab(luba.sabab, lugha, munassiq)}</p>
      <button type="button" className="zir zir--daqiq" onClick={alaNaqr}>
        {t('maktaba.ghiyab.iftah', lugha, { manassa: luba.ism_manassa })}
      </button>
    </li>
  );
}

/**
 * The games that exist and whose record this scan could not finish.
 *
 * Kept out of the unavailable section on purpose: those games are absent, these
 * are present and Taarib's own store failed on them. Telling a player their
 * installed game is missing would be false, and the remedy is different — the
 * error carries its own next step.
 */
function QismTaadhur(khasais: {
  readonly alaab: readonly SijillMutaadhir[];
  readonly lugha: Lugha;
}): JSX.Element | null {
  const { alaab, lugha } = khasais;
  if (alaab.length === 0) {
    return null;
  }

  return (
    <section className="taadhur">
      <h2 className="taadhur__tarwisa">
        {t('maktaba.taadhur.unwan', lugha, { adad: String(alaab.length) })}
      </h2>
      <p className="taadhur__sharh">{t('maktaba.taadhur.sharh', lugha)}</p>
      <ul className="taadhur__qaima">
        {alaab.map((luba) => (
          <li className="taadhur__satr" key={luba.muarrif}>
            <div className="taadhur__tarif">
              <span className="taadhur__ism">{luba.ism}</span>
              <span className="taadhur__manassa">{luba.manassa}</span>
            </div>
            <p className="taadhur__sabab">
              {lugha === 'arabi' ? luba.khata.arabi : luba.khata.injilizi}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * The collapsed section of games the launchers list and Taarib cannot use.
 *
 * At the very end of the grid and closed by default. A player who owns a game,
 * sees it in Steam and does not see it here concludes the product does not
 * support it — so every rejection is kept and explained rather than filtered
 * away. Closed by default because most users never have one, and a library that
 * opens with a list of faults looks broken.
 *
 * Rendered outside the virtualizer, below it, in the same scroll region. The
 * section's height depends on how many rows each of four groups holds and on
 * how long each reason sentence wraps to, which would make it the one variable
 * height item in an otherwise fixed-height windowing model — and a variable
 * height item forces dynamic measurement on every row in the grid above it.
 */
function QismGhiyab(khasais: {
  readonly fiat: readonly FiatGhaiba[];
  readonly adad: number;
  readonly matwi: boolean;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly ala_tabdeel: () => void;
  readonly ala_fath_manassa: (manassa: Manassa, muarrif: string) => void;
}): JSX.Element | null {
  const { fiat, adad, matwi, lugha, munassiq, ala_tabdeel, ala_fath_manassa } = khasais;
  if (fiat.length === 0) {
    return null;
  }

  return (
    <section className="ghaiba">
      <h2 className="ghaiba__tarwisa">
        <button
          type="button"
          className="ghaiba__miftah"
          aria-expanded={!matwi}
          onClick={ala_tabdeel}
        >
          <span className="ghaiba__sahm" aria-hidden="true" data-maftuh={String(!matwi)} />
          <span className="ghaiba__unwan">{t('maktaba.ghiyab.unwan', lugha)}</span>
          <span className="ghaiba__adad">
            {t('maktaba.ghiyab.adad', lugha, { adad: munassiq.raqm(adad) })}
          </span>
        </button>
      </h2>

      {matwi ? null : (
        <motion.div
          className="ghaiba__jism"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={haraka(HARAKAT_LAWHA)}
        >
          <p className="ghaiba__sharh">{t('maktaba.ghiyab.sharh', lugha)}</p>
          {fiat.map((fia) => (
            <div className="ghaiba__fia" key={fia.fia}>
              <h3 className="ghaiba__fia-unwan">{unwanFia(fia.fia, lugha)}</h3>
              <ul className="ghaiba__qaima">
                {fia.alab.map((luba) => (
                  <SatrGhaiba
                    key={luba.muarrif}
                    luba={luba}
                    lugha={lugha}
                    munassiq={munassiq}
                    ala_fath_manassa={ala_fath_manassa}
                  />
                ))}
              </ul>
            </div>
          ))}
        </motion.div>
      )}
    </section>
  );
}

/* ==========================================================================
   Skeletons and empty states.
   ========================================================================== */

/**
 * One placeholder card.
 *
 * The same outer box, the same artwork well, and the same reserve as a real
 * card, taken from the same tokens and the same density classes. That is the
 * whole point of it: when the scan finishes and the real cards arrive, nothing
 * on the screen moves. A placeholder that was a plain rectangle would be a
 * placeholder that reflowed the entire grid at the moment the data landed,
 * which is exactly the reflow the card's reserve exists to prevent.
 *
 * Deliberately still — no shimmer. A shimmering placeholder is an animation on
 * a data update, and this product does not animate data updates.
 */
function HaykalBitaqa(khasais: { readonly kathafa: KathafatBitaqa }): JSX.Element {
  const { kathafa } = khasais;
  const tabaqat =
    kathafa === 'qiyasi' ? 'haykal-bitaqa' : `haykal-bitaqa bitaqa--${kathafa}`;
  return (
    <div className={tabaqat}>
      <div className="haykal-bitaqa__bir">
        <div className="haykal-bitaqa__sura" />
      </div>
      <div className="haykal-bitaqa__shareet" />
    </div>
  );
}

/**
 * A screenful of placeholder cards, laid out exactly like the real grid.
 *
 * **Rows, not a wrapping flow.** The real grid is a stack of rows each
 * `irtifaSaf` tall — one card plus the design's own vertical gap — while the
 * *horizontal* gap has absorbed the row's remainder and is usually wider than
 * that. A single wrapping container given one `gap` therefore spaced the
 * placeholder rows by the horizontal figure, so the skeleton was looser than
 * the grid that replaced it and every row below the first landed somewhere
 * else when the scan finished.
 *
 * **The first heading is reserved.** A grouped library opens with a launcher
 * heading, and a skeleton with no heading row puts its first row of cards where
 * the second row will be — so the whole screen steps down by a heading's height
 * at the moment the data lands, which is precisely the reflow the card's
 * reserve exists to prevent, reintroduced one level up.
 *
 * Only the first. Where the second heading falls depends on how many games the
 * first launcher has, which is the question the scan is still running to
 * answer; a guessed cadence would misplace every row after it rather than only
 * the rows below the fold.
 */
function HaykalShabaka(khasais: {
  readonly kathafa: KathafatBitaqa;
  readonly qiyas: QiyasSufuf;
  readonly adadSufuf: number;
  readonly irtifaUnwan: number;
  readonly yujammi: boolean;
}): JSX.Element {
  const { kathafa, qiyas, adadSufuf, irtifaUnwan, yujammi } = khasais;

  const hashiya = {
    inlineSize: '100%',
    paddingInlineStart: `${String(qiyas.bidaya)}px`,
    paddingInlineEnd: `${String(qiyas.bidaya)}px`,
  } as const;

  const sufuf: number[] = [];
  for (let martaba = 0; martaba < adadSufuf; martaba += 1) {
    sufuf.push(martaba);
  }
  const aamida: number[] = [];
  for (let amud = 0; amud < qiyas.aamida; amud += 1) {
    aamida.push(amud);
  }

  return (
    <div className="shabaka__haykal" aria-hidden="true">
      {yujammi ? (
        <div
          className="shabaka__haykal-unwan"
          style={{ ...hashiya, blockSize: `${String(irtifaUnwan)}px` }}
        >
          {/* The rule and the name-shaped bar live on an inner element for the
              same reason the live heading's do: the row carries the centring
              inset as padding, and a border on the row itself would run the
              full width of the region while the heading it stands in for stops
              at the first card's leading edge. */}
          <span className="shabaka__haykal-unwan-qism" />
        </div>
      ) : null}
      {sufuf.map((martaba) => (
        <div
          key={martaba}
          className="shabaka__haykal-saf"
          // The row's own geometry is arithmetic, not style: the same three
          // numbers the virtualized rows are given, from the same measurement.
          style={{
            ...hashiya,
            display: 'flex',
            blockSize: `${String(qiyas.irtifaSaf)}px`,
            gap: `${String(qiyas.fajwa)}px`,
          }}
        >
          {aamida.map((amud) => (
            <HaykalBitaqa key={amud} kathafa={kathafa} />
          ))}
        </div>
      ))}
    </div>
  );
}

/**
 * The library found nothing.
 *
 * A sentence that names which launchers were searched, and two things the user
 * can do about it. No illustration, no emoji, no encouragement: the user is
 * looking at an empty library and wants to know why, and a friendly drawing is
 * an answer to a question nobody asked.
 *
 * The launchers named are the ones this machine actually has. Telling somebody
 * with Steam and GOG that Taarib also searched Battle.net and Riot makes the
 * sentence read as boilerplate rather than as a report.
 */
function FaraghMaktaba(khasais: {
  readonly manassat: readonly string[];
  readonly lugha: Lugha;
  readonly ala_idafa: () => void;
  readonly ala_tarshih: () => void;
}): JSX.Element {
  const { manassat, lugha, ala_idafa, ala_tarshih } = khasais;
  const fasila = lugha === 'arabi' ? '، ' : ', ';

  return (
    <div className="halat halat--farigh halat--shasha" role="status">
      <p className="halat__unwan">{t('maktaba.faragh.unwan', lugha)}</p>
      <p className="halat__nass">
        {manassat.length === 0
          ? t('maktaba.faragh.la_manassa', lugha)
          : t('maktaba.faragh.nass', lugha, { manassat: manassat.join(fasila) })}
      </p>
      <div className="halat__afal">
        <button type="button" className="zir" onClick={ala_idafa}>
          {t('maktaba.faragh.idafa', lugha)}
        </button>
        <button type="button" className="zir" onClick={ala_tarshih}>
          {t('maktaba.faragh.fahs', lugha)}
        </button>
      </div>
    </div>
  );
}

/** The library has games; the current query or filters exclude all of them. */
function FaraghTasfiya(khasais: {
  readonly adadKulli: number;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly ala_imsah: () => void;
}): JSX.Element {
  const { adadKulli, lugha, munassiq, ala_imsah } = khasais;
  return (
    <div className="halat halat--farigh halat--shasha" role="status">
      <p className="halat__unwan">{t('maktaba.faragh.tasfiya.unwan', lugha)}</p>
      <p className="halat__nass">
        {t('maktaba.faragh.tasfiya.nass', lugha, { kulli: munassiq.raqm(adadKulli) })}
      </p>
      <div className="halat__afal">
        <button type="button" className="zir" onClick={ala_imsah}>
          {t('maktaba.faragh.tasfiya.imsah', lugha)}
        </button>
      </div>
    </div>
  );
}

/* ==========================================================================
   Selection.
   ========================================================================== */

/** What the bulk bar can do to a selection. */
export type NawAmalJamai = 'tathbeet' | 'ikhfa';

/** The selection, and the anchor a range extends from. */
interface HalatIkhtiyar {
  /** Every selected game. */
  readonly mukhtara: ReadonlySet<string>;
  /** Where a Shift+click measures from, or null when nothing is selected. */
  readonly mirsat: string | null;
}

/** Nothing selected. */
const IKHTIYAR_FARIGH: HalatIkhtiyar = { mukhtara: new Set<string>(), mirsat: null };

/** Every identity between two positions in the visual order, inclusive. */
function madaBayn(
  tasalsul: readonly string[],
  awwal: number,
  thani: number,
): readonly string[] {
  const bidaya = Math.min(awwal, thani);
  const nihaya = Math.max(awwal, thani);
  return tasalsul.slice(bidaya, nihaya + 1);
}

/**
 * Multi-select with the modifiers, and range selection from an anchor.
 *
 * The anchor is the piece that is usually missing. Shift+click has to extend
 * from the last *deliberate* selection, not from whatever happens to be
 * highlighted: a user who ctrl-clicks four scattered games and then
 * shift-clicks a fifth expects the range to run from the fourth, and an
 * implementation that measures from the nearest selected item gives them a
 * different range every time depending on where they scrolled.
 *
 * The range is taken over the *visual* order, which is the order after the
 * current sort, filter and grouping. Shift-selecting across a launcher heading
 * therefore selects what the user can see between the two clicks, rather than
 * what the underlying array happens to hold between them.
 *
 * @param tasalsul every visible game, in visual order
 */
function useIkhtiyar(tasalsul: readonly string[]): {
  readonly halat: HalatIkhtiyar;
  readonly baddil: (muarrif: string, hadath: MouseEvent | KeyboardEvent) => void;
  readonly ikhtarKul: () => void;
  readonly imsah: () => void;
} {
  const [halat, haddidHalat] = useState<HalatIkhtiyar>(IKHTIYAR_FARIGH);

  // The visible set changes when a filter changes, and a selection that
  // outlived its games is a bulk action aimed at rows nobody can see. Pruned
  // rather than cleared, so narrowing a filter keeps the part of the selection
  // that is still on screen.
  useEffect(() => {
    haddidHalat((sabiq) => {
      if (sabiq.mukhtara.size === 0) {
        return sabiq;
      }
      const mawjuda = new Set(tasalsul);
      const baqiya = new Set<string>();
      for (const muarrif of sabiq.mukhtara) {
        if (mawjuda.has(muarrif)) {
          baqiya.add(muarrif);
        }
      }
      if (baqiya.size === sabiq.mukhtara.size) {
        return sabiq;
      }
      const mirsat = sabiq.mirsat !== null && mawjuda.has(sabiq.mirsat) ? sabiq.mirsat : null;
      return { mukhtara: baqiya, mirsat };
    });
  }, [tasalsul]);

  const baddil = useCallback(
    (muarrif: string, hadath: MouseEvent | KeyboardEvent) => {
      const mumtadd = hadath.shiftKey;
      const mudaf = hadath.ctrlKey || hadath.metaKey;

      haddidHalat((sabiq) => {
        if (mumtadd && sabiq.mirsat !== null) {
          const mawqiMirsat = tasalsul.indexOf(sabiq.mirsat);
          const mawqiHadaf = tasalsul.indexOf(muarrif);
          if (mawqiMirsat >= 0 && mawqiHadaf >= 0) {
            const mada = madaBayn(tasalsul, mawqiMirsat, mawqiHadaf);
            // Ctrl+Shift adds the range to what is already selected; Shift alone
            // replaces it. Both are what every file manager does, and a user who
            // has learned one has learned the other.
            const jadeeda = mudaf ? new Set(sabiq.mukhtara) : new Set<string>();
            for (const dakhil of mada) {
              jadeeda.add(dakhil);
            }
            return { mukhtara: jadeeda, mirsat: sabiq.mirsat };
          }
        }

        const jadeeda = new Set(sabiq.mukhtara);
        if (jadeeda.has(muarrif)) {
          jadeeda.delete(muarrif);
          // The anchor follows the last deliberate act even when that act was a
          // removal, so a range extended right after one starts where the user
          // last had their pointer.
          return { mukhtara: jadeeda, mirsat: muarrif };
        }
        jadeeda.add(muarrif);
        return { mukhtara: jadeeda, mirsat: muarrif };
      });
    },
    [tasalsul],
  );

  const ikhtarKul = useCallback(() => {
    haddidHalat({ mukhtara: new Set(tasalsul), mirsat: tasalsul.at(-1) ?? null });
  }, [tasalsul]);

  const imsah = useCallback(() => {
    haddidHalat(IKHTIYAR_FARIGH);
  }, []);

  return { halat, baddil, ikhtarKul, imsah };
}

/**
 * The bulk action bar.
 *
 * Present only while a selection exists, and anchored to the block end of the
 * grid rather than replacing the header: a bar that took over the header would
 * hide the search field and the sort control at exactly the moment the user is
 * most likely to want to narrow what they have selected.
 */
function ShareetJamai(khasais: {
  readonly adad: number;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly ala_amal: (naw: NawAmalJamai) => void;
  readonly ala_ikhtiyar_kul: () => void;
  readonly ala_ilgha: () => void;
}): JSX.Element {
  const { adad, lugha, munassiq, ala_amal, ala_ikhtiyar_kul, ala_ilgha } = khasais;

  const alaTathbeet = useCallback(() => {
    ala_amal('tathbeet');
  }, [ala_amal]);

  const alaIkhfa = useCallback(() => {
    ala_amal('ikhfa');
  }, [ala_amal]);

  return (
    <motion.div
      className="shareet-jamai"
      role="toolbar"
      aria-label={t('maktaba.ikhtiyar.shareet', lugha)}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={haraka(HARAKAT_LAWHA)}
    >
      <span className="shareet-jamai__adad">
        {jam('maktaba.ikhtiyar.adad', lugha, adad, munassiq)}
      </span>
      <div className="shareet-jamai__afaal">
        <button type="button" className="zir zir--daqiq" onClick={alaTathbeet}>
          {t('maktaba.ikhtiyar.tathbeet', lugha)}
        </button>
        <button type="button" className="zir zir--daqiq" onClick={alaIkhfa}>
          {t('maktaba.ikhtiyar.ikhfa', lugha)}
        </button>
        <button type="button" className="zir zir--daqiq" onClick={ala_ikhtiyar_kul}>
          {t('maktaba.ikhtiyar.kul', lugha)}
        </button>
        <button type="button" className="zir zir--daqiq" onClick={ala_ilgha}>
          {t('maktaba.ikhtiyar.ilgha', lugha)}
        </button>
      </div>
    </motion.div>
  );
}

/* ==========================================================================
   The grid itself.
   ========================================================================== */

/** Everything the grid needs to draw the library and act on it. */
export interface KhasaisShabakatMaktaba {
  /** The library, already filtered, searched, sorted and grouped. */
  readonly majmuat: readonly MajmuatMaktaba[];
  /** How many games the library holds before any query or filter. */
  readonly adadKulli: number;
  /** Whether a query or a filter is narrowing what is shown. */
  readonly munaqqa: boolean;
  /** The games the launchers list that the existence gate rejected. */
  readonly ghaiba: readonly LubaGhaiba[];
  readonly mutaadhira: readonly SijillMutaadhir[];
  /** The launchers that were searched, by their localized names. */
  readonly manassat: readonly string[];
  /** Whether the first scan is still running. */
  readonly yafhas: boolean;
  /** How large to draw the cards. */
  readonly kathafa: KathafatBitaqa;
  /** Whether the unavailable section is closed. */
  readonly ghiyabMatwi: boolean;
  /** The session's language, which also decides the arrow key semantics. */
  readonly lugha: Lugha;
  /** The session's formatters. */
  readonly munassiq: Munassiqat;
  /** Opens one game. */
  readonly ala_fath: (muarrif: string) => void;
  /**
   * Opens the context menu for one game at a point.
   *
   * Passed straight through from {@link BitaqaLuba}, and in the same coordinate
   * space it reports: `s` and `a` are **physical viewport** distances, from the
   * left and top edges, in both reading directions. A consumer positioning the
   * menu with `inset-inline-start` — which measures from the *right* edge under
   * `dir="rtl"` — has to mirror `s` itself, because the flip needs the width of
   * that menu's containing block and only the consumer knows which box that is.
   */
  readonly ala_qaima: (muarrif: string, s: number, a: number) => void;
  /** Runs one action against the whole selection. */
  readonly ala_amal_jamai: (naw: NawAmalJamai, muarrifat: readonly string[]) => void;
  /** Opens or closes the unavailable section. */
  readonly ala_tabdeel_ghiyab: () => void;
  /** Opens a launcher at one of its games. */
  readonly ala_fath_manassa: (manassa: Manassa, muarrif: string) => void;
  /** Starts the add-a-game-by-hand flow. */
  readonly ala_idafa_yadawiya: () => void;
  /** Starts the nominate-a-folder-to-scan flow. */
  readonly ala_tarshih_mujallad: () => void;
  /** Clears every filter and the query. */
  readonly ala_imsah_tasfiya: () => void;
  /**
   * Where cover art is resolved from, for the rows this grid has mounted.
   *
   * Absent everywhere in the product: the default port talks to the backend,
   * and the library screen has no reason to know that cover art is fetched at
   * all. It exists so that a harness measuring this grid can drive artwork in
   * without a backend behind it — which is the only way the claim that a cover
   * replacing a plate moves nothing can be measured rather than asserted.
   */
  readonly minfath_suwar?: MinfathSuwar;
}

/** How many rows of cards to keep rendered outside the window. */
const IHTIYAT = 2;

/**
 * Escapes a game identity for use inside an attribute selector.
 *
 * Taarib's own identities are UUIDs and need nothing, but a hand-added game is
 * identified by a hash of its executable's path, and a launcher's identifier is
 * whatever that launcher chose — Bottles uses the bottle's name, which is
 * user-supplied text. One quotation mark in a bottle name would turn a
 * `querySelector` call into a syntax error and break focus for the whole grid.
 *
 * `CSS.escape` is the correct answer where it exists; the fallback quotes the
 * characters that can terminate an attribute selector, which is enough because
 * the value is already inside double quotes at the call site.
 */
function cssIqtibas(qeema: string): string {
  const wahdat = globalThis.CSS as { escape?: (nass: string) => string } | undefined;
  const yahrub = wahdat?.escape;
  if (typeof yahrub === 'function') {
    return yahrub(qeema);
  }
  return qeema.replace(/["\\]/gu, (harf) => `\\${harf}`);
}

/**
 * The library grid.
 *
 * @param khasais everything the grid draws and everything it can do
 */
export function ShabakatMaktaba(khasais: KhasaisShabakatMaktaba): JSX.Element {
  const {
    majmuat,
    adadKulli,
    munaqqa,
    ghaiba,
    mutaadhira,
    manassat,
    yafhas,
    kathafa,
    ghiyabMatwi,
    lugha,
    munassiq,
    ala_fath,
    ala_qaima,
    ala_amal_jamai,
    ala_tabdeel_ghiyab,
    ala_fath_manassa,
    ala_idafa_yadawiya,
    ala_tarshih_mujallad,
    ala_imsah_tasfiya,
    minfath_suwar,
  } = khasais;

  const masrah = useRef<HTMLDivElement>(null);
  const miqyas = useRef<HTMLDivElement>(null);
  const lawh = useRef<HTMLDivElement>(null);

  const abaad = useAbaadShabaka(kathafa, miqyas);
  const mutawa = useQiyasMutawa(masrah);
  const qiyas = useMemo(() => ihsibSufuf(mutawa.ard, abaad), [mutawa.ard, abaad]);

  const binya = useMemo(() => ibniSufuf(majmuat, qiyas.aamida), [majmuat, qiyas.aamida]);
  const { sufuf, tasalsul, sufufBitaqat } = binya;

  // Whether the real list will interleave launcher headings — the one thing the
  // skeleton needs to know while the library it would read it from is still
  // empty. It can be read off the shape rather than from a setting: `jammi`
  // returns exactly one section with a null heading for the merged view and one
  // section per launcher otherwise, so a null heading can only mean merged.
  //
  // What the shape cannot say is which view an *empty* library is grouped by,
  // because grouping nothing produces no sections either way — and that is
  // precisely the state the skeleton is drawn in. Guessing wrong is the reflow
  // the skeleton exists to prevent, reintroduced one level up: a heading's
  // height added to or taken from every row on the screen at the moment the
  // scan lands. So the last answer is kept across the empty window a rescan
  // opens, and the product's own default — the merged view, which reserves
  // nothing — stands in until there has been one.
  const jamiSabiq = useRef(false);
  const yujammi = useMemo(
    () =>
      majmuat.length === 0
        ? jamiSabiq.current
        : !majmuat.some((majmua) => majmua.unwan === null),
    [majmuat],
  );
  useEffect(() => {
    if (majmuat.length > 0) {
      jamiSabiq.current = yujammi;
    }
  }, [majmuat.length, yujammi]);

  const fiatGhaiba = useMemo(() => jammiGhiyab(ghaiba), [ghaiba]);
  const { halat, baddil, ikhtarKul, imsah } = useIkhtiyar(tasalsul);

  const [nashit, haddidNashit] = useState<string | null>(null);
  const talabTarkeez = useRef<string | null>(null);

  /**
   * The card whose artwork travels with the next navigation.
   *
   * On the way out it is the card that was opened, named synchronously before
   * the router moves so the document's transition captures it under that
   * name. On the way back it is the game the shell last held, so the cover on
   * the game screen has a card to land on when the person returns.
   */
  const [muntaqal, haddidMuntaqal] = useState<string | null>(
    () => useHikal.getState().akhir_luba?.muarrif ?? null,
  );
  const alaFathMuntaqal = useCallback(
    (muarrif: string) => {
      flushSync(() => {
        haddidMuntaqal(muarrif);
      });
      ala_fath(muarrif);
    },
    [ala_fath],
  );

  const mudawwir = useVirtualizer({
    count: sufuf.length,
    getScrollElement: () => masrah.current,
    estimateSize: (martaba) =>
      sufuf.at(martaba)?.naw === 'unwan' ? abaad.irtifaUnwan : qiyas.irtifaSaf,
    // The scroll position has to survive a library refresh, and this is what
    // makes it survive. Keyed by index — the default — every row's cached
    // measurement belongs to a *position*, so a refresh that inserts one newly
    // installed game at the top shifts every row's identity by one and the
    // virtualizer re-measures the whole window against content that has moved
    // under it. Keyed by the identity of the row's first card, the row the user
    // is looking at keeps its key, its measurement and its offset, and the
    // refresh is invisible.
    //
    // The alternative usually reached for is to record `scrollTop` before the
    // update and write it back afterwards. It does not work here: the write
    // happens in an effect, which is after paint, so there is one frame at the
    // wrong position — a visible jump on every refresh — and it fights the
    // browser's own scroll anchoring rather than cooperating with it.
    getItemKey: (martaba) => sufuf.at(martaba)?.miftah ?? martaba,
    // The scroll region carries the grid's own inset as block padding, so the
    // first row starts that far below the scroll origin. Without this the
    // virtualizer computes its window against an origin the rows are not at,
    // and the last row of a screenful is unmounted while it is still visible.
    scrollMargin: abaad.hashiya,
    overscan: IHTIYAT,
  });

  // The row height changes with the density and with the interface's own
  // density setting, and the virtualizer caches sizes rather than re-reading
  // `estimateSize` for rows it has already measured.
  useEffect(() => {
    mudawwir.measure();
  }, [mudawwir, qiyas.irtifaSaf, abaad.irtifaUnwan, abaad.hashiya]);

  /* --- artwork ---------------------------------------------------------- */

  // Read once and used twice — for the request below and for the rows rendered
  // at the end of this component. Calling it a second time in the JSX would ask
  // the virtualizer to recompute a window that has not moved since.
  const banud = mudawwir.getVirtualItems();

  // The games the window is showing that have no cover on their record. Rebuilt
  // on every scroll frame, which is affordable because it is one window's worth
  // — tens of identities, not thousands — and because `useSuwarMaktaba`
  // compares it by content, so an unchanged window costs one length check and
  // schedules nothing.
  const zahira = useMemo(() => {
    const matluba: string[] = [];
    for (const band of banud) {
      const saf = sufuf.at(band.index);
      if (saf === undefined || saf.naw !== 'bitaqat') {
        continue;
      }
      for (const sijill of saf.sijillat) {
        // Asked for whenever the card's *preferred* source is absent, not only
        // when the row has no artwork at all. A game cached with a portrait and
        // no banner would otherwise sit on a heavy centre crop forever, when
        // the banner it wants is one fetch away. The hook settles a game the
        // moment its event arrives, including an event that resolved nothing,
        // so the single game with no artwork upstream is asked for once rather
        // than on every pass.
        if (sijill.batl === null) {
          matluba.push(sijill.muarrif);
        }
      }
    }
    return matluba;
  }, [banud, sufuf]);

  const suwar = useSuwarMaktaba(zahira, minfath_suwar);

  /* --- focus ------------------------------------------------------------ */

  /**
   * Roving tabindex, applied to the rendered cards from here.
   *
   * The card sets `tabIndex={0}` on itself and cannot do otherwise: it is used
   * outside this grid too, and a card that was unreachable by keyboard on a
   * detail screen would be a bug there. But a grid where every one of ten
   * thousand cards is a tab stop is a grid nobody can tab past, so the grid —
   * which is the thing that owns the focus model — overrides the attribute on
   * the cards it has rendered. One tab stop enters the grid; the arrow keys do
   * the rest.
   */
  useLayoutEffect(() => {
    const unsur = lawh.current;
    if (unsur === null) {
      return;
    }
    const bitaqat = unsur.querySelectorAll<HTMLElement>('[data-muarrif]');
    let awwal: HTMLElement | null = null;
    for (const bitaqa of bitaqat) {
      const muarrif = bitaqa.dataset['muarrif'];
      if (awwal === null) {
        awwal = bitaqa;
      }
      bitaqa.tabIndex = muarrif !== undefined && muarrif === nashit ? 0 : -1;
    }
    // Nothing is active yet, or what was active scrolled out of the window and
    // was replaced. Something inside the grid must remain reachable by Tab, so
    // the first rendered card takes the stop.
    if (nashit === null || unsur.querySelector(`[data-muarrif="${cssIqtibas(nashit)}"]`) === null) {
      if (awwal !== null) {
        awwal.tabIndex = 0;
      }
    }

    const matlub = talabTarkeez.current;
    if (matlub === null) {
      return;
    }
    const hadaf = unsur.querySelector<HTMLElement>(`[data-muarrif="${cssIqtibas(matlub)}"]`);
    if (hadaf !== null) {
      talabTarkeez.current = null;
      hadaf.tabIndex = 0;
      hadaf.focus();
    }
  });

  /** Moves the roving stop, scrolls the row into view, and asks for focus. */
  const intaqil = useCallback(
    (martaba: number) => {
      const muarrif = tasalsul.at(martaba);
      if (muarrif === undefined) {
        return;
      }
      haddidNashit(muarrif);
      talabTarkeez.current = muarrif;
      const saf = sufufBitaqat.at(martaba);
      if (saf !== undefined) {
        mudawwir.scrollToIndex(saf, { align: 'auto' });
      }
    },
    [mudawwir, sufufBitaqat, tasalsul],
  );

  /** The position the keyboard is at, defaulting to the first card. */
  const mawqiNashit = useCallback((): number => {
    if (nashit === null) {
      return tasalsul.length > 0 ? 0 : -1;
    }
    const mawjud = tasalsul.indexOf(nashit);
    return mawjud >= 0 ? mawjud : tasalsul.length > 0 ? 0 : -1;
  }, [nashit, tasalsul]);

  /**
   * Moves by whole rows, keeping the column.
   *
   * Heading rows are stepped over rather than landed on: a heading is not a
   * card and stopping on one would make ArrowDown feel like it sometimes does
   * nothing. A short last row clamps to its final card, which is what puts the
   * cursor somewhere sensible instead of nowhere.
   */
  const intiqalAmudi = useCallback(
    (mawqi: number, khutwa: number): number => {
      const martabatSaf = sufufBitaqat.at(mawqi);
      if (martabatSaf === undefined) {
        return mawqi;
      }
      const safHali = sufuf.at(martabatSaf);
      if (safHali === undefined || safHali.naw !== 'bitaqat') {
        return mawqi;
      }
      const amud = mawqi - safHali.awwal;
      const ittijahKhutwa = khutwa < 0 ? -1 : 1;
      let baqi = Math.abs(khutwa);
      let hadaf = martabatSaf;

      while (baqi > 0) {
        let taali = hadaf + ittijahKhutwa;
        while (taali >= 0 && taali < sufuf.length && sufuf.at(taali)?.naw !== 'bitaqat') {
          taali += ittijahKhutwa;
        }
        if (taali < 0 || taali >= sufuf.length) {
          break;
        }
        hadaf = taali;
        baqi -= 1;
      }

      const safHadaf = sufuf.at(hadaf);
      if (safHadaf === undefined || safHadaf.naw !== 'bitaqat') {
        return mawqi;
      }
      return safHadaf.awwal + Math.min(amud, safHadaf.sijillat.length - 1);
    },
    [sufuf, sufufBitaqat],
  );

  const rtl = ittijah(lugha) === 'rtl';

  // Card names in `tasalsul` order, for type-ahead. Same walk that built it.
  const asmaTasalsul = useMemo(() => {
    const asma: string[] = [];
    for (const saf of sufuf) {
      if (saf.naw === 'bitaqat') {
        for (const sijill of saf.sijillat) {
          asma.push(sijill.ism);
        }
      }
    }
    return asma;
  }, [sufuf]);

  /** The type-ahead buffer: letters typed within 700ms form one prefix. */
  const talqim = useRef({ nass: '', waqt: 0 });

  const alaMiftah = useCallback(
    (hadath: KeyboardEvent<HTMLDivElement>) => {
      // Enter, Space and the context-menu key belong to the card, which has
      // already called `preventDefault` on them by the time they bubble here.
      if (hadath.defaultPrevented || tasalsul.length === 0) {
        return;
      }

      if ((hadath.ctrlKey || hadath.metaKey) && hadath.key.toLowerCase() === 'a') {
        hadath.preventDefault();
        ikhtarKul();
        return;
      }
      if (hadath.key === 'Escape' && halat.mukhtara.size > 0) {
        hadath.preventDefault();
        imsah();
        return;
      }

      const mawqi = mawqiNashit();
      if (mawqi < 0) {
        return;
      }

      const safahat = Math.max(1, Math.floor(mutawa.irtifa / Math.max(1, qiyas.irtifaSaf)));

      switch (hadath.key) {
        case 'ArrowRight': {
          hadath.preventDefault();
          // In a right-to-left layout the leading edge is the right one, so
          // ArrowRight walks *backwards* through the visual order. This is the
          // one place in the grid where direction is not a CSS concern: the
          // keyboard's meaning genuinely inverts.
          intaqil(Math.min(tasalsul.length - 1, Math.max(0, mawqi + (rtl ? -1 : 1))));
          return;
        }
        case 'ArrowLeft': {
          hadath.preventDefault();
          intaqil(Math.min(tasalsul.length - 1, Math.max(0, mawqi + (rtl ? 1 : -1))));
          return;
        }
        case 'ArrowDown': {
          hadath.preventDefault();
          intaqil(intiqalAmudi(mawqi, 1));
          return;
        }
        case 'ArrowUp': {
          hadath.preventDefault();
          intaqil(intiqalAmudi(mawqi, -1));
          return;
        }
        case 'PageDown': {
          hadath.preventDefault();
          intaqil(intiqalAmudi(mawqi, safahat));
          return;
        }
        case 'PageUp': {
          hadath.preventDefault();
          intaqil(intiqalAmudi(mawqi, -safahat));
          return;
        }
        case 'Home': {
          hadath.preventDefault();
          intaqil(0);
          return;
        }
        case 'End': {
          hadath.preventDefault();
          intaqil(tasalsul.length - 1);
          return;
        }
        default: {
          if (
            hadath.key.length !== 1 ||
            hadath.key === ' ' ||
            hadath.ctrlKey ||
            hadath.metaKey ||
            hadath.altKey
          ) {
            return;
          }
          hadath.preventDefault();
          const silsila =
            hadath.timeStamp - talqim.current.waqt < 700
              ? talqim.current.nass + hadath.key
              : hadath.key;
          talqim.current = { nass: silsila, waqt: hadath.timeStamp };
          const ibra = silsila.toLowerCase();
          // A growing prefix keeps matching the current card; a fresh single
          // letter cycles onward from it.
          const min = silsila.length > 1 ? 0 : 1;
          for (let khutwa = min; khutwa <= asmaTasalsul.length; khutwa += 1) {
            const murashshah = (mawqi + khutwa) % asmaTasalsul.length;
            if (asmaTasalsul.at(murashshah)?.toLowerCase().startsWith(ibra) === true) {
              intaqil(murashshah);
              return;
            }
          }
          return;
        }
      }
    },
    [
      asmaTasalsul,
      halat.mukhtara.size,
      ikhtarKul,
      imsah,
      intaqil,
      intiqalAmudi,
      mawqiNashit,
      mutawa.irtifa,
      qiyas.irtifaSaf,
      rtl,
      tasalsul.length,
    ],
  );

  /** Keeps the roving stop on whatever the pointer actually focused. */
  const alaTarkeez = useCallback((hadath: FocusEvent<HTMLDivElement>) => {
    const hadaf = hadath.target.closest<HTMLElement>('[data-muarrif]');
    const muarrif = hadaf?.dataset['muarrif'];
    if (muarrif !== undefined) {
      haddidNashit(muarrif);
    }
  }, []);

  /* --- selection -------------------------------------------------------- */

  const alaAmal = useCallback(
    (naw: NawAmalJamai) => {
      ala_amal_jamai(naw, [...halat.mukhtara]);
    },
    [ala_amal_jamai, halat.mukhtara],
  );

  /* --- render ----------------------------------------------------------- */

  const jahiz = qiyas.irtifaSaf > 1 && abaad.ardBitaqa > 0;
  const laday = tasalsul.length > 0;
  // One row past the fold, so the last placeholder is cut by the viewport edge
  // rather than ending short of it and reading as the end of a small library.
  // One row until the probe has been read: dividing a real region height by the
  // unmeasured row height of one pixel asks for a thousand rows of placeholder.
  const sufufHaykal = jahiz ? Math.ceil(mutawa.irtifa / qiyas.irtifaSaf) + 1 : 1;

  return (
    <div className="shabaka">
      {/* The probe. Carries the card's own density class so that every number
          the layout uses is read out of the card's tokens rather than restated
          here — see the note at the top of this file. */}
      <div className="shabaka__miqyas" ref={miqyas} aria-hidden="true">
        <div
          className={
            kathafa === 'qiyasi'
              ? 'shabaka__miqyas-bitaqa'
              : `shabaka__miqyas-bitaqa bitaqa--${kathafa}`
          }
        />
        <div className="shabaka__miqyas-unwan" />
        <div className="shabaka__miqyas-fajwa" />
      </div>

      <div
        className="shabaka__masrah"
        ref={masrah}
        onKeyDown={alaMiftah}
        onFocusCapture={alaTarkeez}
      >
        {!laday && yafhas ? (
          <HaykalShabaka
            kathafa={kathafa}
            qiyas={qiyas}
            adadSufuf={sufufHaykal}
            irtifaUnwan={abaad.irtifaUnwan}
            yujammi={yujammi}
          />
        ) : null}

        {!laday && !yafhas && munaqqa ? (
          <div className="shabaka__halat">
            <FaraghTasfiya
              adadKulli={adadKulli}
              lugha={lugha}
              munassiq={munassiq}
              ala_imsah={ala_imsah_tasfiya}
            />
          </div>
        ) : null}

        {!laday && !yafhas && !munaqqa ? (
          <div className="shabaka__halat">
            <FaraghMaktaba
              manassat={manassat}
              lugha={lugha}
              ala_idafa={ala_idafa_yadawiya}
              ala_tarshih={ala_tarshih_mujallad}
            />
          </div>
        ) : null}

        {laday && jahiz ? (
          <div
            className="shabaka__sufuf"
            ref={lawh}
            role="grid"
            aria-label={t('maktaba.shabaka.tasmiya', lugha)}
            aria-rowcount={sufuf.length}
            aria-multiselectable="true"
            style={{ blockSize: `${String(mudawwir.getTotalSize())}px` }}
          >
            {banud.map((band) => {
              const saf = sufuf.at(band.index);
              if (saf === undefined) {
                return null;
              }
              // `start` is measured from the scroll origin and therefore
              // includes the scroll margin; the row is positioned inside the
              // list container, which begins after it.
              const asas = {
                transform: `translateY(${String(band.start - abaad.hashiya)}px)`,
                blockSize: `${String(band.size)}px`,
                paddingInlineStart: `${String(qiyas.bidaya)}px`,
                paddingInlineEnd: `${String(qiyas.bidaya)}px`,
              } as const;

              if (saf.naw === 'unwan') {
                return (
                  <div
                    key={band.key}
                    className="shabaka__saf shabaka__saf--unwan"
                    role="row"
                    aria-rowindex={band.index + 1}
                    style={asas}
                  >
                    <span className="shabaka__unwan-qism" role="rowheader">
                      {jam('maktaba.shabaka.qism', lugha, saf.adad, munassiq, {
                        unwan: saf.unwan,
                      })}
                    </span>
                  </div>
                );
              }

              return (
                <div
                  key={band.key}
                  className="shabaka__saf"
                  role="row"
                  aria-rowindex={band.index + 1}
                  style={{ ...asas, gap: `${String(qiyas.fajwa)}px` }}
                >
                  {saf.sijillat.map((sijill, amud) => (
                    <div
                      key={sijill.muarrif}
                      className="shabaka__khaliya"
                      role="gridcell"
                      aria-colindex={amud + 1}
                      // The grid declares itself multi-selectable, and the cell
                      // is where a grid's selection lives. Without this the
                      // declaration was a promise with nothing behind it: the
                      // card's own `aria-pressed` says a button is toggled,
                      // which is not the same fact and is not what a screen
                      // reader in grid mode reads.
                      aria-selected={halat.mukhtara.has(sijill.muarrif)}
                    >
                      <BitaqaLuba
                        muarrif={sijill.muarrif}
                        ism={sijill.ism}
                        // The record's own artwork wins, and is chosen between
                        // by the same function the resolver uses, so the frame
                        // a warm library paints is the frame a cold one arrives
                        // at. Normalised on the way through: the card's
                        // contract is a source the document can already load
                        // and a launcher's artwork arrives as a path on disk.
                        // `masdarSalih` is idempotent, so a value that is
                        // already a URL — from the resolver, or from a backend
                        // that converted it — passes through untouched.
                        ghilaf={
                          masdarSalih(ikhtarMasdar(sijill.ghilaf, sijill.batl)) ??
                          suwar.get(sijill.muarrif) ??
                          null
                        }
                        lawha={sijill.lawha}
                        nass={sijill.nass}
                        sawt={sijill.sawt}
                        lugha_rasmiya={sijill.lugha_rasmiya}
                        jahiziya={sijill.jahiziya}
                        // Whether any of the three below is an answer at all.
                        // The row carries a tier, an engine and a readiness
                        // verdict for a game nothing has ever opened, and each
                        // of those is a pessimistic default indistinguishable
                        // from a real finding; this is the field that separates
                        // them, and without it the card states a default as a
                        // conclusion.
                        mafhusa={sijill.mafhusa}
                        // Which of the three products this game gets, and the
                        // two facts that qualify it. All three straight off the
                        // row: the tier, the engine family and the Arabization
                        // status are each a decision the Rust side already made
                        // from the cached probe, and the card states them
                        // rather than deriving anything from them.
                        tabaqa={sijill.tabaqa}
                        muharrik={sijill.muharrik}
                        hala={sijill.hala}
                        mukhtara={halat.mukhtara.has(sijill.muarrif)}
                        muntaqal={sijill.muarrif === muntaqal}
                        kathafa={kathafa}
                        lugha={lugha}
                        ala_fath={alaFathMuntaqal}
                        ala_ikhtiyar={baddil}
                        ala_qaima={ala_qaima}
                      />
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        ) : null}

        {yafhas ? null : (
          <QismGhiyab
            fiat={fiatGhaiba}
            adad={ghaiba.length}
            matwi={ghiyabMatwi}
            lugha={lugha}
            munassiq={munassiq}
            ala_tabdeel={ala_tabdeel_ghiyab}
            ala_fath_manassa={ala_fath_manassa}
          />
        )}

        {yafhas ? null : <QismTaadhur alaab={mutaadhira} lugha={lugha} />}
      </div>

      {halat.mukhtara.size > 0 ? (
        <ShareetJamai
          adad={halat.mukhtara.size}
          lugha={lugha}
          munassiq={munassiq}
          ala_amal={alaAmal}
          ala_ikhtiyar_kul={ikhtarKul}
          ala_ilgha={imsah}
        />
      ) : null}

      <p className="shabaka__khafi" role="status">
        {yafhas
          ? t('maktaba.fahs.jari', lugha)
          : t('maktaba.adad_zahir', lugha, {
              adad: munassiq.raqm(tasalsul.length),
              kulli: munassiq.raqm(adadKulli),
            })}
      </p>
    </div>
  );
}
