// اللون — what colour a TextMeshPro glyph is actually tinted with, decided once
// and read by both backends.
//
// TextMeshPro does not keep that colour in one place. `color` is only the base.
// Three other things multiply into it before a fragment is lit:
//
//   - the four-corner vertex gradient (`enableVertexGradient` with either
//     `colorGradient` or `colorGradientPreset`), which TMP bakes into the four
//     vertex colours of every quad;
//   - `faceColor`, which is the material's `_FaceColor`, and which every TMP
//     shader multiplies the interpolated vertex colour by;
//   - the `<color>` tags in the string, which arrive here as style spans and
//     are resolved per glyph by Nasij rather than by this file.
//
// Taarib replaces the mesh AND the material, so the shader that would have
// applied `_FaceColor` never runs, and TMP's own vertex writer that would have
// applied the gradient never runs either. Whatever those two would have
// multiplied in has to be folded into the one colour per glyph run this product
// emits, or it is silently dropped.
//
// Dropping it is not a subtle difference. A heading left with `color` at its
// default white and a dark gradient assigned draws dark in the game and white
// through Taarib — a correctly shaped Arabic heading rendered invisible against
// its own panel.
//
// WHY THIS LIVES IN MUSHTARAK. The two adapters read these sources through
// completely different mechanisms — typed reflection delegates under Mono,
// resolved native entry points and field offsets under IL2CPP — but they have
// to agree on the arithmetic to the last bit, or the same game shipped on both
// backends is two different colours. The reads stay in the adapters, where the
// runtime binding lives; the composition is here, once, and both call it.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// لون كسري — one RGBA colour as four floats between zero and one, which is
    /// the shape <c>UnityEngine.Color</c> has on both backends.
    /// </summary>
    /// <remarks>
    /// Not <see cref="LawnRasm"/>: the composition below multiplies colours
    /// together, and eight bits per channel loses a visible amount of a heading's
    /// tint over three multiplications. The narrowing to bytes stays where it is
    /// today, at the end, in each adapter's own packing.
    /// </remarks>
    public struct LawnKasri
    {
        /// <summary>Red.</summary>
        public float Ahmar;

        /// <summary>Green.</summary>
        public float Akhdar;

        /// <summary>Blue.</summary>
        public float Azraq;

        /// <summary>Alpha; one is opaque.</summary>
        public float Shaffafiya;

        /// <summary>One colour from its four channels.</summary>
        /// <param name="ahmar">Red.</param>
        /// <param name="akhdar">Green.</param>
        /// <param name="azraq">Blue.</param>
        /// <param name="shaffafiya">Alpha.</param>
        /// <returns>The colour.</returns>
        public static LawnKasri Min(float ahmar, float akhdar, float azraq, float shaffafiya)
        {
            LawnKasri natija;
            natija.Ahmar = ahmar;
            natija.Akhdar = akhdar;
            natija.Azraq = azraq;
            natija.Shaffafiya = shaffafiya;
            return natija;
        }

        /// <summary>
        /// One colour from the four bytes a <c>Color32</c> holds, which is what
        /// <c>TMP_Text.faceColor</c> answers with.
        /// </summary>
        /// <param name="ahmar">Red.</param>
        /// <param name="akhdar">Green.</param>
        /// <param name="azraq">Blue.</param>
        /// <param name="shaffafiya">Alpha.</param>
        /// <returns>The colour.</returns>
        public static LawnKasri MinBayt(byte ahmar, byte akhdar, byte azraq, byte shaffafiya)
        {
            return Min(ahmar / 255f, akhdar / 255f, azraq / 255f, shaffafiya / 255f);
        }

        /// <summary>Opaque white — the identity of every multiplication here.</summary>
        /// <returns>Opaque white.</returns>
        public static LawnKasri Abyad()
        {
            return Min(1f, 1f, 1f, 1f);
        }
    }

    /// <summary>
    /// أركان التدرّج — the four corner colours of a TextMeshPro vertex gradient.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The corners are numbered rather than named, and that is deliberate.
    /// <see cref="LawnNass.Damj"/> only ever takes their mean, the mean is
    /// symmetric, and so nothing in this product depends on which corner is the
    /// top left. That is what lets the IL2CPP adapter read the engine's own
    /// sixty-four-byte gradient struct as four colours without knowing the order
    /// its fields were declared in, and it is one fewer thing to get wrong for a
    /// type neither adapter can name at compile time.
    /// </para>
    /// </remarks>
    public struct ArkanTadarruj
    {
        /// <summary>One corner.</summary>
        public LawnKasri Awwal;

        /// <summary>Another corner.</summary>
        public LawnKasri Thani;

        /// <summary>Another corner.</summary>
        public LawnKasri Thalith;

        /// <summary>The last corner.</summary>
        public LawnKasri Rabi;

        /// <summary>The four corners as one value.</summary>
        /// <param name="awwal">One corner.</param>
        /// <param name="thani">Another corner.</param>
        /// <param name="thalith">Another corner.</param>
        /// <param name="rabi">The last corner.</param>
        /// <returns>The gradient.</returns>
        public static ArkanTadarruj Min(
            LawnKasri awwal, LawnKasri thani, LawnKasri thalith, LawnKasri rabi)
        {
            ArkanTadarruj natija;
            natija.Awwal = awwal;
            natija.Thani = thani;
            natija.Thalith = thalith;
            natija.Rabi = rabi;
            return natija;
        }

        /// <summary>
        /// The one colour that stands in for the whole gradient: the arithmetic
        /// mean of the four corners, per channel.
        /// </summary>
        /// <returns>The mean.</returns>
        /// <remarks>
        /// <para>
        /// Taarib emits one colour per glyph, so a four-corner gradient is not
        /// representable and something has to stand in for it. The mean is not a
        /// compromise picked for being in the middle — it is the colour the
        /// hardware itself would produce at the centre of the quad. Vertex
        /// colours are interpolated componentwise across a triangle in exactly
        /// the space they are stored in, so the value halfway along both axes is
        /// the componentwise average of the four corners, and a glyph tinted with
        /// it differs from the gradient only by the variation the gradient was
        /// asked for.
        /// </para>
        /// <para>
        /// Taking a corner instead is the failure this whole file exists to
        /// undo. A designer's dark-to-light heading gradient has one pale corner
        /// in it; choosing that corner reproduces the washed-out heading exactly,
        /// and choosing the opposite one over-darkens every gradient in the game
        /// instead. The mean cannot do either.
        /// </para>
        /// </remarks>
        public LawnKasri Mutawassit()
        {
            return LawnKasri.Min(
                (Awwal.Ahmar + Thani.Ahmar + Thalith.Ahmar + Rabi.Ahmar) * 0.25f,
                (Awwal.Akhdar + Thani.Akhdar + Thalith.Akhdar + Rabi.Akhdar) * 0.25f,
                (Awwal.Azraq + Thani.Azraq + Thalith.Azraq + Rabi.Azraq) * 0.25f,
                (Awwal.Shaffafiya + Thani.Shaffafiya + Thalith.Shaffafiya + Rabi.Shaffafiya)
                    * 0.25f);
        }
    }

    /// <summary>
    /// تركيب لون النص — every colour source one TextMeshPro component carries,
    /// as read from it, with a flag per source saying whether it was readable at
    /// all.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Every one of these is a reflective binding against the game's own build of
    /// TextMeshPro, and every one of them can fail to resolve: TMP has shipped as
    /// an asset-store package, as a built-in package, and across three major
    /// versions. So each source carries its own <c>Lahu…</c> flag rather than a
    /// sentinel value, and an absent source multiplies nothing — which is what
    /// makes <see cref="Min"/> produce exactly what this product produced before
    /// any of these sources were read.
    /// </para>
    /// </remarks>
    public struct TarkeebLawnTmp
    {
        /// <summary>
        /// The component's own <c>color</c>, which is also where its
        /// <c>alpha</c> lives: TextMeshPro's <c>alpha</c> property is a second
        /// name for the alpha channel of this colour and not a fifth number, so
        /// there is nothing separate to read for it.
        /// </summary>
        public LawnKasri Asasi;

        /// <summary>Whether <see cref="Wajh"/> was readable.</summary>
        public bool LahuWajh;

        /// <summary>
        /// The material's <c>_FaceColor</c>, from <c>TMP_Text.faceColor</c>.
        /// Four zeros here means the shader has no such property rather than a
        /// colour a designer chose; see <see cref="LawnNass.Damj"/>.
        /// </summary>
        public LawnKasri Wajh;

        /// <summary>The component's <c>enableVertexGradient</c>.</summary>
        public bool TadarrujMufaal;

        /// <summary>Whether <see cref="Qalib"/> was readable.</summary>
        public bool LahuQalib;

        /// <summary>The assigned <c>colorGradientPreset</c>, when there is one.</summary>
        public ArkanTadarruj Qalib;

        /// <summary>Whether <see cref="Dakhili"/> was readable.</summary>
        public bool LahuDakhili;

        /// <summary>The component's own inline <c>colorGradient</c>.</summary>
        public ArkanTadarruj Dakhili;

        /// <summary>The component's <c>overrideColorTags</c>.</summary>
        public bool TajahulWusum;

        /// <summary>
        /// Whether any style span over this string sets a colour — TextMeshPro's
        /// colour stack being deeper than its base entry, expressed in the form
        /// this product has it in.
        /// </summary>
        public bool LiNitaqatAlwan;

        /// <summary>
        /// The composition with one source: the component's own colour, and
        /// nothing else readable.
        /// </summary>
        /// <param name="asasi">The component's <c>color</c>.</param>
        /// <returns>The composition.</returns>
        /// <remarks>
        /// This is the state every binding failing degrades to, and it is exactly
        /// what the TextMeshPro takeover drew before any of the other sources
        /// existed: <see cref="LawnNass.Damj"/> over it answers with
        /// <paramref name="asasi"/> itself.
        /// </remarks>
        public static TarkeebLawnTmp Min(LawnKasri asasi)
        {
            TarkeebLawnTmp natija = default;
            natija.Asasi = asasi;
            return natija;
        }
    }

    /// <summary>
    /// لون النص — the one place that decides what a run of glyphs is tinted
    /// with, from everything a TextMeshPro component can say about it.
    /// </summary>
    public static class LawnNass
    {
        /// <summary>
        /// Composes the colour a glyph run draws in.
        /// </summary>
        /// <param name="tarkeeb">Every source that could be read.</param>
        /// <returns>The colour, every channel between zero and one.</returns>
        /// <remarks>
        /// <para>
        /// <b>The composition, in TextMeshPro's own order.</b> TMP builds the
        /// vertex colour first and its shader multiplies afterwards, so this does
        /// the same three steps in the same sequence:
        /// </para>
        /// <list type="number">
        /// <item><description>
        /// the component's <c>color</c>, clamped — TMP stores the vertex colour
        /// as a <c>Color32</c>, so a colour outside the unit range is already
        /// clamped by the time anything multiplies it;
        /// </description></item>
        /// <item><description>
        /// times the gradient, when one applies, and clamped again for the same
        /// reason — <c>corner * vertexColor</c> is what
        /// <c>TMP_Text.SaveGlyphVertexInfo</c> writes into each of the four
        /// vertices, and the corners collapse to one colour through
        /// <see cref="ArkanTadarruj.Mutawassit"/>;
        /// </description></item>
        /// <item><description>
        /// times <c>faceColor</c> — every TMP shader computes its face colour as
        /// <c>vertexColour * _FaceColor</c>, componentwise and including alpha,
        /// and Taarib's own material has no such property, so a face colour that
        /// is not folded in here is a face colour the player never sees. Four
        /// zeros are skipped rather than multiplied; <see cref="Sifr"/> says why.
        /// </description></item>
        /// </list>
        /// <para>
        /// <b>When the gradient applies.</b> Not simply when
        /// <c>enableVertexGradient</c> is on. TextMeshPro drops the gradient for a
        /// string whose colour stack is deeper than its base entry unless
        /// <c>overrideColorTags</c> is set, because a per-word colour and a
        /// gradient cannot both be the vertex colour. The same rule is applied
        /// here over <see cref="TarkeebLawnTmp.LiNitaqatAlwan"/>: a string with no
        /// colour span behaves as TMP's depth-one stack does, and a heading —
        /// which is the element a gradient is actually used on — has no colour
        /// span in it.
        /// </para>
        /// <para>
        /// <b>A preset beats an inline gradient</b>, which is TMP's own
        /// precedence: <c>SaveGlyphVertexInfo</c> reads
        /// <c>m_fontColorGradientPreset</c> when one is assigned and the
        /// serialized <c>m_fontColorGradient</c> only when it is not.
        /// </para>
        /// <para>
        /// <b>A source that did not resolve multiplies nothing.</b> Every branch
        /// is guarded by its own flag and by <see cref="Salim"/>, so a binding
        /// this game's build of TextMeshPro does not have costs exactly that one
        /// multiplication. With none of them readable this returns the clamped
        /// component colour, which is what the takeover drew before this file
        /// existed — never white, never black, never transparent.
        /// </para>
        /// </remarks>
        public static LawnKasri Damj(in TarkeebLawnTmp tarkeeb)
        {
            LawnKasri natija = Hasr(Salim(in tarkeeb.Asasi) ? tarkeeb.Asasi : LawnKasri.Abyad());

            if (Tadarruj(in tarkeeb, out ArkanTadarruj arkan))
            {
                LawnKasri mutawassit = arkan.Mutawassit();
                if (Salim(in mutawassit))
                {
                    natija = Hasr(Darb(in natija, in mutawassit));
                }
            }

            if (tarkeeb.LahuWajh && Salim(in tarkeeb.Wajh) && !Sifr(in tarkeeb.Wajh))
            {
                natija = Darb(in natija, in tarkeeb.Wajh);
            }

            return Hasr(natija);
        }

        /// <summary>
        /// Whether any style span over a string sets a colour, which is this
        /// product's form of TextMeshPro's colour stack being non-trivial.
        /// </summary>
        /// <param name="nitaqat">The spans over the string.</param>
        /// <returns>Whether one of them sets a colour.</returns>
        public static bool LahuLawnNitaq(ReadOnlySpan<TaaribNitaqUslub> nitaqat)
        {
            for (int i = 0; i < nitaqat.Length; i++)
            {
                if ((nitaqat[i].Alam & Alamat.UslubLawn) != 0)
                {
                    return true;
                }
            }
            return false;
        }

        /// <summary>
        /// Which gradient applies, if either does.
        /// </summary>
        private static bool Tadarruj(in TarkeebLawnTmp tarkeeb, out ArkanTadarruj arkan)
        {
            arkan = default;
            if (!tarkeeb.TadarrujMufaal)
            {
                return false;
            }
            if (!tarkeeb.TajahulWusum && tarkeeb.LiNitaqatAlwan)
            {
                return false;
            }
            if (tarkeeb.LahuQalib)
            {
                arkan = tarkeeb.Qalib;
                return true;
            }
            if (tarkeeb.LahuDakhili)
            {
                arkan = tarkeeb.Dakhili;
                return true;
            }
            return false;
        }

        /// <summary>Componentwise product, alpha included.</summary>
        private static LawnKasri Darb(in LawnKasri awwal, in LawnKasri thani)
        {
            return LawnKasri.Min(
                awwal.Ahmar * thani.Ahmar,
                awwal.Akhdar * thani.Akhdar,
                awwal.Azraq * thani.Azraq,
                awwal.Shaffafiya * thani.Shaffafiya);
        }

        /// <summary>Every channel brought into the unit range.</summary>
        private static LawnKasri Hasr(in LawnKasri lawn)
        {
            return LawnKasri.Min(
                Wahid(lawn.Ahmar),
                Wahid(lawn.Akhdar),
                Wahid(lawn.Azraq),
                Wahid(lawn.Shaffafiya));
        }

        private static float Wahid(float qeema)
        {
            if (qeema < 0f)
            {
                return 0f;
            }
            return qeema > 1f ? 1f : qeema;
        }

        /// <summary>
        /// Whether a colour is a number at all.
        /// </summary>
        /// <remarks>
        /// A reflective read of a member this build of TextMeshPro declares
        /// differently can hand back a NaN, and NaN fails every comparison — it
        /// would pass <see cref="Hasr"/> untouched and then convert to an
        /// arbitrary byte. A source that is not finite is treated as a source
        /// that was never readable, which is the one degradation this file
        /// guarantees.
        /// </remarks>
        private static bool Salim(in LawnKasri lawn)
        {
            return Adad(lawn.Ahmar) && Adad(lawn.Akhdar)
                && Adad(lawn.Azraq) && Adad(lawn.Shaffafiya);
        }

        private static bool Adad(float qeema)
        {
            return !float.IsNaN(qeema) && !float.IsInfinity(qeema);
        }

        /// <summary>
        /// Whether a colour is zero in all four channels.
        /// </summary>
        /// <remarks>
        /// <para>
        /// This exists for one source and one failure. <c>TMP_Text.faceColor</c>
        /// is not a field on the component; its getter asks the shared material
        /// for <c>_FaceColor</c>, and a material whose shader does not declare
        /// that property answers with four zeros rather than refusing. Folding
        /// four zeros in would multiply a heading to invisible — the loudest
        /// possible regression, on exactly the games least likely to be tested.
        /// </para>
        /// <para>
        /// Treating it as absent is not a guess about what the designer meant.
        /// A shader with no <c>_FaceColor</c> is a shader that never multiplied
        /// by one, so the vertex colour alone is the whole of what the engine
        /// would have drawn — which is precisely what skipping this step
        /// produces. A face colour that is genuinely transparent but not black,
        /// such as a fade written into the material's alpha, is a real value and
        /// is still honoured.
        /// </para>
        /// </remarks>
        private static bool Sifr(in LawnKasri lawn)
        {
            return lawn.Ahmar == 0f && lawn.Akhdar == 0f
                && lawn.Azraq == 0f && lawn.Shaffafiya == 0f;
        }
    }
}
