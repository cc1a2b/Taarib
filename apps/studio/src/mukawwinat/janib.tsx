// الشريط الجانبي — the rail: the library, the open game's screens, the tools, and the fold at its foot.

import { Link, useRouterState } from '@tanstack/react-router';
import { motion } from 'motion/react';
import type { JSX } from 'react';

import type { LubaAkhira } from '@/hayat/hikal';
import type { MiftahLugha } from '@/lugha/lugha';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';
import type { KhasaisRamz } from '@/mukawwinat/rumuz';
import {
  RamzIdadat,
  RamzJanib,
  RamzLuba,
  RamzMaktaba,
  RamzMuraja,
  RamzTabaqa,
  RamzTalabat,
  RamzTaqdeem,
  RamzTashkhis,
  RamzTilqai,
  RamzWarsha,
} from '@/mukawwinat/rumuz';
import { HARAKAT_MASAR, haraka } from '@/nizam/haraka';

import './janib.css';

/** The five screens that belong to one game, addressed by its identity. */
type MasarLuba =
  | '/luba/$muarrif'
  | '/tilqai/$muarrif'
  | '/warsha/$muarrif'
  | '/taqdeem/$muarrif'
  | '/tabaqa/$muarrif';

/** The four tools, which need no game. */
type MasarAdaa = '/muraja' | '/talabat' | '/idadat' | '/tashkhis';

interface Band<M extends string> {
  /** Stable key, matching the screen's directory name. */
  readonly muarrif: string;
  readonly miftah: MiftahLugha;
  readonly masar: M;
  readonly ramz: (khasais: KhasaisRamz) => JSX.Element;
}

/**
 * The game's run, in the order a person meets the screens: the game itself,
 * the one-button run, the workshop the run feeds, the submission the workshop
 * produces, and the overlay that stands in when none of that applies.
 */
const SHASHAT_LUBA: readonly Band<MasarLuba>[] = [
  { muarrif: 'luba', miftah: 'shasha.luba', masar: '/luba/$muarrif', ramz: RamzLuba },
  { muarrif: 'tilqai', miftah: 'shasha.tilqai', masar: '/tilqai/$muarrif', ramz: RamzTilqai },
  { muarrif: 'warsha', miftah: 'shasha.warsha', masar: '/warsha/$muarrif', ramz: RamzWarsha },
  { muarrif: 'taqdeem', miftah: 'shasha.taqdeem', masar: '/taqdeem/$muarrif', ramz: RamzTaqdeem },
  { muarrif: 'tabaqa', miftah: 'shasha.tabaqa', masar: '/tabaqa/$muarrif', ramz: RamzTabaqa },
];

const ADAWAT: readonly Band<MasarAdaa>[] = [
  { muarrif: 'muraja', miftah: 'shasha.muraja', masar: '/muraja', ramz: RamzMuraja },
  { muarrif: 'talabat', miftah: 'shasha.talabat', masar: '/talabat', ramz: RamzTalabat },
  { muarrif: 'idadat', miftah: 'shasha.idadat', masar: '/idadat', ramz: RamzIdadat },
  { muarrif: 'tashkhis', miftah: 'shasha.tashkhis', masar: '/tashkhis', ramz: RamzTashkhis },
];

export interface KhasaisJanib {
  readonly lugha: Lugha;
  /**
   * The game the second run points at: the one whose screen is open, or the
   * last one opened on this machine. Null when there has never been one.
   */
  readonly luba: LubaAkhira | null;
  /** Whether the automatic run may be offered for that game; null while not yet known. */
  readonly tilqaiMasmuh: boolean | null;
  /** Whether the session holds the owner key; null when the probe failed. */
  readonly malik: boolean | null;
  /** Why the console cannot be opened, when the probe failed. */
  readonly sababJalsa: string | null;
  /** Whether the rail is folded to its glyphs. */
  readonly matwi: boolean;
  /** Whether the window strip above already carries the product's name. */
  readonly bilaRaas: boolean;
  readonly ala_tabdeel: () => void;
}

/** The moving mark on the live entry. One instance, so it travels between entries. */
function Mushir(): JSX.Element {
  return (
    <motion.span layoutId="mushir-tanaqqul" className="band__mushir" transition={haraka(HARAKAT_MASAR)} />
  );
}

/** An entry that cannot be opened, carrying the reason where there is one. */
function BandMatfi({
  ism,
  ramz: Ramz,
  matwi,
  sabab,
}: {
  readonly ism: string;
  readonly ramz: (khasais: KhasaisRamz) => JSX.Element;
  readonly matwi: boolean;
  readonly sabab: string | null;
}): JSX.Element {
  // A `title` on an element that already has text becomes its accessible
  // description, so the reason is announced as well as hovered. Folded, the
  // name itself is what the pointer needs.
  const talmih = sabab ?? (matwi ? ism : undefined);
  return (
    <li>
      <span className="band band--matfi" aria-disabled="true" title={talmih}>
        <Ramz className="band__ramz" />
        <span className="band__nass">{ism}</span>
      </span>
    </li>
  );
}

/**
 * The rail.
 *
 * Three runs, and the boundary between them is a real one: the library
 * stands alone; the game screens are enabled together when a game is open or
 * was opened, and disabled together with the one sentence that says what
 * would enable them; the tools need nothing. Folded, every entry keeps its
 * place and its glyph and hands its name to the pointer, so a person who
 * learned the rail open can still read it closed.
 */
export function Janib({
  lugha,
  luba,
  tilqaiMasmuh,
  malik,
  sababJalsa,
  matwi,
  bilaRaas,
  ala_tabdeel,
}: KhasaisJanib): JSX.Element {
  const masarHali = useRouterState({
    select: (halat) => halat.matches.at(-1)?.routeId ?? '/',
  });

  const nassTayy = t(matwi ? 'tanaqqul.bast' : 'tanaqqul.tayy', lugha);

  return (
    <aside className={matwi ? 'janib janib--matwi' : 'janib'}>
      {bilaRaas ? null : (
        <div className="janib__tarwisa">
          <span className="janib__ism">{t('tatbiq.ism', lugha)}</span>
        </div>
      )}

      <nav className="janib__tanaqqul" aria-label={t('tanaqqul.unwan', lugha)}>
        <div className="janib__majmua">
          <ul>
            <li>
              <Link
                to="/"
                className={masarHali === '/' ? 'band band--nashit' : 'band'}
                title={matwi ? t('shasha.maktaba', lugha) : undefined}
                {...(masarHali === '/' ? ({ 'aria-current': 'page' } as const) : {})}
              >
                {masarHali === '/' ? <Mushir /> : null}
                <RamzMaktaba className="band__ramz" />
                <span className="band__nass">{t('shasha.maktaba', lugha)}</span>
              </Link>
            </li>
          </ul>
        </div>

        <div className="janib__majmua">
          <h2 className="janib__fasl">{t('maktaba.tanaqqul.luba', lugha)}</h2>
          {luba === null ? (
            <p className="janib__tanbih">{t('maktaba.tanaqqul.bila_luba', lugha)}</p>
          ) : (
            <p className="janib__luba" dir="auto" title={luba.ism}>
              {luba.ism}
            </p>
          )}
          <ul>
            {SHASHAT_LUBA.map((shasha) => {
              const ism = t(shasha.miftah, lugha);
              if (luba === null) {
                return (
                  <BandMatfi key={shasha.muarrif} ism={ism} ramz={shasha.ramz} matwi={matwi} sabab={null} />
                );
              }
              // The second entry point to the automatic run, after the game
              // screen's own. Refused here for the same engines it is refused
              // there — in place, with the reason — because an absent entry
              // is indistinguishable from a broken one.
              if (shasha.muarrif === 'tilqai' && tilqaiMasmuh !== true) {
                return (
                  <BandMatfi
                    key={shasha.muarrif}
                    ism={ism}
                    ramz={shasha.ramz}
                    matwi={matwi}
                    sabab={tilqaiMasmuh === false ? t('luba.tilqai.mamnu', lugha) : null}
                  />
                );
              }
              const nashit = masarHali === shasha.masar;
              const Ramz = shasha.ramz;
              return (
                <li key={shasha.muarrif}>
                  <Link
                    to={shasha.masar}
                    params={{ muarrif: luba.muarrif }}
                    className={nashit ? 'band band--nashit' : 'band'}
                    title={matwi ? ism : undefined}
                    {...(nashit ? ({ 'aria-current': 'page' } as const) : {})}
                  >
                    {nashit ? <Mushir /> : null}
                    <Ramz className="band__ramz" />
                    <span className="band__nass">{ism}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>

        <div className="janib__majmua">
          <h2 className="janib__fasl">{t('maktaba.tanaqqul.taarib', lugha)}</h2>
          <ul>
            {ADAWAT.map((adaa) => {
              const ism = t(adaa.miftah, lugha);
              if (adaa.muarrif === 'muraja' && malik !== true) {
                // The console's route exists only in an owner session's table,
                // so a session that is not one has nothing to link to and the
                // entry is genuinely absent. A session whose probe *failed* is
                // a third case: not "not an owner" but "not known yet", and a
                // genuine owner watching the console vanish concludes it was
                // taken away. So it stays, disabled, carrying the reason.
                if (sababJalsa === null) {
                  return null;
                }
                return (
                  <BandMatfi key={adaa.muarrif} ism={ism} ramz={adaa.ramz} matwi={matwi} sabab={sababJalsa} />
                );
              }
              const nashit = masarHali === adaa.masar;
              const Ramz = adaa.ramz;
              return (
                <li key={adaa.muarrif}>
                  <Link
                    to={adaa.masar}
                    className={nashit ? 'band band--nashit' : 'band'}
                    title={matwi ? ism : undefined}
                    {...(nashit ? ({ 'aria-current': 'page' } as const) : {})}
                  >
                    {nashit ? <Mushir /> : null}
                    <Ramz className="band__ramz" />
                    <span className="band__nass">{ism}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      </nav>

      <button
        type="button"
        className="janib__tayy"
        aria-expanded={!matwi}
        aria-label={nassTayy}
        title={`${nassTayy} (Ctrl+B)`}
        onClick={ala_tabdeel}
      >
        <RamzJanib className="band__ramz" />
        <span className="band__nass">{nassTayy}</span>
      </button>
    </aside>
  );
}
