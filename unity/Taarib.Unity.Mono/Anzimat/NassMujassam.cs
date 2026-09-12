// النص المجسّم — the takeover for world-space UnityEngine.TextMesh.
//
// This is the simplest of the text systems in this directory and the one most
// often forgotten, in both directions: games forget that the floating label
// over a signpost is text at all, and translation plugins forget that a
// TextMesh exists because it belongs to no canvas and never appears in a
// hierarchy search for the usual component names. It is also the one place
// where an untranslated game and a badly translated one look identical from a
// distance and completely different close up.
//
// WHY THERE IS NO HARMONY PATCH IN THIS FILE, AND WHY THAT IS NOT AN OMISSION.
// The other takeovers in this directory patch the method where a component
// decides what to draw. TextMesh has no such method. Every member of
// UnityEngine.TextMesh — text, font, fontSize, characterSize, anchor,
// alignment, lineSpacing, offsetZ, richText, color — is an extern property
// backed by an internal call, and the mesh itself is generated inside the
// engine and handed to the MeshRenderer without passing through managed code at
// any point. An extern method has no IL body, and HarmonyX weaves IL: there is
// nothing to prefix. Installing a patch anyway would throw at load in every
// game that has a TextMesh, which is worse than a takeover that says plainly
// how it works.
//
// So this takeover is driven from the scene instead. A HarsMujassam is attached
// to each TextMesh; it watches the component's own text, and while that text is
// one the installed patch covers it disables the renderer the engine feeds and
// draws Taarib's own mesh from a child object. The engine's generated mesh is
// REPLACED, not corrected. There is no useful correction available: correcting
// it would mean reordering quads the engine already emitted for isolated
// letters, which is reordering the wrong shapes.
//
// WHY THE ENGINE'S OWN MESH IS THE FAILURE THIS FILE EXISTS TO PREVENT. A
// TextMesh draws through UnityEngine.Font, and a dynamic font produces its
// glyph bitmaps through Font.RequestCharactersInTexture — a call that takes a
// string, a size and a style, and rasterizes each CHARACTER independently into
// the font texture. It performs no shaping. It has no notion of joining, of
// contextual forms, of ligatures or of mark attachment, and it reorders
// nothing. Handed Arabic it produces exactly what it was asked for: the
// isolated form of every letter, laid left to right, unjoined, with every
// diacritic occupying its own advance beside the letter it belongs to instead
// of sitting above it. That output is not slightly wrong. It is the specific,
// famous failure this entire product exists to prevent, and it is why nothing
// here asks the game's font for a glyph. Decision 5 forbids reading it in any
// case; the point worth making is that there would be nothing worth reading.
//
// THE PIXEL SIZE A LAYOUT IS MEASURED AT IS NOT THE SIZE IT APPEARS AT. World
// space text faces a camera. It is subject to perspective, it grows as the
// player walks toward it and shrinks as they walk away, and one frame may draw
// the same label at two hundred screen pixels tall and another at three. None
// of that is a layout parameter. A layout is measured at the size the COMPONENT
// declares — TextMesh.fontSize, in font pixels — and mapped into world units by
// characterSize, which Unity defines as a tenth of a unit per font pixel. So
// the quarter-pixel key this adapter looks a precomputed layout up by is
// Nasij.HajmRubi(fontSize), and it is stable for as long as the component's own
// fontSize is: the layout, the atlas key and the glyph images all stay put
// while the player moves. Keying any of it on apparent screen size would
// rasterize a fresh glyph set every frame the camera moved, would evict the
// atlas continuously, and would make text visibly re-flow as a player walked
// toward a sign — which nobody would report as a font bug, because it is not
// one.
//
// A TextMesh whose fontSize is zero draws at the size its font asset was
// imported at. This file does not read that number: Decision 5 forbids sampling
// a metric from the game's font, and the size an asset was imported at is a
// metric of that asset. The answer comes from the patch instead — the compiler
// recorded the size the original draws at in the constraint row for that
// string, in Studio, with the game not running — and falls back to
// HajmIftiradi when the patch carries no constraint for it.

using System;
using System.Collections.Generic;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mono.Suluk;
using Taarib.Unity.Mushtarak;
using UnityEngine;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// نظام النص المجسّم — the world-space text takeover: the patch, the atlas,
    /// the layout buffers, and the geometry every <see cref="HarsMujassam"/>
    /// draws through.
    /// </summary>
    /// <remarks>
    /// <para>
    /// One instance serves every world-space label in the process, and the
    /// buffers below are shared for that reason: Unity drives every
    /// <c>LateUpdate</c> on one thread, one guard at a time, and a mesh
    /// assignment copies into native memory before the call returns — so a
    /// buffer filled and consumed inside a single guard's update cannot be seen
    /// by a second guard halfway through. The same property is what makes the
    /// shared <see cref="Takhtit"/> safe here and unsafe across threads.
    /// </para>
    /// <para>
    /// <b>There is no Harmony instance in the factory,</b> and the header of
    /// this file says why: every member of <see cref="TextMesh"/> is an extern
    /// property with no IL body, so this is the one takeover in the directory
    /// with no method to prefix.
    /// </para>
    /// <para>
    /// <b>What allocates.</b> <see cref="Ibda"/> allocates, once, at load.
    /// <see cref="Fahras"/> allocates: it asks Unity for every
    /// <see cref="TextMesh"/> in the loaded scenes and gets an array back, and
    /// it runs on a scene load rather than on a frame. A rebuild allocates only
    /// when the shared geometry buffers grow to the longest string seen so far.
    /// </para>
    /// <para>
    /// <b>The one unavoidable per-frame allocation is named here.</b>
    /// <see cref="TextMesh.text"/> is an extern property: reading it marshals a
    /// fresh managed string out of native memory every time. A guard has to
    /// read it to notice that the game changed it, and there is no managed
    /// member to patch that would say so instead. <see cref="FatratFahs"/>
    /// bounds the cost — at its default a label is polled once every four
    /// frames, so fifty world-space labels cost fifty short strings every four
    /// frames and a change reaches the screen within about seventy
    /// milliseconds at sixty hertz. Raising it trades latency for garbage in
    /// whichever direction a particular game needs.
    /// </para>
    /// </remarks>
    public sealed class NizamMujassam : IDisposable
    {
        /// <summary>
        /// Unity's own conversion between a TextMesh's font pixels and world
        /// units: <c>characterSize</c> is documented as a tenth of a unit per
        /// font pixel, and the engine's own generator applies exactly this
        /// factor. It is named rather than written inline because it is the
        /// single number that decides whether a label is the right size in the
        /// world, and a wrong one reads as a layout bug rather than as a scale.
        /// </summary>
        public const float WahdatMujassam = 0.1f;

        private static NizamMujassam? hali;

        private readonly MasdarAshkal masdar;
        private readonly Ruqaa ruqaa;
        private readonly Takhtit? takhtit;
        private readonly MaqbadSiyaq? siyaq;
        private readonly MaqbadSilsila? silsila;
        private readonly MakhzanRusum makhzan;
        private readonly NassMuhaddar muhaddar;
        private readonly List<HarsMujassam> hurras;

        private bool amil;
        private bool ballaghSafahat;
        private bool ballaghTakhtit;

        private NizamMujassam(
            MasdarAshkal masdar,
            Ruqaa ruqaa,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit)
        {
            this.masdar = masdar;
            this.ruqaa = ruqaa;
            this.siyaq = siyaq;
            this.silsila = silsila;
            this.takhtit = takhtit;
            makhzan = new MakhzanRusum();
            muhaddar = new NassMuhaddar();
            hurras = new List<HarsMujassam>(32);
            HajmIftiradi = 16f;
            FatratFahs = 4;
            amil = true;
        }

        /// <summary>The live world-space takeover, or <c>null</c> when none is installed.</summary>
        public static NizamMujassam? Hali => hali;

        /// <summary>Whether this takeover is still drawing.</summary>
        public bool Amil => amil;

        /// <summary>
        /// The size in font pixels to lay a label out at when its
        /// <see cref="TextMesh.fontSize"/> is zero and the patch records no
        /// constraint for its string.
        /// </summary>
        /// <remarks>
        /// A zero <c>fontSize</c> means "draw at the size the font asset was
        /// imported at", and this file does not read that number — Decision 5
        /// forbids sampling a metric from the game's own font, and an import
        /// size is one. The patch's constraint row is consulted first, because
        /// the compiler recorded the original's size there with the game not
        /// running. This value is the last resort, and it is a setting rather
        /// than a constant because the right answer differs between games and a
        /// wrong one is visible rather than subtle.
        /// </remarks>
        public float HajmIftiradi { get; set; }

        /// <summary>
        /// How many frames a guard waits between reads of its component's text.
        /// One polls every frame; the default of four is the trade described on
        /// this class. Values below one are treated as one.
        /// </summary>
        public int FatratFahs { get; set; }

        /// <summary>How many world-space labels this takeover is drawing.</summary>
        public int AdadMarsuma { get; private set; }

        /// <summary>
        /// How many guards are attached but declining, because the string on
        /// their component is not one the patch covers. A large number beside a
        /// small <see cref="AdadMarsuma"/> is the signature of a patch compiled
        /// against a different build of the game, which is worth saying rather
        /// than leaving a user to wonder why half the world is still English.
        /// </summary>
        public int AdadMatruka { get; private set; }

        /// <summary>
        /// Confirms this runtime's vertex layout and returns the takeover.
        /// </summary>
        /// <param name="ruqaa">The open patch container.</param>
        /// <param name="masdar">The glyph source and its uploaded atlas.</param>
        /// <param name="siyaq">
        /// The engine context, for text the compiler never laid out.
        /// <c>null</c> leaves precomputed layouts working and declines every
        /// string with no precomputed layout at the size it is drawn at.
        /// </param>
        /// <param name="silsila">The font chain runtime layout shapes with.</param>
        /// <param name="takhtit">
        /// A layout buffer used by this takeover and nothing else. A
        /// <see cref="Takhtit"/> exposes its results as spans over one reused
        /// pair of arrays, so sharing one with another takeover would let one
        /// label's layout overwrite the glyphs another is halfway through
        /// turning into triangles.
        /// </param>
        /// <returns>
        /// The takeover, or <c>null</c> when this runtime's vertex types are
        /// not what Taarib was built against, in which case the reason is in
        /// the log and every other text system is unaffected.
        /// </returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="ruqaa"/> or <paramref name="masdar"/> is null.
        /// </exception>
        public static NizamMujassam? Ibda(
            Ruqaa ruqaa,
            MasdarAshkal masdar,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit? takhtit)
        {
            if (ruqaa is null)
            {
                throw new ArgumentNullException(nameof(ruqaa));
            }
            if (masdar is null)
            {
                throw new ArgumentNullException(nameof(masdar));
            }
            if (hali is not null)
            {
                return hali;
            }

            // The vertex layout is not re-checked here. MasdarAshkal.Insha
            // already ran Nasij.TahaqquqTakhtit once and refused to exist at
            // all on a runtime whose Vector3 is not three floats, and a
            // takeover cannot be built without one — so a second check could
            // only ever agree, and two copies of one refusal is how the two
            // eventually disagree.
            NizamMujassam nizam = new NizamMujassam(masdar, ruqaa, siyaq, silsila, takhtit);
            hali = nizam;
            return nizam;
        }

        /// <summary>
        /// Attaches a guard to every active <see cref="TextMesh"/> in the
        /// loaded scenes.
        /// </summary>
        /// <remarks>
        /// Call this on a scene load rather than on a frame: the search walks
        /// every object in the scene and returns an array. A label spawned
        /// afterwards is registered through <see cref="Sajjil"/> by whoever
        /// spawned it, or picked up by the next scan. There is no cheaper
        /// answer available — a component with no managed constructor and no
        /// managed enable callback cannot announce itself.
        /// </remarks>
        /// <returns>
        /// How many labels are guarded afterwards, including the ones that
        /// already were.
        /// </returns>
        public int Fahras()
        {
            if (!amil)
            {
                return 0;
            }
            TextMesh[] nusus = UnityEngine.Object.FindObjectsOfType<TextMesh>();
            int adad = 0;
            for (int i = 0; i < nusus.Length; i++)
            {
                if (Sajjil(nusus[i]) is not null)
                {
                    adad++;
                }
            }
            return adad;
        }

        /// <summary>
        /// Attaches a guard to one world-space label, or returns the guard it
        /// already has.
        /// </summary>
        /// <param name="mujassam">The label. A destroyed or null one is ignored.</param>
        /// <returns>The guard, or <c>null</c> when one could not be attached.</returns>
        public HarsMujassam? Sajjil(TextMesh mujassam)
        {
            if (!amil || mujassam == null)
            {
                return null;
            }
            try
            {
                GameObject jism = mujassam.gameObject;
                HarsMujassam? hars = jism.GetComponent<HarsMujassam>();
                if (hars == null)
                {
                    hars = jism.AddComponent<HarsMujassam>();
                    hurras.Add(hars);
                }
                hars.Wassil(this, mujassam);
                return hars;
            }
            catch (Exception khata)
            {
                Rabt.Ballagh(
                    "TextMesh: attaching a guard to a world-space label failed; that label "
                    + "is left to the engine and every other one is unaffected",
                    khata);
                return null;
            }
        }

        /// <summary>Taarib's atlas material for the atlas a label was drawn from.</summary>
        /// <remarks>
        /// The other atlas is never offered in its place. A mesh built from the
        /// patch's compiled atlas indexes that atlas's rectangles and one built
        /// at run time indexes this session's; binding the wrong page samples
        /// whatever happens to sit at those coordinates and paints the label as
        /// solid blocks. A label whose own atlas carries no material is left to
        /// the engine's renderer instead.
        /// </remarks>
        /// <param name="minRuqaa">Whether the glyphs came from the patch's compiled atlas.</param>
        /// <returns>The material, or <c>null</c> when that atlas is not resident.</returns>
        public Material? Madda(bool minRuqaa)
        {
            return masdar.Madda(minRuqaa, 0);
        }

        /// <summary>
        /// Switches the takeover off, naming the reason once, and returns every
        /// label to the engine.
        /// </summary>
        /// <param name="sabab">What failed and what it costs.</param>
        /// <param name="khata">The exception behind it.</param>
        public void Awqif(string sabab, Exception khata)
        {
            if (!amil)
            {
                return;
            }
            Rabt.Ballagh(
                "TextMesh: " + sabab + "; world-space text is returned to the engine's own "
                + "renderer and every other text system is unaffected", khata);
            Dispose();
        }

        /// <summary>
        /// Stops drawing and returns every attached guard's component to the
        /// engine's own renderer. The guards stay attached and harmless, so a
        /// takeover installed again does not have to walk the scene twice.
        /// </summary>
        public void Dispose()
        {
            amil = false;
            for (int i = 0; i < hurras.Count; i++)
            {
                HarsMujassam hars = hurras[i];
                if (hars == null)
                {
                    continue;
                }
                try
                {
                    hars.Ruddi();
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh("TextMesh: returning a label to the engine failed", khata);
                }
            }
            hurras.Clear();
            AdadMarsuma = 0;
            AdadMatruka = 0;
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
        }

        /// <summary>
        /// Builds one label's mesh, or reports that the patch does not cover
        /// its string.
        /// </summary>
        /// <param name="mujassam">The label.</param>
        /// <param name="nass">Its current text, read once by the guard.</param>
        /// <param name="nasij">The mesh to fill; cleared before anything else.</param>
        /// <param name="minRuqaa">
        /// Which atlas the glyphs came from, so the guard binds the matching
        /// material.
        /// </param>
        /// <returns>Whether the mesh now holds this label's text.</returns>
        internal bool Irsim(TextMesh mujassam, string nass, Mesh nasij, out bool minRuqaa)
        {
            minRuqaa = true;
            if (!amil || mujassam == null || nasij == null || string.IsNullOrEmpty(nass)
                || !masdar.Amil)
            {
                return false;
            }

            // MiftahMinNass encodes the string to UTF-8 to hash it: under 512
            // bytes that encode is a stackalloc, so the lookup that decides
            // whether Taarib owns a label allocates nothing at all.
            ulong miftah = Ruqaa.MiftahMinNass(nass);
            int fahras = ruqaa.JidNass(miftah);
            if (fahras < 0)
            {
                Rabt.Fawt(nass, miftah);
                return false;
            }

            bool lahuQayd = ruqaa.JidQayd(fahras, out MadkhalQayd qayd);
            float hajm = mujassam.fontSize;
            if (!(hajm > 0f))
            {
                hajm = lahuQayd && qayd.Hajm > 0f ? qayd.Hajm : HajmIftiradi;
            }
            if (!(hajm > 0f))
            {
                return false;
            }

            // The scale between a layout pixel and a world unit. It is not, and
            // must never be, derived from how large the label happens to look:
            // that number changes with the camera and would re-flow the text as
            // the player walked toward it.
            float qiyas = mujassam.characterSize * WahdatMujassam;
            if (!(qiyas > 0f))
            {
                return false;
            }

            ReadOnlySpan<TaaribHarf> huruf;
            ReadOnlySpan<TaaribSatr> sutur;
            ReadOnlySpan<TaaribNitaqUslub> nitaqat;
            float ardTakhtit;
            float irtifaTakhtit;
            float hajmFili;

            // The exact quarter-pixel size, never the nearest, and taken from
            // the component rather than from the screen.
            ushort hajmRubi = Nasij.HajmRubi(hajm);
            minRuqaa = ruqaa.JidTakhtit(fahras, hajmRubi, out MadkhalTakhtit madkhal);
            if (minRuqaa)
            {
                huruf = ruqaa.HurufTakhtit(in madkhal);
                sutur = ruqaa.SuturTakhtit(in madkhal);
                nitaqat = makhzan.Hawwil(ruqaa.NitaqatNass(fahras));
                ardTakhtit = madkhal.Ard;
                irtifaTakhtit = madkhal.Irtifa;
                hajmFili = madkhal.Hajm;
            }
            else if (!Khattit(
                mujassam, fahras, hajm, lahuQayd, in qayd,
                out huruf, out sutur, out nitaqat,
                out ardTakhtit, out irtifaTakhtit, out hajmFili))
            {
                return false;
            }

            if (huruf.IsEmpty || !(hajmFili > 0f))
            {
                return false;
            }

            float hajmLawha = masdar.HajmLawha(hajmFili);
            if (!minRuqaa && !masdar.Aqim(huruf, hajmLawha, tathbit: false))
            {
                return false;
            }

            HayyizRasm hayyiz = default;

            // A TextMesh has no rectangle, so the rectangle is the text block
            // itself and the anchor is the pivot on it. With the block's own
            // size in Ard and Irtifa, Nasij's placement reduces to exactly what
            // TextAnchor means: an Upper anchor puts the block's top edge on
            // the origin, a Middle anchor puts its centre there, a Lower anchor
            // puts its bottom edge there, and the same three cases run
            // horizontally through MihwarS. Vertical placement is Ala with the
            // layout height passed through, which makes the vertical slack
            // zero — the anchor has already placed the block, and a second
            // placement would move it twice.
            hayyiz.Ard = ardTakhtit * qiyas;
            hayyiz.Irtifa = irtifaTakhtit * qiyas;
            hayyiz.MihwarS = MihwarS(mujassam.anchor);
            hayyiz.MihwarA = MihwarA(mujassam.anchor);
            hayyiz.Qiyas = qiyas;
            hayyiz.IrtifaTakhtit = irtifaTakhtit;
            hayyiz.Muhadhaha = MuhadhahaRasiya.Ala;

            TalabNasij talab = default;
            talab.Huruf = huruf;
            talab.Sutur = sutur;
            talab.Nitaqat = nitaqat;
            talab.Hayyiz = hayyiz;
            talab.Lawn = LawnRasm.Min(LawnMuazzam(mujassam.color));
            talab.Hajm = hajmFili;
            talab.HajmLawha = hajmLawha;
            talab.Safha = 0;

            // Never snapped. Snapping exists so that a coverage bitmap, whose
            // subpixel fraction was baked into its own antialiasing, lands on
            // the pixel grid it was rasterized for. World-space text has no
            // pixel grid of its own: a layout pixel becomes a world unit and
            // then whatever the camera makes of it, so snapping in layout space
            // snaps to nothing on screen and only makes the letter spacing
            // uneven at the sizes where it is most visible.
            talab.Tathbit = false;

            makhzan.Wassi(huruf.Length);
            NatijaNasij natija = Nasij.Ibni(
                in talab, masdar.Khareeta(minRuqaa), makhzan.Makhzan());
            if (!natija.Kafa)
            {
                makhzan.Wassi(natija.MatlubRuus / Nasij.RuusLiShakl);
                natija = Nasij.Ibni(in talab, masdar.Khareeta(minRuqaa), makhzan.Makhzan());
                if (!natija.Kafa)
                {
                    Rabt.Ballagh(
                        "TextMesh: the mesh buffer refused a second time after growing to "
                        + "the size it asked for; this label is left to the engine.");
                    return false;
                }
            }
            if (natija.AdadAshkal == 0)
            {
                return false;
            }
            if ((natija.AlamSafahat & ~1UL) != 0 || natija.SafahatBaida)
            {
                BallighSafahat();
            }

            makhzan.Amsah(in natija);
            Aktub(nasij, in natija);
            return true;
        }

        /// <summary>Records that one label started or stopped drawing.</summary>
        /// <param name="yarsum">Whether it is drawing now.</param>
        internal void Ahsi(bool yarsum)
        {
            if (yarsum)
            {
                AdadMarsuma++;
            }
            else if (AdadMarsuma > 0)
            {
                AdadMarsuma--;
            }
        }

        /// <summary>Records that one label's string was not in the patch.</summary>
        internal void AhsiMatruka()
        {
            AdadMatruka++;
        }

        /// <summary>
        /// Hands the built geometry to the mesh.
        /// </summary>
        /// <remarks>
        /// Only the range the build wrote is uploaded. The buffers are grown to
        /// a block size and reused, so handing the mesh the whole array hands it
        /// the tail of the previous, longer label as well — and its triangles
        /// index vertices this label never wrote, which R.E.P.O. drew as a black
        /// wedge across the screen from the TextMeshPro takeover. The bounds are
        /// written explicitly from the box Nasij measured while it walked the
        /// glyphs, which also saves the second pass a <c>RecalculateBounds</c>
        /// would make.
        /// </remarks>
        private void Aktub(Mesh nasij, in NatijaNasij natija)
        {
            // Cleared first, then vertices, then indices. Assigning indices to
            // a mesh whose vertex array is still the previous label's longer
            // one is how a shorter string produces an index out of range inside
            // the engine.
            nasij.Clear();
            int adadRuus = natija.AdadRuus;
            // Six indices per glyph: the field counts indices, not triangles.
            int adadFahras = natija.AdadMuthallathat;
            nasij.SetVertices(makhzan.Ruus, 0, adadRuus);
            nasij.SetUVs(0, makhzan.Malamis, 0, adadRuus);
            nasij.SetColors(makhzan.Alwan, 0, adadRuus);
            nasij.SetIndices(
                makhzan.Muthallathat, 0, adadFahras, MeshTopology.Triangles, 0, false);

            MustatilRasm hudud = natija.Hudud;
            float markazS = hudud.Yasar + (hudud.Ard * 0.5f);
            float markazA = hudud.Asfal + (hudud.Irtifa * 0.5f);
            nasij.bounds = new Bounds(
                new Vector3(markazS, markazA, 0f),
                new Vector3(hudud.Ard, hudud.Irtifa, 0f));
        }

        /// <summary>
        /// Lays the patch's translation out at a size the compiler never
        /// measured, using the constraint the compiler recorded for the slot.
        /// </summary>
        /// <remarks>
        /// The available width comes from the constraint and never from the
        /// component. A world-space label has no rectangle at all, so the only
        /// honest source for a wrapping width is the one the compiler measured
        /// the original in; where the constraint declares none, the text is one
        /// line of whatever width it needs, which is what a TextMesh with no
        /// explicit line breaks draws.
        /// </remarks>
        private bool Khattit(
            TextMesh mujassam,
            int fahras,
            float hajm,
            bool lahuQayd,
            // scoped: the spans handed back come from the layout buffer, never
            // from the constraint entry, so the caller may pass a local.
            scoped in MadkhalQayd qayd,
            out ReadOnlySpan<TaaribHarf> huruf,
            out ReadOnlySpan<TaaribSatr> sutur,
            out ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            out float ardTakhtit,
            out float irtifaTakhtit,
            out float hajmFili)
        {
            huruf = ReadOnlySpan<TaaribHarf>.Empty;
            sutur = ReadOnlySpan<TaaribSatr>.Empty;
            nitaqat = ReadOnlySpan<TaaribNitaqUslub>.Empty;
            ardTakhtit = 0f;
            irtifaTakhtit = 0f;
            hajmFili = hajm;

            Takhtit? buffer = takhtit;
            MaqbadSiyaq? qab = siyaq;
            MaqbadSilsila? chain = silsila;
            if (buffer is null || qab is null || chain is null)
            {
                BallighTakhtit();
                return false;
            }

            // A TextMesh with richText on parses Unity's legacy dialect and
            // nothing else: <b>, <i>, <size>, <color>, <material> and <quad>.
            // It is emphatically not TextMeshPro's, and parsing it as such
            // would find a <sprite=3> where this component draws eleven
            // characters a player is meant to read.
            NawNasq lahja = mujassam.richText ? NawNasq.WajihatUnity : NawNasq.Bila;
            if (!muhaddar.Hayyi(
                ruqaa, fahras, makhzan, lahja, hajm,
                out ReadOnlySpan<char> nass, out ReadOnlySpan<TaaribNitaqUslub> nitaqatNass))
            {
                return false;
            }

            KhiyaratTakhtit khiyarat = default;
            float ardTalab = 0f;
            float irtifaTalab = 0f;
            if (lahuQayd)
            {
                khiyarat = qayd.Khiyarat();
                ardTalab = qayd.ArdMutah;
                irtifaTalab = qayd.IrtifaMutah;
                if (qayd.HajmTilqai && qayd.Hajm > 0f)
                {
                    hajmFili = qayd.Hajm;
                }
            }

            try
            {
                TalabTakhtit talab = default;
                talab.Nass = nass;
                talab.Nitaqat = nitaqatNass;
                talab.Sifat = ReadOnlySpan<TaaribSifa>.Empty;
                talab.Hajm = hajmFili;
                talab.ArdMutah = ardTalab;
                talab.IrtifaMutah = irtifaTalab;
                talab.Khiyarat = khiyarat;
                buffer.Khattit(qab, chain, talab);
            }
            catch (KhataTaarib khata)
            {
                // Attributable to this one string and these parameters, so the
                // takeover stays installed and the next label still draws.
                Rabt.Ballagh("TextMesh: " + khata.Arabi + " | " + khata.Injilizi);
                return false;
            }

            huruf = buffer.Huruf;
            sutur = buffer.Sutur;
            nitaqat = nitaqatNass;
            ardTakhtit = buffer.Ard;
            irtifaTakhtit = buffer.Irtifa;
            hajmFili = buffer.Hajm;
            return !huruf.IsEmpty;
        }

        /// <summary>
        /// Reports a label whose glyphs need more than one atlas page, once.
        /// One MeshRenderer with one material draws one page, so the glyphs on
        /// the others cannot be drawn from here; the fix is a compiler-side
        /// page budget and not anything a player can act on.
        /// </summary>
        private void BallighSafahat()
        {
            if (ballaghSafahat)
            {
                return;
            }
            ballaghSafahat = true;
            Rabt.Ballagh(
                "حروف بعض النصوص المجسّمة موزَّعة على أكثر من صفحة أطلس، والراسم الواحد يربط صفحة واحدة؛ تُرسم حروف الصفحة الأولى وتغيب البقية. | "
                + "Some world-space labels have glyphs on more than one atlas page and one "
                + "renderer binds one page; the glyphs on the first page are drawn and the "
                + "rest are missing from the labels that need them.");
        }

        /// <summary>
        /// Reports, once, that runtime layout is unavailable. World-space
        /// labels feel this more than any other text system: a TextMesh's size
        /// is authored per object, so two signposts in one scene rarely agree
        /// and the compiler cannot have measured every size in advance.
        /// </summary>
        private void BallighTakhtit()
        {
            if (ballaghTakhtit)
            {
                return;
            }
            ballaghTakhtit = true;
            Rabt.Ballagh(
                "لا يوجد تخطيط زمن تشغيل للنصوص المجسّمة، فالنصوص التي لم يقِسها المترجم بهذا الحجم تبقى بلغة اللعبة. | "
                + "No runtime layout is available to the world-space takeover, so a string "
                + "the compiler did not measure at the size a label draws at stays in the "
                + "game's original language.");
        }

        /// <summary>The pivot's horizontal position for one anchor.</summary>
        private static float MihwarS(TextAnchor markaz)
        {
            switch (markaz)
            {
                case TextAnchor.UpperCenter:
                case TextAnchor.MiddleCenter:
                case TextAnchor.LowerCenter:
                    return 0.5f;
                case TextAnchor.UpperRight:
                case TextAnchor.MiddleRight:
                case TextAnchor.LowerRight:
                    return 1f;
                default:
                    return 0f;
            }
        }

        /// <summary>
        /// The pivot's vertical position for one anchor, in Unity's own order
        /// where one is the top edge — which is the order
        /// <see cref="HayyizRasm.MihwarA"/> uses, so nothing is inverted twice.
        /// </summary>
        private static float MihwarA(TextAnchor markaz)
        {
            switch (markaz)
            {
                case TextAnchor.MiddleLeft:
                case TextAnchor.MiddleCenter:
                case TextAnchor.MiddleRight:
                    return 0.5f;
                case TextAnchor.LowerLeft:
                case TextAnchor.LowerCenter:
                case TextAnchor.LowerRight:
                    return 0f;
                default:
                    return 1f;
            }
        }

        /// <summary>
        /// Unity's colour as the packed byte order Nasij writes. Named rather
        /// than open coded because the implicit conversion clamps and rounds,
        /// and doing that in two places is how two call sites come to disagree
        /// about a colour by one unit.
        /// </summary>
        private static uint LawnMuazzam(Color lawn)
        {
            Color32 mahzum = lawn;
            return ((uint)mahzum.r << 24) | ((uint)mahzum.g << 16)
                | ((uint)mahzum.b << 8) | mahzum.a;
        }

    }

    /// <summary>
    /// حرس النص المجسّم — the per-label guard: it watches one
    /// <see cref="TextMesh"/>, and while the installed patch covers that
    /// label's text it draws Taarib's mesh in place of the engine's.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Why a behaviour and not a patch.</b> Every member of
    /// <see cref="TextMesh"/> is an extern property with no IL body, so there
    /// is no managed decision point for HarmonyX to prefix and no callback that
    /// fires when the game changes the text. Polling in
    /// <see cref="LateUpdate"/> is the only mechanism the component offers, and
    /// it is <c>LateUpdate</c> rather than <c>Update</c> because a game's own
    /// script sets the text in <c>Update</c>, and a guard that ran first would
    /// draw one frame behind for the whole session.
    /// </para>
    /// <para>
    /// <b>What it does to the component it guards.</b> Nothing destructive, and
    /// nothing the game can observe through its own API. The text is left
    /// exactly as the game set it, so a script that reads it back gets its own
    /// string; what is switched off is the <see cref="MeshRenderer"/> the
    /// engine feeds, and Taarib's geometry is drawn from a child object with
    /// its own filter and renderer. <see cref="Ruddi"/> puts the renderer back
    /// and hides the child, which is what makes disabling the patch at runtime
    /// a return to the game's own rendering rather than an approximation of it.
    /// </para>
    /// </remarks>
    [DisallowMultipleComponent]
    public sealed class HarsMujassam : MonoBehaviour
    {
        private const string IsmTabi = "Taarib.NassMujassam";

        private NizamMujassam? nizam;
        private TextMesh? mujassam;
        private MeshRenderer? rassamAsli;
        private GameObject? tabi;
        private MeshFilter? murashshih;
        private MeshRenderer? rassam;
        private Mesh? nasij;

        private string? akhirNass;
        private int akhirHajm;
        private float akhirQiyas;
        private TextAnchor akhirMarkaz;
        private Color akhirLawn;
        private int addad;
        private bool marsum;
        private bool ballagh;

        /// <summary>Whether this guard is currently drawing its label.</summary>
        public bool Marsum => marsum;

        /// <summary>The label this guard watches.</summary>
        public TextMesh? Mujassam => mujassam;

        /// <summary>
        /// Binds the guard to a takeover and a label. Called by
        /// <see cref="NizamMujassam.Sajjil"/> immediately after the component
        /// is added, and again on a re-scan, which is why it is idempotent.
        /// </summary>
        /// <param name="nizam">The takeover that supplies the patch and the atlas.</param>
        /// <param name="mujassam">The label to watch.</param>
        public void Wassil(NizamMujassam nizam, TextMesh mujassam)
        {
            this.nizam = nizam;
            this.mujassam = mujassam;
            rassamAsli = mujassam != null ? mujassam.GetComponent<MeshRenderer>() : null;
            akhirNass = null;
            addad = 0;
        }

        /// <summary>
        /// Returns the label to the engine's own renderer and hides Taarib's
        /// child object. Leaves the guard attached and harmless.
        /// </summary>
        public void Ruddi()
        {
            if (marsum)
            {
                marsum = false;
                nizam?.Ahsi(false);
            }
            if (rassamAsli != null)
            {
                rassamAsli.enabled = true;
            }
            if (rassam != null)
            {
                rassam.enabled = false;
            }
            akhirNass = null;
        }

        /// <summary>
        /// The per-frame path: read the label's text on the configured cadence
        /// and rebuild only when something that affects the geometry changed.
        /// </summary>
        /// <remarks>
        /// The read of <see cref="TextMesh.text"/> is the one managed
        /// allocation anywhere on a frame in this assembly, and it is named on
        /// <see cref="NizamMujassam"/> along with the reason there is no way to
        /// avoid it. Everything after the comparison is skipped on a frame
        /// where nothing changed, which is nearly all of them: a signpost's
        /// text changes when the player earns a level, not sixty times a
        /// second.
        /// </remarks>
        private void LateUpdate()
        {
            NizamMujassam? hali = nizam;
            TextMesh? hadaf = mujassam;
            if (hali is null || hadaf == null)
            {
                return;
            }
            if (!hali.Amil)
            {
                if (marsum)
                {
                    Ruddi();
                }
                return;
            }

            int fatra = hali.FatratFahs < 1 ? 1 : hali.FatratFahs;
            addad++;
            if (addad < fatra)
            {
                return;
            }
            addad = 0;

            string nass;
            try
            {
                nass = hadaf.text ?? string.Empty;
            }
            catch (Exception khata)
            {
                Balligh("reading a label's text threw", khata);
                return;
            }

            bool thabit = string.Equals(nass, akhirNass, StringComparison.Ordinal);
            if (marsum && thabit
                && hadaf.fontSize == akhirHajm
                && hadaf.characterSize == akhirQiyas
                && hadaf.anchor == akhirMarkaz
                && hadaf.color == akhirLawn)
            {
                return;
            }
            if (!marsum && thabit && akhirNass is not null)
            {
                // Already declined this exact string. Re-deciding every poll
                // would hash it again for an answer that cannot have changed.
                return;
            }

            akhirNass = nass;
            akhirHajm = hadaf.fontSize;
            akhirQiyas = hadaf.characterSize;
            akhirMarkaz = hadaf.anchor;
            akhirLawn = hadaf.color;

            try
            {
                Bani();
                if (nasij is not null && hali.Irsim(hadaf, nass, nasij, out bool minRuqaa))
                {
                    Madda(hali, minRuqaa);
                    Ashhir(true);
                    return;
                }
                Ashhir(false);
                hali.AhsiMatruka();
            }
            catch (Exception khata)
            {
                Balligh("drawing a label threw", khata);
                Ruddi();
            }
        }

        /// <summary>
        /// Creates the child object, its filter, its renderer and its mesh, the
        /// first time this guard has something to draw. Nothing is created for
        /// a label whose text the patch never covers, which is most of them in
        /// most games.
        /// </summary>
        private void Bani()
        {
            TextMesh? hadaf = mujassam;
            if (hadaf == null || tabi != null)
            {
                return;
            }

            tabi = new GameObject(IsmTabi);
            tabi.hideFlags = HideFlags.DontSave;
            tabi.layer = gameObject.layer;
            Transform mihwar = tabi.transform;
            mihwar.SetParent(hadaf.transform, worldPositionStays: false);
            mihwar.localPosition = new Vector3(0f, 0f, hadaf.offsetZ);
            mihwar.localRotation = Quaternion.identity;
            mihwar.localScale = Vector3.one;

            nasij = new Mesh();
            nasij.name = IsmTabi;
            nasij.hideFlags = HideFlags.DontSave;

            // The mesh is rewritten whenever the string changes, which for a
            // quest marker or a damage number is often. Marking it dynamic
            // tells the driver to keep it in memory it expects to be rewritten
            // rather than in memory it expects to upload once.
            nasij.MarkDynamic();

            murashshih = tabi.AddComponent<MeshFilter>();
            murashshih.sharedMesh = nasij;
            rassam = tabi.AddComponent<MeshRenderer>();
            rassam.enabled = false;

            if (rassamAsli != null)
            {
                // Draw order, shadows and probes come from the renderer this
                // one stands in for. A label that sorted in front of a
                // billboard before the takeover has to sort in front of it
                // afterwards, and copying five values is cheaper than
                // explaining to a user why one sign now renders behind a wall.
                rassam.sortingLayerID = rassamAsli.sortingLayerID;
                rassam.sortingOrder = rassamAsli.sortingOrder;
                rassam.shadowCastingMode = rassamAsli.shadowCastingMode;
                rassam.receiveShadows = rassamAsli.receiveShadows;
                rassam.lightProbeUsage = rassamAsli.lightProbeUsage;
            }
        }

        /// <summary>
        /// Binds the atlas material the glyphs were drawn from. Assigned only
        /// when it changes, because writing <c>sharedMaterial</c> on a renderer
        /// invalidates its batch even when the value is identical.
        /// </summary>
        private void Madda(NizamMujassam hali, bool minRuqaa)
        {
            if (rassam == null)
            {
                return;
            }
            Material? madda = hali.Madda(minRuqaa);
            if (madda != null && !ReferenceEquals(rassam.sharedMaterial, madda))
            {
                rassam.sharedMaterial = madda;
            }
        }

        /// <summary>
        /// Switches between Taarib's renderer and the engine's, and keeps the
        /// takeover's counters honest.
        /// </summary>
        /// <param name="yarsum">Whether Taarib's mesh is what should be seen.</param>
        private void Ashhir(bool yarsum)
        {
            if (rassam != null)
            {
                rassam.enabled = yarsum;
            }
            if (rassamAsli != null)
            {
                rassamAsli.enabled = !yarsum;
            }
            if (yarsum != marsum)
            {
                marsum = yarsum;
                nizam?.Ahsi(yarsum);
            }
        }

        /// <summary>
        /// Reports a failure once per guard. Once, because a guard that throws
        /// on one frame throws on the next, and a log line per label per poll
        /// is a log a user cannot read and a disk a user did not agree to fill.
        /// </summary>
        private void Balligh(string sabab, Exception khata)
        {
            if (ballagh)
            {
                return;
            }
            ballagh = true;
            Rabt.Ballagh(
                "TextMesh: " + sabab + " on '" + name + "'; that label is returned to the "
                + "engine and every other one is unaffected", khata);
        }

        private void OnDestroy()
        {
            if (rassamAsli != null)
            {
                rassamAsli.enabled = true;
            }
            if (tabi != null)
            {
                UnityEngine.Object.Destroy(tabi);
                tabi = null;
            }
            if (nasij != null)
            {
                UnityEngine.Object.Destroy(nasij);
                nasij = null;
            }
            murashshih = null;
            rassam = null;
            nizam = null;
            mujassam = null;
        }
    }
}
