import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { JSX } from 'react';

import { mafatih } from '@/hayat/istifsar';
import { KhataJisr, nadi } from '@/hayat/jisr';
import { ansha } from '@/hayat/tanbihat';
import { t } from '@/lugha/lugha';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Zuhur } from '@/mukawwinat/zuhur';
import type { HalatIqrar, Lugha } from '@/mustalahat/awamir';

import './qism_iqrar.css';

/**
 * قسم الإقرار — the first-run statement, and the one place it is accepted.
 *
 * ## Why this is a component
 *
 * The statement is a precondition of the *product*, not of a screen: the safety
 * gate refuses every install until it is recorded, and the automatic run refuses
 * at its door for the same reason. It used to be drawn on the game screen alone,
 * so a person who reached the automatic run from the library — which is the
 * route the one-button translation is designed around — met a refusal naming a
 * statement that was nowhere on the screen in front of them, after the extraction
 * and the whole machine translation had already run. The statement now travels
 * with every gate it holds.
 *
 * ## What travels with it
 *
 * **The words, not a summary.** The panel shows the statement's own text in the
 * session's language, and the button beneath it says what pressing it means. A
 * shorter paraphrase with an "I agree" underneath would be asking somebody to
 * accept something this interface wrote rather than the thing being accepted.
 *
 * **The rendering is recorded.** Which of the two texts was on screen goes to the
 * backend with the acceptance, because a record that cannot say which words were
 * accepted cannot show that the person was asked in a language they read.
 *
 * **One record, everywhere.** Both screens read and write the same query key, so
 * accepting on either one makes the panel leave both, and a refusal recorded in
 * one place cannot disagree with a button in another.
 *
 * **It disappears when it is done.** Nothing is shown once the record covers the
 * statement this build ships — a permanently visible legal panel is a panel
 * people learn to scroll past, which is the opposite of the point.
 */

/** The default document-unique fragment this block keys its parts off. */
export const MUARRIF_IQRAR = 'iqrar-awwal';

/**
 * The id of the sentence saying what the outstanding statement is holding.
 *
 * Exported for the same reason {@link muarrifMatlub} is: on the automatic screen
 * the primary action sits in the page header, far above this panel, and a grey
 * button with no reason attached is exactly the failure this exists to correct.
 *
 * [muarrifMatlub]: ../iqrar_khatar
 */
export function muarrifSababIqrar(muarrif: string = MUARRIF_IQRAR): string {
  return `${muarrif}-sabab`;
}

/** What the statement's state is, for a screen that has to gate its own buttons on it. */
export interface HalatIqrarAwwal {
  /** Whether the statement still stands between this session and every install. */
  readonly yahtaj: boolean;
  /** Whether the answer is known yet. */
  readonly jahiz: boolean;
}

/**
 * The statement's standing.
 *
 * A screen that gates a control on it calls this; the panel below calls it too,
 * and the query cache makes that one read rather than two.
 *
 * Unknown reads as outstanding. The gate it stands for is an install, and the
 * safe reading of "we could not tell whether this was accepted" is the one that
 * waits — the backend refuses on exactly that reading, so a button that unlocked
 * itself here would be a button that fails at the end of a paid run.
 */
export function useIqrarAwwal(): HalatIqrarAwwal {
  const iqrar = useQuery<HalatIqrar, KhataJisr>({
    queryKey: mafatih.iqrar,
    queryFn: () => nadi('iqrar_aman'),
  });
  return {
    yahtaj: iqrar.data?.yahtaj !== false,
    jahiz: iqrar.data !== undefined,
  };
}

interface Khasais {
  readonly lugha: Lugha;
  /** A document-unique fragment, when one block is not the only one on the page. */
  readonly muarrif?: string;
  /**
   * What the outstanding statement is holding on *this* screen, in this screen's
   * own terms. Shown only while it holds something, because a requirement
   * already met is noise.
   */
  readonly matlub?: string | null;
  /** The game the failure block routes its steps for. Both screens carrying this panel are about one. */
  readonly luba: string;
}

export function QismIqrar({
  lugha,
  muarrif = MUARRIF_IQRAR,
  matlub = null,
  luba,
}: Khasais): JSX.Element {
  const makhzan = useQueryClient();
  const iqrar = useQuery<HalatIqrar, KhataJisr>({
    queryKey: mafatih.iqrar,
    queryFn: () => nadi('iqrar_aman'),
  });

  const sajjil = useMutation<HalatIqrar, KhataJisr, void>({
    // The rendering below, not the stored preference: the record has to name
    // the words this person read, and the panel draws one of the two by the
    // session's own language.
    mutationFn: () => nadi('sajjil_iqrar_aman', { lugha }),
    onSuccess: (hala) => {
      makhzan.setQueryData(mafatih.iqrar, hala);
      // The panel leaves the screen the moment this lands, so the notice is
      // what says the press worked and what it unlocked.
      ansha({ naw: 'najah', nass: t('iqrar.tanbih.sujjil', lugha) });
    },
  });

  const yahtaj = iqrar.data?.yahtaj !== false;
  const muarrifUnwan = `${muarrif}-unwan`;
  const muarrifSabab = muarrifSababIqrar(muarrif);

  const aad = (): void => {
    if (!sajjil.isPending) {
      sajjil.mutate();
    }
  };

  // A statement that cannot be read is not a statement that was accepted, and
  // every install is refused until it can be. Shown rather than swallowed,
  // because the alternative is a screen whose buttons are all inert with nothing
  // on it saying why.
  if (iqrar.error !== null) {
    return (
      <KutlatKhata
        unwan={t('iqrar.khata', lugha)}
        khata={iqrar.error}
        lugha={lugha}
        muarrif={luba}
        aada={() => {
          void iqrar.refetch();
        }}
      />
    );
  }

  return (
    <Zuhur maftuh={yahtaj && iqrar.data !== undefined} asl="mahall">
      {iqrar.data === undefined ? null : (
        <section className="iqrar-awwal" aria-labelledby={muarrifUnwan}>
          <h2 id={muarrifUnwan} className="iqrar-awwal__unwan">
            {t('iqrar.unwan', lugha)}
          </h2>
          {/* The statement's own words, in the session's language. The Arabic
              carries its direction and its language tag explicitly because the
              English half of the block around it does not. */}
          {lugha === 'arabi' ? (
            <p className="iqrar-awwal__nass" dir="rtl" lang="ar">
              {iqrar.data.nass_arabi}
            </p>
          ) : (
            <p className="iqrar-awwal__nass" dir="ltr" lang="en">
              {iqrar.data.nass_injilizi}
            </p>
          )}
          {matlub === null ? null : (
            <p id={muarrifSabab} className="iqrar-awwal__matlub">
              {matlub}
            </p>
          )}
          <div className="iqrar-awwal__afal">
            <button
              type="button"
              className="zir zir--tamyeez"
              aria-busy={sajjil.isPending}
              onClick={aad}
            >
              {t(sajjil.isPending ? 'iqrar.jari' : 'iqrar.zirr', lugha)}
            </button>
          </div>
          {sajjil.error !== null ? (
            <KutlatKhata
              unwan={t('iqrar.khata', lugha)}
              khata={sajjil.error}
              lugha={lugha}
              muarrif={luba}
              aada={aad}
            />
          ) : null}
        </section>
      )}
    </Zuhur>
  );
}
