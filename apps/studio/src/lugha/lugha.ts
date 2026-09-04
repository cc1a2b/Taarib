import type { Lugha, NizamArqam } from '@/mustalahat/awamir';

import ar from './ar.json';
import en from './en.json';

/**
 * اللغة — the string set and the locale-aware formatters.
 *
 * Arabic is the source language, not a translation target: `ar.json` defines
 * which keys exist, and `en.json` is checked against it below. A key present
 * in Arabic and missing in English is a type error at build time, which is the
 * only way to keep a promise like "no English leaks into an Arabic session"
 * across a product this size.
 *
 * Nothing here holds the current language. It is passed in at every call site,
 * because the language lives in the settings the backend owns, and a second
 * copy of it in a module-level variable is a second source of truth that will
 * eventually disagree with the first.
 */

/** Every key the interface may ask for, derived from the Arabic string set. */
export type MiftahLugha = keyof typeof ar;

/**
 * Both string sets, keyed by language.
 *
 * The annotation is the check: `en` must supply a string for every key `ar`
 * defines, or this assignment fails to compile.
 */
const NUSUS: Readonly<Record<Lugha, Readonly<Record<MiftahLugha, string>>>> = {
  arabi: ar,
  injilizi: en,
};

/** `{ism}` in a string, which `t` replaces with a caller-supplied value. */
const NAMAT_MUTAGHAYYIR = /\{(\w+)\}/g;

/** The BCP 47 tag for a language, for `lang` attributes and `Intl`. */
export function wasm(lugha: Lugha): 'ar' | 'en' {
  return lugha === 'arabi' ? 'ar' : 'en';
}

/** The base direction of the interface in a language. */
export function ittijah(lugha: Lugha): 'rtl' | 'ltr' {
  return lugha === 'arabi' ? 'rtl' : 'ltr';
}

/**
 * The string for a key, in a language.
 *
 * Numbers passed as substitutions are interpolated as they are given. Anything
 * a reader will understand as a quantity should be formatted through
 * {@link munassiqat} first, so it carries the digit system the user chose.
 *
 * @param miftah a key that exists in the Arabic string set
 * @param lugha the language of the current session
 * @param mutaghayyirat values for the `{...}` placeholders in the string
 */
export function t(
  miftah: MiftahLugha,
  lugha: Lugha,
  mutaghayyirat?: Readonly<Record<string, string | number>>,
): string {
  const nass = NUSUS[lugha][miftah];
  if (mutaghayyirat === undefined) {
    return nass;
  }
  return nass.replace(NAMAT_MUTAGHAYYIR, (kamil: string, ism: string) => {
    const qeema = mutaghayyirat[ism];
    return qeema === undefined ? kamil : String(qeema);
  });
}

/** A base key whose six plural variants all exist in the Arabic string set. */
type AsasAkhar<M> = M extends `${infer B}.akhar` ? B : never;
type AiladatJam<B extends string> =
  | `${B}.sifr`
  | `${B}.wahid`
  | `${B}.ithnan`
  | `${B}.qalil`
  | `${B}.kathir`;

/**
 * Every plural family: a base key is one only when all six of its variants —
 * sifr, wahid, ithnan, qalil, kathir, akhar — exist in the string set, so a
 * family missing a form is a type error at the call site, not a runtime key.
 */
export type MiftahJam = {
  [B in AsasAkhar<MiftahLugha>]: AiladatJam<B> extends MiftahLugha ? B : never;
}[AsasAkhar<MiftahLugha>];

/** The CLDR category each variant suffix answers for. */
const FIAT_JAM: Readonly<Record<Intl.LDMLPluralRule, string>> = {
  zero: 'sifr',
  one: 'wahid',
  two: 'ithnan',
  few: 'qalil',
  many: 'kathir',
  other: 'akhar',
};

const QAWAID_JAM: Readonly<Record<Lugha, Intl.PluralRules>> = {
  arabi: new Intl.PluralRules('ar'),
  injilizi: new Intl.PluralRules('en'),
};

/**
 * The string for a counted thing, in the grammatical form the count demands.
 *
 * Arabic inflects the counted noun through six forms — zero, one, two, a few,
 * many, and everything else — and a sentence that bolts a digit onto one fixed
 * form is wrong five times out of six. The count is interpolated as `{adad}`,
 * already formatted with the user's digit system.
 *
 * @param asas a base key whose whole six-variant family exists
 * @param lugha the language of the current session
 * @param adad the raw count the grammar is chosen for
 * @param munassiq the session's formatters, for the digits
 * @param mutaghayyirat values for any further `{...}` placeholders
 */
export function jam(
  asas: MiftahJam,
  lugha: Lugha,
  adad: number,
  munassiq: Munassiqat,
  mutaghayyirat?: Readonly<Record<string, string | number>>,
): string {
  const fia = FIAT_JAM[QAWAID_JAM[lugha].select(adad)];
  // Safe by the definition of MiftahJam: every family carries all six forms.
  const miftah = `${asas}.${fia}` as MiftahLugha;
  return t(miftah, lugha, { ...mutaghayyirat, adad: munassiq.raqm(adad) });
}

/** The Unicode numbering system each digit setting selects. */
const ANZIMAT_ARQAM: Readonly<Record<NizamArqam, string>> = {
  latini: 'latn',
  arabi: 'arab',
  farisi: 'arabext',
};

/**
 * Binary size units, smallest first. Binary rather than decimal because every
 * figure Taarib reports next to one — a patch, a backup, a font atlas, a log
 * directory — is measured against what the filesystem reports, not against a
 * drive manufacturer's label.
 */
const WAHADAT_HAJM = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const;

/** A byte unit identifier accepted by `Intl.NumberFormat`. */
type WahdatHajm = (typeof WAHADAT_HAJM)[number];

/** The locale-aware formatters for one session. */
export interface Munassiqat {
  /** A count, with the user's digits and no fractional part. */
  raqm(qeema: number): string;
  /** A fraction from 0 to 1, rendered as a percentage with the user's digits. */
  nisba(kasr: number): string;
  /** A byte count, scaled to the largest unit that leaves a readable number. */
  hajm(bayt: number): string;
}

/**
 * Formatter construction is not free, and these are called once per rendered
 * row, so one set is built per locale and reused for the life of the session.
 */
const MAKHZAN_MUNASSIQAT = new Map<string, Munassiqat>();

/**
 * The formatters for a language and digit system.
 *
 * @param lugha the language of the current session
 * @param arqam the digit system the user chose
 */
export function munassiqat(lugha: Lugha, arqam: NizamArqam): Munassiqat {
  const mahalli = `${wasm(lugha)}-u-nu-${ANZIMAT_ARQAM[arqam]}`;
  const mawjud = MAKHZAN_MUNASSIQAT.get(mahalli);
  if (mawjud !== undefined) {
    return mawjud;
  }

  const adad = new Intl.NumberFormat(mahalli, { maximumFractionDigits: 0 });
  const miawi = new Intl.NumberFormat(mahalli, {
    style: 'percent',
    maximumFractionDigits: 0,
  });

  const bilWahda = new Map<WahdatHajm, Intl.NumberFormat>();
  for (const [martaba, wahda] of WAHADAT_HAJM.entries()) {
    bilWahda.set(
      wahda,
      new Intl.NumberFormat(mahalli, {
        style: 'unit',
        unit: wahda,
        unitDisplay: 'short',
        // Bytes are whole things; everything above them reads better with one
        // decimal than with six digits of false precision.
        maximumFractionDigits: martaba === 0 ? 0 : 1,
      }),
    );
  }

  const jadeed: Munassiqat = {
    raqm: (qeema) => adad.format(qeema),
    nisba: (kasr) => miawi.format(kasr),
    hajm: (bayt) => {
      let qeema = Number.isFinite(bayt) ? Math.max(0, bayt) : 0;
      let martaba = 0;
      while (qeema >= 1024 && martaba < WAHADAT_HAJM.length - 1) {
        qeema /= 1024;
        martaba += 1;
      }
      const wahda = WAHADAT_HAJM[martaba] ?? 'byte';
      const munassiq = bilWahda.get(wahda);
      return munassiq === undefined ? adad.format(qeema) : munassiq.format(qeema);
    },
  };

  MAKHZAN_MUNASSIQAT.set(mahalli, jadeed);
  return jadeed;
}
