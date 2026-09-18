// فحص الرفع — what one frame costs to get onto the GPU.
//
// The adapter's begin-frame call used to mark every atlas page as needing a full
// upload, so a scene that had been unchanged for ten minutes re-uploaded every
// page in its entirety on every frame: four megabytes per 2048-square page,
// through LoadRawTextureData on Unity's main thread, inside somebody's game.
// Nothing about the atlas had changed; the page table had merely been told it
// had.
//
// JadwalSafahat is that page table with the native handle taken out of it, so
// what one frame uploads can be counted here rather than inferred from a frame
// graph. The cases below drive it exactly as Lawha drives it — register a page,
// record the glyphs a frame drew from, settle, collect, upload, mark clean —
// and count texels.
//
// Two of them are about the coupled hazard rather than the cost. Uploading only
// what changed is only safe if "what changed" includes the gutter the packer
// leaves around every glyph: a bilinear tap at a glyph's edge reads one texel
// outside its rectangle, the packer keeps that texel at zero on the CPU, and a
// GPU that was never sent the zeros keeps whatever the letter that occupied the
// space before it left there. The Rust side proves the rule against real
// rasterized pages in crates/taarib-lawha/tests/raf_al_wasikh.rs; what is
// checked here is the arithmetic that implements it.

using System;
using System.Collections.Generic;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Fahs
{
    /// <summary>The cases over what a frame uploads.</summary>
    public static class FahsRafa
    {
        /// <summary>The page size the plugin defaults to, in texels per side.</summary>
        private const int Bud = 2048;

        /// <summary>The gutter the packer leaves around every glyph.</summary>
        private const ushort Hashw = 1;

        /// <summary>How many frames a "static menu" measurement runs for.</summary>
        private const int Itarat = 60;

        /// <summary>Adds every case to the run.</summary>
        /// <param name="halat">The run.</param>
        /// <exception cref="ArgumentNullException"><paramref name="halat"/> is null.</exception>
        public static void Sajjil(List<Halat> halat)
        {
            if (halat is null)
            {
                throw new ArgumentNullException(nameof(halat));
            }

            halat.Add(new Halat("rafa.safha-jadida-kamila", SafhaJadidaKamila));
            halat.Add(new Halat("rafa.mashhad-thabit-la-yarfa", MashhadThabitLaYarfa));
            halat.Add(new Halat("rafa.shakl-jadid-yarfa-mustatilan", ShaklJadidYarfaMustatilan));
            halat.Add(new Halat("rafa.hashw-yudam", HashwYudam));
            halat.Add(new Halat("rafa.hashw-yuhsar-ala-hafat", HashwYuhsarAlaHafat));
            halat.Add(new Halat("rafa.ittihad-yaghti-ithnayn", IttihadYaghtiIthnayn));
            halat.Add(new Halat("rafa.rafa-fashil-yubqi-wasikh", RafaFashilYubqiWasikh));
            halat.Add(new Halat("rafa.tajil-yuid-kull-safha", TajilYuidKullSafha));
            halat.Add(new Halat("rafa.safha-thaniya-la-tulawwith-al-ula", SafhaThaniyaLaTulawwith));
        }

        /// <summary>
        /// A page the adapter has not created a texture for yet goes up whole,
        /// once. A freshly created texture's contents are undefined, so there is
        /// no smaller correct answer.
        /// </summary>
        private static string? SafhaJadidaKamila()
        {
            Mirsad mirsad = new Mirsad(1);
            mirsad.Itar(true, new Shakl(0, 10, 10, 16, 16));
            if (mirsad.AkhirKamila != 1)
            {
                return $"a new page produced {mirsad.AkhirKamila} whole upload(s), expected 1";
            }
            if (mirsad.AkhirTexelat != Bud * Bud)
            {
                return $"a new page uploaded {mirsad.AkhirTexelat} texels, expected {Bud * Bud}";
            }

            mirsad.Itar(true, new Shakl(0, 40, 10, 16, 16));
            if (mirsad.AkhirKamila != 0)
            {
                return "the page went up whole a second time";
            }
            return null;
        }

        /// <summary>
        /// The defect, measured. Sixty frames of a menu nobody touched: the same
        /// glyphs looked up, nothing rasterized, nothing evicted. The old rule
        /// uploaded every page in full on every one of them; the rule that
        /// replaced it uploads nothing at all after the first frame.
        /// </summary>
        private static string? MashhadThabitLaYarfa()
        {
            Shakl[] shasha = Shasha();

            Mirsad qadeem = new Mirsad(1);
            Mirsad jadeed = new Mirsad(1);
            for (int i = 0; i < Itarat; i++)
            {
                // The old begin-frame call: every page marked as needing a full
                // upload, whether or not a texel changed.
                qadeem.Ajjil();
                qadeem.Itar(i == 0, shasha);
                jadeed.Itar(i == 0, shasha);
            }

            long mutawaqqa = (long)Itarat * Bud * Bud;
            if (qadeem.Texelat != mutawaqqa)
            {
                return $"the old rule moved {qadeem.Texelat} texels over {Itarat} frames, "
                    + $"expected {mutawaqqa}";
            }
            if (jadeed.Texelat != (long)Bud * Bud)
            {
                return $"the new rule moved {jadeed.Texelat} texels over {Itarat} frames, "
                    + $"expected one whole page and nothing else";
            }
            if (jadeed.Juziya != 0)
            {
                return $"a static scene uploaded {jadeed.Juziya} rectangle(s) it did not have to";
            }
            return null;
        }

        /// <summary>
        /// One glyph rasterized into a warm page uploads that glyph's own
        /// rectangle and the gutter around it, and nothing else.
        /// </summary>
        private static string? ShaklJadidYarfaMustatilan()
        {
            Mirsad mirsad = new Mirsad(1);
            mirsad.Itar(true, Shasha());

            mirsad.Itar(true, new Shakl(0, 500, 600, 20, 30));
            if (mirsad.AkhirKamila != 0)
            {
                return "one new glyph forced a whole-page upload";
            }
            if (mirsad.AkhirJuziya != 1)
            {
                return $"one new glyph produced {mirsad.AkhirJuziya} rectangle upload(s)";
            }
            long mutawaqqa = (20 + (2 * Hashw)) * (long)(30 + (2 * Hashw));
            if (mirsad.AkhirTexelat != mutawaqqa)
            {
                return $"one new glyph moved {mirsad.AkhirTexelat} texels, expected {mutawaqqa} "
                    + "— its own rectangle grown by the gutter on all four sides";
            }
            return null;
        }

        /// <summary>
        /// The gutter is added on every side, because the tap that reads it can
        /// come from any of them.
        /// </summary>
        private static string? HashwYudam()
        {
            MustatilLawha shakl = new MustatilLawha(100, 200, 16, 24);
            MustatilLawha mamsuh = shakl.Wassi(Hashw, Bud, Bud);
            if (mamsuh.S != 99 || mamsuh.A != 199 || mamsuh.Ard != 18 || mamsuh.Irtifa != 26)
            {
                return $"expected 18x24+1 around 16x24+100+200, got {mamsuh}";
            }

            // A gutter of nothing changes nothing, which is what an atlas built
            // with no padding must get.
            MustatilLawha bila = shakl.Wassi(0, Bud, Bud);
            return bila == shakl ? null : $"a zero gutter changed {shakl} into {bila}";
        }

        /// <summary>
        /// A glyph against the page's edge grows inward only. A rectangle that
        /// started at minus one, or ran a texel past the last row, is a rectangle
        /// an adapter hands to a driver.
        /// </summary>
        private static string? HashwYuhsarAlaHafat()
        {
            MustatilLawha zawiya = new MustatilLawha(0, 0, 8, 8).Wassi(Hashw, 64, 64);
            if (zawiya.S != 0 || zawiya.A != 0 || zawiya.Ard != 9 || zawiya.Irtifa != 9)
            {
                return $"a glyph at the top-left grew to {zawiya}, expected 9x9+0+0";
            }

            MustatilLawha muqabil = new MustatilLawha(56, 56, 8, 8).Wassi(Hashw, 64, 64);
            if (muqabil.S != 55 || muqabil.A != 55 || muqabil.Ard != 9 || muqabil.Irtifa != 9)
            {
                return $"a glyph at the bottom-right grew to {muqabil}, expected 9x9+55+55";
            }
            if (!muqabil.Dakhil(64, 64))
            {
                return $"the grown rectangle {muqabil} runs outside a 64x64 page";
            }
            return null;
        }

        /// <summary>
        /// Two glyphs rasterized in one frame go up as one rectangle covering
        /// both, which is the bounding box the page table keeps on purpose.
        /// </summary>
        private static string? IttihadYaghtiIthnayn()
        {
            Mirsad mirsad = new Mirsad(1);
            mirsad.Itar(true, Shasha());
            mirsad.Itar(true, new Shakl(0, 100, 100, 10, 10), new Shakl(0, 130, 120, 10, 10));

            if (mirsad.AkhirJuziya != 1)
            {
                return $"two glyphs on one page produced {mirsad.AkhirJuziya} upload(s)";
            }
            MustatilLawha wasikh = mirsad.AkhirWasikh;
            if (wasikh.S != 99 || wasikh.A != 99 || wasikh.Ard != 42 || wasikh.Irtifa != 32)
            {
                return $"expected the box 42x32+99+99 around both glyphs and their gutters, "
                    + $"got {wasikh}";
            }
            return null;
        }

        /// <summary>
        /// An upload that failed leaves its page dirty. Marking it clean on
        /// collection would lose the change for good, and the glyph that was
        /// rasterized into it would never appear — which reads as a missing
        /// character rather than as a failed texture write.
        /// </summary>
        private static string? RafaFashilYubqiWasikh()
        {
            JadwalSafahat jadwal = new JadwalSafahat(Hashw);
            jadwal.Qayyid(0, Bud, Bud);
            jadwal.Wassikh(0, new MustatilLawha(10, 10, 16, 16));
            jadwal.Adrij(true);

            SijillRafa[] talabat = new SijillRafa[JadwalSafahat.AqsaSafahat];
            if (jadwal.Iltaqit(talabat) != 1)
            {
                return "the page was not offered for upload";
            }
            // The adapter's graphics call threw; Rufia is not reached.
            if (jadwal.Iltaqit(talabat) != 1)
            {
                return "a page whose upload was never acknowledged went clean on its own";
            }

            jadwal.Rufia(0);
            if (jadwal.Iltaqit(talabat) != 0)
            {
                return "the page stayed dirty after its upload was acknowledged";
            }
            return null;
        }

        /// <summary>
        /// Device-loss recovery still works. Nothing about the atlas changed, so
        /// the dirty tracking reports nothing — which is exactly why an adapter
        /// whose textures were recreated has to say so, and why removing the
        /// per-frame call did not remove the call.
        /// </summary>
        private static string? TajilYuidKullSafha()
        {
            Mirsad mirsad = new Mirsad(2);
            mirsad.Itar(true, new Shakl(0, 10, 10, 16, 16), new Shakl(1, 10, 10, 16, 16));
            mirsad.Itar(false, Array.Empty<Shakl>());
            if (mirsad.AkhirTexelat != 0)
            {
                return "a frame that drew nothing uploaded something";
            }

            mirsad.Ajjil();
            mirsad.Itar(false, Array.Empty<Shakl>());
            if (mirsad.AkhirKamila != 2)
            {
                return $"after a device loss {mirsad.AkhirKamila} page(s) went up whole, "
                    + "expected both";
            }
            if (mirsad.AkhirTexelat != 2L * Bud * Bud)
            {
                return $"after a device loss {mirsad.AkhirTexelat} texels went up, expected "
                    + $"{2L * Bud * Bud}";
            }
            return null;
        }

        /// <summary>
        /// A glyph on the atlas's second page dirties the second page and leaves
        /// the first alone. Pages are tracked apart because they are separate
        /// textures, and a page marked dirty by its neighbour is a whole texture
        /// re-uploaded for a glyph that is not on it.
        /// </summary>
        private static string? SafhaThaniyaLaTulawwith()
        {
            Mirsad mirsad = new Mirsad(2);
            mirsad.Itar(true, new Shakl(0, 10, 10, 16, 16), new Shakl(1, 10, 10, 16, 16));

            mirsad.Itar(true, new Shakl(1, 700, 700, 12, 12));
            if (mirsad.AkhirJuziya != 1)
            {
                return $"a glyph on page 1 produced {mirsad.AkhirJuziya} upload(s)";
            }
            if (mirsad.AkhirSafha != 1)
            {
                return $"a glyph on page 1 dirtied page {mirsad.AkhirSafha}";
            }
            return null;
        }

        /// <summary>
        /// What sixty frames of an unchanged screen cost under each rule, as
        /// numbers rather than as an assertion.
        /// </summary>
        /// <returns>One line per rule, in texels and in mebibytes.</returns>
        /// <remarks>
        /// The same drive <see cref="MashhadThabitLaYarfa"/> asserts, printed:
        /// the case proves the two numbers are what they are, and this is so a
        /// person can see what they are without reading the case.
        /// </remarks>
        public static string Qiyas()
        {
            Shakl[] shasha = Shasha();
            Mirsad qadeem = new Mirsad(1);
            Mirsad jadeed = new Mirsad(1);
            for (int i = 0; i < Itarat; i++)
            {
                qadeem.Ajjil();
                qadeem.Itar(i == 0, shasha);
                jadeed.Itar(i == 0, shasha);
            }

            return $"{Itarat} frame(s) of an unchanged screen, {shasha.Length} glyph(s) on one "
                + $"{Bud}x{Bud} page:" + Environment.NewLine
                + $"  begin-frame marks every page dirty: {Mib(qadeem.Texelat)}"
                + $" in {Itarat} whole-page upload(s)" + Environment.NewLine
                + $"  begin-frame releases pins only:    {Mib(jadeed.Texelat)}"
                + $" in 1 whole-page upload and {jadeed.Juziya} rectangle(s)";
        }

        /// <summary>A texel count with its size in mebibytes beside it.</summary>
        private static string Mib(long texelat)
        {
            // One byte per texel: coverage or a distance, both scalar.
            return $"{texelat} texel(s), {texelat / (1024.0 * 1024.0):0.##} MiB";
        }

        /// <summary>A screenful of glyphs, spread over one page the way a menu is.</summary>
        private static Shakl[] Shasha()
        {
            Shakl[] shasha = new Shakl[400];
            for (int i = 0; i < shasha.Length; i++)
            {
                shasha[i] = new Shakl(0, 8 + (i % 40 * 24), 8 + (i / 40 * 40), 20, 32);
            }
            return shasha;
        }

        /// <summary>One glyph's placement, as a frame hands it to the page table.</summary>
        private readonly struct Shakl
        {
            public Shakl(int safha, int s, int a, int ard, int irtifa)
            {
                Safha = safha;
                Mustatil = new MustatilLawha(s, a, ard, irtifa);
            }

            /// <summary>The page it sits on.</summary>
            public int Safha { get; }

            /// <summary>Its own rectangle, gutter excluded.</summary>
            public MustatilLawha Mustatil { get; }
        }

        /// <summary>
        /// A page table driven frame by frame, with the uploads counted.
        /// </summary>
        /// <remarks>
        /// The frame loop is the adapter's, spelled out: record what the frame
        /// drew from, settle the collected rectangles against whether the atlas
        /// wrote anything, collect, upload, mark clean. Only the upload itself is
        /// replaced — by addition.
        /// </remarks>
        private sealed class Mirsad
        {
            private readonly JadwalSafahat jadwal;
            private readonly SijillRafa[] talabat = new SijillRafa[JadwalSafahat.AqsaSafahat];

            public Mirsad(int safahat)
            {
                jadwal = new JadwalSafahat(Hashw);
                for (int i = 0; i < safahat; i++)
                {
                    jadwal.Qayyid(i, Bud, Bud);
                }
            }

            /// <summary>Texels uploaded over every frame so far.</summary>
            public long Texelat { get; private set; }

            /// <summary>Rectangle uploads over every frame so far.</summary>
            public int Juziya { get; private set; }

            /// <summary>Texels uploaded by the last frame.</summary>
            public long AkhirTexelat { get; private set; }

            /// <summary>Whole-page uploads in the last frame.</summary>
            public int AkhirKamila { get; private set; }

            /// <summary>Rectangle uploads in the last frame.</summary>
            public int AkhirJuziya { get; private set; }

            /// <summary>The rectangle the last frame's final upload covered.</summary>
            public MustatilLawha AkhirWasikh { get; private set; }

            /// <summary>The page the last frame's final upload was on.</summary>
            public int AkhirSafha { get; private set; }

            /// <summary>Marks every page as needing a full upload.</summary>
            public void Ajjil()
            {
                jadwal.Ajjil();
            }

            /// <summary>
            /// Runs one frame: these glyphs were drawn from, and the atlas either
            /// wrote something for them or served them from what it already held.
            /// </summary>
            /// <param name="kutiba">Whether the atlas rasterized, evicted or grew.</param>
            /// <param name="ashkal">The glyphs the frame drew from.</param>
            public void Itar(bool kutiba, params Shakl[] ashkal)
            {
                for (int i = 0; i < ashkal.Length; i++)
                {
                    jadwal.Wassikh(ashkal[i].Safha, ashkal[i].Mustatil);
                }
                jadwal.Adrij(kutiba);

                AkhirTexelat = 0;
                AkhirKamila = 0;
                AkhirJuziya = 0;
                int matlub = jadwal.Iltaqit(talabat);
                for (int i = 0; i < matlub; i++)
                {
                    SijillRafa talab = talabat[i];
                    AkhirTexelat += talab.Wasikh.Adad;
                    Texelat += talab.Wasikh.Adad;
                    if (talab.Jadida)
                    {
                        AkhirKamila++;
                    }
                    else
                    {
                        AkhirJuziya++;
                        Juziya++;
                    }
                    AkhirWasikh = talab.Wasikh;
                    AkhirSafha = talab.Safha;
                    jadwal.Rufia(talab.Safha);
                }
            }
        }
    }
}
