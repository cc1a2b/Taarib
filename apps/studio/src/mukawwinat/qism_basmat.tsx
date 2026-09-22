import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { JSX } from 'react';
import { useMemo, useState } from 'react';

import { mafatih } from '@/hayat/istifsar';
import { KhataJisr, ghayrMusajjal, nadi, nadiKhariji } from '@/hayat/jisr';
import { ansha } from '@/hayat/tanbihat';
import type { Munassiqat } from '@/lugha/lugha';
import { t, wasm } from '@/lugha/lugha';
import { Basma, majmuatMukhtalifa } from '@/mukawwinat/basma';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { ItimanKhariji } from '@/mukawwinat/itiman_khariji';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import type { Lugha } from '@/mustalahat/awamir';
import type { BasmaMualaqa } from '@/mustalahat/khariji';
import { TUL_TAAKID_BASMA, dhaylBasma, majmuatBasma, qarrirBasmat } from '@/mustalahat/khariji';

import './qism_basmat.css';

/**
 * قسم البصمات المعلَّقة — the owner re-pinning an artifact whose author has
 * shipped new bytes.
 *
 * ## What this panel is for
 *
 * A third-party entry pins every artifact by hash. When the author ships a new
 * version the card in the game screen's catalogue offers it, and installing it
 * is refused: the bytes at that address are no longer the bytes the owner
 * looked at, and a version number having moved is not evidence about what is
 * now behind the URL. Only the owner can accept the new bytes, and this is
 * where they do it.
 *
 * ## What stops it being one careless click
 *
 * Three things, and they are cumulative.
 *
 * **The difference is made readable.** Both hashes are shown in full, side by
 * side, cut into eight-character groups, and the groups that differ are marked
 * by weight and by a rule under them as well as by colour. A reader asked
 * whether two sixty-four-character runs are the same string will check the head
 * and the tail and say yes; the mark is what turns that into a decision rather
 * than a glance. The count of differing groups is written out in words beside
 * them, because the visual mark conveys nothing to a screen reader.
 *
 * **Acceptance is a transcription, not a press.** The Accept control is refused
 * until the owner has typed the last {@link TUL_TAAKID_BASMA} characters of the
 * *new* hash into the field beside it. That is short enough to read off the
 * screen and long enough that it cannot be produced by anything except having
 * looked at the value being accepted. There is no keyboard route past it: the
 * button is refused and says why.
 *
 * **The hash travels in the request.** The accepted hash is sent back to the
 * backend, which compares it against what it actually observed and refuses if
 * the two differ. So an accept can only ever pin the bytes that were on the
 * screen when the owner pressed it — a server that changes what it serves
 * between the observation and the acceptance cannot slip a third set of bytes
 * through this panel.
 *
 * ## Credit here too
 *
 * Every row names the maker and their team, and opens their page. A console row
 * is still a listing of somebody else's work, and Taarib does not present one as
 * its own anywhere it appears.
 */

/** One row's identity: an artifact belongs to an entry, and both are needed. */
function miftahSaff(basma: BasmaMualaqa): string {
  return `${basma.ruqaa}:${basma.qitaa}`;
}

interface KhasaisSaff {
  readonly basma: BasmaMualaqa;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** When the change was observed, as the reader's calendar writes it. */
  readonly waqt: string;
  /** What the owner has typed back, for this row. */
  readonly taakid: string;
  readonly alaTaakid: (nass: string) => void;
  /** Whether this row's acceptance is the one running. */
  readonly yuthbit: boolean;
  /** Whether any acceptance in this panel is running. */
  readonly mashghul: boolean;
  readonly alaQabul: () => void;
  readonly yaftahRabt: boolean;
  readonly iftahRabt: () => void;
}

function SaffBasma({
  basma,
  lugha,
  munassiq,
  waqt,
  taakid,
  alaTaakid,
  yuthbit,
  mashghul,
  alaQabul,
  yaftahRabt,
  iftahRabt,
}: KhasaisSaff): JSX.Element {
  const mifta = miftahSaff(basma);
  const mukhtalifa = majmuatMukhtalifa(basma.sha256_mathbut, basma.sha256_marsud);
  const majmu = Math.max(
    majmuatBasma(basma.sha256_mathbut).length,
    majmuatBasma(basma.sha256_marsud).length,
  );
  const dhayl = dhaylBasma(basma.sha256_marsud);
  const mutabiq = taakid.trim().toLowerCase() === dhayl;
  const muarrifHaql = `basmat-taakid-${mifta}`;
  const muarrifMatlub = `${muarrifHaql}-matlub`;

  return (
    <li className="qism-basmat__saff">
      <div className="qism-basmat__raas">
        <span className="qism-basmat__saff-unwan" dir="auto">
          {basma.unwan}
        </span>
        <span className="mono-ltr qism-basmat__qitaa">{basma.qitaa}</span>
      </div>

      <ItimanKhariji
        muallif={basma.muallif}
        fariq={basma.fariq}
        lugha={lugha}
        className="qism-basmat__itiman"
      />

      {/*
        The two hashes, side by side and in full.

        Side by side rather than one above the other because the comparison is
        made across, group against group: stacked, the eye has to carry eight
        characters down a column to the group under them, which is the reading
        this panel exists to make unnecessary. On a narrow console the columns
        stack anyway, and the marked groups are then the only way to read the
        difference — which is why the mark is never colour alone.
      */}
      <div className="qism-basmat__muqarana">
        <div className="qism-basmat__amud">
          <p className="qism-basmat__tasmiya">{t('muraja.basmat.mathbut', lugha)}</p>
          <p className="qism-basmat__tafsil">
            {t('muraja.basmat.qeema', lugha, {
              isdar: basma.isdar_mathbut,
              hajm: munassiq.hajm(basma.hajm_mathbut),
            })}
          </p>
          <Basma basma={basma.sha256_mathbut} mukhtalifa={mukhtalifa} />
        </div>
        <div className="qism-basmat__amud qism-basmat__amud--warid">
          <p className="qism-basmat__tasmiya">{t('muraja.basmat.marsud', lugha)}</p>
          <p className="qism-basmat__tafsil">
            {t('muraja.basmat.qeema', lugha, {
              isdar: basma.isdar,
              hajm: munassiq.hajm(basma.hajm),
            })}
          </p>
          <Basma basma={basma.sha256_marsud} mukhtalifa={mukhtalifa} />
        </div>
      </div>

      {/* The mark above is weight and a rule; neither reaches a screen reader,
          and neither survives a reader skimming. The count says it in words. */}
      <p className="qism-basmat__farq">
        {t('muraja.basmat.farq', lugha, {
          adad: munassiq.raqm(mukhtalifa.size),
          majmu: munassiq.raqm(majmu),
        })}
      </p>

      <p className="qism-basmat__nass-hadi">{t('muraja.basmat.waqt', lugha, { waqt })}</p>
      {/* Where the changed bytes are actually served from, labelled: an
          unlabelled address in a panel about trust is a mystery, and this is
          the address the pin is being taken against. */}
      <p className="qism-basmat__tasmiya">{t('muraja.basmat.rabt', lugha)}</p>
      <p className="qism-basmat__nass-hadi">
        <span className="mono-ltr qism-basmat__rabt" dir="ltr">
          {basma.rabt}
        </span>
      </p>

      <div className="qism-basmat__taakid">
        <label className="qism-basmat__tasmiya" htmlFor={muarrifHaql}>
          {t('muraja.basmat.taakid', lugha, { tul: munassiq.raqm(TUL_TAAKID_BASMA) })}
        </label>
        <input
          id={muarrifHaql}
          className="mono-ltr qism-basmat__haql"
          type="text"
          dir="ltr"
          spellCheck={false}
          autoComplete="off"
          // The field wants exactly these characters and nothing else. A field
          // sixty-four characters wide invites pasting a hash out of whatever
          // message claimed the new version exists, which is the one source this
          // transcription is here to route around.
          maxLength={TUL_TAAKID_BASMA}
          value={taakid}
          aria-describedby={mutabiq ? undefined : muarrifMatlub}
          onChange={(hadath) => {
            alaTaakid(hadath.target.value);
          }}
        />
        {mutabiq ? null : (
          <p id={muarrifMatlub} className="qism-basmat__matlub">
            {t('muraja.basmat.matlub', lugha)}
          </p>
        )}
      </div>

      <div className="qism-basmat__afal">
        <button
          type="button"
          className="zir zir--khatar"
          aria-disabled={!mutabiq || mashghul}
          aria-busy={yuthbit}
          aria-describedby={mutabiq ? undefined : muarrifMatlub}
          onClick={alaQabul}
        >
          {t('muraja.basmat.zir', lugha)}
        </button>
        <button type="button" className="zir" aria-busy={yaftahRabt} onClick={iftahRabt}>
          {t(yaftahRabt ? 'muraja.basmat.jari_fath' : 'muraja.basmat.iftah', lugha)}
        </button>
        <span className="mono-ltr qism-basmat__rabt-muallif" dir="ltr" title={basma.rabt_muallif}>
          {basma.rabt_muallif}
        </span>
      </div>
    </li>
  );
}

interface Khasais {
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** The panel is the owner's; the console draws it only inside its own gate. */
  readonly malik: boolean;
}

export function QismBasmat({ lugha, munassiq, malik }: Khasais): JSX.Element | null {
  const makhzan = useQueryClient();
  const [taakidat, setTaakidat] = useState<Readonly<Record<string, string>>>({});

  const munWaqt = useMemo(
    () =>
      new Intl.DateTimeFormat(wasm(lugha), { dateStyle: 'medium', timeStyle: 'short' }),
    [lugha],
  );
  /** An RFC 3339 instant as the reader's calendar writes it, or as it came. */
  const nassWaqt = (khaam: string): string => {
    const tarikh = new Date(khaam);
    return Number.isNaN(tarikh.getTime()) ? khaam : munWaqt.format(tarikh);
  };

  const basmat = useQuery<readonly BasmaMualaqa[], KhataJisr>({
    queryKey: mafatih.basmat,
    queryFn: () => nadiKhariji('basmat_mualaqa', {}, qarrirBasmat),
    enabled: malik,
  });

  const qabul = useMutation<
    boolean,
    KhataJisr,
    { readonly ruqaa: string; readonly qitaa: string; readonly sha256: string }
  >({
    mutationFn: ({ ruqaa, qitaa, sha256 }) =>
      // The answer is not read. What this panel does next is ask for the
      // pending set again — the accepted row leaves it, and any row the same
      // fetch discovered arrives — so trusting a shape the command surface has
      // not agreed on would buy nothing and would refuse a correct answer for
      // being the wrong type. The call either succeeded or threw.
      nadiKhariji('athbit_basma', { ruqaa, qitaa, sha256 }, () => true),
    onSuccess: (_natija, { ruqaa, qitaa }) => {
      setTaakidat((hali) => {
        const baqi = { ...hali };
        delete baqi[`${ruqaa}:${qitaa}`];
        return baqi;
      });
      void makhzan.invalidateQueries({ queryKey: mafatih.basmat });
      // Every game screen listing this entry was offering an update that is now
      // pinned, and its install is no longer refused.
      void makhzan.invalidateQueries({ queryKey: ['kharijiya'] });
      ansha({ naw: 'najah', nass: t('muraja.basmat.tanbih', lugha), tafsil: qitaa });
    },
  });

  const fath = useMutation<boolean, KhataJisr, string>({
    mutationFn: (rabt) => nadi('iftah_rabt', { rabt }),
    onError: (khata) => {
      ansha({
        naw: 'khatar',
        nass: t('muraja.basmat.khata_fath', lugha),
        tafsil: khata.nass(lugha) ?? khata.message,
        ramz: khata.khata?.ramz ?? null,
      });
    },
  });

  // The panel is the owner's, and the console already draws it inside its own
  // gate. This is the second lock on that door rather than a belt on a brace: a
  // disabled query stays `pending` for ever, so without this a session that
  // somehow reached the panel would sit under a skeleton that never resolves.
  if (!malik) {
    return null;
  }

  // This build of the shell has no third-party support at all: not a failure,
  // and a failure block in the console on every visit would say otherwise.
  if (basmat.error !== null && ghayrMusajjal(basmat.error)) {
    return null;
  }

  const wajh = basmat.isPending ? 'tahmil' : basmat.error !== null ? 'khata' : 'jahiz';

  return (
    <section className="qism-basmat" aria-labelledby="basmat-unwan">
      <h3 id="basmat-unwan" className="qism-basmat__unwan">
        {t('muraja.basmat.unwan', lugha)}
      </h3>
      <p className="qism-basmat__nass-hadi">{t('muraja.basmat.wasf', lugha)}</p>

      <Mashhad miftah={wajh}>
        {basmat.isPending ? (
          <div className="qism-basmat__haykal zuhur-muakhkhar">
            <span className="khafi" role="status">
              {t('muraja.basmat.jari', lugha)}
            </span>
            <span aria-hidden="true" className="haykal__satr haykal__satr--tawil" />
            <span aria-hidden="true" className="haykal__satr haykal__satr--mutawassit" />
            <span aria-hidden="true" className="haykal__satr haykal__satr--qasir" />
          </div>
        ) : basmat.error !== null ? (
          <KutlatKhata
            unwan={t('muraja.basmat.taadhur', lugha)}
            khata={basmat.error}
            lugha={lugha}
            aada={() => {
              void basmat.refetch();
            }}
          />
        ) : basmat.data === undefined || basmat.data.length === 0 ? (
          <HalatFarigha
            unwan={t('muraja.basmat.farigh_unwan', lugha)}
            nass={t('muraja.basmat.farigh', lugha)}
          />
        ) : (
          <ul className="qism-basmat__qaima">
            {basmat.data.map((basma) => {
              const mifta = miftahSaff(basma);
              return (
                <SaffBasma
                  key={mifta}
                  basma={basma}
                  lugha={lugha}
                  munassiq={munassiq}
                  waqt={nassWaqt(basma.waqt)}
                  taakid={taakidat[mifta] ?? ''}
                  alaTaakid={(nass) => {
                    setTaakidat((hali) => ({ ...hali, [mifta]: nass }));
                  }}
                  yuthbit={
                    qabul.isPending &&
                    qabul.variables?.ruqaa === basma.ruqaa &&
                    qabul.variables.qitaa === basma.qitaa
                  }
                  mashghul={qabul.isPending}
                  alaQabul={() => {
                    // Asked again here and not only where the control was drawn:
                    // the field can change between the render that enabled the
                    // button and the press, and the acceptance must carry the
                    // hash the owner actually transcribed.
                    const maktub = (taakidat[mifta] ?? '').trim().toLowerCase();
                    if (
                      !qabul.isPending &&
                      maktub === dhaylBasma(basma.sha256_marsud)
                    ) {
                      qabul.mutate({
                        ruqaa: basma.ruqaa,
                        qitaa: basma.qitaa,
                        sha256: basma.sha256_marsud,
                      });
                    }
                  }}
                  yaftahRabt={fath.isPending && fath.variables === basma.rabt_muallif}
                  iftahRabt={() => {
                    if (!fath.isPending) {
                      fath.mutate(basma.rabt_muallif);
                    }
                  }}
                />
              );
            })}
          </ul>
        )}
      </Mashhad>

      {qabul.error === null ? null : (
        <KutlatKhata unwan={t('muraja.basmat.khata', lugha)} khata={qabul.error} lugha={lugha} />
      )}
    </section>
  );
}
