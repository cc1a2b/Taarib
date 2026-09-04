// نمط — one byte pattern: what it is written as, how it is searched for, and
// what the address it produces has to survive before anybody detours it.
//
// This is the bottom of rung three. When global-metadata.dat is encrypted,
// stripped or of a version Il2CppInterop has never seen, no name lookup can
// work, and the only remaining handle on TMP_Text.GenerateTextMesh is what the
// compiler emitted for it: a specific sequence of bytes at the top of the
// function, with the immediates and displacements that vary between builds
// blanked out. That sequence is written in the ordinary IDA form, as text, in a
// data file:
//
//   48 89 5C 24 ?? 48 89 74 24 ?? 57 48 83 EC 20 48 8B ?? ??
//
// and this file turns that text into a matcher.
//
// WHY WILDCARDS ARE WHOLE BYTES AND NOT NIBBLES. Several signature formats
// allow a half-byte wildcard, written 4? or ?8. Taarib does not, for two
// reasons that both point the same way. The first is cost: a whole-byte mask
// lets a comparison be one AND and one compare over a byte the CPU already has
// in a register, and it lets the skip table below be built over runs of bytes
// that are known exactly. A nibble mask makes every byte of every window a
// shift, two masks and two compares, and it shortens the wildcard-free runs the
// skip table is built from — which is the thing that decides whether a scan of a
// forty-megabyte .text section takes twenty milliseconds or two seconds. The
// second is that nobody has ever needed the precision. What varies between two
// builds of one function is a stack displacement, a relative call target, a
// register allocation or an immediate — whole bytes, every time. A prologue has
// never been expressed with a half-byte of certainty, and paying for the
// capability on every byte of every window to express something nobody writes is
// a bad trade made permanent.
//
// WHY A SECOND MATCH IS A REFUSAL AND NOT A TIE-BREAK.
//
// This is the single most important rule in this file and it is the one every
// other signature scanner gets wrong.
//
// A scanner that takes the first match is making a claim it has not checked:
// that the pattern it was given is unique in the range it searched. When that
// claim is false — and for a twelve-byte MSVC prologue in a native binary
// containing forty thousand functions it is false more often than anyone
// expects — the scanner returns the address of a function that merely looks
// like the target. Taarib then installs a detour on it. The detour is correct,
// the trampoline is correct, the calling convention is correct, and every one
// of them is applied to the wrong function.
//
// What the user experiences is not "Arabic did not render". It is the game
// crashing, or corrupting a mesh, or hanging, at a point with no visible
// relationship to text at all, because some unrelated routine — a physics
// helper, an audio mixer callback, an allocator — now begins with a jump into
// Taarib's replacement for a text layout function that expects entirely
// different arguments. The stack trace, if there is one, names a function in
// GameAssembly.dll that has no name. Nothing in the log says "signature". The
// bug is unfindable by construction, and it was created by an optimisation that
// saved one loop iteration.
//
// So: the scan always runs to the end of the range even after a hit, a second
// hit is a refusal that names both addresses, and the refusal reaches the log.
// A pattern that is not unique is a pattern that is wrong and its author needs
// to be told so, in the one place where finding out is still cheap. Rung three
// declining costs the game its Arabic. Rung three answering wrongly costs the
// user their save file.
//
// WHY THE SEARCH IS A SKIP TABLE AND NOT A NESTED LOOP. Because the scan must
// run to completion for the uniqueness rule above, its cost is never amortised
// by an early exit: every scan is a full pass. GameAssembly.dll's executable
// section in a shipped Unity title is routinely twenty to eighty megabytes, and
// a takeover resolves a dozen targets. A naive loop comparing the first pattern
// byte at every offset touches every one of those bytes once per target and
// then re-reads a handful more on each near miss; it is not catastrophic, but it
// is several hundred milliseconds of a player's loading screen spent doing
// something a 256-entry table makes almost free. Boyer-Moore-Horspool built over
// the pattern's longest wildcard-free run skips ahead by up to the length of
// that run on every mismatch, which for a typical prologue anchor is five to
// twelve bytes per step. The run is used rather than the whole pattern because
// a skip table cannot be built across a wildcard: an unknown byte gives no
// information about how far it is safe to jump.

using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.Text;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Basmat
{
    /// <summary>A processor architecture a pattern can be written for.</summary>
    /// <remarks>
    /// The names match the <c>Mimariya</c> vocabulary in
    /// <c>schemas/muharrik.json</c> value for value, so the signature database
    /// and an engine identification cannot disagree about what "x8664" means.
    /// </remarks>
    public enum Binya
    {
        /// <summary>32-bit x86. Still shipped by a surprising number of titles.</summary>
        X86 = 0,

        /// <summary>64-bit x86.</summary>
        X8664 = 1,

        /// <summary>64-bit ARM: Apple Silicon, Windows on ARM, Android, handhelds.</summary>
        Aarch64 = 2,
    }

    /// <summary>An operating system a pattern can be written for.</summary>
    /// <remarks>
    /// Part of the database key rather than a detail, because the same Unity
    /// version compiled by MSVC and by Clang produces different prologues for
    /// the same C++ function. A pattern taken from a Windows build and applied
    /// to a Linux one is a pattern that will either miss or — far worse — match
    /// something else.
    /// </remarks>
    public enum Minassa
    {
        /// <summary>Windows.</summary>
        Windows = 0,

        /// <summary>Linux, including Steam Deck's Proton-free native builds.</summary>
        Linux = 1,

        /// <summary>macOS.</summary>
        Mac = 2,

        /// <summary>Android.</summary>
        Android = 3,
    }

    /// <summary>What kind of check one validation predicate performs.</summary>
    /// <remarks>
    /// <para>
    /// The kinds are data, not code: an entry in the signature database names
    /// them by string, the loader maps the string to one of these, and the
    /// matcher runs whatever set it was handed. Adding a check that a future
    /// game needs means adding a member here and a case in
    /// <see cref="Namat.Fahs"/> — it does not mean rewriting the resolver, and
    /// it does not mean a boolean flag on every pattern that ever existed.
    /// </para>
    /// <para>
    /// The numbers are stable because they are written into diagnostics bundles
    /// as part of the record of which predicate rejected an address.
    /// </para>
    /// </remarks>
    public enum NawShart
    {
        /// <summary>
        /// The address lies inside the executable range the scan was given.
        /// Catches a negative offset that walked off the front of the section
        /// and an anchor near the end that walked off the back — both of which
        /// otherwise produce an address that is readable, plausible, and not
        /// code.
        /// </summary>
        QismTanfidhi = 0,

        /// <summary>
        /// The address satisfies the architecture's entry-point alignment.
        /// Mandatory on ARM64, where an unaligned branch target is a fault
        /// rather than a slow path, and a useful sanity check on x86-64 where
        /// compilers align function entries even though the hardware does not
        /// demand it.
        /// </summary>
        Muhadhah = 1,

        /// <summary>
        /// The bytes at the address look like the start of a function for this
        /// architecture, checked against the openers in
        /// <see cref="Muqaddimat"/>. This is what separates "the pattern matched
        /// in the middle of a function" from "the pattern matched a function".
        /// </summary>
        Muqaddima = 2,

        /// <summary>
        /// A named byte, or masked byte, is present at a fixed displacement
        /// from the resolved address. The escape hatch for a target whose only
        /// distinguishing feature is somewhere the anchor pattern does not
        /// reach — a specific immediate forty bytes in, for instance.
        /// <see cref="SharkTahaqquq.Qeema"/> carries the expected bytes in the
        /// same text form a pattern uses, so it may contain wildcards.
        /// </summary>
        BaytIndIzaha = 3,

        /// <summary>
        /// The inverse: the bytes at a displacement must NOT match. Used to
        /// exclude a known decoy — a second function in the same binary whose
        /// prologue is identical and which differs three instructions later.
        /// </summary>
        LaysBaytIndIzaha = 4,
    }

    /// <summary>
    /// One condition a resolved address has to satisfy before Taarib will treat
    /// it as the target.
    /// </summary>
    /// <remarks>
    /// A value rather than a delegate on purpose. Predicates arrive from a JSON
    /// file that a user could have edited; they have to be describable, they
    /// have to be printable into a refusal that names which one rejected the
    /// address, and they have to be impossible to express as arbitrary
    /// behaviour. A delegate would be none of those things.
    /// </remarks>
    public readonly struct SharkTahaqquq
    {
        /// <summary>Builds a predicate.</summary>
        /// <param name="naw">Which check.</param>
        /// <param name="izaha">
        /// The displacement from the resolved address, for the kinds that take
        /// one. Ignored otherwise. May be negative.
        /// </param>
        /// <param name="qeema">
        /// The expected bytes, in pattern text form, for the kinds that take
        /// them. Empty otherwise.
        /// </param>
        public SharkTahaqquq(NawShart naw, int izaha, string qeema)
        {
            Naw = naw;
            Izaha = izaha;
            Qeema = qeema ?? string.Empty;
        }

        /// <summary>Which check this is.</summary>
        public NawShart Naw { get; }

        /// <summary>Its displacement from the resolved address, where it takes one.</summary>
        public int Izaha { get; }

        /// <summary>Its expected bytes in pattern text form, where it takes them.</summary>
        public string Qeema { get; }

        /// <summary>Renders the predicate for a refusal sentence and a bundle.</summary>
        /// <returns>A short, stable description.</returns>
        public override string ToString()
        {
            return Qeema.Length == 0
                ? $"{Naw}"
                : $"{Naw}(+{Izaha}, {Qeema})";
        }

        /// <summary>
        /// The three checks every pattern gets when its entry names none.
        /// </summary>
        /// <param name="binya">The architecture, which decides the alignment.</param>
        /// <returns>Executable range, alignment, plausible prologue.</returns>
        /// <remarks>
        /// A default rather than an option, because the failure these prevent —
        /// a detour installed on an address that is not a function entry — is
        /// not one a database author would think to guard against until it had
        /// already happened to somebody.
        /// </remarks>
        public static SharkTahaqquq[] Iftiradiya(Binya binya)
        {
            _ = binya;
            return new[]
            {
                new SharkTahaqquq(NawShart.QismTanfidhi, 0, string.Empty),
                new SharkTahaqquq(NawShart.Muhadhah, 0, string.Empty),
                new SharkTahaqquq(NawShart.Muqaddima, 0, string.Empty),
            };
        }
    }

    /// <summary>
    /// What a function's first bytes may look like, per architecture, as data.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The <see cref="NawShart.Muqaddima"/> predicate is only as good as this
    /// table, so the table is public, extensible and documented rather than a
    /// private array of magic numbers. A build that meets a compiler whose
    /// prologue is not listed here adds an entry; it does not disable the check,
    /// which is what a hard-coded boolean would force it to do.
    /// </para>
    /// <para>
    /// The entries are deliberately short and deliberately permissive. This is a
    /// sanity check on an address that has already matched a full pattern, not a
    /// second signature: its job is to reject an address that landed in the
    /// middle of an instruction stream, and a check tight enough to reject a
    /// real prologue it had not seen would cost far more than it saves.
    /// </para>
    /// </remarks>
    public static class Muqaddimat
    {
        private static readonly string[] X8664Openers =
        {
            "48 8B C4",             // mov rax, rsp      — MSVC large-frame anchor
            "4C 8B DC",             // mov r11, rsp      — the same, via r11
            "48 89 5C 24 ??",       // mov [rsp+X], rbx  — MSVC register save
            "48 89 74 24 ??",       // mov [rsp+X], rsi
            "48 89 7C 24 ??",       // mov [rsp+X], rdi
            "48 89 4C 24 ??",       // mov [rsp+X], rcx  — home the first argument
            "48 89 54 24 ??",       // mov [rsp+X], rdx
            "4C 89 44 24 ??",       // mov [rsp+X], r8
            "4C 89 4C 24 ??",       // mov [rsp+X], r9
            "48 83 EC ??",          // sub rsp, imm8     — frameless leaf
            "48 81 EC ?? ?? ?? ??", // sub rsp, imm32    — large frame
            "48 89 6C 24 ??",       // mov [rsp+X], rbp
            "55 48 8B EC",          // push rbp; mov rbp, rsp — frame pointer
            "55 48 89 E5",          // push rbp; mov rbp, rsp — AT&T-flavoured encoding
            "40 53",                // push rbx (REX)
            "40 55",                // push rbp (REX)
            "40 56",                // push rsi (REX)
            "40 57",                // push rdi (REX)
            "41 54",                // push r12
            "41 55",                // push r13
            "41 56",                // push r14
            "41 57",                // push r15
            "53",                   // push rbx
            "56",                   // push rsi
            "57",                   // push rdi
            "E9 ?? ?? ?? ??",       // jmp rel32 — an ICF thunk or incremental-link stub
        };

        private static readonly string[] X86Openers =
        {
            "55 8B EC",             // push ebp; mov ebp, esp
            "53 56 57",             // push ebx; push esi; push edi
            "56 8B ??",             // push esi; mov reg, ...
            "57 8B ??",             // push edi; mov reg, ...
            "83 EC ??",             // sub esp, imm8
            "81 EC ?? ?? ?? ??",    // sub esp, imm32
            "8B FF 55 8B EC",       // mov edi, edi — hot-patchable prologue
            "E9 ?? ?? ?? ??",       // jmp rel32
        };

        private static readonly string[] Aarch64Openers =
        {
            "?? ?? ?? D1",          // sub sp, sp, #imm
            "?? ?? BD A9",          // stp x?, x?, [sp, #-imm]!  — pre-indexed save
            "?? ?? BE A9",          // stp x?, x?, [sp, #-imm]!  — larger frame
            "?? ?? BF A9",          // stp x?, x?, [sp, #-imm]!  — larger frame still
            "?? ?? 00 F9",          // str x?, [x?, #imm]
            "?? ?? 00 14",          // b   — a tail-call thunk
            "5F 24 03 D5",          // bti c — pointer-authentication landing pad
            "7F 23 03 D5",          // pacibsp
        };

        /// <summary>
        /// The prologue openers accepted for an architecture, parsed once.
        /// </summary>
        /// <param name="binya">The architecture.</param>
        /// <returns>The openers, never empty.</returns>
        /// <exception cref="KhataTaarib">
        /// One of the openers above does not parse, which is this build's bug
        /// and is raised at the first use rather than swallowed — a prologue
        /// check that silently accepts everything is worse than no check.
        /// </exception>
        public static IReadOnlyList<Namat> Li(Binya binya)
        {
            switch (binya)
            {
                case Binya.X86:
                    return Bina(ref x86, X86Openers);
                case Binya.Aarch64:
                    return Bina(ref aarch64, Aarch64Openers);
                default:
                    return Bina(ref x8664, X8664Openers);
            }
        }

        private static Namat[]? x86;
        private static Namat[]? x8664;
        private static Namat[]? aarch64;

        private static Namat[] Bina(ref Namat[]? makhzun, string[] nusus)
        {
            Namat[]? hadir = makhzun;
            if (hadir is not null)
            {
                return hadir;
            }
            Namat[] jadid = new Namat[nusus.Length];
            for (int i = 0; i < nusus.Length; i++)
            {
                jadid[i] = Namat.Hallil(nusus[i]);
            }
            makhzun = jadid;
            return jadid;
        }
    }

    /// <summary>The stretch of process memory one scan is allowed to look at.</summary>
    /// <remarks>
    /// <para>
    /// Carries the executable range separately from the searched range because
    /// they are not always the same: a caller may narrow a scan to one section
    /// of GameAssembly.dll for speed while still wanting
    /// <see cref="NawShart.QismTanfidhi"/> to be checked against the whole
    /// executable image.
    /// </para>
    /// <para>
    /// The memory is the caller's to guarantee. This type checks the numbers it
    /// can — a non-zero base, a positive length, a length that fits an index —
    /// but it cannot check that a range is mapped, and a range that is not
    /// mapped faults in a way .NET cannot turn into an exception. Callers build
    /// this from a module's own headers, never from a guess.
    /// </para>
    /// </remarks>
    public readonly struct MadaMasah
    {
        /// <summary>Describes a range to scan.</summary>
        /// <param name="bidaya">The first byte to search.</param>
        /// <param name="tul">How many bytes to search.</param>
        /// <param name="bidayatTanfidh">The first byte of the executable image.</param>
        /// <param name="tulTanfidh">How long the executable image is.</param>
        /// <param name="ismWahda">The module's name, for a refusal sentence.</param>
        /// <param name="binya">The architecture the module was built for.</param>
        public MadaMasah(
            IntPtr bidaya,
            long tul,
            IntPtr bidayatTanfidh,
            long tulTanfidh,
            string ismWahda,
            Binya binya)
        {
            Bidaya = bidaya;
            Tul = tul;
            BidayatTanfidh = bidayatTanfidh;
            TulTanfidh = tulTanfidh;
            IsmWahda = ismWahda ?? string.Empty;
            Binya = binya;
        }

        /// <summary>The first byte searched.</summary>
        public IntPtr Bidaya { get; }

        /// <summary>How many bytes are searched.</summary>
        public long Tul { get; }

        /// <summary>The first byte of the executable image.</summary>
        public IntPtr BidayatTanfidh { get; }

        /// <summary>How long the executable image is.</summary>
        public long TulTanfidh { get; }

        /// <summary>The module's name, as a refusal should name it.</summary>
        public string IsmWahda { get; }

        /// <summary>The architecture the module was built for.</summary>
        public Binya Binya { get; }

        /// <summary>
        /// The alignment a function entry point must satisfy on this
        /// architecture.
        /// </summary>
        /// <remarks>
        /// Four on ARM64 because every instruction is four bytes and a branch to
        /// an unaligned address faults. One on x86 and x86-64 because the
        /// hardware permits any alignment — compilers align entries to 16 in
        /// practice, but a pattern that anchors a few bytes into a function and
        /// carries a negative offset legitimately resolves to an address that is
        /// not 16-aligned, and rejecting those would reject correct patterns.
        /// </remarks>
        public int Muhadhah => Binya == Binya.Aarch64 ? 4 : 1;

        /// <summary>Whether the numbers describe a range that can be searched.</summary>
        /// <param name="sabab">Why not, when they do not.</param>
        /// <returns>Whether the range is usable.</returns>
        public bool Salih(out string sabab)
        {
            if (Bidaya == IntPtr.Zero)
            {
                sabab = "the scan range has no base address";
                return false;
            }
            if (Tul <= 0)
            {
                sabab = $"the scan range in {Wasf()} is {Tul} bytes long";
                return false;
            }
            if (Tul > int.MaxValue)
            {
                // A span is indexed by int, and no executable section is two
                // gigabytes. Refusing loudly beats silently searching the first
                // 2 GB and reporting a pattern as absent when it was simply
                // past the truncation.
                sabab = $"the scan range in {Wasf()} is {Tul} bytes, past the "
                    + "2 GB a single scan can address";
                return false;
            }
            sabab = string.Empty;
            return true;
        }

        /// <summary>Whether an address is inside the executable image.</summary>
        /// <param name="unwan">The address.</param>
        /// <returns>Whether it is in range.</returns>
        public bool DakhilTanfidh(IntPtr unwan)
        {
            if (BidayatTanfidh == IntPtr.Zero || TulTanfidh <= 0)
            {
                // No executable range was supplied, so fall back to the searched
                // range: something has to bound the answer, and the searched
                // range is the tighter of the two whenever both are known.
                long asas = Bidaya.ToInt64();
                long qeema = unwan.ToInt64();
                return qeema >= asas && qeema < asas + Tul;
            }
            long bidayat = BidayatTanfidh.ToInt64();
            long hadaf = unwan.ToInt64();
            return hadaf >= bidayat && hadaf < bidayat + TulTanfidh;
        }

        /// <summary>Names the module and range, for a message.</summary>
        /// <returns>A short description.</returns>
        public string Wasf()
        {
            string ism = IsmWahda.Length == 0 ? "the module" : IsmWahda;
            return $"{ism} at 0x{Bidaya.ToInt64():X}";
        }
    }

    /// <summary>What one scan produced.</summary>
    /// <remarks>
    /// Four outcomes, and each of them is different: found exactly once and
    /// validated; found exactly once and rejected by a predicate; found more
    /// than once, which is a refusal; not found at all. Collapsing the middle
    /// two into "not found" is how a database author never learns that their
    /// pattern is anchored on the wrong instruction.
    /// </remarks>
    public readonly struct NatijatMasah
    {
        private NatijatMasah(IntPtr unwan, int adad, string sabab)
        {
            Unwan = unwan;
            Adad = adad;
            Sabab = sabab ?? string.Empty;
        }

        /// <summary>The resolved address, or zero.</summary>
        public IntPtr Unwan { get; }

        /// <summary>
        /// How many times the pattern matched. Capped at two, because the scan
        /// stops counting once uniqueness is disproved and the exact multiplicity
        /// of a broken pattern is not information anybody acts on.
        /// </summary>
        public int Adad { get; }

        /// <summary>Why it did not resolve, in one sentence. Empty on success.</summary>
        public string Sabab { get; }

        /// <summary>Whether the scan produced a usable address.</summary>
        public bool Wujid => Unwan != IntPtr.Zero && Adad == 1 && Sabab.Length == 0;

        /// <summary>Records a unique, validated match.</summary>
        /// <param name="unwan">The resolved address.</param>
        /// <returns>The result.</returns>
        public static NatijatMasah Fareed(IntPtr unwan)
        {
            return new NatijatMasah(unwan, 1, string.Empty);
        }

        /// <summary>Records that nothing matched.</summary>
        /// <param name="sabab">Where it was looked for.</param>
        /// <returns>The result.</returns>
        public static NatijatMasah Ghayr(string sabab)
        {
            return new NatijatMasah(IntPtr.Zero, 0, sabab);
        }

        /// <summary>Records that the pattern is not unique, which is a refusal.</summary>
        /// <param name="awwal">The first match.</param>
        /// <param name="thani">The second.</param>
        /// <param name="wasf">What was being searched.</param>
        /// <returns>The result.</returns>
        public static NatijatMasah Mutakarrir(IntPtr awwal, IntPtr thani, string wasf)
        {
            return new NatijatMasah(
                IntPtr.Zero,
                2,
                $"the pattern matches more than once in {wasf} "
                + $"(0x{awwal.ToInt64():X} and 0x{thani.ToInt64():X}); a pattern that does not "
                + "identify one function is refused rather than guessed at, because detouring "
                + "the wrong function crashes the game somewhere unrelated to text");
        }

        /// <summary>Records a unique match that a predicate rejected.</summary>
        /// <param name="unwan">The address that was rejected.</param>
        /// <param name="sabab">Which predicate, and what it saw.</param>
        /// <returns>The result.</returns>
        public static NatijatMasah Marfud(IntPtr unwan, string sabab)
        {
            return new NatijatMasah(
                IntPtr.Zero,
                1,
                $"the pattern matched once at 0x{unwan.ToInt64():X} but {sabab}");
        }
    }

    /// <summary>
    /// One byte pattern: the bytes, the wildcard mask, the offset applied to a
    /// match, and the predicates the resolved address must satisfy.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Immutable once parsed, and safe to share between the targets that use it.
    /// Parsing is the only place a pattern can be malformed, so it is the only
    /// place that refuses; everything after it operates on bytes that are known
    /// to be well-formed.
    /// </para>
    /// <para>
    /// Not thread-safe to scan concurrently with anything that unloads the
    /// module being scanned, which is a statement about the module rather than
    /// about this type: the scan itself holds no state and mutates nothing.
    /// </para>
    /// </remarks>
    public sealed class Namat
    {
        private readonly byte[] buyut;
        private readonly byte[] aqnia;
        private readonly int[] takhatti;
        private readonly int bidayatMirsat;
        private readonly int tulMirsat;

        private Namat(
            string nass,
            byte[] buyut,
            byte[] aqnia,
            int izaha,
            SharkTahaqquq[] shurut,
            int bidayatMirsat,
            int tulMirsat)
        {
            Nass = nass;
            this.buyut = buyut;
            this.aqnia = aqnia;
            Izaha = izaha;
            Shurut = shurut;
            this.bidayatMirsat = bidayatMirsat;
            this.tulMirsat = tulMirsat;
            takhatti = BinaTakhatti(buyut, bidayatMirsat, tulMirsat);
        }

        /// <summary>The pattern as it was written, normalised to upper case.</summary>
        /// <remarks>
        /// Kept so that a refusal, a log line and a diagnostics bundle can quote
        /// the pattern exactly as its author wrote it in the database, which is
        /// what makes the report actionable: the author can search their file
        /// for the string and find the entry.
        /// </remarks>
        public string Nass { get; }

        /// <summary>How many bytes the pattern covers, wildcards included.</summary>
        public int Tul => buyut.Length;

        /// <summary>
        /// The displacement added to a match to reach the target's entry point.
        /// </summary>
        /// <remarks>
        /// Frequently negative. A pattern often anchors on something a few bytes
        /// inside the function — a distinctive immediate, a call to a helper —
        /// because the first instructions of an MSVC function are shared by
        /// thousands of other functions and anchoring there would violate the
        /// uniqueness rule immediately. The offset walks back from the anchor to
        /// the entry point.
        /// </remarks>
        public int Izaha { get; }

        /// <summary>What the resolved address must satisfy.</summary>
        public IReadOnlyList<SharkTahaqquq> Shurut { get; }

        /// <summary>
        /// How many fixed, non-wildcard bytes the pattern names in its longest
        /// unbroken run — the run the skip table is built from.
        /// </summary>
        /// <remarks>
        /// Exposed because it is the one number that predicts a pattern's scan
        /// cost and its selectivity, and a database author reviewing a new entry
        /// should be able to see it without instrumenting a scan.
        /// </remarks>
        public int TulMirsat => tulMirsat;

        /// <summary>Parses a pattern with no offset and the default predicates.</summary>
        /// <param name="nass">The pattern text, in the IDA form.</param>
        /// <returns>The parsed pattern.</returns>
        /// <exception cref="KhataTaarib">The text is not a usable pattern.</exception>
        public static Namat Hallil(string nass)
        {
            return Hallil(nass, 0, null, Binya.X8664);
        }

        /// <summary>Parses a pattern.</summary>
        /// <param name="nass">
        /// The pattern text: hexadecimal byte pairs and wildcards, separated by
        /// whitespace. <c>??</c> and <c>?</c> are both whole-byte wildcards.
        /// </param>
        /// <param name="izaha">The displacement applied to a match. May be negative.</param>
        /// <param name="shurut">
        /// The predicates the resolved address must satisfy, or null for
        /// <see cref="SharkTahaqquq.Iftiradiya"/>.
        /// </param>
        /// <param name="binya">The architecture, which chooses the default predicates.</param>
        /// <returns>The parsed pattern.</returns>
        /// <exception cref="KhataTaarib">
        /// The text is empty, contains something that is neither a byte nor a
        /// wildcard, is shorter than two bytes, or names no fixed byte at all.
        /// </exception>
        public static Namat Hallil(
            string nass,
            int izaha,
            IReadOnlyList<SharkTahaqquq>? shurut,
            Binya binya)
        {
            if (!HawilHallil(nass, izaha, shurut, binya, out Namat? namat, out string sabab))
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6501",
                    $"نمط بصمة غير صالح في قاعدة البصمات: {sabab}",
                    $"A signature pattern in the database is not usable: {sabab}",
                    Khutwa.TahdithTaarib);
            }
            return namat;
        }

        /// <summary>Parses a pattern, reporting rather than throwing.</summary>
        /// <param name="nass">The pattern text.</param>
        /// <param name="izaha">The displacement applied to a match.</param>
        /// <param name="shurut">The predicates, or null for the defaults.</param>
        /// <param name="binya">The architecture.</param>
        /// <param name="namat">The parsed pattern, when it parsed.</param>
        /// <param name="sabab">Why not, when it did not.</param>
        /// <returns>Whether it parsed.</returns>
        /// <remarks>
        /// The form the database loader uses, because a database with one bad
        /// pattern out of two hundred should report that one pattern and load
        /// the rest — a whole file refused for a typo in an entry no game on
        /// this machine would ever have used is a worse outcome than the typo.
        /// </remarks>
        public static bool HawilHallil(
            string nass,
            int izaha,
            IReadOnlyList<SharkTahaqquq>? shurut,
            Binya binya,
            [NotNullWhen(true)] out Namat? namat,
            out string sabab)
        {
            namat = null;
            if (string.IsNullOrWhiteSpace(nass))
            {
                sabab = "the pattern text is empty";
                return false;
            }

            List<byte> buyut = new List<byte>(64);
            List<byte> aqnia = new List<byte>(64);
            int i = 0;
            int tul = nass.Length;
            while (i < tul)
            {
                char harf = nass[i];
                if (char.IsWhiteSpace(harf))
                {
                    i++;
                    continue;
                }

                if (harf == '?')
                {
                    // Both `??` and `?` mean one whole byte. A nibble wildcard
                    // is not accepted, and the header of this file says why; a
                    // form like `4?` therefore fails below rather than being
                    // silently read as `??`, which would widen the pattern
                    // without its author knowing.
                    i++;
                    if (i < tul && nass[i] == '?')
                    {
                        i++;
                    }
                    buyut.Add(0);
                    aqnia.Add(0);
                    continue;
                }

                int alfa = Raqm(harf);
                if (alfa < 0)
                {
                    sabab = $"'{harf}' at position {i} is neither a hexadecimal digit nor a "
                        + "wildcard";
                    return false;
                }
                if (i + 1 >= tul)
                {
                    sabab = $"the pattern ends with the single digit '{harf}'; every byte is "
                        + "written as two digits";
                    return false;
                }
                char thani = nass[i + 1];
                if (thani == '?')
                {
                    sabab = $"'{harf}?' at position {i} is a half-byte wildcard, which Taarib "
                        + "does not support: wildcards cover whole bytes";
                    return false;
                }
                int beta = Raqm(thani);
                if (beta < 0)
                {
                    sabab = $"'{harf}{thani}' at position {i} is not a hexadecimal byte";
                    return false;
                }
                if (i + 2 < tul && Raqm(nass[i + 2]) >= 0)
                {
                    sabab = $"'{harf}{thani}{nass[i + 2]}' at position {i} runs three digits "
                        + "together; bytes are separated by whitespace";
                    return false;
                }
                buyut.Add((byte)((alfa << 4) | beta));
                aqnia.Add(0xFF);
                i += 2;
            }

            if (buyut.Count < 2)
            {
                sabab = $"the pattern covers {buyut.Count} byte(s); a pattern shorter than two "
                    + "bytes cannot identify a function in a binary of this size";
                return false;
            }

            byte[] mabuyut = buyut.ToArray();
            byte[] maqnia = aqnia.ToArray();
            int bidayatMirsat = 0;
            int tulMirsat = 0;
            int hali = 0;
            for (int k = 0; k < maqnia.Length; k++)
            {
                if (maqnia[k] == 0)
                {
                    hali = 0;
                    continue;
                }
                hali++;
                if (hali > tulMirsat)
                {
                    tulMirsat = hali;
                    bidayatMirsat = k - hali + 1;
                }
            }

            if (tulMirsat == 0)
            {
                sabab = "the pattern is all wildcards, so it matches at every address in the "
                    + "module and identifies nothing";
                return false;
            }

            // Pre-mask the fixed bytes so the inner comparison is one AND and one
            // compare, with no branch on whether a position is a wildcard.
            for (int k = 0; k < mabuyut.Length; k++)
            {
                mabuyut[k] &= maqnia[k];
            }

            SharkTahaqquq[] mashrut = shurut is null || shurut.Count == 0
                ? SharkTahaqquq.Iftiradiya(binya)
                : Nasakh(shurut);

            namat = new Namat(
                Sawwi(mabuyut, maqnia),
                mabuyut,
                maqnia,
                izaha,
                mashrut,
                bidayatMirsat,
                tulMirsat);
            sabab = string.Empty;
            return true;
        }

        /// <summary>
        /// Searches a range and returns the one address the pattern identifies.
        /// </summary>
        /// <param name="mada">The range to search and the image to validate against.</param>
        /// <returns>
        /// The address, or a refusal naming what happened: nothing matched, more
        /// than one thing matched, or the one match failed a predicate.
        /// </returns>
        /// <remarks>
        /// <para>
        /// Runs to the end of the range even after a match, because the second
        /// match is the information that matters. Never throws: a scan is on the
        /// resolution ladder, and a rung that throws stops the rungs below it
        /// from being tried.
        /// </para>
        /// <para>
        /// Reads the range through a span over unmanaged memory. The range must
        /// be mapped and must stay mapped for the duration; see
        /// <see cref="MadaMasah"/>.
        /// </para>
        /// </remarks>
        public NatijatMasah Masah(in MadaMasah mada)
        {
            if (!mada.Salih(out string khalal))
            {
                return NatijatMasah.Ghayr(khalal);
            }
            int tulMada = (int)mada.Tul;
            if (tulMada < buyut.Length)
            {
                return NatijatMasah.Ghayr(
                    $"the scan range in {mada.Wasf()} is {tulMada} bytes, shorter than the "
                    + $"{buyut.Length}-byte pattern");
            }

            IntPtr awwal = IntPtr.Zero;
            IntPtr thani = IntPtr.Zero;
            int adad = 0;

            unsafe
            {
                ReadOnlySpan<byte> kawm = new ReadOnlySpan<byte>((void*)mada.Bidaya, tulMada);
                int akhir = tulMada - buyut.Length + bidayatMirsat;
                int mawqi = bidayatMirsat;
                int tulM = tulMirsat;
                while (mawqi <= akhir)
                {
                    // Horspool: compare the anchor run back to front, because a
                    // mismatch at the far end is both the most likely and the
                    // one that yields the longest shift.
                    int k = tulM - 1;
                    while (k >= 0 && kawm[mawqi + k] == buyut[bidayatMirsat + k])
                    {
                        k--;
                    }
                    if (k < 0)
                    {
                        int bidayatNamat = mawqi - bidayatMirsat;
                        if (Yutabiq(kawm, bidayatNamat))
                        {
                            adad++;
                            IntPtr unwan = new IntPtr(mada.Bidaya.ToInt64() + bidayatNamat);
                            if (adad == 1)
                            {
                                awwal = unwan;
                            }
                            else
                            {
                                thani = unwan;
                                break;
                            }
                        }
                        // Advance by one rather than by the skip: overlapping
                        // occurrences are exactly the case uniqueness has to
                        // catch, and a pattern whose own bytes repeat inside
                        // itself would hide its duplicate behind a longer shift.
                        mawqi++;
                        continue;
                    }
                    mawqi += takhatti[kawm[mawqi + tulM - 1]];
                }
            }

            if (adad == 0)
            {
                return NatijatMasah.Ghayr(
                    $"the pattern does not occur in {mada.Wasf()} over {tulMada} bytes");
            }
            if (adad > 1)
            {
                return NatijatMasah.Mutakarrir(awwal, thani, mada.Wasf());
            }

            IntPtr hadaf = new IntPtr(awwal.ToInt64() + Izaha);
            if (!Fahs(hadaf, in mada, out string radd))
            {
                return NatijatMasah.Marfud(hadaf, radd);
            }
            return NatijatMasah.Fareed(hadaf);
        }

        /// <summary>Runs every predicate against a resolved address.</summary>
        /// <param name="unwan">The address, after the offset was applied.</param>
        /// <param name="mada">The range and image the address came from.</param>
        /// <param name="sabab">Which predicate rejected it, and what it saw.</param>
        /// <returns>Whether every predicate accepted.</returns>
        /// <remarks>
        /// Public because <c>khatf</c> re-runs the predicates immediately before
        /// installing a detour. Between resolution and installation the module
        /// could in principle have been relocated by a protector that unpacks
        /// lazily, and the check costs microseconds.
        /// </remarks>
        public bool Fahs(IntPtr unwan, in MadaMasah mada, out string sabab)
        {
            for (int i = 0; i < Shurut.Count; i++)
            {
                SharkTahaqquq shart = Shurut[i];
                switch (shart.Naw)
                {
                    case NawShart.QismTanfidhi:
                        if (!mada.DakhilTanfidh(unwan))
                        {
                            sabab = $"0x{unwan.ToInt64():X} is outside the executable image of "
                                + $"{mada.Wasf()}, so it is not code";
                            return false;
                        }
                        break;

                    case NawShart.Muhadhah:
                        int muhadhah = mada.Muhadhah;
                        if (muhadhah > 1 && (unwan.ToInt64() % muhadhah) != 0)
                        {
                            sabab = $"0x{unwan.ToInt64():X} is not {muhadhah}-byte aligned, and "
                                + $"a {mada.Binya} entry point must be";
                            return false;
                        }
                        break;

                    case NawShart.Muqaddima:
                        if (!Muqaddim(unwan, in mada))
                        {
                            sabab = $"the bytes at 0x{unwan.ToInt64():X} are not a function "
                                + $"prologue this build recognises for {mada.Binya}, so the "
                                + "match is most likely inside a function rather than at one";
                            return false;
                        }
                        break;

                    case NawShart.BaytIndIzaha:
                    case NawShart.LaysBaytIndIzaha:
                        if (!Nadhir(shart, unwan, in mada, out sabab))
                        {
                            return false;
                        }
                        break;

                    default:
                        // An unrecognised predicate is a database written by a
                        // newer build. Refusing the address is the safe answer:
                        // the entry named a condition this build cannot check,
                        // and proceeding would apply a pattern with a guard
                        // silently removed.
                        sabab = $"predicate {(int)shart.Naw} is not one this build understands, "
                            + "so the address cannot be validated";
                        return false;
                }
            }
            sabab = string.Empty;
            return true;
        }

        /// <summary>Renders the pattern for a log line.</summary>
        /// <returns>The pattern text, its offset and its predicates.</returns>
        public override string ToString()
        {
            StringBuilder banna = new StringBuilder(Nass.Length + 48);
            banna.Append(Nass);
            if (Izaha != 0)
            {
                banna.Append(Izaha > 0 ? " +" : " -");
                banna.Append(Math.Abs(Izaha).ToString(CultureInfo.InvariantCulture));
            }
            for (int i = 0; i < Shurut.Count; i++)
            {
                banna.Append(i == 0 ? " [" : ", ");
                banna.Append(Shurut[i].ToString());
            }
            if (Shurut.Count != 0)
            {
                banna.Append(']');
            }
            return banna.ToString();
        }

        private bool Yutabiq(ReadOnlySpan<byte> kawm, int bidaya)
        {
            if (bidaya < 0 || bidaya + buyut.Length > kawm.Length)
            {
                return false;
            }
            for (int i = 0; i < buyut.Length; i++)
            {
                if ((kawm[bidaya + i] & aqnia[i]) != buyut[i])
                {
                    return false;
                }
            }
            return true;
        }

        private static bool Muqaddim(IntPtr unwan, in MadaMasah mada)
        {
            IReadOnlyList<Namat> anmat = Muqaddimat.Li(mada.Binya);
            for (int i = 0; i < anmat.Count; i++)
            {
                Namat wahid = anmat[i];
                if (Ind(unwan, in mada, wahid.buyut, wahid.aqnia))
                {
                    return true;
                }
            }
            return false;
        }

        private static bool Nadhir(
            SharkTahaqquq shart,
            IntPtr unwan,
            in MadaMasah mada,
            out string sabab)
        {
            if (!HawilHallil(shart.Qeema, 0, Array.Empty<SharkTahaqquq>(), mada.Binya,
                    out Namat? muqaran, out string khalalHall))
            {
                sabab = $"predicate {shart.Naw} carries the unusable pattern "
                    + $"'{shart.Qeema}': {khalalHall}";
                return false;
            }
            IntPtr mawdi = new IntPtr(unwan.ToInt64() + shart.Izaha);
            bool wujid = Ind(mawdi, in mada, muqaran.buyut, muqaran.aqnia);
            bool matlub = shart.Naw == NawShart.BaytIndIzaha;
            if (wujid == matlub)
            {
                sabab = string.Empty;
                return true;
            }
            sabab = matlub
                ? $"the bytes at 0x{mawdi.ToInt64():X} are not '{shart.Qeema}', which this "
                    + "pattern requires at that displacement"
                : $"the bytes at 0x{mawdi.ToInt64():X} are '{shart.Qeema}', which this pattern "
                    + "excludes at that displacement";
            return false;
        }

        private static bool Ind(IntPtr unwan, in MadaMasah mada, byte[] buyut, byte[] aqnia)
        {
            // Every read is bounded by the executable image before it happens.
            // A predicate that reads past the end of a section to decide whether
            // an address is valid would be a fault raised while diagnosing a
            // near miss, which .NET cannot turn back into a refusal.
            if (!mada.DakhilTanfidh(unwan))
            {
                return false;
            }
            long akhirSaliha = mada.BidayatTanfidh == IntPtr.Zero || mada.TulTanfidh <= 0
                ? mada.Bidaya.ToInt64() + mada.Tul
                : mada.BidayatTanfidh.ToInt64() + mada.TulTanfidh;
            if (unwan.ToInt64() + buyut.Length > akhirSaliha)
            {
                return false;
            }

            unsafe
            {
                byte* muashir = (byte*)unwan;
                for (int i = 0; i < buyut.Length; i++)
                {
                    if ((muashir[i] & aqnia[i]) != buyut[i])
                    {
                        return false;
                    }
                }
            }
            return true;
        }

        private static int[] BinaTakhatti(byte[] buyut, int bidayatMirsat, int tulMirsat)
        {
            // Horspool's table, over the anchor run only. A wildcard byte says
            // nothing about how far a window may be advanced, so a table built
            // across one would be unsound; the run is the longest stretch that
            // carries information.
            int[] jadwal = new int[256];
            for (int i = 0; i < jadwal.Length; i++)
            {
                jadwal[i] = tulMirsat;
            }
            for (int i = 0; i < tulMirsat - 1; i++)
            {
                jadwal[buyut[bidayatMirsat + i]] = tulMirsat - 1 - i;
            }
            return jadwal;
        }

        private static SharkTahaqquq[] Nasakh(IReadOnlyList<SharkTahaqquq> masdar)
        {
            SharkTahaqquq[] nuskha = new SharkTahaqquq[masdar.Count];
            for (int i = 0; i < masdar.Count; i++)
            {
                nuskha[i] = masdar[i];
            }
            return nuskha;
        }

        private static string Sawwi(byte[] buyut, byte[] aqnia)
        {
            StringBuilder banna = new StringBuilder(buyut.Length * 3);
            for (int i = 0; i < buyut.Length; i++)
            {
                if (i != 0)
                {
                    banna.Append(' ');
                }
                if (aqnia[i] == 0)
                {
                    banna.Append("??");
                    continue;
                }
                banna.Append(buyut[i].ToString("X2", CultureInfo.InvariantCulture));
            }
            return banna.ToString();
        }

        private static int Raqm(char harf)
        {
            if (harf >= '0' && harf <= '9')
            {
                return harf - '0';
            }
            if (harf >= 'A' && harf <= 'F')
            {
                return harf - 'A' + 10;
            }
            if (harf >= 'a' && harf <= 'f')
            {
                return harf - 'a' + 10;
            }
            return -1;
        }
    }
}
