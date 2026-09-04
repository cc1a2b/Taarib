// مقبض اللوحة — the runtime glyph atlas, held safely.
//
// The atlas grows and evicts at runtime for the strings the patch compiler
// never saw — player names, composed text, capture mode. Its methods are the
// warm path: a glyph lookup happens per missing glyph, a frame mark happens
// once per frame, a page borrow happens per upload. None of them allocates a
// managed object, because atlas traffic spikes exactly when a frame is
// already busy — a scoreboard filling with player names is not the moment to
// hand the garbage collector new work.

using System;
using System.Runtime.InteropServices;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// A glyph atlas that grows and evicts at runtime, as a counted handle
    /// over the native <c>TaaribLawha</c>. Create with <see cref="Insha"/>.
    /// The native value is generation-tagged so a stale use is reported
    /// (<see cref="Ramz.MaqbadBatil"/>) rather than executed; this type is
    /// what stops the stale use being attempted in the first place.
    /// </summary>
    public sealed class MaqbadLawha : SafeHandle
    {
        private readonly MaqbadSiyaq siyaq;

        private MaqbadLawha(MaqbadSiyaq siyaq)
            : base(IntPtr.Zero, ownsHandle: true)
        {
            this.siyaq = siyaq;
        }

        /// <summary>Whether the handle holds no live atlas.</summary>
        public override bool IsInvalid => handle == IntPtr.Zero;

        /// <summary>
        /// Creates a runtime atlas.
        /// </summary>
        /// <param name="siyaq">The owning context.</param>
        /// <param name="aqsaArd">
        /// Maximum page width in texels — 4096 by default upstream, 2048 for
        /// the conservative GPU profile; the packer splits rather than
        /// scales past it.
        /// </param>
        /// <param name="aqsaIrtifa">Maximum page height in texels.</param>
        /// <param name="hashw">
        /// Padding around each glyph in texels, so bilinear sampling at the
        /// rectangle's edge never bleeds a neighbouring glyph into view.
        /// </param>
        /// <param name="namat">
        /// Coverage or signed distance field — the patch's declared mode,
        /// obeyed without asking.
        /// </param>
        /// <param name="mizaniya">
        /// The byte budget across all pages. When it is reached the atlas
        /// evicts least-recently-used rectangles rather than growing without
        /// bound inside someone else's game.
        /// </param>
        /// <returns>The owned handle.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="siyaq"/> is null.</exception>
        /// <exception cref="ObjectDisposedException"><paramref name="siyaq"/> is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public static MaqbadLawha Insha(
            MaqbadSiyaq siyaq,
            ushort aqsaArd,
            ushort aqsaIrtifa,
            ushort hashw,
            NamatLawha namat,
            nuint mizaniya)
        {
            if (siyaq is null)
            {
                throw new ArgumentNullException(nameof(siyaq));
            }

            MaqbadLawha maqbad = new MaqbadLawha(siyaq);
            bool madhkur = false;
            siyaq.DangerousAddRef(ref madhkur);
            try
            {
                int halat = Jisr.taarib_lawha_insha(
                    siyaq, aqsaArd, aqsaIrtifa, hashw, (uint)namat, mizaniya, out IntPtr kharij);
                Ramz.Tahaqqaq(halat);
                maqbad.SetHandle(kharij);
                // The context reference now belongs to the atlas handle.
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
        /// Marks the start of a frame. Every glyph looked up after this call
        /// is protected from eviction until the next one — the guarantee
        /// that no frame ever samples an atlas rectangle that was reassigned
        /// under it, which would present as one wrong glyph flickering in a
        /// menu, unreproducible. Call once per frame before the first
        /// <see cref="Shakl"/>.
        /// </summary>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        public void IbdaItar()
        {
            Ramz.Tahaqqaq(Jisr.taarib_lawha_ibda_itar(siyaq, this));
        }

        /// <summary>
        /// Looks a glyph up, rasterizing and packing it on a miss.
        /// Allocation-free, and safe from the render thread — this is the
        /// call that runs when a player name brings a glyph the compiler
        /// never saw.
        /// </summary>
        /// <param name="silsila">
        /// The chain the key's font index refers into — the same chain the
        /// layout that produced the key was shaped with, or the index names
        /// the wrong font.
        /// </param>
        /// <param name="miftah">What makes the rasterized image distinct.</param>
        /// <param name="mawdi">Where the glyph lives, and how to draw it.</param>
        /// <exception cref="ArgumentNullException"><paramref name="silsila"/> is null.</exception>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">
        /// The native call failed — including <see cref="Ramz.LawhaMumtalia"/>
        /// when the budget is exhausted and everything resident is
        /// referenced by the current frame.
        /// </exception>
        public void Shakl(MaqbadSilsila silsila, in TaaribMiftahShakl miftah, out TaaribMawdiShakl mawdi)
        {
            if (silsila is null)
            {
                throw new ArgumentNullException(nameof(silsila));
            }
            mawdi = default;
            int halat;
            unsafe
            {
                TaaribMiftahShakl nuskha = miftah;
                fixed (TaaribMawdiShakl* kharij = &mawdi)
                {
                    halat = Jisr.taarib_lawha_shakl(siyaq, this, silsila, &nuskha, kharij);
                }
            }
            Ramz.Tahaqqaq(halat);
        }

        /// <summary>
        /// Borrows one texture page for upload. The page's bytes stay valid
        /// only until the next call that can change the atlas — upload now,
        /// keep nothing, exactly as the borrow contract in <c>anwa.rs</c>
        /// states.
        /// </summary>
        /// <param name="fahras">The page index, below <see cref="AdadSafahat"/>.</param>
        /// <param name="safha">The borrowed page.</param>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public void Safha(ushort fahras, out TaaribSafha safha)
        {
            safha = default;
            int halat;
            unsafe
            {
                fixed (TaaribSafha* kharij = &safha)
                {
                    halat = Jisr.taarib_lawha_safha(siyaq, this, fahras, kharij);
                }
            }
            Ramz.Tahaqqaq(halat);
        }

        /// <summary>How many pages are open.</summary>
        /// <returns>The page count.</returns>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public uint AdadSafahat()
        {
            Ramz.Tahaqqaq(Jisr.taarib_lawha_adad_safahat(siyaq, this, out uint adad));
            return adad;
        }

        /// <summary>
        /// Reads the atlas counters. A miss count that keeps climbing after
        /// the first minutes of play means the patch compiler missed
        /// strings, and this is where the diagnostics that say so read from.
        /// </summary>
        /// <returns>The counters at this moment.</returns>
        /// <exception cref="ObjectDisposedException">A handle is closed.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        public TaaribIhsaatLawha Ihsaat()
        {
            int halat;
            TaaribIhsaatLawha ihsaat = default;
            unsafe
            {
                halat = Jisr.taarib_lawha_ihsaat(siyaq, this, &ihsaat);
            }
            Ramz.Tahaqqaq(halat);
            return ihsaat;
        }

        /// <summary>
        /// Destroys the atlas, then releases the context reference taken at
        /// creation — in that order, because the destroy call still needs
        /// the context alive.
        /// </summary>
        /// <returns>Whether the native side reported clean destruction.</returns>
        protected override bool ReleaseHandle()
        {
            bool najah = Jisr.taarib_lawha_ihdham(siyaq.DangerousGetHandle(), handle) == Ramz.Najah;
            siyaq.DangerousRelease();
            return najah;
        }
    }
}
