import { useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import type { JSX } from 'react';
import { useEffect } from 'react';

import { HADATH_TAHMIL, KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { t } from '@/lugha/lugha';
import type { Lugha, TaqaddumTahmil } from '@/mustalahat/awamir';

import './tahmil.css';

/**
 * The startup screen, shown until the bundled component tree has mirrored.
 *
 * The mirror copies and hashes every component the bundle carries — half a
 * gigabyte for the offline set — and it used to run on the thread the window is
 * painted from, so the window opened black and reported "Not Responding" until
 * it finished. It now runs off that thread, and this is what stands in front of
 * the product while it does.
 *
 * Both a query and a subscription: a webview that finishes loading after the
 * last report would never hear one, and the screen would stand over a product
 * that is ready. The query answers where the mirror stands right now; the
 * subscription carries it the rest of the way.
 */
export function ShashatTahmil({ lugha }: { readonly lugha: Lugha }): JSX.Element | null {
  const makhzanIstifsar = useQueryClient();
  const tahmil = useQuery<TaqaddumTahmil, KhataJisr>({
    queryKey: mafatih.tahmil,
    queryFn: () => nadi('halat_tahmil'),
    // The mirror reports on an event; refetching on focus would ask a question
    // the subscription already answers.
    refetchOnWindowFocus: false,
    staleTime: Infinity,
  });

  useEffect(() => {
    const wad = listen<TaqaddumTahmil>(HADATH_TAHMIL, (hadath) => {
      makhzanIstifsar.setQueryData<TaqaddumTahmil>(mafatih.tahmil, hadath.payload);
    });
    return () => {
      void wad.then((ilgha) => {
        ilgha();
      });
    };
  }, [makhzanIstifsar]);

  const hala = tahmil.data;

  // A backend that cannot answer this is one the product cannot talk to at all,
  // and the screens behind report that far better than a curtain nobody can
  // lift. The same reasoning ends the wait on a mirror that failed: `tamma` is
  // sent however the mirror finished.
  if (tahmil.error !== null || hala?.tamma === true) {
    return null;
  }

  // Before the manifest has been read there is a wait but no total, so the
  // screen says it is working without claiming a figure it does not have.
  const majhul = hala === undefined || hala.majmu === 0;
  const nisba = majhul || hala === undefined ? 0 : Math.round((hala.munjaz / hala.majmu) * 100);

  return (
    <div
      className={`tahmil${majhul ? ' tahmil--majhul' : ''}`}
      role="status"
      aria-live="polite"
      style={{ ['--tahmil-nisba' as string]: `${nisba}%` }}
    >
      <div className="tahmil__lawh">
        {/* The product's own name, taking ink from the right as the mirror
            advances — the direction the script is read. */}
        <p className="tahmil__kalima" aria-hidden="true">
          تعريب
        </p>
        <div className="tahmil__kashida">
          <span className="tahmil__kashida-madd" />
        </div>
        <p className="tahmil__satr">{t('tahmil.jari', lugha)}</p>
        {majhul || hala === undefined ? null : (
          <p className="tahmil__adad">
            {t('tahmil.adad', lugha, {
              munjaz: String(hala.munjaz),
              majmu: String(hala.majmu),
              nisba: String(nisba),
            })}
          </p>
        )}
      </div>
    </div>
  );
}
