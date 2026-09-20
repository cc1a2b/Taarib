// كتلة الخطأ — the one failure block: the sentence first, the next step under it, the code kept at the foot for a report.

import { Link } from '@tanstack/react-router';
import type { JSX, ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';

import type { MiftahQism } from '@/hayat/aqsam_idadat';
import { QISM_SHASHA } from '@/hayat/aqsam_idadat';
import type { KhataJisr } from '@/hayat/jisr';
import type { MiftahLugha } from '@/lugha/lugha';
import { t } from '@/lugha/lugha';
import type { Lugha, MasarMatlub } from '@/mustalahat/awamir';
import { RamzTahaqquq, RamzTanbeeh } from '@/mukawwinat/rumuz';

import './kutlat_khata.css';

interface Khasais {
  readonly unwan: string;
  readonly khata: KhataJisr;
  readonly lugha: Lugha;
  /**
   * The game the failure belongs to, when one does.
   *
   * Nine of the steps are answered on a screen that is one game's — its
   * workspace, its overlay, its automatic run, its submission, its own page —
   * so this is what decides whether those draw the screen that answers them or
   * fall back to the library and to Diagnostics.
   */
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

interface KhasaisMaslak {
  readonly lugha: Lugha;
  /** What the control says, which is what the reader will do where it lands. */
  readonly miftah: MiftahLugha;
}

/**
 * Diagnostics: the floor under every failure.
 *
 * What a rejection with no step of its own is drawn as, and what a step whose
 * screen needs a game the caller never named falls back to. Never nothing.
 */
function ZirTashkhis({ lugha, miftah }: KhasaisMaslak): JSX.Element {
  return (
    <Link to="/tashkhis" className="zir">
      {t(miftah, lugha)}
    </Link>
  );
}

/** The library: where a game is found, its folder added, and the scan run again. */
function ZirMaktaba({ lugha, miftah }: KhasaisMaslak): JSX.Element {
  return (
    <Link to="/" className="zir">
      {t(miftah, lugha)}
    </Link>
  );
}

/**
 * Settings, opened at the section the step named.
 *
 * The section is the whole point. Settings is eleven folding sections and one
 * long scroll, and an "Open Settings" that lands at the top of it hands the
 * reader a document to search rather than the row their failure was about.
 */
function ZirIdadat({
  lugha,
  miftah,
  qism,
}: KhasaisMaslak & { readonly qism: MiftahQism }): JSX.Element {
  return (
    <Link to="/idadat" search={{ qism }} className="zir">
      {t(miftah, lugha)}
    </Link>
  );
}

/** One game's own screen, which holds its install plan, its patches and its removal. */
function ZirLuba({
  lugha,
  miftah,
  muarrif,
}: KhasaisMaslak & { readonly muarrif: string }): JSX.Element {
  return (
    <Link to="/luba/$muarrif" params={{ muarrif }} className="zir">
      {t(miftah, lugha)}
    </Link>
  );
}

/** The retry the aada family promises, when the caller handed one down. */
function ZirIaada({
  lugha,
  miftah,
  aada,
}: KhasaisMaslak & { readonly aada: () => void }): JSX.Element {
  return (
    <button type="button" className="zir" onClick={aada}>
      {t(miftah, lugha)}
    </button>
  );
}

/**
 * A step whose actor is not Taarib, stated as the directive it is.
 *
 * Three of the steps name something only the user can do, and only outside
 * this product: verifying a game through its own launcher, granting a
 * permission the operating system refused, reaching whoever contributed a
 * patch — a listing carries their display name and no address. Drawn as a
 * control, each would be a button that cannot perform the thing written on it,
 * which is worse than the sentence that says what to do; drawn as nothing,
 * which is what happened until now, the failure arrives with no answer at all.
 */
function Irshad({ lugha, miftah }: KhasaisMaslak): JSX.Element {
  return <p className="halat__khutwa">{t(miftah, lugha)}</p>;
}

/**
 * The drawing for a value this build has no case for.
 *
 * Only reachable from a backend newer than this interface. The `never` is the
 * point: a value added to `Khutwa` or `MasarMatlub` in the Rust vocabulary
 * fails to compile here the moment the bindings regenerate, and keeps failing
 * until the switch that lost it gives it a control. Should one ever arrive at
 * runtime anyway, it is drawn as the floor rather than as nothing.
 */
function ZirMajhul({ lugha }: { readonly lugha: Lugha; readonly qeema: never }): JSX.Element {
  return <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />;
}

/**
 * Where each of the six kinds of path Taarib can ask for is chosen.
 *
 * Three are rows in Settings. Two are a game's own — its folder and its
 * executable — and belong to that game's screen, or to the library when the
 * failure did not say which game. The patch file is answered by that game's
 * patch list, which is where a replacement comes from; the capture session by
 * that game's automatic run, which is what records one.
 */
function ZirMasar({
  matlub,
  lugha,
  muarrif,
}: {
  readonly matlub: MasarMatlub;
  readonly lugha: Lugha;
  readonly muarrif: string | undefined;
}): JSX.Element {
  switch (matlub) {
    case 'mujallad_manassa':
      return <ZirIdadat lugha={lugha} miftah="khutwa.fath_manassat" qism="manassat" />;
    case 'mujallad_ruqaa':
      return <ZirIdadat lugha={lugha} miftah="khutwa.fath_takhzin" qism="takhzin" />;
    case 'malaf_khatt':
      return <ZirIdadat lugha={lugha} miftah="khutwa.ikhtiyar_khatt_aakhar" qism="khutut" />;
    case 'mujallad_luba':
    case 'malaf_tanfidhi':
      return muarrif === undefined ? (
        <ZirMaktaba lugha={lugha} miftah="khutwa.fath_maktaba" />
      ) : (
        <ZirLuba lugha={lugha} miftah="khutwa.fath_luba" muarrif={muarrif} />
      );
    case 'malaf_ruqaa':
      return muarrif === undefined ? (
        <ZirMaktaba lugha={lugha} miftah="khutwa.fath_maktaba" />
      ) : (
        <ZirLuba lugha={lugha} miftah="khutwa.fath_ruqaa" muarrif={muarrif} />
      );
    case 'malaf_jalsa':
      return muarrif === undefined ? (
        <ZirMaktaba lugha={lugha} miftah="khutwa.fath_maktaba" />
      ) : (
        <Link to="/tilqai/$muarrif" params={{ muarrif }} className="zir">
          {t('khutwa.fath_tilqai', lugha)}
        </Link>
      );
    default:
      return <ZirMajhul lugha={lugha} qeema={matlub} />;
  }
}

/**
 * The one action the failure promises, drawn.
 *
 * Every value the backend can attach is drawn as something, because that is
 * what the backend's own type promises: a step is not advice text, it is a
 * value this block turns into a control. For most steps that is a button or a
 * link to the screen where the thing is done; for the three whose actor is
 * outside Taarib it is the directive itself. Only `la_shay` draws nothing, and
 * only because it is the one value that says there is nothing — its sentence
 * carries the whole answer, and a control invented here would be a second and
 * wrong one.
 *
 * A rejection that never reached a command carries no step at all, and is not
 * a dead end either: it is the transient class — the bridge down, the webview
 * between two states — so the caller's retry comes first and Diagnostics, which
 * the fallback sentence itself names, answers when there is no retry to offer.
 * That pair is also the floor for every step: a step never draws less than a
 * failure that had no step.
 */
function Zirr({ khasais }: { readonly khasais: Khasais }): JSX.Element | null {
  const { khata, lugha, muarrif, aada } = khasais;
  const khutwa = khata.khata?.khutwa;
  const iaadaAw = (miftah: MiftahLugha): JSX.Element =>
    aada === undefined ? (
      <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />
    ) : (
      <ZirIaada lugha={lugha} miftah={miftah} aada={aada} />
    );
  const lubaAw = (miftah: MiftahLugha): JSX.Element =>
    muarrif === undefined ? (
      <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />
    ) : (
      <ZirLuba lugha={lugha} miftah={miftah} muarrif={muarrif} />
    );

  if (khutwa === undefined) {
    return iaadaAw('amm.iaada');
  }
  switch (khutwa.naw) {
    case 'la_shay':
      return null;
    case 'aada_muhawala':
      return iaadaAw('amm.iaada');
    // Both rescans are run from a screen even when the caller holds no handler
    // for them: the library's own rescan, and the game screen's engine probe.
    case 'aada_fahs_maktaba':
      return aada === undefined ? (
        <ZirMaktaba lugha={lugha} miftah="khutwa.aada_fahs_maktaba" />
      ) : (
        <ZirIaada lugha={lugha} miftah="khutwa.aada_fahs_maktaba" aada={aada} />
      );
    case 'aada_fahs_muharrik':
      return aada === undefined ? (
        lubaAw('khutwa.aada_fahs_muharrik')
      ) : (
        <ZirIaada lugha={lugha} miftah="khutwa.aada_fahs_muharrik" aada={aada} />
      );
    case 'ikhtiyar_masar':
      return <ZirMasar matlub={khutwa.matlub} lugha={lugha} muarrif={muarrif} />;
    case 'ikhtiyar_khatt_aakhar':
      return <ZirIdadat lugha={lugha} miftah="khutwa.ikhtiyar_khatt_aakhar" qism="khutut" />;
    case 'fath_idadat':
      return (
        <ZirIdadat lugha={lugha} miftah="khutwa.fath_idadat" qism={QISM_SHASHA[khutwa.qism]} />
      );
    case 'fath_tashkhis':
      return <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />;
    // The overflow report is a submission's, not a log's: it lives on the
    // contributor's own submission screen — and, for a session holding the
    // owner key, in the review console — and never on Diagnostics, which is
    // where this step pointed for as long as it has existed.
    case 'fath_taqreer_tajawuz':
      return muarrif === undefined ? (
        <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />
      ) : (
        <Link to="/taqdeem/$muarrif" params={{ muarrif }} className="zir">
          {t('khutwa.fath_taqreer_tajawuz', lugha)}
        </Link>
      );
    case 'fath_nusus':
      return muarrif === undefined ? (
        <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />
      ) : (
        <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
          {t('khutwa.fath_nusus', lugha)}
        </Link>
      );
    case 'fath_tabaqa':
      return muarrif === undefined ? (
        <ZirTashkhis lugha={lugha} miftah="khutwa.fath_tashkhis" />
      ) : (
        <Link to="/tabaqa/$muarrif" params={{ muarrif }} className="zir">
          {t('khutwa.fath_tabaqa', lugha)}
        </Link>
      );
    case 'fath_tilqai':
      return muarrif === undefined ? (
        <ZirMaktaba lugha={lugha} miftah="khutwa.fath_maktaba" />
      ) : (
        <Link to="/tilqai/$muarrif" params={{ muarrif }} className="zir">
          {t('khutwa.fath_tilqai', lugha)}
        </Link>
      );
    case 'tahdith_taarib':
      return <ZirIdadat lugha={lugha} miftah="khutwa.tahdith_taarib" qism="tahdith" />;
    case 'iadat_tarkib_ittar':
      return lubaAw('khutwa.iadat_tarkib_ittar');
    case 'ilgha_tathbeet':
      return lubaAw('khutwa.ilgha_tathbeet');
    case 'iadat_mutabaqa_bina':
      return lubaAw('khutwa.iadat_mutabaqa_bina');
    case 'tahaqquq_salamat_luba':
      return <Irshad lugha={lugha} miftah="khutwa.tahaqquq_salamat_luba" />;
    case 'iblagh_lil_musahim':
      return <Irshad lugha={lugha} miftah="khutwa.iblagh_lil_musahim" />;
    // The bundle for the owner is built on the Diagnostics screen and nowhere
    // else, so the step lands there — saying what to do when it arrives.
    case 'iblagh_lil_malik':
      return <ZirTashkhis lugha={lugha} miftah="khutwa.iblagh_lil_malik" />;
    // Not a disk cleaner, which Taarib is not: the patch store's location and
    // its cache ceiling are the two things in this product that decide how much
    // of the disk it asks for, and both are rows in that one section.
    case 'tahrir_masaha':
      return <ZirIdadat lugha={lugha} miftah="khutwa.tahrir_masaha" qism="takhzin" />;
    case 'manh_salahiya':
      return <Irshad lugha={lugha} miftah="khutwa.manh_salahiya" />;
    default:
      return <ZirMajhul lugha={lugha} qeema={khutwa} />;
  }
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
