import { invoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';

import type { commands, Khata, Lugha } from '@/mustalahat/awamir';

/**
 * الجسر — the one place the interface talks to the backend.
 *
 * Two things happen here and nowhere else. Every call is checked against the
 * command surface — the name, and the argument object with it — so a renamed
 * field, a dropped parameter or a payload built for the previous signature is
 * a build error rather than a promise that rejects at runtime. And every
 * rejection is normalised into a single error class, so a screen never has to
 * guess whether what it caught is a Taarib error with a code and an Arabic
 * sentence, a webview exception, or a string.
 *
 * Nothing about a command is described twice. Its answer, its parameter types
 * and how many it takes are all read out of the bindings `tauri-specta`
 * regenerates on every debug build, so a field renamed in Rust is a type error
 * in the screen that reads it and a parameter renamed in Rust is a type error
 * in the screen that sends it.
 *
 * The bindings are imported as a type. The call still goes through `invoke`,
 * because the generated functions take their arguments positionally and answer
 * with a result object, and every screen here is written against an argument
 * object and a rejection.
 */

/**
 * The window event every download progress report arrives on.
 *
 * `nazzil_ruqaa` reports as it transfers and then returns its last report, so
 * the answer alone is enough for a caller that does not want the stream. A
 * caller that does subscribes to this with `listen<TaqaddumTanzeel>` from
 * `@tauri-apps/api/event`: it is a window event rather than a `Channel`, so the
 * subscription is independent of the call and outlives a component that
 * remounts mid-download.
 */
export const HADATH_TAQADDUM_TANZEEL = 'taarib://taqaddum-tanzeel';

/** The window event each install stage is announced on. */
export const HADATH_MARHALAT_TATHBEET = 'taarib://marhalat-tathbeet';

/** The window event a batch translation reports progress on. */
export const HADATH_TAQADDUM_DUFA = 'taarib://taqaddum-dufa';

/**
 * The window event one game's resolved artwork arrives on.
 *
 * `hassil_suwar_maktaba` walks the library and emits one of these per game as
 * each one settles, so a grid fills in as covers land rather than waiting for
 * the slowest fetch. It fires for a game that resolved *nothing* too, which is
 * what lets a card leave its pending state instead of waiting forever on art
 * that is never coming.
 */
export const HADATH_GHILAF_LUBA = 'taarib://ghilaf-luba';

/**
 * The window event every automatic-arabization snapshot arrives on.
 *
 * `ibda_tilqai` answers as soon as the run is on its own task and then says
 * nothing more; everything after that — each stage, each string, the spend, and
 * the run's own end — arrives here. A window event rather than a `Channel` for
 * the reason the download event gives above: the subscription is independent of
 * the call that started the run, so it survives the screen remounting mid-run
 * and still delivers when the user navigated away and came back.
 *
 * Each payload is a **whole snapshot** keyed by the run's own id, never a delta,
 * which is what lets a subscriber that joined half way through draw the complete
 * picture on the next report. `tilqai/aqd.ts` re-exports this beside the decoder
 * that reads those payloads.
 */
export const HADATH_TILQAI = 'taarib://tilqai';

/**
 * The window event one game's background engine probe is announced on.
 *
 * The library probes itself after every completed scan — every visible game
 * whose capability report is missing or was written by an older probe — and
 * announces each game here the moment its report is written. The payload is
 * the game's identity and exactly the fields its library row derives from the
 * report, so the grid patches that row in place: no rescan, no reflow, and the
 * card's box never changes.
 */
export const HADATH_FAHS_MUHARRIK = 'taarib://fahs-muharrik';

/**
 * The window event the background sweep's own standing arrives on: once when a
 * round starts, once after every game, and once when it ends — that last one
 * with `jariya` false and every failure it met carried by name. `halat_jawla`
 * answers the same shape for a screen that mounts mid-sweep.
 */
export const HADATH_JAWLA_MUHARRIK = 'taarib://jawla-muharrik';

/** The window event the launch-time update check raises when a newer version is offered; payload is a `HalatTahdith` in its `mutah` case. */
export const HADATH_TAHDITH_MUTAH = 'taarib://tahdith-mutah';

/** The window event the startup component mirror reports progress on; payload is a `TaqaddumTahmil`. */
export const HADATH_TAHMIL = 'taarib://tahmil-mukawwinat';

/**
 * The command surface: the name the backend registered, and the names its
 * parameters were declared under, in the order Rust declared them.
 *
 * This is the one thing about a command that cannot be read out of the
 * bindings. TypeScript keeps a tuple's element labels for a reader and exposes
 * them to nobody, so `Parameters<>` gives the types and their order but never
 * the names the payload has to be keyed by. Everything else — the types, the
 * arity, the answer — is derived from those parameters below.
 *
 * Kept in the generated file's own order, so checking this against it is one
 * scan rather than a search per line. A name that drifts is not caught here;
 * an arity that drifts is, by `FahsTartib`, and a command that appears or
 * disappears is, by `FahsSath`.
 */
interface TartibWusata {
  // Application and library.
  readonly idadat_hali: [];
  readonly halat_tahmil: [];
  readonly maalumat_taarib: [];
  readonly maktaba: [];
  readonly fahs_akhir: [];
  readonly hassil_suwar_maktaba: [];
  readonly iftah_manassa: ['muarrif'];
  readonly adif_mujallad_fahs: ['masar'];
  readonly halat_jawla: [];

  // One game: what it is, and what it runs on.
  readonly tafasil_luba: ['muarrif'];
  readonly afhas_muharrik: ['muarrif'];
  readonly dalail_muharrik: ['muarrif'];
  readonly lugha_rasmiya: ['muarrif'];
  readonly fahs_himaya: ['muarrif'];
  readonly ikhfa_luba: ['muarrif', 'mukhfiya'];
  // One game held whole: which product it gets, what is promised, what the
  // limits are, what risks need consent, and what blocks it outright — each
  // answer carrying the producers behind it. It walks the game directory, so a
  // screen asks for it on purpose rather than on mount.
  readonly aql_luba: ['muarrif'];

  // Patches: offered, downloaded, verified, installed, removed.
  readonly ruqaa_luba: ['muarrif'];
  readonly nazzil_ruqaa: ['muarrif', 'ruqaa'];
  readonly tahaqquq_ruqaa: ['muarrif'];
  readonly azil_ruqaa: ['muarrif', 'matlab', 'siyasa'];
  readonly hal_tashtaghil: ['muarrif'];
  readonly iqrar_aman: [];
  readonly sajjil_iqrar_aman: ['lugha'];
  readonly thabbit_ruqaa: ['muarrif', 'masarMalaf', 'iqrarShabaka', 'iqrarTaqribi'];
  // What an install would write and what a removal would leave. Both are the
  // installer's own dry runs — the same `khutta` the write itself is handed —
  // and both read the game directory, so a screen asks for them at the moment
  // the decision is made rather than on mount.
  readonly khuttat_tathbeet: ['muarrif'];
  readonly khuttat_izala: ['muarrif', 'matlab'];

  // The workshop.
  readonly nusus_warsha: ['muarrif'];
  readonly haddith_tarjama: ['muarrif', 'nass', 'hadaf'];
  readonly iqtirahat_nass: ['muarrif', 'nass'];
  readonly tatbiq_iqtirah: ['muarrif', 'nass', 'qayd'];
  readonly tarjim_nass: ['muarrif', 'nass'];
  readonly tarjim_dufa: ['muarrif', 'saqf'];
  readonly alamat_mashru: ['muarrif'];
  readonly wahhid_mustalah: ['muarrif', 'mustalah', 'shakl'];
  readonly muayana: ['muarrif', 'nass', 'hadaf'];
  readonly taaliqat_warsha: ['muarrif'];
  readonly idmaj_huzma: ['muarrif', 'masar'];
  readonly qarrir_nizaat: ['muarrif', 'qararat'];
  readonly anqidh_mashru: ['muarrif'];

  // The submission a contributor prepares.
  readonly jalsati: [];
  readonly musawwadat_luba: ['muarrif'];
  readonly jahhiz_taqdeem: [
    'muarrif',
    'unwan',
    'sharh',
    'taghyeerat',
    'rukhsa',
    'rukhsaIsm',
    'tareeqa',
  ];
  readonly aqirr_tahdheer: ['muarrif', 'tahdheer', 'qeema'];
  readonly sallim_taqdeem: ['muarrif'];
  readonly musahamati: [];

  // The review the owner runs, and the sandbox it is tried in.
  readonly tabur_muraja: [];
  readonly tafasil_muraja: ['ruqaa'];
  readonly allaq_muraja: ['ruqaa', 'nass', 'matn'];
  readonly qarrir_muraja: ['ruqaa', 'ijra', 'sabab'];
  readonly iaatimad_muraja: ['ruqaa'];
  // Approving seals a package on this machine; these two are the step that puts
  // it in the registry, which is the only thing that makes it installable by
  // anybody else.
  readonly halat_nashr_mustawda: [];
  readonly tajawuz_nashr: ['ruqaa', 'sabab'];
  readonly unshur_mustawda: [];
  readonly sijill_muraja_kull: [];
  readonly sandooq_thabbit: ['ruqaa', 'muarrif'];
  readonly sandooq_atliq: ['ruqaa', 'muarrif'];
  readonly sandooq_imsah: ['ruqaa'];

  // The requests board.
  readonly lawhat_talabat: [];
  readonly utlub_tarjama: ['muarrif', 'mulahaza'];
  readonly adad_talabat: ['muarrif'];

  // Settings, credentials and fonts.
  readonly abda_tawthiq_taqdeem: [];
  readonly haddith_idadat: ['idadat'];
  readonly khzin_itimad_muzawwid: ['muzawwid', 'sirr'];
  readonly imsah_itimad_muzawwid: ['muzawwid'];
  readonly hal_itimad_muzawwid: ['muzawwid'];
  readonly ikhtar_khatt: ['masar'];
  readonly khutut_mutaha: [];

  // The overlay: its regions, its history, its disclosure.
  readonly manatiq_luba: ['muarrif'];
  readonly adif_mintaqa: ['muarrif', 'shakl'];
  readonly haddith_mintaqa: ['muarrif', 'raqm', 'shakl', 'mumakkana'];
  readonly ihdhif_mintaqa: ['muarrif', 'raqm'];
  readonly sijill_qira_luba: ['muarrif', 'hadd'];
  readonly imsah_sijill_qira: ['muarrif'];
  readonly nass_ifsah: [];
  readonly aqirr_ifsah: [];

  // Updates and diagnostics.
  readonly tahaqquq_tahdith: [];
  readonly nazzil_tahdith: [];
  readonly sijillat_akhira: ['satr'];
  readonly taqreer_tawafuq: ['muarrif'];
  readonly huzmat_tashkhis: [];
  readonly iftah_tashkhis: [];

  // One-button automatic arabization: the verdict, the run, and the stop.
  readonly hukm_tilqai: ['muarrif'];
  readonly laqtat_tilqai: ['muarrif'];
  readonly ibda_tilqai: ['muarrif', 'istinaf', 'iqrarShabaka', 'dammIltiqat'];
  readonly alghi_tilqai: ['muarrif'];
  readonly halat_iltiqat: ['muarrif'];
  readonly sajjil_iltiqat: ['muarrif', 'mufaal'];

  // Sharing what the overlay read off a screen. `jahhiz` gathers a draft and
  // hands back every entry it would send — never a sample, because a preview of
  // a hundred rows out of two thousand makes consent a formality — and mints a
  // fingerprint over exactly that set. `saddir` refuses unless the fingerprint
  // echoed back is the draft still being held and every warning is acknowledged
  // by name.
  readonly jahhiz_musharaka: ['muarrif', 'khiyarat'];
  readonly saddir_musharaka: ['muarrif', 'basma', 'iqrarat'];
  readonly afhas_musharaka: ['masar', 'miftah'];
  readonly idmij_musharaka: ['basma', 'khiyarat'];

  // What other teams already made for a game, credited and linked, and the one
  // control that opens a maker's page in the browser after the backend has
  // checked the address against the index it came from.
  readonly tarjamat_mujtama: ['muarrif'];
  readonly iftah_rabt: ['rabt'];
}

/** A command name the backend actually registered. */
export type IsmAmr = keyof TartibWusata;

/** The generated bindings, read for their types only. */
type AwamirMuwallada = typeof commands;

/**
 * The registered name as Rust spells it, in the spelling the bindings use.
 *
 * `tauri-specta` camel-cases the function it generates and leaves the name it
 * invokes alone, so the two spellings have to meet somewhere. Doing it here
 * means the bridge is checked rather than remembered.
 */
type IsmMuwallad<M extends string> = M extends `${infer Sadr}_${infer Baqi}`
  ? `${Sadr}${Capitalize<IsmMuwallad<Baqi>>}`
  : M;

/** The generated function for one command. */
type DallatAmr<M extends IsmAmr> = AwamirMuwallada[IsmMuwallad<M> & keyof AwamirMuwallada];

/** Its parameters, in the order Rust declared them. */
type MuamalatAmr<M extends IsmAmr> = DallatAmr<M> extends (...muamalat: infer Q) => unknown
  ? Q
  : never;

/** What it answers with, taken out of the generated result. */
type JawabAmr<M extends IsmAmr> = DallatAmr<M> extends (
  ...muamalat: never[]
) => Promise<infer Natija>
  ? Natija extends { status: 'ok'; data: infer Bayanat }
    ? Bayanat
    : never
  : never;

/** The command surface: the name the backend registered, and what it answers with. */
export type KhareetatAwamir = { readonly [M in IsmAmr]: JawabAmr<M> };

/** Each declared name carrying the type of the parameter that stands there. */
type RabtWusata<
  Mafatih extends readonly string[],
  Qiyam extends readonly unknown[],
> = Mafatih extends readonly [infer Miftah extends string, ...infer Baqi extends readonly string[]]
  ? Qiyam extends readonly [infer Qeema, ...infer BaqiQiyam]
    ? { readonly [K in Miftah]: Qeema } & RabtWusata<Baqi, BaqiQiyam>
    : never
  : unknown;

/** Flattens the intersection the zip builds, so a mismatch reads as one object. */
type Basit<T> = T extends infer Mabsut ? { readonly [K in keyof Mabsut]: Mabsut[K] } : never;

/** One command's argument object, as the payload has to be keyed. */
type WusataAmr<M extends IsmAmr> = Basit<RabtWusata<TartibWusata[M], MuamalatAmr<M>>>;

/** Fails to instantiate unless its argument is `true`. */
type Yajib<T extends true> = T;

/**
 * Every registered command is described here, and everything described here is
 * a registered command.
 *
 * A command added in Rust and not added here would be unreachable through
 * `nadi`; one removed in Rust and left here would type-check against a
 * signature that no longer exists. Exported because an assertion nothing reads
 * is an unused local.
 */
export type FahsSath = Yajib<
  IsmMuwallad<IsmAmr> extends keyof AwamirMuwallada
    ? keyof AwamirMuwallada extends IsmMuwallad<IsmAmr>
      ? true
      : false
    : false
>;

/**
 * Every command lists exactly as many argument names as it has parameters.
 *
 * This is what catches a Rust signature that gained or lost a parameter: the
 * names above stop lining up with the types below them, and the failure lands
 * here rather than in a screen.
 */
export type FahsTartib = Yajib<
  {
    [M in IsmAmr]: TartibWusata[M]['length'] extends MuamalatAmr<M>['length'] ? true : false;
  }[IsmAmr]
>;

/**
 * A failed command.
 *
 * `khata` carries the backend's own error when there was one — its permanent
 * code, its two sentences and the single action the interface can offer — and
 * is null when the call never reached a command at all, which is the case a
 * screen has to phrase for itself because no backend was there to phrase it.
 */
export class KhataJisr extends Error {
  /** The command that failed. */
  readonly amr: IsmAmr;

  /** The backend's error, or null when the failure happened before it. */
  readonly khata: Khata | null;

  constructor(amr: IsmAmr, khata: Khata | null, risala: string, sabab: unknown) {
    super(risala, { cause: sabab });
    this.name = 'KhataJisr';
    this.amr = amr;
    this.khata = khata;
  }

  /**
   * The sentence to show the user, in their language, or null when the
   * failure carries no sentence written for a person.
   */
  nass(lugha: Lugha): string | null {
    if (this.khata === null) {
      return null;
    }
    return lugha === 'arabi' ? this.khata.arabi : this.khata.injilizi;
  }
}

/**
 * Whether a rejected value is a serialized `Khata`.
 *
 * Checked structurally rather than trusted, because anything at all can come
 * back across an IPC boundary — including, on a webview that failed to start,
 * a plain string.
 */
function huwaKhata(qeema: unknown): qeema is Khata {
  if (typeof qeema !== 'object' || qeema === null) {
    return false;
  }
  const kain = qeema as Record<string, unknown>;
  const khutwa = kain['khutwa'];
  return (
    typeof kain['ramz'] === 'string' &&
    typeof kain['khutura'] === 'string' &&
    typeof kain['arabi'] === 'string' &&
    typeof kain['injilizi'] === 'string' &&
    typeof khutwa === 'object' &&
    khutwa !== null &&
    typeof (khutwa as Record<string, unknown>)['naw'] === 'string'
  );
}

/** Normalises whatever a rejected invoke produced into one error type. */
function hawwil(amr: IsmAmr, khaam: unknown): KhataJisr {
  if (huwaKhata(khaam)) {
    return new KhataJisr(amr, khaam, `${khaam.ramz} ${khaam.injilizi}`, khaam);
  }
  if (khaam instanceof Error) {
    return new KhataJisr(amr, null, khaam.message, khaam);
  }
  return new KhataJisr(amr, null, String(khaam), khaam);
}

/**
 * Calls a backend command.
 *
 * @param amr the registered command name
 * @param wusata the command's arguments, keyed as Rust declared them; the
 *   parameter is not there at all for a command that takes none
 * @throws {KhataJisr} whenever the command fails or never runs
 */
export async function nadi<M extends IsmAmr>(
  amr: M,
  ...wusata: MuamalatAmr<M>['length'] extends 0 ? [] : [wusata: WusataAmr<M>]
): Promise<KhareetatAwamir[M]> {
  const hamula = (wusata as readonly unknown[])[0] as InvokeArgs | undefined;
  try {
    return await invoke<KhareetatAwamir[M]>(amr, hamula);
  } catch (khaam) {
    throw hawwil(amr, khaam);
  }
}
