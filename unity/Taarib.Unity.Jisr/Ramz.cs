// رموز الحالة — the status code space, mirrored value for value against
// crates/taarib-jisr/src/khata_c.rs.
//
// Every entry point returns an int32. Zero is success and every failure is
// negative, so a caller in any language can write `if (r < 0)` and be right.
// An integer is not enough to act on, though — the whole error model exists
// so a failure arrives carrying a permanent code, an Arabic sentence, an
// English sentence and one concrete next action. Tahaqqaq is where that
// happens on this side of the boundary: a negative status becomes a
// KhataTaarib built from the thread-local stash, never a bare Exception with
// a formatted string.

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// The status codes every <c>taarib_*</c> entry point returns, mirrored
    /// value for value against <c>crates/taarib-jisr/src/khata_c.rs</c>, and
    /// the one helper that turns a failing status into the thrown
    /// <see cref="KhataTaarib"/>.
    /// </summary>
    public static class Ramz
    {
        /// <summary>The operation succeeded. Mirrors <c>TAARIB_NAJAH</c>.</summary>
        public const int Najah = 0;

        /// <summary>
        /// Something failed that no more specific code describes. The stashed
        /// error still names it exactly; this is the code, not the diagnosis.
        /// Mirrors <c>TAARIB_KHATA_AAM</c>.
        /// </summary>
        public const int KhataAam = -1;

        /// <summary>
        /// A required pointer argument was null. Mirrors
        /// <c>TAARIB_MUASHIR_BATIL</c>.
        /// </summary>
        public const int MuashirBatil = -2;

        /// <summary>
        /// A handle was not one the library issued, or was destroyed already.
        /// This is the use-after-free that generation tagging exists to
        /// catch: reported rather than executed, which is the difference
        /// between a bug report naming a stale handle and a crash dump in
        /// someone's game. Mirrors <c>TAARIB_MAQBAD_BATIL</c>.
        /// </summary>
        public const int MaqbadBatil = -3;

        /// <summary>
        /// The caller's buffer is too small; the required capacity has been
        /// written to the out-parameter and nothing else was written. Not an
        /// error in the usual sense — it is the negotiation the
        /// allocation-free hot path is built on, and <see cref="Takhtit"/>
        /// absorbs it before it can reach <see cref="Tahaqqaq"/>. Mirrors
        /// <c>TAARIB_SIAT_QASIRA</c>.
        /// </summary>
        public const int SiatQasira = -4;

        /// <summary>
        /// The caller was built against a different major version of this
        /// ABI. Mirrors <c>TAARIB_ISDAR_GHAYR_MUTAWAFIQ</c>.
        /// </summary>
        public const int IsdarGhayrMutawafiq = -5;

        /// <summary>
        /// A string argument was not valid UTF-8. Mirrors
        /// <c>TAARIB_TARMIZ_BATIL</c>.
        /// </summary>
        public const int TarmizBatil = -6;

        /// <summary>
        /// A font was rejected, or could not be read. The stashed error names
        /// the missing table or feature, because "this font has no GSUB table
        /// and cannot join Arabic letters" is a sentence a user can act on
        /// and "font error" is not. Mirrors <c>TAARIB_KHATT_MARFUD</c>.
        /// </summary>
        public const int KhattMarfud = -7;

        /// <summary>
        /// Shaping produced nothing for text that is not empty. Mirrors
        /// <c>TAARIB_TASHKEEL_FASHIL</c>.
        /// </summary>
        public const int TashkeelFashil = -8;

        /// <summary>
        /// An allocation failed. Reported rather than aborted: the library
        /// runs inside somebody else's process and does not get to decide
        /// that the process ends. Mirrors <c>TAARIB_DHAKIRA</c>.
        /// </summary>
        public const int Dhakira = -9;

        /// <summary>
        /// An argument was structurally fine but its value is not usable — a
        /// negative width, a size of zero, a style span that points outside
        /// its text. Mirrors <c>TAARIB_QEEMA_BATILA</c>.
        /// </summary>
        public const int QeemaBatila = -10;

        /// <summary>
        /// A panic was caught at the boundary and converted into a status.
        /// This code means the native library has a bug worth reporting — and
        /// the game is still running to report it. Mirrors
        /// <c>TAARIB_INHIYAR</c>.
        /// </summary>
        public const int Inhiyar = -11;

        /// <summary>
        /// The atlas is full and nothing in it may be evicted, because
        /// everything in it is referenced by the frame currently being drawn.
        /// Mirrors <c>TAARIB_LAWHA_MUMTALIA</c>.
        /// </summary>
        public const int LawhaMumtalia = -12;

        /// <summary>
        /// The operation is not available in this build — a feature compiled
        /// out, or a platform that does not offer what was asked for.
        /// Mirrors <c>TAARIB_GHAYR_MADUM</c>.
        /// </summary>
        public const int GhayrMadum = -13;

        /// <summary>
        /// The library has not been initialised, or was shut down. Mirrors
        /// <c>TAARIB_GHAYR_MUHAYYAA</c>.
        /// </summary>
        public const int GhayrMuhayyaa = -14;

        /// <summary>
        /// Turns a failing status into the thrown exception, and is the only
        /// way a status becomes one — no call site formats its own message,
        /// so a failure in the game reads the same as the same failure in
        /// Studio, permanent code and all.
        /// </summary>
        /// <param name="halat">
        /// The status an entry point returned. Zero and above pass through
        /// untouched.
        /// </param>
        /// <remarks>
        /// The hot path never routes <see cref="SiatQasira"/> here:
        /// <see cref="Takhtit"/> answers it by growing its buffer and
        /// retrying, which is the negotiation that code exists for. A
        /// <see cref="SiatQasira"/> that does reach this method — from a call
        /// that performed no negotiation — still throws, carrying whatever
        /// the thread-local stash holds, because swallowing it would hide a
        /// call-site bug.
        /// </remarks>
        /// <exception cref="KhataTaarib">
        /// <paramref name="halat"/> is negative. The exception carries the
        /// permanent code, the Arabic sentence, the English sentence and the
        /// next-action discriminant retrieved from the native thread-local
        /// stash for the current thread — which is why it must be built on
        /// the thread the failing call ran on, before any other native call
        /// from that thread.
        /// </exception>
        public static void Tahaqqaq(int halat)
        {
            if (halat >= Najah)
            {
                return;
            }
            throw KhataTaarib.MinAkhirKhata(halat);
        }
    }
}
