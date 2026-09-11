import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useMemo } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { jam, munassiqat, t } from '@/lugha/lugha';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { RaasShasha } from '@/mukawwinat/raas_shasha';
import type { Idadat, LawhatTalabatHie, Lugha, NizamArqam } from '@/mustalahat/awamir';

import './talabat.css';

/** شاشة الطلبات — the demand-sorted requests board. */

/** How many placeholder rows stand in for the board: a screen's worth, no more. */
const SUFUF_HAYKAL = 7;

/**
 * The board drawn empty: the same three-column grid, the same row padding and
 * rule, and a bar in each cell at that cell's own line height, so the first
 * real row lands exactly where the first placeholder stood.
 */
function HaykalTalabat(): JSX.Element {
  return (
    <ol className="talabat__qaima" aria-hidden="true">
      {Array.from({ length: SUFUF_HAYKAL }, (_, fihris) => (
        <li key={fihris} className="talabat__saff">
          <span className="talabat__ism">
            <span
              className="talabat__haykal-satr talabat__haykal-satr--ism"
              style={{ inlineSize: `${String(36 + ((fihris * 17) % 40))}%` }}
            />
          </span>
          <span className="talabat__adad">
            <span className="talabat__haykal-satr talabat__haykal-satr--adad" />
          </span>
          <span className="talabat__waqt">
            <span className="talabat__haykal-satr talabat__haykal-satr--waqt" />
          </span>
        </li>
      ))}
    </ol>
  );
}

export function Talabat(): JSX.Element {
  const idadat = useQuery<Idadat, KhataJisr>({
    queryKey: mafatih.idadat,
    queryFn: () => nadi('idadat_hali'),
  });
  const lugha: Lugha = idadat.data?.lugha ?? 'arabi';
  const arqam: NizamArqam = idadat.data?.arqam ?? 'latini';
  const munassiq = useMemo(() => munassiqat(lugha, arqam), [lugha, arqam]);

  const lawha = useQuery<LawhatTalabatHie, KhataJisr>({
    queryKey: mafatih.talabat,
    queryFn: () => nadi('lawhat_talabat'),
  });

  const aidLawha = lawha.refetch;
  const awamirShasha = useMemo<readonly AmrLawha[]>(() => {
    if (idadat.data === undefined) {
      return [];
    }
    return [
      {
        muarrif: 'talabat.tahdith',
        unwan: t('talabat.lawha.tahdith', lugha),
        majal: t('shasha.talabat', lugha),
        nafidh: () => {
          void aidLawha();
        },
      },
    ];
  }, [idadat.data, lugha, aidLawha]);
  useSajjilAwamir(awamirShasha);

  const sufuf = lawha.data?.sufuf ?? [];
  const wajh = lawha.isPending
    ? 'tahmil'
    : lawha.error !== null
      ? 'khata'
      : sufuf.length === 0
        ? 'farigh'
        : 'jahiz';

  return (
    <div className="talabat">
      <RaasShasha
        rujoo={{ ila: 'maktaba' }}
        nassRujoo={t('talabat.raji', lugha)}
        unwan={t('shasha.talabat', lugha)}
        tafasil={
          lawha.data === undefined || sufuf.length === 0
            ? null
            : t('maktaba.adad_zahir', lugha, {
                adad: munassiq.raqm(sufuf.length),
                kulli: munassiq.raqm(sufuf.length),
              })
        }
      />
      <Mashhad miftah={wajh} className="talabat__jism">
        {wajh === 'tahmil' ? (
          <HaykalTalabat />
        ) : lawha.error !== null ? (
          <KutlatKhata
            unwan={t('talabat.taadhur', lugha)}
            khata={lawha.error}
            lugha={lugha}
            aada={() => {
              void aidLawha();
            }}
          />
        ) : wajh === 'farigh' ? (
          <HalatFarigha shasha unwan={t('shasha.talabat', lugha)} nass={t('talabat.farigh', lugha)}>
            <Link to="/" className="zir">
              {t('talabat.raji', lugha)}
            </Link>
          </HalatFarigha>
        ) : (
          <ol className="talabat__qaima">
            {sufuf.map((saf) => (
              <li key={saf.muarrif} className="talabat__saff">
                <Link
                  to="/luba/$muarrif"
                  params={{ muarrif: saf.muarrif }}
                  className="talabat__ism"
                >
                  {saf.ism_luba}
                </Link>
                <span className="talabat__adad">
                  {jam('talabat.adad', lugha, saf.adad, munassiq)}
                </span>
                {/* Emitted even when there is no timestamp: the row places three
                    cells into the list's grid, and two would shift the next row. */}
                <span className="talabat__waqt" dir="ltr">
                  {saf.akhir_waqt ?? ''}
                </span>
              </li>
            ))}
          </ol>
        )}
      </Mashhad>
    </div>
  );
}
