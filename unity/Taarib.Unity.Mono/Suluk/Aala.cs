// آلة — the typewriter: a line revealed a piece at a time, out of a layout
// that was shaped once and is never shaped again.
//
// WHAT EVERY GAME DOES, AND WHY IT DESTROYS ARABIC.
//
// A dialogue box reveals its line one character at a time. Written the obvious
// way that is `nass.Substring(0, n)` handed to the text component each frame,
// with a fresh layout behind it. In Latin the result is fine. In Arabic it is
// wrong three times over, and the third failure is not a rendering artefact —
// it is the text visibly re-forming in front of the player.
//
//   - THE CUT LANDS INSIDE A CHARACTER. An index counted in `char` splits a
//     surrogate pair. An index counted in bytes — which is what any offset
//     that has been through the ABI is, because TaaribHarf.Anqud is a UTF-8
//     byte offset — splits a UTF-8 sequence. Either way the prefix ends in
//     half a codepoint and the shaper is handed text that does not exist.
//
//   - THE CUT LANDS BETWEEN A LETTER AND ITS MARK. A base carrying shadda and
//     fatha is one perceived character and three UTF-16 units. Revealing by
//     units draws the letter on one frame, the letter and its shadda on the
//     next, and the fatha on the one after: a diacritic arriving separately
//     from the letter it belongs to, orphaned for two frames, in a script
//     where the mark is not decoration but part of how the word is read.
//
//   - THE WORD RE-SHAPES AS IT GROWS. This is the one that matters. Arabic is
//     cursive, and the glyph a letter takes depends on its neighbours. Lay out
//     the prefix "ك" and the kaf is isolated; add the next letter and the same
//     kaf becomes initial and changes shape entirely; the letter that was
//     final one frame ago is medial now. Reveal a five-letter word letter by
//     letter and the player watches every letter already on screen change form
//     four more times. Nothing is mis-shaped on any single frame — each frame
//     is a correct layout of its own prefix — and the effect is still
//     unmistakably broken, because a word in this script is drawn as one
//     connected thing and this draws it as five different things in a row.
//
// WHAT THIS FILE DOES INSTEAD. Shape once, for the whole final string, and
// then reveal glyphs out of that finished layout. The shapes are the shapes
// the completed sentence will have, from the first frame to the last, because
// they are literally the same glyphs: nothing is re-shaped, so nothing can
// change form. A letter appears in its final joining form and stays in it.
//
// THREE THINGS THAT FOLLOW FROM DOING IT THAT WAY.
//
//   - REVEAL ORDER IS LOGICAL, DRAW ORDER IS VISUAL, AND THE TWO DIFFER. The
//     layout hands back glyphs in visual order — left to right on screen,
//     whatever direction the text runs. Revealing "the first n glyphs" is
//     therefore revealing the leftmost n, which for a right-to-left line means
//     the sentence appears from its end. The reveal has to run in logical
//     order, and the only thing tying a glyph back to its logical position is
//     TaaribHarf.Anqud, the byte offset of the cluster it came from. So this
//     file builds one table — for each glyph, the reveal step it belongs to —
//     and the mesh path asks that table per glyph instead of comparing indices.
//     For a pure right-to-left run the answer happens to coincide with "in
//     glyph order"; for a sentence with an embedded Latin number or a quoted
//     English name it does not, and a reveal built on glyph order would show
//     the number before the words that introduce it.
//
//   - A CLUSTER IS THE UNIT, NOT A GLYPH AND NOT A `char`. Every glyph of a
//     cluster carries that cluster's byte offset — a combining mark carries
//     its base's offset, and a lam-alef ligature carries the offset of the lam
//     — so mapping glyphs through the cluster table makes a base and its marks
//     appear on the same frame, and makes a ligature appear whole. That is not
//     a nicety: half a ligature does not exist as a shape, and a mark whose
//     base has not appeared yet is drawn hanging in the air.
//
//   - PUNCTUATION PAUSES ARE KEYED LOGICALLY TOO. The full stop that ends an
//     Arabic sentence is at the logical end and the visual left. A pause table
//     built by walking glyphs would put the beat at the wrong end of the line.
//
// WHAT ALLOCATES. Hayyi allocates, and only when its two buffers are too small
// for this string; after the longest line of a conversation has been through
// it once, it never allocates again. Qaddim, Akmil, Takhatti, Aid, Ajbir and
// Zahir allocate nothing at all — Qaddim is the per-frame path and it is a
// subtraction, a comparison and, on the frames where something appears, one
// addition per revealed unit.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Mono.Suluk
{
    /// <summary>
    /// نمط الكشف — how much of the line appears at once.
    /// </summary>
    /// <remarks>
    /// Both modes step in whole grapheme clusters; the difference is how many
    /// clusters one step covers. Neither mode can ever split a cluster, which
    /// is the property that keeps a letter and its tashkeel on the same frame.
    /// </remarks>
    public enum NamatKashf
    {
        /// <summary>
        /// One cluster at a time — the classic typewriter, and the mode a
        /// dialogue box wants.
        /// </summary>
        Anqud = 0,

        /// <summary>
        /// A whole word at a time, trailing spaces included. Chosen for text
        /// meant to be read rather than watched, and for languages where the
        /// per-letter effect is unpleasant at speed.
        /// </summary>
        Kalima = 1,
    }

    /// <summary>
    /// خيارات الآلة — the pacing decisions, all of them in seconds or in
    /// clusters per second, none of them in frames.
    /// </summary>
    /// <remarks>
    /// Nothing here is expressed per frame on purpose. A reveal measured in
    /// frames runs at half speed on a thirty-hertz console and at double on a
    /// hundred-and-twenty-hertz monitor, and the resulting dialogue either
    /// crawls or outruns its voice line.
    /// </remarks>
    public struct KhiyaratAala
    {
        /// <summary>
        /// Clusters revealed per second. Not <c>char</c>s per second: in this
        /// script a letter with two marks is three <c>char</c>s and one
        /// perceived character, and a rate expressed in the former runs three
        /// times too slowly through a vocalised line.
        /// </summary>
        /// <remarks>
        /// Zero or less means instantaneous — the whole line appears on the
        /// first <see cref="Aala.Qaddim(float)"/>, which is what a "text speed:
        /// instant" setting maps onto without a second code path.
        /// </remarks>
        public float Suraa;

        /// <summary>How much appears per step.</summary>
        public NamatKashf Namat;

        /// <summary>
        /// Extra seconds held after a unit that ends a sentence — full stop,
        /// exclamation, the Arabic question mark U+061F, the Urdu full stop
        /// U+06D4, an ellipsis.
        /// </summary>
        public float TawaqqufNihaya;

        /// <summary>
        /// Extra seconds held after a unit that ends on a comma-class mark —
        /// comma, semicolon, colon, and the Arabic U+060C and U+061B.
        /// </summary>
        public float TawaqqufFasila;

        /// <summary>
        /// Extra seconds held after a unit containing a line break, which is
        /// the beat between two lines of the same speech.
        /// </summary>
        public float TawaqqufSatr;

        /// <summary>
        /// The pacing a dialogue box gets when nobody has configured one:
        /// thirty clusters a second, one cluster per step, a third of a second
        /// after a sentence, an eighth after a comma, a quarter after a line
        /// break.
        /// </summary>
        /// <returns>The default options.</returns>
        public static KhiyaratAala Iftiradiya()
        {
            KhiyaratAala kh = default;
            kh.Suraa = 30f;
            kh.Namat = NamatKashf.Anqud;
            kh.TawaqqufNihaya = 0.33f;
            kh.TawaqqufFasila = 0.125f;
            kh.TawaqqufSatr = 0.25f;
            return kh;
        }
    }

    /// <summary>
    /// آلة — one typewriter over one already-laid-out line: the reveal table,
    /// the clock, and the four commands a game drives it with.
    /// </summary>
    /// <remarks>
    /// <para>
    /// This object holds no handle, no layout and no span. <see cref="Hayyi"/>
    /// is handed the finished glyphs, reads nothing from them but
    /// <see cref="TaaribHarf.Anqud"/>, and keeps only integers — which is why
    /// it is safe to keep one of these alive across frames while the
    /// <see cref="Takhtit"/> that produced the glyphs goes on to lay out
    /// something else. A typewriter that held the span would be reading
    /// whatever the next label overwrote it with.
    /// </para>
    /// <para>
    /// It also holds no notion of a frame. <see cref="Qaddim(float)"/> takes
    /// elapsed seconds, so the caller decides whether that is scaled or
    /// unscaled time — a pause menu that freezes <c>Time.timeScale</c> should
    /// freeze the reveal with it, and a cutscene that does not, should not.
    /// </para>
    /// </remarks>
    public sealed class Aala
    {
        /// <summary>
        /// The ceiling on either buffer, in elements. Sixty-five thousand
        /// reveal units is a wall of text no dialogue box has ever held; a
        /// string past it is refused rather than allocating without bound
        /// inside somebody's game.
        /// </summary>
        public const int AqsaAnasir = 1 << 16;

        /// <summary>
        /// One reveal unit: the logical span it covers, how many clusters it
        /// is worth for pacing, and the beat held after it.
        /// </summary>
        private struct Wahda
        {
            /// <summary>UTF-8 byte offset of the unit's first byte.</summary>
            public int BidayaBayt;

            /// <summary>One past its last byte.</summary>
            public int NihayaBayt;

            /// <summary>How many clusters it covers — one, in cluster mode.</summary>
            public int AdadAnaqid;

            /// <summary>Clusters through the end of this unit, running total.</summary>
            public int Tarakum;

            /// <summary>Extra seconds held after it.</summary>
            public float Tawaqquf;
        }

        private Wahda[] wahdat;
        private int[] khatwatHuruf;
        private KhiyaratAala khiyarat;
        private string nass;
        private int adadWahdat;
        private int adadHuruf;
        private int makshuf;
        private float dayn;
        private bool mutakhatta;
        private bool muhayya;
        private bool amil;
        private bool ballagha;

        /// <summary>
        /// Creates a typewriter with its initial capacities.
        /// </summary>
        /// <param name="khiyarat">The pacing.</param>
        /// <param name="siaatWahdat">
        /// Initial reveal-unit capacity. Size it for the longest line the call
        /// site expects; the growth in <see cref="Hayyi"/> corrects an
        /// underestimate once and then never again.
        /// </param>
        /// <param name="siaatHuruf">Initial glyph capacity.</param>
        /// <exception cref="ArgumentOutOfRangeException">
        /// A capacity is below one or above <see cref="AqsaAnasir"/>.
        /// </exception>
        public Aala(KhiyaratAala khiyarat, int siaatWahdat = 256, int siaatHuruf = 512)
        {
            if (siaatWahdat < 1 || siaatWahdat > AqsaAnasir)
            {
                throw new ArgumentOutOfRangeException(nameof(siaatWahdat));
            }
            if (siaatHuruf < 1 || siaatHuruf > AqsaAnasir)
            {
                throw new ArgumentOutOfRangeException(nameof(siaatHuruf));
            }
            this.khiyarat = khiyarat;
            wahdat = new Wahda[siaatWahdat];
            khatwatHuruf = new int[siaatHuruf];
            nass = string.Empty;
            amil = true;
        }

        /// <summary>
        /// Whether this typewriter is still working. One switches itself off
        /// after a failure it cannot attribute to the text, and from then on
        /// every glyph reads as visible — which shows the whole line at once
        /// rather than showing nothing, because a dialogue box that never
        /// finishes revealing is a game the player cannot leave.
        /// </summary>
        public bool Amil => amil;

        /// <summary>
        /// Whether a reveal table is present. False before the first
        /// successful <see cref="Hayyi"/> and after a refusal.
        /// </summary>
        public bool Muhayya => muhayya;

        /// <summary>The text the table was built from, in logical order.</summary>
        public string Nass => nass;

        /// <summary>How many reveal units the line has.</summary>
        public int AdadWahdat => adadWahdat;

        /// <summary>How many have been revealed.</summary>
        public int Makshuf => makshuf;

        /// <summary>Whether every unit has been revealed.</summary>
        public bool Intaha => !muhayya || makshuf >= adadWahdat;

        /// <summary>
        /// Whether the reveal was ended by <see cref="Takhatti"/> rather than
        /// by running its course. Games use this: a line the player skipped
        /// does not get its "line finished" flourish.
        /// </summary>
        public bool Mutakhatta => mutakhatta;

        /// <summary>The pacing. Assigning re-times the remainder of the line.</summary>
        /// <remarks>
        /// Changing the granularity mid-line does not rebuild the table — the
        /// units already exist and re-cutting them would move the boundary the
        /// reveal is currently sitting on, which reads as a stutter. Set it
        /// before <see cref="Hayyi"/>, or accept that the rest of this line
        /// keeps the granularity it started with.
        /// </remarks>
        public KhiyaratAala Khiyarat
        {
            get => khiyarat;
            set
            {
                khiyarat = value;
                dayn = ZamanWahda(makshuf);
            }
        }

        /// <summary>
        /// The reveal step of each glyph, parallel to the glyph array
        /// <see cref="Hayyi"/> was given: entry <c>i</c> is the number of units
        /// that must be revealed before glyph <c>i</c> is drawn. A view over
        /// this object's own buffer, valid until the next
        /// <see cref="Hayyi"/>.
        /// </summary>
        /// <remarks>
        /// Exposed so a mesh writer that is already walking glyphs in a tight
        /// loop can compare against <see cref="Makshuf"/> inline instead of
        /// calling <see cref="Zahir(int)"/> per glyph. Both answer the same
        /// question; this one is for the loop that cannot afford the call.
        /// </remarks>
        public ReadOnlySpan<int> KhatwatHuruf => new ReadOnlySpan<int>(khatwatHuruf, 0, adadHuruf);

        /// <summary>
        /// Builds the reveal table for one finished layout. The one method
        /// here that reads glyphs, and the one that may allocate.
        /// </summary>
        /// <remarks>
        /// <para>
        /// Call it once per line, after the whole final string has been laid
        /// out. Calling it again with the same text and the same glyphs is
        /// harmless and rewinds the reveal; calling it with a different string
        /// is how a conversation moves to its next line, and it means the new
        /// line was shaped once, as a whole, exactly like the first.
        /// </para>
        /// <para>
        /// Nothing about the glyphs is retained. Only
        /// <see cref="TaaribHarf.Anqud"/> is read, and only to decide which
        /// step each glyph belongs to.
        /// </para>
        /// </remarks>
        /// <param name="nassJadid">
        /// The complete final text, in logical order — the same string the
        /// layout was produced from. A different string produces a table whose
        /// offsets do not match the glyphs, which is detected below.
        /// </param>
        /// <param name="huruf">
        /// The finished glyphs, in visual order:
        /// <see cref="Takhtit.Huruf"/> or
        /// <see cref="Taarib.Unity.Mushtarak.Ruqaa.HurufTakhtit"/> handed
        /// straight through.
        /// </param>
        /// <returns>
        /// Whether a usable table is present afterwards. <c>false</c> means the
        /// reveal is off for this line and every glyph reads as visible.
        /// </returns>
        public bool Hayyi(string nassJadid, ReadOnlySpan<TaaribHarf> huruf)
        {
            if (!amil)
            {
                return false;
            }
            nassJadid ??= string.Empty;

            muhayya = false;
            adadWahdat = 0;
            adadHuruf = 0;
            makshuf = 0;
            dayn = 0f;
            mutakhatta = false;
            nass = nassJadid;

            if (!IbniWahdat(nassJadid))
            {
                return false;
            }
            if (!IbniKhutuwat(huruf))
            {
                return false;
            }

            muhayya = true;
            dayn = ZamanWahda(0);
            return true;
        }

        /// <summary>
        /// Advances the clock by one frame's elapsed seconds, revealing
        /// whatever that pays for. The per-frame path; allocates nothing.
        /// </summary>
        /// <param name="zamanSura">
        /// Seconds since the last call. Zero or less does nothing at all,
        /// which is what a paused frame and a frame with a bad delta both
        /// want — a negative delta that ran backwards through the loop below
        /// would un-reveal text.
        /// </param>
        /// <returns>
        /// Whether the revealed count changed, so a caller rebuilds its mesh on
        /// the frames that need it and leaves it alone on the frames that do
        /// not. A typewriter at thirty clusters a second on a sixty-hertz
        /// display returns <c>true</c> on half its frames.
        /// </returns>
        public bool Qaddim(float zamanSura)
        {
            if (!muhayya || makshuf >= adadWahdat || !(zamanSura > 0f))
            {
                return false;
            }

            int qabl = makshuf;
            dayn -= zamanSura;

            // The loop, not an if: one long frame after a load screen can pay
            // for many units at once, and a reveal that advanced by at most one
            // unit per frame would fall permanently behind its own clock.
            while (dayn <= 0f && makshuf < adadWahdat)
            {
                int munjaz = makshuf;
                makshuf++;
                if (makshuf >= adadWahdat)
                {
                    dayn = 0f;
                    break;
                }
                dayn += wahdat[munjaz].Tawaqquf + ZamanWahda(makshuf);
            }
            return makshuf != qabl;
        }

        /// <summary>
        /// Reveals the rest of the line at once, without marking it skipped —
        /// what a "text speed: instant" setting and a finished voice line both
        /// do.
        /// </summary>
        /// <returns>Whether anything was still hidden.</returns>
        public bool Akmil()
        {
            if (!muhayya || makshuf >= adadWahdat)
            {
                return false;
            }
            makshuf = adadWahdat;
            dayn = 0f;
            return true;
        }

        /// <summary>
        /// Reveals the rest of the line and records that the player asked for
        /// it, which <see cref="Mutakhatta"/> reports.
        /// </summary>
        /// <returns>Whether anything was still hidden.</returns>
        public bool Takhatti()
        {
            bool ghayyar = Akmil();
            mutakhatta = true;
            return ghayyar;
        }

        /// <summary>
        /// Rewinds to nothing revealed, keeping the table. Replays the same
        /// line without shaping it again.
        /// </summary>
        public void Aid()
        {
            if (!muhayya)
            {
                return;
            }
            makshuf = 0;
            mutakhatta = false;
            dayn = ZamanWahda(0);
        }

        /// <summary>
        /// Forces the revealed count, clamped into the line — the shape a
        /// game's own <c>maxVisibleCharacters</c> assignment maps onto when
        /// the game, not this clock, is driving the reveal.
        /// </summary>
        /// <param name="wahdatMakshufa">How many units should be visible.</param>
        /// <returns>Whether the count changed.</returns>
        public bool Ajbir(int wahdatMakshufa)
        {
            if (!muhayya)
            {
                return false;
            }
            int hadd = wahdatMakshufa < 0 ? 0
                : (wahdatMakshufa > adadWahdat ? adadWahdat : wahdatMakshufa);
            if (hadd == makshuf)
            {
                return false;
            }
            makshuf = hadd;
            dayn = ZamanWahda(makshuf);
            return true;
        }

        /// <summary>
        /// Whether one glyph is drawn yet. The question the mesh path asks per
        /// glyph; a bounds check, an array read and a comparison.
        /// </summary>
        /// <param name="harf">
        /// The glyph's index in the array <see cref="Hayyi"/> was given.
        /// </param>
        /// <returns>
        /// Whether it is visible. An index outside the array, and every glyph
        /// of a typewriter that has switched itself off, reads as visible: an
        /// effect that fails should leave the sentence on screen, not remove
        /// it.
        /// </returns>
        public bool Zahir(int harf)
        {
            if (!muhayya)
            {
                return true;
            }
            if ((uint)harf >= (uint)adadHuruf)
            {
                return true;
            }
            return khatwatHuruf[harf] < makshuf;
        }

        /// <summary>
        /// How many grapheme clusters are revealed — the honest answer to "how
        /// many characters is this showing", and the number a game's own
        /// character counter should be fed.
        /// </summary>
        /// <returns>The cluster count, from zero to the line's total.</returns>
        public int AnaqidMakshufa()
        {
            if (!muhayya || makshuf <= 0)
            {
                return 0;
            }
            return wahdat[makshuf - 1].Tarakum;
        }

        /// <summary>
        /// The UTF-8 byte offset one past the last revealed unit.
        /// </summary>
        /// <remarks>
        /// For bookkeeping only — a caret that follows the reveal, a voice-line
        /// sync point, a capture record. It is not an invitation to cut the
        /// string here and lay the prefix out: doing that is the failure this
        /// whole file exists to prevent, and the number being a valid cluster
        /// boundary does not make the re-shaping any less visible.
        /// </remarks>
        /// <returns>The byte offset, from zero to the encoded length.</returns>
        public int BaytMakshuf()
        {
            if (!muhayya || makshuf <= 0)
            {
                return 0;
            }
            return wahdat[makshuf - 1].NihayaBayt;
        }

        /// <summary>
        /// Whether the table still describes a given string, so a caller can
        /// check cheaply that the text under it has not changed before
        /// trusting <see cref="Zahir(int)"/>.
        /// </summary>
        /// <param name="nassHali">The text to compare against.</param>
        /// <returns>Whether it is the text the table was built from.</returns>
        public bool Yutabiq(string? nassHali)
        {
            if (nassHali is null)
            {
                return false;
            }
            return ReferenceEquals(nass, nassHali)
                || string.Equals(nass, nassHali, StringComparison.Ordinal);
        }

        /// <summary>
        /// Switches this typewriter off, naming the reason once. Everything
        /// after it reports every glyph as visible, which leaves the sentence
        /// complete on screen and only costs the effect.
        /// </summary>
        /// <param name="sabab">What failed and what it costs, in one sentence.</param>
        public void Awqif(string sabab)
        {
            if (!amil)
            {
                return;
            }
            amil = false;
            muhayya = false;
            makshuf = 0;
            adadWahdat = 0;
            adadHuruf = 0;
            Rabt.Ballagh("Aala: " + sabab + "; the reveal is off for this text object only, "
                + "and its text is drawn complete.");
        }

        /// <summary>
        /// Reports a refusal that belongs to one line's arguments rather than
        /// to this object, and only the first time.
        /// </summary>
        /// <remarks>
        /// Attributable to the caller's arguments means the next line may be
        /// perfectly fine, so this does not switch the typewriter off the way
        /// <see cref="Awqif"/> does. It reports once for the same reason a
        /// per-frame log line is worse than no log line: a caller that prepares
        /// with the wrong layout does it on every line of a conversation, and a
        /// hundred identical lines in a BepInEx log train people to stop
        /// reading it.
        /// </remarks>
        private void Irfud(string sabab)
        {
            if (ballagha)
            {
                return;
            }
            ballagha = true;
            Rabt.Ballagh("Aala: " + sabab + "; the reveal degrades for this text object only, "
                + "and this is reported once.");
        }

        /// <summary>
        /// The seconds a given unit takes to type, before its own pause. Zero
        /// past the end of the line, and zero at any rate that is not above
        /// zero, which is how the instant setting falls out of the same clock.
        /// </summary>
        private float ZamanWahda(int fahras)
        {
            if (!(khiyarat.Suraa > 0f) || (uint)fahras >= (uint)adadWahdat)
            {
                return 0f;
            }
            return wahdat[fahras].AdadAnaqid / khiyarat.Suraa;
        }

        /// <summary>
        /// Cuts the text into reveal units, once, in one forward walk.
        /// </summary>
        /// <remarks>
        /// The walk carries the UTF-8 byte offset along with the UTF-16 index
        /// rather than converting each boundary through
        /// <see cref="Tarmiz.Bayt"/>, because that conversion is itself a scan
        /// from the start of the text and calling it per cluster would turn a
        /// linear cut into a quadratic one on the longest lines in the game.
        /// </remarks>
        private bool IbniWahdat(string nassJadid)
        {
            ReadOnlySpan<char> span = nassJadid.AsSpan();
            bool kalima = khiyarat.Namat == NamatKashf.Kalima;

            int mawdi = 0;
            int bayt = 0;
            int bidayaBayt = 0;
            int anaqid = 0;
            int tarakum = 0;
            char akhirDall = '\0';
            bool fihiSatr = false;

            while (mawdi < span.Length)
            {
                int nihaya = Anaqid.Nihaya(span, mawdi);
                if (nihaya <= mawdi)
                {
                    // Anaqid always advances; the guard is here so a future
                    // change to it cannot turn this into an infinite loop
                    // inside a game's frame.
                    nihaya = mawdi + 1;
                }
                bayt += TulBayt(span, mawdi, nihaya);
                anaqid++;
                tarakum++;

                for (int i = mawdi; i < nihaya; i++)
                {
                    char h = span[i];
                    if (h == '\n' || h == '\r' || h == '\u2028' || h == '\u2029')
                    {
                        fihiSatr = true;
                    }
                    else if (!char.IsWhiteSpace(h))
                    {
                        akhirDall = h;
                    }
                }

                bool aakhir = nihaya >= span.Length;
                bool yughliq = !kalima
                    || aakhir
                    || fihiSatr
                    || (char.IsWhiteSpace(span[mawdi]) && !char.IsWhiteSpace(span[nihaya]));

                if (yughliq)
                {
                    if (!Sia(ref wahdat, adadWahdat + 1))
                    {
                        Irfud("the line has more reveal units than any legitimate string");
                        return false;
                    }
                    Wahda wahda;
                    wahda.BidayaBayt = bidayaBayt;
                    wahda.NihayaBayt = bayt;
                    wahda.AdadAnaqid = anaqid;
                    wahda.Tarakum = tarakum;
                    wahda.Tawaqquf = Tawaqquf(akhirDall, fihiSatr);
                    wahdat[adadWahdat] = wahda;
                    adadWahdat++;

                    bidayaBayt = bayt;
                    anaqid = 0;
                    akhirDall = '\0';
                    fihiSatr = false;
                }
                mawdi = nihaya;
            }
            return true;
        }

        /// <summary>
        /// Maps every glyph to the unit it appears with, by the cluster offset
        /// it carries.
        /// </summary>
        /// <remarks>
        /// <para>
        /// A glyph whose offset falls past the end of the text is folded into
        /// the last unit rather than refused. Two things produce one, and they
        /// deserve different treatment than a hard failure: the ellipsis a
        /// truncating overflow policy appends, which genuinely belongs at the
        /// end of the line and appears there; and a caller who prepared with
        /// the previous line's layout, whose whole table is wrong but whose
        /// text still ends up completely visible once the reveal finishes.
        /// Refusing outright would leave a game with no reveal at all over a
        /// case where the honest degradation is one glyph arriving late.
        /// </para>
        /// <para>
        /// It is still reported, once, because the second cause is a bug in the
        /// caller's ordering and a silent fold would hide it.
        /// </para>
        /// </remarks>
        private bool IbniKhutuwat(ReadOnlySpan<TaaribHarf> huruf)
        {
            if (!Sia(ref khatwatHuruf, huruf.Length))
            {
                Irfud("the layout has more glyphs than any legitimate string");
                return false;
            }
            adadHuruf = huruf.Length;
            if (adadWahdat == 0)
            {
                if (adadHuruf > 0)
                {
                    // Glyphs for text this object was not given. Every one of
                    // them reads as visible from the first frame, which draws
                    // the line the game meant to draw and only costs the effect.
                    for (int i = 0; i < adadHuruf; i++)
                    {
                        khatwatHuruf[i] = 0;
                    }
                    Irfud("the layout carries glyphs for text this typewriter was not given");
                }
                return true;
            }

            int nihayatNass = wahdat[adadWahdat - 1].NihayaBayt;
            int kharij = 0;
            for (int i = 0; i < adadHuruf; i++)
            {
                uint anqud = huruf[i].Anqud;
                if (anqud >= (uint)nihayatNass)
                {
                    kharij++;
                    khatwatHuruf[i] = adadWahdat - 1;
                    continue;
                }
                khatwatHuruf[i] = MawdiWahda((int)anqud);
            }
            if (kharij > 0)
            {
                Irfud("the layout names clusters past the end of the text; those glyphs are "
                    + "revealed with the last unit of the line");
            }
            return true;
        }

        /// <summary>
        /// The unit a byte offset falls in, by binary search over the unit
        /// table — which is sorted and contiguous by construction, so the
        /// search is exact rather than nearest.
        /// </summary>
        private int MawdiWahda(int bayt)
        {
            int adna = 0;
            int aqsa = adadWahdat - 1;
            while (adna <= aqsa)
            {
                int wasat = adna + ((aqsa - adna) >> 1);
                if (bayt < wahdat[wasat].BidayaBayt)
                {
                    aqsa = wasat - 1;
                }
                else if (bayt >= wahdat[wasat].NihayaBayt)
                {
                    adna = wasat + 1;
                }
                else
                {
                    return wasat;
                }
            }
            // Unreachable for an offset inside the text, which IbniKhutuwat
            // has already established. The last unit is the safe answer: the
            // glyph appears with the end of the line rather than never.
            return adadWahdat - 1;
        }

        /// <summary>
        /// The beat held after a unit, from the last significant character it
        /// ended on. A unit that both ends a sentence and carries a line break
        /// takes the longer of the two rather than their sum, because the two
        /// pauses are the same pause described twice.
        /// </summary>
        private float Tawaqquf(char akhirDall, bool fihiSatr)
        {
            float tawaqquf = 0f;
            if (YunhiJumla(akhirDall))
            {
                tawaqquf = khiyarat.TawaqqufNihaya;
            }
            else if (YafsilJumla(akhirDall))
            {
                tawaqquf = khiyarat.TawaqqufFasila;
            }
            if (fihiSatr && khiyarat.TawaqqufSatr > tawaqquf)
            {
                tawaqquf = khiyarat.TawaqqufSatr;
            }
            return tawaqquf > 0f ? tawaqquf : 0f;
        }

        /// <summary>
        /// Whether a character ends a sentence. The Arabic question mark and
        /// the Urdu full stop are here for the obvious reason: a table built
        /// from ASCII punctuation alone gives an Arabic line no beats at all,
        /// and the reveal reads as one unbroken run-on.
        /// </summary>
        private static bool YunhiJumla(char h)
        {
            return h == '.' || h == '!' || h == '?'
                || h == '؟'
                || h == '۔'
                || h == '…'
                || h == '。'
                || h == '．';
        }

        /// <summary>Whether a character is a comma-class break.</summary>
        private static bool YafsilJumla(char h)
        {
            return h == ',' || h == ';' || h == ':'
                || h == '،'
                || h == '؛'
                || h == '、';
        }

        /// <summary>
        /// The UTF-8 length of one UTF-16 range, by the same rules
        /// <see cref="Tarmiz.Bayt"/> encodes with — including the lone
        /// surrogate, which becomes U+FFFD and costs three bytes on both
        /// sides, so the offsets this file computes and the offsets the layout
        /// reported stay in step.
        /// </summary>
        private static int TulBayt(ReadOnlySpan<char> nass, int bidaya, int nihaya)
        {
            int bayt = 0;
            for (int i = bidaya; i < nihaya; i++)
            {
                char harf = nass[i];
                if (harf < 0x0080)
                {
                    bayt += 1;
                }
                else if (harf < 0x0800)
                {
                    bayt += 2;
                }
                else if (char.IsHighSurrogate(harf) && i + 1 < nihaya
                    && char.IsLowSurrogate(nass[i + 1]))
                {
                    bayt += 4;
                    i++;
                }
                else
                {
                    bayt += 3;
                }
            }
            return bayt;
        }

        /// <summary>
        /// Grows a buffer to a required length, doubling so a slightly longer
        /// line next time does not allocate again. The only allocation in this
        /// file.
        /// </summary>
        /// <returns>
        /// Whether the requirement is inside <see cref="AqsaAnasir"/>.
        /// </returns>
        private static bool Sia<T>(ref T[] makhzan, int matlub)
        {
            if (matlub > AqsaAnasir)
            {
                return false;
            }
            if (matlub <= makhzan.Length)
            {
                return true;
            }
            long siaa = makhzan.Length < 1 ? 1 : makhzan.Length;
            while (siaa < matlub)
            {
                siaa *= 2;
            }
            makhzan = new T[siaa > AqsaAnasir ? AqsaAnasir : (int)siaa];
            return true;
        }
    }
}
