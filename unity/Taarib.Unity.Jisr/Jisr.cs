// جسر — the P/Invoke surface, and the only one in the solution.
//
// Boundary 2 of ROADMAP section 4.4 has a mechanical form here: this class is
// the only place in Taarib.Unity.* that declares a DllImport. One surface is
// what makes the ABI version refusal unavoidable (there is no path around the
// loader), makes ownership statable once, and makes "no managed allocation on
// the per-frame path" a property you can confirm by reading a single file.
//
// Every import states the exact exported name, and CallingConvention.Cdecl is
// explicit on every one because the default (Winapi) means StdCall in a
// 32-bit Windows process — precisely the process this library must also run
// in — and a calling-convention mismatch there does not fail loudly; it
// corrupts the stack on the game's own frame.
//
// Where a native handle is an input, the parameter is typed as the concrete
// SafeHandle subclass: the marshaller then brackets the call with
// DangerousAddRef/DangerousRelease, so a finalizer on another thread cannot
// destroy the handle mid-call. Where a native handle is an output, or is the
// argument of its own destroy function, the parameter is a raw IntPtr —
// ref/out SafeHandle marshalling is not implemented on the Mono lineage Unity
// ships, and a destroy function is handed the value a SafeHandle is already
// releasing. Those IntPtrs never leave this assembly.
//
// The name "taarib_jisr" resolves per platform to taarib_jisr.dll,
// libtaarib_jisr.so, or libtaarib_jisr.dylib. Muhammil loads the correct
// architecture's file from the patch directory before the first import runs;
// the bare name then binds to the module already in the process. See
// Muhammil.cs for what that binding relies on, per platform.

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// The raw imports over the C ABI of <c>crates/taarib-jisr</c>, one per
    /// exported <c>taarib_*</c> function, named exactly as the export so the
    /// surface can be audited line for line against the generated
    /// <c>include/taarib.h</c>. Internal on purpose: everything public goes
    /// through the safe handles, <see cref="Takhtit"/> and
    /// <see cref="Muhammil"/>, which is where ownership and the version
    /// refusal are enforced.
    /// </summary>
    internal static unsafe class Jisr
    {
        /// <summary>
        /// The import name. Resolves to <c>taarib_jisr.dll</c> on Windows,
        /// <c>libtaarib_jisr.so</c> on Linux and
        /// <c>libtaarib_jisr.dylib</c> on macOS.
        /// </summary>
        internal const string Maktaba = "taarib_jisr";

        // -------------------------------------------------------------------
        // Version
        // -------------------------------------------------------------------

        /// <summary>
        /// Writes the ABI's major and minor version. The first call made
        /// after loading, before anything else — a caller with a mismatched
        /// major refuses to continue rather than continuing into undefined
        /// behaviour.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_abi_isdar(out uint kabir, out uint sagheer);

        /// <summary>
        /// Writes the human-readable version string into a caller-owned
        /// buffer, under the one string convention of the ABI: the required
        /// capacity (including a trailing NUL) goes to <c>matlub</c>, and a
        /// short buffer gets <see cref="Ramz.SiatQasira"/> and nothing
        /// written.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_isdar_nass(byte* hadaf, nuint siaa, out nuint matlub);

        // -------------------------------------------------------------------
        // The thread-local error stash
        // -------------------------------------------------------------------

        /// <summary>
        /// The status code of the calling thread's last failure, or
        /// <see cref="Ramz.Najah"/>. Thread-local so the render thread and a
        /// background loader cannot report each other's failures.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_khata_akhir();

        /// <summary>
        /// The next-action discriminant of the calling thread's last
        /// failure, per <c>raqm_khutwa</c> in <c>khata_c.rs</c>.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_khata_khutwa();

        /// <summary>
        /// Writes the permanent machine code (<c>TAARIB-E-1042</c>) of the
        /// calling thread's last failure, or an empty string when the
        /// failure carried no structured error.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_khata_ramz(byte* hadaf, nuint siaa, out nuint matlub);

        /// <summary>
        /// Writes the sentence of the calling thread's last failure in the
        /// requested language: 0 Arabic, 1 English, anything else Arabic.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_khata_nass(uint lugha, byte* hadaf, nuint siaa, out nuint matlub);

        // -------------------------------------------------------------------
        // Context
        // -------------------------------------------------------------------

        /// <summary>
        /// Creates a context — caches, pooled buffers, the capture channel.
        /// The caller owns the handle and destroys it with
        /// <see cref="taarib_siyaq_ihdham"/>.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_siyaq_insha(TaaribKhiyaratSiyaq* khiyarat, out IntPtr khuruj);

        /// <summary>
        /// Destroys a context. Raw IntPtr because this is what
        /// <see cref="MaqbadSiyaq"/> calls from ReleaseHandle, where the
        /// SafeHandle itself is the thing being released.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_siyaq_ihdham(IntPtr siyaq);

        /// <summary>
        /// Empties the context's layout cache and pooled buffers without
        /// destroying the context — what a patch reload wants, so stale
        /// layouts of replaced strings cannot be served.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_siyaq_amsah(MaqbadSiyaq siyaq);

        // -------------------------------------------------------------------
        // Fonts
        // -------------------------------------------------------------------

        /// <summary>
        /// Loads a font from bytes the caller owns for the duration of the
        /// call only — the library copies into its own shared, immutable
        /// buffer (Decision 4), so the managed array may be collected the
        /// moment this returns. <c>fahs_arabi</c> non-zero demands the
        /// Decision 6 validation: GSUB with the Arabic joining features,
        /// GPOS with mark attachment, declared coverage.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_khatt_min_dhakira(MaqbadSiyaq siyaq, byte* bayt, nuint tul, uint fahras, uint fahs_arabi, out IntPtr khuruj);

        /// <summary>Destroys a font handle. Called from ReleaseHandle only.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_khatt_ihdham(IntPtr siyaq, IntPtr khatt);

        /// <summary>
        /// The font's identity — the value of Decision 4, derived from the
        /// bytes themselves, which is what makes atlas keys and layout cache
        /// keys sound where a path or a family name would not be.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_khatt_huwiya(MaqbadSiyaq siyaq, MaqbadKhatt khatt, out ulong huwiya);

        /// <summary>The font's metrics scaled to a pixel size.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_khatt_qiyasat(MaqbadSiyaq siyaq, MaqbadKhatt khatt, float hajm, TaaribQiyasatKhatt* khuruj);

        /// <summary>
        /// Writes the font's family name, under the string convention.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_khatt_aila(MaqbadSiyaq siyaq, MaqbadKhatt khatt, byte* hadaf, nuint siaa, out nuint matlub);

        // -------------------------------------------------------------------
        // Font chains
        // -------------------------------------------------------------------

        /// <summary>
        /// Creates an ordered chain of fonts, tried in order per character.
        /// The chain takes its own native references to the fonts; the
        /// caller's font handles need not outlive it.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_silsila_insha(MaqbadSiyaq siyaq, IntPtr* khutut, nuint adad, out IntPtr khuruj);

        /// <summary>Destroys a chain handle. Called from ReleaseHandle only.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_silsila_ihdham(IntPtr siyaq, IntPtr silsila);

        // -------------------------------------------------------------------
        // Layout — the hot path
        // -------------------------------------------------------------------

        /// <summary>
        /// The hot path. Writes positioned glyphs and lines into the
        /// caller-owned buffer and allocates nothing; when the buffer is too
        /// small it writes only the required counts and returns
        /// <see cref="Ramz.SiatQasira"/>. There is deliberately no
        /// allocating variant of this function anywhere in the ABI, so no
        /// adapter can reach for one by accident. Every pointer inside
        /// <c>talab</c> and <c>makhzan</c> must stay pinned for the duration
        /// of the call and no longer — the library keeps none of them.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_takhtit(MaqbadSiyaq siyaq, TaaribTalab* talab, TaaribMakhzanTakhtit* makhzan);

        /// <summary>
        /// Measures the request without positioning a glyph — real shaped
        /// advances, never estimates, which is what makes auto-size searches
        /// converge on the truth.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_qiyas(MaqbadSiyaq siyaq, TaaribTalab* talab, TaaribQiyasNass* khuruj);

        /// <summary>The layout cache's counters, for diagnostics.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_makhzan_ihsaat(MaqbadSiyaq siyaq, TaaribIhsaatKhazina* khuruj);

        // -------------------------------------------------------------------
        // Atlas
        // -------------------------------------------------------------------

        /// <summary>
        /// Creates a runtime atlas with a maximum page dimension, glyph
        /// padding, rasterization mode and byte budget.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_lawha_insha(MaqbadSiyaq siyaq, ushort aqsa_ard, ushort aqsa_irtifa, ushort hashw, uint namat, nuint mizaniya, out IntPtr khuruj);

        /// <summary>Destroys an atlas handle. Called from ReleaseHandle only.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_lawha_ihdham(IntPtr siyaq, IntPtr lawha);

        /// <summary>
        /// Marks the start of a frame. Everything looked up after this call
        /// is protected from eviction until the next one — the guarantee
        /// that no frame ever samples a rectangle that has been reassigned
        /// under it.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_lawha_ibda_itar(MaqbadSiyaq siyaq, MaqbadLawha lawha);

        /// <summary>
        /// Looks a glyph up, rasterizing and packing it on a miss. Safe to
        /// call from a render thread; may return
        /// <see cref="Ramz.LawhaMumtalia"/> when the budget is exhausted and
        /// everything resident is referenced by the current frame.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_lawha_shakl(MaqbadSiyaq siyaq, MaqbadLawha lawha, MaqbadSilsila silsila, TaaribMiftahShakl* miftah, TaaribMawdiShakl* khuruj);

        /// <summary>
        /// Borrows one texture page. The page's byte pointer stays valid
        /// only until the next call that can change the atlas.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_lawha_safha(MaqbadSiyaq siyaq, MaqbadLawha lawha, ushort fahras, TaaribSafha* khuruj);

        /// <summary>How many pages are open.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_lawha_adad_safahat(MaqbadSiyaq siyaq, MaqbadLawha lawha, out uint adad);

        /// <summary>The atlas counters, for diagnostics.</summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern unsafe int taarib_lawha_ihsaat(MaqbadSiyaq siyaq, MaqbadLawha lawha, TaaribIhsaatLawha* khuruj);

        // -------------------------------------------------------------------
        // Capture
        // -------------------------------------------------------------------

        /// <summary>
        /// Starts the runtime string capture channel: <c>radd</c> is an
        /// unmanaged function pointer invoked, non-blocking, for every
        /// string that reaches a takeover point, with <c>mustakhdim</c>
        /// passed back verbatim. The caller keeps whatever produced
        /// <c>radd</c> alive until <see cref="taarib_iltiqat_awqif"/>.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_iltiqat_shaghghil(MaqbadSiyaq siyaq, IntPtr radd, IntPtr mustakhdim);

        /// <summary>
        /// Stops the capture channel. After this returns the function
        /// pointer is never invoked again and may be released.
        /// </summary>
        [DllImport(Maktaba, CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
        internal static extern int taarib_iltiqat_awqif(MaqbadSiyaq siyaq);

        // -------------------------------------------------------------------
        // The string convention, walked once
        // -------------------------------------------------------------------

        /// <summary>
        /// The shape shared by every string-returning entry point: write
        /// into a caller buffer, report the required capacity (including the
        /// trailing NUL) through <c>matlub</c>.
        /// </summary>
        private unsafe delegate int KatibNass(byte* hadaf, nuint siaa, out nuint matlub);

        /// <summary>Bound once so retrieval does not rebuild delegates per failure.</summary>
        private static readonly KatibNass KatibRamz = taarib_khata_ramz;

        /// <summary>Bound once. Writes the Arabic sentence.</summary>
        private static readonly KatibNass KatibArabi = KatibNassArabi;

        /// <summary>Bound once. Writes the English sentence.</summary>
        private static readonly KatibNass KatibInjilizi = KatibNassInjilizi;

        /// <summary>Bound once. Writes the library version string.</summary>
        private static readonly KatibNass KatibIsdar = taarib_isdar_nass;

        private static unsafe int KatibNassArabi(byte* hadaf, nuint siaa, out nuint matlub)
        {
            return taarib_khata_nass((uint)Lugha.Arabi, hadaf, siaa, out matlub);
        }

        private static unsafe int KatibNassInjilizi(byte* hadaf, nuint siaa, out nuint matlub)
        {
            return taarib_khata_nass((uint)Lugha.Injilizi, hadaf, siaa, out matlub);
        }

        /// <summary>
        /// Runs the capacity negotiation for one string-returning entry
        /// point and decodes the UTF-8 result. Allocates — every caller is
        /// an error path, a load-time path, or a diagnostics path, never the
        /// per-frame layout path. Never throws: a retrieval that fails
        /// yields an empty string, because retrieval runs while an exception
        /// is being built and a second failure there would bury the first.
        /// </summary>
        internal static unsafe string IqraNass(int ikhtiyar)
        {
            KatibNass katib;
            switch (ikhtiyar)
            {
                case 0: katib = KatibRamz; break;
                case 1: katib = KatibArabi; break;
                case 2: katib = KatibInjilizi; break;
                default: katib = KatibIsdar; break;
            }

            byte[] hadaf = new byte[256];
            for (int muhawala = 0; muhawala < 2; muhawala++)
            {
                int halat;
                nuint matlub;
                fixed (byte* p = hadaf)
                {
                    halat = katib(p, (nuint)hadaf.Length, out matlub);
                }

                if (halat == Ramz.Najah)
                {
                    // matlub counts the trailing NUL the C side appends.
                    if (matlub <= 1)
                    {
                        return string.Empty;
                    }
                    ulong tul = (ulong)matlub - 1;
                    if (tul > (ulong)hadaf.Length)
                    {
                        // The library and the buffer disagree; trust the buffer.
                        tul = (ulong)hadaf.Length;
                    }
                    return Encoding.UTF8.GetString(hadaf, 0, (int)tul);
                }

                if (halat == Ramz.SiatQasira && matlub > 1 && (ulong)matlub <= int.MaxValue)
                {
                    hadaf = new byte[(int)matlub];
                    continue;
                }

                break;
            }
            return string.Empty;
        }

        /// <summary>The calling thread's stashed permanent code, or empty.</summary>
        internal static string IqraRamzDaim()
        {
            return IqraNass(0);
        }

        /// <summary>The calling thread's stashed sentence, in the requested language.</summary>
        internal static string IqraNassKhata(Lugha lugha)
        {
            return IqraNass(lugha == Lugha.Injilizi ? 2 : 1);
        }

        /// <summary>The library's human-readable version string.</summary>
        internal static string IqraIsdarNass()
        {
            return IqraNass(3);
        }
    }
}
