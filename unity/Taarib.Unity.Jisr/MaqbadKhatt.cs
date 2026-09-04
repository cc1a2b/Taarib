// مقبض الخط — a loaded, validated font, held safely.
//
// A font handle is destroyed with (context, font), so this handle keeps a
// counted reference on its context from creation to release. That reference
// is what makes the destruction order unimportant: dispose the context first,
// last, or never, and the context's own native teardown still cannot run
// until this handle has released — so "destroy the context while a font call
// is in flight on another thread" is a bug this type makes unwritable rather
// than merely detectable.

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// A loaded, validated font as a counted handle over the native
    /// <c>TaaribKhatt</c>. Create with <see cref="MinDhakira"/>. The native
    /// value is generation-tagged so a stale use is reported
    /// (<see cref="Ramz.MaqbadBatil"/>) rather than executed; this type is
    /// what stops the stale use being attempted in the first place.
    /// </summary>
    public sealed class MaqbadKhatt : SafeHandle
    {
        private readonly MaqbadSiyaq siyaq;

        private MaqbadKhatt(MaqbadSiyaq siyaq)
            : base(IntPtr.Zero, ownsHandle: true)
        {
            this.siyaq = siyaq;
        }

        /// <summary>Whether the handle holds no live font.</summary>
        public override bool IsInvalid => handle == IntPtr.Zero;

        /// <summary>
        /// Loads a font from bytes — the patch's own bundled font, mapped or
        /// read by the adapter; never a game asset (Decision 5). The library
        /// copies into its own single shared buffer (Decision 4), so
        /// <paramref name="bayt"/> only needs to live for this call.
        /// </summary>
        /// <param name="siyaq">The owning context.</param>
        /// <param name="bayt">The font file bytes.</param>
        /// <param name="fahras">
        /// The face index inside a collection; zero for a plain font file.
        /// </param>
        /// <param name="fahsArabi">
        /// Whether to demand the Decision 6 validation: GSUB with the Arabic
        /// joining features, GPOS with mark attachment, declared coverage.
        /// True for the primary Arabic font — a font without those tables
        /// does not render ugly Arabic, it renders disconnected letters that
        /// are not Arabic at all, and this check converts that silent
        /// failure into a named rejection. False only for a Latin member of
        /// a fallback chain.
        /// </param>
        /// <returns>The owned handle.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="siyaq"/> is null.</exception>
        /// <exception cref="ArgumentException"><paramref name="bayt"/> is empty.</exception>
        /// <exception cref="ObjectDisposedException"><paramref name="siyaq"/> is closed.</exception>
        /// <exception cref="KhataTaarib">
        /// The font was rejected or unreadable — the exception names the
        /// missing table or feature, because "font error" is not a sentence
        /// anyone can act on.
        /// </exception>
        public static MaqbadKhatt MinDhakira(MaqbadSiyaq siyaq, ReadOnlySpan<byte> bayt, uint fahras, bool fahsArabi)
        {
            if (siyaq is null)
            {
                throw new ArgumentNullException(nameof(siyaq));
            }
            if (bayt.IsEmpty)
            {
                throw new ArgumentException(
                    "Font bytes are empty; a zero-length font cannot carry the tables Arabic requires.",
                    nameof(bayt));
            }

            MaqbadKhatt maqbad = new MaqbadKhatt(siyaq);
            bool madhkur = false;
            siyaq.DangerousAddRef(ref madhkur);
            try
            {
                int halat;
                IntPtr kharij;
                unsafe
                {
                    fixed (byte* muashir = bayt)
                    {
                        halat = Jisr.taarib_khatt_min_dhakira(
                            siyaq, muashir, (nuint)bayt.Length, fahras, fahsArabi ? 1u : 0u, out kharij);
                    }
                }
                Ramz.Tahaqqaq(halat);
                maqbad.SetHandle(kharij);
                // The context reference now belongs to the font handle and
                // is released in ReleaseHandle, after the destroy call.
                madhkur = false;
                return maqbad;
            }
            finally
            {
                if (madhkur)
                {
                    siyaq.DangerousRelease();
                }
            }
        }

        /// <summary>
        /// The font's identity, derived from its bytes (Decision 4). This —
        /// never a path, never a family name — is what atlas keys and cache
        /// keys carry, because two paths can resolve to different bytes
        /// tomorrow and an identity of bytes cannot.
        /// </summary>
        /// <returns>The identity.</returns>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        public ulong Huwiya()
        {
            Ramz.Tahaqqaq(Jisr.taarib_khatt_huwiya(siyaq, this, out ulong huwiya));
            return huwiya;
        }

        /// <summary>
        /// The font's metrics scaled to a pixel size — real values from the
        /// font's own tables, which is what container reflow and baseline
        /// alignment are computed from.
        /// </summary>
        /// <param name="hajm">The pixel size.</param>
        /// <returns>The scaled metrics.</returns>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        public TaaribQiyasatKhatt Qiyasat(float hajm)
        {
            int halat;
            TaaribQiyasatKhatt qiyasat = default;
            unsafe
            {
                halat = Jisr.taarib_khatt_qiyasat(siyaq, this, hajm, &qiyasat);
            }
            Ramz.Tahaqqaq(halat);
            return qiyasat;
        }

        /// <summary>
        /// The font's family name, for logs and diagnostics. Allocates the
        /// returned string — call it at load time or from a diagnostics
        /// screen, never on the per-frame path, which has no need of a name.
        /// </summary>
        /// <returns>The family name, or empty if the font declares none.</returns>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        public string Aila()
        {
            byte[] hadaf = new byte[128];
            for (int muhawala = 0; muhawala < 2; muhawala++)
            {
                int halat;
                nuint matlub;
                unsafe
                {
                    fixed (byte* muashir = hadaf)
                    {
                        halat = Jisr.taarib_khatt_aila(siyaq, this, muashir, (nuint)hadaf.Length, out matlub);
                    }
                }

                if (halat == Ramz.Najah)
                {
                    if (matlub <= 1)
                    {
                        return string.Empty;
                    }
                    ulong tul = (ulong)matlub - 1;
                    if (tul > (ulong)hadaf.Length)
                    {
                        tul = (ulong)hadaf.Length;
                    }
                    return Encoding.UTF8.GetString(hadaf, 0, (int)tul);
                }

                if (halat == Ramz.SiatQasira && matlub > 1 && (ulong)matlub <= int.MaxValue)
                {
                    hadaf = new byte[(int)matlub];
                    continue;
                }

                Ramz.Tahaqqaq(halat);
            }
            return string.Empty;
        }

        /// <summary>
        /// Destroys the font, then releases the context reference taken at
        /// creation — in that order, because the destroy call still needs
        /// the context alive.
        /// </summary>
        /// <returns>Whether the native side reported clean destruction.</returns>
        protected override bool ReleaseHandle()
        {
            bool najah = Jisr.taarib_khatt_ihdham(siyaq.DangerousGetHandle(), handle) == Ramz.Najah;
            siyaq.DangerousRelease();
            return najah;
        }
    }
}
