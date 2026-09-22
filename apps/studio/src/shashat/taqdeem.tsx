import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { Link, getRouteApi } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
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
  Idadat,
  IrsalHie,
  JalsaHie,
  Lugha,
  MusahamaHie,
  MusawwadaHie,
  NizamArqam,
  QiyasTajawuzHie,
  SatrFahsHie,
  TalabJihazHie,
} from '@/mustalahat/awamir';

import './taqdeem.css';

/** شاشة التقديم — the pre-flight gate, the metadata, the handoff, and my trail. */

const wajihat = getRouteApi('/taqdeem/$muarrif');

const RUKHAS: readonly string[] = ['cc0', 'cc_by', 'cc_by_sa', 'milkiya_khassa', 'ukhra'];
const TURUQ: readonly string[] = ['bashariya_kamila', 'aaliya_thum_bashariya', 'aaliya_faqat'];

const MIFTAH_RUKHSA: Readonly<Record<string, MiftahLugha>> = {
  cc0: 'taqdeem.rukhsa.cc0',
  cc_by: 'taqdeem.rukhsa.cc_by',
  cc_by_sa: 'taqdeem.rukhsa.cc_by_sa',
  milkiya_khassa: 'taqdeem.rukhsa.milkiya_khassa',
  ukhra: 'taqdeem.rukhsa.ukhra',
};

const MIFTAH_TAREEQA: Readonly<Record<string, MiftahLugha>> = {
  bashariya_kamila: 'taqdeem.tareeqa.bashariya_kamila',
  aaliya_thum_bashariya: 'taqdeem.tareeqa.aaliya_thum_bashariya',
  aaliya_faqat: 'taqdeem.tareeqa.aaliya_faqat',
};

/** The `sallim_taqdeem` refusal that means: no device authorization ran yet. */
const RAMZ_TAWTHIQ_NAQIS = 'TAARIB-E-9076';

/** The three stages the spine holds, so the placeholder holds three too. */
const KHUTUWAT_HAYKAL = 3;

/**
 * The wizard drawn empty: the spine with its three markers, and the first
 * section's heading and fields at the heights the real ones take, so the form
 * lands where the placeholder stood rather than a sentence's height under it.
 */
function HaykalTaqdeem(): JSX.Element {
  return (
    <div className="taqdeem__lawh zuhur-muakhkhar" aria-hidden="true">
      <aside className="taqdeem__janib">
        <ol className="taqdeem__masar">
          {Array.from({ length: KHUTUWAT_HAYKAL }, (_, fihris) => (
            <li key={fihris} className="taqdeem__khatwa">
              <span className="taqdeem__khatwa-ramz" />
              <span className="taqdeem__haykal-satr taqdeem__haykal-satr--khatwa" />
            </li>
          ))}
        </ol>
      </aside>
      <div className="taqdeem__amud">
        <section className="taqdeem__qism">
          <div className="taqdeem__unwan-qism">
            <span className="taqdeem__haykal-satr taqdeem__haykal-satr--unwan" />
          </div>
          <div className="taqdeem__namudhaj">
            <div className="taqdeem__haql-majmua">
              <span className="taqdeem__haykal-satr taqdeem__haykal-satr--tasmiya" />
              <span className="taqdeem__haykal-haql" />
            </div>
            <div className="taqdeem__haql-majmua">
              <span className="taqdeem__haykal-satr taqdeem__haykal-satr--tasmiya" />
              <span className="taqdeem__haykal-haql taqdeem__haykal-haql--nassi" />
            </div>
            <div className="taqdeem__haql-majmua">
              <span className="taqdeem__haykal-satr taqdeem__haykal-satr--tasmiya" />
              <span className="taqdeem__haykal-haql taqdeem__haykal-haql--nassi" />
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

/**
 * The gate's verdicts are drawn as three distinct shapes, not three hues.
 *
 * A checklist that separates pass from fail by colour alone is unreadable to
 * the people most likely to be submitting a translation on a laptop screen in
 * daylight, and this is the last screen before a package is published. The
 * stroke weight is the product's single icon weight, inherited from the base
 * layer rather than restated here.
 */
function RamzSah(): JSX.Element {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeLinecap="round">
      <path d="M3.5 8.4 6.4 11.3 12.5 4.7" />
    </svg>
  );
}

function RamzRafd(): JSX.Element {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeLinecap="round">
      <path d="M4.4 4.4 11.6 11.6M11.6 4.4 4.4 11.6" />
    </svg>
  );
}

function RamzIntizar(): JSX.Element {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeLinecap="round">
      <path d="M8 3.4v5.4M8 12.3v0.01" />
    </svg>
  );
}

/** A check nobody could run: a question, drawn as neither a pass nor a failure. */
function RamzMajhul(): JSX.Element {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeLinecap="round">
      <path d="M5.7 6.2a2.3 2.3 0 1 1 3.3 2.1c-.7.4-1 .9-1 1.6M8 12.3v0.01" />
    </svg>
  );
}

/**
 * The gate's four states as the row is coloured, plus one this screen draws
 * for a pass the measurement does not back: the overflow check over a project
 * in which nothing — or only part — was measured. It is not a warning the
 * contributor can acknowledge and not a failure that blocks; it is a verdict
 * that was never reached, and it must not wear the green check.
 */
type NawBand = 'najah' | 'khatar' | 'tanbeeh' | 'majhul';

function nawBand(hala: string): NawBand {
  switch (hala) {
    case 'ijtaz':
      return 'najah';
    case 'rasab':
      return 'khatar';
    case 'ghayr_maqis':
    case 'maqis_juzi':
      return 'majhul';
    default:
      return 'tanbeeh';
  }
}

/**
 * The state's label. The four the gate mints keep their own keys; anything
 * else is worded by the backend, which sends both languages, so a state this
 * screen has never heard of is shown as what it is rather than as a guess.
 */
function wasmHala(satr: SatrFahsHie, lugha: Lugha): string {
  switch (satr.hala) {
    case 'ijtaz':
      return t('taqdeem.fahs.ijtaz', lugha);
    case 'rasab':
      return t('taqdeem.fahs.rasab', lugha);
    case 'muqarr':
      return t('taqdeem.fahs.muqarr', lugha);
    case 'yantazir_iqrar':
      return t('taqdeem.fahs.yantazir', lugha);
    default:
      return lugha === 'arabi' ? satr.hala_arabi : satr.hala_injilizi;
  }
}

interface KhasaisQiyas {
  readonly qiyas: QiyasTajawuzHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

/** Why the overflow check could not be a verdict: every cause, with what resolves it. */
function AsbabQiyas({ qiyas, lugha, munassiq }: KhasaisQiyas): JSX.Element | null {
  if (qiyas.hala === 'kamil' || qiyas.asbab.length === 0) {
    return null;
  }
  return (
    <ul className="taqdeem__asbab-qiyas">
      {qiyas.asbab.map((sabab) => {
        const ilaj = lugha === 'arabi' ? sabab.ilaj_arabi : sabab.ilaj_injilizi;
        return (
          <li key={sabab.miftah} className="taqdeem__sabab-qiyas">
            <span className="taqdeem__sabab-adad">{munassiq.raqm(sabab.adad)}</span>
            <span className="taqdeem__sabab-jism">
              <span>{lugha === 'arabi' ? sabab.wasf_arabi : sabab.wasf_injilizi}</span>
              {ilaj !== '' ? <span className="taqdeem__sabab-ilaj">{ilaj}</span> : null}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

/** Where one wizard stage stands relative to the contributor's progress. */
type HalatKhatwa = 'tamma' | 'haliya' | 'qadima';

interface Khatwa {
  readonly miftah: MiftahLugha;
  readonly hala: HalatKhatwa;
}

interface KhasaisMasar {
  readonly khutuwat: readonly Khatwa[];
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

/** The spine: three stages, the one being worked, and everything behind it. */
function MasarKhutuwat({ khutuwat, lugha, munassiq }: KhasaisMasar): JSX.Element {
  return (
    <ol className="taqdeem__masar">
      {khutuwat.map((khatwa, fihris) => (
        <li
          key={khatwa.miftah}
          className={`taqdeem__khatwa taqdeem__khatwa--${khatwa.hala}`}
          aria-current={khatwa.hala === 'haliya' ? 'step' : undefined}
        >
          <span className="taqdeem__khatwa-ramz" aria-hidden="true">
            {khatwa.hala === 'tamma' ? <RamzSah /> : munassiq.raqm(fihris + 1)}
          </span>
          <span className="taqdeem__khatwa-ism">{t(khatwa.miftah, lugha)}</span>
        </li>
      ))}
    </ol>
  );
}

interface KhasaisFahs {
  readonly satr: SatrFahsHie;
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly yajri: boolean;
  /** Whether this row's own acknowledgement is the one in flight. */
  readonly mashghul: boolean;
  /** Whether the submission is still the contributor's to change. */
  readonly qabilLilTahreer: boolean;
  readonly alaIqrar: (tahdheer: string, qeema: boolean) => void;
}

function SaffFahs({
  satr,
  muarrif,
  lugha,
  munassiq,
  yajri,
  mashghul,
  qabilLilTahreer,
  alaIqrar,
}: KhasaisFahs): JSX.Element {
  const naw = nawBand(satr.hala);
  return (
    <li className={`taqdeem__band taqdeem__band--${naw}`}>
      <span className="taqdeem__band-ramz" aria-hidden="true">
        {satr.hala === 'rasab' ? (
          <RamzRafd />
        ) : satr.hala === 'yantazir_iqrar' ? (
          <RamzIntizar />
        ) : naw === 'majhul' ? (
          <RamzMajhul />
        ) : (
          <RamzSah />
        )}
      </span>
      <div className="taqdeem__band-jism">
        <p className="taqdeem__band-raas">
          <span className="taqdeem__band-unwan">
            {lugha === 'arabi' ? satr.wasf_arabi : satr.wasf_injilizi}
          </span>
          <span className="taqdeem__band-hala">{wasmHala(satr, lugha)}</span>
        </p>
        <p className="taqdeem__band-tafsil">
          {lugha === 'arabi' ? satr.tafsil_arabi : satr.tafsil_injilizi}
        </p>
        {satr.adad > 0 ? (
          <p className="taqdeem__band-nusus">
            {jam('taqdeem.fahs.nusus', lugha, satr.adad, munassiq)}
            {' — '}
            <Link to="/warsha/$muarrif" params={{ muarrif }} className="taqdeem__rabt">
              {t('taqdeem.fahs.iftah_warsha', lugha)}
            </Link>
          </p>
        ) : null}
        {/* A re-prepare can turn a warning into a pass or a pass into a
            warning; the acknowledgement arrives and leaves with the state
            instead of blinking into a row that was already being read. It
            leaves for good once the submission is out of the contributor's
            hands: the only thing an acknowledgement unlocks is the submit
            button, and that button is gone by then. */}
        <Zuhur
          maftuh={
            qabilLilTahreer &&
            !satr.hasim &&
            (satr.hala === 'yantazir_iqrar' || satr.hala === 'muqarr')
          }
          className="taqdeem__iqrar-hawiya"
        >
          <label className="taqdeem__iqrar" aria-busy={mashghul}>
            <input
              type="checkbox"
              checked={satr.hala === 'muqarr'}
              disabled={yajri}
              onChange={(hadath) => {
                alaIqrar(satr.band, hadath.target.checked);
              }}
            />
            {t('taqdeem.fahs.aqirr', lugha)}
          </label>
        </Zuhur>
      </div>
    </li>
  );
}

export function Taqdeem(): JSX.Element {
  const { muarrif } = wajihat.useParams();
  const makhzan = useQueryClient();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const jalsa = useQuery<JalsaHie, KhataJisr>({
    queryKey: mafatih.jalsa,
    queryFn: () => nadi('jalsati'),
  });

  const musawwada = useQuery<MusawwadaHie | null, KhataJisr>({
    queryKey: mafatih.musawwada(muarrif),
    queryFn: () => nadi('musawwadat_luba', { muarrif }),
  });

  const musahamat = useQuery<MusahamaHie[], KhataJisr>({
    queryKey: mafatih.musahamat,
    queryFn: () => nadi('musahamati'),
  });

  const [unwan, setUnwan] = useState('');
  const [sharh, setSharh] = useState('');
  const [taghyeerat, setTaghyeerat] = useState('');
  const [rukhsa, setRukhsa] = useState('cc_by_sa');
  const [rukhsaIsm, setRukhsaIsm] = useState('');
  const [tareeqa, setTareeqa] = useState('bashariya_kamila');
  const [taakidIrsal, setTaakidIrsal] = useState(false);
  /** The control that opened the confirmation, so closing it hands focus back. */
  const fatihTaakid = useRef<HTMLElement | null>(null);

  const iftahTaakid = (): void => {
    fatihTaakid.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setTaakidIrsal(true);
  };

  const aghliqTaakid = (): void => {
    setTaakidIrsal(false);
    fatihTaakid.current?.focus();
    fatihTaakid.current = null;
  };

  useEffect(() => {
    if (!taakidIrsal) {
      return undefined;
    }
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      if (hadath.key === 'Escape') {
        setTaakidIrsal(false);
        fatihTaakid.current?.focus();
        fatihTaakid.current = null;
      }
    };
    document.addEventListener('keydown', alaMiftah);
    return () => {
      document.removeEventListener('keydown', alaMiftah);
    };
  }, [taakidIrsal]);

  useEffect(() => {
    const bayanat = musawwada.data;
    if (bayanat !== null && bayanat !== undefined) {
      setUnwan(bayanat.unwan);
      setSharh(bayanat.sharh);
      setTaghyeerat(bayanat.taghyeerat);
      setRukhsa(bayanat.rukhsa);
    }
  }, [musawwada.data]);

  const jahhiz = useMutation<MusawwadaHie, KhataJisr, void>({
    mutationFn: () =>
      nadi('jahhiz_taqdeem', {
        muarrif,
        unwan,
        sharh,
        taghyeerat,
        rukhsa,
        rukhsaIsm: rukhsa === 'ukhra' ? rukhsaIsm : null,
        tareeqa,
      }),
    // A rebuild from the form half-way down the page changes a hash at the
    // top of it; the notice carries the new hash to where the eye is.
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.musawwada(muarrif), bayanat);
      void makhzan.invalidateQueries({ queryKey: mafatih.musahamat });
      ansha({
        naw: 'najah',
        nass: t('taqdeem.tanbih.juhhizat', lugha),
        tafsil: bayanat.basmat_huzma,
      });
    },
  });

  const iqrar = useMutation<MusawwadaHie, KhataJisr, { tahdheer: string; qeema: boolean }>({
    mutationFn: ({ tahdheer, qeema }) =>
      nadi('aqirr_tahdheer', { muarrif, tahdheer, qeema }),
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.musawwada(muarrif), bayanat);
    },
  });

  const irsal = useMutation<IrsalHie, KhataJisr, void>({
    mutationFn: () => nadi('sallim_taqdeem', { muarrif }),
    onSuccess: (natija) => {
      setTaakidIrsal(false);
      void makhzan.invalidateQueries({ queryKey: mafatih.musawwada(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.musahamat });
      ansha({
        naw: 'najah',
        nass: t('taqdeem.tanbih.sullimat', lugha),
        tafsil: t('taqdeem.hala.murajaa', lugha, { raqm: munassiq.raqm(natija.murajaa) }),
      });
    },
  });

  const tawthiq = useMutation<TalabJihazHie, KhataJisr, void>({
    mutationFn: () => nadi('abda_tawthiq_taqdeem'),
  });

  const alaTawthiq = (): void => {
    if (!tawthiq.isPending) {
      tawthiq.mutate();
    }
  };

  // A refusal that names missing authorization starts the device flow itself;
  // the button beside تسليم covers starting it before the first attempt.
  const abdaTawthiq = tawthiq.mutate;
  const tawthiqSakin = tawthiq.isIdle;
  useEffect(() => {
    if (irsal.error?.khata?.ramz === RAMZ_TAWTHIQ_NAQIS && tawthiqSakin) {
      abdaTawthiq();
    }
  }, [irsal.error, tawthiqSakin, abdaTawthiq]);

  // The palette registers once per id set, so its action reads the handler
  // through a ref that always holds the current closure.
  const afalHaliya = useRef({ tawthiq: alaTawthiq });
  useEffect(() => {
    afalHaliya.current = { tawthiq: alaTawthiq };
  });

  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    if (idadat.data === undefined) {
      return [];
    }
    return [
      {
        muarrif: `taqdeem.${muarrif}.tawthiq`,
        unwan: t('taqdeem.lawha.tawthiq', lugha),
        majal: t('shasha.taqdeem', lugha),
        nafidh: () => {
          afalHaliya.current.tawthiq();
        },
      },
    ];
  }, [idadat.data, lugha, muarrif]);
  useSajjilAwamir(awamirShasha);

  const bayanat = musawwada.data ?? null;
  const yuhammil = idadat.isPending || musawwada.isPending;

  const jahiza = bayanat?.qaima.jahiza === true;
  const ursilat = irsal.data !== undefined || (bayanat !== null && !bayanat.qabila_lil_tahreer);
  const khutuwat = useMemo<readonly Khatwa[]>(
    () => [
      { miftah: 'taqdeem.bayanat.unwan', hala: bayanat === null ? 'haliya' : 'tamma' },
      {
        miftah: 'taqdeem.fahs.unwan',
        hala: bayanat === null ? 'qadima' : jahiza ? 'tamma' : 'haliya',
      },
      {
        miftah: 'taqdeem.irsal.sallim',
        hala: ursilat ? 'tamma' : jahiza ? 'haliya' : 'qadima',
      },
    ],
    [bayanat, jahiza, ursilat],
  );

  // The body moves only when the contributor's stage does. A re-prepare, an
  // acknowledgement or a refetch changes data inside a stage, never this key,
  // so nothing re-animates on a data update.
  const khatwaHaliya = bayanat === null ? 'bayanat' : ursilat ? 'ursilat' : 'fahs';

  const wajh = yuhammil ? 'tahmil' : musawwada.error !== null ? 'khata' : 'jahiz';

  return (
    <div className="taqdeem">
      <RaasShasha
        rujoo={{ ila: 'luba', muarrif }}
        nassRujoo={t('taqdeem.raji', lugha)}
        unwan={t('shasha.taqdeem', lugha)}
        mawdu={bayanat?.ism_luba ?? null}
        tafasil={
          jalsa.data === undefined ? null : (
            <span className="mono-ltr">{jalsa.data.musahim}</span>
          )
        }
      />

      <Mashhad miftah={wajh} className="taqdeem__jism">
        {wajh === 'tahmil' ? (
          <HaykalTaqdeem />
        ) : musawwada.error !== null ? (
          <KutlatKhata
            unwan={t('taqdeem.khata.tahmil', lugha)}
            khata={musawwada.error}
            lugha={lugha}
            muarrif={muarrif}
          >
            <button
              type="button"
              className="zir"
              onClick={() => {
                void musawwada.refetch();
              }}
            >
              {t('amm.iaada', lugha)}
            </button>
          </KutlatKhata>
        ) : (
          <div className="taqdeem__lawh">
            <aside className="taqdeem__janib">
              <MasarKhutuwat khutuwat={khutuwat} lugha={lugha} munassiq={munassiq} />
              {bayanat !== null ? (
                <p className="taqdeem__mawqif">
                  <span className="taqdeem__mawqif-wasm">
                    {lugha === 'arabi' ? bayanat.hala_arabi : bayanat.hala_injilizi}
                  </span>
                  <span className="taqdeem__mawqif-murajaa">
                    {t('taqdeem.hala.murajaa', lugha, {
                      raqm: munassiq.raqm(bayanat.murajaa),
                    })}
                  </span>
                </p>
              ) : null}
            </aside>

            <div className="taqdeem__amud">
              <Mashhad miftah={khatwaHaliya} className="taqdeem__khatwa-jism">
              {bayanat !== null ? (
                <section className="taqdeem__qism" aria-labelledby="taqdeem-unwan-hala">
                  <h2 id="taqdeem-unwan-hala" className="taqdeem__unwan-qism">
                    {t('taqdeem.hala.unwan', lugha)}
                  </h2>
                  <dl className="taqdeem__jadwal">
                    <div className="taqdeem__saff-jadwal">
                      <dt>{t('taqdeem.hala.unwan_ruqaa', lugha)}</dt>
                      <dd>{bayanat.unwan}</dd>
                    </div>
                    <div className="taqdeem__saff-jadwal">
                      <dt>{t('taqdeem.hala.hajm', lugha)}</dt>
                      <dd className="taqdeem__qeema-raqmiya">
                        {munassiq.hajm(bayanat.hajm_huzma)}
                      </dd>
                    </div>
                    <div className="taqdeem__saff-jadwal">
                      <dt>{t('taqdeem.hala.basma', lugha)}</dt>
                      <dd className="mono-ltr">{bayanat.basmat_huzma}</dd>
                    </div>
                    <div className="taqdeem__saff-jadwal">
                      <dt>{t('taqdeem.hala.nusus', lugha)}</dt>
                      <dd>
                        {t('taqdeem.hala.nusus_qeema', lugha, {
                          nusus: jam(
                            'taqdeem.hala.adad_nusus',
                            lugha,
                            bayanat.adad_nusus,
                            munassiq,
                          ),
                          muakkada: jam(
                            'taqdeem.hala.adad_muakkada',
                            lugha,
                            bayanat.muakkada,
                            munassiq,
                          ),
                          nisba: munassiq.nisba(kasr(bayanat.taghtiya_nisba)),
                        })}
                      </dd>
                    </div>
                    <div className="taqdeem__saff-jadwal">
                      <dt>{t('taqdeem.hala.tareeqa', lugha)}</dt>
                      <dd>{bayanat.tareeqa_arabi}</dd>
                    </div>
                  </dl>
                  {bayanat.tareekh.length > 0 ? (
                    <ol className="taqdeem__tareekh">
                      {bayanat.tareekh.map((intiqal, fihris) => (
                        <li key={`${intiqal.ila}-${String(fihris)}`}>
                          <span className="taqdeem__waqt mono-ltr">{intiqal.waqt}</span>
                          {/* The worded state, not its key: this is a list of
                              things that happened to the contributor's own
                              work, and `mawafaq_yunshar` is not one of them. */}
                          <span className="taqdeem__intiqal">
                            {lugha === 'arabi' ? intiqal.ila_arabi : intiqal.ila_injilizi}
                          </span>
                        </li>
                      ))}
                    </ol>
                  ) : null}
                </section>
              ) : null}

              {bayanat === null || bayanat.qabila_lil_tahreer ? (
                <section className="taqdeem__qism" aria-labelledby="taqdeem-unwan-bayanat">
                  <h2 id="taqdeem-unwan-bayanat" className="taqdeem__unwan-qism">
                    {t(
                      bayanat === null ? 'taqdeem.bayanat.unwan' : 'taqdeem.bayanat.iaada',
                      lugha,
                    )}
                  </h2>
                  <div className="taqdeem__namudhaj">
                    <div className="taqdeem__haql-majmua">
                      <label className="taqdeem__tasmiya" htmlFor="taqdeem-unwan-haql">
                        {t('taqdeem.bayanat.unwan_ruqaa', lugha)}
                      </label>
                      <input
                        id="taqdeem-unwan-haql"
                        className="taqdeem__haql"
                        dir="auto"
                        value={unwan}
                        onChange={(hadath) => {
                          setUnwan(hadath.target.value);
                        }}
                      />
                    </div>
                    <div className="taqdeem__haql-majmua">
                      <label className="taqdeem__tasmiya" htmlFor="taqdeem-sharh">
                        {t('taqdeem.bayanat.sharh', lugha)}
                      </label>
                      <textarea
                        id="taqdeem-sharh"
                        className="taqdeem__haql taqdeem__nassi"
                        dir="auto"
                        value={sharh}
                        onChange={(hadath) => {
                          setSharh(hadath.target.value);
                        }}
                      />
                    </div>
                    <div className="taqdeem__haql-majmua">
                      <label className="taqdeem__tasmiya" htmlFor="taqdeem-taghyeerat">
                        {t('taqdeem.bayanat.taghyeerat', lugha)}
                      </label>
                      <textarea
                        id="taqdeem-taghyeerat"
                        className="taqdeem__haql taqdeem__nassi"
                        dir="auto"
                        value={taghyeerat}
                        onChange={(hadath) => {
                          setTaghyeerat(hadath.target.value);
                        }}
                      />
                    </div>

                    <div className="taqdeem__ikhtiyarat">
                      <div className="taqdeem__ikhtiyar">
                        <label className="taqdeem__ikhtiyar-tasmiya" htmlFor="taqdeem-rukhsa">
                          {t('taqdeem.bayanat.rukhsa', lugha)}
                        </label>
                        <select
                          id="taqdeem-rukhsa"
                          className="taqdeem__muntaqi"
                          value={rukhsa}
                          onChange={(hadath) => {
                            setRukhsa(hadath.target.value);
                          }}
                        >
                          {RUKHAS.map((qeema) => {
                            const miftah = MIFTAH_RUKHSA[qeema];
                            return miftah === undefined ? null : (
                              <option key={qeema} value={qeema}>
                                {t(miftah, lugha)}
                              </option>
                            );
                          })}
                        </select>
                        <Zuhur maftuh={rukhsa === 'ukhra'} className="taqdeem__rukhsa-ukhra">
                          <input
                            className="taqdeem__haql"
                            dir="auto"
                            placeholder={t('taqdeem.bayanat.rukhsa_ism', lugha)}
                            value={rukhsaIsm}
                            onChange={(hadath) => {
                              setRukhsaIsm(hadath.target.value);
                            }}
                          />
                        </Zuhur>
                      </div>
                      <div className="taqdeem__ikhtiyar">
                        <label className="taqdeem__ikhtiyar-tasmiya" htmlFor="taqdeem-tareeqa">
                          {t('taqdeem.bayanat.tareeqa', lugha)}
                        </label>
                        <select
                          id="taqdeem-tareeqa"
                          className="taqdeem__muntaqi"
                          value={tareeqa}
                          onChange={(hadath) => {
                            setTareeqa(hadath.target.value);
                          }}
                        >
                          {TURUQ.map((qeema) => {
                            const miftah = MIFTAH_TAREEQA[qeema];
                            return miftah === undefined ? null : (
                              <option key={qeema} value={qeema}>
                                {t(miftah, lugha)}
                              </option>
                            );
                          })}
                        </select>
                      </div>
                    </div>

                    <div className="taqdeem__saff-afal">
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={
                          jahhiz.isPending || unwan.trim() === '' || sharh.trim() === ''
                        }
                        aria-busy={jahhiz.isPending}
                        title={
                          unwan.trim() === '' || sharh.trim() === ''
                            ? t('taqdeem.bayanat.matlub', lugha)
                            : undefined
                        }
                        onClick={() => {
                          if (!jahhiz.isPending && unwan.trim() !== '' && sharh.trim() !== '') {
                            jahhiz.mutate();
                          }
                        }}
                      >
                        {t(
                          jahhiz.isPending ? 'taqdeem.bayanat.jari' : 'taqdeem.bayanat.jahhiz',
                          lugha,
                        )}
                      </button>
                    </div>
                    {jahhiz.error !== null ? (
                      <KutlatKhata
                        unwan={t('luba.khata.amal', lugha)}
                        khata={jahhiz.error}
                        lugha={lugha}
                        muarrif={muarrif}
                        aada={() => {
                          if (!jahhiz.isPending && unwan.trim() !== '' && sharh.trim() !== '') {
                            jahhiz.mutate();
                          }
                        }}
                      />
                    ) : null}
                  </div>
                </section>
              ) : null}

              {bayanat !== null ? (
                <section className="taqdeem__qism" aria-labelledby="taqdeem-unwan-fahs">
                  <h2 id="taqdeem-unwan-fahs" className="taqdeem__unwan-qism">
                    {t('taqdeem.fahs.unwan', lugha)}
                  </h2>
                  <ul className="taqdeem__fuhusat">
                    {bayanat.qaima.sutur.map((satr) => (
                      <SaffFahs
                        key={satr.band}
                        satr={satr}
                        muarrif={muarrif}
                        lugha={lugha}
                        munassiq={munassiq}
                        yajri={iqrar.isPending}
                        mashghul={iqrar.isPending && iqrar.variables?.tahdheer === satr.band}
                        qabilLilTahreer={bayanat.qabila_lil_tahreer}
                        alaIqrar={(tahdheer, qeema) => {
                          iqrar.mutate({ tahdheer, qeema });
                        }}
                      />
                    ))}
                  </ul>
                  <AsbabQiyas qiyas={bayanat.qiyas_tajawuz} lugha={lugha} munassiq={munassiq} />
                  {iqrar.error !== null ? (
                    <KutlatKhata
                      unwan={t('luba.khata.amal', lugha)}
                      khata={iqrar.error}
                      lugha={lugha}
                      muarrif={muarrif}
                      aada={() => {
                        const akhira = iqrar.variables;
                        if (akhira !== undefined && !iqrar.isPending) {
                          iqrar.mutate(akhira);
                        }
                      }}
                    />
                  ) : null}
                  {bayanat.qabila_lil_tahreer ? (
                    <div className="taqdeem__irsal">
                      <div className="taqdeem__saff-afal">
                        <button
                          type="button"
                          className="zir zir--tamyeez"
                          aria-disabled={!bayanat.qaima.jahiza || irsal.isPending}
                          aria-busy={irsal.isPending}
                          title={
                            bayanat.qaima.jahiza ? undefined : t('taqdeem.irsal.mughlaqa', lugha)
                          }
                          onClick={() => {
                            if (bayanat.qaima.jahiza && !irsal.isPending) {
                              iftahTaakid();
                            }
                          }}
                        >
                          {t(
                            irsal.isPending ? 'taqdeem.irsal.jari' : 'taqdeem.irsal.sallim',
                            lugha,
                          )}
                        </button>
                        <button
                          type="button"
                          className="zir"
                          aria-disabled={tawthiq.isPending}
                          aria-busy={tawthiq.isPending}
                          onClick={alaTawthiq}
                        >
                          {t('taqdeem.tawthiq.zir', lugha)}
                        </button>
                      </div>
                      <Zuhur
                        maftuh={!bayanat.qaima.jahiza}
                        className="taqdeem__mughlaqa"
                        role="status"
                      >
                        <span className="taqdeem__mughlaqa-ramz" aria-hidden="true">
                          <RamzIntizar />
                        </span>
                        {t('taqdeem.irsal.mughlaqa', lugha)}
                      </Zuhur>
                      {/* The last word before the package leaves the machine:
                          what is sent, as which revision, and that no edit
                          follows it. Focus lands on the way out, so Enter pressed
                          once too often cancels rather than sends. */}
                      <Zuhur
                        maftuh={taakidIrsal}
                        className="taqdeem__taakid"
                        role="alertdialog"
                        aria-labelledby="taqdeem-taakid-unwan"
                        aria-describedby="taqdeem-taakid-nass"
                      >
                        <p id="taqdeem-taakid-unwan" className="taqdeem__taakid-unwan">
                          {t('taqdeem.taakid.unwan', lugha)}
                        </p>
                        <p id="taqdeem-taakid-nass" className="taqdeem__taakid-nass">
                          {t('taqdeem.taakid.nass', lugha, {
                            unwan: bayanat.unwan,
                            raqm: munassiq.raqm(bayanat.murajaa),
                          })}
                        </p>
                        <div className="taqdeem__taakid-azrar">
                          <button type="button" className="zir" autoFocus onClick={aghliqTaakid}>
                            {t('taqdeem.taakid.ilgha', lugha)}
                          </button>
                          <button
                            type="button"
                            className="zir zir--tamyeez"
                            aria-busy={irsal.isPending}
                            onClick={() => {
                              if (bayanat.qaima.jahiza && !irsal.isPending) {
                                irsal.mutate();
                              }
                            }}
                          >
                            {t('taqdeem.irsal.sallim', lugha)}
                          </button>
                        </div>
                      </Zuhur>
                    </div>
                  ) : null}
                  {tawthiq.error !== null ? (
                    <KutlatKhata
                      unwan={t('luba.khata.amal', lugha)}
                      khata={tawthiq.error}
                      lugha={lugha}
                      muarrif={muarrif}
                      aada={alaTawthiq}
                    />
                  ) : null}
                  <Zuhur
                    maftuh={tawthiq.data !== undefined && irsal.data === undefined}
                    className="taqdeem__tawthiq"
                    role="status"
                  >
                    {tawthiq.data === undefined ? null : (
                      <>
                        <p className="taqdeem__tasmiya">{t('taqdeem.tawthiq.ramz', lugha)}</p>
                        <p className="taqdeem__tawthiq-ramz mono-ltr">
                          {tawthiq.data.ramz_mustakhdim}
                        </p>
                        <a
                          className="zir zir--tamyeez"
                          href={tawthiq.data.rabt_kamil ?? tawthiq.data.rabt}
                          target="_blank"
                          rel="noreferrer"
                        >
                          {t('taqdeem.tawthiq.iftah', lugha)}
                        </a>
                        <p className="taqdeem__nass-hadi">
                          {jam(
                            'taqdeem.tawthiq.muddat',
                            lugha,
                            Math.max(1, Math.ceil(tawthiq.data.thawani / 60)),
                            munassiq,
                          )}
                          {' — '}
                          {t('taqdeem.tawthiq.sharh', lugha)}
                        </p>
                      </>
                    )}
                  </Zuhur>
                  {irsal.error !== null ? (
                    <KutlatKhata
                      unwan={t('taqdeem.irsal.taadhur', lugha)}
                      khata={irsal.error}
                      lugha={lugha}
                      muarrif={muarrif}
                      aada={() => {
                        if (bayanat.qaima.jahiza && !irsal.isPending) {
                          irsal.mutate();
                        }
                      }}
                    />
                  ) : null}
                  <Zuhur maftuh={irsal.data !== undefined} className="taqdeem__najah" role="status">
                    {irsal.data === undefined ? null : (
                      <>
                        <span className="taqdeem__najah-ramz" aria-hidden="true">
                          <RamzSah />
                        </span>
                        {irsal.data.rabt_talab_damj !== null ? (
                          <a
                            className="taqdeem__rabt"
                            href={irsal.data.rabt_talab_damj}
                            target="_blank"
                            rel="noreferrer"
                          >
                            {t('taqdeem.irsal.rabt', lugha)}
                          </a>
                        ) : (
                          t('taqdeem.irsal.tamma', lugha, {
                            raqm: munassiq.raqm(irsal.data.murajaa),
                          })
                        )}
                      </>
                    )}
                  </Zuhur>
                </section>
              ) : null}
              </Mashhad>

              <section className="taqdeem__qism" aria-labelledby="taqdeem-unwan-musahamat">
                <h2 id="taqdeem-unwan-musahamat" className="taqdeem__unwan-qism">
                  {t('taqdeem.musahamat.unwan', lugha)}
                </h2>
                {musahamat.isPending ? (
                  <ul className="taqdeem__musahamat zuhur-muakhkhar" aria-hidden="true">
                    {Array.from({ length: 2 }, (_, fihris) => (
                      <li key={fihris} className="taqdeem__musahama">
                        <span className="taqdeem__haykal-satr taqdeem__haykal-satr--unwan-musahama" />
                        <span className="taqdeem__haykal-satr taqdeem__haykal-satr--luba" />
                      </li>
                    ))}
                  </ul>
                ) : musahamat.error !== null ? (
                  <KutlatKhata
                    unwan={t('taqdeem.khata.tahmil', lugha)}
                    khata={musahamat.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      void musahamat.refetch();
                    }}
                  />
                ) : musahamat.data === undefined || musahamat.data.length === 0 ? (
                  <HalatFarigha unwan={t('taqdeem.musahamat.la_shay', lugha)}>
                    <Link to="/warsha/$muarrif" params={{ muarrif }} className="zir">
                      {t('khutwa.fath_nusus', lugha)}
                    </Link>
                  </HalatFarigha>
                ) : (
                  <ul className="taqdeem__musahamat">
                    {musahamat.data.map((musahama) => (
                      <li
                        key={`${musahama.ruqaa}-${String(musahama.murajaa)}`}
                        className="taqdeem__musahama"
                      >
                        <p className="taqdeem__musahama-raas">
                          <span className="taqdeem__musahama-unwan">{musahama.unwan}</span>
                          <span className="taqdeem__musahama-hala">
                            {lugha === 'arabi' ? musahama.hala_arabi : musahama.hala_injilizi}
                          </span>
                        </p>
                        <p className="taqdeem__musahama-luba">
                          {musahama.ism_luba}
                          {' — '}
                          {t('taqdeem.hala.murajaa', lugha, {
                            raqm: munassiq.raqm(musahama.murajaa),
                          })}
                        </p>
                        {musahama.sijill.length > 0 ? (
                          <ul className="taqdeem__sijill">
                            {musahama.sijill.map((satr, fihris) => (
                              <li key={`${satr.waqt}-${String(fihris)}`}>
                                <span className="taqdeem__waqt mono-ltr">{satr.waqt}</span>
                                <span className="taqdeem__sijill-wasf">
                                  {satr.wasf_arabi}
                                  {satr.sabab !== null ? `: ${satr.sabab}` : ''}
                                </span>
                              </li>
                            ))}
                          </ul>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </div>
          </div>
        )}
      </Mashhad>

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
