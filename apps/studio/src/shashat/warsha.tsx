import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { kasr } from '@/mustalahat/arqam';
import { Link, getRouteApi } from '@tanstack/react-router';
import { useVirtualizer } from '@tanstack/react-virtual';
import { AnimatePresence, motion } from 'motion/react';
import type { JSX, KeyboardEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { listen } from '@tauri-apps/api/event';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { HADATH_TAQADDUM_DUFA, KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { useTaraju } from '@/hayat/taraju';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, munassiqat, t } from '@/lugha/lugha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type {
  AlamatMashruHie,
  DamjHie,
  DufaHie,
  Idadat,
  InqadhHie,
  IqtirahatHie,
  Lugha,
  MuayanaHie,
  NatijatTarjamaHie,
  NizamArqam,
  NizaaHie,
  NizaatHie,
  QararHie,
  SafWarshaHie,
  TaaliqWarshaHie,
  WarshaHie,
} from '@/mustalahat/awamir';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './warsha.css';

/** شاشة الورشة — the translation workspace: the table, the editor, and everything around them. */

const wajihat = getRouteApi('/warsha/$muarrif');

type HalatSaff = SafWarshaHie['hala'];

/** What the backend says about the string file's integrity, and the damage when there is any. */
type SalamaHie = WarshaHie['salama'];
type TalafHie = NonNullable<SalamaHie['talaf']>;

/**
 * One of two sentences the backend wrote, in the session's language.
 *
 * The integrity sentences are worded by the code that knows the count, the
 * path and the advice, in both languages at once, so the screen never assembles
 * them and the two languages cannot drift apart.
 */
function ikhtar(lugha: Lugha, arabi: string, injilizi: string): string {
  return lugha === 'arabi' ? arabi : injilizi;
}

const HALAT: readonly HalatSaff[] = [
  'lam_tutarjam',
  'tarjama_aaliya',
  'musawwada',
  'lil_muraja',
  'muakkada',
  'marfuda',
];

const MIFTAH_HALA: Readonly<Record<HalatSaff, MiftahLugha>> = {
  lam_tutarjam: 'warsha.hala.lam_tutarjam',
  tarjama_aaliya: 'warsha.hala.tarjama_aaliya',
  musawwada: 'warsha.hala.musawwada',
  lil_muraja: 'warsha.hala.lil_muraja',
  muakkada: 'warsha.hala.muakkada',
  marfuda: 'warsha.hala.marfuda',
};

/**
 * The row height before anything has been painted, in pixels.
 *
 * The real height is `--irtifa-satr-warsha`, which is derived from the density
 * and therefore not a number this module can know. Every rendered row reports
 * its own height back through `measureElement`, so this is the first frame's
 * estimate and nothing else — it happens to be exact at the default density.
 */
const IRTIFA_SATR_MUBDAI = 56;

type MurashshihAlamat = 'kul' | 'ay' | 'khatir';
type MurashshihMasdar = 'kul' | 'aali' | 'bashari' | 'dhakira';
type MurashshihTakleef = 'kul' | 'muayyan' | 'bila' | 'li';

interface Murashshihat {
  readonly bahth: string;
  readonly halat: readonly HalatSaff[];
  readonly alamat: MurashshihAlamat;
  readonly tasnif: string;
  readonly dakhili: boolean;
  readonly masdar: MurashshihMasdar;
  readonly takleef: MurashshihTakleef;
}

const MURASHSHIHAT_BIDAYA: Murashshihat = {
  bahth: '',
  halat: [],
  alamat: 'kul',
  tasnif: 'kul',
  dakhili: false,
  masdar: 'kul',
  takleef: 'kul',
};

function yutabiq(saf: SafWarshaHie, murashshihat: Murashshihat, musahimi: string): boolean {
  if (!murashshihat.dakhili && saf.dakhili) {
    return false;
  }
  if (murashshihat.halat.length > 0 && !murashshihat.halat.includes(saf.hala)) {
    return false;
  }
  if (murashshihat.alamat === 'ay' && saf.alamat.length === 0) {
    return false;
  }
  if (murashshihat.alamat === 'khatir' && !saf.alamat.some((alam) => alam.khatir)) {
    return false;
  }
  if (murashshihat.tasnif !== 'kul' && saf.tasnif !== murashshihat.tasnif) {
    return false;
  }
  if (murashshihat.masdar === 'aali' && saf.muzawwid === null) {
    return false;
  }
  if (murashshihat.masdar === 'dhakira' && saf.muzawwid !== 'الذاكرة') {
    return false;
  }
  if (murashshihat.masdar === 'bashari' && (saf.muzawwid !== null || saf.hadaf === null)) {
    return false;
  }
  if (murashshihat.takleef === 'muayyan' && saf.muayyan === null) {
    return false;
  }
  if (murashshihat.takleef === 'bila' && saf.muayyan !== null) {
    return false;
  }
  if (murashshihat.takleef === 'li' && saf.muayyan !== musahimi) {
    return false;
  }
  if (murashshihat.bahth !== '') {
    const ibra = murashshihat.bahth.toLowerCase();
    const fi_masdar = saf.masdar.toLowerCase().includes(ibra);
    const fi_hadaf = saf.hadaf !== null && saf.hadaf.includes(murashshihat.bahth);
    if (!fi_masdar && !fi_hadaf) {
      return false;
    }
  }
  return true;
}

interface KhasaisSaffQaima {
  readonly saf: SafWarshaHie;
  readonly mukhtar: boolean;
  readonly taaliqat: number;
  readonly lugha: Lugha;
  readonly alaIkhtiyar: () => void;
}

function SaffQaima({
  saf,
  mukhtar,
  taaliqat,
  lugha,
  alaIkhtiyar,
}: KhasaisSaffQaima): JSX.Element {
  const khatir = saf.alamat.some((alam) => alam.khatir);
  const ismHala = t(MIFTAH_HALA[saf.hala], lugha);
  return (
    <button
      type="button"
      className={mukhtar ? 'warsha__saff warsha__saff--mukhtar' : 'warsha__saff'}
      onClick={alaIkhtiyar}
      tabIndex={-1}
    >
      <span
        className={`warsha__nuqta warsha__nuqta--${saf.hala}`}
        title={ismHala}
        aria-hidden="true"
      />
      {/* The mark carries the state for the eye; this carries it for a reader. */}
      <span className="khafi">{ismHala}</span>
      <span className="warsha__saff-nusus">
        <span className="warsha__saff-masdar" dir="auto">
          {saf.masdar}
        </span>
        <span
          className={
            saf.hadaf === null
              ? 'warsha__saff-hadaf warsha__saff-hadaf--faragh'
              : 'warsha__saff-hadaf'
          }
        >
          {saf.hadaf ?? '—'}
        </span>
      </span>
      <span className="warsha__saff-mizat">
        {taaliqat > 0 ? <span className="warsha__saff-taaliq" aria-hidden="true" /> : null}
        {saf.alamat.length > 0 ? (
          <span
            className={
              khatir ? 'warsha__saff-alamat warsha__saff-alamat--khatir' : 'warsha__saff-alamat'
            }
          >
            {saf.alamat.length}
          </span>
        ) : null}
      </span>
    </button>
  );
}

/**
 * The screen it stands in for, drawn empty: the same three columns, the same
 * surfaces and hairlines, and rows of the same height, in the same grid track.
 */
function HaykalWarsha(): JSX.Element {
  return (
    <div className="warsha__haykal" aria-hidden="true">
      <div className="warsha__haykal-amud warsha__haykal-amud--qaima">
        {Array.from({ length: 14 }, (_, fihris) => (
          <div key={fihris} className="warsha__haykal-satr">
            <span className="warsha__haykal-shakhta warsha__haykal-shakhta--tawil" />
            <span className="warsha__haykal-shakhta warsha__haykal-shakhta--qasir" />
          </div>
        ))}
      </div>
      <div className="warsha__haykal-amud warsha__haykal-amud--wasat">
        <span className="warsha__haykal-kutla warsha__haykal-kutla--masdar" />
        <span className="warsha__haykal-kutla warsha__haykal-kutla--tahrir" />
      </div>
      <div className="warsha__haykal-amud warsha__haykal-amud--akhir">
        <span className="warsha__haykal-kutla warsha__haykal-kutla--janib" />
        <span className="warsha__haykal-kutla warsha__haykal-kutla--janib" />
      </div>
    </div>
  );
}

interface KhasaisNizaa {
  readonly nizaa: NizaaHie;
  readonly qarar: QararHie | undefined;
  readonly lugha: Lugha;
  readonly alaQarar: (qarar: QararHie) => void;
}

function BitaqatNizaa({ nizaa, qarar, lugha, alaQarar }: KhasaisNizaa): JSX.Element {
  const [thalith, setThalith] = useState('');
  const janib = (ism: 'ana' | 'hum'): JSX.Element => {
    const bitaqa = ism === 'ana' ? nizaa.ana : nizaa.hum;
    const makhtar =
      qarar !== undefined && qarar.qarar === (ism === 'ana' ? 'khudh_li' : 'khudh_hum');
    return (
      <div className={makhtar ? 'warsha__janib warsha__janib--makhtar' : 'warsha__janib'}>
        <p className="warsha__janib-unwan">
          {t(ism === 'ana' ? 'warsha.damj.ana' : 'warsha.damj.hum', lugha)}
        </p>
        <p className="warsha__janib-hadaf">{bitaqa.hadaf}</p>
        <dl className="warsha__janib-tafsil">
          <div>
            <dt>{t('warsha.damj.janib.hala', lugha)}</dt>
            <dd>{bitaqa.hala}</dd>
          </div>
          {bitaqa.tareeqa_arabi !== null ? (
            <div>
              <dt>{t('warsha.damj.janib.tareeqa', lugha)}</dt>
              <dd>{bitaqa.tareeqa_arabi}</dd>
            </div>
          ) : null}
          {bitaqa.muzawwid !== null ? (
            <div>
              <dt>{t('warsha.damj.janib.muzawwid', lugha)}</dt>
              <dd>{bitaqa.muzawwid}</dd>
            </div>
          ) : null}
          {bitaqa.muharrir !== null ? (
            <div>
              <dt>{t('warsha.damj.janib.muharrir', lugha)}</dt>
              <dd className="mono-ltr">{bitaqa.muharrir}</dd>
            </div>
          ) : null}
          {bitaqa.akhir_tabdeel !== null ? (
            <div>
              <dt>{t('warsha.damj.janib.waqt', lugha)}</dt>
              <dd dir="ltr">{bitaqa.akhir_tabdeel}</dd>
            </div>
          ) : null}
        </dl>
        <button
          type="button"
          className="zir"
          onClick={() => {
            alaQarar(
              ism === 'ana'
                ? { nass: nizaa.nass, qarar: 'khudh_li' }
                : { nass: nizaa.nass, qarar: 'khudh_hum' },
            );
          }}
        >
          {t(ism === 'ana' ? 'warsha.damj.khudh_li' : 'warsha.damj.khudh_hum', lugha)}
        </button>
      </div>
    );
  };

  const thalith_makhtar = qarar !== undefined && qarar.qarar === 'thalith';
  return (
    <li className="warsha__nizaa">
      <p className="warsha__nizaa-naw">
        {t(nizaa.naw === 'hala' ? 'warsha.damj.naw.hala' : 'warsha.damj.naw.tarjama', lugha)}
      </p>
      <p className="warsha__nizaa-masdar" dir="auto">
        {nizaa.masdar}
      </p>
      <div className="warsha__nizaa-ajnab">
        {janib('ana')}
        {janib('hum')}
        <div
          className={
            thalith_makhtar ? 'warsha__janib warsha__janib--makhtar' : 'warsha__janib'
          }
        >
          <p className="warsha__janib-unwan">{t('warsha.damj.thalith', lugha)}</p>
          <textarea
            className="warsha__thalith"
            dir="rtl"
            value={thalith}
            placeholder={t('warsha.damj.thalith_mawdi', lugha)}
            onChange={(hadath) => {
              setThalith(hadath.target.value);
            }}
          />
          <button
            type="button"
            className="zir"
            aria-disabled={thalith.trim() === ''}
            onClick={() => {
              if (thalith.trim() !== '') {
                alaQarar({ nass: nizaa.nass, qarar: 'thalith', hadaf: thalith });
              }
            }}
          >
            {t('warsha.damj.khudh_thalith', lugha)}
          </button>
        </div>
      </div>
    </li>
  );
}

interface KhasaisTalaf {
  readonly salama: SalamaHie;
  readonly talaf: TalafHie;
  readonly lugha: Lugha;
  readonly yajri: boolean;
  readonly khata: KhataJisr | null;
  readonly muarrif: string;
  readonly alaInqadh: () => void;
}

/**
 * The damage panel: what did not read, line by line, and the one choice that
 * writes anything. Shown whenever the string file did not read whole, whether
 * three rows survived or three thousand, because a table that looks complete
 * is the quieter of the two failures.
 */
function LawhatTalaf({
  salama,
  talaf,
  lugha,
  yajri,
  khata,
  muarrif,
  alaInqadh,
}: KhasaisTalaf): JSX.Element {
  const unwan = ikhtar(lugha, talaf.unwan_arabi, talaf.unwan_injilizi);
  return (
    <section className="warsha__lawha" role="alert" aria-label={unwan}>
      <div className="warsha__lawha-dakhil">
        <p className="warsha__tahdheer">{unwan}</p>
        <p className="warsha__nass-hadi">
          {ikhtar(lugha, salama.wasf_arabi, salama.wasf_injilizi)}
        </p>
        <p className="warsha__nass-hadi mono-ltr" dir="ltr">
          {salama.masar}
        </p>
        <ul className="warsha__jiwar">
          {talaf.sutur.map((satr) => (
            <li key={satr.raqm} dir="auto">
              {ikhtar(lugha, satr.wasf_arabi, satr.wasf_injilizi)}
            </li>
          ))}
        </ul>
        <div className="halat__afal">
          <button
            type="button"
            className="zir zir--tamyeez"
            aria-disabled={yajri}
            onClick={() => {
              if (!yajri) {
                alaInqadh();
              }
            }}
          >
            {ikhtar(lugha, talaf.zir_arabi, talaf.zir_injilizi)}
          </button>
        </div>
        {khata !== null ? (
          <KutlatKhata
            unwan={t('luba.khata.amal', lugha)}
            khata={khata}
            lugha={lugha}
            muarrif={muarrif}
          />
        ) : null}
      </div>
    </section>
  );
}

interface KhasaisIqtirahat {
  readonly bayanat: IqtirahatHie;
  readonly mutabbaq: boolean;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  readonly yajri: boolean;
  readonly alaTatbiq: (qayd: number) => void;
}

function LawhatIqtirahat({
  bayanat,
  mutabbaq,
  lugha,
  munassiq,
  yajri,
  alaTatbiq,
}: KhasaisIqtirahat): JSX.Element {
  const tamm = bayanat.tatbiq;
  return (
    <>
      {bayanat.mustalahat.length > 0 ? (
        <>
          <p className="warsha__tasmiya">{t('warsha.iqtirah.masrad', lugha)}</p>
          <ul className="warsha__mustalahat">
            {bayanat.mustalahat.map((mustalah) => (
              <li key={mustalah.masdar} className="warsha__mustalah">
                <span className="warsha__mustalah-masdar" dir="auto">
                  {mustalah.masdar}
                </span>
                {/* Drawn from logical borders, so it turns with the document. */}
                <span className="warsha__sahm" aria-hidden="true" />
                <span className="warsha__mustalah-arabi">{mustalah.arabi}</span>
                {mustalah.mulahaza !== null ? (
                  <span className="warsha__mulahaza">{mustalah.mulahaza}</span>
                ) : null}
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {tamm !== null ? (
        <div className="warsha__iqtirah warsha__iqtirah--tamm">
          <p className="warsha__iqtirah-wasm">{t('warsha.iqtirah.tatbiq', lugha)}</p>
          <p className="warsha__iqtirah-hadaf">{tamm.hadaf}</p>
          <p className="warsha__nass-hadi">
            {t(tamm.muraja_bashariya ? 'warsha.iqtirah.bashari' : 'warsha.iqtirah.aali', lugha)}
          </p>
          {mutabbaq ? (
            <p className="warsha__nass-hadi">{t('warsha.iqtirah.tilqai', lugha)}</p>
          ) : null}
          <button
            type="button"
            className="zir zir--tamyeez"
            aria-disabled={yajri}
            onClick={() => {
              if (!yajri) {
                alaTatbiq(tamm.qayd);
              }
            }}
          >
            {t('warsha.iqtirah.tatbiq_zir', lugha)}
          </button>
        </div>
      ) : null}
      {bayanat.iqtirahat.length === 0 && tamm === null && bayanat.mustalahat.length === 0 ? (
        <p className="warsha__nass-hadi">{t('warsha.iqtirah.la_shay', lugha)}</p>
      ) : null}
      {bayanat.iqtirahat.map((iqtirah) => (
        <div key={iqtirah.qayd} className="warsha__iqtirah">
          <p className="warsha__iqtirah-wasm">
            {t('warsha.iqtirah.tashabuh', lugha, {
              nisba: munassiq.raqm(iqtirah.tashabuh / 10),
            })}
            {' — '}
            {t(
              iqtirah.muraja_bashariya ? 'warsha.iqtirah.bashari' : 'warsha.iqtirah.aali',
              lugha,
            )}
          </p>
          <p className="warsha__iqtirah-masdar" dir="auto">
            {iqtirah.masdar_asli}
          </p>
          <p className="warsha__iqtirah-hadaf">{iqtirah.hadaf}</p>
          <button
            type="button"
            className="zir"
            aria-disabled={yajri}
            onClick={() => {
              if (!yajri) {
                alaTatbiq(iqtirah.qayd);
              }
            }}
          >
            {t('warsha.iqtirah.tatbiq_zir', lugha)}
          </button>
        </div>
      ))}
    </>
  );
}

interface KhasaisMuayana {
  readonly bayanat: MuayanaHie;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}

function LawhatMuayana({ bayanat, lugha, munassiq }: KhasaisMuayana): JSX.Element {
  if (bayanat.ghayr_qabil) {
    return (
      <div className="warsha__ghayr-qabil" role="status">
        <p className="warsha__ghayr-qabil-unwan">{t('warsha.muayana.ghayr_qabil', lugha)}</p>
        {bayanat.sabab_ghayr_qabil !== null ? (
          <p className="warsha__nass-hadi">{bayanat.sabab_ghayr_qabil}</p>
        ) : null}
      </div>
    );
  }
  const mutah = bayanat.mutah ?? 0;
  return (
    <>
      {bayanat.sutur.map((satr, fihris) => {
        const nisba = mutah > 0 ? (kasr(satr.ard) / mutah) * 100 : 0;
        return (
          <div key={`${String(fihris)}-${satr.nass}`} className="warsha__satr-qiyas">
            <div className="warsha__masar-qiyas">
              <div
                className={nisba > 100 ? 'warsha__mila warsha__mila--tajawuz' : 'warsha__mila'}
                style={{ inlineSize: `${String(Math.min(100, nisba))}%` }}
              />
            </div>
            <p className="warsha__satr-nass" dir="rtl">
              {satr.nass}
            </p>
          </div>
        );
      })}
      <p className="warsha__nass-hadi">
        {bayanat.hajm !== null
          ? t('warsha.muayana.hajm', lugha, { hajm: munassiq.raqm(bayanat.hajm) })
          : null}
        {bayanat.mutah !== null ? (
          <>
            {' — '}
            {t('warsha.muayana.mutah', lugha, { mutah: munassiq.raqm(bayanat.mutah) })}
          </>
        ) : null}
      </p>
      {bayanat.tajawuz_biksil !== null && bayanat.tajawuz_biksil > 0 ? (
        <p className="warsha__tahdheer">
          {t('warsha.muayana.tajawuz', lugha, {
            biksil: munassiq.raqm(bayanat.tajawuz_biksil),
            nisba: munassiq.nisba(bayanat.tajawuz_nisba ?? 0),
          })}
        </p>
      ) : (
        <p className="warsha__nass-hadi">{t('warsha.muayana.la_tajawuz', lugha)}</p>
      )}
    </>
  );
}

export function Warsha(): JSX.Element {
  const { muarrif } = wajihat.useParams();
  const makhzan = useQueryClient();

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq: Munassiqat = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const warsha = useQuery<WarshaHie, KhataJisr>({
    queryKey: mafatih.warsha(muarrif),
    queryFn: () => nadi('nusus_warsha', { muarrif }),
  });

  const taaliqat = useQuery<TaaliqWarshaHie[], KhataJisr>({
    queryKey: mafatih.taaliqat(muarrif),
    queryFn: () => nadi('taaliqat_warsha', { muarrif }),
  });
  const taaliqatBilNass = useMemo(() => {
    const kharita = new Map<string, TaaliqWarshaHie[]>();
    for (const taaliq of taaliqat.data ?? []) {
      if (taaliq.nass !== null) {
        const mawjud = kharita.get(taaliq.nass) ?? [];
        mawjud.push(taaliq);
        kharita.set(taaliq.nass, mawjud);
      }
    }
    return kharita;
  }, [taaliqat.data]);

  const [murashshihat, setMurashshihat] = useState<Murashshihat>(MURASHSHIHAT_BIDAYA);
  const [mukhtar, setMukhtar] = useState<string | null>(null);
  const [nassMuharrar, setNassMuharrar] = useState('');
  const [nassMuqas, setNassMuqas] = useState('');
  const [lawhatJawda, setLawhatJawda] = useState(false);
  const [lawhatDamj, setLawhatDamj] = useState(false);
  const [masarHuzma, setMasarHuzma] = useState('');
  const [qararat, setQararat] = useState<Readonly<Record<string, QararHie>>>({});
  const [saqfDufa, setSaqfDufa] = useState('');
  const [taqaddumDufa, setTaqaddumDufa] = useState<DufaHie | null>(null);

  const sufuf = useMemo(() => warsha.data?.sufuf ?? [], [warsha.data]);
  const musahimi = warsha.data?.musahimi ?? '';
  // Three states, kept apart: nothing extracted, everything read, something did not.
  // An empty table is drawn as "no strings yet" only in the first of them.
  const salama = warsha.data?.salama ?? null;
  const talaf = salama?.talaf ?? null;
  const zahira = useMemo(
    () => sufuf.filter((saf) => yutabiq(saf, murashshihat, musahimi)),
    [sufuf, murashshihat, musahimi],
  );
  const adadDakhili = useMemo(() => sufuf.filter((saf) => saf.dakhili).length, [sufuf]);
  const asnaf = useMemo(() => {
    const kharita = new Map<string, string>();
    for (const saf of sufuf) {
      kharita.set(saf.tasnif, saf.tasnif_arabi);
    }
    return [...kharita.entries()];
  }, [sufuf]);

  const safMukhtar = useMemo(
    () => sufuf.find((saf) => saf.nass === mukhtar) ?? null,
    [sufuf, mukhtar],
  );

  useEffect(() => {
    setNassMuharrar(safMukhtar?.hadaf ?? '');
  }, [safMukhtar?.nass, safMukhtar?.hadaf]);

  useEffect(() => {
    const muhla = setTimeout(() => {
      setNassMuqas(nassMuharrar);
    }, 400);
    return () => {
      clearTimeout(muhla);
    };
  }, [nassMuharrar]);

  const haqlBahth = useRef<HTMLInputElement | null>(null);
  const haqlTahrir = useRef<HTMLTextAreaElement | null>(null);
  const hawiyatQaima = useRef<HTMLDivElement | null>(null);

  const iftahSaf = useCallback((nass: string) => {
    setMukhtar(nass);
  }, []);

  const imsahMurashshihat = useCallback(() => {
    setMurashshihat(MURASHSHIHAT_BIDAYA);
  }, []);

  // The visible count is chrome while it agrees with the total, and a fact the
  // moment a filter is hiding something.
  const murashshah = zahira.length !== sufuf.length;

  const mufahris = useVirtualizer({
    count: zahira.length,
    getScrollElement: () => hawiyatQaima.current,
    estimateSize: () => IRTIFA_SATR_MUBDAI,
    overscan: 12,
  });

  useEffect(() => {
    const alaMiftah = (hadath: globalThis.KeyboardEvent): void => {
      if (hadath.ctrlKey && hadath.key === 'f') {
        hadath.preventDefault();
        haqlBahth.current?.focus();
        return;
      }
      const hadafHtml = hadath.target instanceof HTMLElement ? hadath.target.tagName : '';
      if (hadafHtml === 'INPUT' || hadafHtml === 'TEXTAREA') {
        return;
      }
      if (
        hadath.key !== 'ArrowDown' &&
        hadath.key !== 'ArrowUp' &&
        hadath.key !== 'Home' &&
        hadath.key !== 'End' &&
        hadath.key !== 'Enter'
      ) {
        return;
      }
      const fihris = zahira.findIndex((saf) => saf.nass === mukhtar);
      if (hadath.key === 'Enter') {
        if (fihris >= 0) {
          hadath.preventDefault();
          haqlTahrir.current?.focus();
        }
        return;
      }
      hadath.preventDefault();
      const tali =
        hadath.key === 'Home'
          ? 0
          : hadath.key === 'End'
            ? zahira.length - 1
            : hadath.key === 'ArrowDown'
              ? Math.min(fihris + 1, zahira.length - 1)
              : Math.max(fihris - 1, 0);
      const saf = zahira.at(tali < 0 ? 0 : tali);
      if (saf !== undefined) {
        setMukhtar(saf.nass);
        mufahris.scrollToIndex(tali < 0 ? 0 : tali);
      }
    };
    window.addEventListener('keydown', alaMiftah);
    return () => {
      window.removeEventListener('keydown', alaMiftah);
    };
  }, [zahira, mukhtar, mufahris]);

  const badalSaf = useCallback(
    (jadeed: SafWarshaHie): void => {
      makhzan.setQueryData<WarshaHie>(mafatih.warsha(muarrif), (qadeem) =>
        qadeem === undefined
          ? qadeem
          : {
              ...qadeem,
              sufuf: qadeem.sufuf.map((saf) => (saf.nass === jadeed.nass ? jadeed : saf)),
            },
      );
    },
    [makhzan, muarrif],
  );

  const sajjilTaraju = useTaraju((halat) => halat.sajjil);

  const hifz = useMutation<SafWarshaHie, KhataJisr, { nass: string; hadaf: string; sabiq: string }>(
    {
      mutationFn: ({ nass, hadaf }) => nadi('haddith_tarjama', { muarrif, nass, hadaf }),
      onSuccess: (saf, { nass, sabiq }) => {
        badalSaf(saf);
        sajjilTaraju({
          wasf: t('warsha.taraju.tadeel_tarjama', lugha),
          taraju: () =>
            nadi('haddith_tarjama', { muarrif, nass, hadaf: sabiq }).then((qadeem) => {
              badalSaf(qadeem);
            }),
        });
      },
    },
  );

  const alaHifz = useCallback((): void => {
    if (safMukhtar === null || hifz.isPending) {
      return;
    }
    if ((safMukhtar.hadaf ?? '') === nassMuharrar) {
      return;
    }
    // The row's pre-save hadaf, captured before the mutation, is what undo restores.
    hifz.mutate({ nass: safMukhtar.nass, hadaf: nassMuharrar, sabiq: safMukhtar.hadaf ?? '' });
  }, [safMukhtar, nassMuharrar, hifz]);

  const iqtirahat = useQuery<IqtirahatHie, KhataJisr>({
    queryKey: mafatih.iqtirahat(muarrif, mukhtar ?? ''),
    queryFn: () => nadi('iqtirahat_nass', { muarrif, nass: mukhtar ?? '' }),
    enabled: mukhtar !== null,
  });

  const tatbiq = useMutation<
    SafWarshaHie,
    KhataJisr,
    { nass: string; qayd: number; sabiq: string }
  >(
    {
      mutationFn: ({ nass, qayd }) => nadi('tatbiq_iqtirah', { muarrif, nass, qayd }),
      onSuccess: (saf, { nass, sabiq }) => {
        badalSaf(saf);
        void makhzan.invalidateQueries({ queryKey: mafatih.iqtirahat(muarrif, saf.nass) });
        sajjilTaraju({
          wasf: t('warsha.taraju.tatbiq_iqtirah', lugha),
          taraju: () =>
            nadi('haddith_tarjama', { muarrif, nass, hadaf: sabiq }).then((qadeem) => {
              badalSaf(qadeem);
            }),
        });
      },
    },
  );

  const tilqai = useRef(new Set<string>());
  useEffect(() => {
    const tamm = iqtirahat.data?.tatbiq ?? null;
    if (
      mukhtar !== null &&
      tamm !== null &&
      safMukhtar !== null &&
      safMukhtar.nass === mukhtar &&
      safMukhtar.hadaf === null &&
      !tatbiq.isPending &&
      !tilqai.current.has(mukhtar)
    ) {
      // An exact memory match fills an untranslated string on arrival, marked
      // memory-sourced and queued for review; a fuzzy match never applies itself.
      tilqai.current.add(mukhtar);
      tatbiq.mutate({ nass: mukhtar, qayd: tamm.qayd, sabiq: safMukhtar.hadaf ?? '' });
    }
  }, [iqtirahat.data, mukhtar, safMukhtar, tatbiq]);

  const tarjama = useMutation<NatijatTarjamaHie, KhataJisr, { nass: string }>({
    mutationFn: ({ nass }) => nadi('tarjim_nass', { muarrif, nass }),
    onSuccess: (natija) => {
      badalSaf(natija.saf);
    },
  });

  // The registry keeps the first closures under fixed ids, so the two actions that
  // read per-render state go through a ref refreshed on every render.
  const marjiAwamir = useRef<{ tarjim: () => void; hifz: () => void }>({
    tarjim: () => undefined,
    hifz: () => undefined,
  });
  useEffect(() => {
    marjiAwamir.current = {
      tarjim: () => {
        if (safMukhtar !== null && !tarjama.isPending) {
          tarjama.mutate({ nass: safMukhtar.nass });
        }
      },
      hifz: alaHifz,
    };
  });

  const awamirWarsha = useMemo<readonly AmrLawha[]>(
    () => [
      {
        muarrif: 'warsha.bahth',
        unwan: t('warsha.lawha.bahth', lugha),
        majal: t('warsha.lawha.majal', lugha),
        ikhtisar: 'Ctrl+F',
        nafidh: () => {
          // Deferred one tick: the closing palette restores the pre-open focus first.
          window.setTimeout(() => haqlBahth.current?.focus(), 0);
        },
      },
      {
        muarrif: 'warsha.jawda',
        unwan: t('warsha.lawha.jawda', lugha),
        majal: t('warsha.lawha.majal', lugha),
        nafidh: () => {
          setLawhatJawda((hali) => !hali);
        },
      },
      {
        muarrif: 'warsha.damj',
        unwan: t('warsha.lawha.damj', lugha),
        majal: t('warsha.lawha.majal', lugha),
        nafidh: () => {
          setLawhatDamj((hali) => !hali);
        },
      },
      {
        muarrif: 'warsha.tarjim',
        unwan: t('warsha.lawha.tarjim', lugha),
        majal: t('warsha.lawha.majal', lugha),
        nafidh: () => {
          marjiAwamir.current.tarjim();
        },
      },
      {
        muarrif: 'warsha.hifz',
        unwan: t('warsha.lawha.hifz', lugha),
        majal: t('warsha.lawha.majal', lugha),
        nafidh: () => {
          marjiAwamir.current.hifz();
        },
      },
    ],
    [lugha],
  );
  useSajjilAwamir(awamirWarsha);

  useEffect(() => {
    const ilgha = listen<DufaHie>(HADATH_TAQADDUM_DUFA, (hadath) => {
      setTaqaddumDufa(hadath.payload);
    });
    return () => {
      void ilgha.then((f) => {
        f();
      });
    };
  }, []);

  const dufa = useMutation<DufaHie, KhataJisr, { saqf: number }>({
    mutationFn: ({ saqf }) => nadi('tarjim_dufa', { muarrif, saqf }),
    onSettled: () => {
      setTaqaddumDufa(null);
      void makhzan.invalidateQueries({ queryKey: mafatih.warsha(muarrif) });
    },
  });

  const inqadh = useMutation<InqadhHie, KhataJisr, void>({
    mutationFn: () => nadi('anqidh_mashru', { muarrif }),
    onSuccess: () => {
      void makhzan.invalidateQueries({ queryKey: mafatih.warsha(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.alamat(muarrif) });
    },
  });

  const jawda = useQuery<AlamatMashruHie, KhataJisr>({
    queryKey: mafatih.alamat(muarrif),
    queryFn: () => nadi('alamat_mashru', { muarrif }),
    enabled: lawhatJawda,
  });

  const tawheed = useMutation<number, KhataJisr, { mustalah: string; shakl: string }>({
    mutationFn: ({ mustalah, shakl }) => nadi('wahhid_mustalah', { muarrif, mustalah, shakl }),
    onSuccess: () => {
      void makhzan.invalidateQueries({ queryKey: mafatih.warsha(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.alamat(muarrif) });
    },
  });

  const idmaj = useMutation<NizaatHie, KhataJisr, { masar: string }>({
    mutationFn: ({ masar }) => nadi('idmaj_huzma', { muarrif, masar }),
    onSuccess: () => {
      setQararat({});
    },
  });

  const hasm = useMutation<DamjHie, KhataJisr, { qararat: QararHie[] }>({
    mutationFn: ({ qararat: talabat }) => nadi('qarrir_nizaat', { muarrif, qararat: talabat }),
    onSuccess: () => {
      idmaj.reset();
      setQararat({});
      void makhzan.invalidateQueries({ queryKey: mafatih.warsha(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.taaliqat(muarrif) });
    },
  });

  const muayana = useQuery<MuayanaHie, KhataJisr>({
    queryKey: [...mafatih.warsha(muarrif), 'muayana', mukhtar, nassMuqas],
    queryFn: () => nadi('muayana', { muarrif, nass: mukhtar ?? '', hadaf: nassMuqas }),
    enabled: mukhtar !== null,
    staleTime: Infinity,
  });

  const nizaat = idmaj.data?.nizaat ?? [];
  const baqiya = nizaat.filter((nizaa) => qararat[nizaa.nass] === undefined).length;
  const taaliqatMukhtar = mukhtar === null ? [] : (taaliqatBilNass.get(mukhtar) ?? []);

  const yuhammil = warsha.isPending || idadat.isPending;
  const khata = warsha.error ?? idadat.error;

  return (
    <div className="warsha">
      <header className="warsha__shareet-alawi">
        <Link to="/luba/$muarrif" params={{ muarrif }} className="warsha__raji">
          {t('warsha.raji', lugha)}
        </Link>
        <span className="warsha__fasl">{t('shasha.warsha', lugha)}</span>
        {warsha.data !== undefined ? (
          <>
            <span className="warsha__unwan-luba">{warsha.data.ism_luba}</span>
            <span className="warsha__adad-luba">
              {jam('warsha.adad', lugha, warsha.data.adad, munassiq)}
            </span>
            {talaf !== null ? (
              <span className="warsha__adad-luba warsha__tahdheer" role="status">
                {ikhtar(lugha, talaf.mukhtasar_arabi, talaf.mukhtasar_injilizi)}
              </span>
            ) : null}
          </>
        ) : null}
        <div className="warsha__adawat">
          <button
            type="button"
            className="zir"
            aria-expanded={lawhatJawda}
            onClick={() => {
              setLawhatJawda((hali) => !hali);
            }}
          >
            {t('warsha.jawda.zir', lugha)}
          </button>
          <button
            type="button"
            className="zir"
            aria-expanded={lawhatDamj}
            onClick={() => {
              setLawhatDamj((hali) => !hali);
            }}
          >
            {t('warsha.damj.zir', lugha)}
          </button>
        </div>
      </header>

      {yuhammil ? (
        <HaykalWarsha />
      ) : khata !== null ? (
        <div className="warsha__jism-khata">
          <KutlatKhata
            unwan={t('warsha.khata.tahmil', lugha)}
            khata={khata}
            lugha={lugha}
            muarrif={muarrif}
            aada={() => {
              void warsha.refetch();
              void idadat.refetch();
            }}
          />
        </div>
      ) : warsha.data === undefined ? null : (
        <>
          <AnimatePresence initial={false}>
            {lawhatJawda ? (
              <motion.section
                key="jawda"
                className="warsha__lawha"
                aria-label={t('warsha.jawda.unwan', lugha)}
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={haraka(HARAKAT_LAWHA)}
              >
                <div className="warsha__lawha-dakhil">
                  {jawda.isPending ? (
                    <p className="warsha__jari">{t('warsha.jawda.jari', lugha)}</p>
                  ) : jawda.error !== null ? (
                    <KutlatKhata
                      unwan={t('warsha.jawda.taadhur', lugha)}
                      khata={jawda.error}
                      lugha={lugha}
                      muarrif={muarrif}
                      aada={() => {
                        void jawda.refetch();
                      }}
                    />
                  ) : jawda.data === undefined ? null : (
                    <>
                      <p className="warsha__jawda-adad">
                        {t('warsha.jawda.adad', lugha, {
                          adad: munassiq.raqm(jawda.data.adad_alamat),
                          khatira: munassiq.raqm(jawda.data.adad_khatira),
                        })}
                      </p>
                      {jawda.data.tadarubat.length === 0 ? (
                        <p className="warsha__nass-hadi">{t('warsha.jawda.la_tadarub', lugha)}</p>
                      ) : (
                        <ul className="warsha__tadarubat">
                          {jawda.data.tadarubat.map((tadarub) => (
                            <li key={tadarub.mustalah} className="warsha__tadarub">
                              <p className="warsha__tadarub-unwan">
                                {jam('warsha.jawda.tadarub', lugha, tadarub.adad, munassiq, {
                                  mustalah: tadarub.mustalah,
                                })}
                              </p>
                              <div className="warsha__tadarub-ashkal">
                                {tadarub.ashkal.map((shakl) => (
                                  <button
                                    key={shakl}
                                    type="button"
                                    className="zir"
                                    aria-disabled={tawheed.isPending}
                                    onClick={() => {
                                      if (!tawheed.isPending) {
                                        tawheed.mutate({ mustalah: tadarub.mustalah, shakl });
                                      }
                                    }}
                                  >
                                    {shakl}
                                  </button>
                                ))}
                              </div>
                            </li>
                          ))}
                        </ul>
                      )}
                      {tawheed.data !== undefined ? (
                        <p className="warsha__nass-hadi" role="status">
                          {jam('warsha.jawda.wahhidat', lugha, tawheed.data, munassiq)}
                        </p>
                      ) : null}
                      {tawheed.error !== null ? (
                        <KutlatKhata
                          unwan={t('luba.khata.amal', lugha)}
                          khata={tawheed.error}
                          lugha={lugha}
                          muarrif={muarrif}
                        />
                      ) : null}
                    </>
                  )}
                </div>
              </motion.section>
            ) : null}
          </AnimatePresence>

          <AnimatePresence initial={false}>
            {lawhatDamj ? (
              <motion.section
                key="damj"
                className="warsha__lawha"
                aria-label={t('warsha.damj.zir', lugha)}
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={haraka(HARAKAT_LAWHA)}
              >
                <div className="warsha__lawha-dakhil">
                  <div className="warsha__damj-talab">
                    <label className="warsha__tasmiya" htmlFor="warsha-masar-huzma">
                      {t('warsha.damj.masar', lugha)}
                    </label>
                    <input
                      id="warsha-masar-huzma"
                      className="warsha__haql mono-ltr"
                      dir="ltr"
                      value={masarHuzma}
                      onChange={(hadath) => {
                        setMasarHuzma(hadath.target.value);
                      }}
                    />
                    <button
                      type="button"
                      className="zir zir--tamyeez"
                      aria-disabled={idmaj.isPending || masarHuzma.trim() === ''}
                      onClick={() => {
                        if (!idmaj.isPending && masarHuzma.trim() !== '') {
                          idmaj.mutate({ masar: masarHuzma.trim() });
                        }
                      }}
                    >
                      {t('warsha.damj.istawrid', lugha)}
                    </button>
                  </div>
                  {idmaj.isPending ? (
                    <p className="warsha__jari">{t('warsha.damj.jari', lugha)}</p>
                  ) : null}
                  {idmaj.error !== null ? (
                    <KutlatKhata
                      unwan={t('warsha.damj.taadhur', lugha)}
                      khata={idmaj.error}
                      lugha={lugha}
                      muarrif={muarrif}
                    />
                  ) : null}
                  {idmaj.data !== undefined ? (
                    <div className="warsha__damj-natija">
                      <p className="warsha__nass-hadi">
                        {t(
                          idmaj.data.thulathi ? 'warsha.damj.thulathi' : 'warsha.damj.thunai',
                          lugha,
                        )}
                      </p>
                      <p className="warsha__nass-hadi">
                        {t('warsha.damj.adad', lugha, {
                          mutatabiqa: munassiq.raqm(idmaj.data.mutatabiqa),
                          ana: munassiq.raqm(idmaj.data.min_ana),
                          hum: munassiq.raqm(idmaj.data.min_hum),
                          munfarida: munassiq.raqm(idmaj.data.munfarida),
                        })}
                      </p>
                      {idmaj.data.tanbih_arabi !== null && idmaj.data.tanbih_injilizi !== null ? (
                        <p className="warsha__tahdheer" role="status">
                          {ikhtar(lugha, idmaj.data.tanbih_arabi, idmaj.data.tanbih_injilizi)}
                        </p>
                      ) : null}
                      {nizaat.length === 0 ? (
                        <p className="warsha__nass-hadi">{t('warsha.damj.bila_nizaa', lugha)}</p>
                      ) : (
                        <>
                          <p className="warsha__damj-nizaat">
                            {t('warsha.damj.nizaat', lugha, {
                              adad: munassiq.raqm(nizaat.length),
                            })}
                          </p>
                          <ul className="warsha__nizaat">
                            {nizaat.map((nizaa) => (
                              <BitaqatNizaa
                                key={nizaa.nass}
                                nizaa={nizaa}
                                qarar={qararat[nizaa.nass]}
                                lugha={lugha}
                                alaQarar={(qarar) => {
                                  setQararat((hali) => ({ ...hali, [nizaa.nass]: qarar }));
                                }}
                              />
                            ))}
                          </ul>
                        </>
                      )}
                      {baqiya > 0 ? (
                        <p className="warsha__nass-hadi">
                          {jam('warsha.damj.baqi', lugha, baqiya, munassiq)}
                        </p>
                      ) : null}
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={baqiya > 0 || hasm.isPending}
                        onClick={() => {
                          if (baqiya === 0 && !hasm.isPending) {
                            hasm.mutate({
                              qararat: nizaat
                                .map((nizaa) => qararat[nizaa.nass])
                                .filter((qarar): qarar is QararHie => qarar !== undefined),
                            });
                          }
                        }}
                      >
                        {t('warsha.damj.hasm', lugha)}
                      </button>
                    </div>
                  ) : null}
                  {hasm.error !== null ? (
                    <KutlatKhata
                      unwan={t('luba.khata.amal', lugha)}
                      khata={hasm.error}
                      lugha={lugha}
                      muarrif={muarrif}
                    />
                  ) : null}
                  {hasm.data !== undefined ? (
                    <p className="warsha__nass-hadi" role="status">
                      {t('warsha.damj.tamma', lugha, {
                        nusus: jam('warsha.damj.nusus', lugha, hasm.data.sufuf, munassiq),
                        husum: jam('warsha.damj.husum', lugha, hasm.data.husum, munassiq),
                      })}
                    </p>
                  ) : null}
                </div>
              </motion.section>
            ) : null}
          </AnimatePresence>

          {/* One grid child: the workshop's grid has exactly four chrome tracks above the
              body, so the damage panel shares the filter strip's track rather than taking
              a fifth and pushing the strip below the table. */}
          <div>
            {salama !== null && talaf !== null ? (
              <LawhatTalaf
                salama={salama}
                talaf={talaf}
                lugha={lugha}
                yajri={inqadh.isPending}
                khata={inqadh.error}
                muarrif={muarrif}
                alaInqadh={() => {
                  inqadh.mutate();
                }}
              />
            ) : null}
            {inqadh.data !== undefined ? (
              <section className="warsha__lawha" role="status">
                <div className="warsha__lawha-dakhil">
                  <p className="warsha__nass-hadi">
                    {ikhtar(lugha, inqadh.data.wasf_arabi, inqadh.data.wasf_injilizi)}
                  </p>
                </div>
              </section>
            ) : null}
            <div className="warsha__tasfiya" role="search">
              <input
                ref={haqlBahth}
                className="warsha__haql warsha__bahth"
                type="search"
                dir="auto"
                placeholder={t('warsha.bahth.mawdi', lugha)}
                aria-label={t('warsha.bahth.tasmiya', lugha)}
                value={murashshihat.bahth}
                onChange={(hadath) => {
                  setMurashshihat((hali) => ({ ...hali, bahth: hadath.target.value }));
                }}
              />
              <div
                className="warsha__halat"
                role="group"
                aria-label={t('warsha.tasfiya.hala', lugha)}
              >
                {HALAT.map((hala) => {
                  const faal = murashshihat.halat.includes(hala);
                  return (
                    <button
                      key={hala}
                      type="button"
                      className={faal ? 'warsha__riqaqa warsha__riqaqa--faal' : 'warsha__riqaqa'}
                      aria-pressed={faal}
                      onClick={() => {
                        setMurashshihat((hali) => ({
                          ...hali,
                          halat: faal
                            ? hali.halat.filter((wahida) => wahida !== hala)
                            : [...hali.halat, hala],
                        }));
                      }}
                    >
                      <span
                        className={`warsha__nuqta warsha__nuqta--${hala}`}
                        aria-hidden="true"
                      />
                      {t(MIFTAH_HALA[hala], lugha)}
                    </button>
                  );
                })}
              </div>
              <div className="warsha__qawaim">
                <select
                  className="warsha__haql"
                  aria-label={t('warsha.tasfiya.alamat', lugha)}
                  value={murashshihat.alamat}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MurashshihAlamat;
                    setMurashshihat((hali) => ({ ...hali, alamat: qeema }));
                  }}
                >
                  <option value="kul">{t('warsha.tasfiya.alamat_kul', lugha)}</option>
                  <option value="ay">{t('warsha.tasfiya.alamat_ay', lugha)}</option>
                  <option value="khatir">{t('warsha.tasfiya.alamat_khatir', lugha)}</option>
                </select>
                <select
                  className="warsha__haql"
                  aria-label={t('warsha.tasfiya.tasnif', lugha)}
                  value={murashshihat.tasnif}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    setMurashshihat((hali) => ({ ...hali, tasnif: qeema }));
                  }}
                >
                  <option value="kul">{t('warsha.tasfiya.kul', lugha)}</option>
                  {asnaf.map(([qeema, wasf]) => (
                    <option key={qeema} value={qeema}>
                      {wasf}
                    </option>
                  ))}
                </select>
                <select
                  className="warsha__haql"
                  aria-label={t('warsha.tasfiya.masdar_tarjama', lugha)}
                  value={murashshihat.masdar}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MurashshihMasdar;
                    setMurashshihat((hali) => ({ ...hali, masdar: qeema }));
                  }}
                >
                  <option value="kul">{t('warsha.tasfiya.masdar_kul', lugha)}</option>
                  <option value="aali">{t('warsha.tasfiya.masdar_aali', lugha)}</option>
                  <option value="bashari">{t('warsha.tasfiya.masdar_bashari', lugha)}</option>
                  <option value="dhakira">{t('warsha.tasfiya.masdar_dhakira', lugha)}</option>
                </select>
                <select
                  className="warsha__haql"
                  aria-label={t('warsha.tasfiya.takleef', lugha)}
                  value={murashshihat.takleef}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MurashshihTakleef;
                    setMurashshihat((hali) => ({ ...hali, takleef: qeema }));
                  }}
                >
                  <option value="kul">{t('warsha.tasfiya.takleef_kul', lugha)}</option>
                  <option value="muayyan">{t('warsha.tasfiya.takleef_muayyan', lugha)}</option>
                  <option value="li">{t('warsha.tasfiya.takleef_li', lugha)}</option>
                  <option value="bila">{t('warsha.tasfiya.takleef_bila', lugha)}</option>
                </select>
                {adadDakhili > 0 ? (
                  <button
                    type="button"
                    className="zir"
                    aria-pressed={murashshihat.dakhili}
                    onClick={() => {
                      setMurashshihat((hali) => ({ ...hali, dakhili: !hali.dakhili }));
                    }}
                  >
                    {t(
                      murashshihat.dakhili
                        ? 'warsha.tasfiya.dakhili_ikhfa'
                        : 'warsha.tasfiya.dakhili_izhar',
                      lugha,
                      { adad: munassiq.raqm(adadDakhili) },
                    )}
                  </button>
                ) : null}
              </div>
              <span
                className={
                  murashshah
                    ? 'warsha__adad-zahir warsha__adad-zahir--murashshah'
                    : 'warsha__adad-zahir'
                }
              >
                {t('warsha.adad_zahir', lugha, {
                  adad: munassiq.raqm(zahira.length),
                  kulli: munassiq.raqm(sufuf.length),
                })}
              </span>
            </div>
          </div>

          <div className="warsha__amida">
            {zahira.length === 0 ? (
              <div className="warsha__qaima warsha__qaima--faragh">
                {sufuf.length === 0 && salama !== null && talaf !== null ? (
                  // Not "no strings yet": the file exists and did not read. The advice
                  // to re-extract would recreate the project over it, so it is not given.
                  <div className="halat">
                    <p className="halat__unwan">
                      {ikhtar(lugha, talaf.unwan_arabi, talaf.unwan_injilizi)}
                    </p>
                    <p className="halat__nass">
                      {ikhtar(lugha, salama.wasf_arabi, salama.wasf_injilizi)}
                    </p>
                    <div className="halat__afal">
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={inqadh.isPending}
                        onClick={() => {
                          if (!inqadh.isPending) {
                            inqadh.mutate();
                          }
                        }}
                      >
                        {ikhtar(lugha, talaf.zir_arabi, talaf.zir_injilizi)}
                      </button>
                    </div>
                  </div>
                ) : sufuf.length === 0 ? (
                  <div className="halat">
                    <p className="halat__unwan">{t('warsha.faragh.nusus.unwan', lugha)}</p>
                    <p className="halat__nass">{t('warsha.faragh.nusus.nass', lugha)}</p>
                    <div className="halat__afal">
                      <Link to="/luba/$muarrif" params={{ muarrif }} className="zir">
                        {t('warsha.raji', lugha)}
                      </Link>
                    </div>
                  </div>
                ) : (
                  <div className="halat">
                    <p className="halat__unwan">{t('warsha.faragh.tasfiya.unwan', lugha)}</p>
                    <p className="halat__nass">
                      {t('warsha.faragh.tasfiya.nass', lugha, {
                        kulli: munassiq.raqm(sufuf.length),
                      })}
                    </p>
                    <div className="halat__afal">
                      <button type="button" className="zir" onClick={imsahMurashshihat}>
                        {t('warsha.faragh.tasfiya.imsah', lugha)}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div
                className="warsha__qaima"
                ref={hawiyatQaima}
                role="listbox"
                aria-label={t('warsha.qaima.tasmiya', lugha)}
                tabIndex={0}
              >
                <div
                  className="warsha__qaima-jism"
                  style={{ blockSize: `${String(mufahris.getTotalSize())}px` }}
                >
                  {mufahris.getVirtualItems().map((band) => {
                    const saf = zahira.at(band.index);
                    if (saf === undefined) {
                      return null;
                    }
                    return (
                      <div
                        key={saf.nass}
                        className="warsha__band"
                        role="option"
                        aria-selected={saf.nass === mukhtar}
                        data-index={band.index}
                        ref={mufahris.measureElement}
                        style={{ transform: `translateY(${String(band.start)}px)` }}
                      >
                        <SaffQaima
                          saf={saf}
                          mukhtar={saf.nass === mukhtar}
                          taaliqat={(taaliqatBilNass.get(saf.nass) ?? []).length}
                          lugha={lugha}
                          alaIkhtiyar={() => {
                            iftahSaf(saf.nass);
                          }}
                        />
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            <div className="warsha__wasat">
              {safMukhtar === null ? (
                <p className="warsha__nass-hadi warsha__la-ikhtiyar">
                  {t('warsha.muharrir.la_ikhtiyar', lugha)}
                </p>
              ) : (
                <>
                  <section
                    className="warsha__muharrir"
                    aria-label={t('warsha.muharrir.unwan', lugha)}
                  >
                    <p className="warsha__tasmiya">{t('warsha.muharrir.masdar', lugha)}</p>
                    <p className="warsha__masdar" dir="auto">
                      {safMukhtar.masdar}
                    </p>
                    <label className="warsha__tasmiya" htmlFor="warsha-hadaf">
                      {t('warsha.muharrir.hadaf', lugha)}
                    </label>
                    <textarea
                      id="warsha-hadaf"
                      ref={haqlTahrir}
                      className="warsha__tahrir"
                      dir="rtl"
                      value={nassMuharrar}
                      placeholder={t('warsha.muharrir.mawdi', lugha)}
                      onChange={(hadath) => {
                        setNassMuharrar(hadath.target.value);
                      }}
                      onKeyDown={(hadath: KeyboardEvent<HTMLTextAreaElement>) => {
                        if (hadath.ctrlKey && hadath.key === 'Enter') {
                          hadath.preventDefault();
                          alaHifz();
                        }
                      }}
                      onBlur={alaHifz}
                    />
                    <div className="warsha__muharrir-afal">
                      <button
                        type="button"
                        className="zir zir--tamyeez"
                        aria-disabled={hifz.isPending}
                        onClick={alaHifz}
                      >
                        {t(hifz.isPending ? 'warsha.muharrir.jari' : 'warsha.muharrir.hifz', lugha)}
                      </button>
                      <button
                        type="button"
                        className="zir"
                        aria-disabled={tarjama.isPending}
                        onClick={() => {
                          if (!tarjama.isPending) {
                            tarjama.mutate({ nass: safMukhtar.nass });
                          }
                        }}
                      >
                        {t(
                          tarjama.isPending ? 'warsha.tarjama.jari' : 'warsha.tarjama.zir',
                          lugha,
                        )}
                      </button>
                    </div>
                    {hifz.error !== null ? (
                      <KutlatKhata
                        unwan={t('luba.khata.amal', lugha)}
                        khata={hifz.error}
                        lugha={lugha}
                        muarrif={muarrif}
                      />
                    ) : null}
                    {tarjama.error !== null ? (
                      <KutlatKhata
                        unwan={t('luba.khata.amal', lugha)}
                        khata={tarjama.error}
                        lugha={lugha}
                        muarrif={muarrif}
                      />
                    ) : null}
                    {tarjama.data !== undefined &&
                    !tarjama.data.najahat &&
                    tarjama.data.sabab_arabi !== null ? (
                      <p className="warsha__rafd" role="status">
                        {tarjama.data.sabab_arabi}
                      </p>
                    ) : null}
                    {safMukhtar.alamat.length > 0 ? (
                      <ul className="warsha__alamat">
                        {safMukhtar.alamat.map((alam) => (
                          <li
                            key={`${alam.naw}-${alam.wasf_arabi}`}
                            className={
                              alam.khatir
                                ? 'warsha__alam warsha__alam--khatir'
                                : 'warsha__alam'
                            }
                            title={alam.wasf_injilizi}
                          >
                            {alam.wasf_arabi}
                          </li>
                        ))}
                      </ul>
                    ) : null}
                  </section>

                  <section className="warsha__siyaq" aria-label={t('warsha.siyaq.unwan', lugha)}>
                    <h2 className="warsha__unwan-qism">{t('warsha.siyaq.unwan', lugha)}</h2>
                    <dl className="warsha__jadwal">
                      <div className="warsha__saff-jadwal">
                        <dt>{t('warsha.siyaq.hawiya', lugha)}</dt>
                        <dd className="mono-ltr warsha__qat" title={safMukhtar.hawiya}>
                          {safMukhtar.hawiya}
                        </dd>
                      </div>
                      <div className="warsha__saff-jadwal">
                        <dt>{t('warsha.siyaq.mawqi', lugha)}</dt>
                        <dd className="mono-ltr warsha__qat" title={safMukhtar.mawqi}>
                          {safMukhtar.mawqi}
                        </dd>
                      </div>
                      <div className="warsha__saff-jadwal">
                        <dt>{t('warsha.siyaq.tasnif', lugha)}</dt>
                        <dd>{safMukhtar.tasnif_arabi}</dd>
                      </div>
                      {safMukhtar.mutakallim !== null ? (
                        <div className="warsha__saff-jadwal">
                          <dt>{t('warsha.siyaq.mutakallim', lugha)}</dt>
                          <dd>{safMukhtar.mutakallim}</dd>
                        </div>
                      ) : null}
                      {safMukhtar.tareeqa_arabi !== null ? (
                        <div className="warsha__saff-jadwal">
                          <dt>{t('warsha.siyaq.tareeqa', lugha)}</dt>
                          <dd>{safMukhtar.tareeqa_arabi}</dd>
                        </div>
                      ) : null}
                      {safMukhtar.muzawwid !== null ? (
                        <div className="warsha__saff-jadwal">
                          <dt>{t('warsha.siyaq.muzawwid', lugha)}</dt>
                          <dd>{safMukhtar.muzawwid}</dd>
                        </div>
                      ) : null}
                      {safMukhtar.muayyan !== null ? (
                        <div className="warsha__saff-jadwal">
                          <dt>{t('warsha.siyaq.muayyan', lugha)}</dt>
                          <dd className="mono-ltr">{safMukhtar.muayyan}</dd>
                        </div>
                      ) : null}
                    </dl>
                    <p className="warsha__nass-hadi">
                      {jam('warsha.siyaq.takrar', lugha, safMukhtar.takrar, munassiq)}
                    </p>
                    {safMukhtar.aqsa_ard !== null ? (
                      <p className="warsha__nass-hadi">
                        {t('warsha.siyaq.ard', lugha, {
                          adad: munassiq.raqm(safMukhtar.aqsa_ard),
                        })}
                      </p>
                    ) : null}
                    {safMukhtar.hajm_khatt !== null ? (
                      <p className="warsha__nass-hadi">
                        {t('warsha.siyaq.hajm', lugha, {
                          adad: munassiq.raqm(safMukhtar.hajm_khatt),
                        })}
                      </p>
                    ) : null}
                    {safMukhtar.jiwar.length > 0 ? (
                      <>
                        <p className="warsha__tasmiya">{t('warsha.siyaq.jiwar', lugha)}</p>
                        <ul className="warsha__jiwar">
                          {safMukhtar.jiwar.map((satr, fihris) => (
                            <li key={`${satr}-${String(fihris)}`} dir="auto">
                              {satr}
                            </li>
                          ))}
                        </ul>
                      </>
                    ) : null}
                    {taaliqat.error !== null ? (
                      <KutlatKhata
                        unwan={t('warsha.taaliq.taadhur', lugha)}
                        khata={taaliqat.error}
                        lugha={lugha}
                        muarrif={muarrif}
                        aada={() => {
                          void taaliqat.refetch();
                        }}
                      />
                    ) : null}
                    {taaliqatMukhtar.length > 0 ? (
                      <>
                        <p className="warsha__tasmiya">{t('warsha.taaliq.unwan', lugha)}</p>
                        <ul className="warsha__taaliqat">
                          {taaliqatMukhtar.map((taaliq) => (
                            <li
                              key={`${taaliq.kaatib}-${taaliq.waqt}`}
                              className="warsha__taaliq"
                            >
                              <p className="warsha__taaliq-raas">
                                <span className="mono-ltr">{taaliq.kaatib}</span>
                                {taaliq.min_almalik ? (
                                  <span className="warsha__wasm-malik">
                                    {t('warsha.taaliq.malik', lugha)}
                                  </span>
                                ) : null}
                                {taaliq.muhall ? (
                                  <span className="warsha__wasm-muhall">
                                    {t('warsha.taaliq.muhall', lugha)}
                                  </span>
                                ) : null}
                              </p>
                              <p className="warsha__taaliq-matn">{taaliq.matn}</p>
                              <p className="warsha__taaliq-waqt" dir="ltr">
                                {taaliq.waqt}
                              </p>
                            </li>
                          ))}
                        </ul>
                      </>
                    ) : null}
                  </section>
                </>
              )}
            </div>

            <div className="warsha__janib-akhir">
              <section
                className="warsha__iqtirahat"
                aria-label={t('warsha.iqtirah.unwan', lugha)}
              >
                <h2 className="warsha__unwan-qism">{t('warsha.iqtirah.unwan', lugha)}</h2>
                {mukhtar === null ? (
                  <p className="warsha__nass-hadi">{t('warsha.muharrir.la_ikhtiyar', lugha)}</p>
                ) : iqtirahat.isPending ? (
                  <p className="warsha__jari">{t('warsha.iqtirah.jari', lugha)}</p>
                ) : iqtirahat.error !== null ? (
                  <KutlatKhata
                    unwan={t('warsha.iqtirah.taadhur', lugha)}
                    khata={iqtirahat.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      void iqtirahat.refetch();
                    }}
                  />
                ) : iqtirahat.data === undefined ? null : (
                  <>
                    <LawhatIqtirahat
                      bayanat={iqtirahat.data}
                      mutabbaq={safMukhtar?.muzawwid === 'الذاكرة'}
                      lugha={lugha}
                      munassiq={munassiq}
                      yajri={tatbiq.isPending}
                      alaTatbiq={(qayd) => {
                        tatbiq.mutate({ nass: mukhtar, qayd, sabiq: safMukhtar?.hadaf ?? '' });
                      }}
                    />
                    {tatbiq.error !== null ? (
                      <KutlatKhata
                        unwan={t('luba.khata.amal', lugha)}
                        khata={tatbiq.error}
                        lugha={lugha}
                        muarrif={muarrif}
                      />
                    ) : null}
                  </>
                )}
              </section>

              <section className="warsha__dufa" aria-label={t('warsha.dufa.unwan', lugha)}>
                <h2 className="warsha__unwan-qism">{t('warsha.dufa.unwan', lugha)}</h2>
                {warsha.data.muzawwid.hala !== 'mukhtar' ? (
                  // Said here, before a run is started: which provider a run would bill,
                  // in the settings crate's own words — a stale default most of all.
                  <p
                    className={
                      warsha.data.muzawwid.hala === 'badeel'
                        ? 'warsha__tahdheer'
                        : 'warsha__nass-hadi'
                    }
                    role="status"
                  >
                    {ikhtar(
                      lugha,
                      warsha.data.muzawwid.wasf_arabi,
                      warsha.data.muzawwid.wasf_injilizi,
                    )}
                  </p>
                ) : null}
                <label className="warsha__tasmiya" htmlFor="warsha-saqf">
                  {t('warsha.dufa.saqf', lugha)}
                </label>
                <div className="warsha__dufa-talab">
                  <input
                    id="warsha-saqf"
                    className="warsha__haql mono-ltr"
                    type="number"
                    dir="ltr"
                    min="0"
                    step="0.5"
                    value={saqfDufa}
                    onChange={(hadath) => {
                      setSaqfDufa(hadath.target.value);
                    }}
                  />
                  <button
                    type="button"
                    className="zir zir--tamyeez"
                    aria-disabled={dufa.isPending || Number(saqfDufa) <= 0}
                    onClick={() => {
                      const saqf = Number(saqfDufa);
                      if (!dufa.isPending && Number.isFinite(saqf) && saqf > 0) {
                        dufa.mutate({ saqf });
                      }
                    }}
                  >
                    {t('warsha.dufa.bad', lugha)}
                  </button>
                </div>
                <div aria-live="polite">
                  {dufa.isPending && taqaddumDufa !== null ? (
                    <p className="warsha__jari">
                      {t('warsha.dufa.jari', lugha, {
                        mutarjama: munassiq.raqm(taqaddumDufa.mutarjama),
                        fashila: munassiq.raqm(taqaddumDufa.fashila),
                        taklifa: kasr(taqaddumDufa.taklifa).toFixed(2),
                        saqf: kasr(taqaddumDufa.saqf).toFixed(2),
                      })}
                    </p>
                  ) : null}
                  {dufa.error !== null ? (
                    <KutlatKhata
                      unwan={t('warsha.dufa.taadhur', lugha)}
                      khata={dufa.error}
                      lugha={lugha}
                      muarrif={muarrif}
                    />
                  ) : null}
                  {dufa.data !== undefined ? (
                    <>
                      <p className="warsha__nass-hadi">
                        {t('warsha.dufa.tamma', lugha, {
                          mutarjama: munassiq.raqm(dufa.data.mutarjama),
                          fashila: munassiq.raqm(dufa.data.fashila),
                          taklifa: kasr(dufa.data.taklifa).toFixed(2),
                        })}
                      </p>
                      {dufa.data.tawaqqafat_lil_saqf ? (
                        <p className="warsha__tahdheer">{t('warsha.dufa.saqf_waqf', lugha)}</p>
                      ) : null}
                    </>
                  ) : null}
                </div>
              </section>

              <section className="warsha__muayana" aria-label={t('warsha.muayana.unwan', lugha)}>
                <h2 className="warsha__unwan-qism">{t('warsha.muayana.unwan', lugha)}</h2>
                {mukhtar === null ? (
                  <p className="warsha__nass-hadi">{t('warsha.muharrir.la_ikhtiyar', lugha)}</p>
                ) : muayana.isPending ? (
                  <p className="warsha__jari">{t('warsha.muayana.jari', lugha)}</p>
                ) : muayana.error !== null ? (
                  <KutlatKhata
                    unwan={t('warsha.muayana.taadhur', lugha)}
                    khata={muayana.error}
                    lugha={lugha}
                    muarrif={muarrif}
                    aada={() => {
                      void muayana.refetch();
                    }}
                  />
                ) : muayana.data === undefined ? null : (
                  <LawhatMuayana bayanat={muayana.data} lugha={lugha} munassiq={munassiq} />
                )}
              </section>
            </div>
          </div>
        </>
      )}

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
