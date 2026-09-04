import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useEffect, useMemo, useState } from 'react';

import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { jam, munassiqat, t } from '@/lugha/lugha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type {
  HasilatMaktaba,
  Idadat,
  Lugha,
  NizamArqam,
  SijillatHie,
  TashkhisHie,
} from '@/mustalahat/awamir';

import './tashkhis.css';

/** شاشة التشخيص — the log trail, the compatibility report, and the maintainer bundle. */

const SUTUR: readonly number[] = [100, 400, 1000];

/**
 * The five level names `tracing` writes, and the modifier each one takes.
 * A name outside the set falls to the torn-line face rather than to a colour
 * chosen for it, because a level this build does not know is not a severity.
 */
const FIAT_MUSTAWA: Readonly<Record<string, string>> = {
  ERROR: 'khata',
  WARN: 'tanbeeh',
  INFO: 'maluma',
  DEBUG: 'tafsil',
  TRACE: 'tatabbu',
};

/** One tail line as the viewer shows it. */
interface SatrSijill {
  readonly fia: string;
  readonly mustawa: string | null;
  readonly waqt: string | null;
  readonly hadaf: string | null;
  readonly risala: string;
  readonly nitaq: readonly string[];
  readonly huqul: readonly (readonly [string, string])[];
}

function nassAw(qeema: unknown): string | null {
  return typeof qeema === 'string' && qeema !== '' ? qeema : null;
}

function qaimatNusus(qeema: unknown): string[] {
  if (!Array.isArray(qeema)) {
    return [];
  }
  return (qeema as readonly unknown[]).filter((band): band is string => typeof band === 'string');
}

function huqulNusus(qeema: unknown): (readonly [string, string])[] {
  if (typeof qeema !== 'object' || qeema === null) {
    return [];
  }
  return Object.entries(qeema).map(
    ([ism, qeemat]) => [ism, typeof qeemat === 'string' ? qeemat : String(qeemat)] as const,
  );
}

/** The time of day out of an RFC 3339 stamp; the date is the log file's name. */
const NAMAT_WAQT = /T(\d{2}:\d{2}:\d{2})(\.\d{1,3})/;

function waqtQaseer(kamil: string): string {
  const mutabaqa = NAMAT_WAQT.exec(kamil);
  if (mutabaqa === null) {
    return kamil;
  }
  return `${mutabaqa[1] ?? ''}${mutabaqa[2] ?? ''}`;
}

/**
 * An RFC 3339 stamp trimmed to the second, for the file list. The nanoseconds a
 * filesystem reports are precision nobody reading a list of files is using, and
 * they push the column wider than the three of them together need to be.
 */
const NAMAT_LAHZA = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})/;

function lahzaQaseera(kamil: string): string {
  const mutabaqa = NAMAT_LAHZA.exec(kamil);
  if (mutabaqa === null) {
    return kamil;
  }
  return `${mutabaqa[1] ?? ''} ${mutabaqa[2] ?? ''}`;
}

/**
 * One tail line, read.
 *
 * The log is written as JSON lines, so a whole record parses into its parts and
 * the screen can align the timestamps and name the severity. A line the
 * rotating writer tore mid-record does not parse; it is shown exactly as it
 * stands rather than dropped, because a torn tail is the moment a reader most
 * needs the bytes that are there.
 */
function hallilSatr(khaam: string): SatrSijill {
  let munhal: unknown = null;
  try {
    munhal = JSON.parse(khaam);
  } catch {
    munhal = null;
  }

  if (typeof munhal !== 'object' || munhal === null) {
    return {
      fia: 'khaam',
      mustawa: null,
      waqt: null,
      hadaf: null,
      risala: khaam,
      nitaq: [],
      huqul: [],
    };
  }

  const sijill = munhal as Record<string, unknown>;
  const mustawa = nassAw(sijill['mustawa']);
  return {
    fia: (mustawa === null ? undefined : FIAT_MUSTAWA[mustawa]) ?? 'khaam',
    mustawa,
    waqt: nassAw(sijill['waqt']),
    hadaf: nassAw(sijill['hadaf']),
    risala: nassAw(sijill['risala']) ?? '',
    nitaq: qaimatNusus(sijill['nitaq']),
    huqul: huqulNusus(sijill['huqul']),
  };
}

export function Tashkhis(): JSX.Element {
  const makhzan = useQueryClient();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const [satr, setSatr] = useState(400);

  const sijillat = useQuery<SijillatHie, KhataJisr>({
    queryKey: [...mafatih.sijillat, satr],
    queryFn: () => nadi('sijillat_akhira', { satr }),
  });

  const akhir = sijillat.data?.akhir;
  const sutur = useMemo<readonly SatrSijill[]>(
    () => (akhir ?? []).map(hallilSatr),
    [akhir],
  );

  const maktaba = useQuery<HasilatMaktaba, KhataJisr>({
    queryKey: mafatih.maktaba,
    queryFn: () => nadi('maktaba'),
  });

  const [muarrif, setMuarrif] = useState('');
  const alaab = maktaba.data?.alaab;

  useEffect(() => {
    if (muarrif === '' && alaab !== undefined) {
      const awwal = alaab[0];
      if (awwal !== undefined) {
        setMuarrif(awwal.muarrif);
      }
    }
  }, [muarrif, alaab]);

  const taqreer = useMutation<string, KhataJisr, void>({
    mutationFn: () => nadi('taqreer_tawafuq', { muarrif }),
  });

  const huzma = useMutation<TashkhisHie, KhataJisr, void>({
    mutationFn: () => nadi('huzmat_tashkhis'),
  });

  const iftah = useMutation<boolean, KhataJisr, void>({
    mutationFn: () => nadi('iftah_tashkhis'),
  });

  useSajjilAwamir([
    {
      muarrif: 'tashkhis.ibni_huzma',
      unwan: t('tashkhis.awamir.ibni', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        huzma.mutate();
      },
    },
    {
      muarrif: 'tashkhis.iftah_mujallad',
      unwan: t('tashkhis.awamir.iftah', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        iftah.mutate();
      },
    },
    {
      muarrif: 'tashkhis.hadith_sijillat',
      unwan: t('tashkhis.awamir.hadith', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        void makhzan.invalidateQueries({ queryKey: mafatih.sijillat });
      },
    },
  ]);

  // The refresh is the toolbar's action and the action inside both of the log
  // section's empty states, so it is built once and placed in all three.
  const zirTahdith = (
    <button
      type="button"
      className="zir"
      onClick={() => {
        void sijillat.refetch();
      }}
    >
      {t('tashkhis.sijillat.hadith', lugha)}
    </button>
  );

  return (
    <div className="tashkhis">
      <header className="tashkhis__shareet-alawi">
        <Link to="/" className="tashkhis__raji">
          {t('tashkhis.raji', lugha)}
        </Link>
        <span className="tashkhis__fasl">{t('shasha.tashkhis', lugha)}</span>
      </header>

      <div className="tashkhis__jism">
        <section className="tashkhis__qism" aria-labelledby="tashkhis-unwan-sijillat">
          <div className="tashkhis__raas-qism">
            <h2 id="tashkhis-unwan-sijillat" className="tashkhis__unwan-qism">
              {t('tashkhis.sijillat.unwan', lugha)}
            </h2>
          </div>
          <div className="tashkhis__adawat">
            <label className="tashkhis__tasmiya" htmlFor="tashkhis-satr">
              {t('tashkhis.sijillat.satr', lugha)}
            </label>
            <select
              id="tashkhis-satr"
              className="tashkhis__haql"
              value={satr}
              onChange={(hadath) => {
                const qeema = Number(hadath.target.value);
                if (SUTUR.includes(qeema)) {
                  setSatr(qeema);
                }
              }}
            >
              {SUTUR.map((qeema) => (
                <option key={qeema} value={qeema}>
                  {munassiq.raqm(qeema)}
                </option>
              ))}
            </select>
            {zirTahdith}
          </div>
          {sijillat.isPending ? (
            <div className="haykal" aria-hidden="true">
              <span className="haykal__satr haykal__satr--tawil" />
              <span className="haykal__satr haykal__satr--mutawassit" />
              <span className="haykal__satr haykal__satr--qasir" />
            </div>
          ) : sijillat.error !== null ? (
            <KutlatKhata
              unwan={t('tashkhis.sijillat.taadhur', lugha)}
              khata={sijillat.error}
              lugha={lugha}
              aada={() => {
                void sijillat.refetch();
              }}
            />
          ) : sijillat.data === undefined ? null : sijillat.data.malaffat.length === 0 ? (
            <div className="tashkhis__farigh">
              <p className="tashkhis__farigh-unwan">
                {t('tashkhis.sijillat.la_malaffat', lugha)}
              </p>
              {zirTahdith}
            </div>
          ) : (
            <>
              <ul className="tashkhis__malaffat">
                {sijillat.data.malaffat.map((malaf) => (
                  <li key={malaf.ism} className="tashkhis__malaf">
                    <span className="tashkhis__malaf-ism mono-ltr">{malaf.ism}</span>
                    <span className="tashkhis__malaf-hajm">{munassiq.hajm(malaf.hajm)}</span>
                    <span className="tashkhis__malaf-waqt mono-ltr" dir="ltr">
                      {lahzaQaseera(malaf.waqt)}
                    </span>
                  </li>
                ))}
              </ul>
              {sutur.length === 0 ? (
                <div className="tashkhis__farigh">
                  <p className="tashkhis__farigh-unwan">
                    {t('tashkhis.sijillat.la_akhir', lugha)}
                  </p>
                  {zirTahdith}
                </div>
              ) : (
                <div
                  className="tashkhis__dhayl"
                  dir="ltr"
                  tabIndex={0}
                  role="group"
                  aria-label={t('tashkhis.sijillat.dhayl', lugha)}
                >
                  <ol className="tashkhis__sutur">
                    {sutur.map((band, fihris) => (
                      <li
                        key={fihris}
                        className={`tashkhis__satr tashkhis__satr--${band.fia}`}
                      >
                        <span className="tashkhis__satr-mustawa">{band.mustawa ?? ''}</span>
                        <span className="tashkhis__satr-waqt">
                          {band.waqt === null ? '' : waqtQaseer(band.waqt)}
                        </span>
                        <span className="tashkhis__satr-jism">
                          {band.hadaf === null ? null : (
                            <span className="tashkhis__satr-hadaf">{band.hadaf}</span>
                          )}
                          <span className="tashkhis__satr-risala">{band.risala}</span>
                          {band.nitaq.length === 0 ? null : (
                            <span className="tashkhis__satr-nitaq">
                              {band.nitaq.join(' > ')}
                            </span>
                          )}
                          {band.huqul.map(([ism, qeema]) => (
                            <span key={ism} className="tashkhis__satr-haql">
                              <span className="tashkhis__satr-haql-ism">{ism}</span>={qeema}
                            </span>
                          ))}
                        </span>
                      </li>
                    ))}
                  </ol>
                </div>
              )}
            </>
          )}
        </section>

        <section className="tashkhis__qism" aria-labelledby="tashkhis-unwan-tawafuq">
          <div className="tashkhis__raas-qism">
            <h2 id="tashkhis-unwan-tawafuq" className="tashkhis__unwan-qism">
              {t('tashkhis.tawafuq.unwan', lugha)}
            </h2>
          </div>
          {maktaba.isPending ? (
            <div className="haykal" aria-hidden="true">
              <span className="haykal__satr haykal__satr--mutawassit" />
              <span className="haykal__satr haykal__satr--qasir" />
            </div>
          ) : maktaba.error !== null ? (
            <KutlatKhata
              unwan={t('tashkhis.tawafuq.taadhur_maktaba', lugha)}
              khata={maktaba.error}
              lugha={lugha}
              aada={() => {
                void maktaba.refetch();
              }}
            />
          ) : alaab === undefined || alaab.length === 0 ? (
            <div className="tashkhis__farigh">
              <p className="tashkhis__farigh-unwan">{t('tashkhis.tawafuq.la_alab', lugha)}</p>
              <Link to="/" className="zir">
                {t('tashkhis.raji', lugha)}
              </Link>
            </div>
          ) : (
            <>
              <div className="tashkhis__adawat">
                <label className="tashkhis__tasmiya" htmlFor="tashkhis-luba">
                  {t('tashkhis.tawafuq.luba', lugha)}
                </label>
                <select
                  id="tashkhis-luba"
                  className="tashkhis__haql"
                  value={muarrif}
                  onChange={(hadath) => {
                    setMuarrif(hadath.target.value);
                  }}
                >
                  {alaab.map((luba) => (
                    <option key={luba.muarrif} value={luba.muarrif}>
                      {luba.ism}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="zir zir--tamyeez"
                  aria-disabled={taqreer.isPending || muarrif === ''}
                  onClick={() => {
                    if (!taqreer.isPending && muarrif !== '') {
                      taqreer.mutate();
                    }
                  }}
                >
                  {t(taqreer.isPending ? 'tashkhis.tawafuq.jari' : 'tashkhis.tawafuq.anshi', lugha)}
                </button>
              </div>
              {taqreer.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={taqreer.error}
                  lugha={lugha}
                  muarrif={muarrif}
                  aada={() => {
                    taqreer.mutate();
                  }}
                />
              ) : null}
              {taqreer.data !== undefined ? (
                <div className="tashkhis__natija" role="status">
                  <p>{t('tashkhis.tawafuq.tamma', lugha)}</p>
                  <p className="tashkhis__natija-masar mono-ltr">{taqreer.data}</p>
                </div>
              ) : null}
            </>
          )}
        </section>

        <section className="tashkhis__qism" aria-labelledby="tashkhis-unwan-huzma">
          <div className="tashkhis__raas-qism">
            <h2 id="tashkhis-unwan-huzma" className="tashkhis__unwan-qism">
              {t('tashkhis.huzma.unwan', lugha)}
            </h2>
            <p className="tashkhis__sharh-qism">{t('tashkhis.huzma.wasf', lugha)}</p>
          </div>
          <div className="tashkhis__adawat">
            <button
              type="button"
              className="zir zir--tamyeez"
              aria-disabled={huzma.isPending}
              onClick={() => {
                if (!huzma.isPending) {
                  huzma.mutate();
                }
              }}
            >
              {t(huzma.isPending ? 'tashkhis.huzma.jari' : 'tashkhis.huzma.ibni', lugha)}
            </button>
            <button
              type="button"
              className="zir"
              aria-disabled={iftah.isPending}
              onClick={() => {
                if (!iftah.isPending) {
                  iftah.mutate();
                }
              }}
            >
              {t('tashkhis.huzma.iftah', lugha)}
            </button>
          </div>
          {huzma.error !== null ? (
            <KutlatKhata
              unwan={t('luba.khata.amal', lugha)}
              khata={huzma.error}
              lugha={lugha}
              aada={() => {
                huzma.mutate();
              }}
            />
          ) : null}
          {iftah.error !== null ? (
            <KutlatKhata
              unwan={t('luba.khata.amal', lugha)}
              khata={iftah.error}
              lugha={lugha}
              aada={() => {
                iftah.mutate();
              }}
            />
          ) : null}
          {huzma.data !== undefined ? (
            <div className="tashkhis__natija" role="status">
              <p>
                {jam('tashkhis.huzma.tamma', lugha, huzma.data.adad_malaffat, munassiq, {
                  hajm: munassiq.hajm(huzma.data.hajm),
                })}
              </p>
              <p className="tashkhis__natija-masar mono-ltr">{huzma.data.masar}</p>
            </div>
          ) : null}
        </section>
      </div>
    </div>
  );
}
