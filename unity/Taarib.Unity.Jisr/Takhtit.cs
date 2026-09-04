// التخطيط — the managed side of the hot path.
//
// THE CONSTRAINT THIS FILE EXISTS TO ENFORCE: no managed allocation on the
// per-frame path. A layout runs for every text object every time it changes,
// and in capture mode for every text object every frame; a single allocation
// there becomes garbage-collector pressure with someone else's frame budget,
// and the resulting hitch is blamed on the game, not on Taarib. So:
//
//   - The glyph and line arrays are allocated once, owned by this object,
//     and grown only when the native side answers TAARIB_SIAT_QASIRA with
//     the required counts — the caller grows once and retries, and after
//     the first frames the buffer never grows again. There is deliberately
//     no method here that returns a fresh array, ever: results are exposed
//     as Span<T> views over the same reused arrays, so the convenient
//     allocating shape does not exist to be reached for.
//   - Text crosses as UTF-8 written into this object's own reused byte
//     buffer with Encoding.UTF8.GetBytes over spans — never through
//     Marshal.StringToHGlobalAnsi (which both allocates native memory per
//     call and destroys non-ASCII text, the very text this product exists
//     for) and never through a fresh byte[] per call.
//   - Everything native code reads is pinned with `fixed` for exactly the
//     duration of one call. The native library keeps no pointer past the
//     call, and neither does this file.
//
// One consequence to respect: a Takhtit instance, like the native layout
// path itself, is one thread at a time. Give each laying-out thread its own.

using System;
using System.Text;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// The pointer-free layout options — every decision of
    /// <see cref="TaaribKhiyarat"/> except the feature array, which travels
    /// separately as a span so no caller ever stores a pointer. The zero
    /// value is the documented default of every discriminant: automatic
    /// direction, detected language, no justification, leading-edge
    /// alignment, diacritics kept, digits untouched, overflow reported.
    /// </summary>
    public struct KhiyaratTakhtit
    {
        /// <summary>How the base direction is decided.</summary>
        public IttijahAsas Ittijah;

        /// <summary>The declared language, which drives localized letter forms.</summary>
        public LughaNass Lugha;

        /// <summary>How surplus width is absorbed when justifying.</summary>
        public NamatDabt Dabt;

        /// <summary>Where a line sits inside the available width.</summary>
        public Muhadhaha Muhadhaha;

        /// <summary>The diacritics policy.</summary>
        public SiyasatTashkeel Tashkeel;

        /// <summary>The digits policy.</summary>
        public SiyasatArqam Arqam;

        /// <summary>The overflow policy.</summary>
        public SiyasatTajawuz Tajawuz;

        /// <summary>The smallest size shrink-to-fit may use, in pixels.</summary>
        public float HajmAdna;

        /// <summary>Line height in pixels; zero or less lets the font's metrics decide.</summary>
        public float IrtifaSatr;

        /// <summary>Extra spacing between glyphs, applied after shaping so joining survives.</summary>
        public float TabaudAhruf;

        /// <summary>Extra spacing added to every space.</summary>
        public float TabaudKalimat;

        /// <summary>
        /// <see cref="Alamat.KhiyarHiwar"/>, <see cref="Alamat.KhiyarSatrWahid"/>.
        /// </summary>
        public uint Alam;
    }

    /// <summary>
    /// One layout request as managed code states it: text as characters,
    /// spans as spans, options without a pointer in sight.
    /// <see cref="Takhtit"/> turns this into the ABI's
    /// <see cref="TaaribTalab"/> over pinned memory for the duration of one
    /// native call. A ref struct on purpose — it holds spans, so it cannot
    /// be stored, boxed, or captured, which is exactly the lifetime the
    /// pinned request has.
    /// </summary>
    public ref struct TalabTakhtit
    {
        /// <summary>
        /// The text, logical order, markup and placeholders already lifted
        /// into <see cref="Nitaqat"/>. Encoded to UTF-8 into the reused
        /// buffer; invalid surrogates become U+FFFD rather than an
        /// exception, because game strings are not trusted to be well
        /// formed.
        /// </summary>
        public ReadOnlySpan<char> Nass;

        /// <summary>
        /// The style spans, with byte offsets into the UTF-8 encoding of
        /// <see cref="Nass"/>. Empty means one unstyled run.
        /// </summary>
        public ReadOnlySpan<TaaribNitaqUslub> Nitaqat;

        /// <summary>
        /// OpenType feature settings beyond the defaults. Empty means the
        /// defaults.
        /// </summary>
        public ReadOnlySpan<TaaribSifa> Sifat;

        /// <summary>The size in pixels.</summary>
        public float Hajm;

        /// <summary>
        /// The width available in pixels; zero or less means one line of
        /// whatever width the text needs.
        /// </summary>
        public float ArdMutah;

        /// <summary>The height available in pixels; zero or less means unbounded.</summary>
        public float IrtifaMutah;

        /// <summary>The decisions.</summary>
        public KhiyaratTakhtit Khiyarat;
    }

    /// <summary>
    /// The reusable layout buffer and the calls that fill it: one glyph
    /// array, one line array, one UTF-8 text buffer, allocated once and
    /// grown only through the <see cref="Ramz.SiatQasira"/> negotiation.
    /// Results are read as spans over the same arrays — there is no method
    /// that returns an array, so the per-frame path cannot allocate by
    /// accident. One instance serves one thread; give each laying-out
    /// thread its own.
    /// </summary>
    public sealed class Takhtit
    {
        /// <summary>
        /// The ceiling on either array, in elements. Four million glyphs is
        /// far past any real string; a required count above it means the
        /// negotiation itself has gone wrong, and the honest response is a
        /// thrown error rather than a gigabyte of quiet allocation inside a
        /// game.
        /// </summary>
        public const int AqsaAnasir = 1 << 22;

        private TaaribHarf[] huruf;
        private TaaribSatr[] sutur;
        private byte[] nassUtf8;
        private int adadHuruf;
        private int adadSutur;
        private float ard;
        private float irtifa;
        private float hajm;
        private uint alam;
        private TaaribTaqreerTajawuz tajawuz;

        /// <summary>
        /// Creates the buffer with its initial capacities. Size them for the
        /// longest text the call site expects — a menu label buffer and a
        /// dialogue-page buffer are different sizes on purpose — and the
        /// negotiation corrects any underestimate exactly once.
        /// </summary>
        /// <param name="siaatHuruf">Initial glyph capacity, in elements.</param>
        /// <param name="siaatSutur">Initial line capacity, in elements.</param>
        /// <param name="siaatNass">Initial UTF-8 text capacity, in bytes.</param>
        /// <exception cref="ArgumentOutOfRangeException">
        /// A capacity is less than one or above <see cref="AqsaAnasir"/>.
        /// </exception>
        public Takhtit(int siaatHuruf = 512, int siaatSutur = 16, int siaatNass = 2048)
        {
            if (siaatHuruf < 1 || siaatHuruf > AqsaAnasir)
            {
                throw new ArgumentOutOfRangeException(nameof(siaatHuruf));
            }
            if (siaatSutur < 1 || siaatSutur > AqsaAnasir)
            {
                throw new ArgumentOutOfRangeException(nameof(siaatSutur));
            }
            if (siaatNass < 1 || siaatNass > AqsaAnasir)
            {
                throw new ArgumentOutOfRangeException(nameof(siaatNass));
            }
            huruf = new TaaribHarf[siaatHuruf];
            sutur = new TaaribSatr[siaatSutur];
            nassUtf8 = new byte[siaatNass];
        }

        /// <summary>
        /// The positioned glyphs of the last successful layout, in visual
        /// order — a view over the reused buffer, valid until the next call
        /// on this instance. Walk it straight into vertex writing; copying
        /// it out defeats the reason it is a span.
        /// </summary>
        public Span<TaaribHarf> Huruf => new Span<TaaribHarf>(huruf, 0, adadHuruf);

        /// <summary>
        /// The lines of the last successful layout — a view over the reused
        /// buffer, valid until the next call on this instance.
        /// </summary>
        public Span<TaaribSatr> Sutur => new Span<TaaribSatr>(sutur, 0, adadSutur);

        /// <summary>The width of the widest line of the last layout.</summary>
        public float Ard => ard;

        /// <summary>The total height of every line box of the last layout.</summary>
        public float Irtifa => irtifa;

        /// <summary>
        /// The size the text was finally laid out at — smaller than
        /// requested when shrink-to-fit ran.
        /// </summary>
        public float Hajm => hajm;

        /// <summary>The raw layout flags, per <c>Alamat.Takhtit*</c>.</summary>
        public uint Alam => alam;

        /// <summary>Whether the layout's overall direction is right to left.</summary>
        public bool Yameen => (alam & Alamat.TakhtitYameen) != 0;

        /// <summary>Whether the overflow policy truncated the text.</summary>
        public bool Maqsus => (alam & Alamat.TakhtitMaqsus) != 0;

        /// <summary>
        /// Whether the text did not fit — <see cref="Tajawuz"/> then says by
        /// how much.
        /// </summary>
        public bool Mutajawiz => (alam & Alamat.TakhtitTajawuz) != 0;

        /// <summary>
        /// Whether this layout came from the cache — the hit an adapter's
        /// diagnostics count.
        /// </summary>
        public bool MinMakhzan => (alam & Alamat.TakhtitMakhzan) != 0;

        /// <summary>
        /// The overflow measurement, meaningful only while
        /// <see cref="Mutajawiz"/> is true.
        /// </summary>
        public TaaribTaqreerTajawuz Tajawuz => tajawuz;

        /// <summary>The current glyph capacity, for diagnostics.</summary>
        public int SiaatHuruf => huruf.Length;

        /// <summary>The current line capacity, for diagnostics.</summary>
        public int SiaatSutur => sutur.Length;

        /// <summary>The current text buffer capacity in bytes, for diagnostics.</summary>
        public int SiaatNass => nassUtf8.Length;

        /// <summary>
        /// Lays the request out into this buffer. On the steady state this
        /// allocates nothing: the text is encoded into the reused byte
        /// buffer, every pointer is pinned for the one call, and the results
        /// land in the reused arrays. When the native side reports
        /// <see cref="Ramz.SiatQasira"/> the arrays grow — the only
        /// allocation this method can ever make — and the call retries;
        /// growth happens a handful of times in a session and then never
        /// again.
        /// </summary>
        /// <param name="siyaq">The context.</param>
        /// <param name="silsila">The font chain to shape and draw with.</param>
        /// <param name="talab">The request.</param>
        /// <exception cref="ArgumentNullException">A handle argument is null.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">
        /// The native call failed; the exception carries the stashed code
        /// and sentences. The buffer then reads as empty rather than as the
        /// previous layout, because stale glyphs drawn for a new string are
        /// a wrong render, not a fallback.
        /// </exception>
        public void Khattit(MaqbadSiyaq siyaq, MaqbadSilsila silsila, in TalabTakhtit talab)
        {
            if (siyaq is null)
            {
                throw new ArgumentNullException(nameof(siyaq));
            }
            if (silsila is null)
            {
                throw new ArgumentNullException(nameof(silsila));
            }

            adadHuruf = 0;
            adadSutur = 0;
            int tulNass = KtubNass(talab.Nass);

            bool madhkur = false;
            silsila.DangerousAddRef(ref madhkur);
            try
            {
                for (int muhawala = 0; ; muhawala++)
                {
                    int halat;
                    nuint matlubHuruf;
                    nuint matlubSutur;
                    unsafe
                    {
                        fixed (byte* nassM = nassUtf8)
                        fixed (TaaribNitaqUslub* nitaqatM = talab.Nitaqat)
                        fixed (TaaribSifa* sifatM = talab.Sifat)
                        fixed (TaaribHarf* hurufM = huruf)
                        fixed (TaaribSatr* suturM = sutur)
                        {
                            TaaribTalab t = default;
                            t.Nass = nassM;
                            t.TulNass = (nuint)tulNass;
                            t.Nitaqat = nitaqatM;
                            t.AdadNitaqat = (nuint)talab.Nitaqat.Length;
                            t.Silsila = silsila.DangerousGetHandle();
                            t.Hajm = talab.Hajm;
                            t.ArdMutah = talab.ArdMutah;
                            t.IrtifaMutah = talab.IrtifaMutah;
                            t.Khiyarat = MinKhiyarat(talab.Khiyarat, sifatM, (nuint)talab.Sifat.Length);

                            TaaribMakhzanTakhtit m = default;
                            m.Huruf = hurufM;
                            m.SiaatHuruf = (nuint)huruf.Length;
                            m.Sutur = suturM;
                            m.SiaatSutur = (nuint)sutur.Length;

                            halat = Jisr.taarib_takhtit(siyaq, &t, &m);

                            if (halat == Ramz.Najah)
                            {
                                // Clamp before trusting: a count past the
                                // declared capacity would be a native bug,
                                // and a span over it would throw far from
                                // the cause.
                                nuint h = m.AdadHuruf;
                                nuint s = m.AdadSutur;
                                adadHuruf = (int)(h > (nuint)huruf.Length ? (nuint)huruf.Length : h);
                                adadSutur = (int)(s > (nuint)sutur.Length ? (nuint)sutur.Length : s);
                                ard = m.Ard;
                                irtifa = m.Irtifa;
                                hajm = m.Hajm;
                                alam = m.Alam;
                                tajawuz = m.Tajawuz;
                                return;
                            }

                            matlubHuruf = m.AdadHuruf;
                            matlubSutur = m.AdadSutur;
                        }
                    }

                    if (halat != Ramz.SiatQasira)
                    {
                        Ramz.Tahaqqaq(halat);
                    }
                    if (muhawala >= 3)
                    {
                        // Growing to what was asked for and being told
                        // "still too small" four times is not negotiation.
                        throw new KhataTaarib(
                            Ramz.KhataAam,
                            string.Empty,
                            "ظل التخطيط يطلب سعة أكبر مما أُعطي بعد أربع محاولات؛ هذا خلل في تفاوض السعة يستحق البلاغ.",
                            "Layout kept demanding a larger capacity than it was given after four attempts; this is a capacity-negotiation bug worth reporting.",
                            Khutwa.FathTashkhis);
                    }
                    Kabbir(matlubHuruf, matlubSutur);
                }
            }
            finally
            {
                if (madhkur)
                {
                    silsila.DangerousRelease();
                }
            }
        }

        /// <summary>
        /// Measures the request without positioning a glyph — real shaped
        /// widths from the same pipeline, which is what an auto-size search
        /// iterates over. Reuses this buffer's text encoding and, like
        /// <see cref="Khattit"/>, allocates nothing on the steady state.
        /// </summary>
        /// <param name="siyaq">The context.</param>
        /// <param name="silsila">The font chain.</param>
        /// <param name="talab">The request.</param>
        /// <returns>The measurement.</returns>
        /// <exception cref="ArgumentNullException">A handle argument is null.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public TaaribQiyasNass Qis(MaqbadSiyaq siyaq, MaqbadSilsila silsila, in TalabTakhtit talab)
        {
            if (siyaq is null)
            {
                throw new ArgumentNullException(nameof(siyaq));
            }
            if (silsila is null)
            {
                throw new ArgumentNullException(nameof(silsila));
            }

            int tulNass = KtubNass(talab.Nass);

            bool madhkur = false;
            silsila.DangerousAddRef(ref madhkur);
            try
            {
                int halat;
                TaaribQiyasNass qiyas = default;
                unsafe
                {
                    fixed (byte* nassM = nassUtf8)
                    fixed (TaaribNitaqUslub* nitaqatM = talab.Nitaqat)
                    fixed (TaaribSifa* sifatM = talab.Sifat)
                    {
                        TaaribTalab t = default;
                        t.Nass = nassM;
                        t.TulNass = (nuint)tulNass;
                        t.Nitaqat = nitaqatM;
                        t.AdadNitaqat = (nuint)talab.Nitaqat.Length;
                        t.Silsila = silsila.DangerousGetHandle();
                        t.Hajm = talab.Hajm;
                        t.ArdMutah = talab.ArdMutah;
                        t.IrtifaMutah = talab.IrtifaMutah;
                        t.Khiyarat = MinKhiyarat(talab.Khiyarat, sifatM, (nuint)talab.Sifat.Length);

                        halat = Jisr.taarib_qiyas(siyaq, &t, &qiyas);
                    }
                }
                Ramz.Tahaqqaq(halat);
                return qiyas;
            }
            finally
            {
                if (madhkur)
                {
                    silsila.DangerousRelease();
                }
            }
        }

        /// <summary>
        /// Encodes the request text as UTF-8 into the reused buffer, growing
        /// it only when this text is longer than any before — after which
        /// that length never allocates again.
        /// </summary>
        /// <returns>The encoded length in bytes.</returns>
        private int KtubNass(ReadOnlySpan<char> nass)
        {
            if (nass.IsEmpty)
            {
                return 0;
            }
            // Three bytes per UTF-16 unit is the exact worst case, so one
            // comparison replaces a counting pass over the text.
            long aqsa = (long)nass.Length * 3;
            if (aqsa > AqsaAnasir)
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    string.Empty,
                    $"النص أطول من أي نص مشروع ({nass.Length} حرفًا)؛ رفض الترميز بدل حجز غير محدود داخل لعبة.",
                    $"The text is longer than any legitimate string ({nass.Length} chars); encoding is refused rather than allocating without bound inside a game.",
                    Khutwa.FathTashkhis);
            }
            if (aqsa > nassUtf8.Length)
            {
                nassUtf8 = new byte[UdluDarajatain((int)aqsa, nassUtf8.Length)];
            }
            return Encoding.UTF8.GetBytes(nass, nassUtf8);
        }

        /// <summary>
        /// Grows the result arrays to the counts the native side required,
        /// rounded up to a power of two so a slightly longer string next
        /// frame does not trigger another growth.
        /// </summary>
        private void Kabbir(nuint matlubHuruf, nuint matlubSutur)
        {
            long h = (long)(ulong)matlubHuruf;
            long s = (long)(ulong)matlubSutur;
            if (h <= 0 && s <= 0)
            {
                // SiatQasira with no requirement written is a native bug;
                // grow both anyway so the retry loop cannot spin in place.
                h = (long)huruf.Length * 2;
                s = (long)sutur.Length * 2;
            }
            if (h > AqsaAnasir || s > AqsaAnasir)
            {
                throw new KhataTaarib(
                    Ramz.Dhakira,
                    string.Empty,
                    $"طلب التخطيط سعة خارج كل حد معقول ({h} حرفًا و{s} سطرًا)؛ رفض الحجز.",
                    $"Layout demanded a capacity past every reasonable bound ({h} glyphs, {s} lines); the allocation is refused.",
                    Khutwa.FathTashkhis);
            }
            if (h > huruf.Length)
            {
                huruf = new TaaribHarf[UdluDarajatain((int)h, huruf.Length)];
            }
            if (s > sutur.Length)
            {
                sutur = new TaaribSatr[UdluDarajatain((int)s, sutur.Length)];
            }
        }

        /// <summary>
        /// The smallest power-of-two-times-current that satisfies the
        /// requirement, clamped to <see cref="AqsaAnasir"/>.
        /// </summary>
        private static int UdluDarajatain(int matlub, int hali)
        {
            long siaa = hali < 1 ? 1 : hali;
            while (siaa < matlub)
            {
                siaa *= 2;
            }
            return siaa > AqsaAnasir ? AqsaAnasir : (int)siaa;
        }

        /// <summary>
        /// Assembles the ABI options from the pointer-free managed options
        /// plus the feature pointer pinned by the caller for this one call.
        /// </summary>
        private static unsafe TaaribKhiyarat MinKhiyarat(in KhiyaratTakhtit kh, TaaribSifa* sifat, nuint adadSifat)
        {
            TaaribKhiyarat k = default;
            k.Sifat = sifat;
            k.AdadSifat = adadSifat;
            k.Ittijah = kh.Ittijah;
            k.Lugha = kh.Lugha;
            k.Dabt = kh.Dabt;
            k.Muhadhaha = kh.Muhadhaha;
            k.Tashkeel = kh.Tashkeel;
            k.Arqam = kh.Arqam;
            k.Tajawuz = kh.Tajawuz;
            k.HajmAdna = kh.HajmAdna;
            k.IrtifaSatr = kh.IrtifaSatr;
            k.TabaudAhruf = kh.TabaudAhruf;
            k.TabaudKalimat = kh.TabaudKalimat;
            k.Alam = kh.Alam;
            return k;
        }
    }
}
