// مقبض السلسلة — the font fallback chain, held safely.
//
// A chain is the ordered list of fonts tried per character — the Arabic
// primary first, then whatever the patch declares for Latin and symbols. The
// chain is what a layout request names, and every glyph in the output names
// its font by index into it, which is how the atlas knows which font's
// outlines a glyph id belongs to. Like every handle here it is destroyed with
// (context, chain), so it holds a counted reference on its context for its
// whole life.

using System;
using System.Runtime.InteropServices;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// An ordered chain of fonts, tried in order per character, as a counted
    /// handle over the native <c>TaaribSilsila</c>. Create with
    /// <see cref="Insha"/>. The native value is generation-tagged so a stale
    /// use is reported (<see cref="Ramz.MaqbadBatil"/>) rather than
    /// executed; this type is what stops the stale use being attempted in
    /// the first place.
    /// </summary>
    public sealed class MaqbadSilsila : SafeHandle
    {
        /// <summary>
        /// The longest chain the ABI can express: glyphs name their font
        /// with an eight-bit index (<see cref="TaaribHarf.Khatt"/>).
        /// </summary>
        public const int AqsaTul = 256;

        private readonly MaqbadSiyaq siyaq;

        private MaqbadSilsila(MaqbadSiyaq siyaq)
            : base(IntPtr.Zero, ownsHandle: true)
        {
            this.siyaq = siyaq;
        }

        /// <summary>Whether the handle holds no live chain.</summary>
        public override bool IsInvalid => handle == IntPtr.Zero;

        /// <summary>
        /// Builds a chain from loaded fonts, first entry tried first. The
        /// chain takes its own native references to the fonts' shared byte
        /// buffers (Decision 4), so the managed font handles need not
        /// outlive it — disposing a <see cref="MaqbadKhatt"/> while a chain
        /// still uses that font is safe and ordinary.
        /// </summary>
        /// <param name="siyaq">The owning context.</param>
        /// <param name="khutut">
        /// The fonts, in fallback order; at least one, at most
        /// <see cref="AqsaTul"/>.
        /// </param>
        /// <returns>The owned handle.</returns>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="siyaq"/>, <paramref name="khutut"/>, or an
        /// element of it is null.
        /// </exception>
        /// <exception cref="ArgumentException">
        /// <paramref name="khutut"/> is empty or longer than
        /// <see cref="AqsaTul"/>.
        /// </exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public static MaqbadSilsila Insha(MaqbadSiyaq siyaq, params MaqbadKhatt[] khutut)
        {
            if (siyaq is null)
            {
                throw new ArgumentNullException(nameof(siyaq));
            }
            if (khutut is null)
            {
                throw new ArgumentNullException(nameof(khutut));
            }
            if (khutut.Length == 0)
            {
                throw new ArgumentException("A chain needs at least one font.", nameof(khutut));
            }
            if (khutut.Length > AqsaTul)
            {
                throw new ArgumentException(
                    $"A chain holds at most {AqsaTul} fonts; glyphs name their font with an eight-bit index.",
                    nameof(khutut));
            }
            for (int i = 0; i < khutut.Length; i++)
            {
                if (khutut[i] is null)
                {
                    throw new ArgumentNullException(nameof(khutut), $"Chain entry {i} is null.");
                }
            }

            MaqbadSilsila maqbad = new MaqbadSilsila(siyaq);
            bool madhkurSiyaq = false;
            int mudhaf = 0;
            siyaq.DangerousAddRef(ref madhkurSiyaq);
            try
            {
                // Each font is pinned by reference count for the duration of
                // the creating call only: after it, the chain's own native
                // references carry the fonts.
                for (int i = 0; i < khutut.Length; i++)
                {
                    bool madhkur = false;
                    khutut[i].DangerousAddRef(ref madhkur);
                    mudhaf++;
                }

                int halat;
                IntPtr kharij;
                unsafe
                {
                    IntPtr* muashirat = stackalloc IntPtr[khutut.Length];
                    for (int i = 0; i < khutut.Length; i++)
                    {
                        muashirat[i] = khutut[i].DangerousGetHandle();
                    }
                    halat = Jisr.taarib_silsila_insha(siyaq, muashirat, (nuint)khutut.Length, out kharij);
                }
                Ramz.Tahaqqaq(halat);
                maqbad.SetHandle(kharij);
                // The context reference transfers to the chain handle; the
                // font references are dropped in the finally below.
                madhkurSiyaq = false;
                return maqbad;
            }
            finally
            {
                for (int i = 0; i < mudhaf; i++)
                {
                    khutut[i].DangerousRelease();
                }
                if (madhkurSiyaq)
                {
                    siyaq.DangerousRelease();
                }
            }
        }

        /// <summary>
        /// Destroys the chain, then releases the context reference taken at
        /// creation — in that order, because the destroy call still needs
        /// the context alive.
        /// </summary>
        /// <returns>Whether the native side reported clean destruction.</returns>
        protected override bool ReleaseHandle()
        {
            bool najah = Jisr.taarib_silsila_ihdham(siyaq.DangerousGetHandle(), handle) == Ramz.Najah;
            siyaq.DangerousRelease();
            return najah;
        }
    }
}
