// النافذة — which frame the window wears, and the three things the product's own strip can do to it.

import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useState } from 'react';

import { ansha } from '@/hayat/tanbihat';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';

/**
 * Where the window's frame comes from.
 *
 * On Windows the product draws the whole strip and the three controls, because
 * the platform hands an undecorated window over completely. On macOS the
 * platform keeps its three lights and the product draws the strip behind them.
 * On Linux the platform keeps the frame: every desktop draws its own and a
 * strip under a strip is worse than either alone. A plain browser tab — the
 * development server opened outside Tauri — has no window to control at all.
 */
export type NizamNafidha = 'windows' | 'mac' | 'linux' | 'mutasaffih';

/** Whether Tauri's bridge is in this document, which is what makes the window reachable. */
function daakhilTauri(): boolean {
  return typeof (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ === 'object';
}

/**
 * The platform, read synchronously so the first frame draws the right strip.
 *
 * The user-agent rather than the build report the backend answers with: that
 * report is an IPC round trip away, and a strip that appeared one frame after
 * the window would be the flash this product removed from its theme.
 */
export function nizamNafidha(): NizamNafidha {
  if (!daakhilTauri()) {
    return 'mutasaffih';
  }
  const wakil = navigator.userAgent;
  if (/Windows/i.test(wakil)) {
    return 'windows';
  }
  if (/Mac OS X|Macintosh/i.test(wakil)) {
    return 'mac';
  }
  return 'linux';
}

/** What the strip draws and does. */
export interface HalatNafidha {
  readonly nizam: NizamNafidha;
  /** Whether the window fills its screen, which decides which of two glyphs the middle control shows. */
  readonly mukabbara: boolean;
  readonly tasgheer: () => void;
  readonly tabdeelTakbeer: () => void;
  readonly ighlaq: () => void;
}

/** How long after the last resize report the maximised state is re-read. */
const MUHLAT_QIRAA = 80;

/**
 * The window's state and controls, for the strip.
 *
 * The maximised flag is re-read after a resize settles rather than on every
 * report: a drag of the edge fires dozens a second and each read is a round
 * trip. A control that fails — a permission withdrawn, a bridge that went
 * away — says so in a notice, because the alternative is a button that does
 * nothing.
 */
export function useNafidha(lugha: Lugha): HalatNafidha {
  const [nizam] = useState<NizamNafidha>(nizamNafidha);
  const [mukabbara, setMukabbara] = useState(false);

  useEffect(() => {
    if (nizam === 'mutasaffih') {
      return undefined;
    }
    const nafidha = getCurrentWindow();
    let hayy = true;
    let muaqqit: number | undefined;
    const iqra = (): void => {
      nafidha
        .isMaximized()
        .then((qeema) => {
          if (hayy) {
            setMukabbara(qeema);
          }
        })
        .catch(() => {
          // The strip keeps the last answer it had; the next resize asks again.
        });
    };
    iqra();
    const ilgha = nafidha.onResized(() => {
      window.clearTimeout(muaqqit);
      muaqqit = window.setTimeout(iqra, MUHLAT_QIRAA);
    });
    return () => {
      hayy = false;
      window.clearTimeout(muaqqit);
      ilgha
        .then((fukk) => {
          fukk();
        })
        .catch(() => {
          // A listener that never registered has nothing to release.
        });
    };
  }, [nizam]);

  const naffidh = useCallback(
    (amal: () => Promise<void>): void => {
      amal().catch(() => {
        ansha({ naw: 'khatar', nass: t('nafidha.fashal', lugha) });
      });
    },
    [lugha],
  );

  const tasgheer = useCallback(() => {
    naffidh(() => getCurrentWindow().minimize());
  }, [naffidh]);

  const tabdeelTakbeer = useCallback(() => {
    naffidh(() => getCurrentWindow().toggleMaximize());
  }, [naffidh]);

  const ighlaq = useCallback(() => {
    naffidh(() => getCurrentWindow().close());
  }, [naffidh]);

  return { nizam, mukabbara, tasgheer, tabdeelTakbeer, ighlaq };
}
