// مقبض السياق — the engine context, held safely.
//
// Every native handle in this assembly is a SafeHandle, and no raw IntPtr to
// one is public anywhere. The native side generation-tags its handles, so a
// stale handle is *reported* (TAARIB_MAQBAD_BATIL) rather than executed — but
// that is the last line of defence, for the bug that got through. SafeHandle
// is what stops the bug being attempted at all: the reference count brackets
// every in-flight call, so a finalizer on another thread cannot destroy the
// context mid-layout, and destruction runs exactly once however many code
// paths dispose, leak, or crash.

using System;
using System.Runtime.InteropServices;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// An engine context — the layout cache, the pooled buffers, and the
    /// capture channel — as a counted, exactly-once-destroyed handle over
    /// the native <c>TaaribSiyaq</c>. Create with <see cref="Insha"/>;
    /// destruction happens on <see cref="SafeHandle.Dispose()"/> or, if the
    /// adapter never gets the chance, on finalization.
    /// </summary>
    /// <remarks>
    /// The native value is generation-tagged: were a stale copy ever handed
    /// back, the library would answer <see cref="Ramz.MaqbadBatil"/> instead
    /// of touching freed memory. This type exists so that answer is never
    /// needed — the marshaller reference-counts the handle around every
    /// call, and <c>ReleaseHandle</c> is the only caller of the destroy
    /// function. A context is safe to hand between threads; layout through
    /// it is one thread at a time, per the ABI's stated thread contract.
    /// </remarks>
    public sealed class MaqbadSiyaq : SafeHandle
    {
        /// <summary>
        /// The registered capture sink, rooted here for exactly as long as
        /// the native library holds its function pointer. The rooting is
        /// load-bearing, not bookkeeping: the unmanaged thunk behind
        /// <see cref="Marshal.GetFunctionPointerForDelegate{TDelegate}(TDelegate)"/>
        /// lives only while the delegate does, and a delegate collected
        /// while native code still holds the pointer is a call through
        /// freed memory on whichever frame next captures a string — long
        /// after the actual mistake. Written by
        /// <see cref="ShaghghilIltiqat"/>, cleared by
        /// <see cref="AwqifIltiqat"/>, the only two moments the
        /// registration changes.
        /// </summary>
        private TaaribRaddIltiqat? raddIltiqat;

        private MaqbadSiyaq()
            : base(IntPtr.Zero, ownsHandle: true)
        {
        }

        /// <summary>Whether the handle holds no live context.</summary>
        public override bool IsInvalid => handle == IntPtr.Zero;

        /// <summary>
        /// Creates a context. This is the first native work any adapter
        /// does, so it runs <see cref="Muhammil.Taakkad"/> first — the ABI
        /// version refusal cannot be bypassed by simply not calling the
        /// loader.
        /// </summary>
        /// <param name="khiyarat">
        /// The cache budget and pool sizing. The zero struct means no cache
        /// and default pools, which is what an offline compiler wants and a
        /// game never does — a game passes the budget its patch declares.
        /// </param>
        /// <returns>The owned handle.</returns>
        /// <exception cref="KhataTaarib">
        /// The library refused to load or verify
        /// (<see cref="Muhammil.Taakkad"/>), or creation failed — the
        /// exception carries the stashed code and sentences.
        /// </exception>
        public static MaqbadSiyaq Insha(in TaaribKhiyaratSiyaq khiyarat)
        {
            Muhammil.Taakkad();
            MaqbadSiyaq maqbad = new MaqbadSiyaq();
            int halat;
            IntPtr kharij;
            unsafe
            {
                TaaribKhiyaratSiyaq nuskha = khiyarat;
                halat = Jisr.taarib_siyaq_insha(&nuskha, out kharij);
            }
            Ramz.Tahaqqaq(halat);
            maqbad.SetHandle(kharij);
            return maqbad;
        }

        /// <summary>
        /// Empties the layout cache and pooled buffers without destroying
        /// the context. A patch reload calls this so a replaced string can
        /// never be served from a stale cached layout — the failure that
        /// presents as "the old translation flickers back".
        /// </summary>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
        public void Amsah()
        {
            Ramz.Tahaqqaq(Jisr.taarib_siyaq_amsah(this));
        }

        /// <summary>
        /// Reads the layout cache's counters. Diagnostics only — a hit rate
        /// that never rises means the cache key is churning, and this is how
        /// an adapter notices before a player does.
        /// </summary>
        /// <returns>The counters at this moment.</returns>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
        public TaaribIhsaatKhazina IhsaatMakhzan()
        {
            int halat;
            TaaribIhsaatKhazina ihsaat = default;
            unsafe
            {
                halat = Jisr.taarib_makhzan_ihsaat(this, &ihsaat);
            }
            Ramz.Tahaqqaq(halat);
            return ihsaat;
        }

        /// <summary>
        /// Starts the runtime string capture channel — the input to
        /// extraction's runtime capture and to per-size patch compilation.
        /// Every layout request this context has not seen before is
        /// reported to <paramref name="radd"/> before the layout call
        /// returns; cache misses are precisely "new text", so the stream
        /// self-deduplicates and a steady-state frame reports nothing,
        /// which is what makes capture safe to leave running during play.
        /// Registering again replaces the previous sink.
        /// </summary>
        /// <remarks>
        /// <para>
        /// The sink runs on whichever thread called layout, while that
        /// context's lock is held. Both halves of the registration
        /// contract are the implementor's to keep: return promptly,
        /// because a frame is waiting on it, and never call back into
        /// Taarib on any handle — the lock it would need is the lock it is
        /// already running under, and the deadlock lands on the render
        /// thread with no diagnostic beyond a frozen game.
        /// </para>
        /// <para>
        /// This context roots <paramref name="radd"/> for as long as it is
        /// registered, so the caller keeps no reference of its own — a
        /// delegate collected while native code holds its function pointer
        /// is a crash on whichever frame next captures a string, long
        /// after the actual mistake, and holding the reference here is
        /// what makes that mistake unwritable.
        /// </para>
        /// </remarks>
        /// <param name="radd">The sink, invoked under the contract above.</param>
        /// <param name="mustakhdim">
        /// Passed back to the sink verbatim; never dereferenced by the
        /// library.
        /// </param>
        /// <exception cref="ArgumentNullException"><paramref name="radd"/> is null.</exception>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
        public void ShaghghilIltiqat(TaaribRaddIltiqat radd, IntPtr mustakhdim)
        {
            if (radd is null)
            {
                throw new ArgumentNullException(nameof(radd));
            }
            IntPtr muashir = Marshal.GetFunctionPointerForDelegate(radd);
            Ramz.Tahaqqaq(Jisr.taarib_iltiqat_shaghghil(this, muashir, mustakhdim));
            // Rooted only after the library accepted the registration: on a
            // failed call the previous sink, if any, is still the one
            // registered and still the one being kept alive.
            raddIltiqat = radd;
        }

        /// <summary>
        /// Stops the capture channel. Delivery happens under the same
        /// context lock this call takes, so when it returns the last
        /// report has been delivered and the sink is never invoked again —
        /// which is the one moment its delegate may stop being rooted, and
        /// it stops being rooted here.
        /// </summary>
        /// <exception cref="KhataTaarib">The native call failed.</exception>
        /// <exception cref="ObjectDisposedException">The handle is closed.</exception>
        public void AwqifIltiqat()
        {
            Ramz.Tahaqqaq(Jisr.taarib_iltiqat_awqif(this));
            raddIltiqat = null;
        }

        /// <summary>
        /// Destroys the context. Runs exactly once, only after every
        /// in-flight call and every dependent handle has released its
        /// reference — which is the entire point of routing destruction
        /// through <see cref="SafeHandle"/> instead of remembering to.
        /// </summary>
        /// <returns>Whether the native side reported clean destruction.</returns>
        protected override bool ReleaseHandle()
        {
            return Jisr.taarib_siyaq_ihdham(handle) == Ramz.Najah;
        }
    }
}
