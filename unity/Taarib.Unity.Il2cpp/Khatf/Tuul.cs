// طول — how many bytes one instruction is, decided without disassembling it.
//
// A detour overwrites the first bytes of a function with a jump. To do that
// without corrupting the target it must copy out a WHOLE number of
// instructions: the last byte it preserves has to be the last byte of an
// instruction, never the middle of one. An instruction cut in half and copied
// into the trampoline is a fragment the CPU will try to execute as if it were
// whole, and the byte after it — the first byte of the jump we wrote — becomes
// an operand of that fragment. There is no crash at the patch site; the crash
// is somewhere inside the trampoline, later, with a call stack that points at
// nothing. So the one question this file answers is exact: given a pointer at
// the start of an instruction, how long is it.
//
// WHY A TABLE AND NOT A DISASSEMBLER. Length is a function of the opcode maps,
// the prefixes, ModRM, SIB, displacement and immediate sizing rules — and of
// almost nothing else. It does not need to know that 0x01 is ADD or what ADD
// does; it needs to know that 0x01 carries a ModRM byte and no immediate. A
// full disassembler (a Zydis, an iced, a Capstone) answers a far larger
// question, drags in a megabyte of tables and a native dependency, and would
// have to be shipped per architecture into a plugin that is already a guest in
// someone else's process. The length decoder is a few hundred bytes of flag
// tables that fit in this file, have no dependency, and can be read in full by
// the person debugging a bad hook. The right tool is the smallest one that
// answers the exact question, and here that is a table.
//
// WHAT IT ALSO DECODES: CLASSIFICATION. Copying a prologue is not enough on its
// own; Naqla then has to relocate it, and relocation needs to know which
// instructions mean something different at a new address. So the decoder does
// not stop at length. For each instruction it also reports whether it is
// RIP-relative (its memory operand is measured from the next instruction),
// whether it is a relative jump, call or Jcc (rel8 or rel32), whether it is a
// return, and whether it is an int3. Everything Naqla needs to rewrite or to
// refuse is decided here, in the one pass that already walked the bytes.
//
// The tables cover the full one-byte map, the two-byte 0F map, and the
// three-byte 0F 38 and 0F 3A maps — enough to decode any real compiler-emitted
// function prologue, including the modern endbr64 (F3 0F 1E FA) and the
// multi-byte NOP padding (0F 1F ...). VEX and EVEX encodings, which never
// appear in a prologue a compiler writes, are refused rather than guessed at:
// a wrong length is a corrupted trampoline, and a clean refusal is a hook that
// simply does not install.
//
// ARM64 is in the same file because the question is the same, but there the
// length is trivial: every A64 instruction is exactly four bytes. That is the
// whole of the length problem on ARM. It is emphatically NOT the whole of the
// relocation problem — a fixed width makes copying out an instruction free and
// makes moving it correctly no easier at all, because a PC-relative ADRP still
// addresses a different page once it lands four kilobytes away. So the ARM path
// here only classifies: is this word PC-relative (ADR, ADRP, B, BL, B.cond,
// CBZ/CBNZ, TBZ/TBNZ, or an LDR-literal), and Naqla does the arithmetic.

using System;

namespace Taarib.Unity.Il2cpp.Khatf
{
    /// <summary>
    /// What an x86-64 instruction is, as far as relocating it is concerned.
    /// </summary>
    /// <remarks>
    /// Only the distinctions Naqla acts on exist here. Two instructions that
    /// relocate identically — an ADD and a MOV that both address memory through
    /// a plain register — are both <see cref="Adi"/>, because inventing a finer
    /// classification than relocation uses would be a second description of the
    /// instruction that could disagree with the first.
    /// </remarks>
    public enum SinfAmr
    {
        /// <summary>
        /// Position-independent: the same bytes mean the same thing at any
        /// address, so relocation copies it verbatim.
        /// </summary>
        Adi = 0,

        /// <summary>
        /// A memory operand measured from the address of the next instruction
        /// (<c>[rip + disp32]</c>). Moving the instruction changes what that
        /// operand reaches, so the displacement has to be recomputed.
        /// </summary>
        NisbatRip = 1,

        /// <summary>
        /// A relative jump, call or conditional branch — <c>rel8</c> or
        /// <c>rel32</c> — whose target is an offset from the next instruction
        /// and so has to be recomputed, and possibly widened, when moved.
        /// </summary>
        QafzaNisbiya = 2,

        /// <summary>A return. Ends a run of instructions; nothing follows it.</summary>
        Rujoo = 3,

        /// <summary>An <c>int3</c> breakpoint, the one-byte 0xCC.</summary>
        Kasr = 4,
    }

    /// <summary>
    /// Which kind of relative branch an instruction is, which is exactly what
    /// Naqla needs to re-encode it at a new address.
    /// </summary>
    /// <remarks>
    /// The distinction that matters for relocation is not "jump versus call"
    /// but "does a wider form exist to widen into". A short jump has a near
    /// form; a <c>LOOP</c> or a <c>JRCXZ</c> does not, so when one of those can
    /// no longer reach its target from the new address there is nothing to
    /// widen it into and the only correct outcome is a refusal —
    /// <see cref="BilaShaklBaid"/> is how the decoder tells Naqla that in
    /// advance.
    /// </remarks>
    public enum NawQafza : byte
    {
        /// <summary>Not a relative branch.</summary>
        LaShay = 0,

        /// <summary>Short unconditional jump, <c>EB rel8</c>; near form is <c>E9</c>.</summary>
        QafzaQasira = 1,

        /// <summary>Near unconditional jump, <c>E9 rel32</c>.</summary>
        QafzaBaida = 2,

        /// <summary>Near call, <c>E8 rel32</c> (there is no <c>rel8</c> call).</summary>
        Nida = 3,

        /// <summary>
        /// Short conditional jump, <c>70+cc rel8</c>; near form is <c>0F 80+cc</c>.
        /// </summary>
        ShartQasir = 4,

        /// <summary>Near conditional jump, <c>0F 80+cc rel32</c>.</summary>
        ShartBaid = 5,

        /// <summary>
        /// A short branch with no near form — <c>LOOP</c>, <c>LOOPE</c>,
        /// <c>LOOPNE</c> (0xE0-0xE2) and <c>JRCXZ</c> (0xE3). Copyable as long
        /// as its target stays in <c>rel8</c> range from the new address, and
        /// an unconditional refusal the moment it does not, because it cannot
        /// be widened.
        /// </summary>
        BilaShaklBaid = 6,
    }

    /// <summary>
    /// One decoded x86-64 instruction: its length, its class, and — when it is
    /// position-dependent — where its displacement or relative field sits and
    /// what value that field currently holds.
    /// </summary>
    public struct Amr
    {
        /// <summary>Total length in bytes, or zero when decoding failed.</summary>
        public int Tul;

        /// <summary>What relocating it requires.</summary>
        public SinfAmr Sinf;

        /// <summary>
        /// For <see cref="SinfAmr.QafzaNisbiya"/>, which branch it is; otherwise
        /// <see cref="NawQafza.LaShay"/>.
        /// </summary>
        public NawQafza Naw;

        /// <summary>
        /// The condition code (0-15) of a <c>Jcc</c>, valid only for
        /// <see cref="NawQafza.ShartQasir"/> and <see cref="NawQafza.ShartBaid"/>.
        /// It is the low nibble of the opcode and is carried so a short
        /// conditional jump can be widened into the matching near one.
        /// </summary>
        public byte Shart;

        /// <summary>
        /// Byte offset, within the instruction, of the position-dependent field
        /// — the <c>rel8</c>/<c>rel32</c> of a branch or the <c>disp32</c> of a
        /// RIP-relative operand. Zero when the instruction has neither.
        /// </summary>
        public int MawqiHagl;

        /// <summary>
        /// Size in bytes of that field: 1 for a <c>rel8</c>, 4 for a
        /// <c>rel32</c> or a RIP <c>disp32</c>. Zero when there is none.
        /// </summary>
        public int HajmHagl;

        /// <summary>
        /// The signed value that field currently encodes — the relative
        /// displacement of a branch, or the RIP displacement. Its meaning is
        /// tied to the address the instruction came from, which is why moving
        /// the instruction obliges Naqla to recompute it.
        /// </summary>
        public long Qeema;

        /// <summary>Whether decoding produced a real instruction.</summary>
        public readonly bool Salih => Tul > 0;
    }

    /// <summary>
    /// What an A64 instruction is, as far as relocating it is concerned.
    /// </summary>
    /// <remarks>
    /// The width is always four bytes, so unlike <see cref="SinfAmr"/> this
    /// carries no length — only the classification, because on ARM the
    /// difficulty is entirely in the second question. Each PC-relative form has
    /// its own immediate encoding, its own range and, for <see cref="Adrp"/>,
    /// its own page alignment, and Naqla treats each separately.
    /// </remarks>
    public enum SinfAmrArm
    {
        /// <summary>Position-independent; copyable verbatim.</summary>
        Adi = 0,

        /// <summary>
        /// <c>ADR</c>: PC plus a signed 21-bit byte offset, reaching +/-1 MB.
        /// </summary>
        Adr = 1,

        /// <summary>
        /// <c>ADRP</c>: (PC and the low 12 bits cleared) plus a signed 21-bit
        /// offset scaled by 4096, reaching +/-4 GB in 4 KB steps.
        /// </summary>
        Adrp = 2,

        /// <summary>
        /// <c>B</c>: an unconditional branch, PC plus a signed 26-bit offset
        /// scaled by 4, reaching +/-128 MB.
        /// </summary>
        Far = 3,

        /// <summary><c>BL</c>: the same range as <see cref="Far"/>, but it links.</summary>
        Nida = 4,

        /// <summary>
        /// <c>B.cond</c>: PC plus a signed 19-bit offset scaled by 4, reaching
        /// +/-1 MB.
        /// </summary>
        Shart = 5,

        /// <summary>
        /// <c>CBZ</c>/<c>CBNZ</c>: compare-and-branch, a signed 19-bit offset
        /// scaled by 4, +/-1 MB.
        /// </summary>
        Muqarana = 6,

        /// <summary>
        /// <c>TBZ</c>/<c>TBNZ</c>: test-bit-and-branch, a signed 14-bit offset
        /// scaled by 4, reaching only +/-32 KB.
        /// </summary>
        Bit = 7,

        /// <summary>
        /// <c>LDR</c>/<c>LDRSW</c>/<c>PRFM</c> literal: loads relative to PC
        /// through a signed 19-bit offset scaled by 4, +/-1 MB.
        /// </summary>
        HimlNisbi = 8,

        /// <summary><c>RET</c>, <c>ERET</c>: ends a run.</summary>
        Rujoo = 9,

        /// <summary><c>BRK</c>: a breakpoint.</summary>
        Kasr = 10,
    }

    /// <summary>
    /// One decoded A64 instruction: always four bytes, its class, and the raw
    /// word so Naqla can extract whichever immediate the class implies.
    /// </summary>
    public struct AmrArm
    {
        /// <summary>The instruction word, little-endian as stored in memory.</summary>
        public uint Kalima;

        /// <summary>What relocating it requires.</summary>
        public SinfAmrArm Sinf;

        /// <summary>Always four on ARM; a fixed-width constant kept for symmetry.</summary>
        public readonly int Tul => 4;
    }

    /// <summary>
    /// The length decoder: one instruction in, its length and classification
    /// out, table-driven and dependency-free.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Stateless and allocation-free. It is called during hook installation,
    /// never on a frame path, but it holds no state regardless so two threads
    /// installing two hooks cannot interfere.
    /// </para>
    /// <para>
    /// It decodes 64-bit mode only, which is the mode GameAssembly.dll runs in
    /// on every desktop target this plugin supports. Legacy 16- and 32-bit
    /// forms differ in exactly the places 64-bit changed — the REX prefix, the
    /// meaning of <c>mod=00 rm=101</c> as RIP-relative rather than a bare
    /// <c>disp32</c>, the default operand size — and decoding one mode's bytes
    /// under the other's rules is how a length decoder silently returns a wrong
    /// answer. The mode is fixed and named rather than inferred.
    /// </para>
    /// </remarks>
    public static class Tuul
    {
        [Flags]
        private enum Sifat
        {
            Bila = 0,
            ModRM = 1 << 0,
            Imm8 = 1 << 1,
            ImmZ = 1 << 2,
            Imm16 = 1 << 3,
            ImmV = 1 << 4,
            Rel8 = 1 << 5,
            Rel32 = 1 << 6,
            Moffs = 1 << 7,
            ImmEnter = 1 << 8,
            Grp3 = 1 << 9,
            Rujoo = 1 << 10,
            Kasr = 1 << 11,
            Batil = 1 << 12,
        }

        private static readonly Sifat[] Map1 = BinaMap1();
        private static readonly Sifat[] Map0F = BinaMap0F();
        private static readonly Sifat[] Map0F38 = BinaMap0F38();
        private static readonly Sifat[] Map0F3A = BinaMap0F3A();

        /// <summary>The most prologue bytes worth reading to decode past a patch.</summary>
        /// <remarks>
        /// Fifteen is the architectural maximum length of a single x86-64
        /// instruction, so no correctly formed instruction can need more context
        /// than this to length-decode. A decoder handed fewer bytes than an
        /// instruction turns out to occupy reports failure rather than guessing,
        /// which is the caller's cue to read more.
        /// </remarks>
        public const int AqsaTulAmr = 15;

        /// <summary>
        /// Decodes one x86-64 instruction at the start of <paramref name="bayt"/>.
        /// </summary>
        /// <param name="bayt">
        /// Bytes beginning at an instruction boundary. Should hold at least the
        /// whole instruction; a buffer that ends inside the instruction is a
        /// decode failure, not a truncated success.
        /// </param>
        /// <param name="amr">
        /// The decoded instruction; <see cref="Amr.Tul"/> is zero on failure.
        /// </param>
        /// <returns>
        /// <c>false</c> when the bytes are not a decodable instruction — an
        /// encoding this decoder refuses (VEX/EVEX), an invalid opcode, or a
        /// buffer too short to contain what the opcode implies. A refusal here
        /// is deliberate: a hook that cannot decode its target's prologue does
        /// not install, rather than installing over an instruction whose length
        /// it guessed.
        /// </returns>
        public static bool Iqra(ReadOnlySpan<byte> bayt, out Amr amr)
        {
            amr = default;
            int n = bayt.Length;
            if (n == 0)
            {
                return false;
            }

            int i = 0;
            bool osz = false;
            bool asz = false;

            // Legacy prefixes may repeat and appear in any order; consume every
            // one, remembering only the two that change field sizes (0x66
            // operand-size, 0x67 address-size). REX must be the last prefix, so
            // it is read after this loop and not inside it.
            while (i < n)
            {
                byte b = bayt[i];
                bool huwaBadi =
                    b == 0xF0 || b == 0xF2 || b == 0xF3 ||
                    b == 0x2E || b == 0x36 || b == 0x3E || b == 0x26 ||
                    b == 0x64 || b == 0x65 || b == 0x66 || b == 0x67;
                if (!huwaBadi)
                {
                    break;
                }
                if (b == 0x66)
                {
                    osz = true;
                }
                else if (b == 0x67)
                {
                    asz = true;
                }
                i++;
            }

            bool rexW = false;
            if (i < n && bayt[i] >= 0x40 && bayt[i] <= 0x4F)
            {
                rexW = (bayt[i] & 0x08) != 0;
                i++;
            }

            if (i >= n)
            {
                return false;
            }

            byte op = bayt[i];
            i++;
            Sifat flags;
            bool fi0F = false;

            if (op == 0x0F)
            {
                if (i >= n)
                {
                    return false;
                }
                byte b2 = bayt[i];
                i++;
                if (b2 == 0x38)
                {
                    if (i >= n)
                    {
                        return false;
                    }
                    op = bayt[i];
                    i++;
                    flags = Map0F38[op];
                }
                else if (b2 == 0x3A)
                {
                    if (i >= n)
                    {
                        return false;
                    }
                    op = bayt[i];
                    i++;
                    flags = Map0F3A[op];
                }
                else
                {
                    op = b2;
                    flags = Map0F[op];
                    fi0F = true;
                }
            }
            else
            {
                flags = Map1[op];
            }

            if ((flags & Sifat.Batil) != 0)
            {
                return false;
            }

            int reg = 0;
            bool nisbatRip = false;
            int mawqiRip = 0;

            if ((flags & Sifat.ModRM) != 0)
            {
                if (i >= n)
                {
                    return false;
                }
                byte modrm = bayt[i];
                i++;
                int mod = modrm >> 6;
                reg = (modrm >> 3) & 7;
                int rm = modrm & 7;
                int dispSize = 0;

                if (mod != 3)
                {
                    if (rm == 4)
                    {
                        if (i >= n)
                        {
                            return false;
                        }
                        int baseReg = bayt[i] & 7;
                        i++;
                        if (mod == 0 && baseReg == 5)
                        {
                            dispSize = 4;
                        }
                        else if (mod == 1)
                        {
                            dispSize = 1;
                        }
                        else if (mod == 2)
                        {
                            dispSize = 4;
                        }
                    }
                    else if (mod == 0 && rm == 5)
                    {
                        // The one 64-bit-specific addressing form: this is not a
                        // bare disp32, it is [rip + disp32], measured from the
                        // next instruction. Record where the disp lives so Naqla
                        // can recompute it against the trampoline's address.
                        dispSize = 4;
                        nisbatRip = true;
                        mawqiRip = i;
                    }
                    else if (mod == 1)
                    {
                        dispSize = 1;
                    }
                    else if (mod == 2)
                    {
                        dispSize = 4;
                    }
                }

                i += dispSize;
                if (i > n)
                {
                    return false;
                }
            }

            int immBytes = 0;
            if ((flags & Sifat.Grp3) != 0 && (reg == 0 || reg == 1))
            {
                // Group 3 (F6/F7): only /0 and /1 (TEST) carry an immediate;
                // NOT, NEG, MUL, IMUL, DIV, IDIV do not. Sizing it off the reg
                // field is the whole reason this group needs a special flag.
                immBytes += op == 0xF6 ? 1 : (osz ? 2 : 4);
            }
            if ((flags & Sifat.Imm8) != 0)
            {
                immBytes += 1;
            }
            if ((flags & Sifat.Imm16) != 0)
            {
                immBytes += 2;
            }
            if ((flags & Sifat.ImmZ) != 0)
            {
                immBytes += osz ? 2 : 4;
            }
            if ((flags & Sifat.ImmV) != 0)
            {
                // MOV r64, imm64: the one immediate that can be eight bytes, and
                // only when REX.W is set. Without it the operand-size prefix
                // still chooses between two and four.
                immBytes += rexW ? 8 : (osz ? 2 : 4);
            }
            if ((flags & Sifat.ImmEnter) != 0)
            {
                immBytes += 3;
            }
            if ((flags & Sifat.Moffs) != 0)
            {
                immBytes += asz ? 4 : 8;
            }

            int relPos = 0;
            int relSize = 0;
            if ((flags & Sifat.Rel8) != 0)
            {
                relPos = i;
                relSize = 1;
            }
            else if ((flags & Sifat.Rel32) != 0)
            {
                relPos = i;
                relSize = 4;
            }

            int end = i + immBytes + relSize;
            if (end > n)
            {
                return false;
            }

            amr.Tul = end;

            if ((flags & Sifat.Rujoo) != 0)
            {
                amr.Sinf = SinfAmr.Rujoo;
            }
            else if ((flags & Sifat.Kasr) != 0)
            {
                amr.Sinf = SinfAmr.Kasr;
            }
            else if (relSize != 0)
            {
                amr.Sinf = SinfAmr.QafzaNisbiya;
                amr.MawqiHagl = relPos;
                amr.HajmHagl = relSize;
                amr.Qeema = relSize == 1
                    ? (sbyte)bayt[relPos]
                    : ReadI32(bayt, relPos);
                amr.Naw = NawFor(op, fi0F, relSize);
                if (amr.Naw == NawQafza.ShartQasir || amr.Naw == NawQafza.ShartBaid)
                {
                    amr.Shart = (byte)(op & 0x0F);
                }
            }
            else if (nisbatRip)
            {
                amr.Sinf = SinfAmr.NisbatRip;
                amr.MawqiHagl = mawqiRip;
                amr.HajmHagl = 4;
                amr.Qeema = ReadI32(bayt, mawqiRip);
            }
            else
            {
                amr.Sinf = SinfAmr.Adi;
            }

            return true;
        }

        /// <summary>
        /// Decodes one A64 instruction at the start of <paramref name="bayt"/>.
        /// </summary>
        /// <param name="bayt">At least four bytes at an instruction boundary.</param>
        /// <param name="amr">The decoded instruction.</param>
        /// <returns>
        /// <c>false</c> only when fewer than four bytes are available. Every
        /// four-byte word is a valid A64 length; classification never fails,
        /// because an unrecognised encoding is simply position-independent and
        /// copied verbatim.
        /// </returns>
        public static bool IqraArm(ReadOnlySpan<byte> bayt, out AmrArm amr)
        {
            amr = default;
            if (bayt.Length < 4)
            {
                return false;
            }
            uint w = (uint)(bayt[0] | (bayt[1] << 8) | (bayt[2] << 16) | (bayt[3] << 24));
            amr.Kalima = w;
            amr.Sinf = SanfArm(w);
            return true;
        }

        /// <summary>Classifies one A64 word by its fixed bit patterns.</summary>
        private static SinfAmrArm SanfArm(uint w)
        {
            // PC-relative addressing: ADR / ADRP share the 0b1_0000 op field.
            if ((w & 0x1F000000u) == 0x10000000u)
            {
                return (w & 0x80000000u) != 0 ? SinfAmrArm.Adrp : SinfAmrArm.Adr;
            }
            // Unconditional branch immediate: B (0x14000000) and BL (0x94000000).
            if ((w & 0x7C000000u) == 0x14000000u)
            {
                return (w & 0x80000000u) != 0 ? SinfAmrArm.Nida : SinfAmrArm.Far;
            }
            // Conditional branch immediate: B.cond, 0101010 0 ... with bit4 == 0.
            if ((w & 0xFF000010u) == 0x54000000u)
            {
                return SinfAmrArm.Shart;
            }
            // Compare and branch: CBZ/CBNZ, sf 011010 x.
            if ((w & 0x7E000000u) == 0x34000000u)
            {
                return SinfAmrArm.Muqarana;
            }
            // Test and branch: TBZ/TBNZ, b5 011011 x.
            if ((w & 0x7E000000u) == 0x36000000u)
            {
                return SinfAmrArm.Bit;
            }
            // Load register literal: LDR/LDRSW/PRFM (and the SIMD forms).
            if ((w & 0x3B000000u) == 0x18000000u)
            {
                return SinfAmrArm.HimlNisbi;
            }
            // RET / ERET / DRPS: unconditional branch register, opc 0b0010/0100/0101.
            if ((w & 0xFFFFFC1Fu) == 0xD65F0000u)
            {
                return SinfAmrArm.Rujoo;
            }
            // BRK: exception generation, immediate in bits 20..5.
            if ((w & 0xFFE0001Fu) == 0xD4200000u)
            {
                return SinfAmrArm.Kasr;
            }
            return SinfAmrArm.Adi;
        }

        private static NawQafza NawFor(byte op, bool fi0F, int relSize)
        {
            if (fi0F)
            {
                // The only rel-bearing two-byte opcodes are the near Jcc block
                // 0F 80..0F 8F.
                return NawQafza.ShartBaid;
            }
            if (relSize == 1)
            {
                if (op >= 0x70 && op <= 0x7F)
                {
                    return NawQafza.ShartQasir;
                }
                if (op >= 0xE0 && op <= 0xE3)
                {
                    return NawQafza.BilaShaklBaid;
                }
                return NawQafza.QafzaQasira; // 0xEB
            }
            if (op == 0xE8)
            {
                return NawQafza.Nida;
            }
            return NawQafza.QafzaBaida; // 0xE9
        }

        private static int ReadI32(ReadOnlySpan<byte> bayt, int at)
        {
            return bayt[at]
                | (bayt[at + 1] << 8)
                | (bayt[at + 2] << 16)
                | (bayt[at + 3] << 24);
        }

        private static Sifat[] BinaMap1()
        {
            Sifat[] m = new Sifat[256];

            // The eight arithmetic/logic groups (ADD..CMP) at 0x00, 0x08, ...
            // each lay out /r, /r, /r, /r, AL+imm8, eAX+immz across six opcodes.
            int[] usus = { 0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38 };
            foreach (int b in usus)
            {
                m[b + 0] = Sifat.ModRM;
                m[b + 1] = Sifat.ModRM;
                m[b + 2] = Sifat.ModRM;
                m[b + 3] = Sifat.ModRM;
                m[b + 4] = Sifat.Imm8;
                m[b + 5] = Sifat.ImmZ;
            }

            // One-byte forms that do not exist in 64-bit mode: their opcode
            // bytes were repurposed or removed, so meeting one means the bytes
            // are not what we think they are. Refuse rather than assign a length.
            int[] batil =
            {
                0x06, 0x07, 0x0E, 0x16, 0x17, 0x1E, 0x1F, 0x27, 0x2F, 0x37, 0x3F,
                0x60, 0x61, 0x62, 0x82, 0x9A, 0xC4, 0xC5, 0xCE, 0xD4, 0xD5, 0xD6, 0xEA,
            };
            foreach (int b in batil)
            {
                m[b] = Sifat.Batil;
            }

            m[0x63] = Sifat.ModRM;             // MOVSXD
            m[0x68] = Sifat.ImmZ;              // PUSH immz
            m[0x69] = Sifat.ModRM | Sifat.ImmZ; // IMUL r, r/m, immz
            m[0x6A] = Sifat.Imm8;             // PUSH imm8
            m[0x6B] = Sifat.ModRM | Sifat.Imm8; // IMUL r, r/m, imm8

            for (int b = 0x70; b <= 0x7F; b++)
            {
                m[b] = Sifat.Rel8;            // Jcc rel8
            }

            m[0x80] = Sifat.ModRM | Sifat.Imm8;
            m[0x81] = Sifat.ModRM | Sifat.ImmZ;
            m[0x83] = Sifat.ModRM | Sifat.Imm8;
            m[0x84] = Sifat.ModRM;
            m[0x85] = Sifat.ModRM;
            m[0x86] = Sifat.ModRM;
            m[0x87] = Sifat.ModRM;
            m[0x88] = Sifat.ModRM;
            m[0x89] = Sifat.ModRM;
            m[0x8A] = Sifat.ModRM;
            m[0x8B] = Sifat.ModRM;
            m[0x8C] = Sifat.ModRM;
            m[0x8D] = Sifat.ModRM;            // LEA
            m[0x8E] = Sifat.ModRM;
            m[0x8F] = Sifat.ModRM;            // POP r/m

            m[0xA0] = Sifat.Moffs;
            m[0xA1] = Sifat.Moffs;
            m[0xA2] = Sifat.Moffs;
            m[0xA3] = Sifat.Moffs;
            m[0xA8] = Sifat.Imm8;             // TEST AL, imm8
            m[0xA9] = Sifat.ImmZ;             // TEST eAX, immz

            for (int b = 0xB0; b <= 0xB7; b++)
            {
                m[b] = Sifat.Imm8;           // MOV r8, imm8
            }
            for (int b = 0xB8; b <= 0xBF; b++)
            {
                m[b] = Sifat.ImmV;           // MOV r32/r64, imm (imm64 with REX.W)
            }

            m[0xC0] = Sifat.ModRM | Sifat.Imm8;
            m[0xC1] = Sifat.ModRM | Sifat.Imm8;
            m[0xC2] = Sifat.Imm16 | Sifat.Rujoo; // RET imm16
            m[0xC3] = Sifat.Rujoo;               // RET
            m[0xC6] = Sifat.ModRM | Sifat.Imm8;  // MOV r/m8, imm8 (group 11)
            m[0xC7] = Sifat.ModRM | Sifat.ImmZ;  // MOV r/m, immz (group 11)
            m[0xC8] = Sifat.ImmEnter;            // ENTER imm16, imm8
            m[0xCA] = Sifat.Imm16 | Sifat.Rujoo; // RETF imm16
            m[0xCB] = Sifat.Rujoo;               // RETF
            m[0xCC] = Sifat.Kasr;                // INT3
            m[0xCD] = Sifat.Imm8;                // INT imm8

            m[0xD0] = Sifat.ModRM;
            m[0xD1] = Sifat.ModRM;
            m[0xD2] = Sifat.ModRM;
            m[0xD3] = Sifat.ModRM;
            for (int b = 0xD8; b <= 0xDF; b++)
            {
                m[b] = Sifat.ModRM;          // x87 escapes carry a ModRM, no imm
            }

            for (int b = 0xE0; b <= 0xE3; b++)
            {
                m[b] = Sifat.Rel8;           // LOOP/LOOPE/LOOPNE/JRCXZ rel8
            }
            m[0xE4] = Sifat.Imm8;
            m[0xE5] = Sifat.Imm8;
            m[0xE6] = Sifat.Imm8;
            m[0xE7] = Sifat.Imm8;
            m[0xE8] = Sifat.Rel32;           // CALL rel32
            m[0xE9] = Sifat.Rel32;           // JMP rel32
            m[0xEB] = Sifat.Rel8;            // JMP rel8

            m[0xF6] = Sifat.ModRM | Sifat.Grp3;
            m[0xF7] = Sifat.ModRM | Sifat.Grp3;
            m[0xFE] = Sifat.ModRM;           // INC/DEC r/m (group 4)
            m[0xFF] = Sifat.ModRM;           // group 5: INC/DEC/CALL/JMP/PUSH r/m

            return m;
        }

        private static Sifat[] BinaMap0F()
        {
            Sifat[] m = new Sifat[256];

            // The great majority of two-byte opcodes carry a ModRM and no
            // immediate — every SSE move and arithmetic form, CMOVcc, SETcc,
            // MOVZX/MOVSX, the bit instructions, group 15, and the multi-byte
            // NOP 0F 1F and endbr 0F 1E. Start from that and carve out the
            // exceptions.
            for (int b = 0; b < 256; b++)
            {
                m[b] = Sifat.ModRM;
            }

            int[] bila =
            {
                0x05, 0x06, 0x07, 0x08, 0x09, 0x0B, 0x0E,
                0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x37,
                0x77, 0xA0, 0xA1, 0xA2, 0xA8, 0xA9, 0xAA,
                0xC8, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE, 0xCF,
            };
            foreach (int b in bila)
            {
                m[b] = Sifat.Bila;           // SYSCALL, UD2, RDTSC, EMMS, CPUID, BSWAP...
            }

            for (int b = 0x80; b <= 0x8F; b++)
            {
                m[b] = Sifat.Rel32;          // Jcc rel32 (near conditional)
            }

            // Two-byte forms that additionally carry an imm8.
            int[] maImm8 =
            {
                0x0F, 0x70, 0x71, 0x72, 0x73, 0xA4, 0xAC, 0xBA,
                0xC2, 0xC4, 0xC5, 0xC6,
            };
            foreach (int b in maImm8)
            {
                m[b] = Sifat.ModRM | Sifat.Imm8;
            }

            return m;
        }

        private static Sifat[] BinaMap0F38()
        {
            // Every three-byte 0F 38 opcode is a ModRM form with no immediate.
            Sifat[] m = new Sifat[256];
            for (int b = 0; b < 256; b++)
            {
                m[b] = Sifat.ModRM;
            }
            return m;
        }

        private static Sifat[] BinaMap0F3A()
        {
            // Every three-byte 0F 3A opcode is a ModRM form with an imm8.
            Sifat[] m = new Sifat[256];
            for (int b = 0; b < 256; b++)
            {
                m[b] = Sifat.ModRM | Sifat.Imm8;
            }
            return m;
        }
    }
}
