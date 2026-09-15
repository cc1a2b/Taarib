// النسيج — positioned glyphs turned into geometry, without naming a Unity type.
//
// This file is the reason Mushtarak exists. A Vector3 under Mono is a type in
// the game's own UnityEngine.CoreModule.dll; under IL2CPP it is an
// Il2CppInterop proxy generated from that specific game's metadata, in an
// assembly with a different identity. An assembly that named either could not
// load in the other's process. So the shape of a glyph quad — how many
// vertices, in what order, wound which way, with which corner holding which
// texture coordinate — is decided here, once, and each adapter performs the
// call that hands the result to whatever mesh container its runtime has:
// TMP_MeshInfo arrays on one path, a VertexHelper on another.
//
// Two things in here are easy to get subtly wrong and expensive to notice:
//
//   - The Y axis. Taarib measures downward from the layout's top; Unity's UI
//     space measures upward from the rect's pivot. The flip is four lines of
//     arithmetic and it is documented in full on HayyizRasm, because the
//     symptom of getting it wrong is text that renders perfectly and sits ten
//     pixels too low — which reads as a font metrics problem and is not one.
//   - The V axis. Unity's raw texture upload treats row zero as the bottom
//     row, and an atlas page is row-major from the top. Nasij flips V rather
//     than making the upload flip rows; the reasoning is on the Nasij class.
//
// Nothing here allocates. Every destination is a caller-owned span, every
// intermediate is a local struct, and the one lookup that could allocate — the
// glyph map — is behind a generic constraint so a struct implementation is
// dispatched statically.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mushtarak
{
    /// <summary>
    /// One vertex position, in the destination component's local space. Three
    /// floats, sequential, which is the layout of the position type every
    /// Unity mesh container wants — so an adapter reinterprets its own array
    /// with <c>MemoryMarshal.Cast</c> rather than copying element by element.
    /// </summary>
    /// <remarks>
    /// <see cref="Z"/> is always written as zero. Taarib lays text out in two
    /// dimensions; an adapter that wants a depth offset for an outline or a
    /// shadow pass applies it in its own transform or its own shader, not
    /// here, because a z written per vertex would have to be undone by every
    /// consumer that does not want it.
    /// </remarks>
    public struct NuqtaRasm
    {
        /// <summary>Horizontal position, rightward, in local units.</summary>
        public float S;

        /// <summary>Vertical position, upward, in local units.</summary>
        public float A;

        /// <summary>Always zero.</summary>
        public float Z;
    }

    /// <summary>
    /// One texture coordinate into the atlas page, normalized to the unit
    /// square. Two floats, sequential, matching the two-component vector type
    /// every Unity mesh container uses for its first UV channel.
    /// </summary>
    public struct NuqtaMulmas
    {
        /// <summary>Horizontal coordinate, zero at the page's left edge.</summary>
        public float U;

        /// <summary>
        /// Vertical coordinate, zero at the page's <em>bottom</em> edge —
        /// Unity's convention, not the atlas's. The flip that reconciles the
        /// two is described on <see cref="Nasij"/>.
        /// </summary>
        public float V;
    }

    /// <summary>
    /// One vertex colour: four bytes, red green blue alpha, matching the
    /// packed colour type Unity's mesh containers take. Not four floats — the
    /// float form doubles the vertex buffer for a value that has eight bits of
    /// precision on the way to the framebuffer anyway.
    /// </summary>
    public struct LawnRasm
    {
        /// <summary>Red.</summary>
        public byte Ahmar;

        /// <summary>Green.</summary>
        public byte Akhdar;

        /// <summary>Blue.</summary>
        public byte Azraq;

        /// <summary>Alpha; 255 is opaque.</summary>
        public byte Shaffafiya;

        /// <summary>
        /// Unpacks the <c>0xRRGGBBAA</c> word that
        /// <see cref="TaaribNitaqUslub.Lawn"/> carries. That byte order is the
        /// ABI's, and it is not the order this struct stores, which is exactly
        /// why the conversion lives in one named place instead of being open
        /// coded at each call site with the shifts written from memory.
        /// </summary>
        /// <param name="lawn">The packed colour, red in the high byte.</param>
        /// <returns>The unpacked colour.</returns>
        public static LawnRasm Min(uint lawn)
        {
            LawnRasm natija;
            natija.Ahmar = (byte)(lawn >> 24);
            natija.Akhdar = (byte)(lawn >> 16);
            natija.Azraq = (byte)(lawn >> 8);
            natija.Shaffafiya = (byte)lawn;
            return natija;
        }

        /// <summary>An opaque white, the colour a span that sets none resolves to.</summary>
        /// <returns>Opaque white.</returns>
        public static LawnRasm Abyad()
        {
            LawnRasm natija;
            natija.Ahmar = 0xFF;
            natija.Akhdar = 0xFF;
            natija.Azraq = 0xFF;
            natija.Shaffafiya = 0xFF;
            return natija;
        }
    }

    /// <summary>
    /// A rectangle in the destination component's local space: left edge,
    /// bottom edge, and size. Y-up, like everything Nasij hands back, so a
    /// caller never has to remember which of the two vertical conventions a
    /// particular number is in.
    /// </summary>
    public struct MustatilRasm
    {
        /// <summary>The left edge.</summary>
        public float Yasar;

        /// <summary>The bottom edge.</summary>
        public float Asfal;

        /// <summary>Width, never negative.</summary>
        public float Ard;

        /// <summary>Height, never negative.</summary>
        public float Irtifa;
    }

    /// <summary>
    /// One atlas page's dimensions in texels, which is what turns a glyph's
    /// texel rectangle into a normalized texture coordinate.
    /// </summary>
    public struct QiyasSafha
    {
        /// <summary>Width in texels.</summary>
        public ushort Ard;

        /// <summary>Height in texels.</summary>
        public ushort Irtifa;
    }

    /// <summary>
    /// Where a glyph lives in the atlas, as far as mesh building is concerned.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This is a seam, and it is a seam against Taarib rather than against
    /// Unity. <see cref="MaqbadLawha.Shakl"/> would answer both questions
    /// directly, but calling it from inside the vertex loop would mean a
    /// native transition per glyph, and a <see cref="Ramz.LawhaMumtalia"/>
    /// thrown halfway through a mesh, leaving the destination arrays holding a
    /// partial sentence. Residency is a separate concern with a separate file:
    /// it walks the layout once, brings every glyph in, uploads whatever grew,
    /// and only then is a mesh built against a map that cannot miss.
    /// </para>
    /// <para>
    /// Implement it on a <c>struct</c>. <see cref="Nasij.Ibni{TKhareeta}"/> is generic
    /// over this interface with no class constraint, so a value-type
    /// implementation is dispatched statically and never boxed; a class
    /// implementation works and costs one interface call per glyph, which is
    /// measurable on a screen full of dialogue and is the only reason the
    /// generic parameter exists at all.
    /// </para>
    /// </remarks>
    public interface IKhareetatAshkal
    {
        /// <summary>
        /// Where one glyph sits in the atlas.
        /// </summary>
        /// <param name="miftah">
        /// The glyph, its font index, its quantized size and its subpixel
        /// bucket — everything that makes one rasterized image distinct.
        /// </param>
        /// <param name="mawdi">The rectangle, the bearings and the page.</param>
        /// <returns>
        /// <c>false</c> when the glyph is not resident. Nasij then emits no
        /// geometry for it and counts it, rather than drawing whatever
        /// happens to occupy the rectangle a default value points at — which
        /// would be a different letter, from a different word, at full
        /// opacity.
        /// </returns>
        bool Shakl(in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi);

        /// <summary>
        /// One page's dimensions, for normalizing texture coordinates.
        /// </summary>
        /// <param name="fahras">The page index, from <see cref="TaaribMawdiShakl.Safha"/>.</param>
        /// <param name="qiyas">The page's size in texels.</param>
        /// <returns><c>false</c> when there is no such page.</returns>
        bool Safha(ushort fahras, out QiyasSafha qiyas);
    }

    /// <summary>
    /// Where the laid-out block sits vertically inside the destination
    /// rectangle when it is shorter than the rectangle.
    /// </summary>
    /// <remarks>
    /// Horizontal placement is not here, and must not be: Taarib already
    /// applied <see cref="Muhadhaha"/> while laying out, and
    /// <see cref="TaaribSatr.Bidaya"/> carries the result. Applying a
    /// horizontal offset here as well would centre an already-centred line.
    /// Vertical placement has no equivalent in the layout request, because the
    /// layout does not know the height it is being poured into, so it is
    /// resolved here from <see cref="HayyizRasm.Irtifa"/> and the layout's own
    /// measured height.
    /// </remarks>
    public enum MuhadhahaRasiya
    {
        /// <summary>The block's top edge meets the rectangle's top edge.</summary>
        Ala = 0,

        /// <summary>The block is centred vertically.</summary>
        Wasat = 1,

        /// <summary>The block's bottom edge meets the rectangle's bottom edge.</summary>
        Asfal = 2,

        /// <summary>
        /// The first line's baseline meets the rectangle's top edge, which is
        /// what a world-space text object with no rectangle at all wants.
        /// </summary>
        KhattAsas = 3,
    }

    /// <summary>
    /// The destination space: the rectangle a text component draws into, its
    /// pivot, and the scale between layout pixels and that component's local
    /// units. This struct is the whole of the coordinate conversion, and the
    /// conversion is written out in full below because it is the single
    /// easiest thing in this phase to get subtly wrong.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The two spaces.</b> Taarib's layout space has its origin at the
    /// top-left corner of the laid-out block. <see cref="TaaribHarf.S"/> grows
    /// rightward from that corner and <see cref="TaaribHarf.A"/> grows
    /// <em>downward</em> from it, in pixels. Unity's local space for a
    /// rectangle-based component has its origin at the <em>pivot</em> — not at
    /// a corner and not at the centre, but wherever the pivot was authored —
    /// with Y growing upward. Neither the direction of Y nor the position of
    /// the origin agrees between the two, and both disagreements have to be
    /// undone in the same expression.
    /// </para>
    /// <para>
    /// <b>The transform.</b> Writing <c>W</c> for <see cref="Ard"/>, <c>H</c>
    /// for <see cref="Irtifa"/>, <c>px</c> and <c>py</c> for
    /// <see cref="MihwarS"/> and <see cref="MihwarA"/>, <c>k</c> for
    /// <see cref="Qiyas"/> and <c>Lh</c> for <see cref="IrtifaTakhtit"/>:
    /// </para>
    /// <code>
    /// rectangle edges, in local space, whose origin is the pivot point
    ///     yasar = -px * W                      the left edge
    ///     ala   = (1 - py) * H                 the top edge
    ///
    /// vertical slack between the rectangle's top and the block's top
    ///     Ala        fajwa = 0
    ///     Wasat      fajwa = (H - Lh * k) / 2
    ///     Asfal      fajwa =  H - Lh * k
    ///     KhattAsas  fajwa = 0, with Lh passed as zero
    ///
    /// the origin of layout space, expressed in local space
    ///     asl_s = yasar + IzahaS
    ///     asl_a = ala - fajwa + IzahaA
    ///
    /// and finally a layout point (s, a)
    ///     x = asl_s + s * k
    ///     y = asl_a - a * k          the flip: minus, never plus
    /// </code>
    /// <para>
    /// <b>The failure this prevents.</b> Drop the <c>(1 - py)</c> and take
    /// <c>py * H</c>, or forget <c>fajwa</c> entirely, and every glyph still
    /// carries the right outline, the right joining, the right kerning and the
    /// right colour — the whole block simply sits somewhere else. On a
    /// centre-pivoted label with a font size near half the rectangle's height
    /// the error is a handful of pixels, which reads as a font metrics problem
    /// and sends the next person to look at ascent and descent, where there is
    /// nothing to find.
    /// </para>
    /// <para>
    /// <b>The width has to match.</b> The horizontal term is only the
    /// rectangle's left edge because Taarib already resolved alignment while
    /// laying out. That is correct exactly when the layout was given the same
    /// width this rectangle has: <c>talab.ArdMutah == Ard / Qiyas</c>. Lay out
    /// against one width and place against another and a centred line is
    /// centred on the wrong axis — the same symptom, in the other direction.
    /// </para>
    /// <para>
    /// <b>A component with no rectangle.</b> World-space text has a transform
    /// origin and nothing else. Pass <see cref="Ard"/> and
    /// <see cref="Irtifa"/> as zero and the pivot as <c>(0, 0)</c>: both edges
    /// collapse to the origin, and the block then hangs down and to the right
    /// of it, which is what layout space already means.
    /// </para>
    /// </remarks>
    public struct HayyizRasm
    {
        /// <summary>The rectangle's width, in the component's local units.</summary>
        public float Ard;

        /// <summary>The rectangle's height, in the component's local units.</summary>
        public float Irtifa;

        /// <summary>
        /// The pivot's horizontal position: zero at the rectangle's left edge,
        /// one at its right.
        /// </summary>
        public float MihwarS;

        /// <summary>
        /// The pivot's vertical position: zero at the rectangle's bottom edge,
        /// one at its top. Unity's own order, so an adapter passes the pivot
        /// through without swapping anything.
        /// </summary>
        public float MihwarA;

        /// <summary>
        /// Local units per layout pixel. One for a canvas whose units are
        /// pixels; the component's own point-size-to-unit ratio for world-space
        /// text. Must be greater than zero.
        /// </summary>
        public float Qiyas;

        /// <summary>
        /// The layout's total height in layout pixels — <see cref="Takhtit.Irtifa"/>
        /// verbatim. Read only by <see cref="MuhadhahaRasiya.Wasat"/> and
        /// <see cref="MuhadhahaRasiya.Asfal"/>; pass zero with
        /// <see cref="MuhadhahaRasiya.KhattAsas"/>.
        /// </summary>
        public float IrtifaTakhtit;

        /// <summary>
        /// An extra horizontal offset in local units, added after the
        /// rectangle's left edge. This is where a component's own left padding
        /// or margin goes.
        /// </summary>
        public float IzahaS;

        /// <summary>An extra vertical offset in local units, positive upward.</summary>
        public float IzahaA;

        /// <summary>Where the block sits vertically inside the rectangle.</summary>
        public MuhadhahaRasiya Muhadhaha;
    }

    /// <summary>
    /// An inline atom that was positioned but not drawn: its span, the
    /// caller's own reference to it, and the rectangle the engine should put
    /// its own sprite into.
    /// </summary>
    /// <remarks>
    /// The seam is deliberate and it runs exactly here. A span carrying
    /// <see cref="Alamat.UslubDharra"/> is an inline image or a format
    /// placeholder: it occupies a position, contributes a width to line
    /// breaking and justification, and takes part in reordering, all of which
    /// is Taarib's work — but the picture inside it belongs to the game. Its
    /// pixels live in the game's own sprite asset, which Decision 5 forbids
    /// reading, and its identity is a number only the game's asset table can
    /// resolve. So Nasij emits no geometry for it, hands back where it landed,
    /// and the adapter draws it with the engine's own sprite path. An atom
    /// that Nasij tried to draw would be a blank rectangle of atlas at best
    /// and a stray glyph at worst.
    /// </remarks>
    public struct DharraMawduaa
    {
        /// <summary>
        /// The identifier of the span that carried it, matching
        /// <see cref="TaaribNitaqUslub.Id"/> and <see cref="TaaribHarf.Nitaq"/>.
        /// </summary>
        public ushort Nitaq;

        /// <summary>
        /// The caller's own reference, <see cref="TaaribNitaqUslub.MarjaDharra"/>
        /// carried through untouched — an index into whatever atom table the
        /// markup bridge produced.
        /// </summary>
        public uint Marja;

        /// <summary>
        /// Where to draw it, in the destination component's local space, Y-up.
        /// </summary>
        public MustatilRasm Mustatil;

        /// <summary>
        /// The colour the surrounding style resolved to, so a tinted sprite
        /// matches the text it sits in.
        /// </summary>
        public LawnRasm Lawn;

        /// <summary>
        /// Which line it landed on, as an index into the request's line table.
        /// A caller that needs the atom's baseline reads it from there rather
        /// than recovering it from the rectangle.
        /// </summary>
        public int Satr;
    }

    /// <summary>
    /// One mesh-building request: a layout, the styles that coloured it, and
    /// where it is going. A <c>ref struct</c> because it holds spans over
    /// buffers that live exactly as long as the call — the same lifetime
    /// <see cref="TalabTakhtit"/> has, for the same reason.
    /// </summary>
    public ref struct TalabNasij
    {
        /// <summary>
        /// The positioned glyphs, in visual order —
        /// <see cref="Takhtit.Huruf"/> handed straight through. Never copied
        /// out of the layout buffer first; copying it would be the one
        /// allocation this whole path is arranged to avoid.
        /// </summary>
        public ReadOnlySpan<TaaribHarf> Huruf;

        /// <summary>
        /// The lines, from <see cref="Takhtit.Sutur"/>. Glyphs are visited
        /// line by line rather than in one flat pass, because an inline atom
        /// with no declared height falls back to its own line's box height,
        /// and a glyph's line is not otherwise recoverable from the glyph.
        /// </summary>
        public ReadOnlySpan<TaaribSatr> Sutur;

        /// <summary>
        /// The style spans the layout was given, unchanged. Colour is read
        /// from here; nothing else is. Empty means one unstyled run, and the
        /// colour resolver then does no work at all.
        /// </summary>
        public ReadOnlySpan<TaaribNitaqUslub> Nitaqat;

        /// <summary>Where the mesh is going, and how layout space maps into it.</summary>
        public HayyizRasm Hayyiz;

        /// <summary>
        /// The colour for any glyph whose style sets none — the component's
        /// own colour, which is what the game would have drawn.
        /// </summary>
        public LawnRasm Lawn;

        /// <summary>
        /// The size the layout was finally laid out at, in pixels:
        /// <see cref="Takhtit.Hajm"/> verbatim. Not the size that was
        /// requested — shrink-to-fit may have lowered it, and the glyph
        /// rectangles have to be scaled against the size that was actually
        /// used.
        /// </summary>
        public float Hajm;

        /// <summary>
        /// The pixel size the atlas rasterized these glyphs at. Equal to
        /// <see cref="Hajm"/> for a coverage atlas, which holds one bitmap per
        /// size; the atlas's own canonical size for a distance-field atlas,
        /// which holds one bitmap for every size. Every glyph rectangle and
        /// both bearings are multiplied by <c>Hajm / HajmLawha</c>, and that
        /// single ratio is the whole of the difference between the two atlas
        /// modes as far as geometry is concerned.
        /// </summary>
        public float HajmLawha;

        /// <summary>
        /// Which atlas page to emit geometry for. Glyphs on other pages are
        /// skipped and their pages are reported in
        /// <see cref="NatijaNasij.AlamSafahat"/>, because a page is a texture
        /// and a texture is a draw call: two pages in one index buffer would
        /// draw one of them with the other's atlas.
        /// </summary>
        public ushort Safha;

        /// <summary>
        /// Whether to snap the pen to whole layout pixels and carry the
        /// discarded fraction in the glyph key's subpixel bucket.
        /// </summary>
        /// <remarks>
        /// True for a coverage atlas on a component that maps one layout pixel
        /// to one screen pixel: the bitmap was rasterized with the fraction
        /// already folded into its coverage, so the quad must land on the
        /// integer grid or the fraction is applied twice and the whole line
        /// blurs. False for a distance-field atlas, and false for any scaled
        /// or world-space component, where snapping in layout space snaps to
        /// nothing in particular on screen and only makes letter spacing
        /// uneven. One flag governs both the snap and the bucket so the two
        /// cannot disagree — a snapped pen with bucket zero is a glyph drawn
        /// up to a quarter of a pixel from where it was measured.
        /// </remarks>
        public bool Tathbit;
    }

    /// <summary>
    /// The caller-owned destination. Every span belongs to the adapter,
    /// because every one of them is, or is cast from, an array a Unity mesh
    /// container already owns — a <c>TMP_MeshInfo</c>'s parallel arrays on one
    /// path, a <c>VertexHelper</c>'s stream on another. Mushtarak cannot name
    /// those types, so it writes into what it is given.
    /// </summary>
    /// <remarks>
    /// <see cref="Ruus"/>, <see cref="Malamis"/> and <see cref="Alwan"/> are
    /// parallel: element <c>i</c> of each describes the same vertex.
    /// <see cref="Muthallathat"/> indexes into them. All four may be exact
    /// reinterpretations of the adapter's own arrays; see
    /// <see cref="Nasij.TahaqquqTakhtit"/> for the one check that makes the
    /// reinterpretation safe.
    /// </remarks>
    public ref struct MakhzanNasij
    {
        /// <summary>Vertex positions, four per drawn glyph.</summary>
        public Span<NuqtaRasm> Ruus;

        /// <summary>Texture coordinates, one per vertex.</summary>
        public Span<NuqtaMulmas> Malamis;

        /// <summary>Vertex colours, one per vertex.</summary>
        public Span<LawnRasm> Alwan;

        /// <summary>Triangle indices, six per drawn glyph.</summary>
        public Span<int> Muthallathat;

        /// <summary>
        /// Where the inline atoms landed. May be empty when the caller knows
        /// the text has none; atoms are then counted and dropped rather than
        /// failing the call, because a sentence with an unreported sprite in
        /// it is still a correctly rendered sentence.
        /// </summary>
        public Span<DharraMawduaa> Dharrat;

        /// <summary>
        /// Optional. Glyph index to vertex base index: entry <c>i</c> is the
        /// first of the four vertices glyph <c>i</c> produced, or <c>-1</c>
        /// when that glyph produced none — a space, an atom, a glyph on
        /// another page, a glyph the atlas did not hold.
        /// </summary>
        /// <remarks>
        /// Geometry is written densely, so a glyph's vertices are not at four
        /// times its index and cannot be found by arithmetic. A caller that
        /// has to fill the game's own per-character bookkeeping — the arrays
        /// game code reads to place a cursor or drive an effect — needs the
        /// mapping, and rebuilding it on the adapter's side would mean a
        /// second copy of the rule about which glyphs are skipped. Two copies
        /// of that rule is one of them being wrong after the next change.
        /// Leave the span empty when nothing needs the mapping and none is
        /// written.
        /// </remarks>
        public Span<int> MawaqiRuus;
    }

    /// <summary>
    /// What was written, what was skipped, and — when the destination was too
    /// small — what would have been needed.
    /// </summary>
    /// <remarks>
    /// The short-buffer behaviour is the same negotiation
    /// <see cref="Ramz.SiatQasira"/> defines at the ABI: on a shortfall
    /// nothing at all is written, the requirements are reported, the caller
    /// grows once and calls again. Deliberately the same shape as the layout
    /// path's, so an adapter has one growth idiom rather than two.
    /// </remarks>
    public struct NatijaNasij
    {
        /// <summary>Whether everything fit and the destination was written.</summary>
        public bool Kafa;

        /// <summary>
        /// Vertices the geometry occupies: four times <see cref="AdadAshkal"/>.
        /// Written by <see cref="Nasij.Ibni{TKhareeta}"/>, counted only by
        /// <see cref="Nasij.Ihsi{TKhareeta}"/>.
        /// </summary>
        public int AdadRuus;

        /// <summary>Indices the geometry occupies: six times <see cref="AdadAshkal"/>.</summary>
        public int AdadMuthallathat;

        /// <summary>
        /// How many inline atoms the layout carries. When this is larger than
        /// the length of <see cref="MakhzanNasij.Dharrat"/>, only the first
        /// that many were written — the geometry is unaffected, because an
        /// atom draws nothing.
        /// </summary>
        public int AdadDharrat;

        /// <summary>Glyphs that produced geometry.</summary>
        public int AdadAshkal;

        /// <summary>
        /// Glyphs with an empty rectangle — spaces, and anything else the font
        /// draws nothing for. Counted rather than drawn; see
        /// <see cref="Nasij"/> for why an empty quad is worse than no quad.
        /// </summary>
        public int AdadFaragh;

        /// <summary>
        /// Glyphs the atlas did not hold. Non-zero means residency and mesh
        /// building disagree about what was brought in, which is a bug in the
        /// caller's ordering rather than in the text.
        /// </summary>
        public int AdadMafqud;

        /// <summary>
        /// Vertex capacity required, meaningful when <see cref="Kafa"/> is
        /// false.
        /// </summary>
        public int MatlubRuus;

        /// <summary>Index capacity required.</summary>
        public int MatlubMuthallathat;

        /// <summary>Atom capacity required.</summary>
        public int MatlubDharrat;

        /// <summary>
        /// Every atlas page this layout touches, one bit per page index, so a
        /// caller knows exactly which pages still need their own call rather
        /// than probing for them. Bit zero is page zero.
        /// </summary>
        public ulong AlamSafahat;

        /// <summary>
        /// Whether a glyph lives on a page numbered 64 or higher, which
        /// <see cref="AlamSafahat"/> cannot represent. An atlas that reached
        /// sixty-four pages has a budget problem, not a rendering problem, but
        /// the mesh path still has to say so rather than silently dropping the
        /// page.
        /// </summary>
        public bool SafahatBaida;

        /// <summary>
        /// The bounding box of everything written, in the destination's local
        /// space. Zero-sized when nothing was written. Given because a mesh
        /// container wants bounds and computing them from the vertices
        /// afterwards is a second pass over the buffer that this pass can
        /// avoid for free.
        /// </summary>
        public MustatilRasm Hudud;
    }

    /// <summary>
    /// Turns positioned glyphs into geometry. Stateless, allocation-free, and
    /// the only place in the product where a glyph becomes triangles.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>The vertex layout.</b> One drawn glyph is four vertices and two
    /// triangles — six indices — in this order, where the corners are named in
    /// the destination's own Y-up space:
    /// </para>
    /// <code>
    ///     vertex 0   AsfalYasar    bottom-left    (xL, yB)   uv (uL, vB)
    ///     vertex 1   AlaYasar      top-left       (xL, yT)   uv (uL, vT)
    ///     vertex 2   AlaYameen     top-right      (xR, yT)   uv (uR, vT)
    ///     vertex 3   AsfalYameen   bottom-right   (xR, yB)   uv (uR, vB)
    ///
    ///     triangle A   0, 1, 2
    ///     triangle B   2, 3, 0
    /// </code>
    /// <para>
    /// That order is not arbitrary and it is not Taarib's invention. It is the
    /// order Unity's own quad producers already use — the legacy UI's sprite
    /// quad and TextMeshPro's per-character vertex block are both bottom-left,
    /// top-left, top-right, bottom-right — so an adapter writing into either
    /// container writes the same four vertices in the same four slots, and no
    /// adapter has to reorder anything. An adapter that did reorder would be
    /// the only place a mesh could differ between the two backends, which is
    /// the entire class of divergence this assembly exists to prevent.
    /// </para>
    /// <para>
    /// <b>Winding.</b> Walking 0 to 1 to 2 goes up the left edge and then
    /// right along the top, which in a Y-up space viewed from the front is
    /// clockwise. Unity treats clockwise as front-facing, so these triangles
    /// survive the default backface culling. Canvas shaders usually disable
    /// culling and would not care; a world-space text object with an ordinary
    /// opaque material does care, and the symptom there is text that is
    /// invisible from the side a player actually stands on.
    /// </para>
    /// <para>
    /// <b>The V flip.</b> An atlas page is row-major from the top, and Unity's
    /// raw texture upload treats the first row of the data it is handed as the
    /// <em>bottom</em> row of the texture. Uploaded verbatim, a page therefore
    /// arrives in the texture vertically mirrored, and Nasij undoes that in
    /// the texture coordinates: <c>vTop = 1 - A / H</c> and
    /// <c>vBottom = 1 - (A + Irtifa) / H</c>. The flip lives here rather than
    /// in the upload because flipping rows on upload costs a copy of the whole
    /// page every time the atlas grows, on the frame that grew it, while
    /// flipping V costs two subtractions on numbers already being computed.
    /// If an adapter ever flips rows on upload as well, every glyph renders
    /// upside down; this paragraph is the one place to change.
    /// </para>
    /// <para>
    /// No texture coordinates are inset by half a texel. The atlas already
    /// pads every rectangle — that is what the <c>hashw</c> argument to
    /// <see cref="MaqbadLawha.Insha"/> is for — so sampling at the exact
    /// rectangle edge cannot reach a neighbour, and insetting would instead
    /// shave a texel off every glyph.
    /// </para>
    /// <para>
    /// <b>No second UV channel.</b> Taarib's own shaders derive their
    /// antialiasing width from screen-space derivatives, so there is nothing
    /// per-vertex for a second channel to carry. Writing one that no shader
    /// reads would be vertex bandwidth spent on nothing.
    /// </para>
    /// <para>
    /// <b>An empty glyph contributes nothing.</b> A space has an advance and
    /// no image: its atlas rectangle is zero by zero. Nasij emits no vertices
    /// and no indices for it, rather than a degenerate quad, and the
    /// difference is not only the buffer space. A quad with four coincident
    /// corners still enters vertex transform, still occupies index buffer,
    /// still gets binned on a tiled GPU, and — the part that actually shows —
    /// still participates in the mesh's bounding box. Spaces at the start of a
    /// line would drag the bounds toward wherever the pen happened to be, and
    /// bounds that are wrong are text that vanishes when the camera moves,
    /// intermittently, on some screens. Worse, "zero size" survives exactly
    /// zero rounding: after the pivot offset and the scale, two corners that
    /// were equal in layout space can land on different pixel centres, and the
    /// quad then samples the atlas at texture coordinate zero, which is
    /// whatever glyph the packer put in the page's corner. The symptom is a
    /// faint speck of another letter where a space should be, on some
    /// resolutions and not others. No geometry has none of these failures.
    /// </para>
    /// <para>
    /// <b>Colour is per vertex and comes from the style span.</b>
    /// <see cref="TaaribHarf.Nitaq"/> names the span a glyph inherited, and
    /// that field is the entire reason a span identifier rides through
    /// shaping, reordering and line breaking at all: after the bidirectional
    /// algorithm has moved a red word from the middle of a sentence to the
    /// other end of the line, the only thing still connecting that glyph to
    /// its colour is the number it is carrying. A mesh builder that ignored
    /// it would produce a perfectly shaped, perfectly ordered, perfectly
    /// justified sentence in one flat colour — every <c>&lt;color&gt;</c> in
    /// the game silently gone, which reads as "rich text stopped working"
    /// rather than as a dropped field, and sends the next person to look at
    /// the markup bridge, where there is nothing wrong.
    /// </para>
    /// <para>
    /// <b>Marks are ordinary glyphs here.</b> A glyph carrying
    /// <see cref="Alamat.HarfAlama"/> has a zero advance, which matters to
    /// measurement and to caret placement and not at all to this file: the
    /// shaper already put the mark's origin where the font's attachment rules
    /// say it goes, so it is drawn exactly like a letter. Skipping marks
    /// because their advance is zero would delete every diacritic from a
    /// vocalised line, and the line would still measure correctly.
    /// </para>
    /// </remarks>
    public static class Nasij
    {
        /// <summary>Vertices per drawn glyph.</summary>
        public const int RuusLiShakl = 4;

        /// <summary>Triangle indices per drawn glyph.</summary>
        public const int FahrasLiShakl = 6;

        /// <summary>The bottom-left corner's offset within a glyph's four vertices.</summary>
        public const int ZawiyatAsfalYasar = 0;

        /// <summary>The top-left corner's offset.</summary>
        public const int ZawiyatAlaYasar = 1;

        /// <summary>The top-right corner's offset.</summary>
        public const int ZawiyatAlaYameen = 2;

        /// <summary>The bottom-right corner's offset.</summary>
        public const int ZawiyatAsfalYameen = 3;

        /// <summary>
        /// How many horizontal subpixel positions a glyph is rasterized at,
        /// mirroring <c>MAWADI_TAHAZZUZ</c> in
        /// <c>crates/taarib-saff/src/rasm.rs</c>. Four, because the residual
        /// error is then below what antialiasing already blurs while the cache
        /// stays at four entries per glyph rather than one per pen position.
        /// </summary>
        public const byte AdadBakat = 4;

        /// <summary>
        /// The byte size <see cref="NuqtaRasm"/> must have to be
        /// reinterpretable as the adapter's own position type.
        /// </summary>
        public const int HajmNuqtaRasm = 12;

        /// <summary>The byte size <see cref="NuqtaMulmas"/> must have.</summary>
        public const int HajmNuqtaMulmas = 8;

        /// <summary>The byte size <see cref="LawnRasm"/> must have.</summary>
        public const int HajmLawnRasm = 4;

        /// <summary>
        /// Confirms once, at startup, that the adapter's vertex types have the
        /// layout this assembly assumes, before any mesh is built against that
        /// assumption.
        /// </summary>
        /// <remarks>
        /// This is the guard on the one seam that cannot be expressed in the
        /// type system. Mushtarak cannot name the Unity vector and colour
        /// types, so an adapter reinterprets its arrays through
        /// <c>MemoryMarshal.Cast</c> — a cast that is checked for element
        /// count and not for meaning. If a runtime ever presented a position
        /// type that was not three floats, the cast would still succeed and
        /// every vertex would be written into the middle of its neighbour.
        /// Called once with the sizes the adapter measures from its own types,
        /// this turns that into a refusal at load time with a sentence in the
        /// log, which is the difference between a patch that does not start
        /// and a patch that draws noise over someone's game.
        /// </remarks>
        /// <param name="hajmNuqta">The adapter's position type size, in bytes.</param>
        /// <param name="hajmMulmas">The adapter's texture coordinate type size.</param>
        /// <param name="hajmLawn">The adapter's packed colour type size.</param>
        /// <exception cref="KhataTaarib">A size does not match.</exception>
        public static void TahaqquqTakhtit(int hajmNuqta, int hajmMulmas, int hajmLawn)
        {
            if (hajmNuqta == HajmNuqtaRasm
                && hajmMulmas == HajmNuqtaMulmas
                && hajmLawn == HajmLawnRasm)
            {
                return;
            }
            throw new KhataTaarib(
                Ramz.QeemaBatila,
                string.Empty,
                "لا تطابق أنواع الرؤوس في هذا المحرك ما بُني عليه تعريب "
                    + $"({hajmNuqta}/{hajmMulmas}/{hajmLawn} بدل "
                    + $"{HajmNuqtaRasm}/{HajmNuqtaMulmas}/{HajmLawnRasm} بايت)؛ "
                    + "رُفض بناء المجسّمات بدل الكتابة في ذاكرة بترتيب مختلف.",
                "This engine's vertex types do not match what Taarib was built against "
                    + $"({hajmNuqta}/{hajmMulmas}/{hajmLawn} rather than "
                    + $"{HajmNuqtaRasm}/{HajmNuqtaMulmas}/{HajmLawnRasm} bytes); mesh "
                    + "building is refused rather than writing into memory laid out differently.",
                Khutwa.FathTashkhis);
        }

        /// <summary>
        /// The rasterization sizes a coverage atlas is allowed to use when the
        /// drawn size is not the layout size.
        /// </summary>
        /// <remarks>
        /// A ladder rather than the exact figure, because the exact figure on
        /// a world-space surface changes with the camera and every distinct
        /// value is a second copy of every glyph in the atlas. The rungs step
        /// by roughly √2, so any size is served by a bitmap no more than 41%
        /// larger — the point at which minification stops being visible — and
        /// the whole ladder is ten sizes rather than unbounded. Same spacing as
        /// the compiler's default size list, for the same reason.
        /// </remarks>
        private static readonly float[] SalalimLawha =
        {
            8f, 12f, 16f, 24f, 32f, 48f, 64f, 96f, 128f, 192f,
        };

        /// <summary>The smallest rung of <see cref="SalalimLawha"/>.</summary>
        public const float HajmLawhaAdna = 8f;

        /// <summary>The largest rung of <see cref="SalalimLawha"/>.</summary>
        public const float HajmLawhaAqsa = 192f;

        /// <summary>
        /// The pixel size a coverage atlas should rasterize at, for text whose
        /// one layout unit covers <paramref name="bikselLilWahda"/> screen
        /// pixels.
        /// </summary>
        /// <remarks>
        /// <para>
        /// A coverage bitmap has one resolution, so the only question that
        /// matters is how large the glyph ends up on the player's screen — not
        /// how large the number in the component's size field is. On an
        /// unscaled overlay canvas those are the same number and this returns
        /// the size unchanged, which is the pixel-exact path every interface
        /// label has always taken. They are not the same number anywhere else:
        /// a canvas with a scaler maps one layout unit to more than one pixel,
        /// and a world-space component measures its size in world units, where
        /// a heading on an in-game monitor can declare a size of 8 and cover a
        /// third of the screen. Rasterizing that at 8 pixels and magnifying it
        /// is the smear this exists to stop.
        /// </para>
        /// <para>
        /// The result is what <see cref="TalabNasij.HajmLawha"/> takes, and the
        /// <c>Hajm / HajmLawha</c> ratio in <see cref="Ibni{T}"/> scales every
        /// rectangle and bearing back into layout units, so the geometry is
        /// unchanged and only the sampled bitmap gets sharper.
        /// </para>
        /// <para>
        /// This is what a coverage atlas can do. A distance-field atlas is the
        /// real answer for text at an arbitrary on-screen size, and when this
        /// build draws through one, <see cref="NamatLawha.Misafa"/> takes this
        /// path out of the picture entirely.
        /// </para>
        /// </remarks>
        /// <param name="hajm">The size the layout was measured at.</param>
        /// <param name="bikselLilWahda">
        /// Screen pixels per layout unit, as the adapter measured it, or one
        /// when it could not be measured.
        /// </param>
        /// <returns>A size above zero.</returns>
        public static float HajmLawhaMulaim(float hajm, float bikselLilWahda)
        {
            if (!(hajm > 0f))
            {
                return HajmLawhaAdna;
            }
            if (!(bikselLilWahda > 0f))
            {
                bikselLilWahda = 1f;
            }

            float matlub = hajm * bikselLilWahda;
            // One unit to one pixel and a size already worth rasterizing: the
            // exact bitmap is sharper than any rung, because sampling a rung
            // means resampling, and an interface label drawn at its own size is
            // the one case where no resampling is needed at all.
            if (bikselLilWahda > 0.98f
                && bikselLilWahda < 1.02f
                && hajm >= HajmLawhaAdna
                && hajm <= HajmLawhaAqsa)
            {
                return hajm;
            }

            float[] salalim = SalalimLawha;
            for (int i = 0; i < salalim.Length; i++)
            {
                if (matlub <= salalim[i])
                {
                    return salalim[i];
                }
            }
            return HajmLawhaAqsa;
        }

        /// <summary>
        /// Whether the pen should be snapped to whole layout units, which is
        /// what <see cref="TalabNasij.Tathbit"/> governs.
        /// </summary>
        /// <remarks>
        /// True only for a coverage atlas rasterized at the layout size, where
        /// one layout unit is one atlas pixel and the pen's fraction is already
        /// folded into the bitmap. Once the two sizes differ — a scaled canvas,
        /// a world-space surface — a whole layout unit is not a pixel of
        /// anything, and snapping to it only makes letter spacing uneven.
        /// </remarks>
        /// <param name="namat">How the atlas was rasterized.</param>
        /// <param name="hajm">The layout size.</param>
        /// <param name="hajmLawha">The rasterization size.</param>
        /// <returns>Whether to snap.</returns>
        public static bool YuthabbatQalam(NamatLawha namat, float hajm, float hajmLawha)
        {
            return namat == NamatLawha.Taghtiya && hajmLawha == hajm;
        }

        /// <summary>
        /// The quantized pixel size that keys a glyph in the atlas: quarter
        /// pixels, matching <see cref="TaaribMiftahShakl.HajmRubi"/>.
        /// </summary>
        /// <remarks>
        /// Public because atlas residency has to produce the identical number
        /// for the identical size, and two implementations of one rounding
        /// rule eventually round differently — at which point residency brings
        /// a glyph in at one key and the mesh asks for it at another, and the
        /// letter is simply missing from the word. One function, two callers.
        /// </remarks>
        /// <param name="hajm">The pixel size.</param>
        /// <returns>The size in quarter pixels, at least one.</returns>
        public static ushort HajmRubi(float hajm)
        {
            double rubi = Math.Floor((double)hajm * 4.0 + 0.5);
            if (!(rubi >= 1.0))
            {
                return 1;
            }
            return rubi > ushort.MaxValue ? ushort.MaxValue : (ushort)rubi;
        }

        /// <summary>
        /// The subpixel bucket a pen position falls in, or zero when the
        /// caller is not snapping.
        /// </summary>
        /// <remarks>
        /// Public for the same reason as <see cref="HajmRubi"/>: residency and
        /// mesh building must agree on the bucket exactly, and the failure
        /// when they do not is a glyph that was rasterized and is still not
        /// found.
        /// </remarks>
        /// <param name="s">The pen's horizontal position, in layout pixels.</param>
        /// <param name="tathbit">
        /// Whether the pen is being snapped; see <see cref="TalabNasij.Tathbit"/>.
        /// </param>
        /// <returns>A bucket below <see cref="AdadBakat"/>.</returns>
        public static byte Bakat(float s, bool tathbit)
        {
            if (!tathbit)
            {
                return 0;
            }
            double kasr = (double)s - Math.Floor((double)s);
            int bakat = (int)(kasr * AdadBakat);
            if (bakat < 0)
            {
                return 0;
            }
            return bakat >= AdadBakat ? (byte)(AdadBakat - 1) : (byte)bakat;
        }

        /// <summary>
        /// Builds the geometry for one atlas page into the caller's spans.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Allocation-free from end to end: the only state is a handful of
        /// locals, the destination belongs to the caller, and the glyph map is
        /// a generic parameter so a struct implementation is called without a
        /// virtual dispatch and without boxing.
        /// </para>
        /// <para>
        /// The capacity it demands is <c>Huruf.Length</c> times
        /// <see cref="RuusLiShakl"/>, which is an upper bound rather than the
        /// exact count — every space and every inline atom will use less. That
        /// is deliberate: knowing the exact count means an atlas lookup per
        /// glyph before the first vertex is written, which doubles the cost of
        /// the build to spare a caller from over-allocating by the number of
        /// spaces in a sentence. Callers whose destination is sized per
        /// submesh, where the difference is real, ask <see cref="Ihsi{TKhareeta}"/>
        /// first.
        /// </para>
        /// <para>
        /// On a shortfall nothing is written at all, exactly as
        /// <see cref="Ramz.SiatQasira"/> behaves at the ABI. A partially
        /// written mesh is not a smaller sentence; it is a sentence with its
        /// last word missing, drawn at full confidence.
        /// </para>
        /// </remarks>
        /// <typeparam name="TKhareeta">
        /// The glyph map. Make it a struct; see <see cref="IKhareetatAshkal"/>.
        /// </typeparam>
        /// <param name="talab">The layout, the styles and the destination space.</param>
        /// <param name="khareeta">Where each glyph lives in the atlas.</param>
        /// <param name="makhzan">The caller's destination spans.</param>
        /// <returns>What was written, or what would have been needed.</returns>
        /// <exception cref="ArgumentOutOfRangeException">
        /// <see cref="HayyizRasm.Qiyas"/>, <see cref="TalabNasij.Hajm"/> or
        /// <see cref="TalabNasij.HajmLawha"/> is not above zero — each of
        /// which collapses every glyph to a point, which would otherwise be
        /// discovered as an invisible mesh rather than as a bad argument.
        /// </exception>
        public static NatijaNasij Ibni<TKhareeta>(
            in TalabNasij talab,
            TKhareeta khareeta,
            in MakhzanNasij makhzan)
            where TKhareeta : IKhareetatAshkal
        {
            NatijaNasij natija = Nafidh(in talab, khareeta, in makhzan, uktub: true);
            QallibMulmas(in makhzan, natija.AdadRuus);
            return natija;
        }

        /// <summary>
        /// Turns the atlas's own coordinate the right way up for the engine.
        /// </summary>
        /// <param name="makhzan">The buffers the build just wrote into.</param>
        /// <param name="adadRuus">How many vertices it wrote.</param>
        /// <remarks>
        /// Every rectangle in the glyph map is measured from the top of the
        /// page, because that is where the rasterizer starts and how the
        /// patch's baked pages are stored. Unity puts the first uploaded row at
        /// the *bottom* of a texture, so a coordinate handed over unchanged
        /// samples the page upside down — which in a real game means every
        /// translated line reads from a part of the atlas no glyph was ever
        /// written into and draws nothing at all. R.E.P.O. showed exactly that:
        /// the takeover ran, the atlas held the shaped Arabic, and the menu
        /// entry it owned simply disappeared.
        ///
        /// It is done here, once per string, rather than in each adapter,
        /// because every consumer of this builder draws through Unity. The
        /// overlay does not: it composites its own picture and never calls
        /// this.
        /// </remarks>
        private static void QallibMulmas(in MakhzanNasij makhzan, int adadRuus)
        {
            Span<NuqtaMulmas> malamis = makhzan.Malamis;
            int hadd = adadRuus < malamis.Length ? adadRuus : malamis.Length;
            for (int i = 0; i < hadd; i++)
            {
                malamis[i].V = 1f - malamis[i].V;
            }
        }

        /// <summary>
        /// Counts what <see cref="Ibni{TKhareeta}"/> would write for one page, writing
        /// nothing.
        /// </summary>
        /// <remarks>
        /// Reports its counts in the same fields <see cref="Ibni{TKhareeta}"/> does, and
        /// walks the identical code with the writes switched off — so the two
        /// cannot answer differently about the same layout, which a separately
        /// written counter eventually would.
        /// </remarks>
        /// <typeparam name="TKhareeta">The glyph map.</typeparam>
        /// <param name="talab">The layout, the styles and the destination space.</param>
        /// <param name="khareeta">Where each glyph lives in the atlas.</param>
        /// <returns>The exact counts, and every page the layout touches.</returns>
        /// <exception cref="ArgumentOutOfRangeException">
        /// As <see cref="Ibni{TKhareeta}"/>.
        /// </exception>
        public static NatijaNasij Ihsi<TKhareeta>(in TalabNasij talab, TKhareeta khareeta)
            where TKhareeta : IKhareetatAshkal
        {
            MakhzanNasij farigh = default;
            return Nafidh(in talab, khareeta, in farigh, uktub: false);
        }

        /// <summary>
        /// The one walk both entry points take, with the writes switched on or
        /// off.
        /// </summary>
        private static NatijaNasij Nafidh<TKhareeta>(
            in TalabNasij talab,
            TKhareeta khareeta,
            in MakhzanNasij makhzan,
            bool uktub)
            where TKhareeta : IKhareetatAshkal
        {
            NatijaNasij natija = default;
            natija.Kafa = true;

            float k = talab.Hayyiz.Qiyas;
            if (!(k > 0f))
            {
                throw new ArgumentOutOfRangeException(
                    nameof(talab), "Qiyas must be above zero; zero collapses every glyph.");
            }
            if (!(talab.Hajm > 0f) || !(talab.HajmLawha > 0f))
            {
                throw new ArgumentOutOfRangeException(
                    nameof(talab), "Hajm and HajmLawha must both be above zero.");
            }

            int adadHuruf = talab.Huruf.Length;
            if (uktub)
            {
                int hadRuus = adadHuruf * RuusLiShakl;
                int hadFahras = adadHuruf * FahrasLiShakl;
                if (makhzan.Ruus.Length < hadRuus
                    || makhzan.Malamis.Length < hadRuus
                    || makhzan.Alwan.Length < hadRuus
                    || makhzan.Muthallathat.Length < hadFahras)
                {
                    natija.Kafa = false;
                    natija.MatlubRuus = hadRuus;
                    natija.MatlubMuthallathat = hadFahras;
                    return natija;
                }
            }

            float aslS = (-talab.Hayyiz.MihwarS * talab.Hayyiz.Ard) + talab.Hayyiz.IzahaS;
            float aslA = ((1f - talab.Hayyiz.MihwarA) * talab.Hayyiz.Irtifa)
                - Fajwa(in talab.Hayyiz, k)
                + talab.Hayyiz.IzahaA;
            float nisba = talab.Hajm / talab.HajmLawha;
            ushort hajmRubi = HajmRubi(talab.HajmLawha);

            ushort memoNitaq = 0;
            uint memoAnqud = 0;
            LawnRasm memoLawn = talab.Lawn;
            bool memoSalih = false;

            bool badaHudud = false;
            float hududYasar = 0f;
            float hududYameen = 0f;
            float hududAsfal = 0f;
            float hududAla = 0f;

            bool sajjilMawaqi = uktub && !makhzan.MawaqiRuus.IsEmpty;
            if (sajjilMawaqi)
            {
                int hadMawaqi = makhzan.MawaqiRuus.Length < adadHuruf
                    ? makhzan.MawaqiRuus.Length
                    : adadHuruf;
                makhzan.MawaqiRuus.Slice(0, hadMawaqi).Fill(-1);
            }

            int adadSutur = talab.Sutur.Length;
            int mashi = adadSutur > 0 ? adadSutur : 1;
            for (int si = 0; si < mashi; si++)
            {
                int awwal = 0;
                int nihaya = adadHuruf;
                float irtifaSatr = 0f;
                if (adadSutur > 0)
                {
                    TaaribSatr satr = talab.Sutur[si];
                    awwal = satr.AwwalHarf > (uint)adadHuruf ? adadHuruf : (int)satr.AwwalHarf;
                    long baad = (long)awwal + satr.AdadHuruf;
                    nihaya = baad > adadHuruf ? adadHuruf : (int)baad;
                    irtifaSatr = satr.Irtifa;
                }

                for (int hi = awwal; hi < nihaya; hi++)
                {
                    TaaribHarf harf = talab.Huruf[hi];

                    int mawdiNitaq = MawdiNitaq(talab.Nitaqat, harf.Nitaq);
                    LawnRasm lawn;
                    if (talab.Nitaqat.IsEmpty)
                    {
                        lawn = talab.Lawn;
                    }
                    else if (memoSalih && memoNitaq == harf.Nitaq && memoAnqud == harf.Anqud)
                    {
                        lawn = memoLawn;
                    }
                    else
                    {
                        lawn = HallLawn(talab.Nitaqat, mawdiNitaq, harf.Anqud, talab.Lawn);
                        memoNitaq = harf.Nitaq;
                        memoAnqud = harf.Anqud;
                        memoLawn = lawn;
                        memoSalih = true;
                    }

                    if (mawdiNitaq >= 0
                        && (talab.Nitaqat[mawdiNitaq].Alam & Alamat.UslubDharra) != 0)
                    {
                        TaaribNitaqUslub nitaq = talab.Nitaqat[mawdiNitaq];
                        if (uktub && natija.AdadDharrat < makhzan.Dharrat.Length)
                        {
                            makhzan.Dharrat[natija.AdadDharrat] = MinDharra(
                                in nitaq, in harf, si, irtifaSatr, lawn, aslS, aslA, k);
                        }
                        natija.AdadDharrat++;
                        continue;
                    }

                    TaaribMiftahShakl miftah;
                    miftah.Muarrif = harf.Muarrif;
                    miftah.HajmRubi = hajmRubi;
                    miftah.Khatt = harf.Khatt;
                    miftah.Bakat = Bakat(harf.S, talab.Tathbit);

                    if (!khareeta.Shakl(in miftah, out TaaribMawdiShakl mawdi))
                    {
                        natija.AdadMafqud++;
                        continue;
                    }
                    if (mawdi.Ard == 0 || mawdi.Irtifa == 0)
                    {
                        natija.AdadFaragh++;
                        continue;
                    }
                    if (mawdi.Safha < 64)
                    {
                        natija.AlamSafahat |= 1UL << mawdi.Safha;
                    }
                    else
                    {
                        natija.SafahatBaida = true;
                    }
                    if (mawdi.Safha != talab.Safha)
                    {
                        continue;
                    }
                    if (!khareeta.Safha(mawdi.Safha, out QiyasSafha qiyasSafha)
                        || qiyasSafha.Ard == 0
                        || qiyasSafha.Irtifa == 0)
                    {
                        natija.AdadMafqud++;
                        continue;
                    }

                    float qalamS = talab.Tathbit ? (float)Math.Floor((double)harf.S) : harf.S;
                    float qalamA = talab.Tathbit
                        ? (float)Math.Floor((double)harf.A + 0.5)
                        : harf.A;
                    float sYasar = qalamS + (mawdi.IzahaS * nisba);
                    float aAla = qalamA - (mawdi.IzahaA * nisba);

                    float xL = aslS + (sYasar * k);
                    float xR = aslS + ((sYasar + (mawdi.Ard * nisba)) * k);
                    float yT = aslA - (aAla * k);
                    float yB = aslA - ((aAla + (mawdi.Irtifa * nisba)) * k);

                    float pArd = qiyasSafha.Ard;
                    float pIrtifa = qiyasSafha.Irtifa;
                    float uL = mawdi.S / pArd;
                    float uR = (mawdi.S + (float)mawdi.Ard) / pArd;
                    float vT = 1f - (mawdi.A / pIrtifa);
                    float vB = 1f - ((mawdi.A + (float)mawdi.Irtifa) / pIrtifa);

                    if (uktub)
                    {
                        Iktub(in makhzan, natija.AdadAshkal, xL, xR, yT, yB, uL, uR, vT, vB, lawn);
                        if (sajjilMawaqi && hi < makhzan.MawaqiRuus.Length)
                        {
                            makhzan.MawaqiRuus[hi] = natija.AdadAshkal * RuusLiShakl;
                        }
                    }
                    natija.AdadAshkal++;

                    if (!badaHudud)
                    {
                        badaHudud = true;
                        hududYasar = xL;
                        hududYameen = xR;
                        hududAsfal = yB;
                        hududAla = yT;
                    }
                    else
                    {
                        hududYasar = xL < hududYasar ? xL : hududYasar;
                        hududYameen = xR > hududYameen ? xR : hududYameen;
                        hududAsfal = yB < hududAsfal ? yB : hududAsfal;
                        hududAla = yT > hududAla ? yT : hududAla;
                    }
                }
            }

            natija.AdadRuus = natija.AdadAshkal * RuusLiShakl;
            natija.AdadMuthallathat = natija.AdadAshkal * FahrasLiShakl;
            natija.MatlubRuus = natija.AdadRuus;
            natija.MatlubMuthallathat = natija.AdadMuthallathat;
            natija.MatlubDharrat = natija.AdadDharrat;
            natija.Hudud.Yasar = hududYasar;
            natija.Hudud.Asfal = hududAsfal;
            natija.Hudud.Ard = hududYameen - hududYasar;
            natija.Hudud.Irtifa = hududAla - hududAsfal;
            return natija;
        }

        /// <summary>
        /// The vertical slack between the rectangle's top edge and the top of
        /// the laid-out block, in local units.
        /// </summary>
        private static float Fajwa(in HayyizRasm hayyiz, float k)
        {
            float mashghul = hayyiz.IrtifaTakhtit * k;
            switch (hayyiz.Muhadhaha)
            {
                case MuhadhahaRasiya.Wasat:
                    return (hayyiz.Irtifa - mashghul) * 0.5f;
                case MuhadhahaRasiya.Asfal:
                    return hayyiz.Irtifa - mashghul;
                default:
                    return 0f;
            }
        }

        /// <summary>
        /// Finds the span with a given identifier, or <c>-1</c>.
        /// </summary>
        /// <remarks>
        /// The markup bridge hands out identifiers in order from zero, so the
        /// identifier is almost always its own index and the first test
        /// answers. The scan behind it exists because a span table can also
        /// arrive from a compiled patch, where the identifiers are whatever
        /// the compiler recorded and nothing promises they are dense.
        /// </remarks>
        private static int MawdiNitaq(ReadOnlySpan<TaaribNitaqUslub> nitaqat, ushort id)
        {
            if (nitaqat.IsEmpty)
            {
                return -1;
            }
            if (id < nitaqat.Length && nitaqat[id].Id == id)
            {
                return id;
            }
            for (int i = 0; i < nitaqat.Length; i++)
            {
                if (nitaqat[i].Id == id)
                {
                    return i;
                }
            }
            return -1;
        }

        /// <summary>
        /// The colour a glyph draws in: its own span's, else the innermost
        /// enclosing span that sets one, else the component's own colour.
        /// </summary>
        /// <remarks>
        /// The second step is not redundant. A glyph inside
        /// <c>&lt;color=red&gt;a&lt;b&gt;b&lt;/b&gt;&lt;/color&gt;</c> carries
        /// the identifier of the bold span, which sets a weight and no colour;
        /// the red is on the span around it. The spans arrive sorted by start
        /// ascending and by length descending within a start, so a later entry
        /// that still contains the offset is nested more deeply, and taking
        /// the last match is taking the innermost one.
        /// </remarks>
        private static LawnRasm HallLawn(
            ReadOnlySpan<TaaribNitaqUslub> nitaqat,
            int mawdiNitaq,
            uint anqud,
            LawnRasm iftiradi)
        {
            if (mawdiNitaq >= 0 && (nitaqat[mawdiNitaq].Alam & Alamat.UslubLawn) != 0)
            {
                return LawnRasm.Min(nitaqat[mawdiNitaq].Lawn);
            }
            LawnRasm natija = iftiradi;
            for (int i = 0; i < nitaqat.Length; i++)
            {
                if ((nitaqat[i].Alam & Alamat.UslubLawn) == 0)
                {
                    continue;
                }
                uint bidaya = nitaqat[i].Bidaya;
                ulong nihaya = (ulong)bidaya + nitaqat[i].Tul;
                if (anqud >= bidaya && anqud < nihaya)
                {
                    natija = LawnRasm.Min(nitaqat[i].Lawn);
                }
            }
            return natija;
        }

        /// <summary>
        /// Where an inline atom landed, in the destination's local space.
        /// </summary>
        /// <remarks>
        /// None of these numbers is scaled by the atlas ratio the glyph
        /// rectangles use. An atom's width, height and baseline offset came
        /// from the layout, at the size the layout ran at, and never from a
        /// rasterized bitmap — there is no bitmap. Applying the atlas ratio
        /// here would resize every sprite in a distance-field patch by the
        /// ratio between the text size and the atlas's canonical size, which
        /// is a number that has nothing to do with sprites.
        /// </remarks>
        private static DharraMawduaa MinDharra(
            in TaaribNitaqUslub nitaq,
            in TaaribHarf harf,
            int satr,
            float irtifaSatr,
            LawnRasm lawn,
            float aslS,
            float aslA,
            float k)
        {
            // A width of zero means the markup bridge had no engine metrics to
            // give; the advance the layout reserved is the honest fallback. A
            // height of zero means the same, and the line box is what a sprite
            // with no metrics of its own occupies in every engine here.
            float ard = nitaq.ArdDharra > 0f ? nitaq.ArdDharra : harf.Taqaddum;
            float irtifa = nitaq.IrtifaDharra > 0f ? nitaq.IrtifaDharra : irtifaSatr;
            float aAsfal = harf.A - nitaq.AsasDharra;

            DharraMawduaa dharra;
            dharra.Nitaq = nitaq.Id;
            dharra.Marja = nitaq.MarjaDharra;
            dharra.Lawn = lawn;
            dharra.Satr = satr;
            dharra.Mustatil.Yasar = aslS + (harf.S * k);
            dharra.Mustatil.Asfal = aslA - (aAsfal * k);
            dharra.Mustatil.Ard = ard * k;
            dharra.Mustatil.Irtifa = irtifa * k;
            return dharra;
        }

        /// <summary>
        /// Writes one glyph's four vertices and six indices. The only place in
        /// the product that decides which corner is which.
        /// </summary>
        private static void Iktub(
            in MakhzanNasij makhzan,
            int shakl,
            float xL,
            float xR,
            float yT,
            float yB,
            float uL,
            float uR,
            float vT,
            float vB,
            LawnRasm lawn)
        {
            int r = shakl * RuusLiShakl;
            int f = shakl * FahrasLiShakl;

            NuqtaRasm ras;
            ras.Z = 0f;
            ras.S = xL;
            ras.A = yB;
            makhzan.Ruus[r + ZawiyatAsfalYasar] = ras;
            ras.A = yT;
            makhzan.Ruus[r + ZawiyatAlaYasar] = ras;
            ras.S = xR;
            makhzan.Ruus[r + ZawiyatAlaYameen] = ras;
            ras.A = yB;
            makhzan.Ruus[r + ZawiyatAsfalYameen] = ras;

            NuqtaMulmas mulmas;
            mulmas.U = uL;
            mulmas.V = vB;
            makhzan.Malamis[r + ZawiyatAsfalYasar] = mulmas;
            mulmas.V = vT;
            makhzan.Malamis[r + ZawiyatAlaYasar] = mulmas;
            mulmas.U = uR;
            makhzan.Malamis[r + ZawiyatAlaYameen] = mulmas;
            mulmas.V = vB;
            makhzan.Malamis[r + ZawiyatAsfalYameen] = mulmas;

            makhzan.Alwan[r + ZawiyatAsfalYasar] = lawn;
            makhzan.Alwan[r + ZawiyatAlaYasar] = lawn;
            makhzan.Alwan[r + ZawiyatAlaYameen] = lawn;
            makhzan.Alwan[r + ZawiyatAsfalYameen] = lawn;

            makhzan.Muthallathat[f + 0] = r + ZawiyatAsfalYasar;
            makhzan.Muthallathat[f + 1] = r + ZawiyatAlaYasar;
            makhzan.Muthallathat[f + 2] = r + ZawiyatAlaYameen;
            makhzan.Muthallathat[f + 3] = r + ZawiyatAlaYameen;
            makhzan.Muthallathat[f + 4] = r + ZawiyatAsfalYameen;
            makhzan.Muthallathat[f + 5] = r + ZawiyatAsfalYasar;
        }
    }
}
