import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import type { JSX } from 'react';
import { useMemo } from 'react';

import type { AmrLawha } from '@/hayat/awamir_lawha';
import { useSajjilAwamir } from '@/hayat/awamir_lawha';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { mafatih } from '@/hayat/istifsar';
import { jam, munassiqat, t } from '@/lugha/lugha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import type { Idadat, LawhatTalabatHie, Lugha, NizamArqam } from '@/mustalahat/awamir';

import './talabat.css';

/** شاشة الطلبات — the demand-sorted requests board. */

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

  return (
    <div className="talabat">
      <header className="talabat__shareet-alawi">
        <Link to="/" className="talabat__raji">
          {t('talabat.raji', lugha)}
        </Link>
        <span className="talabat__fasl">{t('shasha.talabat', lugha)}</span>
      </header>
      <div className="talabat__jism">
        {lawha.isPending ? (
          <p className="talabat__jari">{t('amm.tahmil', lugha)}</p>
        ) : lawha.error !== null ? (
          <KutlatKhata
            unwan={t('talabat.taadhur', lugha)}
            khata={lawha.error}
            lugha={lugha}
            aada={() => {
              void aidLawha();
            }}
          />
        ) : lawha.data === undefined || lawha.data.sufuf.length === 0 ? (
          <div className="talabat__faragh">
            <h2 className="talabat__faragh-unwan">{t('shasha.talabat', lugha)}</h2>
            <p className="talabat__faragh-nass">{t('talabat.farigh', lugha)}</p>
            <Link to="/" className="zir">
              {t('talabat.raji', lugha)}
            </Link>
          </div>
        ) : (
          <ol className="talabat__qaima">
            {lawha.data.sufuf.map((saf) => (
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
      </div>
    </div>
  );
}
