// فحص اللون — the composition that decides what a run of glyphs is tinted with.
//
// Every case here is arithmetic over values a TextMeshPro component could
// really hold, and none of it needs a game, a display or an engine: LawnNass
// takes four numbers per source and answers four numbers, which is exactly the
// part of this defect that can be settled without the title in front of you.
//
// What these cases are actually protecting:
//
//   - that a build of TextMeshPro exposing none of the extra colour sources
//     draws precisely what this product drew before they were read, so a game
//     that is correct today cannot be made wrong by this file existing;
//   - that a heading whose colour is carried by a gradient rather than by
//     `color` comes out the gradient's colour, which is the reported defect;
//   - that the three sources multiply in TextMeshPro's own order, with its own
//     clamp between the vertex colour and the face colour, because getting that
//     wrong tints every string in the game slightly wrongly instead of one
//     heading obviously wrongly.

using System;
using System.Collections.Generic;
using System.Globalization;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Fahs
{
    /// <summary>The cases over <see cref="LawnNass"/>.</summary>
    public static class FahsLawn
    {
        /// <summary>How far two channels may differ and still be the same colour.</summary>
        private const float Tasamuh = 1e-6f;

        /// <summary>Adds every case to the run.</summary>
        /// <param name="halat">The run.</param>
        /// <exception cref="ArgumentNullException"><paramref name="halat"/> is null.</exception>
        public static void Sajjil(List<Halat> halat)
        {
            if (halat is null)
            {
                throw new ArgumentNullException(nameof(halat));
            }

            halat.Add(new Halat("lawn.bila-masadir", BilaMasadir));
            halat.Add(new Halat("lawn.tadarruj-ghair-mufaal", TadarrujGhairMufaal));
            halat.Add(new Halat("lawn.unwan-bahit", UnwanBahit));
            halat.Add(new Halat("lawn.mutawassit-arkan", MutawassitArkan));
            halat.Add(new Halat("lawn.qalib-yaghlib-dakhili", QalibYaghlibDakhili));
            halat.Add(new Halat("lawn.dakhili-bila-qalib", DakhiliBilaQalib));
            halat.Add(new Halat("lawn.wajh-yadrib", WajhYadrib));
            halat.Add(new Halat("lawn.wajh-sifr-yuhmal", WajhSifrYuhmal));
            halat.Add(new Halat("lawn.thalathat-masadir", ThalathatMasadir));
            halat.Add(new Halat("lawn.nitaq-yusqit-tadarruj", NitaqYusqitTadarruj));
            halat.Add(new Halat("lawn.tajahul-yuid-tadarruj", TajahulYuidTadarruj));
            halat.Add(new Halat("lawn.hasr-qabla-wajh", HasrQablaWajh));
            halat.Add(new Halat("lawn.masdar-ghair-salim", MasdarGhairSalim));
            halat.Add(new Halat("lawn.shaffafiya-tadrib", ShaffafiyaTadrib));
            halat.Add(new Halat("lawn.nitaqat-alwan", NitaqatAlwan));
        }

        /// <summary>
        /// Nothing but the component's own colour resolved, which is every build
        /// of TextMeshPro this product ever ran against before the other sources
        /// were read. The answer must be the colour itself, to the bit.
        /// </summary>
        private static string? BilaMasadir()
        {
            LawnKasri[] alwan =
            {
                LawnKasri.Min(1f, 1f, 1f, 1f),
                LawnKasri.Min(0f, 0f, 0f, 1f),
                LawnKasri.Min(0.16078432f, 0.2f, 0.26666668f, 1f),
                LawnKasri.Min(0.8f, 0.1f, 0.3f, 0.5f),
                LawnKasri.Min(0f, 0f, 0f, 0f),
            };
            for (int i = 0; i < alwan.Length; i++)
            {
                LawnKasri natija = LawnNass.Damj(TarkeebLawnTmp.Min(alwan[i]));
                string? khata = Yutabiq("colour " + i, alwan[i], natija);
                if (khata is not null)
                {
                    return khata;
                }
            }
            return null;
        }

        /// <summary>
        /// A gradient is serialized on the component but its switch is off. This
        /// is the state most components in a game are in, and reading the corners
        /// without reading the switch would tint all of them.
        /// </summary>
        private static string? TadarrujGhairMufaal()
        {
            LawnKasri asasi = LawnKasri.Min(0.9f, 0.9f, 0.9f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(asasi);
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(LawnKasri.Min(0.1f, 0.1f, 0.1f, 1f));
            return Yutabiq("unchanged", asasi, LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// The reported defect, as numbers. A world-space monitor's heading whose
        /// `color` is the inspector's default white and whose real colour is a
        /// dark slate gradient: read `color` alone and it draws white on a
        /// near-white panel, which is exactly "the header not clear".
        /// </summary>
        private static string? UnwanBahit()
        {
            LawnKasri sabbura = LawnKasri.Min(0.16078432f, 0.2f, 0.26666668f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(sabbura);
            return Yutabiq("slate", sabbura, LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// Four different corners collapse to their mean — the colour the
        /// hardware would have interpolated at the centre of the quad — and not
        /// to any one of them.
        /// </summary>
        private static string? MutawassitArkan()
        {
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = ArkanTadarruj.Min(
                LawnKasri.Min(0.2f, 0.4f, 0.6f, 1f),
                LawnKasri.Min(0.4f, 0.4f, 0.6f, 1f),
                LawnKasri.Min(0.6f, 0.4f, 0.6f, 0.5f),
                LawnKasri.Min(0.8f, 0.4f, 0.6f, 0.5f));
            return Yutabiq(
                "mean",
                LawnKasri.Min(0.5f, 0.4f, 0.6f, 0.75f),
                LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// An assigned preset wins over the corners the component serializes
        /// itself, which is TextMeshPro's own precedence.
        /// </summary>
        private static string? QalibYaghlibDakhili()
        {
            LawnKasri qalib = LawnKasri.Min(0.25f, 0.5f, 0.75f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuQalib = true;
            tarkeeb.Qalib = Mutasawi(qalib);
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(LawnKasri.Min(1f, 0f, 0f, 1f));
            return Yutabiq("preset", qalib, LawnNass.Damj(tarkeeb));
        }

        /// <summary>And the inline corners are used when no preset is assigned.</summary>
        private static string? DakhiliBilaQalib()
        {
            LawnKasri dakhili = LawnKasri.Min(0.25f, 0.5f, 0.75f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(dakhili);
            return Yutabiq("inline", dakhili, LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// The material's face colour multiplies the vertex colour. Taarib's own
        /// material has no such property, so a face colour left out here is a
        /// face colour that never reaches the screen.
        /// </summary>
        private static string? WajhYadrib()
        {
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.LahuWajh = true;
            tarkeeb.Wajh = LawnKasri.Min(0.5f, 0.25f, 0.125f, 1f);
            return Yutabiq(
                "face", LawnKasri.Min(0.5f, 0.25f, 0.125f, 1f), LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// A face colour of four zeros is what a material whose shader has no
        /// <c>_FaceColor</c> answers with, and it must be ignored rather than
        /// multiplied in — multiplying it would turn every string in such a game
        /// invisible. A face colour that is transparent but not black is a real
        /// value and still multiplies.
        /// </summary>
        private static string? WajhSifrYuhmal()
        {
            LawnKasri asasi = LawnKasri.Min(0.2f, 0.4f, 0.6f, 1f);

            TarkeebLawnTmp bilSifr = TarkeebLawnTmp.Min(asasi);
            bilSifr.LahuWajh = true;
            bilSifr.Wajh = LawnKasri.Min(0f, 0f, 0f, 0f);
            string? khata = Yutabiq("zero face", asasi, LawnNass.Damj(bilSifr));
            if (khata is not null)
            {
                return khata;
            }

            TarkeebLawnTmp bilShaffaf = TarkeebLawnTmp.Min(asasi);
            bilShaffaf.LahuWajh = true;
            bilShaffaf.Wajh = LawnKasri.Min(1f, 1f, 1f, 0f);
            return Yutabiq(
                "transparent face",
                LawnKasri.Min(0.2f, 0.4f, 0.6f, 0f),
                LawnNass.Damj(bilShaffaf));
        }

        /// <summary>All three sources at once, in TextMeshPro's order.</summary>
        private static string? ThalathatMasadir()
        {
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(0.5f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(LawnKasri.Min(0.5f, 0.5f, 0.5f, 1f));
            tarkeeb.LahuWajh = true;
            tarkeeb.Wajh = LawnKasri.Min(0.5f, 1f, 1f, 0.5f);
            return Yutabiq(
                "composed",
                LawnKasri.Min(0.125f, 0.5f, 0.5f, 0.5f),
                LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// A string that sets a colour of its own drops the gradient, because
        /// TextMeshPro cannot make the vertex colour be both and chooses the
        /// tag. The base colour is what a glyph with no span of its own falls
        /// back to, so it must not carry the gradient either.
        /// </summary>
        private static string? NitaqYusqitTadarruj()
        {
            LawnKasri asasi = LawnKasri.Min(0.9f, 0.9f, 0.9f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(asasi);
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(LawnKasri.Min(0.1f, 0.1f, 0.1f, 1f));
            tarkeeb.LiNitaqatAlwan = true;
            return Yutabiq("unchanged", asasi, LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// Unless the component is set to ignore its own colour tags, in which
        /// case the gradient is back.
        /// </summary>
        private static string? TajahulYuidTadarruj()
        {
            LawnKasri tadarruj = LawnKasri.Min(0.1f, 0.1f, 0.1f, 1f);
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 1f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(tadarruj);
            tarkeeb.LiNitaqatAlwan = true;
            tarkeeb.TajahulWusum = true;
            return Yutabiq("gradient", tadarruj, LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// A colour outside the unit range is clamped before the face colour
        /// multiplies it, not after. TextMeshPro stores the vertex colour as four
        /// bytes, so the same clamp happens there — and the two orders give
        /// different answers, which is why this is a case and not a comment.
        /// </summary>
        private static string? HasrQablaWajh()
        {
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(2f, 1f, 1f, 1f));
            tarkeeb.LahuWajh = true;
            tarkeeb.Wajh = LawnKasri.Min(0.5f, 0.5f, 0.5f, 1f);
            return Yutabiq(
                "clamped", LawnKasri.Min(0.5f, 0.5f, 0.5f, 1f), LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// A source that reads back as something other than a number is treated
        /// as a source that was never readable. It must never turn a heading
        /// black, transparent, or into whatever a NaN converts to.
        /// </summary>
        private static string? MasdarGhairSalim()
        {
            LawnKasri asasi = LawnKasri.Min(0.2f, 0.4f, 0.6f, 1f);

            TarkeebLawnTmp bilWajh = TarkeebLawnTmp.Min(asasi);
            bilWajh.LahuWajh = true;
            bilWajh.Wajh = LawnKasri.Min(float.NaN, 1f, 1f, 1f);
            string? khata = Yutabiq("face NaN", asasi, LawnNass.Damj(bilWajh));
            if (khata is not null)
            {
                return khata;
            }

            TarkeebLawnTmp bilTadarruj = TarkeebLawnTmp.Min(asasi);
            bilTadarruj.TadarrujMufaal = true;
            bilTadarruj.LahuDakhili = true;
            bilTadarruj.Dakhili = Mutasawi(LawnKasri.Min(1f, float.PositiveInfinity, 1f, 1f));
            khata = Yutabiq("gradient infinite", asasi, LawnNass.Damj(bilTadarruj));
            if (khata is not null)
            {
                return khata;
            }

            TarkeebLawnTmp bilAsasi = TarkeebLawnTmp.Min(LawnKasri.Min(float.NaN, 1f, 1f, 1f));
            return Yutabiq(
                "base NaN", LawnKasri.Min(1f, 1f, 1f, 1f), LawnNass.Damj(bilAsasi));
        }

        /// <summary>
        /// Alpha is a channel like the others and multiplies through both the
        /// gradient and the face colour — which is what makes a half-faded
        /// heading fade rather than jump to opaque.
        /// </summary>
        private static string? ShaffafiyaTadrib()
        {
            TarkeebLawnTmp tarkeeb = TarkeebLawnTmp.Min(LawnKasri.Min(1f, 1f, 1f, 0.5f));
            tarkeeb.TadarrujMufaal = true;
            tarkeeb.LahuDakhili = true;
            tarkeeb.Dakhili = Mutasawi(LawnKasri.Min(1f, 1f, 1f, 0.5f));
            tarkeeb.LahuWajh = true;
            tarkeeb.Wajh = LawnKasri.Min(1f, 1f, 1f, 0.5f);
            return Yutabiq(
                "alpha", LawnKasri.Min(1f, 1f, 1f, 0.125f), LawnNass.Damj(tarkeeb));
        }

        /// <summary>
        /// The span scan that decides whether this string has a colour of its
        /// own. Only the colour flag counts: a bold span is not a colour, and
        /// treating it as one would drop the gradient from every emphasised
        /// heading in a game.
        /// </summary>
        private static string? NitaqatAlwan()
        {
            if (LawnNass.LahuLawnNitaq(ReadOnlySpan<TaaribNitaqUslub>.Empty))
            {
                return "an empty span table reported a colour";
            }

            TaaribNitaqUslub[] bilaLawn = new TaaribNitaqUslub[2];
            bilaLawn[0].Alam = Alamat.UslubWazn;
            bilaLawn[1].Alam = Alamat.UslubHajm | Alamat.UslubMaail;
            if (LawnNass.LahuLawnNitaq(bilaLawn))
            {
                return "a weight span and a size span reported a colour";
            }

            TaaribNitaqUslub[] lahuLawn = new TaaribNitaqUslub[3];
            lahuLawn[0].Alam = Alamat.UslubWazn;
            lahuLawn[1].Alam = Alamat.UslubLawn;
            lahuLawn[2].Alam = Alamat.UslubHajm;
            if (!LawnNass.LahuLawnNitaq(lahuLawn))
            {
                return "a colour span was not found";
            }
            return null;
        }

        /// <summary>The gradient every corner of which is the same colour.</summary>
        private static ArkanTadarruj Mutasawi(LawnKasri lawn)
        {
            return ArkanTadarruj.Min(lawn, lawn, lawn, lawn);
        }

        /// <summary>
        /// Whether two colours agree, or a sentence naming the channel that does
        /// not and both values in full.
        /// </summary>
        private static string? Yutabiq(string wasf, LawnKasri matlub, LawnKasri hali)
        {
            if (Qareeb(matlub.Ahmar, hali.Ahmar)
                && Qareeb(matlub.Akhdar, hali.Akhdar)
                && Qareeb(matlub.Azraq, hali.Azraq)
                && Qareeb(matlub.Shaffafiya, hali.Shaffafiya))
            {
                return null;
            }
            return wasf + ": expected " + Nass(matlub) + ", got " + Nass(hali);
        }

        private static bool Qareeb(float matlub, float hali)
        {
            float farq = matlub - hali;
            return (farq < 0f ? -farq : farq) <= Tasamuh;
        }

        private static string Nass(LawnKasri lawn)
        {
            return string.Format(
                CultureInfo.InvariantCulture,
                "({0:0.######}, {1:0.######}, {2:0.######}, {3:0.######})",
                lawn.Ahmar, lawn.Akhdar, lawn.Azraq, lawn.Shaffafiya);
        }
    }
}
