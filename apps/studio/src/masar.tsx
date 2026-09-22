import { useQuery } from '@tanstack/react-query';
import {
  Link,
  createRootRoute,
  createRoute,
  createRouter,
  useNavigate,
  useRouterState,
} from '@tanstack/react-router';
import { listen } from '@tauri-apps/api/event';
import type { JSX } from 'react';
import { useEffect } from 'react';

import type { MiftahQism } from '@/hayat/aqsam_idadat';
import { huwaQism } from '@/hayat/aqsam_idadat';
import { HADATH_TAHDITH_MUTAH, KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { ittijah, munassiqat, t, wasm } from '@/lugha/lugha';
import { ansha } from '@/hayat/tanbihat';
import { Hikal } from '@/mukawwinat/hikal';
// The router's own two states are drawn through the failure block's own parts,
// so the two cannot drift apart from every other failure in the product.
import { KutlatFashal, SatrRamz } from '@/mukawwinat/kutlat_khata';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import type { HalatTahdith, Idadat, Kathafa, Lugha, NizamArqam, Sima } from '@/mustalahat/awamir';
import { anwaIntiqal, yufaddilTaqleelHaraka } from '@/nizam/haraka';
import { IdadatShasha } from '@/shashat/idadat';
import { Luba } from '@/shashat/luba';
import { Maktaba } from '@/shashat/maktaba';
import { Muraja } from '@/shashat/muraja';
import { Tabaqa } from '@/shashat/tabaqa';
import { Talabat } from '@/shashat/talabat';
import { Taqdeem } from '@/shashat/taqdeem';
import { Tashkhis } from '@/shashat/tashkhis';
import { Tilqai } from '@/shashat/tilqai';
import { Warsha } from '@/shashat/warsha';

/**
 * المسار — the route tree.
 *
 * Routes are declared in code rather than generated from the filesystem, so
 * every path, parameter and search field is a type the screens are checked
 * against: a link to a route that does not exist, or to one whose parameters
 * changed shape, fails to compile instead of failing to navigate.
 */

/* ===========================================================================
   المظهر — the display settings, mirrored locally so the first frame is right.

   Theme, density, contrast and language live in the settings tree the backend
   owns, and reading that tree is an IPC round trip. Applying it in an effect
   means the window paints twice: once at the product default and once at the
   user's choice. A user who chose the light theme watches near-black paint and
   then snap to white; a user who chose a density watches the whole interface
   re-scale one frame in. Neither is a rendering nuance — it is the first thing
   the product does on every launch, every time.

   Two honest fixes exist. Render nothing until the settings resolve, which
   trades a wrong frame for a blank window on every launch including the
   overwhelming case where the answer is the same as last time. Or mirror the
   last known answer somewhere that can be read synchronously and apply it
   before React renders, which is what happens below.

   The mirror is a hint and never a source of truth: the backend's answer
   overwrites it the moment it lands, and a mirror that is missing, stale or
   hand-edited costs exactly the one flash this removes. That is the same trade
   `maktaba/hifz_khiyarat.ts` already makes for the grid's own view state, and
   for the same reason.
   =========================================================================== */

/** The part of the settings tree that decides the first painted frame. */
interface Mazhar {
  readonly lugha: Lugha;
  readonly sima: Sima;
  readonly kathafa: Kathafa;
  readonly tabayun: boolean;
  readonly ihtiramHaraka: boolean;
}

/** What a machine that has never run Taarib gets, and what `index.html` asserts. */
const MAZHAR_IFTIRADI: Mazhar = {
  lugha: 'arabi',
  sima: 'daken',
  kathafa: 'murih',
  tabayun: false,
  ihtiramHaraka: true,
};

/** Namespaced, because this origin is the whole product. */
const MIFTAH_MAZHAR = 'taarib.mazhar';

const LUGHAT: readonly Lugha[] = ['arabi', 'injilizi'];
const SIMAT: readonly Sima[] = ['daken', 'fatih', 'nizam'];
const KATHAFAT: readonly Kathafa[] = ['mudmaj', 'murih', 'kabir'];

/** Whether a stored value is one this build still recognises. */
function wahidMin<T extends string>(qeema: unknown, maqbula: readonly T[]): qeema is T {
  return typeof qeema === 'string' && (maqbula as readonly string[]).includes(qeema);
}

/** Real storage where the embedder allows it, and nothing where it does not. */
function takhzin(): Storage | null {
  try {
    const mawjud = globalThis.localStorage;
    return typeof mawjud === 'object' ? mawjud : null;
  } catch {
    // Reading `localStorage` throws rather than answering undefined when site
    // data is disabled, so the guard has to be a try block.
    return null;
  }
}

/**
 * The mirror, validated field by field.
 *
 * Per field rather than all-or-nothing, because the entry is written by
 * whichever build ran last: a density that gained a value should not cost the
 * user their theme.
 */
function iqraMazhar(): Mazhar {
  const makhzan = takhzin();
  if (makhzan === null) {
    return MAZHAR_IFTIRADI;
  }
  let khaam: unknown = null;
  try {
    const mahfuz = makhzan.getItem(MIFTAH_MAZHAR);
    khaam = mahfuz === null ? null : JSON.parse(mahfuz);
  } catch {
    return MAZHAR_IFTIRADI;
  }
  if (typeof khaam !== 'object' || khaam === null || Array.isArray(khaam)) {
    return MAZHAR_IFTIRADI;
  }
  const sijill = khaam as Readonly<Record<string, unknown>>;
  return {
    lugha: wahidMin(sijill['lugha'], LUGHAT) ? sijill['lugha'] : MAZHAR_IFTIRADI.lugha,
    sima: wahidMin(sijill['sima'], SIMAT) ? sijill['sima'] : MAZHAR_IFTIRADI.sima,
    kathafa: wahidMin(sijill['kathafa'], KATHAFAT) ? sijill['kathafa'] : MAZHAR_IFTIRADI.kathafa,
    tabayun: typeof sijill['tabayun'] === 'boolean' ? sijill['tabayun'] : MAZHAR_IFTIRADI.tabayun,
    ihtiramHaraka:
      typeof sijill['ihtiram_haraka'] === 'boolean'
        ? sijill['ihtiram_haraka']
        : MAZHAR_IFTIRADI.ihtiramHaraka,
  };
}

/** Records the answer the backend gave, for the next launch's first frame. */
function ihfazMazhar(mazhar: Mazhar): void {
  const makhzan = takhzin();
  if (makhzan === null) {
    return;
  }
  try {
    makhzan.setItem(
      MIFTAH_MAZHAR,
      JSON.stringify({
        lugha: mazhar.lugha,
        sima: mazhar.sima,
        kathafa: mazhar.kathafa,
        tabayun: mazhar.tabayun,
        ihtiram_haraka: mazhar.ihtiramHaraka,
      }),
    );
  } catch {
    // A full or read-only store costs the next launch its first frame and
    // nothing else, which is not worth taking a window down over.
  }
}

/** Whether the platform is asking for the light palette right now. */
function nizamFatih(): boolean {
  if (typeof globalThis.matchMedia !== 'function') {
    return false;
  }
  return globalThis.matchMedia('(prefers-color-scheme: light)').matches;
}

/** The document's own transition, in the shape the typed form takes. */
interface WathiqaMuntaqila {
  readonly startViewTransition?: (khiyarat: {
    readonly update: () => void;
    readonly types: readonly string[];
  }) => unknown;
}

/**
 * Applies a change of appearance through the document's transition, so the
 * window cross-fades from one look to the other rather than snapping.
 *
 * Applied plainly when the engine has no transition, when the session asked
 * for less motion, or when the engine has only the untyped form and refuses
 * the typed one — the change itself never depends on the fade.
 */
function intiqalMazhar(taghyeer: () => void): void {
  const wathiqa = document as Document & WathiqaMuntaqila;
  if (typeof wathiqa.startViewTransition !== 'function' || yufaddilTaqleelHaraka()) {
    taghyeer();
    return;
  }
  try {
    wathiqa.startViewTransition({ update: taghyeer, types: ['mazhar'] });
  } catch {
    taghyeer();
  }
}

/**
 * The resolved palette, written out in full.
 *
 * `data-sima='daken'` selects nothing — dark is the bare `:root` in the token
 * layer, not a mode — so this attribute is not what puts the dark palette on
 * screen. It is written anyway, and always with the *resolved* value rather
 * than the stored one, so that the root element states which of the two
 * palettes the document is actually wearing. `nizam` therefore never reaches
 * the DOM, `index.html`'s own `data-sima='daken'` stops being a claim nothing
 * maintains, and a diagnostics report can read the answer off the document
 * instead of re-deriving it from a setting plus a media query.
 */
function tabbiqSima(jidhrWathiqa: HTMLElement, fatih: boolean): void {
  jidhrWathiqa.setAttribute('data-sima', fatih ? 'fatih' : 'daken');
}

/** Language and base direction, which are one attribute each and nothing else. */
function tabbiqLugha(jidhrWathiqa: HTMLElement, lugha: Lugha): void {
  jidhrWathiqa.lang = wasm(lugha);
  jidhrWathiqa.dir = ittijah(lugha);
}

/** Density, contrast, and the reduced-motion override. */
function tabbiqQiyas(
  jidhrWathiqa: HTMLElement,
  kathafa: Kathafa,
  tabayun: boolean,
  ihtiramHaraka: boolean,
): void {
  jidhrWathiqa.setAttribute('data-kathafa', kathafa);
  if (tabayun) {
    jidhrWathiqa.setAttribute('data-tabayun', 'aali');
  } else {
    jidhrWathiqa.removeAttribute('data-tabayun');
  }
  // data-haraka='hurr' releases the reduced-motion zeroing for a user who
  // turned the honouring off; absent, the platform preference is honoured.
  if (ihtiramHaraka) {
    jidhrWathiqa.removeAttribute('data-haraka');
  } else {
    jidhrWathiqa.setAttribute('data-haraka', 'hurr');
  }
}

/**
 * The mirror as it stood when this module was evaluated, applied immediately.
 *
 * Module evaluation is hoisted above every statement in `main.tsx`, so this
 * runs before the session probe that file awaits and long before the first
 * render: the attributes are on the root element for the first paint rather
 * than one frame after it. The same value is the fallback the layout below
 * renders against, so the frame React draws before the settings arrive matches
 * the frame the browser has already painted.
 */
const MAZHAR_MABDAI: Mazhar = iqraMazhar();

tabbiqLugha(document.documentElement, MAZHAR_MABDAI.lugha);
tabbiqSima(
  document.documentElement,
  MAZHAR_MABDAI.sima === 'nizam' ? nizamFatih() : MAZHAR_MABDAI.sima === 'fatih',
);
tabbiqQiyas(
  document.documentElement,
  MAZHAR_MABDAI.kathafa,
  MAZHAR_MABDAI.tabayun,
  MAZHAR_MABDAI.ihtiramHaraka,
);

/* ===========================================================================
   The router's own two states.
   =========================================================================== */

/**
 * The session's language, from the cache the root layout has already filled.
 *
 * The two components below are instantiated by the router rather than by the
 * layout, so they cannot be handed the language as a prop. Reading it through
 * the same query key costs nothing: by the time either of them renders, the
 * answer is either in the cache or still in flight for the layout as well.
 */
function useLughatJalsa(): Lugha {
  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  return idadat.data?.lugha ?? MAZHAR_MABDAI.lugha;
}

interface KhasaisHalat {
  readonly lugha: Lugha;
  /** The machine's own name for what happened: a code, a command, an address. */
  readonly ramz: string;
  readonly unwan: string;
  readonly sharh: string;
  /** The machine's own text, when the sentence above is a stand-in for one. */
  readonly khaam: string | null;
  /**
   * Whether something failed, as against an address that was simply never a
   * screen. It picks the block's state glyph and its leading hairline, and it
   * decides whether a screen reader is interrupted for this.
   */
  readonly fashal: boolean;
  readonly children: JSX.Element;
}

/**
 * One page for both router states, in the product's own two vocabularies.
 *
 * An unmatched address and a route that threw are the two screens a person is
 * most likely to be looking at when they decide whether this product is
 * finished, and neither of them is an exception page. A throw is a failure and
 * takes the failure card — the sentence, the way on, and the machine's name
 * for it kept at the foot. An address nobody routed is not a failure and takes
 * the empty state, with the address itself where the code would be. Both sit
 * in the library's own shell, in a region that scrolls, because these two
 * pages have no grid of their own to hand the scrolling to.
 */
function HalatMasar({
  lugha,
  ramz,
  unwan,
  sharh,
  khaam,
  fashal,
  children,
}: KhasaisHalat): JSX.Element {
  return (
    <div className="hala-masar">
      <RaasShasha
        rujoo={{ ila: 'maktaba' }}
        nassRujoo={t('hajiz.rujoo', lugha)}
        unwan={t('tatbiq.ism', lugha)}
      />
      <div className="hala-masar__jism">
        {fashal ? (
          <KutlatFashal
            unwan={unwan}
            nass={sharh}
            khaam={khaam}
            ramz={ramz}
            tafsil={khaam}
            lugha={lugha}
          >
            {children}
          </KutlatFashal>
        ) : (
          <div className="halat halat--farigh halat--shasha" role="status">
            <p className="halat__unwan">{unwan}</p>
            <p className="halat__nass">{sharh}</p>
            <div className="halat__afal">{children}</div>
            <SatrRamz ramz={ramz} lugha={lugha} tasmiya={t('masar.unwan', lugha)} />
          </div>
        )}
      </div>
    </div>
  );
}

/** لا مسار — an address the session's route table has no screen for. */
function LaMasar(): JSX.Element {
  const lugha = useLughatJalsa();
  // The address itself is the only permanent identifier this failure has, so it
  // takes the place the code takes everywhere else.
  const mawqi = useRouterState({ select: (halat) => halat.location.href });
  return (
    <HalatMasar
      lugha={lugha}
      ramz={mawqi}
      unwan={t('masar.la_masar.unwan', lugha)}
      sharh={t('masar.la_masar.sharh', lugha)}
      khaam={null}
      fashal={false}
    >
      <Link to="/" className="zir zir--tamyeez">
        {t('hajiz.rujoo', lugha)}
      </Link>
    </HalatMasar>
  );
}

/** The machine's name for a throw: its code, the command it failed on, or its class. */
function ramzKhata(khata: Error): string {
  if (khata instanceof KhataJisr) {
    return khata.khata?.ramz ?? khata.amr;
  }
  return khata.name;
}

/** The sentence written for a person, when the throw carried one. */
function sharhKhata(khata: Error, lugha: Lugha): string {
  if (khata instanceof KhataJisr) {
    return khata.nass(lugha) ?? t('faragh.jisr', lugha);
  }
  return t('hajiz.sharh', lugha);
}

/**
 * The machine's own text, when the sentence above it is a stand-in.
 *
 * A throw that carried a structured error has already said everything it has
 * to say in the user's own language, and repeating the same two facts in
 * English underneath it is noise. Everything else — a bare string across the
 * IPC boundary, a webview exception, a throw from a screen's own render — has
 * exactly one line worth reporting, and this is it.
 */
function khaamKhata(khata: Error): string | null {
  if (khata instanceof KhataJisr && khata.khata !== null) {
    return null;
  }
  return khata.message === '' ? null : khata.message;
}

/**
 * Whatever a boundary caught, as the one type the three readers above ask
 * questions of.
 *
 * A `throw` carries the value it was given and the router hands that through
 * untouched, so the caught value is an `Error` only by convention: a webview
 * that failed to start rejects with a bare string, and a screen's own render
 * can throw anything at all. Settling it once here is what lets `ramzKhata`
 * and the other two keep reading `.name` and `.message` off a real error
 * instead of off a string, where both are undefined.
 */
function hawwilKhata(qeema: unknown): Error {
  if (qeema instanceof Error) {
    return qeema;
  }
  if (typeof qeema === 'string') {
    return new Error(qeema);
  }
  // A null-prototype object, and anything whose own `toString` throws, make
  // `String` throw — which is the one thing a failure path may not do.
  try {
    return new Error(String(qeema));
  } catch {
    return new Error(Object.prototype.toString.call(qeema));
  }
}

/** خطأ المسار — a route that threw before it could draw anything. */
function KhataMasar({
  error,
  reset,
}: {
  readonly error: unknown;
  readonly reset: () => void;
}): JSX.Element {
  const lugha = useLughatJalsa();
  const khata = hawwilKhata(error);
  return (
    <HalatMasar
      lugha={lugha}
      ramz={ramzKhata(khata)}
      unwan={t('amm.khata', lugha)}
      sharh={sharhKhata(khata, lugha)}
      khaam={khaamKhata(khata)}
      fashal
    >
      <>
        <button type="button" className="zir zir--tamyeez" onClick={reset}>
          {t('amm.iaada', lugha)}
        </button>
        <Link to="/tashkhis" className="zir">
          {t('khutwa.fath_tashkhis', lugha)}
        </Link>
        <Link to="/" className="zir">
          {t('hajiz.rujoo', lugha)}
        </Link>
      </>
    </HalatMasar>
  );
}

/* ===========================================================================
   The layout.
   =========================================================================== */

/**
 * The root layout: the shell around the matched route, and the one place the
 * display settings become document state. Language, theme, density, contrast
 * and the reduced-motion override are all attributes on the root element, so
 * the token layer re-points itself and no component ever asks twice.
 */
function JidhrTakhtit(): JSX.Element {
  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });

  // Every fallback is the mirror rather than the product default. Falling back
  // to the default would undo the correct frame the module scope above already
  // painted, and reintroduce the same flash from the other side.
  const lugha: Lugha = idadat.data?.lugha ?? MAZHAR_MABDAI.lugha;
  const sima = idadat.data?.sima ?? MAZHAR_MABDAI.sima;
  const kathafa = idadat.data?.kathafa ?? MAZHAR_MABDAI.kathafa;
  const tabayun = idadat.data?.tabayun_aali ?? MAZHAR_MABDAI.tabayun;
  const ihtiramHaraka = idadat.data?.ihtiram_taqleel_haraka ?? MAZHAR_MABDAI.ihtiramHaraka;

  // Each of the three is applied through the appearance transition only when
  // it actually changes what is on the root element: the first run of every
  // effect re-applies what the module scope already painted, and a fade
  // between two identical frames is a snapshot taken for nothing.
  useEffect(() => {
    const jidhrWathiqa = document.documentElement;
    if (jidhrWathiqa.lang === wasm(lugha) && jidhrWathiqa.dir === ittijah(lugha)) {
      return;
    }
    intiqalMazhar(() => {
      tabbiqLugha(jidhrWathiqa, lugha);
    });
  }, [lugha]);

  // 'nizam' follows the platform, live: the token layer keys off data-sima, so
  // the listener translates the platform preference into the same attribute.
  useEffect(() => {
    const jidhrWathiqa = document.documentElement;
    const tabbiq = (fatih: boolean): void => {
      if (jidhrWathiqa.getAttribute('data-sima') === (fatih ? 'fatih' : 'daken')) {
        return;
      }
      intiqalMazhar(() => {
        tabbiqSima(jidhrWathiqa, fatih);
      });
    };
    if (sima !== 'nizam') {
      tabbiq(sima === 'fatih');
      return undefined;
    }
    const tafdil = window.matchMedia('(prefers-color-scheme: light)');
    tabbiq(tafdil.matches);
    const alaTaghyeer = (hadath: MediaQueryListEvent): void => {
      tabbiq(hadath.matches);
    };
    tafdil.addEventListener('change', alaTaghyeer);
    return () => {
      tafdil.removeEventListener('change', alaTaghyeer);
    };
  }, [sima]);

  useEffect(() => {
    const jidhrWathiqa = document.documentElement;
    const kama =
      jidhrWathiqa.getAttribute('data-kathafa') === kathafa &&
      jidhrWathiqa.hasAttribute('data-tabayun') === tabayun &&
      jidhrWathiqa.hasAttribute('data-haraka') === !ihtiramHaraka;
    if (kama) {
      return;
    }
    intiqalMazhar(() => {
      tabbiqQiyas(jidhrWathiqa, kathafa, tabayun, ihtiramHaraka);
    });
  }, [kathafa, tabayun, ihtiramHaraka]);

  // Written only from a real answer. Mirroring the fallbacks would let a launch
  // whose backend never replied overwrite a good mirror with the defaults, and
  // then the flash comes back on the launch after it.
  useEffect(() => {
    const jawab = idadat.data;
    if (jawab === undefined) {
      return;
    }
    ihfazMazhar({
      lugha: jawab.lugha,
      sima: jawab.sima,
      kathafa: jawab.kathafa,
      tabayun: jawab.tabayun_aali,
      ihtiramHaraka: jawab.ihtiram_taqleel_haraka,
    });
  }, [idadat.data]);

  // The launch-time update check's one visible effect: a notice raised only
  // when a newer version is actually offered, carrying the way straight to the
  // update section. Subscribed from the root route, which never unmounts, so a
  // check that finishes after the first frame still lands. Digits fall back to
  // Western, which the settings type documents as correct for build numbers.
  const tanaqqul = useNavigate();
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  useEffect(() => {
    const munassiq = munassiqat(lugha, arqam);
    const wad = listen<HalatTahdith>(HADATH_TAHDITH_MUTAH, (hadath) => {
      if (hadath.payload.hala !== 'mutah') {
        return;
      }
      ansha({
        naw: 'maluma',
        nass: t('idadat.tahdith.mutah', lugha, {
          isdar: hadath.payload.isdar,
          hajm: munassiq.hajm(hadath.payload.hajm),
        }),
        mudda: null,
        amal: {
          unwan: t('idadat.tahdith.urud', lugha),
          nafidh: () => {
            void tanaqqul({ to: '/idadat' });
          },
        },
      });
    });
    return () => {
      void wad.then((ilgha) => {
        ilgha();
      });
    };
  }, [tanaqqul, lugha, arqam]);

  const ikhtisarat = idadat.data?.ikhtisarat ?? { lawha: 'ctrl+k', taraju: 'ctrl+z' };
  return (
    <Hikal lugha={lugha} ikhtisarLawha={ikhtisarat.lawha} ikhtisarTaraju={ikhtisarat.taraju} />
  );
}

const jidhr = createRootRoute({
  component: JidhrTakhtit,
});

/** The library, which is where the application opens. */
const masarMaktaba = createRoute({
  getParentRoute: () => jidhr,
  path: '/',
  component: Maktaba,
});

/** One game's detail, addressed by the identifier the library gave it. */
const masarLuba = createRoute({
  getParentRoute: () => jidhr,
  path: '/luba/$muarrif',
  component: Luba,
});

/** One game's automatic run: the verdict, the stages, and what it produced. */
const masarTilqai = createRoute({
  getParentRoute: () => jidhr,
  path: '/tilqai/$muarrif',
  component: Tilqai,
});

/** One game's translation workspace, addressed by the same identifier. */
const masarWarsha = createRoute({
  getParentRoute: () => jidhr,
  path: '/warsha/$muarrif',
  component: Warsha,
});

/** One game's submission wizard and the contributor's trail. */
const masarTaqdeem = createRoute({
  getParentRoute: () => jidhr,
  path: '/taqdeem/$muarrif',
  component: Taqdeem,
});

/** One game's overlay control: regions, history, and the disclosure. */
const masarTabaqa = createRoute({
  getParentRoute: () => jidhr,
  path: '/tabaqa/$muarrif',
  component: Tabaqa,
});

/**
 * The settings tree, and the section a caller wants it opened at.
 *
 * Settings is eleven folding sections and one scroll, so "open Settings" on
 * its own lands the reader at the top of a document and leaves them hunting
 * for the row the failure was about. Every failure that names a section
 * already carries which one; the address is what carries it the rest of the
 * way, and carrying it there rather than in a store means the deep link
 * survives a reload and can be shared in a report.
 */
const masarIdadat = createRoute({
  getParentRoute: () => jidhr,
  path: '/idadat',
  validateSearch: (khaam: Record<string, unknown>): { readonly qism?: MiftahQism } => {
    const qism = khaam['qism'];
    return huwaQism(qism) ? { qism } : {};
  },
  component: IdadatShasha,
});

/** The diagnostics surface. */
const masarTashkhis = createRoute({
  getParentRoute: () => jidhr,
  path: '/tashkhis',
  component: Tashkhis,
});

/** The demand-sorted requests board. */
const masarTalabat = createRoute({
  getParentRoute: () => jidhr,
  path: '/talabat',
  component: Talabat,
});

/**
 * The review console. Only a session that proved it holds the owner key gets a
 * route table containing it; a contributor session's table simply has no such
 * path.
 */
const masarMuraja = createRoute({
  getParentRoute: () => jidhr,
  path: '/muraja',
  component: Muraja,
});

const shajarat = jidhr.addChildren([
  masarMaktaba,
  masarLuba,
  masarTilqai,
  masarWarsha,
  masarTaqdeem,
  masarTabaqa,
  masarIdadat,
  masarTashkhis,
  masarTalabat,
  masarMuraja,
]);

type Shajarat = typeof shajarat;

/**
 * The application's router, built once the session is known.
 *
 * The console route exists only in an owner session's table. The contributor
 * tree is narrower than the type the rest of the application is checked
 * against, which is exactly the point: the check keeps every `Link` honest,
 * and the runtime table keeps the console genuinely absent rather than hidden.
 */
export function binniMuwajjih(malik: boolean) {
  const atfal = malik
    ? shajarat
    : (jidhr.addChildren([
        masarMaktaba,
        masarLuba,
        masarTilqai,
        masarWarsha,
        masarTaqdeem,
        masarTabaqa,
        masarIdadat,
        masarTashkhis,
        masarTalabat,
      ]) as unknown as Shajarat);
  return createRouter({
    routeTree: atfal,
    // The backend is a local process, so preloading on pointer intent costs a
    // few milliseconds of IPC and buys a screen that is already populated by
    // the time the pointer finishes travelling to it.
    defaultPreload: 'intent',
    defaultPreloadStaleTime: 0,
    // Returning to a list must return to where the user was in it, not to the
    // top of it. In a right-to-left layout that includes the horizontal offset.
    scrollRestoration: true,
    // Every navigation is handed to the document's own transition, typed by
    // which way it travels, so the content region moves and the shell does
    // not; `nizam/qaida.css` draws it. An engine without the API, and a session
    // that asked for less motion, get the plain swap.
    defaultViewTransition: { types: anwaIntiqal },
    // Declared as router defaults rather than on the root route, because the
    // root route's own pair covers only the root: a throw inside any child
    // route resolves against these.
    defaultNotFoundComponent: LaMasar,
    defaultErrorComponent: KhataMasar,
  });
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof binniMuwajjih>;
  }
}
