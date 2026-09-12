import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { ansha } from '@/hayat/tanbihat';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t } from '@/lugha/lugha';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import { Zuhur } from '@/mukawwinat/zuhur';
import type {
  FahsAkhirHie,
  HalatMatjarHie,
  HasilatMaktaba,
  Idadat,
  Lugha,
  NizamArqam,
  SijillatHie,
  TashkhisHie,
} from '@/mustalahat/awamir';

import './tashkhis.css';

/** شاشة التشخيص — the log trail, the compatibility report, the maintainer bundle, and the last scan. */

const SUTUR: readonly number[] = [100, 400, 1000];

/** How many log files the placeholder stands in for: the rotation keeps about this many. */
const MALAFFAT_HAYKAL = 3;

/** How many launcher rows the scan placeholder stands in for. */
const MATAJIR_HAYKAL = 3;

/** How many facts the scan record leads with: kind, number, start, finish. */
const HUQUL_FAHS = 4;


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

const MIFTAH_HALAT_MATJAR: Readonly<Record<HalatMatjarHie, MiftahLugha>> = {
  tamma: 'tashkhis.fahs.hala.tamma',
  naqisa: 'tashkhis.fahs.hala.naqisa',
  ghayr_muthabbat: 'tashkhis.fahs.hala.ghayr_muthabbat',
};

/** Where an action was asked for, which decides where its failure has to be said. */
interface MasdarAmal {
  readonly minLawha: boolean;
}

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

/**
 * A failure said where the eye is. Three of this screen's actions can be fired
 * from the palette while the reader is looking at another section — or another
 * screen's worth of log — so the block that explains them is not guaranteed to
 * be in view when it appears.
 */
function anshaKhatar(khata: KhataJisr, lugha: Lugha): void {
  ansha({
    naw: 'khatar',
    nass: khata.nass(lugha) ?? t('faragh.jisr', lugha),
    ramz: khata.khata?.ramz ?? khata.amr,
  });
}

/**
 * The log section drawn empty: three file rows at the row's own height and the
 * tail box at the tail's own height, so the section does not grow by a box
 * when the answer lands.
 */
function HaykalSijillat(): JSX.Element {
  return (
    <div className="tashkhis__haykal zuhur-muakhkhar" aria-hidden="true">
      <ul className="tashkhis__malaffat">
        {Array.from({ length: MALAFFAT_HAYKAL }, (_, fihris) => (
          <li key={fihris} className="tashkhis__malaf">
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--ism" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--hajm" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--waqt" />
          </li>
        ))}
      </ul>
      <div className="tashkhis__dhayl tashkhis__dhayl--haykal" />
    </div>
  );
}

/** The report section's controls drawn empty: a select and a button, at band height. */
function HaykalAdawat(): JSX.Element {
  return (
    <div className="tashkhis__adawat tashkhis__haykal zuhur-muakhkhar" aria-hidden="true">
      <span className="tashkhis__haykal-satr tashkhis__haykal-satr--tasmiya" />
      <span className="tashkhis__haykal-haql" />
      <span className="tashkhis__haykal-haql tashkhis__haykal-haql--zir" />
    </div>
  );
}

/**
 * An answer box drawn empty while the report or the bundle is being written:
 * the sentence's line and the path's line, in the box the answer will take, so
 * the result lands where the reader is already looking.
 */
function HaykalNatija(): JSX.Element {
  return (
    <div className="tashkhis__natija tashkhis__natija--haykal zuhur-muakhkhar" aria-hidden="true">
      <span className="tashkhis__haykal-satr tashkhis__haykal-satr--jumla" />
      <span className="tashkhis__haykal-satr tashkhis__haykal-satr--masar" />
    </div>
  );
}

/** The scan record drawn empty: its facts on two columns and three launcher rows. */
function HaykalFahs(): JSX.Element {
  return (
    <div className="tashkhis__haykal zuhur-muakhkhar" aria-hidden="true">
      <div className="tashkhis__bayan">
        {Array.from({ length: HUQUL_FAHS }, (_, fihris) => (
          <div key={fihris} className="tashkhis__bayan-saff">
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--tasmiya" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--qeema" />
          </div>
        ))}
      </div>
      <ul className="tashkhis__matajir">
        {Array.from({ length: MATAJIR_HAYKAL }, (_, fihris) => (
          <li key={fihris} className="tashkhis__matjar">
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--ism" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--hala" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--hajm" />
            <span className="tashkhis__haykal-satr tashkhis__haykal-satr--waqt" />
          </li>
        ))}
      </ul>
    </div>
  );
}

interface KhasaisFahs {
  readonly bayanat: FahsAkhirHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

/**
 * The last scan, read back: what kind it was, when, what it saw, and every
 * warning the launchers left, each tied to the launcher that raised it. A
 * warning that may hide games is called out by name, because that is the one
 * fact a reader comes here for when a game is missing from the library.
 */
function FahsAkhir({ bayanat, lugha, munassiq }: KhasaisFahs): JSX.Element {
  const arabi = lugha === 'arabi';
  const tanbihat = bayanat.matajir.flatMap((matjar) =>
    matjar.tanbihat.map((tanbih, fihris) => ({
      miftah: `${matjar.muarrif}-${String(fihris)}`,
      ism: arabi ? matjar.ism_arabi : matjar.ism_injilizi,
      tanbih,
    })),
  );

  return (
    <>
      <dl className="tashkhis__bayan">
        <div className="tashkhis__bayan-saff">
          <dt className="tashkhis__bayan-tasmiya">{t('tashkhis.fahs.raqm', lugha)}</dt>
          <dd className="tashkhis__bayan-qeema">{munassiq.raqm(bayanat.raqm)}</dd>
        </div>
        <div className="tashkhis__bayan-saff">
          <dt className="tashkhis__bayan-tasmiya">{t('tashkhis.fahs.naw', lugha)}</dt>
          <dd className="tashkhis__bayan-qeema">
            {t(bayanat.kamil ? 'tashkhis.fahs.kamil' : 'tashkhis.fahs.juzi', lugha)}
          </dd>
        </div>
        <div className="tashkhis__bayan-saff">
          <dt className="tashkhis__bayan-tasmiya">{t('tashkhis.fahs.bidaya', lugha)}</dt>
          <dd className="tashkhis__bayan-qeema mono-ltr">{lahzaQaseera(bayanat.bidaya)}</dd>
        </div>
        <div className="tashkhis__bayan-saff">
          <dt className="tashkhis__bayan-tasmiya">{t('tashkhis.fahs.nihaya', lugha)}</dt>
          {bayanat.nihaya === null ? (
            <dd className="tashkhis__bayan-qeema tashkhis__bayan-qeema--tanbeeh">
              {t('tashkhis.fahs.lam_yantahi', lugha)}
            </dd>
          ) : (
            <dd className="tashkhis__bayan-qeema mono-ltr">{lahzaQaseera(bayanat.nihaya)}</dd>
          )}
        </div>
      </dl>
      <p className="tashkhis__jumla">
        {jam('tashkhis.fahs.alaab', lugha, bayanat.adad_alaab, munassiq)}
      </p>
      {bayanat.matajir.length === 0 ? null : (
        <>
          <h3 className="tashkhis__unwan-far">{t('tashkhis.fahs.manassat', lugha)}</h3>
          <ul className="tashkhis__matajir">
            {bayanat.matajir.map((matjar) => (
              <li
                key={matjar.muarrif}
                className={`tashkhis__matjar tashkhis__matjar--${matjar.hala}`}
              >
                <span className="tashkhis__matjar-ism">
                  {arabi ? matjar.ism_arabi : matjar.ism_injilizi}
                </span>
                <span className="tashkhis__matjar-hala">
                  {t(MIFTAH_HALAT_MATJAR[matjar.hala], lugha)}
                </span>
                <span className="tashkhis__matjar-adad">{munassiq.raqm(matjar.adad_alaab)}</span>
                <span className="tashkhis__matjar-mudda">
                  {t('tashkhis.fahs.muddat_ms', lugha, { adad: munassiq.raqm(matjar.muddat_ms) })}
                </span>
                <span className="tashkhis__matjar-wasf">
                  {arabi ? matjar.wasf_arabi : matjar.wasf_injilizi}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
      <h3 className="tashkhis__unwan-far">{t('tashkhis.fahs.tanbihat', lugha)}</h3>
      {tanbihat.length === 0 ? (
        <p className="tashkhis__jumla">{t('tashkhis.fahs.la_tanbihat', lugha)}</p>
      ) : (
        <ul className="tashkhis__tanbihat">
          {tanbihat.map(({ miftah, ism, tanbih }) => (
            <li key={miftah} className="tashkhis__tanbih">
              <span className="tashkhis__tanbih-raas">
                <span className="tashkhis__tanbih-matjar">{ism}</span>
                <span className="tashkhis__tanbih-mawdi mono-ltr">{tanbih.mawdi}</span>
              </span>
              <span className="tashkhis__tanbih-sabab" dir="auto">
                {tanbih.sabab}
              </span>
              {tanbih.yukhfi_alaab === true ? (
                <span className="tashkhis__tanbih-yukhfi">{t('tashkhis.fahs.yukhfi', lugha)}</span>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

export function Tashkhis(): JSX.Element {
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

  const fahs = useQuery<FahsAkhirHie | null, KhataJisr>({
    queryKey: mafatih.fahs_akhir,
    queryFn: () => nadi('fahs_akhir'),
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

  // The bundle and the folder are the two actions the palette can fire from
  // anywhere. Their result blocks are in this screen's last section; a notice
  // says what happened wherever the reader is. A failure is said twice only
  // when the block can be out of view — from the palette — because a failure
  // under the pointer already has its block.
  const huzma = useMutation<TashkhisHie, KhataJisr, MasdarAmal>({
    mutationFn: () => nadi('huzmat_tashkhis'),
    onSuccess: (natija) => {
      ansha({
        naw: 'najah',
        nass: t('tashkhis.huzma.tamma_tanbih', lugha),
        tafsil: natija.masar,
      });
    },
    onError: (khata, { minLawha }) => {
      if (minLawha) {
        anshaKhatar(khata, lugha);
      }
    },
  });

  const iftah = useMutation<boolean, KhataJisr, MasdarAmal>({
    mutationFn: () => nadi('iftah_tashkhis'),
    onSuccess: () => {
      ansha({ naw: 'najah', nass: t('tashkhis.huzma.futiha', lugha) });
    },
    onError: (khata, { minLawha }) => {
      if (minLawha) {
        anshaKhatar(khata, lugha);
      }
    },
  });

  // A refresh that returns the same lines changes nothing on screen, so the
  // notice is the only thing that says it ran.
  const alaTahdith = (): void => {
    void sijillat.refetch().then((natija) => {
      if (natija.error !== null) {
        anshaKhatar(natija.error, lugha);
        return;
      }
      ansha({ naw: 'najah', nass: t('tashkhis.sijillat.tamma_tahdith', lugha) });
    });
  };

  // The palette registers once per id set, so its action reads through a ref
  // that always holds the current handler and the current language.
  const marjiTahdith = useRef(alaTahdith);
  useEffect(() => {
    marjiTahdith.current = alaTahdith;
  });

  useSajjilAwamir([
    {
      muarrif: 'tashkhis.ibni_huzma',
      unwan: t('tashkhis.awamir.ibni', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        huzma.mutate({ minLawha: true });
      },
    },
    {
      muarrif: 'tashkhis.iftah_mujallad',
      unwan: t('tashkhis.awamir.iftah', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        iftah.mutate({ minLawha: true });
      },
    },
    {
      muarrif: 'tashkhis.hadith_sijillat',
      unwan: t('tashkhis.awamir.hadith', lugha),
      majal: t('shasha.tashkhis', lugha),
      nafidh: () => {
        marjiTahdith.current();
      },
    },
  ]);

  // The refresh is the toolbar's action and the action inside both of the log
  // section's empty states, so it is built once and placed in all three.
  const zirTahdith = (
    <button type="button" className="zir" aria-busy={sijillat.isFetching} onClick={alaTahdith}>
      {t('tashkhis.sijillat.hadith', lugha)}
    </button>
  );

  const wajhSijillat = sijillat.isPending
    ? 'tahmil'
    : sijillat.error !== null
      ? 'khata'
      : sijillat.data === undefined || sijillat.data.malaffat.length === 0
        ? 'farigh'
        : 'jahiz';

  const wajhMaktaba = maktaba.isPending
    ? 'tahmil'
    : maktaba.error !== null
      ? 'khata'
      : alaab === undefined || alaab.length === 0
        ? 'farigh'
        : 'jahiz';

  const wajhFahs = fahs.isPending
    ? 'tahmil'
    : fahs.error !== null
      ? 'khata'
      : fahs.data === null || fahs.data === undefined
        ? 'farigh'
        : 'jahiz';

  // An action's answer has four states of its own — nothing yet, being
  // written, written, refused — and each replaces the last through the same
  // switch the sections use, so a result never pops in under a button.
  const wajhTaqreer = taqreer.isPending
    ? 'jari'
    : taqreer.error !== null
      ? 'khata'
      : taqreer.data !== undefined
        ? 'natija'
        : 'la_shay';

  const wajhHuzma = huzma.isPending
    ? 'jari'
    : huzma.error !== null
      ? 'khata'
      : huzma.data !== undefined
        ? 'natija'
        : 'la_shay';

  return (
    <div className="tashkhis">
      <RaasShasha
        rujoo={{ ila: 'maktaba' }}
        nassRujoo={t('tashkhis.raji', lugha)}
        unwan={t('shasha.tashkhis', lugha)}
      />

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
          <Mashhad miftah={wajhSijillat} className="tashkhis__mashhad">
            {wajhSijillat === 'tahmil' ? (
              <HaykalSijillat />
            ) : sijillat.error !== null ? (
              <KutlatKhata
                unwan={t('tashkhis.sijillat.taadhur', lugha)}
                khata={sijillat.error}
                lugha={lugha}
                aada={() => {
                  void sijillat.refetch();
                }}
              />
            ) : sijillat.data === undefined || wajhSijillat === 'farigh' ? (
              <HalatFarigha unwan={t('tashkhis.sijillat.la_malaffat', lugha)}>
                {zirTahdith}
              </HalatFarigha>
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
                  <HalatFarigha unwan={t('tashkhis.sijillat.la_akhir', lugha)}>
                    {zirTahdith}
                  </HalatFarigha>
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
          </Mashhad>
        </section>

        <section className="tashkhis__qism" aria-labelledby="tashkhis-unwan-tawafuq">
          <div className="tashkhis__raas-qism">
            <h2 id="tashkhis-unwan-tawafuq" className="tashkhis__unwan-qism">
              {t('tashkhis.tawafuq.unwan', lugha)}
            </h2>
          </div>
          <Mashhad miftah={wajhMaktaba} className="tashkhis__mashhad">
            {wajhMaktaba === 'tahmil' ? (
              <HaykalAdawat />
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
              <HalatFarigha unwan={t('tashkhis.tawafuq.la_alab', lugha)}>
                <Link to="/" className="zir">
                  {t('tashkhis.raji', lugha)}
                </Link>
              </HalatFarigha>
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
                    aria-disabled={muarrif === ''}
                    aria-busy={taqreer.isPending}
                    onClick={() => {
                      if (!taqreer.isPending && muarrif !== '') {
                        taqreer.mutate();
                      }
                    }}
                  >
                    {t(taqreer.isPending ? 'tashkhis.tawafuq.jari' : 'tashkhis.tawafuq.anshi', lugha)}
                  </button>
                </div>
                <Mashhad miftah={wajhTaqreer} className="tashkhis__natija-mashhad">
                  {taqreer.isPending ? (
                    <HaykalNatija />
                  ) : taqreer.error !== null ? (
                    <KutlatKhata
                      unwan={t('luba.khata.amal', lugha)}
                      khata={taqreer.error}
                      lugha={lugha}
                      muarrif={muarrif}
                      aada={() => {
                        taqreer.mutate();
                      }}
                    />
                  ) : taqreer.data !== undefined ? (
                    <div className="tashkhis__natija" role="status">
                      <p>{t('tashkhis.tawafuq.tamma', lugha)}</p>
                      <p className="tashkhis__natija-masar mono-ltr">{taqreer.data}</p>
                    </div>
                  ) : null}
                </Mashhad>
              </>
            )}
          </Mashhad>
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
              aria-busy={huzma.isPending}
              onClick={() => {
                if (!huzma.isPending) {
                  huzma.mutate({ minLawha: false });
                }
              }}
            >
              {t(huzma.isPending ? 'tashkhis.huzma.jari' : 'tashkhis.huzma.ibni', lugha)}
            </button>
            <button
              type="button"
              className="zir"
              aria-busy={iftah.isPending}
              onClick={() => {
                if (!iftah.isPending) {
                  iftah.mutate({ minLawha: false });
                }
              }}
            >
              {t('tashkhis.huzma.iftah', lugha)}
            </button>
          </div>
          <Mashhad miftah={wajhHuzma} className="tashkhis__natija-mashhad">
            {huzma.isPending ? (
              <HaykalNatija />
            ) : huzma.error !== null ? (
              <KutlatKhata
                unwan={t('luba.khata.amal', lugha)}
                khata={huzma.error}
                lugha={lugha}
                aada={() => {
                  huzma.mutate({ minLawha: false });
                }}
              />
            ) : huzma.data !== undefined ? (
              <div className="tashkhis__natija" role="status">
                <p>
                  {jam('tashkhis.huzma.tamma', lugha, huzma.data.adad_malaffat, munassiq, {
                    hajm: munassiq.hajm(huzma.data.hajm),
                  })}
                </p>
                <p className="tashkhis__natija-masar mono-ltr">{huzma.data.masar}</p>
              </div>
            ) : null}
          </Mashhad>
          <Zuhur maftuh={iftah.error !== null} asl="mahall">
            {iftah.error === null ? null : (
              <KutlatKhata
                unwan={t('luba.khata.amal', lugha)}
                khata={iftah.error}
                lugha={lugha}
                aada={() => {
                  iftah.mutate({ minLawha: false });
                }}
              />
            )}
          </Zuhur>
        </section>

        <section className="tashkhis__qism" aria-labelledby="tashkhis-unwan-fahs">
          <div className="tashkhis__raas-qism">
            <h2 id="tashkhis-unwan-fahs" className="tashkhis__unwan-qism">
              {t('tashkhis.fahs.unwan', lugha)}
            </h2>
            <p className="tashkhis__sharh-qism">{t('tashkhis.fahs.sharh', lugha)}</p>
          </div>
          <Mashhad miftah={wajhFahs} className="tashkhis__mashhad">
            {wajhFahs === 'tahmil' ? (
              <HaykalFahs />
            ) : fahs.error !== null ? (
              <KutlatKhata
                unwan={t('tashkhis.fahs.taadhur', lugha)}
                khata={fahs.error}
                lugha={lugha}
                aada={() => {
                  void fahs.refetch();
                }}
              />
            ) : fahs.data === null || fahs.data === undefined ? (
              <HalatFarigha unwan={t('tashkhis.fahs.la_fahs', lugha)}>
                <Link to="/" className="zir">
                  {t('tashkhis.raji', lugha)}
                </Link>
              </HalatFarigha>
            ) : (
              <FahsAkhir bayanat={fahs.data} lugha={lugha} munassiq={munassiq} />
            )}
          </Mashhad>
        </section>
      </div>
    </div>
  );
}
