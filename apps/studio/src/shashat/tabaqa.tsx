import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { getRouteApi } from '@tanstack/react-router';
import type { JSX, ReactNode } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { ansha } from '@/hayat/tanbihat';
import { useTaraju } from '@/hayat/taraju';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { munassiqat, t } from '@/lugha/lugha';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import { Zuhur } from '@/mukawwinat/zuhur';
import type {
  Idadat,
  IfsahHie,
  Lugha,
  MadkhalQiraHie,
  ManatiqHie,
  MintaqaHie,
  NizamArqam,
  QaidaHie,
  RaqmMintaqa,
  SijillQiraHie,
} from '@/mustalahat/awamir';

import './tabaqa.css';

/** شاشة الطبقة — the disclosure gate, then the capture regions and the reading history. */

const wajihat = getRouteApi('/tabaqa/$muarrif');

const HADD_SIJILL = 200;

/** How many region rows and history entries the placeholder stands in for. */
const SUFUF_HAYKAL = 3;

const QAWAID: readonly QaidaHie[] = ['taghayyur', 'muaqqit', 'yadawi'];

/** The region table drawn empty: rows at their own height, a bar in each cell. */
function HaykalManatiq(): JSX.Element {
  return (
    <div className="tabaqa__jadwal zuhur-muakhkhar" aria-hidden="true">
      <ul className="tabaqa__manatiq">
        {Array.from({ length: SUFUF_HAYKAL }, (_, fihris) => (
          <li key={fihris} className="tabaqa__mintaqa">
            <span className="tabaqa__haykal-murabba" />
            <span className="tabaqa__mintaqa-tarif">
              <span className="tabaqa__haykal-satr tabaqa__haykal-satr--ism" />
              <span className="tabaqa__haykal-satr tabaqa__haykal-satr--qaida" />
            </span>
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--raqm" />
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--raqm" />
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--raqm" />
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--raqm" />
            <span className="tabaqa__haykal-zir" />
          </li>
        ))}
      </ul>
    </div>
  );
}

/** The reading history drawn empty: three entries at the entry's own three lines. */
function HaykalSijill(): JSX.Element {
  return (
    <ul className="tabaqa__sijill zuhur-muakhkhar" aria-hidden="true">
      {Array.from({ length: SUFUF_HAYKAL }, (_, fihris) => (
        <li key={fihris} className="tabaqa__madkhal">
          <span className="tabaqa__haykal-satr tabaqa__haykal-satr--nass" />
          <span className="tabaqa__haykal-satr tabaqa__haykal-satr--arabi" />
          <span className="tabaqa__haykal-satr tabaqa__haykal-satr--waqt" />
        </li>
      ))}
    </ul>
  );
}

/**
 * The two columns drawn empty: the region table's rows at their own height,
 * the history's entries at theirs, in the same grid, so the answer lands
 * where the placeholders stood. The gate is the other body this can become,
 * and it is a page of reading text; the columns are the shape the screen
 * settles into, so they are the shape it waits in.
 */
function HaykalTabaqa(): JSX.Element {
  return (
    <div className="tabaqa__amida zuhur-muakhkhar" aria-hidden="true">
      <div className="tabaqa__amud">
        <section className="tabaqa__qism">
          <div className="tabaqa__qism-raas">
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--unwan" />
          </div>
          <span className="tabaqa__haykal-satr tabaqa__haykal-satr--nass" />
          <HaykalManatiq />
        </section>
      </div>
      <aside className="tabaqa__janib">
        <section className="tabaqa__qism">
          <div className="tabaqa__qism-raas">
            <span className="tabaqa__haykal-satr tabaqa__haykal-satr--unwan" />
          </div>
          <HaykalSijill />
        </section>
      </aside>
    </div>
  );
}

/** A chevron on the disclosure's re-read control; it turns when the text is open. */
function RamzSuqut(): JSX.Element {
  return (
    <svg
      className="tabaqa__ifsah-ramz"
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

const MIFTAH_QAIDA: Readonly<Record<QaidaHie, MiftahLugha>> = {
  taghayyur: 'tabaqa.qaida.taghayyur',
  muaqqit: 'tabaqa.qaida.muaqqit',
  yadawi: 'tabaqa.qaida.yadawi',
};

/**
 * Whether a `<select>` value is one of the rules the backend defines.
 *
 * The slug is a generated enum now, so the narrowing is checked against the
 * Rust side rather than asserted: adding a fourth rule breaks the record above
 * at compile time, and this guard follows it without being edited.
 */
function huwaQaida(khaam: string): khaam is QaidaHie {
  return QAWAID.some((qaida) => qaida === khaam);
}

type DilaMustatil = 'yasar' | 'aala' | 'ard' | 'irtifa';

const HUQUL_MUSTATIL: readonly { readonly dila: DilaMustatil; readonly tasmiya: MiftahLugha }[] = [
  { dila: 'yasar', tasmiya: 'tabaqa.idafa.yasar' },
  { dila: 'aala', tasmiya: 'tabaqa.idafa.aala' },
  { dila: 'ard', tasmiya: 'tabaqa.idafa.ard' },
  { dila: 'irtifa', tasmiya: 'tabaqa.idafa.irtifa' },
];

const MUSTATIL_FARIGH: Readonly<Record<DilaMustatil, string>> = {
  yasar: '',
  aala: '',
  ard: '',
  irtifa: '',
};

function kasrSalih(khaam: string, sifrMasmuh: boolean): boolean {
  if (khaam.trim() === '') {
    return false;
  }
  const qeema = Number(khaam);
  if (!Number.isFinite(qeema) || qeema < 0 || qeema > 1) {
    return false;
  }
  return sifrMasmuh || qeema > 0;
}

function fasilaSaliha(khaam: string): boolean {
  if (khaam.trim() === '') {
    return true;
  }
  const qeema = Number(khaam);
  return Number.isInteger(qeema) && qeema > 0;
}

/** The region's shape as the form holds it: text, until it is checked and sent. */
interface ShaklMintaqa {
  readonly ism: string;
  readonly mustatil: Readonly<Record<DilaMustatil, string>>;
  readonly qaida: QaidaHie;
  readonly fasila: string;
}

function shaklSalih(shakl: ShaklMintaqa): boolean {
  return (
    shakl.ism.trim() !== '' &&
    kasrSalih(shakl.mustatil.yasar, true) &&
    kasrSalih(shakl.mustatil.aala, true) &&
    kasrSalih(shakl.mustatil.ard, false) &&
    kasrSalih(shakl.mustatil.irtifa, false) &&
    fasilaSaliha(shakl.fasila)
  );
}

/** A stored region as the editor first shows it; a null edge shows as an empty field. */
function shaklMin(mintaqa: MintaqaHie): ShaklMintaqa {
  const nass = (qeema: number | null): string => (qeema === null ? '' : String(qeema));
  return {
    ism: mintaqa.ism,
    mustatil: {
      yasar: nass(mintaqa.yasar),
      aala: nass(mintaqa.aala),
      ard: nass(mintaqa.ard),
      irtifa: nass(mintaqa.irtifa),
    },
    qaida: mintaqa.qaida,
    fasila: mintaqa.fasila_milli === null ? '' : String(mintaqa.fasila_milli),
  };
}

/** The region being edited in place: which one, and what the fields hold. */
interface Muharrar extends ShaklMintaqa {
  readonly raqm: RaqmMintaqa;
}

/** Why a region is being rewritten, which decides what the notice says. */
type SababTahdith = 'tabdil' | 'tahreer';

interface TahdithMintaqa {
  readonly mintaqa: MintaqaHie;
  readonly sabab: SababTahdith;
}

interface KhasaisMintaqa {
  readonly mintaqa: MintaqaHie;
  readonly iftiradiya: number;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly yajri: boolean;
  /** Whether this row's own toggle is the write in flight. */
  readonly mashghulTabdil: boolean;
  /** Whether this row's own deletion is the write in flight. */
  readonly mashghulHadhf: boolean;
  /** Whether this row's editor is open. */
  readonly muharrar: boolean;
  readonly alaTabdil: (mintaqa: MintaqaHie) => void;
  readonly alaTahreer: (mintaqa: MintaqaHie) => void;
  readonly alaHadhf: (mintaqa: MintaqaHie) => void;
  /** The editor, when this row has one open; it spans the row's whole width. */
  readonly children?: ReactNode;
}

function SaffMintaqa({
  mintaqa,
  iftiradiya,
  lugha,
  munassiq,
  yajri,
  mashghulTabdil,
  mashghulHadhf,
  muharrar,
  alaTabdil,
  alaTahreer,
  alaHadhf,
  children,
}: KhasaisMintaqa): JSX.Element {
  const mustatil = t('tabaqa.mintaqa.mustatil', lugha, {
    yasar: munassiq.nisba(kasr(mintaqa.yasar)),
    aala: munassiq.nisba(kasr(mintaqa.aala)),
    ard: munassiq.nisba(kasr(mintaqa.ard)),
    irtifa: munassiq.nisba(kasr(mintaqa.irtifa)),
  });
  const fasila =
    mintaqa.fasila_milli !== null
      ? t('tabaqa.mintaqa.fasila', lugha, { adad: munassiq.raqm(mintaqa.fasila_milli) })
      : mintaqa.qaida === 'muaqqit'
        ? t('tabaqa.mintaqa.fasila_iftiradiya', lugha, { adad: munassiq.raqm(iftiradiya) })
        : null;
  const halat = mintaqa.mumakkana ? 'tabaqa__mintaqa--faala' : 'tabaqa__mintaqa--sakina';
  const sababRafd = yajri ? t('tabaqa.mintaqa.yajri', lugha) : undefined;
  return (
    <li className={`tabaqa__mintaqa ${halat}`}>
      <label className="tabaqa__mumakkana" aria-busy={mashghulTabdil} title={sababRafd}>
        <input
          type="checkbox"
          checked={mintaqa.mumakkana}
          disabled={yajri}
          onChange={() => {
            alaTabdil({ ...mintaqa, mumakkana: !mintaqa.mumakkana });
          }}
        />
        <span className="khafi">{t('tabaqa.mintaqa.mumakkana', lugha)}</span>
      </label>
      <div className="tabaqa__mintaqa-tarif">
        <span className="tabaqa__mintaqa-ism" dir="auto">
          {mintaqa.ism}
        </span>
        <span className="tabaqa__mintaqa-qaida">
          {t(MIFTAH_QAIDA[mintaqa.qaida], lugha)}
          {fasila !== null ? ` · ${fasila}` : ''}
        </span>
      </div>
      {/* The four numbers read down the list as columns; the sentence form is
          kept beside them for anything that cannot see a column. */}
      <span className="khafi">{mustatil}</span>
      <span className="tabaqa__raqm" aria-hidden="true">
        {munassiq.nisba(kasr(mintaqa.yasar))}
      </span>
      <span className="tabaqa__raqm" aria-hidden="true">
        {munassiq.nisba(kasr(mintaqa.aala))}
      </span>
      <span className="tabaqa__raqm" aria-hidden="true">
        {munassiq.nisba(kasr(mintaqa.ard))}
      </span>
      <span className="tabaqa__raqm" aria-hidden="true">
        {munassiq.nisba(kasr(mintaqa.irtifa))}
      </span>
      <span className="tabaqa__mintaqa-afal">
        <button
          type="button"
          className="zir"
          aria-disabled={yajri}
          aria-expanded={muharrar}
          title={sababRafd}
          onClick={() => {
            if (!yajri) {
              alaTahreer(mintaqa);
            }
          }}
        >
          {t('tabaqa.tahreer.zir', lugha)}
        </button>
        <button
          type="button"
          className="zir zir--khatar"
          aria-disabled={yajri}
          aria-busy={mashghulHadhf}
          title={sababRafd}
          onClick={() => {
            if (!yajri) {
              alaHadhf(mintaqa);
            }
          }}
        >
          {t('tabaqa.mintaqa.hadhf', lugha)}
        </button>
      </span>
      {children}
    </li>
  );
}

interface KhasaisMuharrir {
  readonly muharrar: Muharrar;
  readonly lugha: Lugha;
  /** Whether the save is the write in flight. */
  readonly mashghul: boolean;
  readonly alaTaghyeer: (shakl: Muharrar) => void;
  readonly alaHifz: () => void;
  readonly alaIlgha: () => void;
}

/**
 * The in-place editor: the same fields the add form has, over the row they
 * describe, so a region's geometry is corrected where it is read rather than
 * by deleting it and typing four numbers again from memory.
 */
function MuharrirMintaqa({
  muharrar,
  lugha,
  mashghul,
  alaTaghyeer,
  alaHifz,
  alaIlgha,
}: KhasaisMuharrir): JSX.Element {
  const salih = shaklSalih(muharrar);
  const asas = `tabaqa-tahreer-${String(muharrar.raqm)}`;
  return (
    <div className="tabaqa__tahreer-namudhaj">
      <div className="tabaqa__haql-majmua">
        <label className="tabaqa__tasmiya" htmlFor={`${asas}-ism`}>
          {t('tabaqa.idafa.ism', lugha)}
        </label>
        <input
          id={`${asas}-ism`}
          className="tabaqa__haql"
          dir="auto"
          autoFocus
          value={muharrar.ism}
          onChange={(hadath) => {
            alaTaghyeer({ ...muharrar, ism: hadath.target.value });
          }}
        />
      </div>
      <div className="tabaqa__arqam">
        {HUQUL_MUSTATIL.map((haql) => (
          <div key={haql.dila} className="tabaqa__haql-majmua">
            <label className="tabaqa__tasmiya" htmlFor={`${asas}-${haql.dila}`}>
              {t(haql.tasmiya, lugha)}
            </label>
            <input
              id={`${asas}-${haql.dila}`}
              className="tabaqa__haql tabaqa__haql--raqmi mono-ltr"
              type="number"
              dir="ltr"
              min="0"
              max="1"
              step="0.01"
              value={muharrar.mustatil[haql.dila]}
              onChange={(hadath) => {
                const qeema = hadath.target.value;
                const mustatil: Record<DilaMustatil, string> = { ...muharrar.mustatil };
                mustatil[haql.dila] = qeema;
                alaTaghyeer({ ...muharrar, mustatil });
              }}
            />
          </div>
        ))}
      </div>
      <div className="tabaqa__ikhtiyarat">
        <div className="tabaqa__haql-majmua">
          <label className="tabaqa__tasmiya" htmlFor={`${asas}-qaida`}>
            {t('tabaqa.idafa.qaida', lugha)}
          </label>
          <select
            id={`${asas}-qaida`}
            className="tabaqa__haql tabaqa__muntaqi"
            value={muharrar.qaida}
            onChange={(hadath) => {
              const qeema = hadath.target.value;
              if (huwaQaida(qeema)) {
                alaTaghyeer({ ...muharrar, qaida: qeema });
              }
            }}
          >
            {QAWAID.map((qaida) => (
              <option key={qaida} value={qaida}>
                {t(MIFTAH_QAIDA[qaida], lugha)}
              </option>
            ))}
          </select>
        </div>
        <div className="tabaqa__haql-majmua">
          <label className="tabaqa__tasmiya" htmlFor={`${asas}-fasila`}>
            {t('tabaqa.idafa.fasila', lugha)}
          </label>
          <input
            id={`${asas}-fasila`}
            className="tabaqa__haql tabaqa__haql--raqmi mono-ltr"
            type="number"
            dir="ltr"
            min="1"
            step="1"
            value={muharrar.fasila}
            onChange={(hadath) => {
              alaTaghyeer({ ...muharrar, fasila: hadath.target.value });
            }}
          />
        </div>
      </div>
      <div className="tabaqa__tahreer-afal">
        <button
          type="button"
          className="zir zir--tamyeez"
          aria-disabled={mashghul || !salih}
          aria-busy={mashghul}
          title={salih ? undefined : t('tabaqa.idafa.matlub', lugha)}
          onClick={() => {
            if (!mashghul && salih) {
              alaHifz();
            }
          }}
        >
          {t(mashghul ? 'tabaqa.tahreer.jari' : 'tabaqa.tahreer.hifz', lugha)}
        </button>
        <button type="button" className="zir" onClick={alaIlgha}>
          {t('tabaqa.tahreer.ilgha', lugha)}
        </button>
      </div>
    </div>
  );
}

interface KhasaisQira {
  readonly madkhal: MadkhalQiraHie;
  readonly lugha: Lugha;
}

function SaffQira({ madkhal, lugha }: KhasaisQira): JSX.Element {
  const thawani = (madkhal.mudda_milli / 1000).toFixed(1);
  return (
    <li className="tabaqa__madkhal">
      <p className="tabaqa__madkhal-nass" dir="auto">
        {madkhal.nass}
      </p>
      {madkhal.arabi !== null ? (
        <p className="tabaqa__madkhal-arabi" lang="ar" dir="rtl">
          {madkhal.arabi}
        </p>
      ) : null}
      <p className="tabaqa__madkhal-tafsil">
        <span className="tabaqa__madkhal-waqt mono-ltr">{madkhal.waqt}</span>
        {madkhal.masdar !== null ? (
          <span className="tabaqa__madkhal-masdar" dir="auto">
            {madkhal.masdar}
          </span>
        ) : null}
        <span className="tabaqa__madkhal-mudda">
          {t('tabaqa.sijill.mudda', lugha, { thawani })}
        </span>
      </p>
    </li>
  );
}

export function Tabaqa(): JSX.Element {
  const { muarrif } = wajihat.useParams();
  const makhzan = useQueryClient();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq: Munassiqat = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const ifsah = useQuery<IfsahHie, KhataJisr>({
    queryKey: mafatih.ifsah,
    queryFn: () => nadi('nass_ifsah'),
  });
  const muqarr = ifsah.data?.muqarr === true;

  const muqarrRef = useRef(false);
  useEffect(() => {
    muqarrRef.current = muqarr;
  }, [muqarr]);

  const faqarat = useMemo(
    () =>
      (ifsah.data?.nass_arabi ?? '')
        .split(/\n+/)
        .map((faqra) => faqra.trim())
        .filter((faqra) => faqra !== ''),
    [ifsah.data?.nass_arabi],
  );

  const manatiq = useQuery<ManatiqHie, KhataJisr>({
    queryKey: mafatih.manatiq(muarrif),
    queryFn: () => nadi('manatiq_luba', { muarrif }),
    enabled: muqarr,
  });

  const sijill = useQuery<SijillQiraHie, KhataJisr>({
    queryKey: mafatih.sijill_qira(muarrif),
    queryFn: () => nadi('sijill_qira_luba', { muarrif, hadd: HADD_SIJILL }),
    enabled: muqarr,
  });

  const [ismJadeed, setIsmJadeed] = useState('');
  const [mustatil, setMustatil] = useState<Readonly<Record<DilaMustatil, string>>>(MUSTATIL_FARIGH);
  const [qaidaJadeeda, setQaidaJadeeda] = useState<QaidaHie>('taghayyur');
  const [fasilaJadeeda, setFasilaJadeeda] = useState('');
  const [taakidMash, setTaakidMash] = useState(false);
  const [muharrar, setMuharrar] = useState<Muharrar | null>(null);
  const [ifsahMaftuh, setIfsahMaftuh] = useState(false);
  /** The name field of the add form, which the empty state hands focus to. */
  const haqlIsm = useRef<HTMLInputElement | null>(null);

  // The palette registers its actions once per id set, so whether there is a
  // history to clear is read through a ref that always holds the current answer.
  const fihiSijill = (sijill.data?.sutur.length ?? 0) > 0;
  const fihiSijillRef = useRef(false);
  useEffect(() => {
    fihiSijillRef.current = fihiSijill;
  }, [fihiSijill]);
  /** The control that opened the editor or the confirmation, so closing hands focus back. */
  const fatih = useRef<HTMLElement | null>(null);

  const sajjilFatih = (): void => {
    fatih.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  };

  const ardidFatih = (): void => {
    fatih.current?.focus();
    fatih.current = null;
  };

  const aghliqTahreer = (): void => {
    setMuharrar(null);
    ardidFatih();
  };

  const aghliqMash = (): void => {
    setTaakidMash(false);
    ardidFatih();
  };

  // One Escape for both sometimes-there things; the editor's own key events
  // bubble here too, which is what makes Escape inside a field cancel the edit.
  const tahreerMaftuh = muharrar !== null;
  useEffect(() => {
    if (!tahreerMaftuh && !taakidMash) {
      return undefined;
    }
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      if (hadath.key === 'Escape') {
        setMuharrar(null);
        setTaakidMash(false);
        fatih.current?.focus();
        fatih.current = null;
      }
    };
    document.addEventListener('keydown', alaMiftah);
    return () => {
      document.removeEventListener('keydown', alaMiftah);
    };
  }, [tahreerMaftuh, taakidMash]);

  const sajjilKhatwa = useTaraju((halat) => halat.sajjil);

  const iqrar = useMutation<IfsahHie, KhataJisr, void>({
    mutationFn: () => nadi('aqirr_ifsah'),
    // The body switches to the columns, but nothing on them says the record
    // was kept; the notice is what says the gate will not ask again.
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.ifsah, bayanat);
      ansha({ naw: 'najah', nass: t('tabaqa.tanbih.iqrar', lugha) });
    },
  });

  const idafa = useMutation<ManatiqHie, KhataJisr, void>({
    mutationFn: () =>
      nadi('adif_mintaqa', {
        muarrif,
        shakl: {
          ism: ismJadeed.trim(),
          yasar: Number(mustatil.yasar),
          aala: Number(mustatil.aala),
          ard: Number(mustatil.ard),
          irtifa: Number(mustatil.irtifa),
          qaida: qaidaJadeeda,
          fasila_milli: fasilaJadeeda.trim() === '' ? null : Number(fasilaJadeeda),
        },
      }),
    // The new row lands at the end of a table the form sits under, so the
    // notice is what says it landed, and under which name.
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
      ansha({ naw: 'najah', nass: t('tabaqa.tanbih.udifat', lugha, { ism: ismJadeed.trim() }) });
      setIsmJadeed('');
      setMustatil(MUSTATIL_FARIGH);
      setQaidaJadeeda('taghayyur');
      setFasilaJadeeda('');
    },
  });

  const tahdith = useMutation<ManatiqHie, KhataJisr, TahdithMintaqa>({
    mutationFn: ({ mintaqa }) =>
      nadi('haddith_mintaqa', {
        muarrif,
        raqm: mintaqa.raqm,
        shakl: {
          ism: mintaqa.ism,
          yasar: mintaqa.yasar,
          aala: mintaqa.aala,
          ard: mintaqa.ard,
          irtifa: mintaqa.irtifa,
          qaida: mintaqa.qaida,
          fasila_milli: mintaqa.fasila_milli,
        },
        mumakkana: mintaqa.mumakkana,
      }),
    // A toggle and an edit are the same command with different consequences
    // for the scheduler, so they are said differently.
    onSuccess: (bayanat, { mintaqa, sabab }) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
      if (sabab === 'tahreer') {
        aghliqTahreer();
      }
      ansha({
        naw: 'najah',
        nass: t(
          sabab === 'tahreer'
            ? 'tabaqa.tanbih.huddithat'
            : mintaqa.mumakkana
              ? 'tabaqa.tanbih.fuilat'
              : 'tabaqa.tanbih.uqifat',
          lugha,
          { ism: mintaqa.ism },
        ),
      });
    },
  });

  const hadhf = useMutation<ManatiqHie, KhataJisr, MintaqaHie>({
    mutationFn: (mintaqa) => nadi('ihdhif_mintaqa', { muarrif, raqm: mintaqa.raqm }),
    onSuccess: (bayanat, mintaqa) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
      setMuharrar((hali) => (hali?.raqm === mintaqa.raqm ? null : hali));
      sajjilKhatwa({
        wasf: t('tabaqa.taraju.hadhf', lugha, { ism: mintaqa.ism }),
        taraju: async (): Promise<void> => {
          try {
            const natija = await nadi('adif_mintaqa', {
              muarrif,
              shakl: {
                ism: mintaqa.ism,
                yasar: mintaqa.yasar,
                aala: mintaqa.aala,
                ard: mintaqa.ard,
                irtifa: mintaqa.irtifa,
                qaida: mintaqa.qaida,
                fasila_milli: mintaqa.fasila_milli,
              },
            });
            makhzan.setQueryData(mafatih.manatiq(muarrif), natija);
          } catch (khata) {
            // The undo chord's handler puts a failed step back and says
            // nothing, so this is the one place the failure can reach the
            // person who asked for it. Rethrown so the step is kept.
            if (khata instanceof KhataJisr) {
              ansha({
                naw: 'khatar',
                nass: khata.nass(lugha) ?? t('faragh.jisr', lugha),
                ramz: khata.khata?.ramz ?? khata.amr,
              });
            }
            throw khata;
          }
        },
      });
      // The row is gone from the table, so the confirmation is the only thing
      // on screen that says what happened — and it carries the way back.
      ansha({
        naw: 'najah',
        nass: t('tabaqa.tanbih.hudhifat', lugha, { ism: mintaqa.ism }),
        amal: {
          unwan: t('taraju.zirr', lugha),
          nafidh: () => {
            void useTaraju
              .getState()
              .taraju()
              .then((wasf) => {
                if (wasf !== null) {
                  ansha({ naw: 'maluma', nass: t('taraju.tamma', lugha, { wasf }) });
                }
              })
              .catch(() => {
                // Already announced by the step itself, above.
              });
          },
        },
      });
    },
  });

  const mash = useMutation<boolean, KhataJisr, void>({
    mutationFn: () => nadi('imsah_sijill_qira', { muarrif }),
    // The command answers whether there was a file to remove; a history that
    // was already empty is not a clearing, and is not announced as one.
    onSuccess: (kanat) => {
      aghliqMash();
      void makhzan.invalidateQueries({ queryKey: mafatih.sijill_qira(muarrif) });
      ansha({
        naw: kanat ? 'najah' : 'maluma',
        nass: t(kanat ? 'tabaqa.tanbih.musiha' : 'tabaqa.tanbih.la_sijill', lugha),
      });
    },
  });

  const iqrarMutate = iqrar.mutate;
  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    if (idadat.data === undefined) {
      return [];
    }
    const majal = t('shasha.tabaqa', lugha);
    return [
      {
        muarrif: 'tabaqa.inash_manatiq',
        unwan: t('tabaqa.awamir.inash', lugha),
        majal,
        nafidh: () => {
          void makhzan.invalidateQueries({ queryKey: mafatih.manatiq(muarrif) });
        },
      },
      {
        muarrif: 'tabaqa.mash_sijill',
        unwan: t('tabaqa.awamir.mash', lugha),
        majal,
        // The confirmation lives under the history, so with no history there
        // is nothing to open; the palette says so instead of doing nothing.
        nafidh: () => {
          if (!muqarrRef.current) {
            return;
          }
          if (fihiSijillRef.current) {
            setTaakidMash(true);
          } else {
            ansha({ naw: 'maluma', nass: t('tabaqa.tanbih.la_sijill', lugha) });
          }
        },
      },
      {
        muarrif: 'tabaqa.iqrar_ifsah',
        unwan: t('tabaqa.awamir.iqrar', lugha),
        majal,
        nafidh: () => {
          if (!muqarrRef.current) {
            iqrarMutate();
          }
        },
      },
    ];
  }, [idadat.data, lugha, makhzan, muarrif, iqrarMutate]);
  useSajjilAwamir(awamirShasha);

  const namudhajSalih = shaklSalih({
    ism: ismJadeed,
    mustatil,
    qaida: qaidaJadeeda,
    fasila: fasilaJadeeda,
  });

  const yajriSaff = tahdith.isPending || hadhf.isPending;
  const yuhammil = idadat.isPending || ifsah.isPending;
  const khataBawwaba = ifsah.error ?? idadat.error;
  const bayanatManatiq = manatiq.data;

  const iftahTahreer = (mintaqa: MintaqaHie): void => {
    sajjilFatih();
    setMuharrar({ raqm: mintaqa.raqm, ...shaklMin(mintaqa) });
  };

  const ihfazTahreer = (): void => {
    const asl = bayanatManatiq?.manatiq.find((mintaqa) => mintaqa.raqm === muharrar?.raqm);
    if (muharrar === null || asl === undefined || tahdith.isPending || !shaklSalih(muharrar)) {
      return;
    }
    tahdith.mutate({
      sabab: 'tahreer',
      mintaqa: {
        ...asl,
        ism: muharrar.ism.trim(),
        yasar: Number(muharrar.mustatil.yasar),
        aala: Number(muharrar.mustatil.aala),
        ard: Number(muharrar.mustatil.ard),
        irtifa: Number(muharrar.mustatil.irtifa),
        qaida: muharrar.qaida,
        fasila_milli: muharrar.fasila.trim() === '' ? null : Number(muharrar.fasila),
      },
    });
  };
  const wajh = yuhammil
    ? 'tahmil'
    : khataBawwaba !== null
      ? 'khata'
      : ifsah.data === undefined
        ? 'tahmil'
        : muqarr
          ? 'jahiz'
          : 'bawwaba';

  return (
    <div className="tabaqa">
      <RaasShasha
        rujoo={{ ila: 'luba', muarrif }}
        nassRujoo={t('tabaqa.raji', lugha)}
        unwan={t('shasha.tabaqa', lugha)}
        mawdu={bayanatManatiq?.ism_luba ?? null}
      />

      <Mashhad miftah={wajh} className="tabaqa__mashhad">
      {wajh === 'tahmil' ? (
        <HaykalTabaqa />
      ) : khataBawwaba !== null ? (
        <div className="tabaqa__jism">
          <KutlatKhata
            unwan={t('amm.khata', lugha)}
            khata={khataBawwaba}
            lugha={lugha}
            muarrif={muarrif}
            aada={() => {
              void ifsah.refetch();
              void idadat.refetch();
            }}
          />
        </div>
      ) : ifsah.data === undefined ? null : !muqarr ? (
        <div className="tabaqa__bawwaba">
          <section className="tabaqa__ifsah" aria-labelledby="tabaqa-ifsah-unwan">
            <h2 id="tabaqa-ifsah-unwan" className="tabaqa__ifsah-unwan">
              {t('tabaqa.ifsah.unwan', lugha)}
            </h2>
            <div className="tabaqa__ifsah-nass">
              {faqarat.map((faqra, fihris) => (
                <p
                  key={`${String(fihris)}-${faqra.slice(0, 24)}`}
                  className="tabaqa__ifsah-faqra"
                  lang="ar"
                  dir="rtl"
                >
                  {faqra}
                </p>
              ))}
            </div>
            <div className="tabaqa__ifsah-afal">
              <button
                type="button"
                className="zir zir--tamyeez"
                aria-disabled={iqrar.isPending}
                aria-busy={iqrar.isPending}
                onClick={() => {
                  if (!iqrar.isPending) {
                    iqrar.mutate();
                  }
                }}
              >
                {t(iqrar.isPending ? 'tabaqa.ifsah.jari' : 'tabaqa.ifsah.aqirr', lugha)}
              </button>
            </div>
            {iqrar.error !== null ? (
              <KutlatKhata
                unwan={t('luba.khata.amal', lugha)}
                khata={iqrar.error}
                lugha={lugha}
                muarrif={muarrif}
                aada={() => {
                  iqrar.mutate();
                }}
              />
            ) : null}
          </section>
        </div>
      ) : (
        <div className="tabaqa__amida">
          <div className="tabaqa__amud">
            <section className="tabaqa__qism" aria-labelledby="tabaqa-manatiq-unwan">
              <div className="tabaqa__qism-raas">
                <h2 id="tabaqa-manatiq-unwan" className="tabaqa__unwan-qism">
                  {t('tabaqa.manatiq.unwan', lugha)}
                </h2>
              </div>
              <p className="tabaqa__nass-hadi">{t('tabaqa.manatiq.talmih', lugha)}</p>
              {manatiq.isPending ? (
                <HaykalManatiq />
              ) : manatiq.error !== null ? (
                <KutlatKhata
                  unwan={t('tabaqa.khata.manatiq', lugha)}
                  khata={manatiq.error}
                  lugha={lugha}
                  muarrif={muarrif}
                  aada={() => {
                    void manatiq.refetch();
                  }}
                />
              ) : bayanatManatiq === undefined ? null : bayanatManatiq.manatiq.length === 0 ? (
                <HalatFarigha unwan={t('tabaqa.manatiq.faragh', lugha)}>
                  <button
                    type="button"
                    className="zir zir--tamyeez"
                    onClick={() => {
                      haqlIsm.current?.focus();
                    }}
                  >
                    {t('tabaqa.manatiq.adif', lugha)}
                  </button>
                </HalatFarigha>
              ) : (
                <div className="tabaqa__jadwal">
                  {/* Column headings for the geometry, not a data row: every
                      region already carries the same four numbers as a
                      sentence for anything not reading the columns. */}
                  <div className="tabaqa__manatiq-raas" aria-hidden="true">
                    <span />
                    <span className="tabaqa__amud-tasmiya">{t('tabaqa.idafa.ism', lugha)}</span>
                    {HUQUL_MUSTATIL.map((haql) => (
                      <span key={haql.dila} className="tabaqa__amud-tasmiya tabaqa__amud-raqm">
                        {t(haql.tasmiya, lugha)}
                      </span>
                    ))}
                    <span />
                  </div>
                  <ul className="tabaqa__manatiq">
                    {bayanatManatiq.manatiq.map((mintaqa) => (
                      <SaffMintaqa
                        key={mintaqa.raqm}
                        mintaqa={mintaqa}
                        iftiradiya={bayanatManatiq.fasila_iftiradiya_milli}
                        lugha={lugha}
                        munassiq={munassiq}
                        yajri={yajriSaff}
                        mashghulTabdil={
                          tahdith.isPending &&
                          tahdith.variables?.sabab === 'tabdil' &&
                          tahdith.variables.mintaqa.raqm === mintaqa.raqm
                        }
                        mashghulHadhf={hadhf.isPending && hadhf.variables?.raqm === mintaqa.raqm}
                        muharrar={muharrar?.raqm === mintaqa.raqm}
                        alaTabdil={(muaddala) => {
                          tahdith.mutate({ mintaqa: muaddala, sabab: 'tabdil' });
                        }}
                        alaTahreer={(hadaf) => {
                          if (muharrar?.raqm === hadaf.raqm) {
                            aghliqTahreer();
                          } else {
                            iftahTahreer(hadaf);
                          }
                        }}
                        alaHadhf={(mahdhufa) => {
                          hadhf.mutate(mahdhufa);
                        }}
                      >
                        <Zuhur
                          maftuh={muharrar?.raqm === mintaqa.raqm}
                          className="tabaqa__tahreer"
                          role="group"
                          aria-label={t('tabaqa.tahreer.unwan', lugha, { ism: mintaqa.ism })}
                        >
                          {muharrar === null || muharrar.raqm !== mintaqa.raqm ? null : (
                            <MuharrirMintaqa
                              muharrar={muharrar}
                              lugha={lugha}
                              mashghul={
                                tahdith.isPending && tahdith.variables?.sabab === 'tahreer'
                              }
                              alaTaghyeer={setMuharrar}
                              alaHifz={ihfazTahreer}
                              alaIlgha={aghliqTahreer}
                            />
                          )}
                        </Zuhur>
                      </SaffMintaqa>
                    ))}
                  </ul>
                </div>
              )}
              {tahdith.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={tahdith.error}
                  lugha={lugha}
                  muarrif={muarrif}
                />
              ) : null}
              {hadhf.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={hadhf.error}
                  lugha={lugha}
                  muarrif={muarrif}
                />
              ) : null}
            </section>

            <section className="tabaqa__qism" aria-labelledby="tabaqa-idafa-unwan">
              <div className="tabaqa__qism-raas">
                <h2 id="tabaqa-idafa-unwan" className="tabaqa__unwan-qism">
                  {t('tabaqa.idafa.unwan', lugha)}
                </h2>
              </div>
              <div className="tabaqa__namudhaj">
                <div className="tabaqa__haql-majmua">
                  <label className="tabaqa__tasmiya" htmlFor="tabaqa-ism">
                    {t('tabaqa.idafa.ism', lugha)}
                  </label>
                  <input
                    id="tabaqa-ism"
                    ref={haqlIsm}
                    className="tabaqa__haql"
                    dir="auto"
                    placeholder={t('tabaqa.idafa.ism_mawdi', lugha)}
                    value={ismJadeed}
                    onChange={(hadath) => {
                      setIsmJadeed(hadath.target.value);
                    }}
                  />
                </div>
                <div className="tabaqa__arqam">
                  {HUQUL_MUSTATIL.map((haql) => (
                    <div key={haql.dila} className="tabaqa__haql-majmua">
                      <label className="tabaqa__tasmiya" htmlFor={`tabaqa-${haql.dila}`}>
                        {t(haql.tasmiya, lugha)}
                      </label>
                      <input
                        id={`tabaqa-${haql.dila}`}
                        className="tabaqa__haql tabaqa__haql--raqmi mono-ltr"
                        type="number"
                        dir="ltr"
                        min="0"
                        max="1"
                        step="0.01"
                        value={mustatil[haql.dila]}
                        onChange={(hadath) => {
                          const qeema = hadath.target.value;
                          setMustatil((hali) => {
                            const jadeed: Record<DilaMustatil, string> = { ...hali };
                            jadeed[haql.dila] = qeema;
                            return jadeed;
                          });
                        }}
                      />
                    </div>
                  ))}
                </div>
                <div className="tabaqa__ikhtiyarat">
                  <div className="tabaqa__haql-majmua">
                    <label className="tabaqa__tasmiya" htmlFor="tabaqa-qaida">
                      {t('tabaqa.idafa.qaida', lugha)}
                    </label>
                    <select
                      id="tabaqa-qaida"
                      className="tabaqa__haql tabaqa__muntaqi"
                      value={qaidaJadeeda}
                      onChange={(hadath) => {
                        const qeema = hadath.target.value;
                        if (huwaQaida(qeema)) {
                          setQaidaJadeeda(qeema);
                        }
                      }}
                    >
                      {QAWAID.map((qaida) => (
                        <option key={qaida} value={qaida}>
                          {t(MIFTAH_QAIDA[qaida], lugha)}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="tabaqa__haql-majmua">
                    <label className="tabaqa__tasmiya" htmlFor="tabaqa-fasila">
                      {t('tabaqa.idafa.fasila', lugha)}
                    </label>
                    <input
                      id="tabaqa-fasila"
                      className="tabaqa__haql tabaqa__haql--raqmi mono-ltr"
                      type="number"
                      dir="ltr"
                      min="1"
                      step="1"
                      value={fasilaJadeeda}
                      onChange={(hadath) => {
                        setFasilaJadeeda(hadath.target.value);
                      }}
                    />
                  </div>
                </div>
                <div className="tabaqa__idafa-afal">
                  <button
                    type="button"
                    className="zir zir--tamyeez"
                    aria-disabled={idafa.isPending || !namudhajSalih}
                    aria-busy={idafa.isPending}
                    title={namudhajSalih ? undefined : t('tabaqa.idafa.matlub', lugha)}
                    onClick={() => {
                      if (!idafa.isPending && namudhajSalih) {
                        idafa.mutate();
                      }
                    }}
                  >
                    {t(idafa.isPending ? 'tabaqa.idafa.jari' : 'tabaqa.idafa.zir', lugha)}
                  </button>
                </div>
                {idafa.error !== null ? (
                  <KutlatKhata
                    unwan={t('luba.khata.amal', lugha)}
                    khata={idafa.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      idafa.mutate();
                    }}
                  />
                ) : null}
              </div>
            </section>
          </div>

          <aside className="tabaqa__janib">
            <section className="tabaqa__qism" aria-labelledby="tabaqa-sijill-unwan">
              <div className="tabaqa__qism-raas">
                <h2 id="tabaqa-sijill-unwan" className="tabaqa__unwan-qism">
                  {t('tabaqa.sijill.unwan', lugha)}
                </h2>
                {sijill.data !== undefined ? (
                  <span className="tabaqa__adad">
                    {t('tabaqa.sijill.adad', lugha, {
                      adad: munassiq.raqm(sijill.data.adad_kulli),
                    })}
                  </span>
                ) : null}
              </div>
              {sijill.isPending ? (
                <HaykalSijill />
              ) : sijill.error !== null ? (
                <KutlatKhata
                  unwan={t('tabaqa.khata.sijill', lugha)}
                  khata={sijill.error}
                  lugha={lugha}
                  muarrif={muarrif}
                  aada={() => {
                    void sijill.refetch();
                  }}
                />
              ) : sijill.data === undefined ? null : sijill.data.sutur.length === 0 ? (
                <HalatFarigha unwan={t('tabaqa.sijill.faragh', lugha)} />
              ) : (
                <>
                  <ul className="tabaqa__sijill">
                    {sijill.data.sutur.map((madkhal) => (
                      <SaffQira key={madkhal.raqm} madkhal={madkhal} lugha={lugha} />
                    ))}
                  </ul>
                  <div className="tabaqa__mash-afal">
                    <button
                      type="button"
                      className="zir"
                      aria-expanded={taakidMash}
                      onClick={() => {
                        if (taakidMash) {
                          aghliqMash();
                        } else {
                          sajjilFatih();
                          setTaakidMash(true);
                        }
                      }}
                    >
                      {t('tabaqa.sijill.mash', lugha)}
                    </button>
                  </div>
                  <Zuhur
                    maftuh={taakidMash}
                    className="tabaqa__mash"
                    role="alertdialog"
                    aria-labelledby="tabaqa-mash-nass"
                  >
                    <p id="tabaqa-mash-nass" className="tabaqa__tahdheer">
                      {t('tabaqa.sijill.mash_taakid', lugha)}
                    </p>
                    <div className="tabaqa__mash-afal">
                      {/* Focus lands on the way out, so Enter pressed once too
                          often cancels rather than clears. */}
                      <button type="button" className="zir" autoFocus onClick={aghliqMash}>
                        {t('tabaqa.sijill.mash_ilgha', lugha)}
                      </button>
                      <button
                        type="button"
                        className="zir zir--khatar"
                        aria-disabled={mash.isPending}
                        aria-busy={mash.isPending}
                        onClick={() => {
                          if (!mash.isPending) {
                            mash.mutate();
                          }
                        }}
                      >
                        {t(
                          mash.isPending
                            ? 'tabaqa.sijill.mash_jari'
                            : 'tabaqa.sijill.mash_tathbit',
                          lugha,
                        )}
                      </button>
                    </div>
                  </Zuhur>
                </>
              )}
              {mash.error !== null ? (
                <KutlatKhata
                  unwan={t('luba.khata.amal', lugha)}
                  khata={mash.error}
                  lugha={lugha}
                  muarrif={muarrif}
                  aada={() => {
                    mash.mutate();
                  }}
                />
              ) : null}
            </section>
            <div className="tabaqa__muqarr">
              {ifsah.data.waqt !== null ? (
                <p>
                  {t('tabaqa.ifsah.muqarr_mundhu', lugha)}{' '}
                  <span className="mono-ltr">{ifsah.data.waqt}</span>
                </p>
              ) : null}
              {/* What was agreed to stays readable after it was agreed to;
                  a disclosure that vanishes on acknowledgement is a receipt
                  nobody can check. */}
              <button
                type="button"
                className="tabaqa__ifsah-zir"
                aria-expanded={ifsahMaftuh}
                aria-controls="tabaqa-ifsah-jism"
                onClick={() => {
                  setIfsahMaftuh((hali) => !hali);
                }}
              >
                <RamzSuqut />
                {t(ifsahMaftuh ? 'tabaqa.ifsah.ighlaq' : 'tabaqa.ifsah.iftah', lugha)}
              </button>
              <Zuhur maftuh={ifsahMaftuh} id="tabaqa-ifsah-jism" className="tabaqa__ifsah-jism">
                {faqarat.map((faqra, fihris) => (
                  <p
                    key={`${String(fihris)}-${faqra.slice(0, 24)}`}
                    className="tabaqa__ifsah-faqra"
                    lang="ar"
                    dir="rtl"
                  >
                    {faqra}
                  </p>
                ))}
              </Zuhur>
            </div>
          </aside>
        </div>
      )}
      </Mashhad>

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
