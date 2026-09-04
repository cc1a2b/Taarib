// نقلة — moving a copied instruction so it still means what it meant.
//
// Tuul decides how many bytes to copy. It does not follow that the copy means
// the same thing at its new address, and for a large class of instructions it
// does not. Anything whose encoding names a place RELATIVE to itself — a
// RIP-relative memory operand, a relative jump, call or conditional branch —
// was measured from where it used to sit. Copy it four kilobytes away and it
// now addresses somewhere four kilobytes off from what it meant. This file is
// the arithmetic that puts that right, and the refusal for the cases where it
// cannot be put right.
//
// THE ONE RULE THAT MATTERS. A relocation that cannot be performed correctly is
// a refusal. It is never a truncation, never a best effort, never "close
// enough". The reason is specific and it is severe: a relative branch whose
// displacement was silently truncated does not fault at the trampoline. It
// transfers control to whatever byte the wrong displacement happens to land on
// — the middle of an unrelated function, the middle of an unrelated
// instruction — and executes from there. The symptom is a crash somewhere else
// entirely, on someone else's frame, in a game we do not have the source to,
// with a call stack that names nothing that is wrong. There is no debugging
// that after the fact. So when the numbers do not fit, this code throws, the
// hook does not install, and the game keeps running in its original language —
// which is the whole product's failure contract: degrade to the original, never
// corrupt.
//
// THE RANGES, NUMERICALLY.
//   x86-64 RIP-relative disp32:  the recomputed displacement must fit a signed
//                                32-bit field, i.e. [-2^31, 2^31 - 1].
//   x86-64 rel8:                 [-128, 127] from the next instruction.
//   x86-64 rel32:                [-2^31, 2^31 - 1] from the next instruction.
//   A rel8 branch that no longer reaches is WIDENED to its rel32 form where one
//   exists (EB -> E9, 70+cc -> 0F 80+cc). LOOP/LOOPE/LOOPNE/JRCXZ have no near
//   form; when one of those no longer reaches, there is nothing to widen into
//   and the relocation is refused.
//   ARM64 ADR:      signed 21-bit byte offset, +/-1 MB.
//   ARM64 ADRP:     signed 21-bit offset scaled by 4096, +/-4 GB, 4 KB aligned.
//   ARM64 B/BL:     signed 26-bit offset scaled by 4, +/-128 MB.
//   ARM64 B.cond,
//         CBZ/CBNZ,
//         LDR-literal: signed 19-bit offset scaled by 4, +/-1 MB.
//   ARM64 TBZ/TBNZ: signed 14-bit offset scaled by 4, +/-32 KB.
//   ARM has no widening: a word is a word. Out of range is a refusal, and it is
//   Takhsis's job to keep the trampoline near enough that it does not happen.
//
// TARGETS INSIDE THE OVERWRITTEN BYTES. A branch or a PC-relative reference
// whose destination falls inside the very bytes the detour is about to
// overwrite is refused, unconditionally. Those bytes will shortly be a jump to
// our replacement; the instruction the branch used to reach no longer exists as
// itself. Relocating the branch to point at the original address would aim it
// at our patch, and relocating it to point "at the copy in the trampoline"
// would require a second fix-up pass this code deliberately does not attempt.
// A prologue that branches into its own first bytes is pathological and rare;
// meeting one is a reason to leave the function alone, not to improvise.

using System;
using Taarib.Unity.Jisr;

namespace Taarib.Unity.Il2cpp.Khatf
{
    /// <summary>
    /// Relocates copied instructions from an original address to a trampoline
    /// address, rewriting every position-dependent field and refusing any
    /// instruction that cannot be moved without changing its meaning.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Stateless. Each call relocates exactly one instruction, because the
    /// address an instruction lands at depends on how many bytes the
    /// instructions before it grew to — a rel8 widened to a rel32 pushes
    /// everything after it along by three bytes — and that running address is
    /// bookkeeping only <see cref="Mihmaz"/>, which owns the trampoline layout,
    /// can keep. This file supplies the per-instruction arithmetic; the layout
    /// loop is in Mihmaz.
    /// </para>
    /// <para>
    /// Every branch and PC-relative target is treated as an ABSOLUTE address,
    /// reconstructed from the original instruction's own address plus the
    /// encoded displacement. That is what makes a relocated branch still reach
    /// the same code: the copy points at the original destination, wherever the
    /// copy itself ends up.
    /// </para>
    /// </remarks>
    public static class Naqla
    {
        /// <summary>
        /// Relocates one x86-64 instruction into <paramref name="hadaf"/>.
        /// </summary>
        /// <param name="amr">The decoded instruction, from <see cref="Tuul.Iqra"/>.</param>
        /// <param name="masdar">
        /// The original bytes; at least <see cref="Amr.Tul"/> long.
        /// </param>
        /// <param name="unwanAsli">The instruction's original absolute address.</param>
        /// <param name="unwanJadid">
        /// The absolute address it will occupy in the trampoline.
        /// </param>
        /// <param name="bidayatBaqaa">
        /// First address of the region being overwritten by the detour.
        /// </param>
        /// <param name="nihayatBaqaa">
        /// One past the last overwritten address. A control-flow or PC-relative
        /// target inside <c>[bidayatBaqaa, nihayatBaqaa)</c> is refused.
        /// </param>
        /// <param name="hadaf">
        /// Where to write the relocated instruction; may need to be longer than
        /// the source when a short branch is widened.
        /// </param>
        /// <returns>How many bytes were written, which can exceed the source length.</returns>
        /// <exception cref="ArgumentException">
        /// <paramref name="masdar"/> is shorter than the instruction, or
        /// <paramref name="hadaf"/> cannot hold the result — both of which are
        /// bugs in the caller's buffer sizing rather than anything about the
        /// target.
        /// </exception>
        /// <exception cref="KhataTaarib">
        /// The instruction cannot be relocated correctly: a displacement that no
        /// longer fits, a short branch with no wider form left, or a target
        /// inside the bytes being overwritten.
        /// </exception>
        public static int AnqilAmr(
            in Amr amr,
            ReadOnlySpan<byte> masdar,
            IntPtr unwanAsli,
            IntPtr unwanJadid,
            IntPtr bidayatBaqaa,
            IntPtr nihayatBaqaa,
            Span<byte> hadaf)
        {
            if (!amr.Salih || masdar.Length < amr.Tul)
            {
                throw new ArgumentException(
                    "The source is shorter than the decoded instruction.", nameof(masdar));
            }

            long asli = unwanAsli.ToInt64();
            long jadid = unwanJadid.ToInt64();
            long baqaaMin = bidayatBaqaa.ToInt64();
            long baqaaMax = nihayatBaqaa.ToInt64();

            switch (amr.Sinf)
            {
                case SinfAmr.Adi:
                case SinfAmr.Rujoo:
                case SinfAmr.Kasr:
                    return Insakh(masdar, amr.Tul, hadaf);

                case SinfAmr.NisbatRip:
                {
                    long target = asli + amr.Tul + amr.Qeema;
                    RfudDakhilIqlim(target, baqaaMin, baqaaMax);
                    long jadidNext = jadid + amr.Tul;
                    long disp = target - jadidNext;
                    if (!YasaI32(disp))
                    {
                        throw Rafd(
                            "TAARIB-E-6510",
                            "تعذّر نقل تعليمة تقرأ من عنوان نسبيّ إلى مؤشّر التعليمة؛ "
                                + "الإزاحة الجديدة أكبر من أن تُمثَّل في 32 بت، فرُفض التركيب "
                                + "بدل بتر عنوانٍ يقود إلى قراءة من موضع خاطئ.",
                            "A RIP-relative instruction could not be moved: the new "
                                + "displacement no longer fits a signed 32-bit field, so the "
                                + "hook is refused rather than truncated into a read from the "
                                + "wrong address.");
                    }
                    int tul = Insakh(masdar, amr.Tul, hadaf);
                    IktubI32(hadaf, amr.MawqiHagl, (int)disp);
                    return tul;
                }

                case SinfAmr.QafzaNisbiya:
                    return AnqilQafza(in amr, masdar, jadid, baqaaMin, baqaaMax, asli, hadaf);

                default:
                    // Every SinfAmr value has a case above; a value outside them
                    // is a decoder that produced something this relocator does
                    // not understand, which must not be copied blind.
                    throw Rafd(
                        "TAARIB-E-6519",
                        "صُنِّفت تعليمة تصنيفًا لا يعرف الناقل كيف يتعامل معه؛ رُفض النقل "
                            + "بدل نسخ تعليمةٍ قد تتغيّر دلالتها.",
                        "An instruction was classified in a way this relocator does not "
                            + "handle; the move is refused rather than copying an instruction "
                            + "whose meaning may change.");
            }
        }

        /// <summary>
        /// Relocates one A64 instruction and returns the rewritten word.
        /// </summary>
        /// <param name="amr">The decoded instruction, from <see cref="Tuul.IqraArm"/>.</param>
        /// <param name="unwanAsli">Its original absolute address.</param>
        /// <param name="unwanJadid">The address it will occupy in the trampoline.</param>
        /// <param name="bidayatBaqaa">First overwritten address.</param>
        /// <param name="nihayatBaqaa">One past the last overwritten address.</param>
        /// <returns>The relocated 32-bit instruction word.</returns>
        /// <exception cref="KhataTaarib">
        /// The PC-relative offset no longer fits the instruction's field — ARM
        /// has no wider form to grow into — or the target lies inside the
        /// overwritten bytes.
        /// </exception>
        public static uint AnqilAmrArm(
            in AmrArm amr,
            IntPtr unwanAsli,
            IntPtr unwanJadid,
            IntPtr bidayatBaqaa,
            IntPtr nihayatBaqaa)
        {
            long asli = unwanAsli.ToInt64();
            long jadid = unwanJadid.ToInt64();
            long baqaaMin = bidayatBaqaa.ToInt64();
            long baqaaMax = nihayatBaqaa.ToInt64();
            uint w = amr.Kalima;

            switch (amr.Sinf)
            {
                case SinfAmrArm.Adi:
                case SinfAmrArm.Rujoo:
                case SinfAmrArm.Kasr:
                    return w;

                case SinfAmrArm.Adr:
                {
                    long off = ImmAdr(w);
                    long target = asli + off;
                    RfudDakhilIqlim(target, baqaaMin, baqaaMax);
                    long jadidOff = target - jadid;
                    RfudMadaArm(YasaImaraat(jadidOff, 21), "ADR");
                    return DamjAdr(w, jadidOff);
                }

                case SinfAmrArm.Adrp:
                {
                    long imm = ImmAdr(w);
                    long targetPage = (asli & ~0xFFFL) + (imm << 12);
                    RfudDakhilIqlim(targetPage, baqaaMin, baqaaMax);
                    long jadidPages = (targetPage - (jadid & ~0xFFFL)) >> 12;
                    RfudMadaArm(YasaImaraat(jadidPages, 21), "ADRP");
                    return DamjAdr(w, jadidPages);
                }

                case SinfAmrArm.Far:
                case SinfAmrArm.Nida:
                {
                    long off = SignExtend((long)(w & 0x03FFFFFFu) << 2, 28);
                    long target = asli + off;
                    RfudDakhilIqlim(target, baqaaMin, baqaaMax);
                    long jadidOff = target - jadid;
                    RfudMadaArm((jadidOff & 3) == 0 && YasaImaraat(jadidOff >> 2, 26), "B/BL");
                    return (w & ~0x03FFFFFFu) | (uint)((jadidOff >> 2) & 0x03FFFFFF);
                }

                case SinfAmrArm.Shart:
                case SinfAmrArm.Muqarana:
                case SinfAmrArm.HimlNisbi:
                {
                    long off = SignExtend((long)((w >> 5) & 0x7FFFFu) << 2, 21);
                    long target = asli + off;
                    RfudDakhilIqlim(target, baqaaMin, baqaaMax);
                    long jadidOff = target - jadid;
                    RfudMadaArm((jadidOff & 3) == 0 && YasaImaraat(jadidOff >> 2, 19), "imm19");
                    uint imm19 = (uint)((jadidOff >> 2) & 0x7FFFF);
                    return (w & ~(0x7FFFFu << 5)) | (imm19 << 5);
                }

                case SinfAmrArm.Bit:
                {
                    long off = SignExtend((long)((w >> 5) & 0x3FFFu) << 2, 16);
                    long target = asli + off;
                    RfudDakhilIqlim(target, baqaaMin, baqaaMax);
                    long jadidOff = target - jadid;
                    RfudMadaArm((jadidOff & 3) == 0 && YasaImaraat(jadidOff >> 2, 14), "TBZ/TBNZ");
                    uint imm14 = (uint)((jadidOff >> 2) & 0x3FFF);
                    return (w & ~(0x3FFFu << 5)) | (imm14 << 5);
                }

                default:
                    throw Rafd(
                        "TAARIB-E-6519",
                        "صُنِّفت تعليمة تصنيفًا لا يعرف الناقل كيف يتعامل معه؛ رُفض النقل.",
                        "An instruction was classified in a way this relocator does not "
                            + "handle; the move is refused.");
            }
        }

        private static int AnqilQafza(
            in Amr amr,
            ReadOnlySpan<byte> masdar,
            long jadid,
            long baqaaMin,
            long baqaaMax,
            long asli,
            Span<byte> hadaf)
        {
            long target = asli + amr.Tul + amr.Qeema;
            RfudDakhilIqlim(target, baqaaMin, baqaaMax);

            switch (amr.Naw)
            {
                case NawQafza.QafzaBaida:
                case NawQafza.Nida:
                case NawQafza.ShartBaid:
                {
                    // Already a rel32; keep the exact bytes and rewrite the field.
                    long rel = target - (jadid + amr.Tul);
                    if (!YasaI32(rel))
                    {
                        throw RfudBaid();
                    }
                    int tul = Insakh(masdar, amr.Tul, hadaf);
                    IktubI32(hadaf, amr.MawqiHagl, (int)rel);
                    return tul;
                }

                case NawQafza.QafzaQasira:
                {
                    long rel8 = target - (jadid + amr.Tul);
                    if (YasaRel8(rel8))
                    {
                        int tul = Insakh(masdar, amr.Tul, hadaf);
                        hadaf[amr.MawqiHagl] = (byte)(sbyte)rel8;
                        return tul;
                    }
                    long rel = target - (jadid + 5);
                    if (!YasaI32(rel))
                    {
                        throw RfudBaid();
                    }
                    if (hadaf.Length < 5)
                    {
                        throw QillatMakhzan();
                    }
                    hadaf[0] = 0xE9;
                    IktubI32(hadaf, 1, (int)rel);
                    return 5;
                }

                case NawQafza.ShartQasir:
                {
                    long rel8 = target - (jadid + amr.Tul);
                    if (YasaRel8(rel8))
                    {
                        int tul = Insakh(masdar, amr.Tul, hadaf);
                        hadaf[amr.MawqiHagl] = (byte)(sbyte)rel8;
                        return tul;
                    }
                    long rel = target - (jadid + 6);
                    if (!YasaI32(rel))
                    {
                        throw RfudBaid();
                    }
                    if (hadaf.Length < 6)
                    {
                        throw QillatMakhzan();
                    }
                    hadaf[0] = 0x0F;
                    hadaf[1] = (byte)(0x80 | (amr.Shart & 0x0F));
                    IktubI32(hadaf, 2, (int)rel);
                    return 6;
                }

                default:
                {
                    // BilaShaklBaid: LOOP/LOOPE/LOOPNE/JRCXZ. There is no near
                    // form, so a target out of rel8 range cannot be re-encoded
                    // at all.
                    long rel8 = target - (jadid + amr.Tul);
                    if (YasaRel8(rel8))
                    {
                        int tul = Insakh(masdar, amr.Tul, hadaf);
                        hadaf[amr.MawqiHagl] = (byte)(sbyte)rel8;
                        return tul;
                    }
                    throw Rafd(
                        "TAARIB-E-6512",
                        "يحوي أوّل الدالة قفزةً قصيرةً من نوعٍ لا نظير واسع له "
                            + "(LOOP/JRCXZ) وصار هدفها خارج مدى البايت الواحد بعد النقل؛ "
                            + "لا سبيل لإعادة ترميزها، فرُفض التركيب.",
                        "The prologue contains a short branch with no wide form (LOOP or "
                            + "JRCXZ) whose target is out of one-byte range after the move; "
                            + "it cannot be re-encoded, so the hook is refused.");
                }
            }
        }

        private static int Insakh(ReadOnlySpan<byte> masdar, int tul, Span<byte> hadaf)
        {
            if (hadaf.Length < tul)
            {
                throw QillatMakhzan();
            }
            masdar.Slice(0, tul).CopyTo(hadaf);
            return tul;
        }

        private static void RfudDakhilIqlim(long target, long min, long max)
        {
            if (target >= min && target < max)
            {
                throw Rafd(
                    "TAARIB-E-6513",
                    "تشير تعليمةٌ في أوّل الدالة إلى موضعٍ داخل البايتات التي سيكتب عليها "
                        + "التحويل نفسه؛ تلك البايتات لن تبقى كما هي، فرُفض النقل بدل توجيه "
                        + "التعليمة إلى رقعتنا.",
                    "An instruction in the prologue targets an address inside the very "
                        + "bytes the detour is about to overwrite; those bytes will not "
                        + "survive, so the move is refused rather than aimed at our patch.");
            }
        }

        private static void RfudMadaArm(bool yasa, string ism)
        {
            if (yasa)
            {
                return;
            }
            throw Rafd(
                "TAARIB-E-6516",
                $"تعليمةٌ نسبيّةٌ لعدّاد البرنامج ({ism}) صار هدفها خارج مدى حقلها بعد النقل، "
                    + "ولا يملك معمار ARM صيغةً أوسع ننقلها إليها؛ رُفض التركيب بدل بتر الإزاحة.",
                $"A PC-relative instruction ({ism}) has a target outside its field's range "
                    + "after the move, and ARM has no wider form to grow into; the hook is "
                    + "refused rather than truncating the offset.");
        }

        private static KhataTaarib RfudBaid()
        {
            return Rafd(
                "TAARIB-E-6511",
                "قفزةٌ أو نداءٌ نسبيّ في أوّل الدالة صار هدفه أبعد من مدى 32 بت بعد نقله "
                    + "إلى المنطقة الوسيطة؛ رُفض التركيب بدل بتر الإزاحة وتحويل التنفيذ إلى "
                    + "موضعٍ خاطئ.",
                "A relative jump or call in the prologue is now further than a signed "
                    + "32-bit range after being moved into the trampoline; the hook is "
                    + "refused rather than truncating the offset into a wrong-address jump.");
        }

        private static KhataTaarib QillatMakhzan()
        {
            // The trampoline buffer Mihmaz hands in is too small for a widened
            // instruction. This is a sizing bug in Mihmaz, not anything about
            // the target, and it is surfaced as a refusal so it can never
            // silently write one byte past the trampoline into adjacent pages.
            return new KhataTaarib(
                Ramz.QeemaBatila,
                "TAARIB-E-6517",
                "المخزن الوسيط أصغر من التعليمة بعد توسيعها؛ هذا خلل في تحديد الحجم داخل "
                    + "الملحق، رُفض بدل الكتابة خارج الصفحة.",
                "The trampoline buffer is smaller than the widened instruction; this is a "
                    + "sizing bug inside the plugin, refused rather than writing past the page.",
                Khutwa.IblaghLilMalik);
        }

        private static KhataTaarib Rafd(string ramz, string arabi, string injilizi)
        {
            return new KhataTaarib(Ramz.QeemaBatila, ramz, arabi, injilizi, Khutwa.FathTashkhis);
        }

        private static bool YasaI32(long v)
        {
            return v >= int.MinValue && v <= int.MaxValue;
        }

        private static bool YasaRel8(long v)
        {
            return v >= -128 && v <= 127;
        }

        private static bool YasaImaraat(long v, int bits)
        {
            long hadd = 1L << (bits - 1);
            return v >= -hadd && v <= hadd - 1;
        }

        private static long SignExtend(long v, int bits)
        {
            int shift = 64 - bits;
            return (v << shift) >> shift;
        }

        private static long ImmAdr(uint w)
        {
            long immlo = (w >> 29) & 0x3;
            long immhi = (w >> 5) & 0x7FFFF;
            long imm21 = (immhi << 2) | immlo;
            return SignExtend(imm21, 21);
        }

        private static uint DamjAdr(uint w, long imm21)
        {
            uint field = (uint)(imm21 & 0x1FFFFF);
            uint immlo = field & 0x3;
            uint immhi = (field >> 2) & 0x7FFFF;
            uint saf = w & ~((0x3u << 29) | (0x7FFFFu << 5));
            return saf | (immlo << 29) | (immhi << 5);
        }

        private static void IktubI32(Span<byte> hadaf, int at, int qeema)
        {
            hadaf[at] = (byte)qeema;
            hadaf[at + 1] = (byte)(qeema >> 8);
            hadaf[at + 2] = (byte)(qeema >> 16);
            hadaf[at + 3] = (byte)(qeema >> 24);
        }
    }
}
