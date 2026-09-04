import { Link } from '@tanstack/react-router';
import type { JSX, ReactNode } from 'react';

import type { KhataJisr } from '@/hayat/jisr';
import { t } from '@/lugha/lugha';
import type { Khutwa, Lugha, MasarMatlub } from '@/mustalahat/awamir';
import { RamzMaluma, RamzTanbeeh } from '@/mukawwinat/rumuz';

import './kutlat_khata.css';

/** كتلة الخطأ — the one error block: the code, the sentence, and the next step. */

interface Khasais {
  readonly unwan: string;
  readonly khata: KhataJisr;
  readonly lugha: Lugha;
  /** The game the failure belongs to, when one does — it routes the fath_nusus step. */
  readonly muarrif?: string;
  /**
   * Runs the retry the aada-family steps promise, when the caller has one. A
   * rejection that never reached a command gets the same retry: it is the
   * transient class, and the caller's own refetch is the right first answer.
   */
  readonly aada?: () => void;
  readonly children?: ReactNode;
}

/**
 * The label over a machine's own words.
 *
 * `ar.json` and `en.json` are outside this change's file set, so the one string
 * this block gained lives here. It keeps `t`'s contract — present in both
 * languages — so lifting it into the string set under `khata.khaam` is a copy
 * rather than a rewrite.
 */
const TASMIYAT_KHAAM: Readonly<Record<Lugha, string>> = {
  arabi: 'نصّ الخدمة، حرفيًّا',
  injilizi: "The service's own words",
};

/**
 * The block's first line: the state, then the machine's name for it.
 *
 * The code leads the block because it is the only part of a failure that reads
 * the same in both languages and the only part worth typing into a search or a
 * report — but a code alone says nothing at a glance, so the family's state
 * glyph carries what kind of thing this is. It is the warning triangle rather
 * than the family's error cross: a cross is the shape of every close control in
 * every product, and a marker people try to click is worse than no marker.
 *
 * Exported alongside {@link NassKhaam} because the router's own two states are
 * the same three parts in the same order, and two copies of this would drift.
 */
export function SatrRamz({
  ramz,
  fashal,
}: {
  readonly ramz: string;
  /** A failure, as against a state that is merely not the one that was asked for. */
  readonly fashal: boolean;
}): JSX.Element {
  return (
    <p className="halat__ramz">
      {fashal ? <RamzTanbeeh /> : <RamzMaluma />}
      <span className="mono-ltr">{ramz}</span>
    </p>
  );
}

/**
 * A machine's own text, quoted rather than paraphrased.
 *
 * Its own label, its own surface and its own monospaced left-to-right run, so
 * that it reads as evidence and not as the product speaking badly.
 */
export function NassKhaam({
  nass,
  lugha,
}: {
  readonly nass: string;
  readonly lugha: Lugha;
}): JSX.Element {
  return (
    <figure className="halat__khaam">
      <figcaption className="halat__khaam-tasmiya">{TASMIYAT_KHAAM[lugha]}</figcaption>
      <pre className="halat__khaam-nass mono-ltr">{nass}</pre>
    </figure>
  );
}

/**
 * The path steps whose target is chosen in Settings. The other three — a game's
 * folder, its executable, a patch file to import — belong to the game screen
 * that raised them, which offers its own picker through `children`.
 */
const MATALIB_IDADAT: ReadonlySet<MasarMatlub> = new Set<MasarMatlub>([
  'mujallad_manassa',
  'mujallad_ruqaa',
  'malaf_khatt',
]);

/** Whether a step is answered by opening Settings, where fonts, paths and updates live. */
function yaftahIdadat(khutwa: Khutwa): boolean {
  switch (khutwa.naw) {
    case 'fath_idadat':
    case 'ikhtiyar_khatt_aakhar':
    case 'tahdith_taarib':
      return true;
    case 'ikhtiyar_masar':
      return MATALIB_IDADAT.has(khutwa.matlub);
    default:
      return false;
  }
}

/**
 * The one action the failure promises.
 *
 * A rejection that never reached a command carries no step, but it is not a
 * dead end: it is the transient class — the bridge down, the webview between
 * two states — so the caller's retry comes first, and the Diagnostics screen
 * the fallback sentence itself names is the answer when there is no retry to
 * offer. A step this block has no route for — reinstalling a framework,
 * freeing disk space, a launcher's own file check — is the caller's to add.
 */
function Zirr({ khasais }: { readonly khasais: Khasais }): JSX.Element | null {
  const { khata, lugha, muarrif, aada } = khasais;
  const khutwa = khata.khata?.khutwa;
  if (khutwa === undefined) {
    if (aada !== undefined) {
      return (
        <button type="button" className="zir" onClick={aada}>
          {t('amm.iaada', lugha)}
        </button>
      );
    }
    return (
      <Link to="/tashkhis" className="zir">
        {t('khutwa.fath_tashkhis', lugha)}
      </Link>
    );
  }
  const naw = khutwa.naw;
  if (
    (naw === 'aada_muhawala' || naw === 'aada_fahs_maktaba' || naw === 'aada_fahs_muharrik') &&
    aada !== undefined
  ) {
    const miftah =
      naw === 'aada_fahs_maktaba'
        ? ('khutwa.aada_fahs_maktaba' as const)
        : naw === 'aada_fahs_muharrik'
          ? ('khutwa.aada_fahs_muharrik' as const)
          : ('amm.iaada' as const);
    return (
      <button type="button" className="zir" onClick={aada}>
        {t(miftah, lugha)}
      </button>
    );
  }
  if (yaftahIdadat(khutwa)) {
    return (
      <Link to="/idadat" className="zir">
        {t('khutwa.fath_idadat', lugha)}
      </Link>
    );
  }
  if (naw === 'fath_tashkhis' || naw === 'fath_taqreer_tajawuz') {
    return (
      <Link to="/tashkhis" className="zir">
        {t('khutwa.fath_tashkhis', lugha)}
      </Link>
    );
  }
  if (naw === 'fath_nusus' && muarrif !== undefined) {
    return (
      <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
        {t('khutwa.fath_nusus', lugha)}
      </Link>
    );
  }
  return null;
}

/**
 * The permanent code, the sentence, the one promised action, and anything the
 * caller adds.
 *
 * When the rejection never reached a command there is no code and no sentence
 * written for a person: Tauri rejects a malformed invoke with a bare string, so
 * `KhataJisr` correctly declines to treat it as a structured error and this
 * block would otherwise say only that something went wrong. It has two things
 * even then — the command it failed on, and whatever text the bridge produced —
 * and both are shown: the command in the code's place, and the text below the
 * sentence, marked as a machine's words rather than dressed up as one of ours.
 * A user who can copy that line can be helped; a user reading "an error
 * occurred" cannot.
 */
export function KutlatKhata(khasais: Khasais): JSX.Element {
  const { unwan, khata, lugha, children } = khasais;
  const jumla = khata.nass(lugha);
  const ramz = khata.khata?.ramz ?? khata.amr;
  // A rejection that carried no text at all has nothing to quote, and an empty
  // quotation under its own label reads as the block itself having failed.
  const khaam = jumla === null && khata.message !== '' ? khata.message : null;
  return (
    <div className="halat halat--khata halat--fashal" role="alert">
      <SatrRamz ramz={ramz} fashal />
      <p className="halat__unwan">{unwan}</p>
      <p className="halat__nass">{jumla ?? t('faragh.jisr', lugha)}</p>
      {khaam === null ? null : <NassKhaam nass={khaam} lugha={lugha} />}
      <div className="halat__afal">
        <Zirr khasais={khasais} />
        {children}
      </div>
    </div>
  );
}
