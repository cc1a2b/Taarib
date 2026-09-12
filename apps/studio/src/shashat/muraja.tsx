import type { FetchStatus } from '@tanstack/react-query';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { Link } from '@tanstack/react-router';
import { useVirtualizer } from '@tanstack/react-virtual';
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
  JalsaHie,
  Kathafa,
  Lugha,
  MudkhalTaburHie,
  MusawwadaHie,
  NizamArqam,
  SafHuzmaHie,
  SatrSijillHie,
  TaburHie,
  TafasilMurajaHie,
  TaqreerSandooqHie,
  TaaliqWarshaHie,
} from '@/mustalahat/awamir';

import './muraja.css';

/** شاشة المراجعة — the owner's console: the queue, the evidence, and the ten decisions. */

/** The comment anchor: a real NassId from a checklist row, and the label the chip shows. */
interface MirsatTaaliq {
  readonly nass: string;
  readonly wasm: string;
}

type MurashshihFuhus = 'kul' | 'najahat' | 'akhfaqat' | 'lam_tujra';
type TarteebTabur = 'intizar' | 'taghtiya';

/** The three decisions a written reason unlocks, as `qarrir_muraja` names them. */
type IjraQarar = 'talab_taadil' | 'rafd' | 'sahb';

/**
 * The decisions that ask once more before they run. A request for changes
 * is answered by the next revision; these three are not — a signature is
 * published, a rejection and a revocation are written into the audit log —
 * so a mis-click has to be caught here, not in the log afterwards.
 */
type QararMuakkad = 'iaatimad' | 'rafd' | 'sahb';

/** The bulk decisions, which run once per selected row. */
type IjraJumla = 'talab_taadil' | 'rafd';

/** Which decision is waiting on the reviewer's second word, and over what. */
type Taakid =
  | { readonly nitaq: 'wahid'; readonly ijra: QararMuakkad }
  | { readonly nitaq: 'jumla'; readonly ijra: IjraJumla };

const MIFTAH_TANBIH_QARAR: Readonly<Record<IjraQarar, MiftahLugha>> = {
  talab_taadil: 'muraja.tanbih.talab_taadil',
  rafd: 'muraja.tanbih.rafd',
  sahb: 'muraja.tanbih.sahb',
};

const MIFTAH_TAAKID: Readonly<Record<QararMuakkad, MiftahLugha>> = {
  iaatimad: 'muraja.taakid.iaatimad',
  rafd: 'muraja.taakid.rafd',
  sahb: 'muraja.taakid.sahb',
};

const MIFTAH_TAAKID_JUMLA: Readonly<Record<IjraJumla, MiftahLugha>> = {
  talab_taadil: 'muraja.taakid.jumla_talab_taadil',
  rafd: 'muraja.taakid.jumla_rafd',
};

/** The confirming button repeats the decision's own verb, never a generic "yes". */
const MIFTAH_ZIR_QARAR: Readonly<Record<QararMuakkad | 'talab_taadil', MiftahLugha>> = {
  iaatimad: 'muraja.afal.iaatimad',
  talab_taadil: 'muraja.afal.talab_taadil',
  rafd: 'muraja.afal.rafd',
  sahb: 'muraja.afal.sahb',
};

/** The first guess only: every row is measured from the DOM once it mounts. */
const IRTIFA_MUBDAI_SAFF = 44;

/** How many queue rows the placeholder stands in for: a short queue's worth. */
const SUFUF_HAYKAL = 5;

/** A bulk decision's progress: how many of the selected rows it has reached. */
interface TaqaddumJumla {
  readonly tamma: number;
  readonly majmu: number;
}

/**
 * The queue drawn empty: the real column heads over rows of bars at the row's
 * own height, so the first row lands on the first placeholder. Still: a
 * shimmer is an animation on a data update.
 */
function HaykalTabur({ lugha }: { readonly lugha: Lugha }): JSX.Element {
  return (
    <table className="muraja__jadwal-tabur zuhur-muakhkhar" aria-hidden="true">
      <thead>
        <tr>
          <th />
          <th>{t('muraja.tabur.unwan_amud', lugha)}</th>
          <th>{t('muraja.tabur.luba', lugha)}</th>
          <th>{t('muraja.tabur.musahim', lugha)}</th>
          <th className="muraja__khaliya-raqm">{t('muraja.tabur.taghtiya', lugha)}</th>
          <th>{t('muraja.tabur.fuhus', lugha)}</th>
          <th>{t('muraja.tabur.intizar', lugha)}</th>
        </tr>
      </thead>
      <tbody>
        {Array.from({ length: SUFUF_HAYKAL }, (_, fihris) => (
          <tr key={fihris} className="muraja__saff muraja__saff--haykal">
            <td>
              <span className="muraja__haykal-murabba" />
            </td>
            <td>
              <span
                className="muraja__haykal-satr"
                style={{ inlineSize: `${String(50 + ((fihris * 19) % 45))}%` }}
              />
            </td>
            <td>
              <span className="muraja__haykal-satr" style={{ inlineSize: '70%' }} />
            </td>
            <td>
              <span className="muraja__haykal-satr" style={{ inlineSize: '60%' }} />
            </td>
            <td className="muraja__khaliya-raqm">
              <span className="muraja__haykal-satr muraja__haykal-satr--raqm" />
            </td>
            <td>
              <span className="muraja__haykal-satr" style={{ inlineSize: '5ch' }} />
            </td>
            <td>
              <span className="muraja__haykal-satr" style={{ inlineSize: '6ch' }} />
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** The case pane drawn empty: the title, its line of facts, and a first section. */
function HaykalTafasil(): JSX.Element {
  return (
    <div className="zuhur-muakhkhar" aria-hidden="true">
      <section className="muraja__qism muraja__qism--ras">
        <span className="muraja__haykal-satr muraja__haykal-satr--unwan" />
        <span className="muraja__haykal-satr" style={{ inlineSize: '40%' }} />
        <span className="muraja__haykal-satr muraja__haykal-satr--sharh" />
        <span className="muraja__haykal-satr muraja__haykal-satr--sharh" style={{ inlineSize: '55%' }} />
      </section>
      <section className="muraja__qism">
        <span className="muraja__haykal-satr" style={{ inlineSize: '14ch' }} />
        {Array.from({ length: 3 }, (_, fihris) => (
          <span
            key={fihris}
            className="muraja__haykal-satr muraja__haykal-satr--sharh"
            style={{ inlineSize: `${String(72 - fihris * 9)}%` }}
          />
        ))}
      </section>
    </div>
  );
}

/** The whole console drawn empty, under the strip: the filter row, the queue, the case. */
function HaykalMuraja({ lugha }: { readonly lugha: Lugha }): JSX.Element {
  return (
    <div className="muraja__badan zuhur-muakhkhar" aria-hidden="true">
      <div className="muraja__tasfiya">
        <span className="muraja__haql muraja__haql--bahth muraja__haykal-haql" />
        <span className="muraja__haql muraja__haykal-haql muraja__haykal-haql--qasir" />
        <span className="muraja__haql muraja__haykal-haql muraja__haykal-haql--qasir" />
      </div>
      <div className="muraja__amida">
        <div className="muraja__tabur">
          <HaykalTabur lugha={lugha} />
        </div>
        <div className="muraja__tafasil">
          <HaykalTafasil />
        </div>
      </div>
    </div>
  );
}

/** How many audit lines the placeholder stands in for: a screen's worth of a short log. */
const SUTUR_HAYKAL_SIJILL = 3;

/** The whole audit log drawn empty, in the log's own three columns. */
function HaykalSijill(): JSX.Element {
  return (
    <ul className="muraja__sijill muraja__sijill--kamil zuhur-muakhkhar" aria-hidden="true">
      {Array.from({ length: SUTUR_HAYKAL_SIJILL }, (_, fihris) => (
        <li key={fihris}>
          <span className="muraja__haykal-satr" style={{ inlineSize: '11ch' }} />
          <span className="muraja__haykal-satr" style={{ inlineSize: '6ch' }} />
          <span
            className="muraja__haykal-satr"
            style={{ inlineSize: `${String(72 - fihris * 14)}%` }}
          />
        </li>
      ))}
    </ul>
  );
}

/** A chevron on the disclosure control; it turns when the log is open. */
function RamzSuqut(): JSX.Element {
  return (
    <svg
      className="muraja__sijill-ramz"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M5.5 9.5L12 16L18.5 9.5" />
    </svg>
  );
}

/** Where one query stands, with the switched-off case separated from the rest. */
type HalatIstifsar = 'muattal' | 'jari' | 'khata' | 'farigh' | 'jahiz';

/** The three fields of a query result the state below is derived from. */
interface QiraatIstifsar {
  readonly isPending: boolean;
  readonly fetchStatus: FetchStatus;
  readonly error: KhataJisr | null;
}

/**
 * A query that is switched off reports itself as pending and holds no data, so
 * a screen that reads `isPending` alone shows a spinner for something it never
 * asked for, and one that reads `data === undefined` shows an empty state for
 * the same thing. Both sentences are lies; this tells the four cases apart.
 */
function halatIstifsar(qiraa: QiraatIstifsar, farigh: boolean): HalatIstifsar {
  if (qiraa.error !== null) {
    return 'khata';
  }
  if (qiraa.isPending) {
    // Pending with nothing in flight is the shape of `enabled: false`.
    return qiraa.fetchStatus === 'idle' ? 'muattal' : 'jari';
  }
  return farigh ? 'farigh' : 'jahiz';
}

/** The kinds of checklist outcome the state mark can be coloured for. */
type FiatFahs = 'najahat' | 'akhfaqat' | 'tanbeeh' | 'majhul' | 'lam_tujra';

/**
 * The mark's kind, from the stable key `slug_hala_band` emits.
 *
 * The label beside the mark is `hala_arabi`, sent from the same enum, so this
 * screen no longer keeps its own words for the states — which is what let the
 * previous version compare against `najah`, a key the backend has never sent.
 * The pass slug is `ijtaz`, so every passing row on this console read as "not
 * run" until the label arrived on the wire. What remains here is only the
 * colour: a pass, a failure, a warning that fired whether or not it has been
 * acknowledged, the overflow check over a project that was not — or only
 * partly — measured, and a slug this build has never heard of. That last one
 * reads as unrun rather than as a pass, because a failure shown as a failure
 * is the half of this column a reviewer cannot afford to be told wrongly.
 */
function fiatFahs(hala: string): FiatFahs {
  switch (hala) {
    case 'ijtaz':
      return 'najahat';
    case 'rasab':
      return 'akhfaqat';
    case 'yantazir_iqrar':
    case 'muqarr':
      return 'tanbeeh';
    case 'ghayr_maqis':
    case 'maqis_juzi':
      return 'majhul';
    default:
      return 'lam_tujra';
  }
}

interface KhasaisJadwal {
  readonly sufuf: readonly MudkhalTaburHie[];
  readonly mukhtar: string | null;
  readonly muhaddada: ReadonlySet<string>;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly alaIkhtiyar: (ruqaa: string) => void;
  readonly alaTahdid: (ruqaa: string, qeema: boolean, mumtadd: boolean) => void;
}

function JadwalTabur({
  sufuf,
  mukhtar,
  muhaddada,
  lugha,
  munassiq,
  alaIkhtiyar,
  alaTahdid,
}: KhasaisJadwal): JSX.Element {
  // Click fires before change, so the modifier survives into the change handler.
  const kanMumtadd = useRef(false);
  return (
    <table className="muraja__jadwal-tabur">
      <thead>
        <tr>
          <th aria-label={t('muraja.tabur.tahdid', lugha)} />
          <th>{t('muraja.tabur.unwan_amud', lugha)}</th>
          <th>{t('muraja.tabur.luba', lugha)}</th>
          <th>{t('muraja.tabur.musahim', lugha)}</th>
          <th className="muraja__khaliya-raqm">{t('muraja.tabur.taghtiya', lugha)}</th>
          <th>{t('muraja.tabur.fuhus', lugha)}</th>
          <th>{t('muraja.tabur.intizar', lugha)}</th>
        </tr>
      </thead>
      <tbody>
        {sufuf.map((saf) => (
          <tr
            key={`${saf.ruqaa}-${String(saf.murajaa)}`}
            className={
              saf.ruqaa === mukhtar ? 'muraja__saff muraja__saff--mukhtar' : 'muraja__saff'
            }
            onClick={() => {
              alaIkhtiyar(saf.ruqaa);
            }}
          >
            <td>
              <input
                type="checkbox"
                checked={muhaddada.has(saf.ruqaa)}
                aria-label={t('muraja.tabur.tahdid', lugha)}
                onClick={(hadath) => {
                  hadath.stopPropagation();
                  kanMumtadd.current = hadath.shiftKey;
                }}
                onChange={(hadath) => {
                  alaTahdid(saf.ruqaa, hadath.target.checked, kanMumtadd.current);
                }}
              />
            </td>
            <td className="muraja__khaliya-unwan">{saf.unwan}</td>
            <td>{saf.ism_luba}</td>
            <td>
              {saf.ism_musahim}{' '}
              <span className="mono-ltr muraja__basma">{saf.musahim}</span>
            </td>
            <td className="muraja__khaliya-raqm">{munassiq.nisba(kasr(saf.taghtiya_nisba))}</td>
            <td>
              <span className={`muraja__fuhus muraja__fuhus--${saf.fuhus}`}>
                {saf.fuhus_arabi}
              </span>
            </td>
            <td>{saf.umr_arabi}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

interface KhasaisNusus {
  readonly sufuf: readonly SafHuzmaHie[];
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** The density, which is what changes how tall a measured row comes back. */
  readonly kathafa: Kathafa;
  /** The selected row's index into `sufuf`, stable across the local search filter. */
  readonly mukhtar?: number | undefined;
  readonly alaIkhtiyar?: (fihris: number) => void;
}

function JadwalNusus({
  sufuf,
  lugha,
  munassiq,
  kathafa,
  mukhtar,
  alaIkhtiyar,
}: KhasaisNusus): JSX.Element {
  const [bahth, setBahth] = useState('');
  const zahira = useMemo(() => {
    // The original index rides along, so selection survives filtering.
    const mafhursa = sufuf.map((saf, fihris) => ({ saf, fihris }));
    if (bahth === '') {
      return mafhursa;
    }
    const ibra = bahth.toLowerCase();
    return mafhursa.filter(
      ({ saf }) => saf.masdar.toLowerCase().includes(ibra) || saf.hadaf.includes(bahth),
    );
  }, [sufuf, bahth]);

  const hawiya = useRef<HTMLDivElement | null>(null);
  const mufahris = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: zahira.length,
    getScrollElement: () => hawiya.current,
    estimateSize: () => IRTIFA_MUBDAI_SAFF,
    overscan: 10,
  });

  // Measured heights are cached per row, so a density change has to throw them
  // away; each row is then measured again from the DOM as it mounts, which is
  // what keeps the box and the type inside it growing together.
  useEffect(() => {
    mufahris.measure();
  }, [mufahris, kathafa]);

  const asasSaff =
    alaIkhtiyar === undefined
      ? 'muraja__nusus-saff'
      : 'muraja__nusus-saff muraja__nusus-saff--qabil';

  return (
    <div className="muraja__nusus">
      <input
        className="muraja__haql muraja__haql--bahth"
        type="search"
        dir="auto"
        placeholder={t('muraja.nusus.bahth', lugha)}
        value={bahth}
        onChange={(hadath) => {
          setBahth(hadath.target.value);
        }}
      />
      <div
        className="muraja__nusus-qaima"
        ref={hawiya}
        role={alaIkhtiyar === undefined ? undefined : 'listbox'}
        aria-label={alaIkhtiyar === undefined ? undefined : t('muraja.tafasil.nusus', lugha)}
        tabIndex={alaIkhtiyar === undefined ? undefined : 0}
        onKeyDown={
          alaIkhtiyar === undefined
            ? undefined
            : (hadath) => {
                if (
                  hadath.key !== 'ArrowDown' &&
                  hadath.key !== 'ArrowUp' &&
                  hadath.key !== 'Home' &&
                  hadath.key !== 'End'
                ) {
                  return;
                }
                hadath.preventDefault();
                const zahir = zahira.findIndex(({ fihris }) => fihris === mukhtar);
                const hadaf =
                  hadath.key === 'Home'
                    ? 0
                    : hadath.key === 'End'
                      ? zahira.length - 1
                      : hadath.key === 'ArrowDown'
                        ? Math.min(zahir + 1, zahira.length - 1)
                        : Math.max(zahir - 1, 0);
                const band = zahira.at(hadaf < 0 ? 0 : hadaf);
                if (band !== undefined) {
                  alaIkhtiyar(band.fihris);
                  mufahris.scrollToIndex(hadaf < 0 ? 0 : hadaf);
                }
              }
        }
      >
        <div
          className="muraja__nusus-jism"
          style={{ height: `${String(mufahris.getTotalSize())}px` }}
        >
          {mufahris.getVirtualItems().map((band) => {
            const zahir = zahira.at(band.index);
            if (zahir === undefined) {
              return null;
            }
            const { saf, fihris } = zahir;
            return (
              <div
                key={`${String(fihris)}-${saf.masdar}`}
                data-index={band.index}
                ref={mufahris.measureElement}
                className={
                  mukhtar === fihris ? `${asasSaff} muraja__nusus-saff--mukhtar` : asasSaff
                }
                style={{ transform: `translateY(${String(band.start)}px)` }}
                role={alaIkhtiyar === undefined ? undefined : 'option'}
                aria-selected={alaIkhtiyar === undefined ? undefined : mukhtar === fihris}
                onClick={
                  alaIkhtiyar === undefined
                    ? undefined
                    : () => {
                        alaIkhtiyar(fihris);
                      }
                }
              >
                <span className="muraja__nass-masdar" dir="auto">
                  {saf.masdar}
                </span>
                <span className="muraja__nass-hadaf">{saf.hadaf === '' ? '—' : saf.hadaf}</span>
              </div>
            );
          })}
        </div>
      </div>
      <p className="muraja__nass-hadi">
        {jam('muraja.nusus.adad', lugha, zahira.length, munassiq)}
      </p>
    </div>
  );
}

export function Muraja(): JSX.Element {
  const makhzan = useQueryClient();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const kathafa: Kathafa = idadat.data?.kathafa ?? 'murih';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const jalsa = useQuery<JalsaHie, KhataJisr>({
    queryKey: mafatih.jalsa,
    queryFn: () => nadi('jalsati'),
  });
  const malik = jalsa.data?.malik === true;

  const tabur = useQuery<TaburHie, KhataJisr>({
    queryKey: mafatih.tabur,
    queryFn: () => nadi('tabur_muraja'),
    enabled: malik,
  });

  const [mukhtar, setMukhtar] = useState<string | null>(null);
  const [muhaddada, setMuhaddada] = useState<ReadonlySet<string>>(new Set());
  /** The last row whose checkbox was toggled — the anchor a shift-click extends from. */
  const marjiTahdid = useRef<string | null>(null);
  const [bahth, setBahth] = useState('');
  const [fuhus, setFuhus] = useState<MurashshihFuhus>('kul');
  const [tarteeb, setTarteeb] = useState<TarteebTabur>('intizar');
  const [sabab, setSabab] = useState('');
  const [matn, setMatn] = useState('');
  const [sandooq, setSandooq] = useState<TaqreerSandooqHie | null>(null);
  const [nassMukhtar, setNassMukhtar] = useState<number | null>(null);
  const [rabt, setRabt] = useState(false);
  const [mirsat, setMirsat] = useState<MirsatTaaliq | null>(null);
  const [taakid, setTaakid] = useState<Taakid | null>(null);
  const [sijillMaftuh, setSijillMaftuh] = useState(false);
  /** The control that opened the confirmation, so closing it hands focus back. */
  const fatihTaakid = useRef<HTMLElement | null>(null);

  const iftahTaakid = (jadeed: Taakid): void => {
    fatihTaakid.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setTaakid(jadeed);
  };

  const aghliqTaakid = (): void => {
    setTaakid(null);
    fatihTaakid.current?.focus();
    fatihTaakid.current = null;
  };

  // Anchors, string selection and a pending confirmation belong to one
  // submission's draft, never to the next one's. The bulk confirmation is
  // about the selection, not the open row, so it survives a change of row.
  useEffect(() => {
    setNassMukhtar(null);
    setRabt(false);
    setMirsat(null);
    setTaakid((hali) => (hali?.nitaq === 'wahid' ? null : hali));
  }, [mukhtar]);

  useEffect(() => {
    if (taakid === null) {
      return undefined;
    }
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      if (hadath.key === 'Escape') {
        setTaakid(null);
        fatihTaakid.current?.focus();
        fatihTaakid.current = null;
      }
    };
    document.addEventListener('keydown', alaMiftah);
    return () => {
      document.removeEventListener('keydown', alaMiftah);
    };
  }, [taakid]);

  const sufuf = useMemo(() => {
    const kul = tabur.data?.sufuf ?? [];
    const ibra = bahth.toLowerCase();
    const murashshaha = kul.filter((saf) => {
      if (fuhus !== 'kul' && saf.fuhus !== fuhus) {
        return false;
      }
      if (ibra !== '') {
        const fi =
          saf.unwan.toLowerCase().includes(ibra) ||
          saf.ism_luba.toLowerCase().includes(ibra) ||
          saf.ism_musahim.toLowerCase().includes(ibra);
        if (!fi) {
          return false;
        }
      }
      return true;
    });
    if (tarteeb === 'taghtiya') {
      return [...murashshaha].sort((awwal, thani) => kasr(thani.taghtiya_nisba) - kasr(awwal.taghtiya_nisba));
    }
    return murashshaha;
  }, [tabur.data, bahth, fuhus, tarteeb]);

  useEffect(() => {
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      const hadafHtml = hadath.target instanceof HTMLElement ? hadath.target.tagName : '';
      if (hadafHtml === 'INPUT' || hadafHtml === 'TEXTAREA') {
        return;
      }
      if (hadath.key === 'Enter') {
        // Enter never steals activation from a focused control; it confirms the queue selection.
        if (hadafHtml === 'BUTTON' || hadafHtml === 'SELECT' || hadafHtml === 'A') {
          return;
        }
        const saf = sufuf.find((wahid) => wahid.ruqaa === mukhtar) ?? sufuf.at(0);
        if (saf !== undefined) {
          hadath.preventDefault();
          setMukhtar(saf.ruqaa);
        }
        return;
      }
      if (
        hadath.key !== 'ArrowDown' &&
        hadath.key !== 'ArrowUp' &&
        hadath.key !== 'Home' &&
        hadath.key !== 'End'
      ) {
        return;
      }
      hadath.preventDefault();
      const fihris = sufuf.findIndex((saf) => saf.ruqaa === mukhtar);
      const tali =
        hadath.key === 'Home'
          ? 0
          : hadath.key === 'End'
            ? sufuf.length - 1
            : hadath.key === 'ArrowDown'
              ? Math.min(fihris + 1, sufuf.length - 1)
              : Math.max(fihris - 1, 0);
      const saf = sufuf.at(tali < 0 ? 0 : tali);
      if (saf !== undefined) {
        setMukhtar(saf.ruqaa);
      }
    };
    window.addEventListener('keydown', alaMiftah);
    return () => {
      window.removeEventListener('keydown', alaMiftah);
    };
  }, [sufuf, mukhtar]);

  const tafasil = useQuery<TafasilMurajaHie, KhataJisr>({
    queryKey: mafatih.muraja(mukhtar ?? ''),
    queryFn: () => nadi('tafasil_muraja', { ruqaa: mukhtar ?? '' }),
    enabled: malik && mukhtar !== null,
  });

  const sijillKull = useQuery<SatrSijillHie[], KhataJisr>({
    queryKey: mafatih.sijill_muraja,
    queryFn: () => nadi('sijill_muraja_kull'),
    enabled: malik,
  });

  const aidTahmil = (): void => {
    void makhzan.invalidateQueries({ queryKey: mafatih.tabur });
    if (mukhtar !== null) {
      void makhzan.invalidateQueries({ queryKey: mafatih.muraja(mukhtar) });
    }
    void makhzan.invalidateQueries({ queryKey: mafatih.sijill_muraja });
  };

  const taaliq = useMutation<TaaliqWarshaHie[], KhataJisr, { matn: string }>({
    // SafHuzmaHie carries no NassId, so a table-selected string cannot be anchored by id;
    // only a checklist-set anchor carries one, and everything else honestly sends null.
    mutationFn: ({ matn: badan }) =>
      nadi('allaq_muraja', { ruqaa: mukhtar ?? '', nass: mirsat?.nass ?? null, matn: badan }),
    onSuccess: () => {
      setMatn('');
      setMirsat(null);
      setRabt(false);
      aidTahmil();
      ansha({ naw: 'najah', nass: t('muraja.tanbih.taaliq', lugha) });
    },
  });

  const qarar = useMutation<MusawwadaHie, KhataJisr, { ruqaa: string; ijra: IjraQarar }>({
    mutationFn: ({ ruqaa, ijra }) => nadi('qarrir_muraja', { ruqaa, ijra, sabab }),
    // The decision lands at the foot of a pane that is usually scrolled well
    // above it, and the row it was about may leave the queue on the refetch;
    // the notice is the one place the reviewer is sure to see it.
    onSuccess: (natija, { ijra }) => {
      setSabab('');
      aidTahmil();
      ansha({ naw: 'najah', nass: t(MIFTAH_TANBIH_QARAR[ijra], lugha), tafsil: natija.unwan });
    },
  });

  const iaatimad = useMutation<MusawwadaHie, KhataJisr, { ruqaa: string }>({
    mutationFn: ({ ruqaa }) => nadi('iaatimad_muraja', { ruqaa }),
    onSuccess: (natija) => {
      aidTahmil();
      ansha({ naw: 'najah', nass: t('muraja.tanbih.iaatimad', lugha), tafsil: natija.unwan });
    },
  });

  const [taqaddumJumla, setTaqaddumJumla] = useState<TaqaddumJumla | null>(null);

  const jumla = useMutation<number, KhataJisr, { ijra: IjraJumla }>({
    mutationFn: async ({ ijra }) => {
      let adad = 0;
      const majmu = muhaddada.size;
      setTaqaddumJumla({ tamma: 0, majmu });
      for (const ruqaa of muhaddada) {
        // One record per identity, exactly as the single action writes it.
        // eslint-disable-next-line no-await-in-loop
        await nadi('qarrir_muraja', { ruqaa, ijra, sabab });
        adad += 1;
        setTaqaddumJumla({ tamma: adad, majmu });
      }
      return adad;
    },
    // The selection is gone by the time this runs, so the count in the notice
    // is the only record on screen of how many rows the decision reached. On a
    // failure the progress is kept instead: the rows already decided stay
    // decided, and the error block below says how many that was.
    onSuccess: (adad) => {
      setMuhaddada(new Set());
      setSabab('');
      setTaqaddumJumla(null);
      aidTahmil();
      ansha({ naw: 'najah', nass: jam('muraja.tanbih.jumla', lugha, adad, munassiq) });
    },
  });

  const muarrifMukhtar = useMemo(
    () => sufuf.find((saf) => saf.ruqaa === mukhtar)?.muarrif ?? null,
    [sufuf, mukhtar],
  );

  const sandooqThabbit = useMutation<TaqreerSandooqHie, KhataJisr, void>({
    mutationFn: () =>
      nadi('sandooq_thabbit', { ruqaa: mukhtar ?? '', muarrif: muarrifMukhtar ?? '' }),
    // An install whose post-write verification did not hold is not a success
    // said quietly: it is the one outcome the sandbox exists to catch.
    onSuccess: (natija) => {
      setSandooq(natija);
      ansha({
        naw: natija.salim ? 'najah' : 'tanbeeh',
        nass: t(
          natija.salim ? 'muraja.tanbih.sandooq_thabbit' : 'muraja.tanbih.sandooq_naqis',
          lugha,
        ),
        tafsil: natija.hasila_arabi,
      });
    },
  });
  const sandooqAtliq = useMutation<boolean, KhataJisr, void>({
    mutationFn: () =>
      nadi('sandooq_atliq', { ruqaa: mukhtar ?? '', muarrif: muarrifMukhtar ?? '' }),
    // The game opens in a window of its own; nothing on this screen changes.
    onSuccess: () => {
      ansha({ naw: 'najah', nass: t('muraja.tanbih.sandooq_atliq', lugha) });
    },
  });
  const sandooqImsah = useMutation<boolean, KhataJisr, void>({
    mutationFn: () => nadi('sandooq_imsah', { ruqaa: mukhtar ?? '' }),
    onSuccess: () => {
      setSandooq(null);
      ansha({ naw: 'najah', nass: t('muraja.tanbih.sandooq_imsah', lugha) });
    },
  });

  // The palette registers closures once per id set, so they read live state through this ref.
  const muharrikatLawha = {
    hadithTabur: (): void => {
      void makhzan.invalidateQueries({ queryKey: mafatih.tabur });
    },
    iftahAwwal: (): void => {
      const saf = sufuf.at(0);
      if (saf !== undefined) {
        setMukhtar(saf.ruqaa);
      }
    },
    // The palette reaches the same confirmation the buttons do; a keyboard
    // shortcut is not a shorter road past the second word.
    iaatimad: (): void => {
      if (mukhtar !== null && !iaatimad.isPending) {
        iftahTaakid({ nitaq: 'wahid', ijra: 'iaatimad' });
      }
    },
    talabTaadil: (): void => {
      if (mukhtar !== null && sabab.trim() !== '' && !qarar.isPending) {
        qarar.mutate({ ruqaa: mukhtar, ijra: 'talab_taadil' });
      }
    },
    rafd: (): void => {
      if (mukhtar !== null && sabab.trim() !== '' && !qarar.isPending) {
        iftahTaakid({ nitaq: 'wahid', ijra: 'rafd' });
      }
    },
  };
  const marjaLawha = useRef(muharrikatLawha);
  useEffect(() => {
    marjaLawha.current = muharrikatLawha;
  });

  const jahizLawha = malik && idadat.data !== undefined;
  const awamirLawha = useMemo<AmrLawha[]>(() => {
    if (!jahizLawha) {
      return [];
    }
    const majal = t('shasha.muraja', lugha);
    return [
      {
        muarrif: 'muraja.hadith_tabur',
        majal,
        unwan: t('muraja.lawha.hadith_tabur', lugha),
        nafidh: () => {
          marjaLawha.current.hadithTabur();
        },
      },
      {
        muarrif: 'muraja.iftah_awwal',
        majal,
        unwan: t('muraja.lawha.iftah_awwal', lugha),
        nafidh: () => {
          marjaLawha.current.iftahAwwal();
        },
      },
      {
        muarrif: 'muraja.iaatimad',
        majal,
        unwan: t('muraja.lawha.iaatimad', lugha),
        nafidh: () => {
          marjaLawha.current.iaatimad();
        },
      },
      {
        muarrif: 'muraja.talab_taadil',
        majal,
        unwan: t('muraja.lawha.talab_taadil', lugha),
        nafidh: () => {
          marjaLawha.current.talabTaadil();
        },
      },
      {
        muarrif: 'muraja.rafd',
        majal,
        unwan: t('muraja.lawha.rafd', lugha),
        nafidh: () => {
          marjaLawha.current.rafd();
        },
      },
    ];
  }, [jahizLawha, lugha]);
  useSajjilAwamir(awamirLawha);

  const alaIkhtiyarNass = (fihris: number): void => {
    if (nassMukhtar === fihris) {
      setNassMukhtar(null);
      setRabt(false);
      return;
    }
    setNassMukhtar(fihris);
  };

  const naffidhTaakid = (): void => {
    if (taakid === null) {
      return;
    }
    if (taakid.nitaq === 'jumla') {
      if (!jumla.isPending && sabab.trim() !== '') {
        jumla.mutate({ ijra: taakid.ijra });
      }
    } else if (mukhtar !== null) {
      if (taakid.ijra === 'iaatimad') {
        if (!iaatimad.isPending) {
          iaatimad.mutate({ ruqaa: mukhtar });
        }
      } else if (!qarar.isPending && sabab.trim() !== '') {
        qarar.mutate({ ruqaa: mukhtar, ijra: taakid.ijra });
      }
    }
    aghliqTaakid();
  };

  const bayanat = tafasil.data;
  const murashshah = bahth !== '' || fuhus !== 'kul';
  const halatTabur = halatIstifsar(tabur, sufuf.length === 0);
  const sijillSufuf = sijillKull.data ?? [];
  const halatSijill = halatIstifsar(sijillKull, sijillSufuf.length === 0);

  // A session that failed to answer is not a session that answered "no": until
  // this was read, a backend that never replied looked exactly like a refusal.
  const wajh =
    jalsa.isPending || idadat.isPending
      ? 'tahmil'
      : jalsa.error !== null
        ? 'khata'
        : malik
          ? 'jahiz'
          : 'mamnu';

  const raas = (
    <RaasShasha
      rujoo={{ ila: 'maktaba' }}
      nassRujoo={t('muraja.raji', lugha)}
      unwan={t('shasha.muraja', lugha)}
      tafasil={
        tabur.data === undefined
          ? null
          : t('muraja.ihsa', lugha, {
              majmu: munassiq.raqm(tabur.data.majmu),
              najahat: munassiq.raqm(tabur.data.najahat),
              akhfaqat: munassiq.raqm(tabur.data.akhfaqat),
              lam_tujra: munassiq.raqm(tabur.data.lam_tujra),
            })
      }
    />
  );

  if (wajh !== 'jahiz') {
    return (
      <div className="muraja">
        {raas}
        <Mashhad miftah={wajh} className="muraja__mashhad">
          {wajh === 'tahmil' ? (
            <HaykalMuraja lugha={lugha} />
          ) : jalsa.error !== null ? (
            <div className="muraja__hala">
              <KutlatKhata
                unwan={t('amm.khata', lugha)}
                khata={jalsa.error}
                lugha={lugha}
                aada={() => {
                  void jalsa.refetch();
                }}
              />
            </div>
          ) : (
            <div className="muraja__hala">
              <HalatFarigha shasha unwan={t('shasha.muraja', lugha)} nass={t('muraja.mamnu', lugha)}>
                <Link to="/" className="zir">
                  {t('muraja.raji', lugha)}
                </Link>
              </HalatFarigha>
            </div>
          )}
        </Mashhad>
      </div>
    );
  }

  const wajhTabur = tabur.error !== null ? 'khata' : halatTabur;
  const wajhTafasil =
    mukhtar === null
      ? 'la_ikhtiyar'
      : tafasil.isPending
        ? `tahmil:${mukhtar}`
        : tafasil.error !== null
          ? `khata:${mukhtar}`
          : `jahiz:${mukhtar}`;

  return (
    <div className="muraja">
      {raas}

      <Mashhad miftah="jahiz" className="muraja__mashhad">
      <div className="muraja__badan">
      <div className="muraja__tasfiya">
        <input
          className="muraja__haql muraja__haql--bahth"
          type="search"
          dir="auto"
          placeholder={t('muraja.tasfiya.bahth', lugha)}
          value={bahth}
          onChange={(hadath) => {
            setBahth(hadath.target.value);
          }}
        />
        <select
          className="muraja__haql"
          aria-label={t('muraja.tabur.fuhus', lugha)}
          value={fuhus}
          onChange={(hadath) => {
            setFuhus(hadath.target.value as MurashshihFuhus);
          }}
        >
          <option value="kul">{t('muraja.tasfiya.kul', lugha)}</option>
          <option value="najahat">{t('muraja.tasfiya.najahat', lugha)}</option>
          <option value="akhfaqat">{t('muraja.tasfiya.akhfaqat', lugha)}</option>
          <option value="lam_tujra">{t('muraja.tasfiya.lam_tujra', lugha)}</option>
        </select>
        <select
          className="muraja__haql"
          aria-label={t('muraja.tasfiya.tarteeb', lugha)}
          value={tarteeb}
          onChange={(hadath) => {
            setTarteeb(hadath.target.value as TarteebTabur);
          }}
        >
          <option value="intizar">{t('muraja.tasfiya.intizar', lugha)}</option>
          <option value="taghtiya">{t('muraja.tasfiya.taghtiya', lugha)}</option>
        </select>
        <Zuhur maftuh={muhaddada.size > 0} className="muraja__jumla">
          <span>{jam('muraja.jumla.adad', lugha, muhaddada.size, munassiq)}</span>
          {/* The bulk decision walks the selection one record at a time, so
              its progress has a real denominator and draws against it. */}
          <Zuhur maftuh={jumla.isPending && taqaddumJumla !== null} className="muraja__miqyas">
            {taqaddumJumla === null ? null : (
              <>
                <span
                  className="muraja__miqyas-masar"
                  role="progressbar"
                  aria-label={t('muraja.jumla.jari', lugha)}
                  aria-valuemin={0}
                  aria-valuemax={taqaddumJumla.majmu}
                  aria-valuenow={taqaddumJumla.tamma}
                >
                  <span
                    className="muraja__miqyas-malu"
                    style={{
                      inlineSize: `${String(
                        taqaddumJumla.majmu === 0
                          ? 0
                          : (taqaddumJumla.tamma / taqaddumJumla.majmu) * 100,
                      )}%`,
                    }}
                  />
                </span>
                <span className="muraja__miqyas-nass">
                  {t('amm.taqaddum.min', lugha, {
                    tamma: munassiq.raqm(taqaddumJumla.tamma),
                    majmu: munassiq.raqm(taqaddumJumla.majmu),
                  })}
                </span>
              </>
            )}
          </Zuhur>
          <button
            type="button"
            className="zir"
            aria-disabled={jumla.isPending || sabab.trim() === ''}
            aria-busy={jumla.isPending && jumla.variables?.ijra === 'talab_taadil'}
            title={sabab.trim() === '' ? t('muraja.afal.sabab_matlub', lugha) : undefined}
            onClick={() => {
              if (!jumla.isPending && sabab.trim() !== '') {
                iftahTaakid({ nitaq: 'jumla', ijra: 'talab_taadil' });
              }
            }}
          >
            {t('muraja.afal.talab_taadil', lugha)}
          </button>
          <span className="muraja__fasil-afal" aria-hidden="true" />
          <button
            type="button"
            className="zir zir--khatar muraja__qarar--rafd"
            aria-disabled={jumla.isPending || sabab.trim() === ''}
            aria-busy={jumla.isPending && jumla.variables?.ijra === 'rafd'}
            title={sabab.trim() === '' ? t('muraja.afal.sabab_matlub', lugha) : undefined}
            onClick={() => {
              if (!jumla.isPending && sabab.trim() !== '') {
                iftahTaakid({ nitaq: 'jumla', ijra: 'rafd' });
              }
            }}
          >
            {t('muraja.afal.rafd', lugha)}
          </button>
        </Zuhur>
        <Zuhur
          maftuh={taakid?.nitaq === 'jumla' && muhaddada.size > 0}
          className="muraja__taakid muraja__taakid--jumla"
          role="alertdialog"
          aria-labelledby="muraja-taakid-jumla-unwan"
          aria-describedby="muraja-taakid-jumla-nass"
        >
          {taakid === null || taakid.nitaq !== 'jumla' ? null : (
            <>
              <p id="muraja-taakid-jumla-unwan" className="muraja__taakid-unwan">
                {t('muraja.taakid.unwan', lugha)}
              </p>
              <p id="muraja-taakid-jumla-nass" className="muraja__taakid-nass">
                {t(MIFTAH_TAAKID_JUMLA[taakid.ijra], lugha)}
              </p>
              <p className="muraja__taakid-sabab" dir="rtl">
                {sabab}
              </p>
              <div className="muraja__taakid-azrar">
                {/* Focus lands on the way out, so Enter pressed once too often
                    cancels rather than confirms. */}
                <button type="button" className="zir" autoFocus onClick={aghliqTaakid}>
                  {t('muraja.taakid.ilgha', lugha)}
                </button>
                <button
                  type="button"
                  className={
                    taakid.ijra === 'rafd' ? 'zir zir--khatar muraja__qarar--rafd' : 'zir'
                  }
                  onClick={naffidhTaakid}
                >
                  {t(MIFTAH_ZIR_QARAR[taakid.ijra], lugha)}
                </button>
              </div>
            </>
          )}
        </Zuhur>
        {jumla.error !== null ? (
          <div className="muraja__khata-jumla">
            <KutlatKhata unwan={t('luba.khata.amal', lugha)} khata={jumla.error} lugha={lugha} />
            {/* The rows already decided stay decided; without this line a
                failure on the fourth of ten reads as a failure of all ten. */}
            {taqaddumJumla === null ? null : (
              <p className="muraja__nass-hadi">
                {t('muraja.jumla.tamma_qabl', lugha, {
                  adad: munassiq.raqm(taqaddumJumla.tamma),
                })}
              </p>
            )}
          </div>
        ) : null}
      </div>

      <div className="muraja__amida">
        <div className="muraja__tabur">
          <Mashhad miftah={wajhTabur} className="muraja__mashhad-tabur">
          {tabur.error !== null ? (
            <KutlatKhata
              unwan={t('muraja.khata.tabur', lugha)}
              khata={tabur.error}
              lugha={lugha}
              aada={() => {
                void tabur.refetch();
              }}
            />
          ) : halatTabur === 'jari' ? (
            <HaykalTabur lugha={lugha} />
          ) : halatTabur === 'muattal' ? (
            <HalatFarigha unwan={t('shasha.muraja', lugha)} nass={t('muraja.mamnu', lugha)} />
          ) : halatTabur === 'farigh' ? (
            /* Two different facts: a queue with nothing in it, and a queue whose
               every row the reviewer's own filter has hidden. The first sentence
               would be a lie in the second case. */
            <HalatFarigha
              shasha
              unwan={t('shasha.muraja', lugha)}
              nass={t(murashshah ? 'muraja.tabur.la_mutabaqa' : 'muraja.tabur.farigh', lugha)}
            >
              <button
                type="button"
                className="zir"
                onClick={() => {
                  if (murashshah) {
                    setBahth('');
                    setFuhus('kul');
                    return;
                  }
                  void makhzan.invalidateQueries({ queryKey: mafatih.tabur });
                }}
              >
                {t(murashshah ? 'muraja.tasfiya.kul' : 'muraja.lawha.hadith_tabur', lugha)}
              </button>
            </HalatFarigha>
          ) : (
            <JadwalTabur
              sufuf={sufuf}
              mukhtar={mukhtar}
              muhaddada={muhaddada}
              lugha={lugha}
              munassiq={munassiq}
              alaIkhtiyar={setMukhtar}
              alaTahdid={(ruqaa, qeema, mumtadd) => {
                const marja = marjiTahdid.current;
                marjiTahdid.current = ruqaa;
                setMuhaddada((hali) => {
                  const talia = new Set(hali);
                  const min = sufuf.findIndex((saf) => saf.ruqaa === marja);
                  const ila = sufuf.findIndex((saf) => saf.ruqaa === ruqaa);
                  if (mumtadd && min >= 0 && ila >= 0) {
                    // Shift extends from the anchor over the visible order,
                    // applying the clicked box's new state to the whole range.
                    for (const saf of sufuf.slice(Math.min(min, ila), Math.max(min, ila) + 1)) {
                      if (qeema) {
                        talia.add(saf.ruqaa);
                      } else {
                        talia.delete(saf.ruqaa);
                      }
                    }
                    return talia;
                  }
                  if (qeema) {
                    talia.add(ruqaa);
                  } else {
                    talia.delete(ruqaa);
                  }
                  return talia;
                });
              }}
            />
          )}
          </Mashhad>

          <div className="muraja__sijill-kull">
            <button
              type="button"
              className="muraja__sijill-zir"
              aria-expanded={sijillMaftuh}
              aria-controls="muraja-sijill-kull"
              onClick={() => {
                setSijillMaftuh((hali) => !hali);
              }}
            >
              <RamzSuqut />
              {t('muraja.sijill.unwan', lugha)}
            </button>
            <Zuhur maftuh={sijillMaftuh} id="muraja-sijill-kull" className="muraja__sijill-jism">
              {sijillKull.error !== null ? (
                <KutlatKhata
                  unwan={t('amm.khata', lugha)}
                  khata={sijillKull.error}
                  lugha={lugha}
                  aada={() => {
                    void sijillKull.refetch();
                  }}
                />
              ) : halatSijill === 'jari' ? (
                <HaykalSijill />
              ) : halatSijill === 'muattal' ? (
                <p className="muraja__nass-hadi">{t('muraja.mamnu', lugha)}</p>
              ) : halatSijill === 'farigh' ? (
                <HalatFarigha
                  unwan={t('muraja.sijill.unwan', lugha)}
                  nass={t('muraja.sijill.farigh', lugha)}
                />
              ) : (
                <ul className="muraja__sijill muraja__sijill--kamil">
                  {sijillSufuf.map((satr, fihris) => (
                    <li key={`${satr.waqt}-${String(fihris)}`}>
                      <span className="muraja__sijill-waqt" dir="ltr">
                        {satr.waqt}
                      </span>
                      <span className="mono-ltr muraja__basma">{satr.ruqaa.slice(0, 8)}</span>
                      <span className="muraja__sijill-nass">
                        {satr.wasf_arabi}
                        {satr.sabab !== null ? `: ${satr.sabab}` : ''}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Zuhur>
          </div>
        </div>

        <div className="muraja__tafasil">
          <Mashhad miftah={wajhTafasil} className="muraja__mashhad-tafasil">
          {mukhtar === null ? (
            <HalatFarigha
              shasha
              unwan={t('muraja.afal.unwan', lugha)}
              nass={t('muraja.tafasil.la_ikhtiyar', lugha)}
            >
              <button
                type="button"
                className="zir"
                aria-disabled={sufuf.length === 0}
                title={sufuf.length === 0 ? t('muraja.tabur.la_saff', lugha) : undefined}
                onClick={() => {
                  const saf = sufuf.at(0);
                  if (saf !== undefined) {
                    setMukhtar(saf.ruqaa);
                  }
                }}
              >
                {t('muraja.lawha.iftah_awwal', lugha)}
              </button>
            </HalatFarigha>
          ) : tafasil.isPending ? (
            <HaykalTafasil />
          ) : tafasil.error !== null ? (
            <KutlatKhata
              unwan={t('muraja.khata.tafasil', lugha)}
              khata={tafasil.error}
              lugha={lugha}
              aada={() => {
                void tafasil.refetch();
              }}
            />
          ) : bayanat === undefined ? null : (
            <>
              <section className="muraja__qism muraja__qism--ras">
                <h2 className="muraja__unwan-qism">{bayanat.musawwada.unwan}</h2>
                <p className="muraja__ras-wasf">
                  <span className="muraja__ras-luba">{bayanat.musawwada.ism_luba}</span>
                  <span className="muraja__nuqta" aria-hidden="true" />
                  <span>{bayanat.musawwada.hala_arabi}</span>
                  <span className="muraja__nuqta" aria-hidden="true" />
                  <span>{bayanat.musawwada.tareeqa_arabi}</span>
                </p>
                <p className="muraja__sharh">{bayanat.musawwada.sharh}</p>
                {bayanat.musawwada.taghyeerat !== '' ? (
                  <p className="muraja__sharh">
                    <span className="muraja__tasmiya">
                      {t('muraja.tafasil.taghyeerat', lugha)}
                    </span>
                    {bayanat.musawwada.taghyeerat}
                  </p>
                ) : null}
                <p className="muraja__sumaa">
                  {t('muraja.sumaa', lugha, {
                    manshura: munassiq.raqm(bayanat.sumaa.manshura),
                    nazeefa: munassiq.raqm(bayanat.sumaa.qubila_bila_taadil),
                    taadil: munassiq.raqm(bayanat.sumaa.tulib_taadil),
                    marfuda: munassiq.raqm(bayanat.sumaa.marfuda),
                    masbuba: munassiq.raqm(bayanat.sumaa.masbuba),
                  })}
                </p>
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.tafasil.fuhusat', lugha)}</h3>
                <ul className="muraja__fuhusat">
                  {bayanat.musawwada.qaima.sutur.map((satr) => {
                    const fia = fiatFahs(satr.hala);
                    // The one id-anchored path: checklist rows carry real NassId strings.
                    const awwalNass = fia === 'akhfaqat' ? satr.nusus.at(0) : undefined;
                    return (
                      <li key={satr.band} className="muraja__fahs-satr">
                        <span className={`muraja__fahs-hala muraja__fahs-hala--${fia}`}>
                          {lugha === 'arabi' ? satr.hala_arabi : satr.hala_injilizi}
                        </span>
                        <span className="muraja__fahs-nass">
                          <span className="muraja__fahs-wasf">
                            {lugha === 'arabi' ? satr.wasf_arabi : satr.wasf_injilizi}
                          </span>
                          <span className="muraja__fahs-tafsil">
                            {lugha === 'arabi' ? satr.tafsil_arabi : satr.tafsil_injilizi}
                          </span>
                        </span>
                        {/* Emitted even when empty: the row places three cells into
                            the list's grid, and two would shift the next row. */}
                        <span className="muraja__fahs-ijra">
                          {awwalNass !== undefined ? (
                            <button
                              type="button"
                              className="zir muraja__rabt-nass"
                              onClick={() => {
                                setMirsat({ nass: awwalNass, wasm: satr.wasf_arabi });
                              }}
                            >
                              {t('muraja.taaliq.rabt_awwal', lugha)}
                            </button>
                          ) : null}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.tafasil.nusus', lugha)}</h3>
                {bayanat.sufuf.length === 0 ? (
                  <p className="muraja__nass-hadi">{t('muraja.nusus.la_mashru', lugha)}</p>
                ) : (
                  <JadwalNusus
                    sufuf={bayanat.sufuf}
                    lugha={lugha}
                    munassiq={munassiq}
                    kathafa={kathafa}
                    mukhtar={nassMukhtar ?? undefined}
                    alaIkhtiyar={alaIkhtiyarNass}
                  />
                )}
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.tafasil.tajawuz', lugha)}</h3>
                {/* The measurement first, and always: an empty overrun list is
                    "no overflow" only when everything submitted was measured.
                    Under any other verdict the list below is the answer for the
                    measured part, or for nothing at all. */}
                <p className={`muraja__qiyas muraja__qiyas--${bayanat.qiyas_tajawuz.hala}`}>
                  {lugha === 'arabi'
                    ? bayanat.qiyas_tajawuz.wasf_arabi
                    : bayanat.qiyas_tajawuz.wasf_injilizi}
                </p>
                {bayanat.tajawuz.length === 0 ? (
                  bayanat.qiyas_tajawuz.hala === 'kamil' ? (
                    <p className="muraja__nass-hadi">{t('muraja.tajawuz.la_shay', lugha)}</p>
                  ) : null
                ) : (
                  <ul className="muraja__tajawuz">
                    {bayanat.tajawuz.map((satr, fihris) => (
                      <li
                        key={`${satr.nass}-${String(fihris)}`}
                        className={`muraja__tajawuz-satr muraja__tajawuz-satr--${satr.shidda}`}
                      >
                        <span dir="rtl">{satr.nass}</span>
                        {' — '}
                        {t('muraja.tajawuz.satr', lugha, {
                          ard: munassiq.raqm(kasr(satr.ard)),
                          mutah: munassiq.raqm(kasr(satr.mutah)),
                          hajm: munassiq.raqm(kasr(satr.hajm)),
                        })}
                        <span className="muraja__tajawuz-wasf">
                          {lugha === 'arabi' ? satr.wasf_arabi : satr.wasf_injilizi}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
                {bayanat.qiyas_tajawuz.asbab.length > 0 ? (
                  <ul className="muraja__asbab-qiyas">
                    {bayanat.qiyas_tajawuz.asbab.map((sabab) => {
                      const wasf = lugha === 'arabi' ? sabab.wasf_arabi : sabab.wasf_injilizi;
                      const ilaj = lugha === 'arabi' ? sabab.ilaj_arabi : sabab.ilaj_injilizi;
                      return (
                        <li key={sabab.miftah} className="muraja__sabab-qiyas">
                          <span className="muraja__sabab-adad">{munassiq.raqm(sabab.adad)}</span>
                          <span className="muraja__sabab-jism">
                            <span>{wasf}</span>
                            {ilaj !== '' ? (
                              <span className="muraja__sabab-ilaj">{ilaj}</span>
                            ) : null}
                          </span>
                        </li>
                      );
                    })}
                  </ul>
                ) : null}
                {bayanat.ghayr_maqis.length > 0 ? (
                  <ul className="muraja__ghayr-maqis">
                    {bayanat.ghayr_maqis.map((satr, fihris) => (
                      <li key={`${satr.hawiya}-${satr.mawqi}-${String(fihris)}`}>
                        <span dir="rtl">{satr.nass}</span>
                        <span className="mono-ltr muraja__ghayr-maqis-mawqi">
                          {satr.hawiya}:{satr.mawqi}
                        </span>
                        <span className="muraja__ghayr-maqis-sabab">
                          {lugha === 'arabi' ? satr.tasnif_arabi : satr.tasnif_injilizi}
                          {' — '}
                          {lugha === 'arabi' ? satr.sabab_arabi : satr.sabab_injilizi}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.tafasil.farq', lugha)}</h3>
                {bayanat.farq_sabiq === null ? (
                  <p className="muraja__nass-hadi">{t('muraja.farq.la_shay', lugha)}</p>
                ) : (
                  <p className="muraja__sharh">{bayanat.farq_sabiq}</p>
                )}
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('warsha.taaliq.unwan', lugha)}</h3>
                {bayanat.taaliqat.length === 0 ? (
                  <p className="muraja__nass-hadi">{t('muraja.taaliq.la_shay', lugha)}</p>
                ) : (
                  <ul className="muraja__taaliqat">
                    {bayanat.taaliqat.map((wahid, fihris) => (
                      <li key={`${wahid.waqt}-${String(fihris)}`} className="muraja__taaliq">
                        <span className="muraja__taaliq-ras">
                          <span className="mono-ltr">{wahid.kaatib}</span>
                          {wahid.min_almalik ? (
                            <span className="muraja__taaliq-malik">
                              {t('warsha.taaliq.malik', lugha)}
                            </span>
                          ) : null}
                          {wahid.muhall ? <span>{t('warsha.taaliq.muhall', lugha)}</span> : null}
                        </span>
                        <span className="muraja__taaliq-matn">{wahid.matn}</span>
                      </li>
                    ))}
                  </ul>
                )}
                <div className="muraja__taaliq-talab">
                  <Zuhur maftuh={mirsat !== null || nassMukhtar !== null} className="muraja__mirsat">
                    <button
                      type="button"
                      className={
                        mirsat !== null || rabt
                          ? 'muraja__riqaqa muraja__riqaqa--faal'
                          : 'muraja__riqaqa'
                      }
                      aria-pressed={mirsat !== null || rabt}
                      onClick={() => {
                        if (mirsat !== null) {
                          setMirsat(null);
                          return;
                        }
                        setRabt((hali) => !hali);
                      }}
                    >
                      {mirsat !== null
                        ? mirsat.wasm
                        : rabt
                          ? t('muraja.taaliq.murtabit', lugha)
                          : t('muraja.taaliq.aam', lugha)}
                    </button>
                  </Zuhur>
                  <input
                    className="muraja__haql"
                    dir="rtl"
                    placeholder={t('muraja.taaliq.mawdi', lugha)}
                    value={matn}
                    onChange={(hadath) => {
                      setMatn(hadath.target.value);
                    }}
                  />
                  <button
                    type="button"
                    className="zir"
                    aria-disabled={taaliq.isPending || matn.trim() === ''}
                    aria-busy={taaliq.isPending}
                    title={matn.trim() === '' ? t('muraja.taaliq.matlub', lugha) : undefined}
                    onClick={() => {
                      if (!taaliq.isPending && matn.trim() !== '') {
                        taaliq.mutate({ matn });
                      }
                    }}
                  >
                    {t('muraja.taaliq.arsil', lugha)}
                  </button>
                </div>
                {taaliq.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={taaliq.error}
                    lugha={lugha}
                  />
                ) : null}
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.sandooq.unwan', lugha)}</h3>
                <div className="muraja__saff-afal">
                  <button
                    type="button"
                    className="zir"
                    aria-disabled={sandooqThabbit.isPending}
                    aria-busy={sandooqThabbit.isPending}
                    onClick={() => {
                      if (!sandooqThabbit.isPending) {
                        sandooqThabbit.mutate();
                      }
                    }}
                  >
                    {t(
                      sandooqThabbit.isPending
                        ? 'muraja.sandooq.jari'
                        : 'muraja.sandooq.thabbit',
                      lugha,
                    )}
                  </button>
                  <button
                    type="button"
                    className="zir"
                    aria-disabled={sandooqAtliq.isPending}
                    aria-busy={sandooqAtliq.isPending}
                    onClick={() => {
                      if (!sandooqAtliq.isPending) {
                        sandooqAtliq.mutate();
                      }
                    }}
                  >
                    {t('muraja.sandooq.atliq', lugha)}
                  </button>
                  <button
                    type="button"
                    className="zir"
                    aria-disabled={sandooqImsah.isPending}
                    aria-busy={sandooqImsah.isPending}
                    onClick={() => {
                      if (!sandooqImsah.isPending) {
                        sandooqImsah.mutate();
                      }
                    }}
                  >
                    {t('muraja.sandooq.imsah', lugha)}
                  </button>
                </div>
                <Zuhur maftuh={sandooq !== null} className="muraja__nass-hadi" role="status">
                  {sandooq === null ? null : sandooq.hasila_arabi}
                </Zuhur>
                {sandooqThabbit.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={sandooqThabbit.error}
                    lugha={lugha}
                  />
                ) : null}
                {sandooqAtliq.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={sandooqAtliq.error}
                    lugha={lugha}
                  />
                ) : null}
                {sandooqImsah.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={sandooqImsah.error}
                    lugha={lugha}
                  />
                ) : null}
              </section>

              <section className="muraja__qism muraja__qism--qarar">
                <h3 className="muraja__unwan-farii">{t('muraja.afal.unwan', lugha)}</h3>
                <div className="muraja__haqli">
                  <label className="muraja__tasmiya" htmlFor="muraja-sabab">
                    {t('muraja.afal.sabab', lugha)}
                  </label>
                  <textarea
                    id="muraja-sabab"
                    className="muraja__haql muraja__sabab"
                    dir="rtl"
                    value={sabab}
                    onChange={(hadath) => {
                      setSabab(hadath.target.value);
                    }}
                  />
                </div>
                <div className="muraja__saff-afal">
                  <button
                    type="button"
                    className="zir zir--tamyeez muraja__qarar--iaatimad"
                    aria-disabled={iaatimad.isPending}
                    aria-busy={iaatimad.isPending}
                    onClick={() => {
                      if (!iaatimad.isPending) {
                        iftahTaakid({ nitaq: 'wahid', ijra: 'iaatimad' });
                      }
                    }}
                  >
                    {t('muraja.afal.iaatimad', lugha)}
                  </button>
                  <button
                    type="button"
                    className="zir"
                    aria-disabled={qarar.isPending || sabab.trim() === ''}
                    aria-busy={qarar.isPending && qarar.variables?.ijra === 'talab_taadil'}
                    title={sabab.trim() === '' ? t('muraja.afal.sabab_matlub', lugha) : undefined}
                    onClick={() => {
                      if (!qarar.isPending && sabab.trim() !== '') {
                        qarar.mutate({ ruqaa: mukhtar, ijra: 'talab_taadil' });
                      }
                    }}
                  >
                    {t('muraja.afal.talab_taadil', lugha)}
                  </button>
                  {/* The two irreversible decisions live at the far end, behind a
                      rule: hue alone is one mis-click away from the two above. */}
                  <span className="muraja__fasil-afal" aria-hidden="true" />
                  <button
                    type="button"
                    className="zir zir--khatar muraja__qarar--rafd"
                    aria-disabled={qarar.isPending || sabab.trim() === ''}
                    aria-busy={qarar.isPending && qarar.variables?.ijra === 'rafd'}
                    title={sabab.trim() === '' ? t('muraja.afal.sabab_matlub', lugha) : undefined}
                    onClick={() => {
                      if (!qarar.isPending && sabab.trim() !== '') {
                        iftahTaakid({ nitaq: 'wahid', ijra: 'rafd' });
                      }
                    }}
                  >
                    {t('muraja.afal.rafd', lugha)}
                  </button>
                  <button
                    type="button"
                    className="zir zir--khatar muraja__qarar--rafd"
                    aria-disabled={qarar.isPending || sabab.trim() === ''}
                    aria-busy={qarar.isPending && qarar.variables?.ijra === 'sahb'}
                    title={sabab.trim() === '' ? t('muraja.afal.sabab_matlub', lugha) : undefined}
                    onClick={() => {
                      if (!qarar.isPending && sabab.trim() !== '') {
                        iftahTaakid({ nitaq: 'wahid', ijra: 'sahb' });
                      }
                    }}
                  >
                    {t('muraja.afal.sahb', lugha)}
                  </button>
                </div>
                <Zuhur
                  maftuh={taakid?.nitaq === 'wahid'}
                  className="muraja__taakid"
                  role="alertdialog"
                  aria-labelledby="muraja-taakid-unwan"
                  aria-describedby="muraja-taakid-nass"
                >
                  {taakid === null || taakid.nitaq !== 'wahid' ? null : (
                    <>
                      <p id="muraja-taakid-unwan" className="muraja__taakid-unwan">
                        {t('muraja.taakid.unwan', lugha)}
                      </p>
                      <p id="muraja-taakid-nass" className="muraja__taakid-nass">
                        {t(MIFTAH_TAAKID[taakid.ijra], lugha, {
                          unwan: bayanat.musawwada.unwan,
                        })}
                      </p>
                      {/* The reason is read back because it is what gets written
                          into the log, and a typo there is permanent. */}
                      {taakid.ijra === 'iaatimad' ? null : (
                        <p className="muraja__taakid-sabab" dir="rtl">
                          {sabab}
                        </p>
                      )}
                      <div className="muraja__taakid-azrar">
                        <button type="button" className="zir" autoFocus onClick={aghliqTaakid}>
                          {t('muraja.taakid.ilgha', lugha)}
                        </button>
                        <button
                          type="button"
                          className={
                            taakid.ijra === 'iaatimad'
                              ? 'zir zir--tamyeez muraja__qarar--iaatimad'
                              : 'zir zir--khatar muraja__qarar--rafd'
                          }
                          onClick={naffidhTaakid}
                        >
                          {t(MIFTAH_ZIR_QARAR[taakid.ijra], lugha)}
                        </button>
                      </div>
                    </>
                  )}
                </Zuhur>
                {qarar.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={qarar.error}
                    lugha={lugha}
                  />
                ) : null}
                {iaatimad.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={iaatimad.error}
                    lugha={lugha}
                  />
                ) : null}
                <Zuhur maftuh={iaatimad.data !== undefined} className="muraja__najah" role="status">
                  {iaatimad.data === undefined ? null : t('muraja.afal.nushirat', lugha)}
                </Zuhur>
              </section>

              <section className="muraja__qism">
                <h3 className="muraja__unwan-farii">{t('muraja.sijill.li_hadha', lugha)}</h3>
                {bayanat.sijill.length === 0 ? (
                  <p className="muraja__nass-hadi">{t('muraja.sijill.farigh', lugha)}</p>
                ) : (
                  <ul className="muraja__sijill">
                    {bayanat.sijill.map((satr, fihris) => (
                      <li key={`${satr.waqt}-${String(fihris)}`}>
                        <span className="muraja__sijill-waqt" dir="ltr">
                          {satr.waqt}
                        </span>
                        <span className="muraja__sijill-nass">
                          {satr.wasf_arabi}
                          {satr.sabab !== null ? `: ${satr.sabab}` : ''}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </>
          )}
          </Mashhad>
        </div>
      </div>
      </div>
      </Mashhad>
    </div>
  );
}
