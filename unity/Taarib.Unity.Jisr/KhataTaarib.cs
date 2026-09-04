// خطأ تعريب — the failure that crossed the boundary, reassembled.
//
// The C ABI can only return an integer, so the native side stashes the full
// error — permanent code, Arabic sentence, English sentence, next action —
// in thread-local storage on its way out (crates/taarib-jisr/src/khata_c.rs).
// This type is the managed reassembly of that stash: one exception carrying
// all four things, built on the thread the failing call ran on, before any
// other native call from that thread could overwrite the stash.
//
// Message is the Arabic sentence, because Arabic is the primary text of this
// product; the English sentence rides along for a log read abroad, and
// ToString renders both so a BepInEx log line is complete in either language.

using System;

namespace Taarib.Unity.Jisr
{
    /// <summary>
    /// The language a retrieved error sentence is asked for in. Mirrors the
    /// discriminant of <c>lugha_min_raqm</c> in
    /// <c>crates/taarib-jisr/src/khata_c.rs</c>: anything unrecognised
    /// resolves to Arabic on the native side, because Arabic is the primary
    /// text of this product and a caller that passed a wrong number gets the
    /// real sentence rather than nothing.
    /// </summary>
    public enum Lugha : uint
    {
        /// <summary>العربية.</summary>
        Arabi = 0,

        /// <summary>English.</summary>
        Injilizi = 1,
    }

    /// <summary>
    /// The one concrete thing the user can do next, as the stable
    /// discriminant the ABI exposes. Mirrors <c>raqm_khutwa</c> in
    /// <c>crates/taarib-jisr/src/khata_c.rs</c> over the <c>Khutwa</c> enum
    /// of <c>crates/taarib-usus/src/khata.rs</c>.
    /// </summary>
    /// <remarks>
    /// The numbering is stable and additive: a new action takes the next
    /// free number, and a caller that receives a number it does not
    /// recognise shows the sentence without a button rather than showing
    /// nothing — which is why code switching on this enum must carry a
    /// default arm, never an exhaustiveness assumption.
    /// </remarks>
    public enum Khutwa : int
    {
        /// <summary>Nothing to do; the message is complete on its own.</summary>
        LaShay = 0,

        /// <summary>Try the same operation again — transient contention.</summary>
        AadaMuhawala = 1,

        /// <summary>Rescan the library.</summary>
        AadaFahsMaktaba = 2,

        /// <summary>Re-probe this game's engine.</summary>
        AadaFahsMuharrik = 3,

        /// <summary>Point Taarib at a path it could not find.</summary>
        IkhtiyarMasar = 4,

        /// <summary>Choose a different font — the current one cannot render Arabic.</summary>
        IkhtiyarKhattAakhar = 5,

        /// <summary>Open a section of Settings.</summary>
        FathIdadat = 6,

        /// <summary>Open Diagnostics.</summary>
        FathTashkhis = 7,

        /// <summary>Open the overflow report for the current patch or submission.</summary>
        FathTaqreerTajawuz = 8,

        /// <summary>Open the offending strings in the workspace.</summary>
        FathNusus = 9,

        /// <summary>Update Taarib itself — the data is newer than this build understands.</summary>
        TahdithTaarib = 10,

        /// <summary>Reinstall the framework for this game.</summary>
        IadatTarkibIttar = 11,

        /// <summary>Uninstall the patch and restore the game.</summary>
        IlghaTathbeet = 12,

        /// <summary>Re-match the patch against the game's new build.</summary>
        IadatMutabaqaBina = 13,

        /// <summary>Verify the game's files through its own launcher.</summary>
        TahaqquqSalamatLuba = 14,

        /// <summary>Contact the patch's contributor.</summary>
        IblaghLilMusahim = 15,

        /// <summary>Send a diagnostics bundle to the project owner.</summary>
        IblaghLilMalik = 16,

        /// <summary>Free disk space and retry.</summary>
        TahrirMasaha = 17,

        /// <summary>Grant Taarib the permission the operating system refused.</summary>
        ManhSalahiya = 18,
    }

    /// <summary>
    /// A failure reported by the native library, carrying everything the
    /// error model promises: the status an entry point returned, the
    /// permanent machine code (<c>TAARIB-E-1042</c>), the Arabic sentence, the
    /// English sentence, and the one next action. <see cref="Exception.Message"/>
    /// is the Arabic sentence, because that is the primary text of this
    /// product.
    /// </summary>
    /// <remarks>
    /// Nothing in this assembly throws any other exception for a native
    /// failure, and nothing formats its own message from a status code —
    /// which is what keeps a failure inside a game reading exactly like the
    /// same failure inside Studio, permanent code and all, so a user's bug
    /// report matches what the owner's console knows.
    /// </remarks>
    public sealed class KhataTaarib : Exception
    {
        /// <summary>
        /// Builds the exception from its parts, for the two places a failure
        /// is known without a native stash: the loader's refusals, and a
        /// caller-side guard that failed before any native call ran.
        /// </summary>
        /// <param name="halat">The status code, one of the <see cref="Ramz"/> constants.</param>
        /// <param name="ramzDaim">
        /// The permanent machine code (<c>TAARIB-E-1042</c>), or an empty
        /// string when the failure carried none.
        /// </param>
        /// <param name="arabi">The Arabic sentence a user reads.</param>
        /// <param name="injilizi">The same sentence in English.</param>
        /// <param name="khutwa">The one thing the user can do next.</param>
        public KhataTaarib(int halat, string ramzDaim, string arabi, string injilizi, Khutwa khutwa)
            : base(arabi)
        {
            Halat = halat;
            RamzDaim = ramzDaim ?? string.Empty;
            Arabi = arabi ?? string.Empty;
            Injilizi = injilizi ?? string.Empty;
            Khutwa = khutwa;
        }

        /// <summary>
        /// The status code the failing entry point returned — one of the
        /// <see cref="Ramz"/> constants, always negative.
        /// </summary>
        public int Halat { get; }

        /// <summary>
        /// The permanent machine code, rendered <c>TAARIB-E-1042</c>. Empty
        /// when the failure carried no structured error behind it — a null
        /// pointer, a stale handle, a caught panic — in which case
        /// <see cref="Halat"/> is the whole diagnosis. Permanent means
        /// searchable: users paste these into bug reports and find them
        /// years later.
        /// </summary>
        public string RamzDaim { get; }

        /// <summary>The sentence shown to an Arabic-speaking user. Same value as <see cref="Exception.Message"/>.</summary>
        public string Arabi { get; }

        /// <summary>The same sentence in English.</summary>
        public string Injilizi { get; }

        /// <summary>
        /// The one concrete next action, as a value an interface turns into
        /// a button — so a failure can never arrive with nothing actionable
        /// attached. May carry a number outside the named members when the
        /// native side is newer than this assembly; show the sentence
        /// without a button in that case.
        /// </summary>
        public Khutwa Khutwa { get; }

        /// <summary>
        /// Renders the failure for a log that will be read in either
        /// language: code, status, both sentences, the next action, then the
        /// ordinary stack trace. A BepInEx log line built from this needs no
        /// second lookup to be understood.
        /// </summary>
        /// <returns>The bilingual rendering.</returns>
        public override string ToString()
        {
            string ras = RamzDaim.Length == 0
                ? $"KhataTaarib ({Halat})"
                : $"KhataTaarib {RamzDaim} ({Halat})";
            string jasad = $"{ras}: {Arabi} | {Injilizi} | khutwa: {Khutwa}";
            string? athar = StackTrace;
            return athar is null ? jasad : jasad + Environment.NewLine + athar;
        }

        /// <summary>
        /// The status code of the current thread's most recent native
        /// failure, from <c>taarib_khata_akhir</c>. Thread-local on the
        /// native side, so the render thread and a background loader can
        /// both be in the library at once without one thread's failure being
        /// reported as the other's.
        /// </summary>
        /// <returns><see cref="Ramz.Najah"/> when the last call on this thread succeeded.</returns>
        public static int AkhirHalat()
        {
            return Jisr.taarib_khata_akhir();
        }

        /// <summary>
        /// Reassembles the current thread's stashed failure into the
        /// exception <see cref="Ramz.Tahaqqaq"/> throws. Must run on the
        /// thread the failing call ran on, before any other native call from
        /// that thread — the stash is one slot deep.
        /// </summary>
        /// <param name="halat">The status the failing entry point returned.</param>
        /// <returns>The exception, always constructed, never thrown here.</returns>
        /// <remarks>
        /// Retrieval allocates: it decodes two sentences and a code. That is
        /// acceptable precisely because this is the error path — the
        /// allocation-free guarantee belongs to the per-frame layout path,
        /// and a layout call that failed is not going to be retried every
        /// frame. Retrieval itself never throws; when the stash holds no
        /// structured error (a null pointer, a stale handle, a caught
        /// panic), the built-in sentences below stand in, so the exception
        /// always carries something a person can read.
        /// </remarks>
        internal static KhataTaarib MinAkhirKhata(int halat)
        {
            string ramzDaim = Jisr.IqraRamzDaim();
            string arabi = Jisr.IqraNassKhata(Lugha.Arabi);
            string injilizi = Jisr.IqraNassKhata(Lugha.Injilizi);
            Khutwa khutwa = (Khutwa)Jisr.taarib_khata_khutwa();

            if (arabi.Length == 0 && injilizi.Length == 0)
            {
                (arabi, injilizi) = JumalIhtiyatiya(halat);
            }

            return new KhataTaarib(halat, ramzDaim, arabi, injilizi, khutwa);
        }

        /// <summary>
        /// The stand-in sentences for a failure that stashed no structured
        /// error. Written per status code rather than as one generic
        /// apology, because "a required pointer argument was null" points at
        /// a call site and "an error occurred" points at nothing.
        /// </summary>
        private static (string Arabi, string Injilizi) JumalIhtiyatiya(int halat)
        {
            switch (halat)
            {
                case Ramz.KhataAam:
                    return ("فشلت العملية داخل مكتبة تعريب دون تفاصيل إضافية.",
                            "The operation failed inside the Taarib library with no further detail.");
                case Ramz.MuashirBatil:
                    return ("مُرِّر مؤشر فارغ إلى مكتبة تعريب؛ هذا خلل في الملحق وليس في اللعبة.",
                            "A null pointer was passed to the Taarib library; this is a bug in the plugin, not in the game.");
                case Ramz.MaqbadBatil:
                    return ("استُخدم مقبض بعد تحريره أو من غير هذه المكتبة؛ اكتُشف الخطأ قبل أن يقع.",
                            "A handle was used after it was destroyed, or was never issued by this library; the mistake was detected before it could execute.");
                case Ramz.SiatQasira:
                    return ("المخزن المؤقت أصغر من المطلوب، وقد كُتبت السعة اللازمة في المعامل الخارج.",
                            "The buffer is smaller than required; the needed capacity was written to the out-parameter.");
                case Ramz.IsdarGhayrMutawafiq:
                    return ("إصدار واجهة تعريب لا يطابق ما بُني عليه هذا الملحق.",
                            "The Taarib ABI version does not match what this plugin was built against.");
                case Ramz.TarmizBatil:
                    return ("نص مُرِّر إلى المكتبة ليس UTF-8 صالحًا.",
                            "A string passed to the library was not valid UTF-8.");
                case Ramz.KhattMarfud:
                    return ("رُفض الخط أو تعذّرت قراءته.",
                            "The font was rejected or could not be read.");
                case Ramz.TashkeelFashil:
                    return ("لم يُنتج التشكيل شيئًا لنص غير فارغ.",
                            "Shaping produced nothing for text that is not empty.");
                case Ramz.Dhakira:
                    return ("فشل حجز الذاكرة داخل مكتبة تعريب.",
                            "A memory allocation failed inside the Taarib library.");
                case Ramz.QeemaBatila:
                    return ("قيمة معامل غير صالحة: عرض سالب، أو حجم صفري، أو نطاق خارج نصه.",
                            "An argument value is not usable: a negative width, a zero size, or a span outside its text.");
                case Ramz.Inhiyar:
                    return ("أُمسك انهيار داخل مكتبة تعريب عند الحدود وتحول إلى رمز حالة؛ هذا خلل يستحق البلاغ واللعبة ما تزال تعمل للإبلاغ عنه.",
                            "A panic inside the Taarib library was caught at the boundary and converted to a status; this is a bug worth reporting, and the game is still running to report it.");
                case Ramz.LawhaMumtalia:
                    return ("لوحة الحروف ممتلئة وكل ما فيها مستخدم في الإطار الجاري رسمه.",
                            "The glyph atlas is full and everything in it is referenced by the frame being drawn.");
                case Ramz.GhayrMadum:
                    return ("العملية غير متاحة في هذه النسخة أو على هذه المنصة.",
                            "The operation is not available in this build or on this platform.");
                case Ramz.GhayrMuhayyaa:
                    return ("مكتبة تعريب لم تُهيأ بعد أو أُغلقت.",
                            "The Taarib library has not been initialised, or was shut down.");
                default:
                    return ($"أعادت مكتبة تعريب رمز حالة غير معروف ({halat}).",
                            $"The Taarib library returned an unrecognised status code ({halat}).");
            }
        }
    }
}
