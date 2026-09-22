import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import { kasr } from '@/mustalahat/arqam';
import type { JSX, ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import type { MiftahQism } from '@/hayat/aqsam_idadat';
import { hallil } from '@/hayat/ikhtisarat';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { ansha } from '@/hayat/tanbihat';
import { useTaraju } from '@/hayat/taraju';
import type { MiftahLugha } from '@/lugha/lugha';
import { munassiqat, t } from '@/lugha/lugha';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import type { KhasaisRamz } from '@/mukawwinat/rumuz';
import { ramz } from '@/mukawwinat/rumuz';
import { Zuhur } from '@/mukawwinat/zuhur';
import type {
  HalatTahdith,
  Idadat,
  IdadatManassat,
  IdadatMuzawwid,
  Kathafa,
  KhattHie,
  Lugha,
  MawqiTabaqa,
  MuharrikQira,
  MustawaSijill,
  NawMuzawwid,
  NizamArqam,
  QanatTahdith,
  Sima,
} from '@/mustalahat/awamir';

import './idadat.css';

/** شاشة الإعدادات — a working copy of the whole tree, edited locally and written back in one save. */

const wajihat = getRouteApi('/idadat');

type MiftahManassa = Exclude<keyof IdadatManassat, 'mujalladat_idafiya' | 'fahs_tilqai'>;

const MANASSAT: readonly MiftahManassa[] = [
  'steam',
  'epic',
  'gog',
  'ea',
  'ubisoft',
  'battlenet',
  'xbox',
  'itch',
  'heroic',
  'amazon',
  'rockstar',
  'riot',
  'lutris',
  'bottles',
  'legendary',
  'playnite',
];

const MIFTAH_MANASSA: Readonly<Record<MiftahManassa, MiftahLugha>> = {
  steam: 'idadat.manassat.steam',
  epic: 'idadat.manassat.epic',
  gog: 'idadat.manassat.gog',
  ea: 'idadat.manassat.ea',
  ubisoft: 'idadat.manassat.ubisoft',
  battlenet: 'idadat.manassat.battlenet',
  xbox: 'idadat.manassat.xbox',
  itch: 'idadat.manassat.itch',
  heroic: 'idadat.manassat.heroic',
  amazon: 'idadat.manassat.amazon',
  rockstar: 'idadat.manassat.rockstar',
  riot: 'idadat.manassat.riot',
  lutris: 'idadat.manassat.lutris',
  bottles: 'idadat.manassat.bottles',
  legendary: 'idadat.manassat.legendary',
  playnite: 'idadat.manassat.playnite',
};

const LUGHAT: readonly Lugha[] = ['arabi', 'injilizi'];
const MIFTAH_LUGHAT: Readonly<Record<Lugha, MiftahLugha>> = {
  arabi: 'idadat.lugha.arabi',
  injilizi: 'idadat.lugha.injilizi',
};

const ANZIMAT_ARQAM: readonly NizamArqam[] = ['latini', 'arabi', 'farisi'];
const MIFTAH_ARQAM: Readonly<Record<NizamArqam, MiftahLugha>> = {
  latini: 'idadat.arqam.latini',
  arabi: 'idadat.arqam.arabi',
  farisi: 'idadat.arqam.farisi',
};

const SIMAT: readonly Sima[] = ['daken', 'fatih', 'nizam'];
const MIFTAH_SIMA: Readonly<Record<Sima, MiftahLugha>> = {
  daken: 'idadat.sima.daken',
  fatih: 'idadat.sima.fatih',
  nizam: 'idadat.sima.nizam',
};

const KATHAFAT: readonly Kathafa[] = ['mudmaj', 'murih', 'kabir'];
const MIFTAH_KATHAFA: Readonly<Record<Kathafa, MiftahLugha>> = {
  mudmaj: 'idadat.kathafa.mudmaj',
  murih: 'idadat.kathafa.murih',
  kabir: 'idadat.kathafa.kabir',
};

const ANWA_MUZAWWID: readonly NawMuzawwid[] = [
  'anthropic',
  'open_ai_mutawafiq',
  'gemini',
  'deepl',
  'google_tarjama',
  'microsoft_tarjama',
  'mahalli',
  'google_majjani',
];
const MIFTAH_NAW: Readonly<Record<NawMuzawwid, MiftahLugha>> = {
  anthropic: 'idadat.naw.anthropic',
  open_ai_mutawafiq: 'idadat.naw.open_ai_mutawafiq',
  gemini: 'idadat.naw.gemini',
  deepl: 'idadat.naw.deepl',
  google_tarjama: 'idadat.naw.google_tarjama',
  microsoft_tarjama: 'idadat.naw.microsoft_tarjama',
  mahalli: 'idadat.naw.mahalli',
  google_majjani: 'idadat.naw.google_majjani',
};

/**
 * The built-in free provider's identifier and model, mirroring
 * `taarib_usus::idadat::MUARRIF_GOOGLE_MAJJANI` and `NAMUDHAJ_GOOGLE_MAJJANI`.
 * It is not in the settings tree — nothing deletes it, no key unlocks it, no
 * field tunes it — so the screen draws it from these rather than from the
 * working copy, and the backend refuses a user row that borrows the name.
 */
const MUARRIF_GOOGLE_MAJJANI = 'google-majjani';
const NAMUDHAJ_GOOGLE_MAJJANI = 'gtx';

const MUSTAWAYAT: readonly MustawaSijill[] = ['khata', 'tanbeeh', 'maluma', 'tafsil', 'tatabbu'];
const MIFTAH_MUSTAWA: Readonly<Record<MustawaSijill, MiftahLugha>> = {
  khata: 'idadat.mustawa.khata',
  tanbeeh: 'idadat.mustawa.tanbeeh',
  maluma: 'idadat.mustawa.maluma',
  tafsil: 'idadat.mustawa.tafsil',
  tatabbu: 'idadat.mustawa.tatabbu',
};

const QANAWAT: readonly QanatTahdith[] = ['mustaqirr', 'tajribi'];
const MIFTAH_QANAT: Readonly<Record<QanatTahdith, MiftahLugha>> = {
  mustaqirr: 'idadat.qanat.mustaqirr',
  tajribi: 'idadat.qanat.tajribi',
};

const MUHARRIKAT_QIRA: readonly MuharrikQira[] = ['nizam_asli', 'mahmul'];
const MIFTAH_MUHARRIK: Readonly<Record<MuharrikQira, MiftahLugha>> = {
  nizam_asli: 'idadat.muharrik.nizam_asli',
  mahmul: 'idadat.muharrik.mahmul',
};

const MAWAQI_TABAQA: readonly MawqiTabaqa[] = ['fawq', 'bijanib', 'lawha'];
const MIFTAH_MAWQI: Readonly<Record<MawqiTabaqa, MiftahLugha>> = {
  fawq: 'idadat.mawqi.fawq',
  bijanib: 'idadat.mawqi.bijanib',
  lawha: 'idadat.mawqi.lawha',
};

/** The blank provider a new table row starts from. */
const MUZAWWID_FARIGH: IdadatMuzawwid = {
  muarrif: '',
  naw: 'anthropic',
  namudhaj: '',
  asas: null,
  hisab_miftah: null,
  mufaal: false,
  hadd_talabat: 50,
  mizaniya: null,
};

/**
 * What the provider list amounts to, and therefore what it does not offer.
 *
 * The same four states as `taarib_usus::idadat::HalatMuzawwidin`, and derived
 * here rather than asked for, because this screen edits a **working copy**: the
 * backend's answer describes the tree that was last saved, and the sentence has
 * to describe the tree the person is looking at. A key mirrors the state, so the
 * two sets of words stay one translation apart rather than one paraphrase.
 *
 * `mukhtar` has no sentence. A configuration that is doing what it says needs no
 * paragraph explaining that, and a screen that annotates every healthy state is
 * a screen whose annotations stop being read.
 */
type HalatMuzawwidin = 'faragh' | 'muattala' | 'mukhtar' | 'badeel';

const ATHAR_MUZAWWIDIN: Readonly<Record<HalatMuzawwidin, MiftahLugha | null>> = {
  faragh: 'idadat.muzawwidun.athar_faragh',
  muattala: 'idadat.muzawwidun.athar_muattala',
  mukhtar: null,
  badeel: 'idadat.muzawwidun.athar_badeel',
};

function halatMuzawwidin(muzawwidun: Idadat['muzawwidun']): HalatMuzawwidin {
  const { qaima, iftiradi } = muzawwidun;
  const muntakhab =
    (iftiradi === null
      ? undefined
      : qaima.find((muzawwid) => muzawwid.muarrif === iftiradi && muzawwid.mufaal)) ??
    qaima.find((muzawwid) => muzawwid.mufaal);
  if (muntakhab === undefined) {
    return qaima.length === 0 ? 'faragh' : 'muattala';
  }
  return iftiradi !== null && iftiradi !== muntakhab.muarrif ? 'badeel' : 'mukhtar';
}

/**
 * The one permanent provider row: the built-in free Google Translate service,
 * which a new translation falls back to whenever the list above it elects
 * nothing. It carries a state word in place of a Remove button and says, in
 * one paragraph, exactly what it is worth — unofficial, sentence-level, rate
 * limited, free — because a row that looked like the others would read as a
 * provider somebody chose.
 */
function MuzawwidMudmaj({ lugha, yustakhdam }: { lugha: Lugha; yustakhdam: boolean }): JSX.Element {
  return (
    <li className="idadat__muzawwid idadat__muzawwid--mudmaj">
      <div className="idadat__muzawwid-raas">
        <h3 className="idadat__muzawwid-unwan">{t('idadat.naw.google_majjani', lugha)}</h3>
        <span className="idadat__muzawwid-muarrif mono-ltr">{MUARRIF_GOOGLE_MAJJANI}</span>
        <span className="idadat__muzawwid-thabit">{t('idadat.muzawwidun.mudmaj.thabit', lugha)}</span>
      </div>
      <p
        className={
          yustakhdam
            ? 'idadat__mudakhkhal idadat__muzawwid-hala idadat__muzawwid-hala--yustakhdam'
            : 'idadat__mudakhkhal idadat__muzawwid-hala'
        }
        role="status"
      >
        {t(
          yustakhdam ? 'idadat.muzawwidun.mudmaj.yustakhdam' : 'idadat.muzawwidun.mudmaj.ihtiyat',
          lugha,
        )}
      </p>
      <div className="idadat__saff">
        <span className="idadat__tasmiya">{t('idadat.muzawwidun.namudhaj', lugha)}</span>
        <span className="idadat__muzawwid-qeema mono-ltr">{NAMUDHAJ_GOOGLE_MAJJANI}</span>
      </div>
      <p className="idadat__mudakhkhal idadat__nass-hadi">
        {t('idadat.muzawwidun.mudmaj.wasf', lugha)}
      </p>
    </li>
  );
}

/** How many rows each placeholder section holds: the display section's own count. */
const SUFUF_HAYKAL = 4;

/**
 * The settings document drawn empty: two sections on the screen's own two
 * columns, each row a label bar beside a field-shaped box at the field's own
 * height, so the first real row lands on the first placeholder rather than on
 * a paragraph's worth of bars above it.
 */
function HaykalIdadat(): JSX.Element {
  return (
    <div className="idadat__haykal zuhur-muakhkhar" aria-hidden="true">
      {Array.from({ length: 2 }, (_, qism) => (
        <section key={qism} className="idadat__qism">
          <div className="idadat__raas-qism">
            <span className="idadat__haykal-satr idadat__haykal-satr--unwan" />
          </div>
          {Array.from({ length: SUFUF_HAYKAL }, (_, saff) => (
            <div key={saff} className="idadat__saff">
              <span
                className="idadat__haykal-satr idadat__haykal-satr--tasmiya"
                style={{ inlineSize: `${String(40 + ((saff * 23 + qism * 11) % 45))}%` }}
              />
              <span className="idadat__haykal-haql" />
            </div>
          ))}
        </section>
      ))}
    </div>
  );
}

/** A list skeleton on the screen's own bars, not the library's card boxes. */
function HaykalQaima(): JSX.Element {
  return (
    <div className="idadat__haykal-qaima zuhur-muakhkhar" aria-hidden="true">
      <span className="idadat__haykal-satr idadat__haykal-satr--tawil" />
      <span className="idadat__haykal-satr idadat__haykal-satr--mutawassit" />
    </div>
  );
}

/**
 * A failure said where the eye is. On a document this long the block that
 * explains it can sit a screen away from the reader — under the save bar,
 * inside a section they scrolled past, behind a palette action — so the
 * notice carries the same sentence and the same code to wherever they are.
 */
function anshaKhatar(khata: KhataJisr, lugha: Lugha): void {
  ansha({
    naw: 'khatar',
    nass: khata.nass(lugha) ?? t('faragh.jisr', lugha),
    ramz: khata.khata?.ramz ?? khata.amr,
  });
}

/**
 * A plain chevron. Symmetric about the grid's centre line, so the reading
 * direction never mirrors it; the fold state is what turns it.
 */
function RamzTayy(khasais: KhasaisRamz): JSX.Element {
  return ramz(khasais, <path d="M6 9.5L12 15.5L18 9.5" />);
}

interface KhasaisQism {
  readonly muarrif: MiftahQism;
  readonly unwan: string;
  readonly sharh?: string;
  readonly maftuh: boolean;
  readonly alaTabdeel: (muarrif: MiftahQism) => void;
  readonly children: ReactNode;
}

/**
 * One section of the document: a heading that folds its body. The sentence
 * under the heading stays out when the body is folded, because it is what
 * says whether this is the section the reader is looking for.
 */
function QismIdadat({
  muarrif,
  unwan,
  sharh,
  maftuh,
  alaTabdeel,
  children,
}: KhasaisQism): JSX.Element {
  const muarrifUnwan = `idadat-unwan-${muarrif}`;
  return (
    <section className="idadat__qism" aria-labelledby={muarrifUnwan}>
      <div className="idadat__raas-qism">
        <h2 className="idadat__unwan-qism">
          <button
            type="button"
            id={muarrifUnwan}
            className="idadat__zir-qism"
            aria-expanded={maftuh}
            onClick={() => {
              alaTabdeel(muarrif);
            }}
          >
            <RamzTayy
              className={
                maftuh ? 'idadat__ramz-qism idadat__ramz-qism--maftuh' : 'idadat__ramz-qism'
              }
            />
            {unwan}
          </button>
        </h2>
        {sharh === undefined ? null : <p className="idadat__sharh-qism">{sharh}</p>}
      </div>
      <Zuhur maftuh={maftuh} asl="fawq" className="idadat__jism-qism">
        {children}
      </Zuhur>
    </section>
  );
}

/** Defensive number parsing: empty keeps the old value, NaN is ignored. */
function raqmAw(khaam: string, qadeem: number): number {
  if (khaam.trim() === '') {
    return qadeem;
  }
  const qeema = Number(khaam);
  return Number.isFinite(qeema) ? qeema : qadeem;
}

/** The documented floors and ranges, enforced once at save time. */
function tahdheeb(shajara: Idadat): Idadat {
  const lawha = hallil(shajara.ikhtisarat.lawha) !== null ? shajara.ikhtisarat.lawha : 'ctrl+k';
  let taraju = hallil(shajara.ikhtisarat.taraju) !== null ? shajara.ikhtisarat.taraju : 'ctrl+z';
  if (taraju === lawha) {
    // Two actions cannot share a chord; the undo yields and returns to a default.
    taraju = lawha === 'ctrl+z' ? 'ctrl+shift+z' : 'ctrl+z';
  }
  return {
    ...shajara,
    takhzin: {
      ...shajara.takhzin,
      hadd_makhbaa_mb: Math.max(64, shajara.takhzin.hadd_makhbaa_mb),
    },
    masadir: {
      ...shajara.masadir,
      fatra_tahdith: Math.max(5, shajara.masadir.fatra_tahdith),
    },
    tashkhis: {
      ...shajara.tashkhis,
      ayyam_hifz: Math.max(1, shajara.tashkhis.ayyam_hifz),
    },
    tabaqa: {
      ...shajara.tabaqa,
      shaffafiya: Math.min(1, Math.max(0, kasr(shajara.tabaqa.shaffafiya))),
    },
    ikhtisarat: { lawha, taraju },
  };
}

interface KhasaisQaimatMasarat {
  readonly qeem: readonly string[];
  readonly tasmiya: string;
  readonly mawdi: string;
  readonly lugha: Lugha;
  readonly alaTaghyeer: (jadeed: string[]) => void;
}

/** An editable path list: existing rows with a remove each, and one add row. */
function QaimatMasarat({
  qeem,
  tasmiya,
  mawdi,
  lugha,
  alaTaghyeer,
}: KhasaisQaimatMasarat): JSX.Element {
  const [jadeed, setJadeed] = useState('');

  // The add row is the empty state's action as well as the list's footer, so it
  // is built once and placed inside whichever of the two is rendered. The ghost
  // text is dropped in the empty state, where the sentence above already says
  // what the field takes and the two would otherwise read twice.
  const saffIdafa = (
    <div className="idadat__saff-idafa">
      <input
        className="idadat__haql mono-ltr"
        dir="ltr"
        value={jadeed}
        placeholder={qeem.length === 0 ? '' : mawdi}
        aria-label={tasmiya}
        onChange={(hadath) => {
          setJadeed(hadath.target.value);
        }}
      />
      <button
        type="button"
        className="zir"
        aria-disabled={jadeed.trim() === ''}
        onClick={() => {
          const safi = jadeed.trim();
          if (safi !== '') {
            alaTaghyeer([...qeem, safi]);
            setJadeed('');
            ansha({
              naw: 'najah',
              nass: t('idadat.qaima.tamma_idafa', lugha, { tasmiya }),
              tafsil: safi,
            });
          }
        }}
      >
        {t('idadat.qaima.adif', lugha)}
      </button>
    </div>
  );

  return (
    <div className="idadat__qaima-masarat">
      {qeem.length === 0 ? (
        <HalatFarigha unwan={t('idadat.qaima.farigha', lugha)} nass={mawdi}>
          {saffIdafa}
        </HalatFarigha>
      ) : (
        <>
          <ul className="idadat__masarat">
            {qeem.map((masar, fihris) => (
              <li key={`${masar}-${String(fihris)}`} className="idadat__masar">
                <span className="mono-ltr idadat__masar-nass">{masar}</span>
                <button
                  type="button"
                  className="zir"
                  onClick={() => {
                    alaTaghyeer(qeem.filter((_, ayn) => ayn !== fihris));
                    ansha({
                      naw: 'najah',
                      nass: t('idadat.qaima.tamma_izala', lugha, { tasmiya }),
                      tafsil: masar,
                    });
                  }}
                >
                  {t('idadat.qaima.izala', lugha)}
                </button>
              </li>
            ))}
          </ul>
          {saffIdafa}
        </>
      )}
    </div>
  );
}

interface KhasaisItimad {
  readonly muzawwid: string;
  readonly lugha: Lugha;
}

/** Every answer the keychain can give about one provider's credential. */
type HalatItimad = 'bila' | 'jari' | 'mawjud' | 'ghaib' | 'taadhur';

const MIFTAH_HALAT_ITIMAD: Readonly<Record<HalatItimad, MiftahLugha>> = {
  bila: 'idadat.itimad.bila_muarrif',
  jari: 'amm.tahmil',
  mawjud: 'idadat.itimad.mawjud',
  ghaib: 'idadat.itimad.ghaib',
  taadhur: 'idadat.itimad.taadhur',
};

/**
 * The marker each answer draws. Only a settled answer draws a shape: while the
 * lookup is in flight, and before an identifier exists to look up, the screen
 * has claimed nothing about the keychain and must not appear to.
 */
const WASM_HALAT_ITIMAD: Readonly<Record<HalatItimad, string>> = {
  bila: 'bila',
  jari: 'bila',
  mawjud: 'mawjud',
  ghaib: 'ghaib',
  taadhur: 'taadhur',
};

/**
 * One provider's keychain credential: the status sentence, the store field and
 * the clear button. These act immediately against the keychain, never through
 * the save bar, and the secret is never rendered back.
 */
function KutlatItimad({ muzawwid, lugha }: KhasaisItimad): JSX.Element {
  const makhzan = useQueryClient();
  const [sirr, setSirr] = useState('');
  const safi = muzawwid.trim();
  const faal = safi !== '';

  const hal = useQuery<boolean, KhataJisr>({
    queryKey: mafatih.itimad(safi),
    queryFn: () => nadi('hal_itimad_muzawwid', { muzawwid: safi }),
    enabled: faal,
  });

  const khzin = useMutation<boolean, KhataJisr, { sirr: string }>({
    mutationFn: ({ sirr: qeema }) => nadi('khzin_itimad_muzawwid', { muzawwid: safi, sirr: qeema }),
    onSuccess: () => {
      setSirr('');
      void makhzan.invalidateQueries({ queryKey: mafatih.itimad(safi) });
      ansha({ naw: 'najah', nass: t('idadat.itimad.tamma_khzin', lugha) });
    },
  });

  const imsah = useMutation<boolean, KhataJisr, void>({
    mutationFn: () => nadi('imsah_itimad_muzawwid', { muzawwid: safi }),
    onSuccess: () => {
      void makhzan.invalidateQueries({ queryKey: mafatih.itimad(safi) });
      ansha({ naw: 'najah', nass: t('idadat.itimad.tamma_imsah', lugha) });
    },
  });

  const hala: HalatItimad = !faal
    ? 'bila'
    : hal.isPending
      ? 'jari'
      : hal.error !== null
        ? 'taadhur'
        : hal.data === true
          ? 'mawjud'
          : 'ghaib';

  return (
    <div className="idadat__itimad">
      <div className="idadat__itimad-hala">
        <span className="idadat__itimad-unwan">{t('idadat.itimad.unwan', lugha)}</span>
        <span
          className={`idadat__itimad-natija idadat__itimad-natija--${WASM_HALAT_ITIMAD[hala]}`}
          role="status"
        >
          {hala === 'jari' ? (
            <>
              <span
                className="idadat__haykal-satr idadat__haykal-satr--hala zuhur-muakhkhar"
                aria-hidden="true"
              />
              <span className="khafi">{t(MIFTAH_HALAT_ITIMAD[hala], lugha)}</span>
            </>
          ) : (
            t(MIFTAH_HALAT_ITIMAD[hala], lugha)
          )}
        </span>
      </div>
      <div className="idadat__itimad-afal">
        <input
          id={`idadat-itimad-${muzawwid}`}
          className="idadat__haql idadat__haql--sirr mono-ltr"
          type="password"
          dir="ltr"
          autoComplete="off"
          value={sirr}
          aria-label={t('idadat.itimad.sirr', lugha)}
          onChange={(hadath) => {
            setSirr(hadath.target.value);
          }}
        />
        <button
          type="button"
          className="zir"
          aria-disabled={!faal || sirr === ''}
          aria-busy={khzin.isPending}
          onClick={() => {
            if (faal && sirr !== '' && !khzin.isPending) {
              khzin.mutate({ sirr });
            }
          }}
        >
          {t(khzin.isPending ? 'idadat.itimad.jari_khzin' : 'idadat.itimad.khzin', lugha)}
        </button>
        {/* The one control on this screen that destroys something the moment it
            is pressed: the save bar cannot undo a cleared keychain entry. */}
        <button
          type="button"
          className="zir zir--khatar"
          aria-disabled={!faal}
          aria-busy={imsah.isPending}
          onClick={() => {
            if (faal && !imsah.isPending) {
              imsah.mutate();
            }
          }}
        >
          {t(imsah.isPending ? 'idadat.itimad.jari_imsah' : 'idadat.itimad.imsah', lugha)}
        </button>
      </div>
      <Zuhur maftuh={khzin.error !== null} asl="mahall">
        {khzin.error === null ? null : (
          <KutlatKhata unwan={t('luba.khata.amal', lugha)} khata={khzin.error} lugha={lugha} />
        )}
      </Zuhur>
      <Zuhur maftuh={imsah.error !== null} asl="mahall">
        {imsah.error === null ? null : (
          <KutlatKhata unwan={t('luba.khata.amal', lugha)} khata={imsah.error} lugha={lugha} />
        )}
      </Zuhur>
    </div>
  );
}

interface TalabHifz {
  readonly jadeed: Idadat;
  readonly sabiq: Idadat;
}

export function IdadatShasha(): JSX.Element {
  const makhzan = useQueryClient();
  const sajjilKhatwa = useTaraju((halat) => halat.sajjil);

  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  // The working copy: loaded once when the tree first arrives, edited locally,
  // written back in one save. Nothing below mutates the query cache directly.
  const bayanat = idadat.data;
  const [nuskha, setNuskha] = useState<Idadat | null>(null);
  useEffect(() => {
    if (bayanat !== undefined) {
      setNuskha((hali) => hali ?? bayanat);
    }
  }, [bayanat]);

  const haddid = useCallback((tabdeel: (hali: Idadat) => Idadat): void => {
    setNuskha((hali) => (hali === null ? hali : tabdeel(hali)));
  }, []);

  // Which sections are folded. Open is the default, so nothing is hidden from
  // a reader who never touches a heading; the set holds only the exceptions.
  const [maghluqa, setMaghluqa] = useState<ReadonlySet<MiftahQism>>(() => new Set());
  const tabdeelQism = useCallback((muarrif: MiftahQism): void => {
    setMaghluqa((hali) => {
      const jadeed = new Set(hali);
      if (!jadeed.delete(muarrif)) {
        jadeed.add(muarrif);
      }
      return jadeed;
    });
  }, []);
  const iftahQism = useCallback((muarrif: MiftahQism): void => {
    setMaghluqa((hali) => {
      if (!hali.has(muarrif)) {
        return hali;
      }
      const jadeed = new Set(hali);
      jadeed.delete(muarrif);
      return jadeed;
    });
  }, []);

  const mutaghayyir = useMemo(
    () =>
      nuskha !== null &&
      bayanat !== undefined &&
      JSON.stringify(nuskha) !== JSON.stringify(bayanat),
    [nuskha, bayanat],
  );

  const halatQaima = useMemo(
    () => (nuskha === null ? null : halatMuzawwidin(nuskha.muzawwidun)),
    [nuskha],
  );
  const athar = halatQaima === null ? null : ATHAR_MUZAWWIDIN[halatQaima];

  const hifz = useMutation<Idadat, KhataJisr, TalabHifz>({
    mutationFn: ({ jadeed }) => nadi('haddith_idadat', { idadat: jadeed }),
    onSuccess: (shajara, { sabiq }) => {
      // The undo step writes the pre-save tree back through the same command.
      sajjilKhatwa({
        wasf: t('idadat.taraju.hifz', lugha),
        taraju: async () => {
          const qadeem = await nadi('haddith_idadat', { idadat: sabiq });
          makhzan.setQueryData(mafatih.idadat, qadeem);
          setNuskha(qadeem);
          await makhzan.invalidateQueries({ queryKey: mafatih.idadat });
        },
      });
      makhzan.setQueryData(mafatih.idadat, shajara);
      setNuskha(shajara);
      void makhzan.invalidateQueries({ queryKey: mafatih.idadat });
      // Said in the language the save just chose, because the interface has
      // switched to it by the time the notice is read. The one action is the
      // undo step registered above; a failed undo is announced, never dropped.
      const lughaJadeeda = shajara.lugha;
      ansha({
        naw: 'najah',
        nass: t('idadat.hifz.tamma', lughaJadeeda),
        amal: {
          unwan: t('taraju.zirr', lughaJadeeda),
          nafidh: () => {
            useTaraju
              .getState()
              .taraju()
              .catch((khata: unknown) => {
                if (khata instanceof KhataJisr) {
                  anshaKhatar(khata, lughaJadeeda);
                } else {
                  ansha({ naw: 'khatar', nass: t('faragh.jisr', lughaJadeeda) });
                }
              });
          },
        },
      });
    },
    // The block that explains the failure sits above a bar anchored to the
    // foot of the region, which is off-screen from anywhere but the very end.
    onError: (khata) => {
      anshaKhatar(khata, lugha);
    },
  });

  const alaHifz = useCallback((): void => {
    if (nuskha === null || bayanat === undefined || hifz.isPending || !mutaghayyir) {
      return;
    }
    hifz.mutate({ jadeed: tahdheeb(nuskha), sabiq: bayanat });
  }, [nuskha, bayanat, hifz, mutaghayyir]);

  // The palette registers once per id set, so the actions read through refs
  // that always hold the current save handler and the first named provider.
  const marjiHifz = useRef<() => void>(() => {});
  const marjiItimad = useRef<string | null>(null);
  useEffect(() => {
    marjiHifz.current = alaHifz;
    marjiItimad.current =
      nuskha?.muzawwidun.qaima.find((muzawwid) => muzawwid.muarrif.trim() !== '')?.muarrif ?? null;
  });

  // A field asked for while its section is folded does not exist yet. The
  // request waits here and is honoured on the first render that mounts it.
  const marjiTarkeez = useRef<string | null>(null);
  useEffect(() => {
    const hadaf = marjiTarkeez.current;
    if (hadaf === null) {
      return;
    }
    const unsur = document.getElementById(hadaf);
    if (unsur !== null) {
      marjiTarkeez.current = null;
      unsur.focus();
    }
  });

  // The other half of every "Open Settings" in the product. A failure that
  // names a section carries it in the address, and the screen answers by
  // unfolding that section and putting the caret on its heading — which
  // scrolls it into view — instead of dropping the reader at the top of an
  // eleven-section document to hunt for the row the failure was about. The
  // heading only exists once the tree has loaded, so the request goes through
  // the same waiting ref the palette uses rather than straight to `focus`.
  const { qism } = wajihat.useSearch();
  useEffect(() => {
    if (qism === undefined) {
      return;
    }
    iftahQism(qism);
    marjiTarkeez.current = `idadat-unwan-${qism}`;
  }, [qism, iftahQism]);

  const awamirShasha = useMemo<readonly AmrLawha[]>(
    () => [
      {
        muarrif: 'idadat.hifz',
        unwan: t('idadat.awamir.hifz', lugha),
        majal: t('shasha.idadat', lugha),
        nafidh: () => {
          marjiHifz.current();
        },
      },
      {
        muarrif: 'idadat.itimad',
        unwan: t('idadat.awamir.itimad', lugha),
        majal: t('shasha.idadat', lugha),
        nafidh: () => {
          const muarrif = marjiItimad.current;
          if (muarrif === null) {
            return;
          }
          const hadaf = `idadat-itimad-${muarrif}`;
          const unsur = document.getElementById(hadaf);
          if (unsur !== null) {
            unsur.focus();
            return;
          }
          marjiTarkeez.current = hadaf;
          iftahQism('muzawwidun');
        },
      },
    ],
    [lugha, iftahQism],
  );
  useSajjilAwamir(awamirShasha);

  const khutut = useQuery<KhattHie[], KhataJisr>({
    queryKey: mafatih.khutut,
    queryFn: () => nadi('khutut_mutaha'),
  });

  const tahdith = useQuery<HalatTahdith, KhataJisr>({
    queryKey: mafatih.tahdith,
    queryFn: () => nadi('tahaqquq_tahdith'),
    // The channel is a network round trip against a signed manifest; it is
    // asked when the screen opens, not on every focus.
    staleTime: 5 * 60_000,
  });

  // The check on its own asks nothing of the reader when the screen opens; an
  // explicit check answers where they are, whichever way it came out. A channel
  // nobody has published and a machine that was told not to go looking are
  // answers, not failures, so neither raises an alarm.
  const alaTahaqquq = (): void => {
    void tahdith.refetch().then((natija) => {
      if (natija.error !== null) {
        anshaKhatar(natija.error, lugha);
        return;
      }
      const hala = natija.data;
      if (hala === undefined) {
        return;
      }
      switch (hala.hala) {
        case 'ahdath':
          ansha({ naw: 'najah', nass: t('idadat.tahdith.ahdath', lugha) });
          return;
        case 'ghayr_manshura':
          ansha({
            naw: 'maluma',
            nass: t('idadat.tahdith.ghayr_manshura', lugha),
            tafsil: t('idadat.tahdith.ghayr_manshura_masdar', lugha, { masdar: hala.masdar }),
          });
          return;
        case 'ghayr_muttasil':
          ansha({ naw: 'maluma', nass: t('idadat.tahdith.ghayr_muttasil', lugha) });
          return;
        default:
          ansha({
            naw: 'maluma',
            nass: t('idadat.tahdith.mutah', lugha, {
              isdar: hala.isdar,
              hajm: munassiq.hajm(hala.hajm),
            }),
          });
      }
    });
  };

  const nazzil = useMutation<string, KhataJisr, void>({
    mutationFn: () => nadi('nazzil_tahdith'),
    onSuccess: (masar) => {
      ansha({ naw: 'najah', nass: t('idadat.tahdith.jahiz', lugha), tafsil: masar });
    },
    onError: (khata) => {
      anshaKhatar(khata, lugha);
    },
  });

  const [masarKhatt, setMasarKhatt] = useState('');
  const istirad = useMutation<KhattHie, KhataJisr, { masar: string }>({
    mutationFn: ({ masar }) => nadi('ikhtar_khatt', { masar }),
    onSuccess: (khatt) => {
      setMasarKhatt('');
      void makhzan.invalidateQueries({ queryKey: mafatih.khutut });
      ansha({ naw: 'najah', nass: t('idadat.khutut.tamma', lugha, { ism: khatt.ism }) });
    },
    onError: (khata) => {
      anshaKhatar(khata, lugha);
    },
  });

  const haddidManassa = (miftah: MiftahManassa, khaam: string): void => {
    haddid((hali) => {
      const manassat: IdadatManassat = { ...hali.manassat };
      manassat[miftah] = khaam === '' ? null : khaam;
      return { ...hali, manassat };
    });
  };

  const haddidMuzawwid = (fihris: number, juz: Partial<IdadatMuzawwid>): void => {
    haddid((hali) => ({
      ...hali,
      muzawwidun: {
        ...hali.muzawwidun,
        qaima: hali.muzawwidun.qaima.map((muzawwid, ayn) =>
          ayn === fihris ? { ...muzawwid, ...juz } : muzawwid,
        ),
      },
    }));
  };

  const haddidMuarrifMuzawwid = (fihris: number, jadeed: string): void => {
    haddid((hali) => {
      const qadeem = hali.muzawwidun.qaima.at(fihris);
      if (qadeem === undefined) {
        return hali;
      }
      return {
        ...hali,
        muzawwidun: {
          iftiradi:
            hali.muzawwidun.iftiradi !== null && hali.muzawwidun.iftiradi === qadeem.muarrif
              ? jadeed === ''
                ? null
                : jadeed
              : hali.muzawwidun.iftiradi,
          qaima: hali.muzawwidun.qaima.map((muzawwid, ayn) =>
            ayn === fihris ? { ...muzawwid, muarrif: jadeed } : muzawwid,
          ),
        },
      };
    });
  };

  const adifMuzawwid = (): void => {
    haddid((hali) => ({
      ...hali,
      muzawwidun: {
        ...hali.muzawwidun,
        qaima: [...hali.muzawwidun.qaima, MUZAWWID_FARIGH],
      },
    }));
  };

  const ihdhifMuzawwid = (fihris: number): void => {
    haddid((hali) => {
      const mahdhuf = hali.muzawwidun.qaima.at(fihris);
      return {
        ...hali,
        muzawwidun: {
          iftiradi:
            mahdhuf !== undefined && hali.muzawwidun.iftiradi === mahdhuf.muarrif
              ? null
              : hali.muzawwidun.iftiradi,
          qaima: hali.muzawwidun.qaima.filter((_, ayn) => ayn !== fihris),
        },
      };
    });
  };

  const yuhammil = idadat.isPending || nuskha === null;
  const khututFarigha = khutut.data === undefined || khutut.data.length === 0;

  // The import row is the font list's footer and the empty state's action both,
  // so it is built once and placed inside whichever of the two is rendered. The
  // ghost text is dropped in the empty state, where the sentence above it
  // already says what the field takes.
  const saffIstirad = (
    <div className="idadat__saff-idafa">
      <input
        className="idadat__haql mono-ltr"
        dir="ltr"
        value={masarKhatt}
        placeholder={khututFarigha ? '' : t('idadat.khutut.istirad', lugha)}
        aria-label={t('idadat.khutut.istirad', lugha)}
        onChange={(hadath) => {
          setMasarKhatt(hadath.target.value);
        }}
      />
      <button
        type="button"
        className="zir"
        aria-disabled={masarKhatt.trim() === ''}
        aria-busy={istirad.isPending}
        onClick={() => {
          const safi = masarKhatt.trim();
          if (!istirad.isPending && safi !== '') {
            istirad.mutate({ masar: safi });
          }
        }}
      >
        {t(istirad.isPending ? 'idadat.khutut.jari_istirad' : 'idadat.khutut.istawrid', lugha)}
      </button>
    </div>
  );

  const wajh = idadat.error !== null && nuskha === null ? 'khata' : nuskha === null ? 'tahmil' : 'jahiz';

  // The update answer moves only when its state does; a refetch of the same
  // state keeps the key and so paints in place.
  const wajhTahdith = tahdith.isPending
    ? 'tahmil'
    : tahdith.error !== null
      ? 'khata'
      : (tahdith.data?.hala ?? 'ahdath');

  // Five answers, not an offer and an error: a channel nobody has published and
  // a machine that was told not to go looking are states with their own
  // sentences, and only a source that could not be reached is worth a retry.
  const rasmTahdith = (): ReactNode => {
    if (tahdith.isPending) {
      return (
        <>
          <span
            className="idadat__haykal-satr idadat__haykal-satr--jumla zuhur-muakhkhar"
            aria-hidden="true"
          />
          <span className="khafi">{t('idadat.tahdith.jari', lugha)}</span>
        </>
      );
    }
    if (tahdith.error !== null) {
      return (
        <KutlatKhata
          unwan={t('idadat.tahdith.taadhur', lugha)}
          khata={tahdith.error}
          lugha={lugha}
          aada={() => {
            void tahdith.refetch();
          }}
        />
      );
    }
    const hala = tahdith.data;
    if (hala === undefined || hala.hala === 'ahdath') {
      return <p className="idadat__nass-hadi">{t('idadat.tahdith.ahdath', lugha)}</p>;
    }
    if (hala.hala === 'ghayr_manshura') {
      return (
        <>
          <p className="idadat__nass-hadi">{t('idadat.tahdith.ghayr_manshura', lugha)}</p>
          <p className="idadat__nass-hadi mono-ltr" dir="ltr">
            {t('idadat.tahdith.ghayr_manshura_masdar', lugha, { masdar: hala.masdar })}
          </p>
        </>
      );
    }
    if (hala.hala === 'ghayr_muttasil') {
      return <p className="idadat__nass-hadi">{t('idadat.tahdith.ghayr_muttasil', lugha)}</p>;
    }
    return (
      <>
        <p className="idadat__nass-hadi">
          {t('idadat.tahdith.mutah', lugha, {
            isdar: hala.isdar,
            hajm: munassiq.hajm(hala.hajm),
          })}
        </p>
        {hala.qabil_lil_tabdil ? (
          <div className="idadat__saff-afal">
            <button
              type="button"
              className="zir zir--tamyeez"
              aria-busy={nazzil.isPending}
              onClick={() => {
                if (!nazzil.isPending) {
                  nazzil.mutate();
                }
              }}
            >
              {t(nazzil.isPending ? 'idadat.tahdith.jari_tanzil' : 'idadat.tahdith.nazzil', lugha)}
            </button>
          </div>
        ) : (
          <p className="idadat__nass-hadi">{t('idadat.tahdith.mudar', lugha)}</p>
        )}
        <Zuhur maftuh={nazzil.data !== undefined} asl="fawq" className="idadat__najah">
          {nazzil.data === undefined ? null : t('idadat.tahdith.jahiz', lugha)}
        </Zuhur>
        <Zuhur maftuh={nazzil.error !== null} asl="mahall">
          {nazzil.error === null ? null : (
            <KutlatKhata
              unwan={t('idadat.tahdith.taadhur_tanzil', lugha)}
              khata={nazzil.error}
              lugha={lugha}
            />
          )}
        </Zuhur>
      </>
    );
  };

  return (
    <div className="idadat">
      <RaasShasha
        rujoo={{ ila: 'maktaba' }}
        nassRujoo={t('idadat.raji', lugha)}
        unwan={t('shasha.idadat', lugha)}
        tafasil={
          mutaghayyir ? (
            <span className="idadat__hala-hifz idadat__hala-hifz--mutaghayyir">
              {t('idadat.hifz.mutaghayyir', lugha)}
            </span>
          ) : null
        }
      />

      <Mashhad miftah={wajh} className="idadat__jism">
        {idadat.error !== null && nuskha === null ? (
          <KutlatKhata
            unwan={t('idadat.khata.tahmil', lugha)}
            khata={idadat.error}
            lugha={lugha}
            aada={() => {
              void idadat.refetch();
            }}
          />
        ) : nuskha === null ? (
          <HaykalIdadat />
        ) : (
          <>
            <QismIdadat
              muarrif="ard"
              unwan={t('idadat.ard.unwan', lugha)}
              maftuh={!maghluqa.has('ard')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ard.lugha', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.lugha}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as Lugha;
                    haddid((hali) => ({ ...hali, lugha: qeema }));
                  }}
                >
                  {LUGHAT.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_LUGHAT[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ard.arqam', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.arqam}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as NizamArqam;
                    haddid((hali) => ({ ...hali, arqam: qeema }));
                  }}
                >
                  {ANZIMAT_ARQAM.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_ARQAM[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ard.sima', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.sima}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as Sima;
                    haddid((hali) => ({ ...hali, sima: qeema }));
                  }}
                >
                  {SIMAT.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_SIMA[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ard.kathafa', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.kathafa}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as Kathafa;
                    haddid((hali) => ({ ...hali, kathafa: qeema }));
                  }}
                >
                  {KATHAFAT.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_KATHAFA[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.tabayun_aali}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({ ...hali, tabayun_aali: qeema }));
                  }}
                />
                {t('idadat.ard.tabayun', lugha)}
              </label>
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.ihtiram_taqleel_haraka}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({ ...hali, ihtiram_taqleel_haraka: qeema }));
                  }}
                />
                {t('idadat.ard.taqleel_haraka', lugha)}
              </label>
            </QismIdadat>

            <QismIdadat
              muarrif="manassat"
              unwan={t('idadat.manassat.unwan', lugha)}
              sharh={t('idadat.manassat.sharh', lugha)}
              maftuh={!maghluqa.has('manassat')}
              alaTabdeel={tabdeelQism}
            >
              {MANASSAT.map((manassa) => (
                <label key={manassa} className="idadat__saff">
                  <span className="idadat__tasmiya">{t(MIFTAH_MANASSA[manassa], lugha)}</span>
                  <input
                    className="idadat__haql mono-ltr"
                    dir="ltr"
                    value={nuskha.manassat[manassa] ?? ''}
                    placeholder={t('idadat.manassat.mawdi', lugha)}
                    onChange={(hadath) => {
                      haddidManassa(manassa, hadath.target.value);
                    }}
                  />
                </label>
              ))}
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.manassat.fahs_tilqai}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      manassat: { ...hali.manassat, fahs_tilqai: qeema },
                    }));
                  }}
                />
                {t('idadat.manassat.fahs_tilqai', lugha)}
              </label>
              <div className="idadat__saff idadat__saff--kutla">
                <span className="idadat__tasmiya">{t('idadat.manassat.idafiya', lugha)}</span>
                <QaimatMasarat
                  qeem={nuskha.manassat.mujalladat_idafiya}
                  tasmiya={t('idadat.manassat.idafiya', lugha)}
                  mawdi={t('idadat.manassat.idafiya_mawdi', lugha)}
                  lugha={lugha}
                  alaTaghyeer={(jadeed) => {
                    haddid((hali) => ({
                      ...hali,
                      manassat: { ...hali.manassat, mujalladat_idafiya: jadeed },
                    }));
                  }}
                />
              </div>
            </QismIdadat>

            <QismIdadat
              muarrif="taareeb"
              unwan={t('idadat.taareeb.unwan', lugha)}
              sharh={t('idadat.taareeb.sharh', lugha)}
              maftuh={!maghluqa.has('taareeb')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.istibdal_lugha_rasmiya}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({ ...hali, istibdal_lugha_rasmiya: qeema }));
                  }}
                />
                {t('idadat.taareeb.istibdal', lugha)}
              </label>
              {/* The consequence, stated where the decision is made and only
                  when the decision has been made — a sentence read after the
                  fact is a sentence that was not given. */}
              <p className="idadat__mudakhkhal idadat__nass-hadi">
                {t('idadat.taareeb.athar', lugha)}
              </p>
              <Zuhur
                maftuh={nuskha.istibdal_lugha_rasmiya}
                asl="fawq"
                className="idadat__mudakhkhal idadat__athar"
                role="status"
              >
                {t('idadat.taareeb.mufaal', lugha)}
              </Zuhur>
            </QismIdadat>

            <QismIdadat
              muarrif="takhzin"
              unwan={t('idadat.takhzin.unwan', lugha)}
              maftuh={!maghluqa.has('takhzin')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.takhzin.jidhr', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.takhzin.jidhr_ruqaa ?? ''}
                  placeholder={t('idadat.amm.farigh_iftiradi', lugha)}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      takhzin: { ...hali.takhzin, jidhr_ruqaa: qeema === '' ? null : qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.takhzin.hadd_makhbaa', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="64"
                  value={String(nuskha.takhzin.hadd_makhbaa_mb)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      takhzin: {
                        ...hali.takhzin,
                        hadd_makhbaa_mb: raqmAw(khaam, hali.takhzin.hadd_makhbaa_mb),
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.takhzin.ibqa_nusakh}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      takhzin: { ...hali.takhzin, ibqa_nusakh: qeema },
                    }));
                  }}
                />
                {t('idadat.takhzin.ibqa', lugha)}
              </label>
            </QismIdadat>

            <QismIdadat
              muarrif="khutut"
              unwan={t('idadat.khutut.unwan', lugha)}
              maftuh={!maghluqa.has('khutut')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.khutut.wajiha', lugha)}</span>
                <input
                  className="idadat__haql"
                  dir="auto"
                  value={nuskha.khutut.khatt_wajiha}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      khutut: { ...hali.khutut, khatt_wajiha: qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.khutut.luba', lugha)}</span>
                <input
                  className="idadat__haql"
                  dir="auto"
                  value={nuskha.khutut.khatt_luba_iftiradi}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      khutut: { ...hali.khutut, khatt_luba_iftiradi: qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">
                  {t('idadat.khutut.masar_mustakhdim', lugha)}
                </span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.khutut.masar_khutut_mustakhdim ?? ''}
                  placeholder={t('idadat.amm.farigh_iftiradi', lugha)}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      khutut: {
                        ...hali.khutut,
                        masar_khutut_mustakhdim: qeema === '' ? null : qeema,
                      },
                    }));
                  }}
                />
              </label>
              <div className="idadat__saff idadat__saff--kutla">
                <span className="idadat__tasmiya">{t('idadat.khutut.mawjuda', lugha)}</span>
                <div className="idadat__qaima-masarat">
                  {khutut.isPending ? (
                    <HaykalQaima />
                  ) : khutut.error !== null ? (
                    <KutlatKhata
                      unwan={t('idadat.khutut.taadhur', lugha)}
                      khata={khutut.error}
                      lugha={lugha}
                      aada={() => {
                        void khutut.refetch();
                      }}
                    />
                  ) : khutut.data === undefined || khutut.data.length === 0 ? (
                    <HalatFarigha
                      unwan={t('idadat.khutut.la_shay', lugha)}
                      nass={t('idadat.khutut.istirad', lugha)}
                    >
                      {saffIstirad}
                    </HalatFarigha>
                  ) : (
                    <>
                      <ul className="idadat__khutut">
                        {khutut.data.map((khatt) => (
                          <li key={khatt.ism} className="idadat__khatt">
                            <span className="idadat__khatt-ism" dir="auto">
                              {khatt.ism}
                            </span>
                            <span
                              className={
                                khatt.arabi
                                  ? 'idadat__wasm idadat__wasm--arabi'
                                  : 'idadat__wasm idadat__wasm--latini'
                              }
                            >
                              {t(
                                khatt.arabi ? 'idadat.khutut.arabi' : 'idadat.khutut.latini',
                                lugha,
                              )}
                            </span>
                            <span className="idadat__khatt-hajm">{munassiq.hajm(khatt.hajm)}</span>
                          </li>
                        ))}
                      </ul>
                      {saffIstirad}
                    </>
                  )}
                  <Zuhur maftuh={istirad.error !== null} asl="mahall">
                    {istirad.error === null ? null : (
                      <KutlatKhata
                        unwan={t('idadat.khutut.taadhur_istirad', lugha)}
                        khata={istirad.error}
                        lugha={lugha}
                      />
                    )}
                  </Zuhur>
                  <Zuhur
                    maftuh={istirad.data !== undefined}
                    asl="fawq"
                    className="idadat__najah"
                    role="status"
                  >
                    {istirad.data === undefined
                      ? null
                      : t('idadat.khutut.tamma', lugha, { ism: istirad.data.ism })}
                  </Zuhur>
                </div>
              </div>
            </QismIdadat>

            <QismIdadat
              muarrif="muzawwidun"
              unwan={t('idadat.muzawwidun.unwan', lugha)}
              maftuh={!maghluqa.has('muzawwidun')}
              alaTabdeel={tabdeelQism}
            >
              {/* What the current list means, drawn from the working copy so it
                  follows an enable, a rename and a removal without a save. There
                  is no empty state: the built-in free provider is always the last
                  row, so a list with nothing of the user's own is a list of one,
                  and the sentence above it says that the one is doing the work. */}
              <Zuhur
                maftuh={athar !== null}
                asl="fawq"
                className="idadat__mudakhkhal idadat__athar"
                role="status"
              >
                {athar === null ? null : t(athar, lugha)}
              </Zuhur>
              <ul className="idadat__muzawwidun">
                {nuskha.muzawwidun.qaima.map((muzawwid, fihris) => (
                  <li key={String(fihris)} className="idadat__muzawwid">
                    <div className="idadat__muzawwid-raas">
                      <h3 className="idadat__muzawwid-unwan">
                        {t(MIFTAH_NAW[muzawwid.naw], lugha)}
                      </h3>
                      {muzawwid.muarrif.trim() === '' ? null : (
                        <span className="idadat__muzawwid-muarrif mono-ltr">
                          {muzawwid.muarrif}
                        </span>
                      )}
                      <button
                        type="button"
                        className="zir idadat__muzawwid-izala"
                        onClick={() => {
                          ihdhifMuzawwid(fihris);
                        }}
                      >
                        {t('idadat.muzawwidun.izala', lugha)}
                      </button>
                    </div>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.muarrif', lugha)}
                      </span>
                      <input
                        className="idadat__haql mono-ltr"
                        dir="ltr"
                        value={muzawwid.muarrif}
                        onChange={(hadath) => {
                          haddidMuarrifMuzawwid(fihris, hadath.target.value);
                        }}
                      />
                    </label>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.naw', lugha)}
                      </span>
                      <select
                        className="idadat__haql"
                        value={muzawwid.naw}
                        onChange={(hadath) => {
                          haddidMuzawwid(fihris, {
                            naw: hadath.target.value as NawMuzawwid,
                          });
                        }}
                      >
                        {ANWA_MUZAWWID.map((naw) => (
                          <option key={naw} value={naw}>
                            {t(MIFTAH_NAW[naw], lugha)}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.namudhaj', lugha)}
                      </span>
                      <input
                        className="idadat__haql mono-ltr"
                        dir="ltr"
                        value={muzawwid.namudhaj}
                        onChange={(hadath) => {
                          haddidMuzawwid(fihris, { namudhaj: hadath.target.value });
                        }}
                      />
                    </label>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.asas', lugha)}
                      </span>
                      <input
                        className="idadat__haql mono-ltr"
                        dir="ltr"
                        value={muzawwid.asas ?? ''}
                        onChange={(hadath) => {
                          const qeema = hadath.target.value;
                          haddidMuzawwid(fihris, { asas: qeema === '' ? null : qeema });
                        }}
                      />
                    </label>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.hadd', lugha)}
                      </span>
                      <input
                        className="idadat__haql idadat__haql--raqm mono-ltr"
                        type="number"
                        dir="ltr"
                        min="1"
                        value={String(muzawwid.hadd_talabat)}
                        onChange={(hadath) => {
                          haddidMuzawwid(fihris, {
                            hadd_talabat: raqmAw(hadath.target.value, muzawwid.hadd_talabat),
                          });
                        }}
                      />
                    </label>
                    <label className="idadat__saff">
                      <span className="idadat__tasmiya">
                        {t('idadat.muzawwidun.mizaniya', lugha)}
                      </span>
                      <input
                        className="idadat__haql idadat__haql--raqm mono-ltr"
                        type="number"
                        dir="ltr"
                        min="0"
                        step="0.5"
                        value={muzawwid.mizaniya === null ? '' : String(muzawwid.mizaniya)}
                        onChange={(hadath) => {
                          const khaam = hadath.target.value;
                          if (khaam.trim() === '') {
                            haddidMuzawwid(fihris, { mizaniya: null });
                            return;
                          }
                          const qeema = Number(khaam);
                          if (Number.isFinite(qeema)) {
                            haddidMuzawwid(fihris, { mizaniya: qeema });
                          }
                        }}
                      />
                    </label>
                    <label className="idadat__ikhtiyar">
                      <input
                        type="checkbox"
                        checked={muzawwid.mufaal}
                        onChange={(hadath) => {
                          haddidMuzawwid(fihris, { mufaal: hadath.target.checked });
                        }}
                      />
                      {t('idadat.muzawwidun.mufaal', lugha)}
                    </label>
                    <label className="idadat__ikhtiyar">
                      <input
                        type="radio"
                        name="idadat-muzawwid-iftiradi"
                        checked={
                          muzawwid.muarrif !== '' &&
                          nuskha.muzawwidun.iftiradi === muzawwid.muarrif
                        }
                        onChange={() => {
                          if (muzawwid.muarrif !== '') {
                            haddid((hali) => ({
                              ...hali,
                              muzawwidun: {
                                ...hali.muzawwidun,
                                iftiradi: muzawwid.muarrif,
                              },
                            }));
                          }
                        }}
                      />
                      {t('idadat.muzawwidun.iftiradi', lugha)}
                    </label>
                    <KutlatItimad muzawwid={muzawwid.muarrif} lugha={lugha} />
                  </li>
                ))}
                <MuzawwidMudmaj
                  lugha={lugha}
                  yustakhdam={halatQaima === 'faragh' || halatQaima === 'muattala'}
                />
              </ul>
              <div className="idadat__saff-afal">
                <button type="button" className="zir" onClick={adifMuzawwid}>
                  {t('idadat.muzawwidun.adif', lugha)}
                </button>
              </div>
            </QismIdadat>

            <QismIdadat
              muarrif="masadir"
              unwan={t('idadat.masadir.unwan', lugha)}
              maftuh={!maghluqa.has('masadir')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.masadir.rasmi', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.masadir.rasmi}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, rasmi: qeema },
                    }));
                  }}
                />
              </label>
              <div className="idadat__saff idadat__saff--kutla">
                <span className="idadat__tasmiya">{t('idadat.masadir.maraya', lugha)}</span>
                <QaimatMasarat
                  qeem={nuskha.masadir.maraya}
                  tasmiya={t('idadat.masadir.maraya', lugha)}
                  mawdi={t('idadat.masadir.maraya_mawdi', lugha)}
                  lugha={lugha}
                  alaTaghyeer={(jadeed) => {
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, maraya: jadeed },
                    }));
                  }}
                />
              </div>
              <div className="idadat__saff idadat__saff--kutla">
                <span className="idadat__tasmiya">{t('idadat.masadir.mahalliya', lugha)}</span>
                <QaimatMasarat
                  qeem={nuskha.masadir.mahalliya}
                  tasmiya={t('idadat.masadir.mahalliya', lugha)}
                  mawdi={t('idadat.masadir.mahalliya_mawdi', lugha)}
                  lugha={lugha}
                  alaTaghyeer={(jadeed) => {
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, mahalliya: jadeed },
                    }));
                  }}
                />
              </div>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">
                  {t('idadat.masadir.mustawda_taqdeem', lugha)}
                </span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.masadir.mustawda_taqdeem ?? ''}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      masadir: {
                        ...hali.masadir,
                        mustawda_taqdeem: qeema === '' ? null : qeema,
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.masadir.muarrif_amil', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.masadir.muarrif_amil ?? ''}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, muarrif_amil: qeema === '' ? null : qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.masadir.rabt_tajheez', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.masadir.rabt_tajheez ?? ''}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, rabt_tajheez: qeema === '' ? null : qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.masadir.fatra', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="5"
                  value={String(nuskha.masadir.fatra_tahdith)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      masadir: {
                        ...hali.masadir,
                        fatra_tahdith: raqmAw(khaam, hali.masadir.fatra_tahdith),
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.masadir.wadaa_ghayr_muttasil}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      masadir: { ...hali.masadir, wadaa_ghayr_muttasil: qeema },
                    }));
                  }}
                />
                {t('idadat.masadir.ghayr_muttasil', lugha)}
              </label>
            </QismIdadat>

            <QismIdadat
              muarrif="tahdith"
              unwan={t('idadat.tahdith.unwan', lugha)}
              maftuh={!maghluqa.has('tahdith')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.tahdith.tilqai}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      tahdith: { ...hali.tahdith, tilqai: qeema },
                    }));
                  }}
                />
                {t('idadat.tahdith.tilqai', lugha)}
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tahdith.qanat', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.tahdith.qanat}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as QanatTahdith;
                    haddid((hali) => ({
                      ...hali,
                      tahdith: { ...hali.tahdith, qanat: qeema },
                    }));
                  }}
                >
                  {QANAWAT.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_QANAT[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.tahdith.fahs_ind_bad}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      tahdith: { ...hali.tahdith, fahs_ind_bad: qeema },
                    }));
                  }}
                />
                {t('idadat.tahdith.fahs_ind_bad', lugha)}
              </label>
              <div className="idadat__saff-afal idadat__mudakhkhal">
                <button
                  type="button"
                  className="zir"
                  aria-busy={tahdith.isFetching}
                  onClick={alaTahaqquq}
                >
                  {t('idadat.tahdith.tahaqqaq', lugha)}
                </button>
              </div>
              <div className="idadat__tahdith-hala idadat__mudakhkhal" role="status">
                <Mashhad miftah={wajhTahdith} className="idadat__tahdith-mashhad">
                  {rasmTahdith()}
                </Mashhad>
              </div>
            </QismIdadat>

            <QismIdadat
              muarrif="tashkhis"
              unwan={t('idadat.tashkhis.unwan', lugha)}
              maftuh={!maghluqa.has('tashkhis')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tashkhis.mustawa', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.tashkhis.mustawa}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MustawaSijill;
                    haddid((hali) => ({
                      ...hali,
                      tashkhis: { ...hali.tashkhis, mustawa: qeema },
                    }));
                  }}
                >
                  {MUSTAWAYAT.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_MUSTAWA[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tashkhis.ayyam', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="1"
                  value={String(nuskha.tashkhis.ayyam_hifz)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      tashkhis: {
                        ...hali.tashkhis,
                        ayyam_hifz: raqmAw(khaam, hali.tashkhis.ayyam_hifz),
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tashkhis.hadd_hajm', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="1"
                  value={String(nuskha.tashkhis.hadd_hajm_mb)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      tashkhis: {
                        ...hali.tashkhis,
                        hadd_hajm_mb: raqmAw(khaam, hali.tashkhis.hadd_hajm_mb),
                      },
                    }));
                  }}
                />
              </label>
            </QismIdadat>

            <QismIdadat
              muarrif="tabaqa"
              unwan={t('idadat.tabaqa.unwan', lugha)}
              maftuh={!maghluqa.has('tabaqa')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__ikhtiyar">
                <input
                  type="checkbox"
                  checked={nuskha.tabaqa.mufaal}
                  onChange={(hadath) => {
                    const qeema = hadath.target.checked;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: { ...hali.tabaqa, mufaal: qeema },
                    }));
                  }}
                />
                {t('idadat.tabaqa.mufaal', lugha)}
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tabaqa.muharrik', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.tabaqa.muharrik_qira}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MuharrikQira;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: { ...hali.tabaqa, muharrik_qira: qeema },
                    }));
                  }}
                >
                  {MUHARRIKAT_QIRA.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_MUHARRIK[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tabaqa.shaffafiya', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="0"
                  max="1"
                  step="0.05"
                  value={String(nuskha.tabaqa.shaffafiya)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: {
                        ...hali.tabaqa,
                        shaffafiya: raqmAw(khaam, kasr(hali.tabaqa.shaffafiya)),
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tabaqa.hajm_khatt', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="1"
                  value={String(nuskha.tabaqa.hajm_khatt)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: {
                        ...hali.tabaqa,
                        hajm_khatt: raqmAw(khaam, kasr(hali.tabaqa.hajm_khatt)),
                      },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tabaqa.mawqi', lugha)}</span>
                <select
                  className="idadat__haql"
                  value={nuskha.tabaqa.mawqi}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value as MawqiTabaqa;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: { ...hali.tabaqa, mawqi: qeema },
                    }));
                  }}
                >
                  {MAWAQI_TABAQA.map((qeema) => (
                    <option key={qeema} value={qeema}>
                      {t(MIFTAH_MAWQI[qeema], lugha)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.tabaqa.tul_sijill', lugha)}</span>
                <input
                  className="idadat__haql idadat__haql--raqm mono-ltr"
                  type="number"
                  dir="ltr"
                  min="0"
                  value={String(nuskha.tabaqa.tul_sijill_qira)}
                  onChange={(hadath) => {
                    const khaam = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      tabaqa: {
                        ...hali.tabaqa,
                        tul_sijill_qira: raqmAw(khaam, hali.tabaqa.tul_sijill_qira),
                      },
                    }));
                  }}
                />
              </label>
            </QismIdadat>

            <QismIdadat
              muarrif="ikhtisarat"
              unwan={t('idadat.ikhtisarat.unwan', lugha)}
              sharh={t('idadat.ikhtisarat.sharh', lugha)}
              maftuh={!maghluqa.has('ikhtisarat')}
              alaTabdeel={tabdeelQism}
            >
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ikhtisarat.lawha', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.ikhtisarat.lawha}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      ikhtisarat: { ...hali.ikhtisarat, lawha: qeema },
                    }));
                  }}
                />
              </label>
              <label className="idadat__saff">
                <span className="idadat__tasmiya">{t('idadat.ikhtisarat.taraju', lugha)}</span>
                <input
                  className="idadat__haql mono-ltr"
                  dir="ltr"
                  value={nuskha.ikhtisarat.taraju}
                  onChange={(hadath) => {
                    const qeema = hadath.target.value;
                    haddid((hali) => ({
                      ...hali,
                      ikhtisarat: { ...hali.ikhtisarat, taraju: qeema },
                    }));
                  }}
                />
              </label>
            </QismIdadat>

            {/* Above the bar, not below it: the bar is anchored to the foot of
                the region, and a failure that appears under it is a failure the
                reader has to scroll past the bar to find. */}
            <Zuhur maftuh={hifz.error !== null} asl="mahall">
              {hifz.error === null ? null : (
                <KutlatKhata
                  unwan={t('idadat.hifz.taadhur', lugha)}
                  khata={hifz.error}
                  lugha={lugha}
                />
              )}
            </Zuhur>
            <div className="idadat__shareet-hifz">
              <button
                type="button"
                className="zir zir--tamyeez"
                aria-disabled={!mutaghayyir}
                aria-busy={hifz.isPending}
                onClick={alaHifz}
              >
                {t(hifz.isPending ? 'idadat.hifz.jari' : 'idadat.hifz.zir', lugha)}
              </button>
              <button
                type="button"
                className="zir"
                aria-disabled={!mutaghayyir || hifz.isPending}
                onClick={() => {
                  if (mutaghayyir && !hifz.isPending && bayanat !== undefined) {
                    setNuskha(bayanat);
                    hifz.reset();
                  }
                }}
              >
                {t('idadat.hifz.istirja', lugha)}
              </button>
              {mutaghayyir ? (
                <span className="idadat__hala-hifz idadat__hala-hifz--mutaghayyir">
                  {t('idadat.hifz.mutaghayyir', lugha)}
                </span>
              ) : hifz.isSuccess ? (
                <span className="idadat__hala-hifz idadat__hala-hifz--tamma" role="status">
                  {t('idadat.hifz.tamma', lugha)}
                </span>
              ) : null}
            </div>
          </>
        )}
      </Mashhad>

      <p className="khafi" role="status">
        {yuhammil ? t('amm.tahmil', lugha) : ''}
      </p>
    </div>
  );
}
