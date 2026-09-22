import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'motion/react';
import type { JSX, KeyboardEvent, ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';

import { mafatih } from '@/hayat/istifsar';
import { KhataJisr, ghayrMusajjal, nadi, nadiKhariji } from '@/hayat/jisr';
import { ansha } from '@/hayat/tanbihat';
import type { MiftahLugha, Munassiqat } from '@/lugha/lugha';
import { jam, t } from '@/lugha/lugha';
import { Basma } from '@/mukawwinat/basma';
import { HalatFarigha } from '@/mukawwinat/halat_farigha';
import { IqrarKhatar, muarrifMatlub } from '@/mukawwinat/iqrar_khatar';
import { ItimanKhariji, nassItiman } from '@/mukawwinat/itiman_khariji';
import { KutlatKhata } from '@/mukawwinat/kutlat_khata';
import { Mashhad } from '@/mukawwinat/mashhad';
import { Zuhur } from '@/mukawwinat/zuhur';
import type { Lugha } from '@/mustalahat/awamir';
import type { TaqreerIzalaHie } from '@/mustalahat/awamir';
import type {
  IdhnMasdar,
  NatijatKharijiya,
  QitaatTanzeel,
  RukhsaRuqaa,
  RuqaaKharijiya,
  TadakhulKhariji,
  TahdheerKhariji,
} from '@/mustalahat/khariji';
import {
  hajmQitaa,
  maadhun,
  qarrirKatalog,
  qarrirNatija,
  qarrirTadakhul,
  qitaaLiBina,
} from '@/mustalahat/khariji';
import { HARAKAT_LAWHA, haraka } from '@/nizam/haraka';

import './qism_khariji.css';

/**
 * قسم الرقع الخارجية — patches other teams made, listed and installed but never
 * built, on the game screen beside the ones Taarib compiled.
 *
 * ## The marker, and why it is this shape
 *
 * These entries sit in the same list region as Taarib's own patches, wear the
 * same card, and offer the same verb. At a glance they must still be
 * unmistakably a different *kind* of thing — and the difference has to read as a
 * category rather than as a caution, because this is legitimate work being
 * offered rather than something to be wary of. So the marker is neither a colour
 * swap nor a hazard badge:
 *
 * **The rail is stitched rather than solid.** Every install card on this screen
 * carries a two-hairline rule along its leading edge in the game's own accent —
 * `luba__lawhat-tathbeet` in `luba.css` — and that rule is what "Taarib's own"
 * looks like here. A third-party card keeps the box, the ground and the hairline
 * and replaces that rule with a run of short segments in the neutral text
 * colour. The edge reads as bound in rather than built in, the difference
 * carries in peripheral vision down a column of cards, and it is a *shape*: it
 * survives a monochrome display, both palettes, the high-contrast set, and a
 * reader who cannot tell teal from grey.
 *
 * **The same stitch is the plate's glyph.** The card's head carries one neutral
 * plate naming the category, in the vocabulary the library card already uses for
 * facts about a game — `--sath-0`, a hairline, `--nass-2` — with the rail's own
 * stitch drawn at glyph size beside the words. Two marks that say one thing.
 *
 * **The credit is the card's subject line.** A Taarib card leads with its title
 * and drops the contributor to a quiet second line. This one states whose work
 * it is directly under the title, at reading weight, because that is the fact
 * the card exists to carry.
 *
 * Nothing here is `--khatar` and nothing is `--tanbeeh`. The one amber rule on
 * these cards belongs to the maker's own safety warnings, which are a different
 * statement and are shown at the moment they apply.
 *
 * ## What is said, and where
 *
 * The credit is on the card. What Taarib did and did not do is on the card, in
 * the open, above everything that can be folded away — a sentence that matters
 * only before the decision cannot live behind a disclosure. The maker's own
 * warnings are shown when install is pressed and before anything is fetched,
 * in the entry's own words rather than in this interface's. A collision names
 * what is already there and what has to go first.
 *
 * ## While the backend half is being written
 *
 * The three commands behind this section are not in the generated bindings yet,
 * so they go through `nadiKhariji` and every answer is decoded before it is
 * read. A shell that has not registered them at all answers "command not found",
 * which {@link ghayrMusajjal} recognises: this build simply does not offer
 * third-party patches, the section draws nothing, and a red failure block on
 * every game screen would be saying something went wrong when nothing did.
 */

/** The licence the work was offered under, named from the string set. */
const MIFTAH_RUKHSA: Readonly<Record<RukhsaRuqaa['naw'], MiftahLugha>> = {
  cc0: 'khariji.rukhsa.cc0',
  cc_by: 'khariji.rukhsa.cc_by',
  cc_by_sa: 'khariji.rukhsa.cc_by_sa',
  milkiya_khassa: 'khariji.rukhsa.milkiya_khassa',
  ukhra: 'khariji.rukhsa.ukhra',
};

/**
 * The licence as one line: a licence the importer could not place is named by
 * whatever the page called it, which is the one word a reader can look up, and
 * falls back to the kind when the page named nothing at all.
 */
function nassRukhsa(rukhsa: RukhsaRuqaa, lugha: Lugha): string {
  if (rukhsa.naw === 'ukhra') {
    return rukhsa.ism.trim() === '' ? t(MIFTAH_RUKHSA.ukhra, lugha) : rukhsa.ism;
  }
  return t(MIFTAH_RUKHSA[rukhsa.naw], lugha);
}

/** What establishes the right to carry the work, named from the string set. */
function miftahIdhn(idhn: IdhnMasdar): MiftahLugha {
  if (idhn === 'rukhsa') {
    return 'khariji.idhn.rukhsa';
  }
  if (idhn === 'lam_yuthbat') {
    return 'khariji.idhn.lam_yuthbat';
  }
  return 'khariji.idhn.katabi';
}

/** The maker's own written grant, when that is what the permission is. */
function bayanIdhn(idhn: IdhnMasdar): string | null {
  return typeof idhn === 'string' ? null : idhn.katabi.bayan;
}

interface KhasaisSaff {
  readonly unwan: string;
  readonly children: ReactNode;
}

/** One labelled fact. The game screen's own row, in this section's stylesheet. */
function Saff({ unwan, children }: KhasaisSaff): JSX.Element {
  return (
    <div className="qism-khariji__saff">
      <dt>{unwan}</dt>
      <dd>{children}</dd>
    </div>
  );
}

/** The lines this section's query reserves while it runs. */
function Haykal({ nass }: { readonly nass: string }): JSX.Element {
  return (
    <div className="qism-khariji__haykal zuhur-muakhkhar">
      <span className="khafi" role="status">
        {nass}
      </span>
      {['tawil', 'mutawassit', 'qasir', 'tawil'].map((tul, martaba) => (
        <span
          key={`${String(martaba)}:${tul}`}
          aria-hidden="true"
          className={`haykal__satr haykal__satr--${tul}`}
        />
      ))}
    </div>
  );
}

/**
 * One fetched artifact, pinned.
 *
 * The address and the hash are both shown, and neither is abbreviated. The
 * address is where the bytes actually come from, which is the one thing a
 * reader cannot check anywhere else on this screen; the hash is the promise
 * that what arrives from that address is what the owner looked at.
 */
function Qitaa({
  qitaa,
  lugha,
  munassiq,
}: {
  readonly qitaa: QitaatTanzeel;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
}): JSX.Element {
  return (
    <li className="qism-khariji__qitaa">
      <p className="qism-khariji__qitaa-raas">
        <span className="mono-ltr qism-khariji__qitaa-ism">{qitaa.ism}</span>
        <span className="qism-khariji__qitaa-hajm">{munassiq.hajm(qitaa.hajm)}</span>
      </p>
      <dl className="qism-khariji__jadwal">
        <Saff unwan={t('khariji.qitaa.rabt', lugha)}>
          <span className="mono-ltr qism-khariji__rabt" dir="ltr">
            {qitaa.rabt}
          </span>
        </Saff>
        <Saff unwan={t('khariji.qitaa.abniya', lugha)}>
          {qitaa.abniya.length === 0 ? (
            t('khariji.qitaa.kul_abniya', lugha)
          ) : (
            <ul className="qism-khariji__riqaq">
              {qitaa.abniya.map((bina) => (
                <li key={bina} className="qism-khariji__riqaqa mono-ltr">
                  {bina}
                </li>
              ))}
            </ul>
          )}
        </Saff>
        <Saff unwan={t('khariji.qitaa.basma', lugha)}>
          <Basma basma={qitaa.sha256} />
        </Saff>
      </dl>
    </li>
  );
}

/** A list of paths, each on its own line, in its own direction. */
function QaimatMasarat({
  unwan,
  masarat,
  hadi,
}: {
  readonly unwan: string;
  readonly masarat: readonly string[];
  /** A sentence under the heading, when the list needs one to be honest. */
  readonly hadi?: string;
}): JSX.Element | null {
  if (masarat.length === 0) {
    return null;
  }
  return (
    <>
      <h3 className="qism-khariji__unwan-farii">{unwan}</h3>
      {hadi === undefined ? null : <p className="qism-khariji__nass-hadi">{hadi}</p>}
      <ul className="qism-khariji__masarat">
        {masarat.map((masar, martaba) => (
          <li key={`${String(martaba)}:${masar}`} className="mono-ltr" title={masar}>
            {masar}
          </li>
        ))}
      </ul>
    </>
  );
}

/**
 * What is already in the game directory that this entry cannot sit beside.
 *
 * A refusal that says only "conflict" sends a person to a forum. This names the
 * patch that is there, the loader slot the two are fighting over, and every path
 * that has to go before the install can run — and it says which of the two kinds
 * of removal that is: Taarib's own, which the strip further down this screen
 * performs and can undo, or another team's, which Taarib did not write and will
 * not delete on somebody's behalf.
 */
function Tadakhul({
  tadakhul,
  lugha,
  alaIzala,
}: {
  readonly tadakhul: TadakhulKhariji;
  readonly lugha: Lugha;
  readonly alaIzala: () => void;
}): JSX.Element {
  const sahib = lugha === 'arabi' ? tadakhul.sahib_arabi : tadakhul.sahib_injilizi;
  return (
    <div className="qism-khariji__tadakhul">
      <h3 className="qism-khariji__unwan-farii">{t('khariji.tadakhul.unwan', lugha)}</h3>
      <p className="qism-khariji__tadakhul-nass" dir="auto">
        {t('khariji.tadakhul.nass', lugha, { sahib, manfadh: tadakhul.manfadh })}
      </p>
      <QaimatMasarat unwan={t('khariji.tadakhul.yuzal', lugha)} masarat={tadakhul.yuzal} />
      <p className="qism-khariji__nass-hadi">
        {t(
          tadakhul.taarib_yuzil ? 'khariji.tadakhul.taarib' : 'khariji.tadakhul.ghayr_taarib',
          lugha,
        )}
      </p>
      {tadakhul.taarib_yuzil ? (
        <div className="qism-khariji__afal">
          <button type="button" className="zir" onClick={alaIzala}>
            {t('khariji.tadakhul.idhhab', lugha)}
          </button>
        </div>
      ) : null}
    </div>
  );
}

/** The maker's own safety sentences, in the reader's language, unsoftened. */
function Tahdheerat({
  tahdheerat,
  lugha,
}: {
  readonly tahdheerat: readonly TahdheerKhariji[];
  readonly lugha: Lugha;
}): JSX.Element | null {
  if (tahdheerat.length === 0) {
    return null;
  }
  return (
    <div className="qism-khariji__tahdheerat">
      <h3 className="qism-khariji__unwan-farii">{t('khariji.tahdheer.unwan', lugha)}</h3>
      <ul className="qism-khariji__tahdheer-qaima">
        {tahdheerat.map((tahdheer, martaba) => (
          // Keyed by position and by the sentence itself: the list is the
          // entry's own bytes and its order is part of what was published.
          <li key={`${String(martaba)}:${tahdheer.injilizi}`} dir="auto">
            {lugha === 'arabi' ? tahdheer.arabi : tahdheer.injilizi}
          </li>
        ))}
      </ul>
    </div>
  );
}

interface KhasaisMadkhal {
  readonly muarrif: string;
  readonly madkhal: RuqaaKharijiya;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** Actions are locked while the game runs or the first-run statement stands. */
  readonly muqfal: boolean;
  /** Why they are locked, in the reader's language, or null when they are not. */
  readonly sababQafl: string | null;
  /** Whether this row's install is the one running. */
  readonly yuthabbat: boolean;
  /** Whether any install in this section is running. */
  readonly mashghul: boolean;
  /** Whether this row's warnings and acknowledgement are open. */
  readonly maftuh: boolean;
  readonly muqirr: boolean;
  readonly alaIqrar: (muqirr: boolean) => void;
  /** Opens or closes this row's warnings. */
  readonly alaTalab: () => void;
  readonly alaIlgha: () => void;
  readonly alaTanfidh: () => void;
  /** Registers this row's install button, so cancelling returns focus to it. */
  readonly sajjilZir: (uqda: HTMLButtonElement | null) => void;
  /** The cancel button, which focus lands on when the panel opens. */
  readonly marjaIlgha: (uqda: HTMLButtonElement | null) => void;
  /** Takes the reader to the removal strip further down the game screen. */
  readonly alaIzala: () => void;
  /** Takes this entry back off, which the strip above cannot do. */
  readonly alaIzalatNafsiha: () => void;
  readonly tuzal: boolean;
  readonly yaftahRabt: boolean;
  readonly iftahRabt: (rabt: string) => void;
}

function MadkhalKhariji({
  muarrif,
  madkhal,
  lugha,
  munassiq,
  muqfal,
  sababQafl,
  yuthabbat,
  mashghul,
  maftuh,
  muqirr,
  alaIqrar,
  alaTalab,
  alaIlgha,
  alaTanfidh,
  sajjilZir,
  marjaIlgha,
  alaIzala,
  alaIzalatNafsiha,
  tuzal,
  yaftahRabt,
  iftahRabt,
}: KhasaisMadkhal): JSX.Element {
  /**
   * What is in the way, asked per entry because the answer is about a
   * particular loader slot: a game holding Taarib's own `version.dll` blocks one
   * entry and not another. It walks the game directory, so it is asked only
   * where it changes a decision — an entry that is already installed is not
   * about to collide with anything.
   */
  const tadakhul = useQuery<TadakhulKhariji, KhataJisr>({
    queryKey: mafatih.tadakhul(muarrif, madkhal.id),
    queryFn: () =>
      nadiKhariji('tadakhul_kharijiya', { muarrif, ruqaa: madkhal.id }, qarrirTadakhul),
    enabled: !madkhal.muthabbata,
  });

  const qitaa = qitaaLiBina(madkhal, madkhal.bina_mutabaqa);
  const hajm = hajmQitaa(qitaa);
  const idhnMaadhun = maadhun(madkhal.masdar);
  const mutadakhil = tadakhul.data?.mutadakhil === true;
  const bayan = bayanIdhn(madkhal.masdar.idhn);
  const muarrifIqrar = `khariji-iqrar-${madkhal.id}`;

  // Refused, and every reason for it said rather than left to a grey button.
  // The collision is checked before the install is offered, not after: a
  // control that is going to be refused by the backend is worse than no control.
  const sababRafd =
    sababQafl !== null
      ? sababQafl
      : !idhnMaadhun
        ? t('khariji.idhn.matlub', lugha)
        : mutadakhil
          ? t('khariji.tadakhul.matlub', lugha)
          : tadakhul.error !== null
            ? t('khariji.tadakhul.majhul', lugha)
            : null;
  // The lock is checked as well as its sentence, on the same terms the
  // registry's own listing uses: `sababQafl` is what a locked screen says, and
  // a control refused by the lock must not become live because a sentence went
  // missing on the way here.
  const yaqbal = !muqfal && sababRafd === null && !mashghul;

  return (
    <li className="luba__lawhat-tathbeet qism-khariji__madkhal">
      <div className="qism-khariji__raas">
        <span className="qism-khariji__unwan" dir="auto">
          {madkhal.unwan}
        </span>
        {/*
          The category plate. Hidden from assistive technology because the
          section's own heading and the credit line below already say, in words,
          exactly what it says in three: announced here as well it would be the
          same fact read three times in a row.
        */}
        <span className="qism-khariji__wasm" aria-hidden="true">
          {t('khariji.wasm', lugha)}
        </span>
      </div>

      {/* Whose work this is, directly under the title and at reading weight. */}
      <ItimanKhariji
        muallif={madkhal.masdar.ism}
        fariq={madkhal.fariq}
        lugha={lugha}
        className="qism-khariji__itiman"
      />

      {/*
        What Taarib did and did not do, in the open.

        Not inside the disclosure below it, and not at the foot of the card: it
        is the sentence that decides how much weight to put on everything else
        here, and a sentence that only matters before a decision cannot live
        somewhere a reader arrives at afterwards.
      */}
      <p className="qism-khariji__tahaqquq">{t('khariji.tahaqquq', lugha)}</p>

      <dl className="qism-khariji__jadwal">
        <Saff unwan={t('khariji.isdar', lugha)}>
          <span dir="auto">{madkhal.isdar}</span>
        </Saff>
        <Saff unwan={t('khariji.bina', lugha)}>
          {madkhal.bina_mutabaqa === null ? (
            <span className="qism-khariji__nass-hadi">{t('khariji.bina_majhul', lugha)}</span>
          ) : (
            <span className="mono-ltr">{madkhal.bina_mutabaqa}</span>
          )}
        </Saff>
        <Saff unwan={t('khariji.masdar', lugha)}>
          {madkhal.mira.naw === 'min_almuallif' ? (
            t('khariji.mira.min_almuallif', lugha)
          ) : (
            <>
              {t('khariji.mira.min_alsijill', lugha)}
              <span className="mono-ltr qism-khariji__rabt" dir="ltr">
                {madkhal.mira.rabt}
              </span>
            </>
          )}
        </Saff>
        <Saff unwan={t('khariji.tanzeel_unwan', lugha)}>
          {jam('khariji.tanzeel', lugha, qitaa.length, munassiq, {
            hajm: munassiq.hajm(hajm),
          })}
        </Saff>
        <Saff unwan={t('khariji.rukhsa', lugha)}>
          <span dir="auto">{nassRukhsa(madkhal.masdar.rukhsa, lugha)}</span>
        </Saff>
        <Saff unwan={t('khariji.idhn', lugha)}>
          {t(miftahIdhn(madkhal.masdar.idhn), lugha)}
          {bayan === null || bayan.trim() === '' ? null : (
            <span className="qism-khariji__bayan" dir="auto">
              {bayan}
            </span>
          )}
        </Saff>
      </dl>

      {/*
        Everything the install touches, one press away from the button that
        spends it. Folded because it is long — for the seed entry it is two
        archives, eight written paths and seventeen removals — and because none
        of it changes the decision the way the six facts above it do. Nothing
        that qualifies the offer is in here; this is the inventory.
      */}
      <details className="qism-khariji__tafsil">
        <summary className="qism-khariji__tafsil-unwan">{t('khariji.tafsil', lugha)}</summary>

        {/* The credit again, because this block is read on its own once it is
            opened and a page of paths with no name on it is an anonymous page. */}
        <ItimanKhariji
          muallif={madkhal.masdar.ism}
          fariq={madkhal.fariq}
          lugha={lugha}
          className="qism-khariji__nass-hadi"
        />

        <h3 className="qism-khariji__unwan-farii">{t('khariji.qitaa.unwan', lugha)}</h3>
        {qitaa.length === 0 ? (
          <p className="qism-khariji__nass-hadi">{t('khariji.qitaa.la_shay', lugha)}</p>
        ) : (
          <ul className="qism-khariji__qitaa-qaima">
            {qitaa.map((wahid) => (
              <Qitaa key={wahid.ism} qitaa={wahid} lugha={lugha} munassiq={munassiq} />
            ))}
          </ul>
        )}

        <QaimatMasarat unwan={t('khariji.yaktub', lugha)} masarat={madkhal.takhtit.yaktub} />
        <QaimatMasarat
          unwan={t('khariji.yahdhif', lugha)}
          masarat={madkhal.takhtit.yahdhif}
          hadi={t('khariji.yahdhif_sharh', lugha)}
        />

        {madkhal.abniya.length === 0 ? null : (
          <>
            <h3 className="qism-khariji__unwan-farii">{t('khariji.abniya', lugha)}</h3>
            <ul className="qism-khariji__riqaq">
              {madkhal.abniya.map((bina) => (
                <li key={bina} className="qism-khariji__riqaqa mono-ltr">
                  {bina}
                </li>
              ))}
            </ul>
          </>
        )}

        {madkhal.hawiyat_manassa.length === 0 ? null : (
          <>
            <h3 className="qism-khariji__unwan-farii">{t('khariji.manassat', lugha)}</h3>
            <ul className="qism-khariji__riqaq">
              {madkhal.hawiyat_manassa.map((hawiya) => (
                <li key={hawiya} className="qism-khariji__riqaqa" dir="auto">
                  {hawiya}
                </li>
              ))}
            </ul>
          </>
        )}
      </details>

      {/* What is in the way, whenever the walk found something. The failure to
          walk at all is its own state below, because "we could not look" is not
          "there is nothing there". */}
      {tadakhul.data === undefined || !tadakhul.data.mutadakhil ? null : (
        <Tadakhul tadakhul={tadakhul.data} lugha={lugha} alaIzala={alaIzala} />
      )}
      {tadakhul.error === null ? null : (
        <KutlatKhata
          unwan={t('khariji.tadakhul.taadhur', lugha)}
          khata={tadakhul.error}
          lugha={lugha}
          muarrif={muarrif}
          aada={() => {
            void tadakhul.refetch();
          }}
        />
      )}

      <div className="qism-khariji__afal">
        {madkhal.muthabbata ? (
          <>
            <p className="qism-khariji__muthabbata">{t('khariji.muthabbata', lugha)}</p>
            <button
              type="button"
              className="zir"
              onClick={alaIzalatNafsiha}
              aria-disabled={tuzal || muqfal}
              aria-busy={tuzal}
            >
              {t(tuzal ? 'khariji.jari_izala' : 'khariji.izalat_nafsiha', lugha)}
            </button>
          </>
        ) : (
          <button
            type="button"
            className="zir zir--tamyeez"
            ref={sajjilZir}
            aria-disabled={!yaqbal}
            aria-busy={yuthabbat}
            aria-expanded={maftuh}
            title={sababRafd ?? undefined}
            onClick={alaTalab}
          >
            {t('khariji.thabbit', lugha)}
          </button>
        )}
        <button
          type="button"
          className="zir"
          aria-busy={yaftahRabt}
          onClick={() => {
            iftahRabt(madkhal.masdar.rabt);
          }}
        >
          {t(yaftahRabt ? 'khariji.jari_fath' : 'khariji.iftah', lugha)}
        </button>
        <span className="mono-ltr qism-khariji__rabt-muallif" dir="ltr" title={madkhal.masdar.rabt}>
          {madkhal.masdar.rabt}
        </span>
      </div>
      {sababRafd === null ? null : (
        <p className="qism-khariji__nass-hadi qism-khariji__rafd">{sababRafd}</p>
      )}

      {/*
        The maker's own warnings, and the one tick that stands for having read
        them, opened inside the entry they are about — never over it, for the
        reason the acknowledgements on Taarib's own patches give: a dialogue
        would take the version, the artifacts and the credit off screen at the
        exact moment they are what the decision rests on.

        Opened by the install button and closed by the cancel beside it. Nothing
        is fetched and nothing is written until the button inside this panel is
        pressed, so the warnings are read before the install rather than after.
      */}
      <AnimatePresence initial={false}>
        {maftuh ? (
          <motion.div
            key="iqrar"
            className="qism-khariji__tawassu"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={haraka(HARAKAT_LAWHA)}
          >
            <div
              className="qism-khariji__tawassu-dakhil"
              onKeyDown={(hadath: KeyboardEvent<HTMLDivElement>) => {
                if (hadath.key === 'Escape') {
                  hadath.stopPropagation();
                  alaIlgha();
                }
              }}
            >
              <Tahdheerat tahdheerat={madkhal.tahdheerat} lugha={lugha} />
              <IqrarKhatar
                muarrif={muarrifIqrar}
                unwan={t('khariji.iqrar.unwan', lugha)}
                tahdheer={t('khariji.iqrar.tahdheer', lugha)}
                tafsil={nassItiman(madkhal.masdar.ism, madkhal.fariq, lugha)}
                nassIqrar={t('khariji.iqrar.nass', lugha)}
                muqirr={muqirr}
                alaTabdil={alaIqrar}
                matlub={t('khariji.iqrar.matlub', lugha)}
              />
              <div className="qism-khariji__afal">
                <button
                  type="button"
                  className="zir zir--khatar"
                  aria-disabled={!yaqbal || !muqirr}
                  aria-describedby={muqirr ? undefined : muarrifMatlub(muarrifIqrar)}
                  onClick={alaTanfidh}
                >
                  {t('khariji.iqrar.thabbit', lugha)}
                </button>
                <button type="button" className="zir" ref={marjaIlgha} onClick={alaIlgha}>
                  {t('khariji.iqrar.ilgha', lugha)}
                </button>
              </div>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </li>
  );
}

interface Khasais {
  readonly muarrif: string;
  readonly lugha: Lugha;
  readonly munassiq: Munassiqat;
  /** A refused game gets no install controls at all, exactly as elsewhere. */
  readonly mahmiya: boolean;
  readonly muqfal: boolean;
  readonly sababQafl: string | null;
  /** Takes the reader to the removal strip further down the game screen. */
  readonly alaIzala: () => void;
}

export function QismKhariji({
  muarrif,
  lugha,
  munassiq,
  mahmiya,
  muqfal,
  sababQafl,
  alaIzala,
}: Khasais): JSX.Element | null {
  const makhzan = useQueryClient();

  const katalog = useQuery<readonly RuqaaKharijiya[], KhataJisr>({
    queryKey: mafatih.kharijiya(muarrif),
    queryFn: () => nadiKhariji('ruqaa_kharijiya', { muarrif }, qarrirKatalog),
  });

  /* -------------------------------------------------------------------------
     The question, and the answer to it.

     `sual` is the entry whose warnings are open, so at most one is asked at a
     time, and the tick starts false on every opening: a tick given about one
     team's patch is not a tick about the next one's.
     ----------------------------------------------------------------------- */
  const [sual, setSual] = useState<string | null>(null);
  const [muqirr, setMuqirr] = useState(false);

  const azrarTathbeet = useRef(new Map<string, HTMLButtonElement | null>());
  const zirIlghaRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    setSual(null);
    setMuqirr(false);
  }, [muarrif]);

  // The safe control, as every other panel that can write into a game directory
  // does it: opening the panel must not land focus on the button that writes.
  useEffect(() => {
    if (sual !== null) {
      zirIlghaRef.current?.focus();
    }
  }, [sual]);

  const alaIlgha = (): void => {
    if (sual !== null) {
      azrarTathbeet.current.get(sual)?.focus();
    }
    setSual(null);
  };

  /**
   * Taking one of these back off.
   *
   * Its own call rather than the removal strip further down the screen: that
   * strip drives Taarib's own installs at the game's backup root, and a
   * third-party entry keeps its manifest and its originals under its own
   * lineage. Pointing the strip at this would remove the wrong thing, and
   * pointing this at the strip would remove nothing.
   */
  const izala = useMutation<TaqreerIzalaHie, KhataJisr, { readonly ruqaa: string }>({
    mutationFn: ({ ruqaa }) => nadi('azil_kharijiya', { muarrif, ruqaa }),
    onError: (khata) => {
      ansha({
        naw: 'khatar',
        nass: t('khariji.khata_izala', lugha),
        tafsil: khata.nass(lugha) ?? khata.message,
        ramz: khata.khata?.ramz ?? null,
      });
    },
    onSuccess: (taqreer, { ruqaa }) => {
      ansha({
        naw: taqreer.nazif ? 'najah' : 'tanbeeh',
        nass: t(taqreer.nazif ? 'khariji.tanbih.uzilat' : 'khariji.tanbih.uzilat_juzii', lugha, {
          adad: munassiq.raqm(taqreer.mustaada),
        }),
        // Named rather than counted: a file the store replaced since the
        // install keeps the store's copy, and the reader is owed which.
        tafsil:
          taqreer.mustabdala.length > 0 ? taqreer.mustabdala.join('\n') : null,
      });
      void makhzan.invalidateQueries({ queryKey: mafatih.kharijiya(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.tadakhul(muarrif, ruqaa) });
      // The loader slot is free again and the directory has changed, so every
      // answer this screen holds about it is stale.
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.khutta(muarrif) });
      void makhzan.invalidateQueries({ queryKey: ['khuttat_izala', muarrif] });
    },
  });

  const tathbeet = useMutation<NatijatKharijiya, KhataJisr, { readonly ruqaa: string }>({
    mutationFn: ({ ruqaa }) =>
      // The acknowledgement is always `true` here, and that is not a constant
      // standing in for a decision: this call is only reachable from the button
      // inside the panel below, which is refused until the tick is given. The
      // backend is told what the person actually said, and the person said it
      // on a screen that was showing the maker's own warnings.
      nadiKhariji('thabbit_kharijiya', { muarrif, ruqaa, iqrar: true }, qarrirNatija),
    onSuccess: (natija, { ruqaa }) => {
      const madkhal = katalog.data?.find((wahid) => wahid.id === ruqaa);
      // Credit at the moment it lands, too: the notice is the one line that
      // reaches a reader who has already scrolled away from the card.
      const itiman =
        madkhal === undefined
          ? null
          : nassItiman(madkhal.masdar.ism, madkhal.fariq, lugha);
      ansha(
        natija.basmat_mutabiqa
          ? {
              naw: 'najah',
              nass: t('khariji.tanbih.tamma', lugha, {
                adad: munassiq.raqm(natija.adad_maktub),
              }),
              tafsil: itiman,
            }
          : { naw: 'tanbeeh', nass: t('khariji.tanbih.basma', lugha), tafsil: itiman },
      );
      void makhzan.invalidateQueries({ queryKey: mafatih.kharijiya(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.tadakhul(muarrif, ruqaa) });
      // The game's loader slot is now taken and its files are now there, so
      // every answer this screen holds about the directory is stale.
      void makhzan.invalidateQueries({ queryKey: mafatih.tafasil(muarrif) });
      void makhzan.invalidateQueries({ queryKey: mafatih.khutta(muarrif) });
      void makhzan.invalidateQueries({ queryKey: ['khuttat_izala', muarrif] });
    },
  });

  const fath = useMutation<boolean, KhataJisr, string>({
    mutationFn: (rabt) => nadi('iftah_rabt', { rabt }),
    // A browser that did not open leaves no other trace on the screen, so the
    // refusal is said where the reader is, with the backend's own sentence.
    onError: (khata) => {
      ansha({
        naw: 'khatar',
        nass: t('khariji.khata_fath', lugha),
        tafsil: khata.nass(lugha) ?? khata.message,
        ramz: khata.khata?.ramz ?? null,
      });
    },
  });

  const alaTalab = (ruqaa: string): void => {
    if (tathbeet.isPending || muqfal) {
      return;
    }
    // A second press on an open row closes it, which is what `aria-expanded`
    // on that button promises; reopening would silently clear the tick.
    if (sual === ruqaa) {
      alaIlgha();
      return;
    }
    setMuqirr(false);
    setSual(ruqaa);
  };

  const alaTanfidh = (ruqaa: string): void => {
    if (tathbeet.isPending || muqfal || !muqirr) {
      return;
    }
    tathbeet.mutate({ ruqaa });
    setSual(null);
  };

  // This build of the shell has no third-party support at all. Not a failure
  // and not drawn as one; see the note at the top of this file.
  if (katalog.error !== null && ghayrMusajjal(katalog.error)) {
    return null;
  }

  const wajh = katalog.isPending ? 'tahmil' : katalog.error !== null ? 'khata' : 'jahiz';

  return (
    <section className="luba__qism qism-khariji" aria-labelledby="khariji-unwan">
      <h2 id="khariji-unwan" className="luba__unwan-qism">
        {t('khariji.unwan', lugha)}
      </h2>
      <p className="qism-khariji__sharh">{t('khariji.sharh', lugha)}</p>

      <Mashhad miftah={wajh}>
        {katalog.isPending ? (
          <Haykal nass={t('khariji.jari', lugha)} />
        ) : katalog.error !== null ? (
          <KutlatKhata
            unwan={t('khariji.taadhur', lugha)}
            khata={katalog.error}
            lugha={lugha}
            muarrif={muarrif}
            aada={() => {
              void katalog.refetch();
            }}
          />
        ) : katalog.data === undefined || katalog.data.length === 0 ? (
          <HalatFarigha
            unwan={t('khariji.la_shay_unwan', lugha)}
            nass={t('khariji.la_shay', lugha)}
          />
        ) : (
          <ul className="qism-khariji__qaima">
            {katalog.data.map((madkhal) => (
              <MadkhalKhariji
                key={madkhal.id}
                muarrif={muarrif}
                madkhal={madkhal}
                lugha={lugha}
                munassiq={munassiq}
                // A refused game is offered nothing to install, exactly as the
                // registry's own listing is. The entry is still listed, still
                // credited and still explained; only the verb is withheld.
                muqfal={muqfal || mahmiya}
                sababQafl={mahmiya ? t('khariji.mahmiya', lugha) : sababQafl}
                yuthabbat={tathbeet.isPending && tathbeet.variables?.ruqaa === madkhal.id}
                mashghul={tathbeet.isPending}
                maftuh={sual === madkhal.id}
                muqirr={muqirr}
                alaIqrar={setMuqirr}
                alaTalab={() => {
                  alaTalab(madkhal.id);
                }}
                alaIlgha={alaIlgha}
                alaTanfidh={() => {
                  alaTanfidh(madkhal.id);
                }}
                sajjilZir={(uqda) => {
                  azrarTathbeet.current.set(madkhal.id, uqda);
                }}
                marjaIlgha={(uqda) => {
                  zirIlghaRef.current = uqda;
                }}
                alaIzala={alaIzala}
                alaIzalatNafsiha={() => {
                  izala.mutate({ ruqaa: madkhal.id });
                }}
                tuzal={izala.isPending && izala.variables?.ruqaa === madkhal.id}
                yaftahRabt={fath.isPending && fath.variables === madkhal.masdar.rabt}
                iftahRabt={(rabt) => {
                  if (!fath.isPending) {
                    fath.mutate(rabt);
                  }
                }}
              />
            ))}
          </ul>
        )}
      </Mashhad>

      <div className="qism-khariji__mintaqa" aria-live="polite">
        {tathbeet.isPending ? (
          <p className="qism-khariji__jari">{t('khariji.jari_tathbeet', lugha)}</p>
        ) : null}
        {tathbeet.error !== null ? (
          <KutlatKhata
            unwan={t('khariji.khata_tathbeet', lugha)}
            khata={tathbeet.error}
            lugha={lugha}
            muarrif={muarrif}
          >
            {/* Not a retry. A refused install is usually a refusal with a
                reason — a collision, a permission, a hash that did not match —
                and re-firing the same call cannot change any of those. This
                reopens the question on the entry that was pressed, with the
                backend's own sentence still above it. */}
            {tathbeet.variables === undefined ? null : (
              <button
                type="button"
                className="zir"
                onClick={() => {
                  const akhir = tathbeet.variables;
                  if (akhir !== undefined && !tathbeet.isPending && !muqfal) {
                    setMuqirr(false);
                    setSual(akhir.ruqaa);
                  }
                }}
              >
                {t('khariji.muraja', lugha)}
              </button>
            )}
          </KutlatKhata>
        ) : null}
        <Zuhur maftuh={tathbeet.data !== undefined} className="qism-khariji__natija">
          {tathbeet.data === undefined ? null : (
            <>
              <p className="qism-khariji__natija-nass">
                <span
                  className={
                    tathbeet.data.basmat_mutabiqa
                      ? 'luba__nuqta luba__nuqta--najah'
                      : 'luba__nuqta luba__nuqta--khatar'
                  }
                  aria-hidden="true"
                />
                {t(
                  tathbeet.data.basmat_mutabiqa
                    ? 'khariji.natija.basmat'
                    : 'khariji.natija.basmat_la',
                  lugha,
                )}
              </p>
              <ul className="qism-khariji__adad">
                <li>
                  {t('khariji.natija.maktub', lugha, {
                    adad: munassiq.raqm(tathbeet.data.adad_maktub),
                  })}
                </li>
                <li>
                  {t('khariji.natija.muhtafaz', lugha, {
                    adad: munassiq.raqm(tathbeet.data.adad_muhtafaz),
                  })}
                </li>
              </ul>
              <p className="qism-khariji__nass-hadi">
                {t('khariji.natija.sijill', lugha)}
                <span className="mono-ltr qism-khariji__rabt" dir="ltr">
                  {tathbeet.data.sijill}
                </span>
              </p>
            </>
          )}
        </Zuhur>
      </div>
    </section>
  );
}
