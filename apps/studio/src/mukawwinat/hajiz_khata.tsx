/**
 * The last boundary: what the window shows when a screen throws while rendering.
 *
 * Without one of these, a single throw anywhere in the tree unmounts everything
 * and leaves a white rectangle with no text, no code and no way back — the one
 * failure mode a user cannot report, because there is nothing on screen to
 * report. React offers no hook form of this; a class is the only implementation.
 *
 * It deliberately does not try to recover the screen that threw. A component
 * that threw once during render will throw again on the same props, so a retry
 * button here would loop. What it offers instead is the two things that are
 * actually useful: the error, and a way back to the library.
 */

import './hajiz_khata.css';

import type { ErrorInfo, JSX, ReactNode } from 'react';
import { Component } from 'react';

import type { Lugha } from '@/mustalahat/awamir';
import { t } from '@/lugha/lugha';

/** What the boundary holds once something below it has thrown. */
type HalatHajiz = {
  readonly khata: Error | null;
};

/** What it wraps, and the language to speak in when it has to. */
type KhasaisHajiz = {
  readonly lugha: Lugha;
  readonly children: ReactNode;
};

export class HajizKhata extends Component<KhasaisHajiz, HalatHajiz> {
  public override state: HalatHajiz = { khata: null };

  public static getDerivedStateFromError(khata: Error): HalatHajiz {
    return { khata };
  }

  /**
   * The throw reaches the log before it reaches the screen.
   *
   * A render failure is the one class of defect with no server-side trace, so
   * the component stack is the only evidence of where it happened.
   */
  public override componentDidCatch(khata: Error, mawqi: ErrorInfo): void {
    // eslint-disable-next-line no-console -- the browser console is the only
    // sink a webview render throw can reach; the Rust log is not in scope here.
    console.error('taarib: a screen threw while rendering', khata, mawqi.componentStack);
  }

  public override render(): ReactNode {
    const { khata } = this.state;
    const { lugha, children } = this.props;
    if (khata === null) {
      return children;
    }
    return sifhatKhata(khata, lugha);
  }
}

/** The page the boundary renders in place of the screen that threw. */
function sifhatKhata(khata: Error, lugha: Lugha): JSX.Element {
  return (
    <div className="hajiz" role="alert">
      <h1 className="hajiz__unwan">{t('hajiz.unwan', lugha)}</h1>
      <p className="hajiz__sharh">{t('hajiz.sharh', lugha)}</p>
      <pre className="hajiz__tafsil">{khata.message}</pre>
      <a className="hajiz__rujoo" href="/">
        {t('hajiz.rujoo', lugha)}
      </a>
    </div>
  );
}
