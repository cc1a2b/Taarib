import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { Link, getRouteApi } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { useTaraju } from '@/hayat/taraju';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { munassiqat, t } from '@/lugha/lugha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type {
  Idadat,
  IfsahHie,
  Lugha,
  MadkhalQiraHie,
  ManatiqHie,
  MintaqaHie,
  NizamArqam,
  QaidaHie,
  SijillQiraHie,
} from '@/mustalahat/awamir';

import './tabaqa.css';

/** شاشة الطبقة — the disclosure gate, then the capture regions and the reading history. */

const wajihat = getRouteApi('/tabaqa/$muarrif');

const HADD_SIJILL = 200;

const QAWAID: readonly QaidaHie[] = ['taghayyur', 'muaqqit', 'yadawi'];

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

interface KhasaisMintaqa {
  readonly mintaqa: MintaqaHie;
  readonly iftiradiya: number;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly yajri: boolean;
  readonly alaTabdil: (mintaqa: MintaqaHie) => void;
  readonly alaHadhf: (mintaqa: MintaqaHie) => void;
}

function SaffMintaqa({
  mintaqa,
  iftiradiya,
  lugha,
  munassiq,
  yajri,
  alaTabdil,
  alaHadhf,
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
  return (
    <li className={`tabaqa__mintaqa ${halat}`}>
      <label className="tabaqa__mumakkana">
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
      <button
        type="button"
        className="zir zir--khatar"
        aria-disabled={yajri}
        onClick={() => {
          if (!yajri) {
            alaHadhf(mintaqa);
          }
        }}
      >
        {t('tabaqa.mintaqa.hadhf', lugha)}
      </button>
    </li>
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

  const sajjilKhatwa = useTaraju((halat) => halat.sajjil);

  const iqrar = useMutation<IfsahHie, KhataJisr, void>({
    mutationFn: () => nadi('aqirr_ifsah'),
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.ifsah, bayanat);
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
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
      setIsmJadeed('');
      setMustatil(MUSTATIL_FARIGH);
      setQaidaJadeeda('taghayyur');
      setFasilaJadeeda('');
    },
  });

  const tahdith = useMutation<ManatiqHie, KhataJisr, MintaqaHie>({
    mutationFn: (mintaqa) =>
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
    onSuccess: (bayanat) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
    },
  });

  const hadhf = useMutation<ManatiqHie, KhataJisr, MintaqaHie>({
    mutationFn: (mintaqa) => nadi('ihdhif_mintaqa', { muarrif, raqm: mintaqa.raqm }),
    onSuccess: (bayanat, mintaqa) => {
      makhzan.setQueryData(mafatih.manatiq(muarrif), bayanat);
      sajjilKhatwa({
        wasf: t('tabaqa.taraju.hadhf', lugha, { ism: mintaqa.ism }),
        taraju: async (): Promise<void> => {
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
        },
      });
    },
  });

  const mash = useMutation<boolean, KhataJisr, void>({
    mutationFn: () => nadi('imsah_sijill_qira', { muarrif }),
    onSuccess: () => {
      setTaakidMash(false);
      void makhzan.invalidateQueries({ queryKey: mafatih.sijill_qira(muarrif) });
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
        nafidh: () => {
          if (muqarrRef.current) {
            setTaakidMash(true);
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

  const namudhajSalih =
    ismJadeed.trim() !== '' &&
    kasrSalih(mustatil.yasar, true) &&
    kasrSalih(mustatil.aala, true) &&
    kasrSalih(mustatil.ard, false) &&
    kasrSalih(mustatil.irtifa, false) &&
    fasilaSaliha(fasilaJadeeda);

  const yajriSaff = tahdith.isPending || hadhf.isPending;
  const yuhammil = idadat.isPending || ifsah.isPending;
  const khataBawwaba = ifsah.error ?? idadat.error;
  const bayanatManatiq = manatiq.data;

  return (
    <div className="tabaqa">
      <header className="tabaqa__shareet-alawi">
        <Link to="/luba/$muarrif" params={{ muarrif }} className="tabaqa__raji">
          {t('tabaqa.raji', lugha)}
        </Link>
        <span className="tabaqa__fasl">{t('shasha.tabaqa', lugha)}</span>
        {bayanatManatiq !== undefined ? (
          <span className="tabaqa__unwan-luba">{bayanatManatiq.ism_luba}</span>
        ) : null}
      </header>

      {yuhammil ? (
        <div className="tabaqa__jism">
          <p className="tabaqa__jari">{t('amm.tahmil', lugha)}</p>
        </div>
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
                <p className="tabaqa__jari">{t('amm.tahmil', lugha)}</p>
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
                <p className="tabaqa__faragh">{t('tabaqa.manatiq.faragh', lugha)}</p>
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
                        alaTabdil={(muaddala) => {
                          tahdith.mutate(muaddala);
                        }}
                        alaHadhf={(mahdhufa) => {
                          hadhf.mutate(mahdhufa);
                        }}
                      />
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
                <p className="tabaqa__jari">{t('amm.tahmil', lugha)}</p>
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
                <p className="tabaqa__faragh">{t('tabaqa.sijill.faragh', lugha)}</p>
              ) : (
                <>
                  <ul className="tabaqa__sijill">
                    {sijill.data.sutur.map((madkhal) => (
                      <SaffQira key={madkhal.raqm} madkhal={madkhal} lugha={lugha} />
                    ))}
                  </ul>
                  {taakidMash ? (
                    <div className="tabaqa__mash">
                      <p className="tabaqa__tahdheer">{t('tabaqa.sijill.mash_taakid', lugha)}</p>
                      <div className="tabaqa__mash-afal">
                        <button
                          type="button"
                          className="zir zir--khatar"
                          aria-disabled={mash.isPending}
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
                        <button
                          type="button"
                          className="zir"
                          onClick={() => {
                            setTaakidMash(false);
                          }}
                        >
                          {t('tabaqa.sijill.mash_ilgha', lugha)}
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="tabaqa__mash-afal">
                      <button
                        type="button"
                        className="zir"
                        onClick={() => {
                          setTaakidMash(true);
                        }}
                      >
                        {t('tabaqa.sijill.mash', lugha)}
                      </button>
                    </div>
                  )}
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
            {ifsah.data.waqt !== null ? (
              <p className="tabaqa__muqarr">
                {t('tabaqa.ifsah.muqarr_mundhu', lugha)}{' '}
                <span className="mono-ltr">{ifsah.data.waqt}</span>
              </p>
            ) : null}
          </aside>
        </div>
      )}

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
