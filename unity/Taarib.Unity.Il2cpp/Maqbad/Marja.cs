// مرجع — holding an IL2CPP object across a frame, and the discipline that makes
// that safe.
//
// An Il2CppObjectBase, an Il2CppString*, an Il2CppArray* — every one of them is
// a raw pointer into a heap that a tracing collector owns. The collector is
// entitled to reclaim anything it cannot reach from a root, and Unity's IL2CPP
// runtime does not treat a pointer sitting in a .NET field as a root. It cannot:
// the BepInEx host runs a second, independent CLR inside the game process, and
// the IL2CPP collector has never heard of its heap. So a pointer this plugin
// stored last frame is, as far as the collector is concerned, a number nobody
// is using.
//
// WHY THIS IS THE BUG THAT PASSES TESTING. Unity's shipping IL2CPP runtime uses
// a conservative, non-moving collector. Non-moving means an object that survives
// a collection is at the same address afterwards, so a stale pointer to a live
// object keeps working. Conservative means the collector scans the native stack
// for anything that looks like a heap pointer and treats it as a root, so an
// object whose only reference is a pointer in a register or a stack slot often
// survives by accident. Between the two, holding raw pointers across frames
// works — on the developer's machine, in a short test, on the titles that were
// tried. It stops working when a game ships with the incremental collector
// enabled, when the heap is under pressure and a collection actually runs at a
// moment when the pointer is only in a managed field, or when Unity changes the
// collector, which it has announced it intends to. The failure is then a read of
// freed memory: not an exception, not a null, but plausible garbage that
// produces a wrong glyph index, a corrupted length, or a jump into whatever was
// allocated there since. "It worked in testing" is the expected symptom of
// getting this wrong, not evidence of getting it right.
//
// WHAT IS NOT SAFE, STATED ONCE SO IT CAN BE POINTED AT.
//
//   Holding a raw IL2CPP pointer across ANY call that can allocate. Not just
//   across a frame — across one call. Constructing an Il2CppString, boxing a
//   value, invoking a managed method through the runtime, adding to an
//   Il2CppList: any of these can trigger a collection, and a collection is
//   allowed to free everything unreachable and (on a moving collector) relocate
//   everything else. A pointer read before such a call and used after it is
//   already wrong; it merely has not been caught yet.
//
//   Storing a pointer in a static, a field, a captured local, or a Dictionary
//   key. All four outlive the call that produced them, and none of them is a
//   root the IL2CPP collector can see.
//
//   Passing a pointer to a background thread. The layout work and the native
//   library are happy off the main thread; IL2CPP object pointers are not,
//   because a thread the runtime has not attached is a thread whose stack the
//   collector does not scan.
//
// WHAT IS SAFE. A pointer used entirely within one call, with no allocating
// call in between — which is the ordinary case at a takeover point, where the
// string arrives as an argument, is copied out, and is never referred to again.
// And a pointer obtained through a Marja, which is a real root the collector
// knows about, re-read through the handle every time rather than cached.
//
// THE STRING RULE. Text crossing into the layout engine is copied out into a
// Taarib-owned buffer before anything can allocate — never handed across as a
// pointer into the IL2CPP heap. That is not caution; the layout call is where
// glyph buffers grow and where the native library can allocate, and a
// GC-relocated Il2CppString under an in-flight shaping call is a corrupted
// paragraph with no stack trace attached to it.

using System;
using System.Text;
using System.Threading;
using Il2CppInterop.Runtime;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Maqbad
{
    /// <summary>What kind of root a <see cref="Marja"/> installs.</summary>
    /// <remarks>
    /// <para>
    /// WHICH TO USE. <see cref="Qawi"/> is the default and is what almost
    /// everything wants: it keeps the object alive and costs the collector
    /// nothing beyond one more root to trace. Use it for anything held across a
    /// frame — a TMP_Text component the takeover is tracking, a Material, a
    /// cached font asset.
    /// </para>
    /// <para>
    /// <see cref="Thabit"/> keeps the object alive AND forbids the collector to
    /// relocate it, which is what makes an address stable enough to hand to
    /// native code that will hold it. That guarantee is not free: a pinned
    /// object is an object a compacting collector has to arrange the heap
    /// around, and enough of them fragment it. Use it only where an address
    /// genuinely escapes into native code that outlives the call — a pixel
    /// buffer being uploaded asynchronously, an array a native routine will
    /// write into after returning. Do not reach for it merely because pinning
    /// sounds safer; it is the more expensive tool, not the stronger one.
    /// </para>
    /// <para>
    /// <see cref="Daeef"/> does not keep the object alive. It answers "is this
    /// still here?" without being the reason it is, which is what a cache keyed
    /// by a game object wants: a weak marja to a destroyed TMP_Text reports dead
    /// and the entry is dropped, where a strong one would keep every text
    /// component the game ever created resident for the life of the process.
    /// </para>
    /// </remarks>
    public enum NawMarja
    {
        /// <summary>Keeps the object alive; the collector may still move it.</summary>
        Qawi = 0,

        /// <summary>Keeps the object alive and forbids relocation. Costs the collector.</summary>
        Thabit = 1,

        /// <summary>Observes without keeping alive. May report dead at any time.</summary>
        Daeef = 2,
    }

    /// <summary>
    /// A root the IL2CPP collector knows about, wrapping one object pointer for
    /// an explicit, disposable lifetime.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Create with <see cref="Ihjiz"/>, read through <see cref="Kaen"/>, and
    /// dispose when the reference is no longer needed. The intended shape is a
    /// <c>using</c> for a scope and a field plus explicit disposal for anything
    /// held across frames.
    /// </para>
    /// <para>
    /// <see cref="Kaen"/> asks the runtime through the handle on every read and
    /// never caches the pointer, which is the entire point of the type. A cached
    /// address would be exactly the raw pointer this class exists to replace,
    /// wearing a safer-looking name.
    /// </para>
    /// <para>
    /// Not thread-safe, and must be created and disposed on a thread the IL2CPP
    /// runtime has attached — which for a takeover means Unity's main thread.
    /// </para>
    /// </remarks>
    public sealed class Marja : IDisposable
    {
        private static int musarraba;

        // IntPtr, not uint: Il2CppInterop's gchandle entry points take and
        // return the runtime's own handle width, and narrowing it here would
        // truncate a handle on a 64-bit runtime that ever hands one back above
        // 2^32.
        private readonly IntPtr raqm;
        private int mutakhalla;

        private Marja(IntPtr raqm, NawMarja naw)
        {
            this.raqm = raqm;
            Naw = naw;
        }

        /// <summary>
        /// How many handles have been abandoned to the finalizer instead of
        /// being disposed, for the diagnostics bundle.
        /// </summary>
        /// <remarks>
        /// A non-zero value is a leak this build cannot repair at run time — see
        /// the finalizer for why it does not try — and it is reported rather
        /// than hidden because a slowly growing count is the visible end of a
        /// takeover that is pinning every string it ever saw.
        /// </remarks>
        public static int Musarraba => Volatile.Read(ref musarraba);

        /// <summary>What kind of root this is.</summary>
        public NawMarja Naw { get; }

        /// <summary>Whether this handle has been released.</summary>
        public bool Mutakhalla => Volatile.Read(ref mutakhalla) != 0;

        /// <summary>
        /// The object's current address, asked of the runtime on every read.
        /// </summary>
        /// <remarks>
        /// <see cref="IntPtr.Zero"/> for a weak marja whose object has been
        /// collected, and for a disposed one. Never cache the value: for a
        /// strong marja it is valid only until the next call that can allocate,
        /// and re-reading is one runtime call, which is nothing next to being
        /// wrong.
        /// </remarks>
        public IntPtr Kaen
        {
            get
            {
                if (Mutakhalla)
                {
                    return IntPtr.Zero;
                }
                return IL2CPP.il2cpp_gchandle_get_target(raqm);
            }
        }

        /// <summary>Whether the object is still reachable through this handle.</summary>
        public bool Hayy => Kaen != IntPtr.Zero;

        /// <summary>The raw runtime handle number, for a log line only.</summary>
        public IntPtr Raqm => raqm;

        /// <summary>
        /// Roots an IL2CPP object pointer for as long as this handle lives.
        /// </summary>
        /// <param name="kaen">
        /// The object pointer, valid at the moment of the call — which means no
        /// allocating call may have run between obtaining it and passing it
        /// here.
        /// </param>
        /// <param name="naw">Which kind of root to install.</param>
        /// <returns>The handle.</returns>
        /// <exception cref="KhataTaarib">
        /// <paramref name="kaen"/> is null, the IL2CPP runtime is not loaded, or
        /// the runtime refused to issue a handle.
        /// </exception>
        public static Marja Ihjiz(IntPtr kaen, NawMarja naw)
        {
            if (kaen == IntPtr.Zero)
            {
                throw new KhataTaarib(
                    Ramz.MuashirBatil,
                    "TAARIB-E-6410",
                    "طُلب حجز مرجع لكائن فارغ في IL2CPP؛ هذا خلل في الملحق وليس في اللعبة.",
                    "A Taarib GC handle was requested for a null IL2CPP object; this is a "
                    + "bug in the plugin, not in the game.",
                    Khutwa.FathTashkhis);
            }

            IntPtr raqm;
            try
            {
                raqm = naw == NawMarja.Daeef
                    ? IL2CPP.il2cpp_gchandle_new_weakref(kaen, false)
                    : IL2CPP.il2cpp_gchandle_new(kaen, naw == NawMarja.Thabit);
            }
            catch (Exception khata)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6411",
                    $"تعذّر حجز مرجع في جامع نفايات IL2CPP ({khata.GetType().Name}).",
                    $"A GC handle could not be taken in the IL2CPP collector "
                    + $"({khata.GetType().Name}: {khata.Message}).",
                    Khutwa.FathTashkhis);
            }

            if (raqm == IntPtr.Zero)
            {
                throw new KhataTaarib(
                    Ramz.Dhakira,
                    "TAARIB-E-6412",
                    "رفض جامع نفايات IL2CPP إصدار مقبض جديد؛ غالبًا لنفاد الذاكرة.",
                    "The IL2CPP collector refused to issue a handle, which usually means "
                    + "it is out of memory.",
                    Khutwa.FathTashkhis);
            }

            return new Marja(raqm, naw);
        }

        /// <summary>Roots an object strongly. The ordinary case.</summary>
        /// <param name="kaen">The object pointer.</param>
        /// <returns>The handle.</returns>
        /// <exception cref="KhataTaarib">See <see cref="Ihjiz"/>.</exception>
        public static Marja Qawi(IntPtr kaen)
        {
            return Ihjiz(kaen, NawMarja.Qawi);
        }

        /// <summary>
        /// Roots an object and forbids relocation. For addresses that escape
        /// into native code outliving the call, and nothing else.
        /// </summary>
        /// <param name="kaen">The object pointer.</param>
        /// <returns>The handle.</returns>
        /// <exception cref="KhataTaarib">See <see cref="Ihjiz"/>.</exception>
        public static Marja Thabit(IntPtr kaen)
        {
            return Ihjiz(kaen, NawMarja.Thabit);
        }

        /// <summary>
        /// Observes an object without keeping it alive. For caches keyed by game
        /// objects.
        /// </summary>
        /// <param name="kaen">The object pointer.</param>
        /// <returns>The handle.</returns>
        /// <exception cref="KhataTaarib">See <see cref="Ihjiz"/>.</exception>
        public static Marja Daeef(IntPtr kaen)
        {
            return Ihjiz(kaen, NawMarja.Daeef);
        }

        /// <summary>
        /// Releases the root. Idempotent, and safe to call from a
        /// <c>using</c> whose body threw.
        /// </summary>
        /// <remarks>
        /// Must run on a thread the IL2CPP runtime has attached, for the same
        /// reason the finalizer does not do this.
        /// </remarks>
        public void Dispose()
        {
            if (Interlocked.Exchange(ref mutakhalla, 1) != 0)
            {
                return;
            }

            GC.SuppressFinalize(this);
            try
            {
                IL2CPP.il2cpp_gchandle_free(raqm);
            }
            catch (Exception)
            {
                // Disposal runs on shutdown paths and inside finally blocks
                // whose exception is the one worth reporting. A runtime that has
                // already torn down its collector will refuse the free, and
                // letting that replace the original failure would lose the
                // diagnosis for a handle the process is about to stop caring
                // about anyway.
            }
        }

        /// <summary>
        /// Records the leak and frees nothing.
        /// </summary>
        /// <remarks>
        /// DELIBERATELY DOES NOT FREE. il2cpp_gchandle_free is a call into the
        /// IL2CPP collector, and the collector expects to be called from a
        /// thread it has attached. The .NET finalizer thread belongs to the
        /// second CLR BepInEx hosts and is not attached to anything; calling
        /// into the collector from it races the collector's own bookkeeping in a
        /// way that produces a crash in the player's game with a stack in the
        /// finalizer thread and no trace of what allocated the handle. A leaked
        /// handle costs one rooted object for the life of the process, which is
        /// a bug worth reporting and not worth crashing over — so the finalizer
        /// counts it into <see cref="Musarraba"/>, the diagnostics bundle
        /// reports the count, and the leak is fixed in source where it belongs.
        /// </remarks>
        ~Marja()
        {
            Interlocked.Increment(ref musarraba);
        }
    }

    /// <summary>
    /// Copies text out of the IL2CPP heap into Taarib-owned buffers, before
    /// anything can allocate.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Every method here reads the IL2CPP string and finishes the copy without
    /// a single allocation in between, which is what makes them safe to call
    /// with a raw <c>Il2CppString*</c> that arrived as an argument at a takeover
    /// point. Encoding.UTF8's span overloads are used precisely because they
    /// write into the caller's buffer rather than returning a new array: an
    /// allocating conversion would give the collector an opportunity between
    /// reading the character pointer and using it, which is the whole hazard.
    /// </para>
    /// <para>
    /// The short-buffer answer is the same negotiation the native ABI uses
    /// everywhere else in Taarib: the required capacity is written out and
    /// nothing else is, so a caller grows once and retries rather than guessing
    /// or allocating per call. See <c>Taarib.Unity.Jisr.Takhtit</c> for the
    /// same shape on the layout path.
    /// </para>
    /// </remarks>
    public static class Nasakh
    {
        /// <summary>
        /// The longest IL2CPP string this will copy, in UTF-16 code units.
        /// </summary>
        /// <remarks>
        /// A bound on a length field read out of another runtime's heap, not on
        /// anything a game legitimately renders. A corrupted or hostile
        /// Il2CppString reporting a length of two billion would otherwise have
        /// this code construct a span over four gigabytes of whatever follows it
        /// in memory. Sixteen million characters is far past any string a text
        /// component holds and far short of anything that can be read past a
        /// mapping.
        /// </remarks>
        public const int AqsaTul = 16 * 1024 * 1024;

        /// <summary>
        /// How many UTF-16 code units an IL2CPP string holds, or -1 when it is
        /// null or reports an unusable length.
        /// </summary>
        /// <param name="silsila">The <c>Il2CppString*</c>.</param>
        /// <returns>The length, or -1.</returns>
        public static int Tul(IntPtr silsila)
        {
            if (silsila == IntPtr.Zero)
            {
                return -1;
            }
            int tul = IL2CPP.il2cpp_string_length(silsila);
            return tul < 0 || tul > AqsaTul ? -1 : tul;
        }

        /// <summary>
        /// How many UTF-8 bytes
        /// <see cref="IlaUtf8(IntPtr, Span{byte}, out int, out int)"/> will
        /// write for a string.
        /// </summary>
        /// <param name="silsila">The <c>Il2CppString*</c>.</param>
        /// <returns>The byte count, or -1 when the string cannot be read.</returns>
        /// <remarks>
        /// Counts rather than estimates. A three-byte-per-character worst case
        /// would over-allocate by a factor of three for Arabic and by six for
        /// Latin, and the count is one pass over memory that is already resident
        /// and already in cache from the call that produced the string.
        /// </remarks>
        public static unsafe int SiaMatluba(IntPtr silsila)
        {
            int tul = Tul(silsila);
            if (tul < 0)
            {
                return -1;
            }
            if (tul == 0)
            {
                return 0;
            }

            char* huruf = IL2CPP.il2cpp_string_chars(silsila);
            if (huruf == null)
            {
                return -1;
            }

            // SOUND: the character array is inside the Il2CppString object whose
            // pointer the caller holds, its length came from that same object's
            // length field and is bounded by AqsaTul above, and nothing between
            // il2cpp_string_chars and the return allocates — GetByteCount over a
            // span does not. So the object cannot have been collected or
            // relocated while the span exists.
            ReadOnlySpan<char> masdar = new ReadOnlySpan<char>(huruf, tul);
            return Encoding.UTF8.GetByteCount(masdar);
        }

        /// <summary>
        /// Copies an IL2CPP string into a caller-owned UTF-8 buffer, allocating
        /// nothing.
        /// </summary>
        /// <param name="silsila">The <c>Il2CppString*</c>.</param>
        /// <param name="hadaf">The destination.</param>
        /// <param name="maktub">How many bytes were written.</param>
        /// <param name="matlub">
        /// How many bytes are required, written whether or not the copy
        /// succeeded, so a short buffer is grown once rather than probed.
        /// </param>
        /// <returns>Whether the copy completed.</returns>
        /// <remarks>
        /// UTF-8 because that is what the native layout engine takes: the ABI in
        /// <c>crates/taarib-jisr</c> accepts UTF-8 byte spans and rejects
        /// anything else with <see cref="Ramz.TarmizBatil"/>. Converting here,
        /// at the boundary, rather than inside the layout call is what keeps the
        /// per-frame path free of both an allocation and a second copy.
        /// </remarks>
        public static unsafe bool IlaUtf8(
            IntPtr silsila, Span<byte> hadaf, out int maktub, out int matlub)
        {
            maktub = 0;
            matlub = 0;

            int tul = Tul(silsila);
            if (tul < 0)
            {
                return false;
            }
            if (tul == 0)
            {
                return true;
            }

            char* huruf = IL2CPP.il2cpp_string_chars(silsila);
            if (huruf == null)
            {
                return false;
            }

            // SOUND: as SiaMatluba. The span covers exactly the characters the
            // string reports, GetByteCount and GetBytes both write into the
            // caller's buffer and allocate nothing, and no other call is made
            // before the copy finishes — so there is no point at which the
            // collector could run between reading the pointer and finishing with
            // it.
            ReadOnlySpan<char> masdar = new ReadOnlySpan<char>(huruf, tul);
            matlub = Encoding.UTF8.GetByteCount(masdar);
            if (matlub > hadaf.Length)
            {
                return false;
            }
            maktub = Encoding.UTF8.GetBytes(masdar, hadaf);

            return true;
        }

        /// <summary>
        /// Copies an IL2CPP string into a caller-owned UTF-16 buffer, allocating
        /// nothing.
        /// </summary>
        /// <param name="silsila">The <c>Il2CppString*</c>.</param>
        /// <param name="hadaf">The destination.</param>
        /// <param name="maktub">How many code units were written.</param>
        /// <param name="matlub">How many are required.</param>
        /// <returns>Whether the copy completed.</returns>
        /// <remarks>
        /// For the takeover points that hand text back to the engine rather than
        /// to the layout engine — a measurement path, a comparison against a
        /// patch key — where converting to UTF-8 and back would cost two passes
        /// and lose nothing but time.
        /// </remarks>
        public static unsafe bool IlaUtf16(
            IntPtr silsila, Span<char> hadaf, out int maktub, out int matlub)
        {
            maktub = 0;
            matlub = 0;

            int tul = Tul(silsila);
            if (tul < 0)
            {
                return false;
            }
            matlub = tul;
            if (tul == 0)
            {
                return true;
            }
            if (tul > hadaf.Length)
            {
                return false;
            }

            char* huruf = IL2CPP.il2cpp_string_chars(silsila);
            if (huruf == null)
            {
                return false;
            }

            // SOUND: as SiaMatluba. CopyTo on a span of a blittable type is a
            // memmove and allocates nothing.
            ReadOnlySpan<char> masdar = new ReadOnlySpan<char>(huruf, tul);
            masdar.CopyTo(hadaf);

            maktub = tul;
            return true;
        }

        /// <summary>
        /// Copies an IL2CPP string through a live root, for text held across a
        /// call that can allocate.
        /// </summary>
        /// <param name="marja">A marja rooting the string.</param>
        /// <param name="hadaf">The destination.</param>
        /// <param name="maktub">How many bytes were written.</param>
        /// <param name="matlub">How many are required.</param>
        /// <returns>Whether the copy completed.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="marja"/> is null.</exception>
        /// <remarks>
        /// The raw-pointer overloads are correct only when the pointer arrived
        /// in the current call and nothing has allocated since. When that is not
        /// obviously true — a string kept from an earlier frame, a string
        /// obtained before a runtime call — the string must be rooted first and
        /// copied through this overload, which re-reads the address from the
        /// handle at the moment of the copy rather than trusting one taken
        /// earlier.
        /// </remarks>
        public static bool IlaUtf8(
            Marja marja, Span<byte> hadaf, out int maktub, out int matlub)
        {
            if (marja is null)
            {
                throw new ArgumentNullException(nameof(marja));
            }
            return IlaUtf8(marja.Kaen, hadaf, out maktub, out matlub);
        }

        /// <summary>
        /// Materialises an IL2CPP string as a managed string. Allocates, and is
        /// for logs and diagnostics only.
        /// </summary>
        /// <param name="silsila">The <c>Il2CppString*</c>.</param>
        /// <returns>The text, or null when the string could not be read.</returns>
        /// <remarks>
        /// Deliberately not on the layout path and deliberately named so that a
        /// reviewer notices it there. Every per-frame consumer takes a span; the
        /// only callers of this are the ones writing a sentence a person will
        /// read, where one allocation on an error path costs nothing and a
        /// String.Empty in a bug report costs an afternoon.
        /// </remarks>
        public static unsafe string? Nass(IntPtr silsila)
        {
            int tul = Tul(silsila);
            if (tul < 0)
            {
                return null;
            }
            if (tul == 0)
            {
                return string.Empty;
            }

            char* huruf = IL2CPP.il2cpp_string_chars(silsila);
            if (huruf == null)
            {
                return null;
            }

            // SOUND: as SiaMatluba, with one difference worth naming. The string
            // constructor allocates on the .NET heap, which the IL2CPP collector
            // does not observe and cannot be triggered by, so it is not an
            // allocation that can move the source. It is still the last thing
            // done with the pointer.
            return new string(huruf, 0, tul);
        }
    }
}
