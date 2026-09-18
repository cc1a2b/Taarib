// فحص الصفحة — which atlas page a string is drawn from, and what happens when
// there is no single answer.
//
// A page is a texture and a texture is a draw call, so one mesh samples one
// page. Nasij takes the page to build for and skips every glyph on any other
// one, reporting the pages it saw in NatijaNasij.AlamSafahat. The adapters used
// to ask for page zero unconditionally and ignore that report, which meant:
//
//   - a string whose glyphs all landed on a page opened after page zero filled
//     produced no geometry at all, and the label was simply not there;
//   - a string straddling two pages produced the part on page zero, which in
//     cursive Arabic is fragments of words on the line;
//
// and neither was counted anywhere. Measured over the installed patch of The
// Stalked 3 with the plugin's own defaults — 2048-square pages, a 64 MiB budget
// — the 192-pixel rung puts 3,478 of 169,142 glyph lookups on page one, across
// 1,661 strings that straddle two pages and one string entirely on page one. On
// 1024-square pages the same rung puts 70,974 of 169,142 there, 42 per cent.
//
// These cases are the rule that replaced it, over a glyph map that exists only
// here: JadwalSafahat.SafhaWahida picks the page when there is one, and refuses
// when there is not, and Nasij is driven both ways to show what each answer is
// worth in geometry.

using System;
using System.Collections.Generic;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Fahs
{
    /// <summary>The cases over atlas page selection.</summary>
    public static class FahsSafha
    {
        /// <summary>Adds every case to the run.</summary>
        /// <param name="halat">The run.</param>
        /// <exception cref="ArgumentNullException"><paramref name="halat"/> is null.</exception>
        public static void Sajjil(List<Halat> halat)
        {
            if (halat is null)
            {
                throw new ArgumentNullException(nameof(halat));
            }

            halat.Add(new Halat("safha.wahida-sifr", WahidaSifr));
            halat.Add(new Halat("safha.wahida-ukhra", WahidaUkhra));
            halat.Add(new Halat("safha.mushattata-turfad", MushattataTurfad));
            halat.Add(new Halat("safha.baida-turfad", BaidaTurfad));
            halat.Add(new Halat("safha.bila-ashkal-tanjah", BilaAshkalTanjah));
            halat.Add(new Halat("safha.safha-ukhra-kanat-farigha", SafhaUkhraKanatFarigha));
            halat.Add(new Halat("safha.tashattut-yaksib-juzan-faqat", TashattutYaksibJuzanFaqat));
        }

        /// <summary>A layout entirely on page zero elects page zero.</summary>
        private static string? WahidaSifr()
        {
            if (!JadwalSafahat.SafhaWahida(1UL, false, out ushort safha))
            {
                return "a layout entirely on page zero was refused";
            }
            return safha == 0 ? null : $"expected page 0, got {safha}";
        }

        /// <summary>
        /// A layout entirely on one other page elects that page. This is the
        /// case that used to draw nothing whatsoever.
        /// </summary>
        private static string? WahidaUkhra()
        {
            ulong[] alamat = { 1UL << 1, 1UL << 7, 1UL << 63 };
            ushort[] mutawaqqa = { 1, 7, 63 };
            for (int i = 0; i < alamat.Length; i++)
            {
                if (!JadwalSafahat.SafhaWahida(alamat[i], false, out ushort safha))
                {
                    return $"a layout entirely on page {mutawaqqa[i]} was refused";
                }
                if (safha != mutawaqqa[i])
                {
                    return $"expected page {mutawaqqa[i]}, got {safha}";
                }
            }
            return null;
        }

        /// <summary>Two pages or more has no single answer and must be refused.</summary>
        private static string? MushattataTurfad()
        {
            ulong[] alamat = { 0b11UL, 0b101UL, (1UL << 0) | (1UL << 63), ulong.MaxValue };
            for (int i = 0; i < alamat.Length; i++)
            {
                if (JadwalSafahat.SafhaWahida(alamat[i], false, out ushort safha))
                {
                    return $"mask {alamat[i]:x} was accepted as page {safha}";
                }
            }
            return null;
        }

        /// <summary>
        /// A glyph on a page the mask cannot name is refused even when the mask
        /// itself looks like one clean page. The mask stops at sixty-four and a
        /// page past it is a page nothing can report.
        /// </summary>
        private static string? BaidaTurfad()
        {
            if (JadwalSafahat.SafhaWahida(1UL, true, out _))
            {
                return "a layout with a glyph past page 63 was accepted";
            }
            if (JadwalSafahat.SafhaWahida(0UL, true, out _))
            {
                return "a layout whose only glyphs are past page 63 was accepted";
            }
            return null;
        }

        /// <summary>
        /// A layout that drew no glyph at all — a line of spaces — succeeds with
        /// page zero. There is nothing to pick a page for, and refusing would
        /// hand a blank label back to the engine for no reason.
        /// </summary>
        private static string? BilaAshkalTanjah()
        {
            if (!JadwalSafahat.SafhaWahida(0UL, false, out ushort safha))
            {
                return "a layout with no drawable glyph was refused";
            }
            return safha == 0 ? null : $"expected page 0, got {safha}";
        }

        /// <summary>
        /// The defect, and the fix, in geometry. Every glyph of the string is on
        /// page one: built for page zero it produces nothing at all, and built
        /// for the page the election names it produces every quad.
        /// </summary>
        private static string? SafhaUkhraKanatFarigha()
        {
            KhareetaMuzayyafa khareeta = KhareetaMuzayyafa.KullShayAla(1, 12);
            Makhzan makhzan = new Makhzan(12);

            NatijaNasij sifr = Ibni(khareeta, makhzan, 0);
            if (sifr.AdadAshkal != 0)
            {
                return $"building for page 0 drew {sifr.AdadAshkal} glyph(s) that live on page 1";
            }
            if (sifr.AlamSafahat != 1UL << 1)
            {
                return $"the page mask was {sifr.AlamSafahat:x}, expected only page 1";
            }

            if (!JadwalSafahat.SafhaWahida(sifr.AlamSafahat, sifr.SafahatBaida, out ushort safha))
            {
                return "the string is entirely on page 1 and the election refused it";
            }
            NatijaNasij ukhra = Ibni(khareeta, makhzan, safha);
            if (ukhra.AdadAshkal != 12)
            {
                return $"building for page {safha} drew {ukhra.AdadAshkal} of 12 glyph(s)";
            }
            if (ukhra.AdadRuus != 12 * Nasij.RuusLiShakl)
            {
                return $"12 glyphs produced {ukhra.AdadRuus} vertices";
            }
            return null;
        }

        /// <summary>
        /// A string straddling two pages: whichever page is built for, some of
        /// its letters are not drawn. That is the whole reason the string is
        /// left to the engine instead.
        /// </summary>
        private static string? TashattutYaksibJuzanFaqat()
        {
            KhareetaMuzayyafa khareeta = KhareetaMuzayyafa.Munawaba(12);
            Makhzan makhzan = new Makhzan(12);

            NatijaNasij sifr = Ibni(khareeta, makhzan, 0);
            NatijaNasij wahid = Ibni(khareeta, makhzan, 1);
            if (sifr.AdadAshkal + wahid.AdadAshkal != 12)
            {
                return $"the two pages together drew {sifr.AdadAshkal + wahid.AdadAshkal} of 12";
            }
            if (sifr.AdadAshkal == 12 || wahid.AdadAshkal == 12)
            {
                return "one page drew the whole string, so this is not a straddling string";
            }
            if (JadwalSafahat.SafhaWahida(sifr.AlamSafahat, sifr.SafahatBaida, out ushort safha))
            {
                return $"a string on two pages elected page {safha} instead of being refused";
            }
            return null;
        }

        /// <summary>Builds one string's geometry for one page.</summary>
        private static NatijaNasij Ibni(KhareetaMuzayyafa khareeta, Makhzan makhzan, ushort safha)
        {
            TaaribHarf[] huruf = khareeta.Huruf();
            TalabNasij talab = default;
            talab.Huruf = huruf;
            talab.Hayyiz = Hayyiz();
            talab.Lawn = LawnRasm.Min(0xFFFFFFFFu);
            talab.Hajm = 16f;
            talab.HajmLawha = 16f;
            talab.Safha = safha;
            return Nasij.Ibni(in talab, khareeta, makhzan.Makhzani());
        }

        /// <summary>A canvas-sized destination rectangle, one unit per pixel.</summary>
        private static HayyizRasm Hayyiz()
        {
            HayyizRasm hayyiz = default;
            hayyiz.Ard = 400f;
            hayyiz.Irtifa = 40f;
            hayyiz.MihwarS = 0f;
            hayyiz.MihwarA = 1f;
            hayyiz.Qiyas = 1f;
            hayyiz.IrtifaTakhtit = 20f;
            hayyiz.Muhadhaha = MuhadhahaRasiya.Ala;
            return hayyiz;
        }

        /// <summary>
        /// Destination arrays sized once, the way a real adapter reuses the
        /// buffers a mesh container already owns.
        /// </summary>
        private sealed class Makhzan
        {
            private readonly NuqtaRasm[] ruus;
            private readonly NuqtaMulmas[] malamis;
            private readonly LawnRasm[] alwan;
            private readonly int[] muthallathat;

            public Makhzan(int ashkal)
            {
                ruus = new NuqtaRasm[ashkal * Nasij.RuusLiShakl];
                malamis = new NuqtaMulmas[ashkal * Nasij.RuusLiShakl];
                alwan = new LawnRasm[ashkal * Nasij.RuusLiShakl];
                muthallathat = new int[ashkal * Nasij.FahrasLiShakl];
            }

            public MakhzanNasij Makhzani()
            {
                MakhzanNasij makhzan = default;
                makhzan.Ruus = ruus;
                makhzan.Malamis = malamis;
                makhzan.Alwan = alwan;
                makhzan.Muthallathat = muthallathat;
                return makhzan;
            }
        }

        /// <summary>
        /// A glyph map with no atlas behind it: every glyph is the same
        /// sixteen-by-sixteen rectangle, and only the page differs.
        /// </summary>
        /// <remarks>
        /// A struct rather than a class for the same reason
        /// <c>KhareetaMuakkada</c> is one: <see cref="Nasij.Ibni"/> is generic
        /// over the map with no class constraint, so a value type is dispatched
        /// statically and never boxed.
        /// </remarks>
        private readonly struct KhareetaMuzayyafa : IKhareetatAshkal
        {
            private readonly ushort safhaThabita;
            private readonly bool munawaba;
            private readonly int adad;

            private KhareetaMuzayyafa(ushort safhaThabita, bool munawaba, int adad)
            {
                this.safhaThabita = safhaThabita;
                this.munawaba = munawaba;
                this.adad = adad;
            }

            /// <summary>Every glyph on one page.</summary>
            public static KhareetaMuzayyafa KullShayAla(ushort safha, int adad)
            {
                return new KhareetaMuzayyafa(safha, false, adad);
            }

            /// <summary>Alternating pages, which is what a straddling string is.</summary>
            public static KhareetaMuzayyafa Munawaba(int adad)
            {
                return new KhareetaMuzayyafa(0, true, adad);
            }

            /// <summary>The glyphs of one line, laid out left to right.</summary>
            public TaaribHarf[] Huruf()
            {
                TaaribHarf[] huruf = new TaaribHarf[adad];
                for (int i = 0; i < adad; i++)
                {
                    huruf[i].Muarrif = (uint)(i + 1);
                    huruf[i].Anqud = (uint)i;
                    huruf[i].S = i * 18f;
                    huruf[i].A = 16f;
                    huruf[i].Taqaddum = 18f;
                }
                return huruf;
            }

            /// <inheritdoc/>
            public bool Shakl(in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
            {
                mawdi = default;
                if (miftah.Muarrif == 0 || miftah.Muarrif > (uint)adad)
                {
                    return false;
                }
                mawdi.S = 8;
                mawdi.A = 8;
                mawdi.Ard = 16;
                mawdi.Irtifa = 16;
                mawdi.IzahaS = 0;
                mawdi.IzahaA = 16;
                mawdi.Taqaddum = 18f;
                mawdi.Safha = munawaba ? (ushort)(miftah.Muarrif % 2) : safhaThabita;
                return true;
            }

            /// <inheritdoc/>
            public bool Safha(ushort fahras, out QiyasSafha qiyas)
            {
                qiyas = default;
                if (fahras > 1)
                {
                    return false;
                }
                qiyas.Ard = 256;
                qiyas.Irtifa = 256;
                return true;
            }
        }
    }
}
