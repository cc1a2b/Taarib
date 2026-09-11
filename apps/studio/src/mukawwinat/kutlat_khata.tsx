// كتلة الخطأ — the one failure block: the sentence first, the next step under it, the code kept at the foot for a report.

import { Link } from '@tanstack/react-router';
import type { JSX, ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';

import type { KhataJisr } from '@/hayat/jisr';
import { t } from '@/lugha/lugha';
import type { Khutwa, Lugha, MasarMatlub } from '@/mustalahat/awamir';
import { RamzTahaqquq, RamzTanbeeh } from '@/mukawwinat/rumuz';

import './kutlat_khata.css';

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

/** How long the copy confirmation stands before the control reads as a control again. */
const MUDDAT_TAAKID = 1800;

interface KhasaisSatrRamz {
  /** The machine's own name for what happened: a code, a command, an address. */
  readonly ramz: string;
  readonly lugha: Lugha;
  /** What travels with the code into the clipboard — the machine's English, the command. */
  readonly tafsil?: string | null;
  /** The label over the code, when it is not an error code. */
  readonly tasmiya?: string;
}

/**
 * The code, at the foot of the block, and the one control that copies it.
 *
 * It used to lead the block with a warning glyph beside it, which made every
 * failure read as a developer dialog: the first thing a person saw was the
 * one thing written for a machine. The code is still the only part of a
 * failure that reads the same in both languages, and the only part worth
 * putting in a report, so it stays — quiet, last, and one press from the
 * clipboard.
 */
export function SatrRamz({ ramz, lugha, tafsil, tasmiya }: KhasaisSatrRamz): JSX.Element {
  const [nusikha, setNusikha] = useState(false);
  const muaqqit = useRef<number | null>(null);
  const marjaRamz = useRef<HTMLSpanElement | null>(null);

  useEffect(
    () => () => {
      if (muaqqit.current !== null) {
        window.clearTimeout(muaqqit.current);
      }
    },
    [],
  );

  const insakh = async (): Promise<void> => {
    const matn = tafsil === undefined || tafsil === null ? ramz : `${ramz} · ${tafsil}`;
    try {
      await navigator.clipboard.writeText(matn);
      setNusikha(true);
      if (muaqqit.current !== null) {
        window.clearTimeout(muaqqit.current);
      }
      muaqqit.current = window.setTimeout(() => {
        setNusikha(false);
      }, MUDDAT_TAAKID);
    } catch {
      // A webview that refuses the clipboard still lets the reader copy by
      // hand, so the code is selected for them instead of failing silently.
      const ikhtiyar = window.getSelection();
      if (ikhtiyar !== null && marjaRamz.current !== null) {
        ikhtiyar.selectAllChildren(marjaRamz.current);
      }
    }
  };

  return (
    <p className="halat__hamish">
      <span className="halat__hamish-tasmiya">{tasmiya ?? t('khata.ramz', lugha)}</span>
      <span ref={marjaRamz} className="halat__ramz mono-ltr">
        {ramz}
      </span>
      <button
        type="button"
        className={nusikha ? 'halat__nasakh halat__nasakh--tamm' : 'halat__nasakh'}
        aria-live="polite"
        onClick={() => {
          void insakh();
        }}
      >
        {nusikha ? <RamzTahaqquq /> : null}
        {t(nusikha ? 'khata.nusikha' : 'khata.nasakh', lugha)}
      </button>
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
      <figcaption className="halat__khaam-tasmiya">{t('khata.khaam', lugha)}</figcaption>
      <pre className="halat__khaam-nass mono-ltr">{nass}</pre>
    </figure>
  );
}

export interface KhasaisKutlatFashal {
  readonly unwan: string;
  readonly nass: string;
  /** The machine's own text, when the sentence above is a stand-in for one. */
  readonly khaam?: string | null;
  readonly ramz: string;
  /** What is copied beside the code for a report. */
  readonly tafsil?: string | null;
  readonly lugha: Lugha;
  readonly children?: ReactNode;
}

/**
 * The failure card, in reading order: what happened, what to do, and — last —
 * what to tell a maintainer. Every failure in the product is drawn through
 * this, whether it arrived as a rejected command, a snapshot's own error, or a
 * route that threw, so the three cannot drift apart.
 */
export function KutlatFashal({
  unwan,
  nass,
  khaam,
  ramz,
  tafsil,
  lugha,
  children,
}: KhasaisKutlatFashal): JSX.Element {
  return (
    <div className="halat halat--khata halat--fashal" role="alert">
      <p className="halat__unwan">
        <RamzTanbeeh className="halat__ramz-hala" />
        <span>{unwan}</span>
      </p>
      <p className="halat__nass">{nass}</p>
      {khaam === undefined || khaam === null ? null : <NassKhaam nass={khaam} lugha={lugha} />}
      {children === undefined || children === null ? null : (
        <div className="halat__afal">{children}</div>
      )}
      <SatrRamz ramz={ramz} lugha={lugha} tafsil={tafsil ?? null} />
    </div>
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
 * The sentence, the one promised action, anything the caller adds, and the
 * permanent code kept at the foot.
 *
 * When the rejection never reached a command there is no code and no sentence
 * written for a person: Tauri rejects a malformed invoke with a bare string, so
 * `KhataJisr` correctly declines to treat it as a structured error and this
 * block would otherwise say only that something went wrong. It has two things
 * even then — the command it failed on, and whatever text the bridge produced —
 * and both are shown: the command in the code's place, and the text below the
 * sentence, marked as a machine's words rather than dressed up as one of ours.
 *
 * A rejection whose severity is `maluma` — worth recording, invisible to the
 * user — is not a failure at all. It is a state the backend chose to answer
 * with rather than with data, and it is drawn as one: no glyph, no code, no
 * alert, just the sentence and the way on.
 */
export function KutlatKhata(khasais: Khasais): JSX.Element {
  const { unwan, khata, lugha, children } = khasais;
  const jumla = khata.nass(lugha) ?? t('faragh.jisr', lugha);
  const ramz = khata.khata?.ramz ?? khata.amr;
  // A rejection that carried no text at all has nothing to quote, and an empty
  // quotation under its own label reads as the block itself having failed.
  const khaam = khata.khata === null && khata.message !== '' ? khata.message : null;

  if (khata.khata?.khutura === 'maluma') {
    return (
      <div className="halat halat--farigh" role="status">
        <p className="halat__unwan">{unwan}</p>
        <p className="halat__nass">{jumla}</p>
        <div className="halat__afal">
          <Zirr khasais={khasais} />
          {children}
        </div>
      </div>
    );
  }

  const tafsil = [khata.amr, khata.khata?.injilizi ?? khaam ?? ''].filter((juz) => juz !== '').join(' · ');
  return (
    <KutlatFashal
      unwan={unwan}
      nass={jumla}
      khaam={khaam}
      ramz={ramz}
      tafsil={tafsil === '' ? null : tafsil}
      lugha={lugha}
    >
      <Zirr khasais={khasais} />
      {children}
    </KutlatFashal>
  );
}
