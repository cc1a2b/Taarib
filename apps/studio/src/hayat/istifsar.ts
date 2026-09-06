import { QueryClient } from '@tanstack/react-query';

/**
 * الاستفسار — the backend state cache.
 *
 * The defaults below are tuned for a backend that is a local process rather
 * than a server on the other side of the internet, which changes three of them
 * from what a web application would want.
 */
export const istifsar = new QueryClient({
  defaultOptions: {
    queries: {
      // The backend is a process on this machine, and it pushes an event when
      // something it owns changes. Refetching everything because the user
      // alt-tabbed back into the window would be work nobody asked for.
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      refetchOnMount: true,

      // Once. A local command that fails twice is failing for a reason a third
      // attempt will not fix, and the user is owed the error instead of three
      // seconds of silence.
      retry: 1,
      retryDelay: 250,

      // Long enough that moving between screens does not re-ask for the same
      // answer, short enough that a change made outside Taarib — a game
      // installed while the window was open — is picked up on the next visit.
      staleTime: 30_000,
      gcTime: 5 * 60_000,

      // The browser's online/offline flag describes the internet, and this
      // backend is not on the internet. Left at its default, every query would
      // pause on a machine that is deliberately offline — which is a supported
      // way to run this product.
      networkMode: 'always',
    },
    mutations: {
      retry: 0,
      networkMode: 'always',
    },
  },
});

/**
 * The query keys, in one place.
 *
 * A key written inline at a call site is a key that will eventually be written
 * differently at the second call site, and the two will not invalidate each
 * other.
 */
export const mafatih = {
  /** The settings tree. */
  idadat: ['idadat'] as const,
  /** The build and platform report. */
  maalumat: ['maalumat'] as const,
  /** The whole library. */
  maktaba: ['maktaba'] as const,
  /** One game's engine, capability report and build. */
  tafasil: (muarrif: string) => ['tafasil', muarrif] as const,
  /**
   * One game held whole: the core's five answers and the chain behind each.
   *
   * One key for every surface that asks, so arriving at the automatic-run
   * screen from the game screen costs nothing — and, more to the point, so the
   * two cannot be looking at two different answers about one game.
   */
  aql: (muarrif: string) => ['aql', muarrif] as const,
  /** Every patch the registry offers for one game. */
  ruqaa: (muarrif: string) => ['ruqaa', muarrif] as const,
  /** Whether one game's executable is running. */
  tashghil: (muarrif: string) => ['tashghil', muarrif] as const,
  /** Where the first-run acknowledgement stands. */
  iqrar: ['iqrar'] as const,
  /** The whole workspace table for one project. */
  warsha: (muarrif: string) => ['warsha', muarrif] as const,
  /** Suggestions for one string. */
  iqtirahat: (muarrif: string, nass: string) => ['iqtirahat', muarrif, nass] as const,
  /** The project-wide quality summary. */
  alamat: (muarrif: string) => ['alamat', muarrif] as const,
  /** Anchored comments for one project. */
  taaliqat: (muarrif: string) => ['taaliqat', muarrif] as const,
  /** Who this session is. */
  jalsa: ['jalsa'] as const,
  /** One game's submission draft. */
  musawwada: (muarrif: string) => ['musawwada', muarrif] as const,
  /** Every submission of mine. */
  musahamat: ['musahamat'] as const,
  /** The review queue. */
  tabur: ['tabur'] as const,
  /** One submission's console detail. */
  muraja: (ruqaa: string) => ['muraja', ruqaa] as const,
  /** The whole audit log. */
  sijill_muraja: ['sijill_muraja'] as const,
  /** The requests board. */
  talabat: ['talabat'] as const,
  /** One game's open request count. */
  talabat_luba: (muarrif: string) => ['talabat', muarrif] as const,
  /** The observations behind one game's engine identification. */
  dalail: (muarrif: string) => ['dalail', muarrif] as const,
  /** Whether one game's publisher already ships Arabic, containers included. */
  lugha_rasmiya: (muarrif: string) => ['lugha_rasmiya', muarrif] as const,
  /** One game's capture regions. */
  manatiq: (muarrif: string) => ['manatiq', muarrif] as const,
  /** One game's overlay reading history. */
  sijill_qira: (muarrif: string) => ['sijill_qira', muarrif] as const,
  /** The graphics-tier disclosure state. */
  ifsah: ['ifsah'] as const,
  /** Taarib's own font set. */
  khutut: ['khutut'] as const,
  /** Whether one provider has a keychain credential. */
  itimad: (muzawwid: string) => ['itimad', muzawwid] as const,
  /** Whether the update channel offers a newer version. */
  tahdith: ['tahdith'] as const,
  /** The diagnostics log view. */
  sijillat: ['sijillat'] as const,
} as const;
