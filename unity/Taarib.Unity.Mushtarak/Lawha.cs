// لوحة — atlas residency: which pages exist, which bytes changed, and what an
// adapter has to upload before the next draw.
//
// The native side rasterizes glyphs into single-channel pages and hands back
// TaaribSafha structs — a pointer, a length, a width, a height, one byte per
// texel, row-major from the top, no row padding. Somebody has to get those
// bytes onto the GPU and keep them there as the atlas grows. This file is the
// half of that job which does not depend on a graphics API.
//
// WHY THE UPLOAD ITSELF IS NOT HERE. This assembly may not name a single
// UnityEngine type. Under Mono, Texture2D is a type from the game's own
// UnityEngine.CoreModule; under IL2CPP it reaches managed code as an
// Il2CppInterop-generated proxy with a different assembly identity. An assembly
// that named either could not load in the other, so the takeover would have to
// be written twice and the two copies would drift. Instead this file owns every
// decision and every piece of bookkeeping, and each adapter owns the four or
// five calls that touch its runtime's binding. What crosses the seam is
// SijillRafa: a plain value describing what to upload where.
//
// THE DECISIONS, AND WHY THEY ARE NOT THE ADAPTER'S TO MAKE.
//
//   Single channel, eight bits. A glyph image is coverage or a distance, both
//   scalar. Uploading it as RGBA would cost four times the memory and the
//   bandwidth for three channels that are copies of the first.
//
//   No mipmaps. Text is drawn at the size it was rasterized at, so a mip chain
//   is never sampled from except when something has already gone wrong — and
//   generating one blurs small text, because a 12-pixel glyph's first mip is
//   6 pixels of mush that trilinear filtering then blends back in.
//
//   No compression. DXT and its relatives quantize in 4x4 blocks. A hinted stem
//   is one or two texels wide, so block compression puts the error exactly
//   where the eye is looking, and the artefact reads as colour fringing along
//   vertical strokes — which in Arabic is most of the strokes.
//
//   Clamp, not repeat. A glyph at the edge of a page must not sample the glyph
//   on the opposite edge. With repeat addressing and bilinear filtering it
//   would, by half a texel, and the result is a faint ghost of an unrelated
//   letter along one side of a character.
//
//   Point or bilinear is the ADAPTER's choice, because it depends on whether
//   the game renders at integer scale. That one is exposed rather than decided.
//
// DIRTY REGIONS. When jisr rasterizes a glyph the atlas did not hold, one small
// rectangle of one page changed. Re-uploading a 4096-square page for it is 16 MB
// of PCIe traffic in the middle of a frame, and a game that meets a new glyph
// every few seconds — which is every game with a dialogue system — would do that
// continually. So this file tracks the changed rectangle per page, unions
// successive changes, and hands the adapter the smallest rectangle that covers
// them. Uploading a sub-rectangle costs what the sub-rectangle costs.
//
// The rectangle that goes up is the glyph's own plus the gutter the packer
// reserved around it, because that gutter is what a bilinear tap at the glyph's
// edge reads and it is only zero on the GPU if somebody sent the zeros. The
// gutter width is not this file's to know — it is what the atlas was created
// with — so it is handed in at construction rather than assumed, and the same
// number goes to `taarib_lawha_insha` and to `Lawha` from one place in each
// adapter.
//
// And a rectangle collected is not yet a rectangle sent. Looking a glyph up is
// not the same event as writing one: a menu that has been on screen for ten
// minutes looks every glyph it draws up on every frame, and dirtying each one
// would upload the union of a screenful of them sixty times a second for a page
// nothing has touched. Rasterizing, evicting and opening a page are the only
// three things that change a texel and the atlas counts all three, so the
// rectangles are held aside and promoted only when one of those counters has
// moved. A frame that rasterized nothing uploads nothing.
//
// The union is deliberately a bounding box rather than a list of rectangles.
// Two glyphs rasterized into opposite corners of a page produce a bounding box
// covering the whole page, which is worse than uploading two small rectangles —
// but tracking a list means allocating one, merging it, and deciding when to
// collapse it, on a path that runs inside somebody's game. The packer fills
// pages in rows, so successive glyphs are adjacent in practice and the bounding
// box stays small. This is stated rather than hidden because it is a real
// trade-off and a reader should be able to see it was made on purpose.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// The texture format an atlas page needs, as this assembly can express it
    /// without naming a Unity type.
    /// </summary>
    /// <remarks>
    /// One member today, and it is still an enumeration rather than an implied
    /// constant: the adapter's mapping from this to its runtime's real format
    /// enum is the place the decision is applied, and a mapping with nothing to
    /// switch on is a mapping that quietly does the wrong thing when a second
    /// format appears.
    /// </remarks>
    public enum SighatLawha
    {
        /// <summary>
        /// One unsigned byte per texel. Maps to <c>TextureFormat.R8</c> under
        /// Unity, or <c>Alpha8</c> on the older runtimes that lack it — both are
        /// eight bits in one channel and the shader is told which it got.
        /// </summary>
        Bayt8 = 0,
    }

    /// <summary>How a page's texels should be sampled.</summary>
    /// <remarks>
    /// The one sampling decision left to the adapter, because it depends on
    /// something this assembly cannot see: whether the game draws its interface
    /// at an integer scale. At integer scale, point sampling is exactly right
    /// and bilinear softens every stem by a fraction of a texel. At fractional
    /// scale the reverse is true and point sampling produces stems that shimmer
    /// as the element moves.
    /// </remarks>
    public enum TasfiyatLawha
    {
        /// <summary>Nearest texel. Correct when the interface is drawn at integer scale.</summary>
        Nuqta = 0,

        /// <summary>Bilinear. Correct when it is not.</summary>
        Thunai = 1,
    }

    /// <summary>A rectangle of texels inside one atlas page.</summary>
    /// <remarks>
    /// Half-open on the right and bottom: <see cref="Ard"/> and
    /// <see cref="Irtifa"/> are counts, not inclusive coordinates. Stated
    /// because an off-by-one here uploads a row short and leaves a line of stale
    /// texels along the bottom of every glyph in the region.
    /// </remarks>
    public readonly struct MustatilLawha : IEquatable<MustatilLawha>
    {
        /// <summary>Builds a rectangle.</summary>
        /// <param name="s">Left edge, in texels from the page's left.</param>
        /// <param name="a">Top edge, in texels from the page's top.</param>
        /// <param name="ard">Width in texels.</param>
        /// <param name="irtifa">Height in texels.</param>
        public MustatilLawha(int s, int a, int ard, int irtifa)
        {
            S = s;
            A = a;
            Ard = ard;
            Irtifa = irtifa;
        }

        /// <summary>Left edge, in texels from the page's left.</summary>
        public int S { get; }

        /// <summary>Top edge, in texels from the page's top.</summary>
        public int A { get; }

        /// <summary>Width in texels.</summary>
        public int Ard { get; }

        /// <summary>Height in texels.</summary>
        public int Irtifa { get; }

        /// <summary>Whether the rectangle covers no texels at all.</summary>
        public bool Khali => Ard <= 0 || Irtifa <= 0;

        /// <summary>How many texels it covers.</summary>
        public long Adad => Khali ? 0L : (long)Ard * Irtifa;

        /// <summary>An empty rectangle, which is what a clean page reports.</summary>
        public static MustatilLawha Faragh => new MustatilLawha(0, 0, 0, 0);

        /// <summary>The smallest rectangle covering both.</summary>
        /// <param name="akhar">The other rectangle.</param>
        /// <returns>Their bounding box, or whichever is non-empty when one is not.</returns>
        public MustatilLawha Ittihad(MustatilLawha akhar)
        {
            if (Khali)
            {
                return akhar;
            }
            if (akhar.Khali)
            {
                return this;
            }
            int s = Math.Min(S, akhar.S);
            int a = Math.Min(A, akhar.A);
            int yameen = Math.Max(S + Ard, akhar.S + akhar.Ard);
            int asfal = Math.Max(A + Irtifa, akhar.A + akhar.Irtifa);
            return new MustatilLawha(s, a, yameen - s, asfal - a);
        }

        /// <summary>
        /// The rectangle grown by a gutter on every side and clipped to a page.
        /// </summary>
        /// <param name="hashw">The gutter, in texels.</param>
        /// <param name="ard">The page's width in texels.</param>
        /// <param name="irtifa">The page's height in texels.</param>
        /// <returns>The grown rectangle, clipped to the page.</returns>
        /// <remarks>
        /// A bilinear tap at the edge of a glyph's rectangle reads one texel
        /// outside it, and the packer reserves exactly that much around every
        /// glyph and keeps it at zero. The zeros are on the CPU side; they reach
        /// the GPU only if the upload carries them. An upload of the glyph's own
        /// texels alone leaves the gutter holding whatever the letter that
        /// occupied that space before this one put there, and the symptom is a
        /// sliver of an unrelated letter along one edge of this one — the exact
        /// defect the gutter exists to prevent, arriving through the uploader.
        /// </remarks>
        public MustatilLawha Wassi(int hashw, int ard, int irtifa)
        {
            if (Khali || hashw <= 0 || ard <= 0 || irtifa <= 0)
            {
                return this;
            }
            int s = S - hashw;
            int a = A - hashw;
            s = s < 0 ? 0 : s;
            a = a < 0 ? 0 : a;
            int yameen = S + Ard + hashw;
            int asfal = A + Irtifa + hashw;
            yameen = yameen > ard ? ard : yameen;
            asfal = asfal > irtifa ? irtifa : asfal;
            if (yameen <= s || asfal <= a)
            {
                return this;
            }
            return new MustatilLawha(s, a, yameen - s, asfal - a);
        }

        /// <summary>Whether this rectangle lies entirely inside a page of the given size.</summary>
        /// <param name="ard">The page's width in texels.</param>
        /// <param name="irtifa">The page's height in texels.</param>
        /// <returns>Whether it fits.</returns>
        public bool Dakhil(int ard, int irtifa)
        {
            if (Khali)
            {
                return true;
            }
            return S >= 0 && A >= 0 && Ard > 0 && Irtifa > 0
                && S <= ard - Ard && A <= irtifa - Irtifa;
        }

        /// <inheritdoc/>
        public bool Equals(MustatilLawha akhar)
        {
            return S == akhar.S && A == akhar.A && Ard == akhar.Ard && Irtifa == akhar.Irtifa;
        }

        /// <inheritdoc/>
        public override bool Equals(object? obj)
        {
            return obj is MustatilLawha akhar && Equals(akhar);
        }

        /// <inheritdoc/>
        public override int GetHashCode()
        {
            unchecked
            {
                int h = S;
                h = (h * 397) ^ A;
                h = (h * 397) ^ Ard;
                h = (h * 397) ^ Irtifa;
                return h;
            }
        }

        /// <summary>Whether two rectangles cover the same texels.</summary>
        /// <param name="awwal">The first.</param>
        /// <param name="thani">The second.</param>
        /// <returns>Whether they are equal.</returns>
        public static bool operator ==(MustatilLawha awwal, MustatilLawha thani)
        {
            return awwal.Equals(thani);
        }

        /// <summary>Whether two rectangles differ.</summary>
        /// <param name="awwal">The first.</param>
        /// <param name="thani">The second.</param>
        /// <returns>Whether they are unequal.</returns>
        public static bool operator !=(MustatilLawha awwal, MustatilLawha thani)
        {
            return !awwal.Equals(thani);
        }

        /// <inheritdoc/>
        public override string ToString()
        {
            return $"{Ard}x{Irtifa}+{S}+{A}";
        }
    }

    /// <summary>
    /// What one page needs uploaded, as a value the adapter reads and acts on.
    /// </summary>
    /// <remarks>
    /// Deliberately a value type with no reference to the atlas: an adapter
    /// takes a copy of this, makes its graphics calls, and reports back. Holding
    /// a reference to the atlas across a graphics call would mean the atlas
    /// could grow underneath the upload, and the page being read would be a page
    /// that had been reallocated.
    /// </remarks>
    public readonly struct SijillRafa
    {
        /// <summary>Records what one page needs.</summary>
        /// <param name="safha">Its index in the atlas.</param>
        /// <param name="ard">The page's width in texels.</param>
        /// <param name="irtifa">The page's height in texels.</param>
        /// <param name="wasikh">The rectangle whose texels changed.</param>
        /// <param name="jadida">Whether the page did not exist at the last upload.</param>
        public SijillRafa(int safha, int ard, int irtifa, MustatilLawha wasikh, bool jadida)
        {
            Safha = safha;
            Ard = ard;
            Irtifa = irtifa;
            Wasikh = wasikh;
            Jadida = jadida;
        }

        /// <summary>The page's index in the atlas.</summary>
        public int Safha { get; }

        /// <summary>The page's width in texels.</summary>
        public int Ard { get; }

        /// <summary>The page's height in texels.</summary>
        public int Irtifa { get; }

        /// <summary>The rectangle whose texels changed since the last upload.</summary>
        public MustatilLawha Wasikh { get; }

        /// <summary>
        /// Whether this page did not exist at the last upload, so the adapter
        /// must create a texture for it before writing anything into it.
        /// </summary>
        /// <remarks>
        /// When this is set, <see cref="Wasikh"/> covers the whole page: a
        /// freshly created texture's contents are undefined, so uploading only
        /// the part the packer has filled would leave the rest as whatever the
        /// driver handed back, and glyphs packed into it later would be composited
        /// over garbage until they were themselves uploaded.
        /// </remarks>
        public bool Jadida { get; }
    }

    /// <summary>
    /// جدول الصفحات — which pages exist, how large each one is, and which of
    /// their texels the GPU's copy is behind on.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Separated from <see cref="Lawha"/> because every decision here is
    /// arithmetic over page indices and rectangles, and <see cref="Lawha"/>'s
    /// other half is a native handle. A test that wanted to count what one
    /// frame uploads would otherwise need a loaded <c>taarib_jisr</c> and a
    /// rasterizer to ask a question that involves neither, so the part that can
    /// be checked on a build machine is the part that is reachable from one.
    /// </para>
    /// <para>
    /// A page is registered with <see cref="Qayyid"/>, dirtied with
    /// <see cref="Wassikh"/>, collected with <see cref="Iltaqit"/> and marked
    /// clean with <see cref="Rufia"/> — in that order, once per frame.
    /// </para>
    /// </remarks>
    public sealed class JadwalSafahat
    {
        /// <summary>
        /// The largest page dimension this bookkeeping will accept, in texels.
        /// </summary>
        /// <remarks>
        /// Four thousand and ninety-six, which is the packer's own maximum and
        /// the largest texture dimension every graphics API the product targets
        /// guarantees. A page larger than this is not a page this build wrote,
        /// and accepting it would mean trusting a number to size a rectangle
        /// that an adapter is about to hand to a driver.
        /// </remarks>
        public const int AqsaBud = 4096;

        /// <summary>
        /// The most pages this bookkeeping will track.
        /// </summary>
        /// <remarks>
        /// Sixty-four. The packer's documented maximum for the default profile
        /// is eight, and a patch with sixty-four full pages would be a gigabyte
        /// of atlas — so this is a ceiling on a hostile number rather than a
        /// limit anything real approaches. It is also the width of the page
        /// mask a mesh build reports, which is why the two numbers are one.
        /// </remarks>
        public const int AqsaSafahat = 64;

        private readonly MustatilLawha[] wasikh = new MustatilLawha[AqsaSafahat];
        private readonly MustatilLawha[] itar = new MustatilLawha[AqsaSafahat];
        private readonly bool[] jadida = new bool[AqsaSafahat];
        private readonly int[] abaad = new int[AqsaSafahat * 2];
        private readonly int hashw;
        private int adadSafahat;
        private ulong jeel;

        /// <summary>Builds an empty page table.</summary>
        /// <param name="hashw">
        /// The gutter the packer leaves around every glyph, in texels — the
        /// same number the atlas was created with. See
        /// <see cref="MustatilLawha.Wassi"/> for what an upload that ignored it
        /// leaves on the screen.
        /// </param>
        public JadwalSafahat(ushort hashw)
        {
            this.hashw = hashw;
        }

        /// <summary>The gutter every dirty rectangle is grown by.</summary>
        public int Hashw => hashw;

        /// <summary>How many pages the table holds.</summary>
        public int AdadSafahat => adadSafahat;

        /// <summary>
        /// A counter that increases whenever anything about the table changed.
        /// </summary>
        public ulong Jeel => jeel;

        /// <summary>Whether anything at all needs uploading.</summary>
        public bool Muattal
        {
            get
            {
                for (int i = 0; i < adadSafahat; i++)
                {
                    if (jadida[i] || !wasikh[i].Khali)
                    {
                        return true;
                    }
                }
                return false;
            }
        }

        /// <summary>One page's width in texels, or zero for a page it has not seen.</summary>
        /// <param name="safha">The page index.</param>
        /// <returns>The width.</returns>
        public int Ard(int safha)
        {
            return (uint)safha < (uint)adadSafahat ? abaad[safha * 2] : 0;
        }

        /// <summary>One page's height in texels, or zero for a page it has not seen.</summary>
        /// <param name="safha">The page index.</param>
        /// <returns>The height.</returns>
        public int Irtifa(int safha)
        {
            return (uint)safha < (uint)adadSafahat ? abaad[(safha * 2) + 1] : 0;
        }

        /// <summary>
        /// Records a page's dimensions, marking it as needing a full upload the
        /// first time it is seen.
        /// </summary>
        /// <param name="safha">The page index.</param>
        /// <param name="ard">Its width in texels.</param>
        /// <param name="irtifa">Its height in texels.</param>
        /// <returns>Whether the page was accepted.</returns>
        /// <remarks>
        /// A page index past <see cref="AqsaSafahat"/>, or a dimension outside
        /// <c>1..=<see cref="AqsaBud"/></c>, is refused rather than recorded: a
        /// number this table believes becomes a rectangle an adapter hands to a
        /// driver.
        /// </remarks>
        public bool Qayyid(int safha, int ard, int irtifa)
        {
            if ((uint)safha >= (uint)AqsaSafahat
                || ard <= 0 || irtifa <= 0 || ard > AqsaBud || irtifa > AqsaBud)
            {
                return false;
            }
            if (safha >= adadSafahat)
            {
                for (int i = adadSafahat; i <= safha; i++)
                {
                    jadida[i] = true;
                    wasikh[i] = MustatilLawha.Faragh;
                }
                adadSafahat = safha + 1;
                jeel++;
            }
            abaad[safha * 2] = ard;
            abaad[(safha * 2) + 1] = irtifa;
            return true;
        }

        /// <summary>
        /// Records that one glyph was drawn from, so that the texels it and the
        /// gutter around it occupy are uploaded if anything wrote to the atlas.
        /// </summary>
        /// <param name="safha">The page the glyph sits on.</param>
        /// <param name="shakl">The glyph's own rectangle, gutter excluded.</param>
        /// <remarks>
        /// <para>
        /// The gutter is added here rather than by the caller because the caller
        /// is handed the glyph's rectangle by the native atlas and has no reason
        /// to know the packer reserved anything around it. One place adds it, so
        /// one place can be wrong about it.
        /// </para>
        /// <para>
        /// This does not itself make anything dirty: a glyph the atlas already
        /// held is the same image in the same rectangle, and a menu that has
        /// been on screen for ten minutes looks every one of its glyphs up on
        /// every frame. The rectangles collected here are candidates, and
        /// <see cref="Adrij"/> promotes them only when the atlas says it wrote
        /// something.
        /// </para>
        /// </remarks>
        public void Wassikh(int safha, MustatilLawha shakl)
        {
            if ((uint)safha >= (uint)adadSafahat || shakl.Khali)
            {
                return;
            }
            MustatilLawha mamsuh = shakl.Wassi(hashw, abaad[safha * 2], abaad[(safha * 2) + 1]);
            itar[safha] = itar[safha].Ittihad(mamsuh);
        }

        /// <summary>
        /// Settles the rectangles collected since the last call: dirty when the
        /// atlas wrote something, discarded when it did not.
        /// </summary>
        /// <param name="kutiba">
        /// Whether the atlas rasterized, evicted or grew since the last call.
        /// Nothing else writes a texel, so a run of lookups with all three
        /// counters unmoved cannot have changed a page — and the rectangles it
        /// collected name texels the GPU already holds.
        /// </param>
        /// <remarks>
        /// The gate is the whole of the frame-time fix. Without it every glyph
        /// looked up dirties its own rectangle, the union of a screen's worth of
        /// them approaches the page, and a scene that has not changed in minutes
        /// uploads it on every frame. With it, a frame that rasterized nothing
        /// uploads nothing.
        /// </remarks>
        public void Adrij(bool kutiba)
        {
            for (int i = 0; i < adadSafahat; i++)
            {
                MustatilLawha murashah = itar[i];
                itar[i] = MustatilLawha.Faragh;
                if (!kutiba || murashah.Khali)
                {
                    continue;
                }
                MustatilLawha sabiq = wasikh[i];
                MustatilLawha jadid = sabiq.Ittihad(murashah);
                if (jadid != sabiq)
                {
                    wasikh[i] = jadid;
                    jeel++;
                }
            }
        }

        /// <summary>
        /// Collects what needs uploading, without changing anything.
        /// </summary>
        /// <param name="hadaf">
        /// A buffer of at least <see cref="AdadSafahat"/> entries, which the
        /// caller owns and reuses. Supplied by the caller rather than allocated
        /// here so that collecting per frame costs nothing.
        /// </param>
        /// <returns>How many entries were written.</returns>
        /// <exception cref="ArgumentException">The buffer is too small.</exception>
        /// <remarks>
        /// Nothing is marked clean here. The adapter calls <see cref="Rufia"/>
        /// after its upload actually succeeded — because a graphics call that
        /// threw, or a texture that failed to allocate, must leave the page dirty
        /// so the next frame tries again. Marking clean on collection would lose
        /// the change permanently and the glyph would never appear.
        /// </remarks>
        public int Iltaqit(Span<SijillRafa> hadaf)
        {
            if (hadaf.Length < adadSafahat)
            {
                throw new ArgumentException(
                    $"The buffer holds {hadaf.Length} entries and the atlas has "
                    + $"{adadSafahat} pages.",
                    nameof(hadaf));
            }

            int adad = 0;
            for (int i = 0; i < adadSafahat; i++)
            {
                bool jadid = jadida[i];
                MustatilLawha mustatil = wasikh[i];
                if (!jadid && mustatil.Khali)
                {
                    continue;
                }
                int budS = abaad[i * 2];
                int budA = abaad[(i * 2) + 1];
                if (jadid)
                {
                    // A texture the adapter is about to create has undefined
                    // contents, so the whole page goes up, not just the part the
                    // packer has filled.
                    mustatil = new MustatilLawha(0, 0, budS, budA);
                }
                hadaf[adad] = new SijillRafa(i, budS, budA, mustatil, jadid);
                adad++;
            }
            return adad;
        }

        /// <summary>
        /// Marks one page clean after its upload succeeded.
        /// </summary>
        /// <param name="safha">The page index.</param>
        /// <remarks>
        /// Call this only when the upload actually completed. A page marked clean
        /// after a failed upload keeps the stale texels forever, and the glyph
        /// that was rasterized into it never appears — which looks like a missing
        /// character rather than a failed texture write, and is diagnosed
        /// accordingly and wrongly.
        /// </remarks>
        public void Rufia(int safha)
        {
            if ((uint)safha >= (uint)AqsaSafahat)
            {
                return;
            }
            wasikh[safha] = MustatilLawha.Faragh;
            jadida[safha] = false;
        }

        /// <summary>Marks every page as needing a full upload.</summary>
        /// <remarks>
        /// For the cases where the GPU-side copy was lost rather than made stale:
        /// a device reset, a scene load that destroyed the textures, or a
        /// resolution change that recreated them. Nothing about the atlas
        /// changed, so the dirty tracking would report nothing, and the adapter
        /// would draw from textures that no longer hold anything.
        /// </remarks>
        public void Ajjil()
        {
            for (int i = 0; i < adadSafahat; i++)
            {
                jadida[i] = true;
                wasikh[i] = new MustatilLawha(0, 0, abaad[i * 2], abaad[(i * 2) + 1]);
                // Whole pages are going up; a candidate rectangle inside one of
                // them has nothing left to add.
                itar[i] = MustatilLawha.Faragh;
            }
            jeel++;
        }

        /// <summary>
        /// The single atlas page a finished mesh build touched, if there is one.
        /// </summary>
        /// <param name="alamSafahat">
        /// The page mask a mesh build reports: bit <c>i</c> set means at least
        /// one glyph of that layout lives on page <c>i</c>.
        /// </param>
        /// <param name="safahatBaida">
        /// Whether the build met a glyph on a page the mask cannot name.
        /// </param>
        /// <param name="safha">The page to draw from, meaningful on success.</param>
        /// <returns>
        /// Whether every glyph of the layout is on one page. A layout that drew
        /// no glyph at all — a line of spaces — succeeds with page zero, because
        /// there is nothing to pick a page for.
        /// </returns>
        /// <remarks>
        /// A page is a texture and a texture is a draw call, so one mesh can
        /// only sample one page. The caller's choice is therefore between
        /// drawing the layout from the page it is on and leaving the whole
        /// string to the engine: drawing it from page zero regardless emits
        /// nothing for every glyph on any other page, which is a sentence with
        /// letters missing and no report that they are.
        /// </remarks>
        public static bool SafhaWahida(ulong alamSafahat, bool safahatBaida, out ushort safha)
        {
            safha = 0;
            if (safahatBaida)
            {
                return false;
            }
            if (alamSafahat == 0)
            {
                return true;
            }
            if ((alamSafahat & (alamSafahat - 1)) != 0)
            {
                return false;
            }
            ulong baqi = alamSafahat;
            int fahras = 0;
            while ((baqi & 1UL) == 0)
            {
                baqi >>= 1;
                fahras++;
            }
            safha = (ushort)fahras;
            return true;
        }
    }

    /// <summary>
    /// لوحة — the atlas's managed side: page bookkeeping, dirty tracking, and
    /// the format decisions each adapter applies.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Construct one around a <see cref="MaqbadLawha"/>, call
    /// <see cref="Sajjil"/> whenever a glyph has been rasterized, and call
    /// <see cref="Iltaqit"/> once per frame — before drawing — to collect what
    /// needs uploading. <see cref="Rufia"/> tells the atlas an upload succeeded
    /// so the page can be marked clean.
    /// </para>
    /// <para>
    /// Ownership: this type does NOT own the native handle. The handle's
    /// lifetime belongs to whoever created it — normally the plugin's root
    /// object, which outlives every adapter — and disposing this type releases
    /// only its own bookkeeping. Two lifetimes tangled into one is how an atlas
    /// comes to be freed while a second adapter is still drawing from it.
    /// </para>
    /// <para>
    /// Thread safety: none, deliberately. Every call belongs on the thread that
    /// draws. Locking a per-frame path to support a case that does not arise
    /// would cost every frame to protect none of them.
    /// </para>
    /// </remarks>
    public sealed class Lawha : IDisposable
    {
        /// <summary>
        /// The largest page dimension this bookkeeping will accept, in texels.
        /// </summary>
        public const int AqsaBud = JadwalSafahat.AqsaBud;

        /// <summary>
        /// The most pages this bookkeeping will track.
        /// </summary>
        public const int AqsaSafahat = JadwalSafahat.AqsaSafahat;

        private readonly MaqbadLawha maqbad;
        private readonly bool yamlikMaqbad;
        private readonly JadwalSafahat jadwal;
        private ulong kitabat;
        private bool mutlaf;

        /// <summary>
        /// Wraps an atlas handle for residency bookkeeping.
        /// </summary>
        /// <param name="maqbad">The atlas. Not owned unless <paramref name="yamlik"/>.</param>
        /// <param name="namat">How its pages were rasterized.</param>
        /// <param name="hashw">
        /// The gutter the atlas was created with, in texels — the same value
        /// that went to <see cref="MaqbadLawha.Insha"/>. It is what every dirty
        /// rectangle is grown by, so an uploader that sends only what changed
        /// still sends the zeros a bilinear tap at a glyph's edge reads.
        /// </param>
        /// <param name="yamlik">
        /// Whether disposing this object should also dispose the handle. False
        /// for the normal case, where the plugin root owns the atlas and several
        /// adapters read it.
        /// </param>
        /// <exception cref="ArgumentNullException"><paramref name="maqbad"/> is null.</exception>
        /// <exception cref="ArgumentException">The handle is already invalid.</exception>
        public Lawha(MaqbadLawha maqbad, NamatLawha namat, ushort hashw, bool yamlik = false)
        {
            if (maqbad is null)
            {
                throw new ArgumentNullException(nameof(maqbad));
            }
            if (maqbad.IsInvalid)
            {
                throw new ArgumentException(
                    "The atlas handle is already invalid.", nameof(maqbad));
            }
            this.maqbad = maqbad;
            yamlikMaqbad = yamlik;
            jadwal = new JadwalSafahat(hashw);
            Namat = namat;
            Tasfiya = TasfiyatLawha.Nuqta;
            Zamin();
        }

        /// <summary>How this atlas's pages were rasterized.</summary>
        /// <remarks>
        /// Coverage and a signed distance field upload identically — one byte per
        /// texel — and are drawn by different shaders. An adapter that uploaded
        /// one and drew it as the other would produce text that is legible in a
        /// screenshot and wrong in motion: the SDF shader's alpha test would
        /// threshold a coverage ramp, so stems would thin and thicken as the
        /// element moved. The mode travels with the pages for that reason.
        /// </remarks>
        public NamatLawha Namat { get; }

        /// <summary>The texture format every page of this atlas needs.</summary>
        public SighatLawha Sigha => SighatLawha.Bayt8;

        /// <summary>Whether the pages need mipmaps. Always false; see the file header.</summary>
        public bool Mustawayat => false;

        /// <summary>Whether the pages may be compressed. Always false; see the header.</summary>
        public bool Madghuta => false;

        /// <summary>How the pages should be sampled.</summary>
        /// <remarks>
        /// The one sampling decision the adapter makes, because it depends on
        /// whether the game draws its interface at an integer scale — which this
        /// assembly cannot see.
        /// </remarks>
        public TasfiyatLawha Tasfiya { get; set; }

        /// <summary>How many pages the atlas currently holds.</summary>
        public int AdadSafahat => jadwal.AdadSafahat;

        /// <summary>
        /// A counter that increases whenever anything about the atlas changed.
        /// </summary>
        /// <remarks>
        /// <para>
        /// What lets an adapter decide in one comparison whether its uploaded
        /// copy is stale, without walking the page table. Compare against the
        /// value held from the last upload; equal means nothing to do.
        /// </para>
        /// <para>
        /// Reading it settles the rectangles <see cref="Sajjil"/> has collected
        /// since the last read, which costs one call into the atlas for its
        /// counters. That is deliberate rather than incidental: the answer to
        /// "has anything changed" is the atlas's and not this object's, and an
        /// adapter that asked the question without it would be told yes on every
        /// frame that drew a glyph — which is every frame.
        /// </para>
        /// </remarks>
        public ulong Jeel
        {
            get
            {
                Adrij();
                return jadwal.Jeel;
            }
        }

        /// <summary>Whether anything at all needs uploading.</summary>
        /// <remarks>Settles the collected rectangles first, as <see cref="Jeel"/> does.</remarks>
        public bool Muattal
        {
            get
            {
                Adrij();
                return jadwal.Muattal;
            }
        }

        /// <summary>
        /// Looks a glyph image up, rasterizing it if the atlas does not hold it,
        /// and records any page growth or texel change that resulted.
        /// </summary>
        /// <param name="silsila">The font chain to rasterize from.</param>
        /// <param name="miftah">The glyph, size, font and subpixel bucket.</param>
        /// <param name="mawdi">Where the image sits.</param>
        /// <exception cref="KhataTaarib">The native side refused.</exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// This is the one method on the per-frame path, and it allocates
        /// nothing. The page count is only re-read when the returned page index
        /// is one this object has not seen, so the common case — a glyph already
        /// resident — costs one native call and one array write.
        /// </remarks>
        public void Sajjil(
            MaqbadSilsila silsila, in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
        {
            LazimHay();
            maqbad.Shakl(silsila, miftah, out mawdi);

            int safha = mawdi.Safha;
            if (safha >= jadwal.AdadSafahat)
            {
                // The lookup grew the atlas. Re-reading the page table is the
                // expensive path and it runs only when it actually happened.
                Zamin();
                if (safha >= jadwal.AdadSafahat)
                {
                    return;
                }
            }

            // A glyph with no image — a space — changed nothing, and marking its
            // page dirty would re-upload a page per space drawn.
            if (mawdi.Ard == 0 || mawdi.Irtifa == 0)
            {
                return;
            }

            MustatilLawha mustatil = new MustatilLawha(
                mawdi.S, mawdi.A, mawdi.Ard, mawdi.Irtifa);
            int budS = jadwal.Ard(safha);
            int budA = jadwal.Irtifa(safha);
            if (!mustatil.Dakhil(budS, budA))
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6101",
                    $"شكل مرسوم عند {mustatil} يتجاوز حدود صفحة اللوحة {safha} "
                    + $"({budS}×{budA}).",
                    $"A rasterized glyph at {mustatil} runs past the bounds of atlas page "
                    + $"{safha} ({budS}x{budA}).",
                    Khutwa.IadatTarkibIttar);
            }

            jadwal.Wassikh(safha, mustatil);
        }

        /// <summary>
        /// Collects what needs uploading, without changing anything.
        /// </summary>
        /// <param name="hadaf">
        /// A buffer of at least <see cref="AdadSafahat"/> entries, which the
        /// caller owns and reuses. Supplied by the caller rather than allocated
        /// here so that collecting per frame costs nothing.
        /// </param>
        /// <returns>How many entries were written.</returns>
        /// <exception cref="ArgumentException">The buffer is too small.</exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// Nothing is marked clean here. The adapter calls <see cref="Rufia"/>
        /// after its upload actually succeeded — because a graphics call that
        /// threw, or a texture that failed to allocate, must leave the page dirty
        /// so the next frame tries again. Marking clean on collection would lose
        /// the change permanently and the glyph would never appear.
        /// </remarks>
        public int Iltaqit(Span<SijillRafa> hadaf)
        {
            LazimHay();
            Adrij();
            return jadwal.Iltaqit(hadaf);
        }

        /// <summary>One page's texels, borrowed from the native atlas.</summary>
        /// <param name="safha">The page index.</param>
        /// <returns>
        /// Its texels: one byte each, row-major from the top, no row padding.
        /// </returns>
        /// <exception cref="ArgumentOutOfRangeException">The index names no page.</exception>
        /// <exception cref="KhataTaarib">The native side refused.</exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// The span points into memory the native atlas owns and stays valid only
        /// until the next call that can change the atlas — which is any call to
        /// <see cref="Sajjil"/>, or destroying the handle. Upload it and do not
        /// keep it. This is the same contract <c>taarib_lawha_safha</c> states,
        /// carried forward rather than softened, because softening it would mean
        /// copying every page.
        /// </remarks>
        public ReadOnlySpan<byte> Texelat(int safha)
        {
            LazimHay();
            if ((uint)safha >= (uint)jadwal.AdadSafahat)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(safha), safha, $"The atlas has {jadwal.AdadSafahat} pages.");
            }
            maqbad.Safha((ushort)safha, out TaaribSafha wasf);
            return wasf.Muhtawa();
        }

        /// <summary>
        /// The texels of one rectangle of one page, row by row.
        /// </summary>
        /// <param name="safha">The page index.</param>
        /// <param name="mustatil">The rectangle, which must lie inside the page.</param>
        /// <param name="hadaf">
        /// A buffer of at least <c>mustatil.Ard * mustatil.Irtifa</c> bytes.
        /// </param>
        /// <returns>How many bytes were written.</returns>
        /// <exception cref="ArgumentOutOfRangeException">The index names no page.</exception>
        /// <exception cref="ArgumentException">
        /// The rectangle runs outside the page, or the buffer is too small.
        /// </exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// Copies, and says so. An adapter whose graphics API can upload a
        /// sub-rectangle directly from a strided source should use
        /// <see cref="Texelat"/> and the rectangle instead; this exists for the
        /// APIs that insist on a contiguous buffer, and for those the copy is the
        /// cheapest thing available. It is still far cheaper than uploading the
        /// whole page: a 32x32 glyph is a kilobyte against sixteen megabytes.
        /// </remarks>
        public int Nasakh(int safha, MustatilLawha mustatil, Span<byte> hadaf)
        {
            ReadOnlySpan<byte> kull = Texelat(safha);
            int budS = jadwal.Ard(safha);
            int budA = jadwal.Irtifa(safha);
            if (!mustatil.Dakhil(budS, budA))
            {
                throw new ArgumentException(
                    $"The rectangle {mustatil} runs past a page of {budS}x{budA}.",
                    nameof(mustatil));
            }
            if (mustatil.Khali)
            {
                return 0;
            }
            long matlub = mustatil.Adad;
            if (hadaf.Length < matlub)
            {
                throw new ArgumentException(
                    $"The buffer holds {hadaf.Length} bytes and the rectangle needs {matlub}.",
                    nameof(hadaf));
            }

            int maktub = 0;
            for (int satr = 0; satr < mustatil.Irtifa; satr++)
            {
                int masdar = ((mustatil.A + satr) * budS) + mustatil.S;
                if (masdar < 0 || masdar > kull.Length - mustatil.Ard)
                {
                    break;
                }
                kull.Slice(masdar, mustatil.Ard).CopyTo(hadaf.Slice(maktub, mustatil.Ard));
                maktub += mustatil.Ard;
            }
            return maktub;
        }

        /// <summary>
        /// Marks one page clean after its upload succeeded.
        /// </summary>
        /// <param name="safha">The page index.</param>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// Call this only when the upload actually completed. A page marked clean
        /// after a failed upload keeps the stale texels forever, and the glyph
        /// that was rasterized into it never appears — which looks like a missing
        /// character rather than a failed texture write, and is diagnosed
        /// accordingly and wrongly.
        /// </remarks>
        public void Rufia(int safha)
        {
            LazimHay();
            jadwal.Rufia(safha);
        }

        /// <summary>Marks every page as needing a full upload.</summary>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// For the cases where the GPU-side copy was lost rather than made stale:
        /// a device reset, a scene load that destroyed the textures, or a
        /// resolution change that recreated them. Nothing about the atlas
        /// changed, so the dirty tracking would report nothing, and the adapter
        /// would draw from textures that no longer hold anything.
        /// </remarks>
        public void Ajjil()
        {
            LazimHay();
            jadwal.Ajjil();
        }

        /// <summary>
        /// Begins a frame: releases the pins the last frame took, so the evictor
        /// may reuse rectangles nothing on screen references any more.
        /// </summary>
        /// <exception cref="KhataTaarib">The native side refused.</exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        /// <remarks>
        /// <para>
        /// Call once per engine frame, before the frame's first
        /// <see cref="Sajjil"/>. Every glyph looked up after this call is pinned
        /// until the next one, which is what stops the evictor reassigning a
        /// rectangle a vertex buffer already points at.
        /// </para>
        /// <para>
        /// This does <b>not</b> touch the dirty tracking, and the distinction is
        /// the whole of a frame-time defect that shipped: beginning a frame used
        /// to call <see cref="Ajjil"/> as well, so every page of the atlas was
        /// re-uploaded in full — four megabytes per 2048-square page, through
        /// <c>LoadRawTextureData</c> on the main thread — on every frame that
        /// drew a single string, whether or not one texel had changed. Losing a
        /// frame's pins and losing a frame's upload state are different events
        /// with different causes, and only the first one happens every frame.
        /// </para>
        /// </remarks>
        public void Ibda()
        {
            LazimHay();
            maqbad.IbdaItar();
        }

        /// <summary>The atlas's own counters, from the native side.</summary>
        /// <returns>Hits, misses, evictions, growth events, bytes and budget.</returns>
        /// <exception cref="KhataTaarib">The native side refused.</exception>
        /// <exception cref="ObjectDisposedException">This object was disposed.</exception>
        public TaaribIhsaatLawha Ihsaat()
        {
            LazimHay();
            return maqbad.Ihsaat();
        }

        /// <summary>Releases this object's bookkeeping.</summary>
        /// <remarks>
        /// Disposes the native handle only if this object was constructed as its
        /// owner. In the normal arrangement it was not: the plugin root owns the
        /// atlas, several adapters read it, and the first adapter to unload must
        /// not take the atlas away from the others.
        /// </remarks>
        public void Dispose()
        {
            if (mutlaf)
            {
                return;
            }
            mutlaf = true;
            if (yamlikMaqbad)
            {
                maqbad.Dispose();
            }
        }

        /// <summary>
        /// Asks the atlas whether it has written anything since the last time,
        /// and settles the collected rectangles accordingly.
        /// </summary>
        /// <remarks>
        /// Rasterizing a glyph, reclaiming a rectangle and opening a page are
        /// the only three things that change a texel, and the atlas counts all
        /// three. A failure to read the counters is treated as "something was
        /// written": the cost of being wrong that way is one upload nobody
        /// needed, and the cost of being wrong the other way is a glyph that
        /// never appears.
        /// </remarks>
        private void Adrij()
        {
            if (mutlaf)
            {
                return;
            }
            ulong hali;
            try
            {
                TaaribIhsaatLawha ihsaat = maqbad.Ihsaat();
                hali = ihsaat.Ikhfaqat + ihsaat.Ikhlaat + ihsaat.AhdathNamu;
            }
            catch (KhataTaarib)
            {
                jadwal.Adrij(true);
                return;
            }
            catch (ObjectDisposedException)
            {
                // The context this atlas belongs to was closed underneath it,
                // which is teardown. Nothing will be uploaded again either way;
                // the conservative answer costs nothing here and keeps a
                // property from throwing during shutdown.
                jadwal.Adrij(true);
                return;
            }
            bool kutiba = hali != kitabat;
            kitabat = hali;
            jadwal.Adrij(kutiba);
        }

        /// <summary>Re-reads the page table from the native atlas.</summary>
        private void Zamin()
        {
            uint adad = maqbad.AdadSafahat();
            int hadd = adad > AqsaSafahat ? AqsaSafahat : (int)adad;
            for (int i = 0; i < hadd; i++)
            {
                maqbad.Safha((ushort)i, out TaaribSafha wasf);
                int ard = wasf.Ard;
                int irtifa = wasf.Irtifa;
                if (ard <= 0 || irtifa <= 0 || ard > AqsaBud || irtifa > AqsaBud)
                {
                    throw new KhataTaarib(
                        Ramz.QeemaBatila,
                        "TAARIB-E-6102",
                        $"صفحة اللوحة {i} تعلن أبعادًا {ard}×{irtifa}، خارج المدى المسموح "
                        + $"1..{AqsaBud}.",
                        $"Atlas page {i} declares dimensions {ard}x{irtifa}, outside the "
                        + $"permitted range 1..{AqsaBud}.",
                        Khutwa.IadatTarkibIttar);
                }

                // The byte count must equal the dimensions. A page that disagrees
                // with itself uploads a texture whose rows are shifted against the
                // rectangles the glyph map names, and every glyph then draws a
                // little wrong in a way that reads as a font bug.
                long madum = (long)ard * irtifa;
                long fili = (long)wasf.Tul;
                if (fili != madum)
                {
                    throw new KhataTaarib(
                        Ramz.QeemaBatila,
                        "TAARIB-E-6103",
                        $"صفحة اللوحة {i} تحمل {fili} بايت لأبعاد {ard}×{irtifa}، والمتوقع "
                        + $"{madum}.",
                        $"Atlas page {i} carries {fili} bytes for dimensions {ard}x{irtifa}; "
                        + $"{madum} were expected.",
                        Khutwa.IadatTarkibIttar);
                }

                jadwal.Qayyid(i, ard, irtifa);
            }
        }

        private void LazimHay()
        {
            if (mutlaf)
            {
                throw new ObjectDisposedException(nameof(Lawha));
            }
        }
    }
}
