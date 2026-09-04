// مهماز — the spur: the detour installer that drives a live native function
// into our replacement and keeps a way back to the original.
//
// This is the file the other three exist for. Tuul says how long each prologue
// instruction is; Naqla moves those instructions to a new address without
// changing what they mean; Takhsis finds executable memory close enough to jump
// to in a few bytes. Mihmaz spends all three to do one thing: overwrite the
// first bytes of a target function with a jump to a replacement, while
// preserving the overwritten instructions in a trampoline the replacement can
// call to run the original.
//
// THE SHAPE OF AN INSTALLED HOOK. Write T for the target, R for the
// replacement, and M for the trampoline that Takhsis placed in reach of T:
//
//   * The first N bytes of T are replaced by a jump to R, the replacement.
//     N is the smallest whole number of instructions that covers the patch
//     (5 bytes on x86-64, 4 on ARM64), never a partial instruction.
//   * M holds the original N bytes, RELOCATED by Naqla, followed by a jump back
//     to T+N. Calling M therefore runs the original prologue and then the rest
//     of the original function. R calls M when it wants the original behaviour.
//   * Because R (a managed method's native entry) is usually further than a
//     short jump can reach, the jump out of T does not target R directly; it
//     targets a small absolute-jump thunk inside M's allocation, which is in
//     reach, and the thunk jumps to R. That keeps the patch at T down to the
//     5 (or 4) bytes a short prologue can spare.
//
// THE HIDDEN MethodInfo* ARGUMENT — THE ONE IL2CPP-SPECIFIC HAZARD.
//
// Every method IL2CPP compiles from managed code takes one more argument than
// its managed signature shows: a trailing `MethodInfo*`, appended AFTER the
// declared parameters (and after the implicit `this` of an instance method).
// A managed `int Damage(float amount)` on a class becomes, in the native
// binary, `int Damage(void* self, float amount, MethodInfo* method)`. The
// pointer is passed in the next ordinary argument register or stack slot, under
// the platform's ordinary calling convention (Microsoft x64, System V, or the
// ARM64 AAPCS) — there is nothing exotic about how it is passed, only that it
// is THERE and unmentioned.
//
// The consequence for a replacement is exact and unforgiving: the replacement's
// native signature must declare that trailing pointer, or every argument after
// the mismatch is read from the wrong place. A replacement written as an
// [UnmanagedCallersOnly] static method, or bound through an unmanaged function
// pointer, must therefore be
//     static ReturnType Badil(void* self, <the declared params...>, IntPtr method)
// for an instance target, dropping `self` for a static one. When the
// replacement wants the original, it calls M (see <see cref="Muaqqat"/>) with
// the SAME arguments including that `method` pointer, because M is the original
// native code and expects the register state the function was entered with. Get
// the trailing pointer wrong and the failure is not a clean crash: it is the
// original function reading its MethodInfo out of a float, which is the kind of
// corruption that surfaces three calls later.
//
// This file does not fabricate the replacement — the adapter writes that, in a
// place that can name the game's types. Mihmaz installs whatever native pointer
// it is handed. What it guarantees is that the ABI of the seam it builds is the
// plain one described above, so a correctly declared replacement simply works.
//
// THREAD SAFETY, STATED HONESTLY. Overwriting the first bytes of a function is
// not atomic. If another thread is executing exactly those bytes at the instant
// they are half-written, it executes a half-written jump, and that is a crash
// with no owner. Mihmaz does NOT suspend threads, does NOT use a serializing
// cross-modification protocol, and does not pretend to. Its mitigation is a
// discipline, not a mechanism: Taarib installs its hooks during plugin
// initialisation, which under BepInEx runs before the game has begun driving
// its own logic, so the targeted functions are quiescent while they are
// patched. The one thing enforced in code is the refusal to patch a function
// whose prologue is too short to hold the patch without spilling into whatever
// follows it — because that failure is detectable and silent corruption of the
// next function is not.
//
// IDEMPOTENCE AND PARTIAL FAILURE. Installation touches the target's bytes only
// as its final step; everything before it — decoding, allocation, building and
// sealing the trampoline — is done off to the side, and if any of it fails the
// allocation is released and the target is left untouched. Removal restores the
// saved original bytes exactly and is idempotent: calling it twice, or
// disposing an already-removed hook, does nothing the second time. A hook that
// fails to install leaves no trace; a hook that installs can always be removed
// to the byte.
//
// ALREADY DETOURED BY SOMEONE ELSE. If the target's prologue is already a jump,
// another plugin has already hooked it, and Mihmaz refuses rather than hooking
// on top. Chaining blind is how two mod frameworks corrupt each other: the
// second framework copies the first's jump into its trampoline as if it were
// original code, so "call the original" now calls the first framework's
// replacement, and removing either hook restores bytes the other has already
// moved. Two frameworks that both chain leave a target that cannot be returned
// to its real original by anyone. Refusing is the only cooperative behaviour;
// the refusal names the conflict so the player can be told which two mods
// disagree.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Khatf
{
    /// <summary>
    /// One installed native detour: the patch over a target function, the
    /// trampoline that preserves its original prologue, and the operations to
    /// remove it exactly.
    /// </summary>
    /// <remarks>
    /// Build one with <see cref="Rakkib"/> during plugin initialisation, hold
    /// it for as long as the hook should live, and <see cref="Dispose"/> it to
    /// restore the target. Not thread-safe: install and remove from the same
    /// initialisation and shutdown paths that own every other hook, never from
    /// a frame while the game is running the targeted code.
    /// </remarks>
    public sealed class Mihmaz : IDisposable
    {
        private const int PatchX64 = 5;
        private const int PatchArm = 4;
        private const int HududQiraaX64 = 24;
        private const int HududQiraaArm = 16;

        private readonly IntPtr hadaf;
        private readonly IntPtr badil;
        private readonly Jazira jazira;
        private readonly byte[] asli;
        private bool munazzal;

        private Mihmaz(IntPtr hadaf, IntPtr badil, Jazira jazira, byte[] asli)
        {
            this.hadaf = hadaf;
            this.badil = badil;
            this.jazira = jazira;
            this.asli = asli;
        }

        /// <summary>The address of the target function that was patched.</summary>
        public IntPtr Hadaf => hadaf;

        /// <summary>The replacement the patch jumps to.</summary>
        public IntPtr Badil => badil;

        /// <summary>
        /// The trampoline entry: call this to run the original function. It
        /// executes the original prologue and continues into the untouched rest
        /// of the target, so it behaves exactly as the target did before the
        /// hook — including expecting the target's arguments, the implicit
        /// <c>this</c>, and the trailing <c>MethodInfo*</c>.
        /// </summary>
        public IntPtr Muaqqat => jazira.Unwan;

        /// <summary>How many original bytes the patch overwrote.</summary>
        public int TulManqul => asli.Length;

        /// <summary>Whether the hook has been removed.</summary>
        public bool Munazzal => munazzal;

        /// <summary>
        /// Installs a detour over <paramref name="hadaf"/> that jumps to
        /// <paramref name="badil"/>.
        /// </summary>
        /// <param name="hadaf">The target function's native entry point.</param>
        /// <param name="badil">
        /// The replacement's native entry point — an unmanaged-callable pointer
        /// whose signature mirrors the target's, trailing <c>MethodInfo*</c>
        /// included (see this file's header).
        /// </param>
        /// <returns>The installed hook; keep it to call through or to remove.</returns>
        /// <exception cref="KhataTaarib">
        /// The target is already detoured, its prologue is too short or cannot be
        /// decoded, no trampoline could be placed in reach, or the running
        /// architecture is not one this installer implements. In every case the
        /// target is left exactly as it was.
        /// </exception>
        public static Mihmaz Rakkib(IntPtr hadaf, IntPtr badil)
        {
            if (hadaf == IntPtr.Zero || badil == IntPtr.Zero)
            {
                throw new KhataTaarib(
                    Ramz.MuashirBatil,
                    "TAARIB-E-6532",
                    "طُلب تركيب تحويلٍ على عنوانٍ فارغ أو إلى بديلٍ فارغ؛ هذا خلل في الملحق.",
                    "A detour was requested over a null target or to a null replacement; "
                        + "this is a bug in the plugin.",
                    Khutwa.IblaghLilMalik);
            }

            switch (System.Runtime.InteropServices.RuntimeInformation.ProcessArchitecture)
            {
                case System.Runtime.InteropServices.Architecture.X64:
                    return RakkibX64(hadaf, badil);
                case System.Runtime.InteropServices.Architecture.Arm64:
                    return RakkibArm(hadaf, badil);
                default:
                    throw new KhataTaarib(
                        Ramz.GhayrMadum,
                        "TAARIB-E-6534",
                        "لا يدعم هذا الإصدار تركيب التحويلات على معمار المعالج الحالي؛ تعمل "
                            + "اللعبة بلغتها الأصلية.",
                        "This build does not install detours on the current processor "
                            + "architecture; the game keeps running in its original language.",
                        Khutwa.FathTashkhis);
            }
        }

        /// <summary>
        /// Removes the hook, restoring the target's original bytes exactly.
        /// Idempotent.
        /// </summary>
        /// <remarks>
        /// The trampoline memory is not freed here, only on <see cref="Dispose"/>,
        /// because a call already in flight through <see cref="Muaqqat"/> is
        /// still executing those bytes. Freeing them under a running call is the
        /// same crash removal exists to avoid; the same quiescence discipline
        /// that makes patching safe makes disposal safe.
        /// </remarks>
        /// <exception cref="KhataTaarib">
        /// The page protection could not be changed to restore the bytes.
        /// </exception>
        public void Azil()
        {
            if (munazzal)
            {
                return;
            }
            Takhsis.IktubFawqShifra(hadaf, asli);
            munazzal = true;
        }

        /// <summary>Removes the hook if present and frees the trampoline.</summary>
        public void Dispose()
        {
            Azil();
            jazira.Dispose();
        }

        private static Mihmaz RakkibX64(IntPtr hadaf, IntPtr badil)
        {
            byte[] awwal = IqraHadaf(hadaf, HududQiraaX64);

            if (awwal[0] == 0xE9 || awwal[0] == 0xEB
                || (awwal[0] == 0xFF && awwal[1] == 0x25))
            {
                throw MudawwanBalfil();
            }

            int n = 0;
            while (n < PatchX64)
            {
                if (!Tuul.Iqra(awwal.AsSpan(n), out Amr a))
                {
                    throw LaYufakkak();
                }
                if ((a.Sinf == SinfAmr.Rujoo || a.Sinf == SinfAmr.Kasr)
                    && n + a.Tul < PatchX64)
                {
                    throw MuqaddimaQasira();
                }
                n += a.Tul;
                if (n > HududQiraaX64)
                {
                    throw LaYufakkak();
                }
            }

            int tulManqul = n;
            nuint hajm = (nuint)((tulManqul * 2) + 64);
            Jazira jazira = Takhsis.Ihjuz(hadaf, hajm, Takhsis.MadaX64);
            try
            {
                long trampBase = jazira.Unwan.ToInt64();
                byte[] scratch = new byte[(tulManqul * 2) + 16];
                int outLen = 0;
                int off = 0;
                while (off < tulManqul)
                {
                    Tuul.Iqra(awwal.AsSpan(off), out Amr a);
                    int wrote = Naqla.AnqilAmr(
                        in a,
                        awwal.AsSpan(off, a.Tul),
                        new IntPtr(hadaf.ToInt64() + off),
                        new IntPtr(trampBase + outLen),
                        hadaf,
                        new IntPtr(hadaf.ToInt64() + tulManqul),
                        scratch.AsSpan(outLen));
                    outLen += wrote;
                    off += a.Tul;
                }

                long jbFrom = trampBase + outLen;
                long jbRel = (hadaf.ToInt64() + tulManqul) - (jbFrom + 5);
                if (!YasaI32(jbRel))
                {
                    throw KhrujMinMada();
                }
                scratch[outLen] = 0xE9;
                IktubI32(scratch, outLen + 1, (int)jbRel);
                int trampLen = outLen + 5;

                int thunkOff = trampLen;
                long thunkAddr = trampBase + thunkOff;
                byte[] thunk = new byte[14];
                thunk[0] = 0xFF;
                thunk[1] = 0x25; // JMP [rip + 0]: the absolute target follows.
                IktubI64(thunk, 6, badil.ToInt64());

                jazira.Iktub(scratch.AsSpan(0, trampLen), 0);
                jazira.Iktub(thunk, thunkOff);
                jazira.Uhkim();

                long pRel = thunkAddr - (hadaf.ToInt64() + 5);
                if (!YasaI32(pRel))
                {
                    throw KhrujMinMada();
                }
                byte[] patch = new byte[tulManqul];
                for (int i = 0; i < patch.Length; i++)
                {
                    patch[i] = 0x90; // NOP-fill the tail of the overwritten span.
                }
                patch[0] = 0xE9;
                IktubI32(patch, 1, (int)pRel);

                byte[] asli = new byte[tulManqul];
                Array.Copy(awwal, asli, tulManqul);

                Takhsis.IktubFawqShifra(hadaf, patch);
                return new Mihmaz(hadaf, badil, jazira, asli);
            }
            catch
            {
                jazira.Dispose();
                throw;
            }
        }

        private static Mihmaz RakkibArm(IntPtr hadaf, IntPtr badil)
        {
            byte[] awwal = IqraHadaf(hadaf, HududQiraaArm);
            Tuul.IqraArm(awwal.AsSpan(0, 4), out AmrArm a0);
            uint w1 = ReadU32(awwal, 4);

            if (a0.Sinf == SinfAmrArm.Far
                || (a0.Kalima == 0x58000050u && w1 == 0xD61F0200u))
            {
                throw MudawwanBalfil();
            }

            const int tulManqul = PatchArm;
            nuint hajm = (nuint)(4 + 16 + 16 + 16);
            Jazira jazira = Takhsis.Ihjuz(hadaf, hajm, Takhsis.MadaArm64);
            try
            {
                long trampBase = jazira.Unwan.ToInt64();
                uint reloc = Naqla.AnqilAmrArm(
                    in a0,
                    hadaf,
                    jazira.Unwan,
                    hadaf,
                    new IntPtr(hadaf.ToInt64() + tulManqul));

                byte[] tramp = new byte[4 + 16];
                IktubU32(tramp, 0, reloc);
                int jb = FaraArm(tramp, 4, trampBase + 4, hadaf.ToInt64() + tulManqul);
                int trampLen = 4 + jb;

                int islandOff = trampLen;
                long islandAddr = trampBase + islandOff;
                byte[] island = new byte[16];
                IktubU32(island, 0, 0x58000050u); // LDR x16, #8
                IktubU32(island, 4, 0xD61F0200u); // BR  x16
                IktubI64(island, 8, badil.ToInt64());

                jazira.Iktub(tramp.AsSpan(0, trampLen), 0);
                jazira.Iktub(island, islandOff);
                jazira.Uhkim();

                long pOff = islandAddr - hadaf.ToInt64();
                if ((pOff & 3) != 0 || !YasaImaraat(pOff >> 2, 26))
                {
                    throw KhrujMinMada();
                }
                byte[] patch = new byte[4];
                IktubU32(patch, 0, 0x14000000u | (uint)((pOff >> 2) & 0x03FFFFFF));

                byte[] asli = new byte[tulManqul];
                Array.Copy(awwal, asli, tulManqul);

                Takhsis.IktubFawqShifra(hadaf, patch);
                return new Mihmaz(hadaf, badil, jazira, asli);
            }
            catch
            {
                jazira.Dispose();
                throw;
            }
        }

        /// <summary>
        /// Emits an ARM64 jump from <paramref name="here"/> to
        /// <paramref name="target"/>: a single <c>B</c> when it is in range,
        /// otherwise a 16-byte absolute load-and-branch.
        /// </summary>
        private static int FaraArm(byte[] buf, int at, long here, long target)
        {
            long off = target - here;
            if ((off & 3) == 0 && YasaImaraat(off >> 2, 26))
            {
                IktubU32(buf, at, 0x14000000u | (uint)((off >> 2) & 0x03FFFFFF));
                return 4;
            }
            IktubU32(buf, at, 0x58000050u);      // LDR x16, #8
            IktubU32(buf, at + 4, 0xD61F0200u);  // BR  x16
            IktubI64(buf, at + 8, target);
            return 16;
        }

        private static unsafe byte[] IqraHadaf(IntPtr hadaf, int adad)
        {
            byte[] b = new byte[adad];
            // Sound: `hadaf` is a function entry point in this process's own
            // executable image, which is readable, and `adad` bounds the read to
            // the buffer just allocated.
            new ReadOnlySpan<byte>((void*)hadaf, adad).CopyTo(b);
            return b;
        }

        private static KhataTaarib MudawwanBalfil()
        {
            return new KhataTaarib(
                Ramz.QeemaBatila,
                "TAARIB-E-6531",
                "أوّل الدالة الهدف قفزةٌ بالفعل: ملحقٌ آخر ركّب تحويلًا عليها. رُفض التركيب "
                    + "فوقه بدل تسلسلٍ يُفسد به الإطاران بعضهما، ولا يمكن بعده إعادة الدالة "
                    + "إلى أصلها.",
                "The target's prologue is already a jump: another plugin has detoured it. "
                    + "Stacking on top is refused rather than chaining, which would let the "
                    + "two frameworks corrupt each other and leave the function impossible to "
                    + "restore.",
                Khutwa.FathTashkhis);
        }

        private static KhataTaarib MuqaddimaQasira()
        {
            return new KhataTaarib(
                Ramz.QeemaBatila,
                "TAARIB-E-6530",
                "أوّل الدالة الهدف أقصر من أن يتّسع للرقعة دون تجاوزٍ إلى ما بعدها؛ رُفض "
                    + "التركيب بدل الكتابة فوق دالةٍ مجاورة.",
                "The target's prologue is too short to hold the patch without spilling past "
                    + "it; the hook is refused rather than overwriting a neighbouring "
                    + "function.",
                Khutwa.FathTashkhis);
        }

        private static KhataTaarib LaYufakkak()
        {
            return new KhataTaarib(
                Ramz.QeemaBatila,
                "TAARIB-E-6533",
                "تعذّر فكّ ترميز أوّل الدالة الهدف إلى تعليماتٍ كاملة؛ رُفض التركيب بدل نسخ "
                    + "نصف تعليمةٍ إلى المنطقة الوسيطة.",
                "The target's prologue could not be decoded into whole instructions; the "
                    + "hook is refused rather than copying half an instruction into the "
                    + "trampoline.",
                Khutwa.FathTashkhis);
        }

        private static KhataTaarib KhrujMinMada()
        {
            return new KhataTaarib(
                Ramz.QeemaBatila,
                "TAARIB-E-6535",
                "خرجت إحدى قفزات التحويل عن مداها رغم وضع المنطقة الوسيطة قريبًا؛ رُفض "
                    + "التركيب بدل قفزةٍ إلى موضعٍ خاطئ.",
                "One of the detour's jumps fell out of range despite the trampoline being "
                    + "placed nearby; the hook is refused rather than jumping to a wrong "
                    + "address.",
                Khutwa.FathTashkhis);
        }

        private static bool YasaI32(long v)
        {
            return v >= int.MinValue && v <= int.MaxValue;
        }

        private static bool YasaImaraat(long v, int bits)
        {
            long hadd = 1L << (bits - 1);
            return v >= -hadd && v <= hadd - 1;
        }

        private static uint ReadU32(byte[] b, int at)
        {
            return (uint)(b[at] | (b[at + 1] << 8) | (b[at + 2] << 16) | (b[at + 3] << 24));
        }

        private static void IktubU32(byte[] b, int at, uint v)
        {
            b[at] = (byte)v;
            b[at + 1] = (byte)(v >> 8);
            b[at + 2] = (byte)(v >> 16);
            b[at + 3] = (byte)(v >> 24);
        }

        private static void IktubI32(byte[] b, int at, int v)
        {
            IktubU32(b, at, (uint)v);
        }

        private static void IktubI64(byte[] b, int at, long v)
        {
            IktubU32(b, at, (uint)v);
            IktubU32(b, at + 4, (uint)(v >> 32));
        }
    }
}
