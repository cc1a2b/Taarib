// الهيكل — the shell every screen is mounted in: the window strip, the rail, the content, the status strip, and the layers that float over all of them.

import { useQuery } from '@tanstack/react-query';
import { Outlet, useNavigate, useRouter, useRouterState } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useCallback, useEffect, useMemo } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import type { LubaAkhira } from '@/hayat/hikal';
import { useHikal } from '@/hayat/hikal';
import { mafatih } from '@/hayat/istifsar';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { useMuraqibTilqai } from '@/hayat/muraqib_tilqai';
import { nizamNafidha } from '@/hayat/nafidha';
import type { MiftahLugha } from '@/lugha/lugha';
import { t } from '@/lugha/lugha';
import { jahiziyaSaf, tasil } from '@/maktaba/jahiziya';
import type {
  HasilatMaktaba,
  JalsaHie,
  Lugha,
  MaalumatTaarib,
  TafasilLuba,
} from '@/mustalahat/awamir';
import { Janib } from '@/mukawwinat/janib';
import { LawhatAwamir } from '@/mukawwinat/lawhat_awamir';
import { Nafidha } from '@/mukawwinat/nafidha';
import { RamzTanbeeh } from '@/mukawwinat/rumuz';
import { Tanbihat } from '@/mukawwinat/tanbihat';
import { TaqaddumAam } from '@/mukawwinat/taqaddum_aam';

import './hikal.css';

/** The operating system's name, in the string set. */
function miftahNizam(nizam: MaalumatTaarib['nizam']): MiftahLugha {
  switch (nizam) {
    case 'windows':
      return 'nizam.windows';
    case 'linux':
      return 'nizam.linux';
    case 'mac':
      return 'nizam.mac';
  }
}

/** The architecture's name, in the string set. */
function miftahMimariya(mimariya: MaalumatTaarib['mimariya']): MiftahLugha {
  switch (mimariya) {
    case 'x86':
      return 'mimariya.x86';
    case 'x8664':
      return 'mimariya.x8664';
    case 'aarch64':
      return 'mimariya.aarch64';
  }
}

/** The game identity in the current address, or null on a screen that has none. */
function muarrifMin(params: unknown): string | null {
  if (typeof params !== 'object' || params === null) {
    return null;
  }
  const qeema = (params as Readonly<Record<string, unknown>>)['muarrif'];
  return typeof qeema === 'string' && qeema !== '' ? qeema : null;
}

/** Each route's screen name, for the window strip. A route absent here is one of the router's own two pages. */
const ASMA_SHASHAT: Readonly<Record<string, MiftahLugha>> = {
  '/': 'shasha.maktaba',
  '/idadat': 'shasha.idadat',
  '/tashkhis': 'shasha.tashkhis',
  '/talabat': 'shasha.talabat',
  '/muraja': 'shasha.muraja',
  '/luba/$muarrif': 'shasha.luba',
  '/tilqai/$muarrif': 'shasha.tilqai',
  '/warsha/$muarrif': 'shasha.warsha',
  '/taqdeem/$muarrif': 'shasha.taqdeem',
  '/tabaqa/$muarrif': 'shasha.tabaqa',
};

/** The chord that folds the rail; the same one every editor uses for its own. */
function chordTayy(hadath: KeyboardEvent): boolean {
  return (
    (hadath.ctrlKey || hadath.metaKey) &&
    !hadath.shiftKey &&
    !hadath.altKey &&
    hadath.key.toLowerCase() === 'b'
  );
}

export interface KhasaisHikal {
  readonly lugha: Lugha;
  readonly ikhtisarLawha: string;
  readonly ikhtisarTaraju: string;
}

/**
 * The shell.
 *
 * It holds the three things that are true on every screen — who this session
 * is, what build is running, and which game the address is about — and draws
 * the frame around the screen from them. The screen itself is the outlet in
 * the middle, which is the only part of the window a navigation moves.
 */
export function Hikal({ lugha, ikhtisarLawha, ikhtisarTaraju }: KhasaisHikal): JSX.Element {
  const router = useRouter();
  const intiqal = useNavigate();

  const janibMatwi = useHikal((halat) => halat.janib_matwi);
  const baddilJanib = useHikal((halat) => halat.baddilJanib);
  const akhirLuba = useHikal((halat) => halat.akhir_luba);
  const sajjilLuba = useHikal((halat) => halat.sajjilLuba);

  const muarrif = useRouterState({
    select: (halat) => muarrifMin(halat.matches.at(-1)?.params),
  });
  const masarHali = useRouterState({
    select: (halat) => halat.matches.at(-1)?.routeId ?? '/',
  });

  const maalumat = useQuery<MaalumatTaarib, KhataJisr>({
    queryKey: mafatih.maalumat,
    queryFn: () => nadi('maalumat_taarib'),
  });

  const jalsa = useQuery<JalsaHie, KhataJisr>({
    queryKey: mafatih.jalsa,
    queryFn: () => nadi('jalsati'),
  });

  // The library, for one fact per game: whether the automatic run may be
  // offered. The library screen asks the same question under the same key, so
  // this costs nothing there, and a launch that opens straight onto a game
  // starts the scan the person will want a moment later anyway.
  const maktaba = useQuery<HasilatMaktaba, KhataJisr>({
    queryKey: mafatih.maktaba,
    queryFn: () => nadi('maktaba'),
    staleTime: 5 * 60_000,
  });

  // The open game's name, from the same answer its screen reads, so the rail
  // and the screen cannot disagree about which game this is.
  const tafasil = useQuery<TafasilLuba, KhataJisr>({
    queryKey: mafatih.tafasil(muarrif ?? ''),
    queryFn: () => nadi('tafasil_luba', { muarrif: muarrif ?? '' }),
    enabled: muarrif !== null,
  });

  useEffect(() => {
    const bayanat = tafasil.data;
    if (muarrif !== null && bayanat !== undefined && bayanat.muarrif === muarrif) {
      sajjilLuba({ muarrif, ism: bayanat.ism });
    }
  }, [muarrif, tafasil.data, sajjilLuba]);

  /**
   * The game the rail's second run points at.
   *
   * The open one while a game screen is up — named from its own answer when
   * that has landed, from the library's row when it has not, and from the
   * address alone until either does, so the run is live from the first frame
   * rather than after a round trip. The last one opened otherwise.
   */
  const lubaJanib = useMemo<LubaAkhira | null>(() => {
    if (muarrif === null) {
      return akhirLuba;
    }
    const minTafasil = tafasil.data?.muarrif === muarrif ? tafasil.data.ism : null;
    const minMaktaba = maktaba.data?.alaab.find((saf) => saf.muarrif === muarrif)?.ism ?? null;
    const minAkhir = akhirLuba?.muarrif === muarrif ? akhirLuba.ism : null;
    return { muarrif, ism: minTafasil ?? minMaktaba ?? minAkhir ?? muarrif };
  }, [muarrif, akhirLuba, tafasil.data, maktaba.data]);

  // A game's name for a notice about it, from whatever the shell already
  // holds; the identity itself when nothing does, which is still an answer.
  const ismLuba = useCallback(
    (muarrifLuba: string): string =>
      maktaba.data?.alaab.find((saf) => saf.muarrif === muarrifLuba)?.ism ??
      (akhirLuba?.muarrif === muarrifLuba ? akhirLuba.ism : muarrifLuba),
    [maktaba.data, akhirLuba],
  );
  useMuraqibTilqai(lugha, ismLuba);

  const tilqaiMasmuh = useMemo<boolean | null>(() => {
    if (lubaJanib === null || maktaba.data === undefined) {
      return null;
    }
    const saf = maktaba.data.alaab.find((sijill) => sijill.muarrif === lubaJanib.muarrif);
    return saf === undefined ? null : tasil(jahiziyaSaf(saf));
  }, [lubaJanib, maktaba.data]);

  const sababJalsa =
    jalsa.error === null ? null : (jalsa.error.nass(lugha) ?? t('faragh.jisr', lugha));
  const sababMaalumat =
    maalumat.error === null ? null : (maalumat.error.nass(lugha) ?? t('faragh.jisr', lugha));

  // The fold, and the two history keys every desktop application answers:
  // Alt with an arrow, and the two extra buttons on a mouse. Physical arrows
  // on purpose — the platform's own convention does not mirror them.
  useEffect(() => {
    const alaMiftah = (hadath: KeyboardEvent): void => {
      if (chordTayy(hadath)) {
        hadath.preventDefault();
        baddilJanib();
        return;
      }
      if (!hadath.altKey || hadath.ctrlKey || hadath.metaKey || hadath.shiftKey) {
        return;
      }
      if (hadath.key === 'ArrowLeft') {
        hadath.preventDefault();
        router.history.back();
      } else if (hadath.key === 'ArrowRight') {
        hadath.preventDefault();
        router.history.forward();
      }
    };
    const alaFaara = (hadath: MouseEvent): void => {
      if (hadath.button === 3) {
        hadath.preventDefault();
        router.history.back();
      } else if (hadath.button === 4) {
        hadath.preventDefault();
        router.history.forward();
      }
    };
    window.addEventListener('keydown', alaMiftah);
    window.addEventListener('mouseup', alaFaara);
    return () => {
      window.removeEventListener('keydown', alaMiftah);
      window.removeEventListener('mouseup', alaFaara);
    };
  }, [router, baddilJanib]);

  const malik = jalsa.data?.malik ?? null;

  /** The shell's own palette entries: every screen that needs no game, and the fold. */
  const awamir = useMemo<readonly AmrLawha[]>(() => {
    const majal = t('tatbiq.ism', lugha);
    const qaima: AmrLawha[] = [
      {
        muarrif: 'hikal.tayy',
        unwan: t('hikal.lawha.tayy', lugha),
        majal,
        ikhtisar: 'Ctrl+B',
        nafidh: baddilJanib,
      },
      {
        muarrif: 'hikal.iftah_maktaba',
        unwan: t('hikal.lawha.iftah_maktaba', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/' });
        },
      },
      {
        muarrif: 'hikal.iftah_idadat',
        unwan: t('maktaba.lawha.iftah_idadat', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/idadat' });
        },
      },
      {
        muarrif: 'hikal.iftah_tashkhis',
        unwan: t('maktaba.lawha.iftah_tashkhis', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/tashkhis' });
        },
      },
      {
        muarrif: 'hikal.iftah_talabat',
        unwan: t('maktaba.lawha.iftah_talabat', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/talabat' });
        },
      },
    ];
    if (malik === true) {
      qaima.push({
        muarrif: 'hikal.iftah_muraja',
        unwan: t('hikal.lawha.iftah_muraja', lugha),
        majal,
        nafidh: () => {
          void intiqal({ to: '/muraja' });
        },
      });
    }
    return qaima;
  }, [lugha, baddilJanib, intiqal, malik]);
  useSajjilAwamir(awamir);

  const nizam = nizamNafidha();
  const bilaRaas = nizam === 'windows' || nizam === 'mac';

  // The strip says where the person is: the screen, and the game after it
  // when the screen is about one, joined the way the rest of the product
  // joins a screen to its subject.
  const miftahShasha = ASMA_SHASHAT[masarHali];
  const ismShasha = miftahShasha === undefined ? null : t(miftahShasha, lugha);
  const siyaqNafidha =
    ismShasha === null
      ? null
      : muarrif !== null && lubaJanib !== null
        ? `${ismShasha} · ${lubaJanib.ism}`
        : ismShasha;

  return (
    <div className={janibMatwi ? 'hikal hikal--matwi' : 'hikal'}>
      <Nafidha lugha={lugha} siyaq={siyaqNafidha} />

      <Janib
        lugha={lugha}
        luba={lubaJanib}
        tilqaiMasmuh={tilqaiMasmuh}
        malik={malik}
        sababJalsa={sababJalsa}
        matwi={janibMatwi}
        bilaRaas={bilaRaas}
        ala_tabdeel={baddilJanib}
      />

      <main className="hikal__mutawa">
        <TaqaddumAam />
        <Outlet />
      </main>

      <footer className="shareet">
        {sababMaalumat !== null ? (
          /*
            A placeholder is a promise that something is coming. This probe is
            not coming back on its own, so leaving the skeleton in place would
            have the strip shimmering for the rest of the session over a
            failure nobody was told about. The reason takes the row instead,
            truncated like the data path's own long value and readable in full
            on hover.
          */
          <dl className="shareet__qaima">
            <div className="shareet__band shareet__band--masar">
              <dd title={sababMaalumat}>{sababMaalumat}</dd>
            </div>
          </dl>
        ) : maalumat.data === undefined ? (
          <div className="shareet__haykal" aria-hidden="true">
            <span className="shareet__satr" />
          </div>
        ) : (
          /*
            Five facts at one weight is five things to read and no reason to
            read any of them. Only one of the five is about this session — a
            build that trusts the published development key rather than the
            release key — so that one leads, carries the warning colour and its
            glyph, and states itself in three words with the full sentence
            behind them. The other four are provenance: what is running and
            where it keeps its files. They lose their labels — a strip that
            names every value it shows says each thing twice — and sit at the
            far end in the quietest text the scale has.
          */
          <dl className="shareet__qaima">
            {maalumat.data.hawiyat_thiqa === 'tatwir' ? (
              <div className="shareet__band">
                <dt className="khafi">{t('maalumat.tatwir', lugha)}</dt>
                <dd className="shareet__wasm-tatwir">
                  <RamzTanbeeh />
                  {t('maktaba.shareet.tatwir', lugha)}
                  <span className="khafi">{t('maalumat.tatwir', lugha)}</span>
                </dd>
              </div>
            ) : null}
            <div className="shareet__band shareet__band--masar">
              <dt className="khafi">{t('maalumat.bayanat', lugha)}</dt>
              <dd className="mono-ltr" title={maalumat.data.jidhr_bayanat}>
                {maalumat.data.jidhr_bayanat}
              </dd>
            </div>
            <div className="shareet__band shareet__band--tarif">
              <dt className="khafi">{t('maalumat.isdar', lugha)}</dt>
              <dd className="mono-ltr" title={t('maalumat.isdar', lugha)}>
                {maalumat.data.isdar}
              </dd>
            </div>
            <div className="shareet__band shareet__band--tarif">
              <dt className="khafi">{t('maalumat.nizam', lugha)}</dt>
              <dd title={t('maalumat.nizam', lugha)}>
                {t(miftahNizam(maalumat.data.nizam), lugha)}
              </dd>
            </div>
            <div className="shareet__band shareet__band--tarif">
              <dt className="khafi">{t('maalumat.mimariya', lugha)}</dt>
              <dd className="mono-ltr" title={t('maalumat.mimariya', lugha)}>
                {t(miftahMimariya(maalumat.data.mimariya), lugha)}
              </dd>
            </div>
          </dl>
        )}
      </footer>

      <Tanbihat lugha={lugha} />
      <LawhatAwamir lugha={lugha} ikhtisarLawha={ikhtisarLawha} ikhtisarTaraju={ikhtisarTaraju} />
    </div>
  );
}
