// شريط النافذة — the product's own strip along the top of its window: the name, the drag, and the controls where the platform hands them over.

import type { JSX } from 'react';

import { useNafidha } from '@/hayat/nafidha';
import { t } from '@/lugha/lugha';
import type { Lugha } from '@/mustalahat/awamir';
import { RamzIstiada, RamzKhata, RamzTakbeer, RamzTasgheer } from '@/mukawwinat/rumuz';

import './nafidha.css';

/**
 * On Windows the platform's frame is declined and this strip is the whole
 * top of the window: the mark and the name at the reading edge, the three
 * controls at the far one, and everything between them drags the window.
 * On macOS the platform keeps its lights and the strip is drawn behind them.
 * On Linux, and outside Tauri, there is no strip: the desktop's own frame is
 * the frame, and a name under a name would say it twice.
 *
 * The drag is Tauri's own: it reads the attribute off the element under the
 * pointer, which is why the mark and the name take no pointer of their own —
 * a drag that started on the name would otherwise be a click on nothing.
 */
export interface KhasaisNafidha {
  readonly lugha: Lugha;
  /** Where the person is — the screen's name, and the game's when the screen is about one. */
  readonly siyaq: string | null;
}

export function Nafidha({ lugha, siyaq }: KhasaisNafidha): JSX.Element | null {
  const { nizam, mukabbara, tasgheer, tabdeelTakbeer, ighlaq } = useNafidha(lugha);

  if (nizam === 'linux' || nizam === 'mutasaffih') {
    return null;
  }

  const takbeer = t(mukabbara ? 'nafidha.istiada' : 'nafidha.takbeer', lugha);

  return (
    <header className={`nafidha nafidha--${nizam}`} data-tauri-drag-region>
      <div className="nafidha__hawiya">
        <img className="nafidha__shiar" src="/ayquna.png" alt="" draggable={false} />
        <span className="nafidha__ism">{t('tatbiq.ism', lugha)}</span>
        {siyaq === null ? null : (
          <>
            <span className="nafidha__fasil" aria-hidden="true" />
            <span className="nafidha__siyaq" dir="auto">
              {siyaq}
            </span>
          </>
        )}
      </div>
      {nizam === 'windows' ? (
        <div className="nafidha__azrar" role="group" aria-label={t('nafidha.tahakkum', lugha)}>
          <button
            type="button"
            className="nafidha__zirr"
            aria-label={t('nafidha.tasgheer', lugha)}
            title={t('nafidha.tasgheer', lugha)}
            onClick={tasgheer}
          >
            <RamzTasgheer />
          </button>
          <button
            type="button"
            className="nafidha__zirr"
            aria-label={takbeer}
            title={takbeer}
            onClick={tabdeelTakbeer}
          >
            {mukabbara ? <RamzIstiada /> : <RamzTakbeer />}
          </button>
          <button
            type="button"
            className="nafidha__zirr nafidha__zirr--ighlaq"
            aria-label={t('nafidha.ighlaq', lugha)}
            title={t('nafidha.ighlaq', lugha)}
            onClick={ighlaq}
          >
            <RamzKhata />
          </button>
        </div>
      ) : null}
    </header>
  );
}
