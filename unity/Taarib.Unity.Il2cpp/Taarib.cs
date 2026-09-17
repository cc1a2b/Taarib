// تعريب — the plugin root for Unity games on the IL2CPP scripting backend.
//
// The same product as the Mono adapter, and deliberately almost the same file.
// Everything that decides how Arabic is drawn lives in Taarib.Unity.Mushtarak
// and is compiled into both plugins unchanged: the patch reader, the mesh
// builder, the markup bridge, atlas residency, capture. What is different here
// is only how a method is found and how it is hooked, and that difference is
// confined to hall/ and khatf/.
//
// THE RULE THIS FILE ENFORCES, AS IN THE MONO ROOT: nothing thrown here reaches
// BepInEx. A plugin that throws out of Load can stop a game from starting, and a
// player who installed a translation and got a game that will not launch has been
// harmed far more than they were helped. Every step is guarded, every failure is
// logged in both languages with its permanent code, and any failure at all
// leaves the game running in its original language.
//
// WHY THE ENGINE VERSION IS READ FROM DISK AND NOT FROM Application.unityVersion.
// This is the one piece of ordering that is genuinely IL2CPP's and is easy to get
// backwards. The signature database is keyed by engine version. The signature
// database exists for games whose global-metadata.dat cannot be read. But
// UnityEngine.Application.unityVersion is itself reached through the generated
// metadata — so on exactly the games where the version matters most, asking the
// engine for it fails. The version therefore comes from the file system, before
// anything managed is touched, and only falls back to the engine when the file
// system did not answer. Kashf owns that, and the ordering is the whole reason
// it exists.
//
// WHAT IS NOT HERE. There is no C++ shim. The roadmap allows one where a raw
// vtable dispatch or a naked thunk is unavoidable; on .NET 6 it is avoidable,
// because [UnmanagedCallersOnly] produces a real native entry point with a known
// calling convention and khatf's trampoline is written as bytes rather than as
// hand-written assembly. A shim would have been a second build system and a
// second artifact to sign for no capability. If a target ever needs one, this is
// the comment that should be revisited.

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using BepInEx;
using BepInEx.Configuration;
using BepInEx.Logging;
using BepInEx.Unity.IL2CPP;
using HarmonyLib;
using Taarib.Unity.Il2cpp.Anzimat;
using Taarib.Unity.Il2cpp.Basmat;
using Taarib.Unity.Il2cpp.Hall;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Il2cpp
{
    /// <summary>What state the takeover reached.</summary>
    public enum HalatTaarib
    {
        /// <summary>Nothing has run yet.</summary>
        Bila = 0,

        /// <summary>The player turned the patch off in configuration.</summary>
        Muattal = 1,

        /// <summary>Everything came up and text is being replaced.</summary>
        Amil = 2,

        /// <summary>Everything came up and text is being recorded, not replaced.</summary>
        Iltiqat = 3,

        /// <summary>
        /// Something failed. The game is running in its original language and
        /// the log names the step and the reason.
        /// </summary>
        Faashil = 4,
    }

    /// <summary>
    /// The contract every IL2CPP text-system takeover satisfies.
    /// </summary>
    /// <remarks>
    /// Deliberately the same three questions the Mono root asks, with one added
    /// argument: the resolver. Under Mono a takeover finds its own members by
    /// reflection because reflection is available and cheap; under IL2CPP every
    /// address comes from the ladder, so the ladder is passed in rather than
    /// reached for. That also means every resolution a takeover performs is
    /// recorded in one place and reaches the diagnostics bundle, which is the
    /// property the ladder exists to provide.
    /// </remarks>
    public interface INizamIl2cpp
    {
        /// <summary>The system's name, for the log and the diagnostics screen.</summary>
        string Ism { get; }

        /// <summary>
        /// Whether this system is present in this game at all.
        /// </summary>
        /// <param name="sullam">The resolver, for a cheap probe of one known type.</param>
        /// <returns>Whether the game uses this system.</returns>
        /// <remarks>
        /// Must not throw and must not hook anything. Probing through the
        /// resolver rather than through a type lookup is what makes the answer
        /// correct on a game whose metadata cannot be read: a system that is
        /// present but invisible to reflection is still present.
        /// </remarks>
        bool Mawjud(Sullam sullam);

        /// <summary>Installs the takeover.</summary>
        /// <param name="mawarid">Everything shared.</param>
        /// <param name="tarqee">The Harmony instance, for targets a managed hook can reach.</param>
        /// <exception cref="KhataTaarib">
        /// A target could not be resolved on any rung. The root catches it,
        /// disables this system alone, and leaves the others running.
        /// </exception>
        void Rakkib(MawaridIl2cpp mawarid, Harmony tarqee);

        /// <summary>Removes the takeover and restores whatever it changed.</summary>
        /// <remarks>
        /// Must not throw. Called during teardown, possibly after a partial
        /// installation. A native detour that is not removed here outlives the
        /// plugin and points at freed code, which is a crash on the next call.
        /// </remarks>
        void Fukk();
    }

    /// <summary>
    /// Everything an IL2CPP takeover needs, built once by the plugin root.
    /// </summary>
    public sealed class MawaridIl2cpp : IDisposable
    {
        /// <summary>Gathers the shared resources.</summary>
        /// <param name="ruqaa">The mapped patch.</param>
        /// <param name="siyaq">The native layout context.</param>
        /// <param name="silsila">The font chain.</param>
        /// <param name="lawha">The atlas and its residency bookkeeping.</param>
        /// <param name="sullam">The resolution ladder.</param>
        /// <param name="jalsa">The capture session, or null when not capturing.</param>
        /// <param name="sijill">Where a takeover writes its log lines.</param>
        /// <exception cref="ArgumentNullException">Any required argument is null.</exception>
        public MawaridIl2cpp(
            Ruqaa ruqaa,
            MaqbadSiyaq siyaq,
            MaqbadSilsila silsila,
            Lawha lawha,
            Sullam sullam,
            JalsatIltiqat? jalsa,
            ManualLogSource sijill)
        {
            Ruqaa = ruqaa ?? throw new ArgumentNullException(nameof(ruqaa));
            Siyaq = siyaq ?? throw new ArgumentNullException(nameof(siyaq));
            Silsila = silsila ?? throw new ArgumentNullException(nameof(silsila));
            Lawha = lawha ?? throw new ArgumentNullException(nameof(lawha));
            Sullam = sullam ?? throw new ArgumentNullException(nameof(sullam));
            Jalsa = jalsa;
            Sijill = sijill ?? throw new ArgumentNullException(nameof(sijill));
            Takhtit = new Takhtit();
        }

        /// <summary>The mapped patch.</summary>
        public Ruqaa Ruqaa { get; }

        /// <summary>The native layout context, for text the compiler never saw.</summary>
        public MaqbadSiyaq Siyaq { get; }

        /// <summary>The font chain.</summary>
        public MaqbadSilsila Silsila { get; }

        /// <summary>The atlas and its residency bookkeeping.</summary>
        public Lawha Lawha { get; }

        /// <summary>The resolution ladder. Every address a takeover uses comes from here.</summary>
        public Sullam Sullam { get; }

        /// <summary>
        /// The capture session, or <c>null</c> when replacing rather than
        /// recording. Non-null puts every takeover into observe-only posture.
        /// </summary>
        public JalsatIltiqat? Jalsa { get; }

        /// <summary>Whether the plugin is recording rather than replacing.</summary>
        public bool Yaltaqit => Jalsa is not null;

        /// <summary>Where a takeover writes its log lines.</summary>
        public ManualLogSource Sijill { get; }

        /// <summary>
        /// The scratch layout every takeover lays runtime text into.
        /// </summary>
        /// <remarks>
        /// One instance, shared, safe because every takeover point runs on
        /// Unity's main thread one string at a time and the result is consumed
        /// into a mesh before the next call begins.
        /// </remarks>
        public Takhtit Takhtit { get; }

        /// <summary>Releases nothing it does not own.</summary>
        /// <remarks>
        /// Every handle here belongs to the plugin root, which disposes them in
        /// the reverse order it built them. This type exists to carry them, not
        /// to own them, and a Dispose that closed them would close them out from
        /// under a second takeover that had not finished unloading.
        /// </remarks>
        public void Dispose()
        {
        }
    }

    /// <summary>
    /// The BepInEx plugin: the IL2CPP half of the Unity takeover.
    /// </summary>
    [BepInPlugin(Muarrif, IsmMaruud, Isdar)]
    public sealed class MulhaqTaarib : BasePlugin
    {
        /// <summary>The plugin's GUID.</summary>
        public const string Muarrif = "com.cc1a2b.taarib.unity.il2cpp";

        /// <summary>The plugin's display name.</summary>
        public const string IsmMaruud = "Taarib";

        /// <summary>The plugin's version.</summary>
        public const string Isdar = "1.0.1";

        /// <summary>The folder, beside the plugin, that the installer writes into.</summary>
        public const string DalilTaarib = "Taarib";

        /// <summary>The subfolder holding the font files the patch names.</summary>
        public const string DalilKhutut = "khutut";

        /// <summary>The file name of the signature database.</summary>
        public const string MalafBasmat = "basmat.json";

        /// <summary>The extension of a patch container.</summary>
        public const string ImtidadRuqaa = ".ruqaa";

        /// <summary>The name of the capture file a recording session writes.</summary>
        public const string MalafIltiqat = "iltiqat.jsonl";

        private readonly List<INizamIl2cpp> anzima = new List<INizamIl2cpp>();

        // Assigned by IqraIdadat, the first thing Load does and the only thing
        // that runs before anything reads them.
        private ConfigEntry<bool> mufaal = null!;
        private ConfigEntry<bool> yaltaqit = null!;
        private ConfigEntry<bool> tashkhis = null!;
        private ConfigEntry<int> budLawha = null!;
        private ConfigEntry<int> mizaniyatLawha = null!;

        // Genuinely absent until each step succeeds, and absent again after
        // teardown. Nullable, and null-checked everywhere.
        private Harmony? tarqee;
        private Ruqaa? ruqaa;
        private MaqbadSiyaq? siyaq;
        private MaqbadSilsila? silsila;
        private MaqbadKhatt[]? khutut;
        private Lawha? lawha;
        private JalsatIltiqat? jalsa;
        private MawaridIl2cpp? mawarid;
        private Sullam? sullam;
        private string dalil = string.Empty;

        /// <summary>What state the takeover reached.</summary>
        public HalatTaarib Halat { get; private set; } = HalatTaarib.Bila;

        /// <summary>
        /// The one sentence describing why the takeover is not running, or an
        /// empty string when it is.
        /// </summary>
        public string SababAlFashal { get; private set; } = string.Empty;

        /// <summary>Brings the takeover up. Never throws.</summary>
        public override void Load()
        {
            try
            {
                IqraIdadat();
                if (!mufaal.Value)
                {
                    Halat = HalatTaarib.Muattal;
                    Log.LogInfo("تعريب معطَّل في الإعدادات. | Taarib is disabled in configuration.");
                    return;
                }

                dalil = DalilAlMulhaq();
                Hammil();

                Ruqaa maftuha = IftahRuqaa();
                ruqaa = maftuha;
                MaqbadSiyaq siyaqJadid = InshaSiyaq();
                siyaq = siyaqJadid;
                MaqbadSilsila silsilaJadida = IbniSilsila(siyaqJadid, maftuha);
                silsila = silsilaJadida;
                Lawha lawhaJadida = IbniLawha(siyaqJadid, maftuha);
                lawha = lawhaJadida;
                jalsa = IbdaIltiqat();

                Sullam sullamJadid = IbniSullam();
                sullam = sullamJadid;

                MawaridIl2cpp mushtaraka = new MawaridIl2cpp(
                    maftuha, siyaqJadid, silsilaJadida, lawhaJadida, sullamJadid, jalsa, Log);
                mawarid = mushtaraka;

                Harmony harmuni = new Harmony(Muarrif);
                tarqee = harmuni;
                RakkibAnzima(mushtaraka, harmuni);

                Halat = mushtaraka.Yaltaqit ? HalatTaarib.Iltiqat : HalatTaarib.Amil;
                BallighAnAlSullam(sullamJadid);
            }
            catch (KhataTaarib khata)
            {
                Ista(khata.Arabi, khata.Injilizi, khata.RamzDaim, khata);
            }
            catch (Exception khata)
            {
                Ista(
                    "تعذّر تشغيل تعريب، وتُعرض اللعبة بلغتها الأصلية.",
                    "Taarib could not start; the game is shown in its original language.",
                    string.Empty,
                    khata);
            }
        }

        /// <summary>Tears the takeover down. Never throws.</summary>
        /// <returns>Whether BepInEx may unload the plugin.</returns>
        public override bool Unload()
        {
            for (int i = anzima.Count - 1; i >= 0; i--)
            {
                try
                {
                    anzima[i].Fukk();
                }
                catch (Exception khata)
                {
                    Log.LogError(
                        $"تعذّر فك نظام {anzima[i].Ism}. | Text system {anzima[i].Ism} could "
                        + $"not be detached. {khata.Message}");
                }
            }
            anzima.Clear();

            Jarrib(() => tarqee?.UnpatchSelf(), "Harmony");
            Jarrib(() => mawarid?.Dispose(), "mawarid");
            Jarrib(NahiIltiqat, "iltiqat");
            Jarrib(() => lawha?.Dispose(), "lawha");
            Jarrib(() => silsila?.Dispose(), "silsila");
            Jarrib(
                () =>
                {
                    if (khutut is null)
                    {
                        return;
                    }
                    for (int i = 0; i < khutut.Length; i++)
                    {
                        khutut[i]?.Dispose();
                    }
                },
                "khutut");
            Jarrib(() => siyaq?.Dispose(), "siyaq");
            Jarrib(() => ruqaa?.Dispose(), "ruqaa");

            tarqee = null;
            mawarid = null;
            sullam = null;
            lawha = null;
            silsila = null;
            khutut = null;
            siyaq = null;
            ruqaa = null;
            Halat = HalatTaarib.Bila;
            return base.Unload();
        }

        private void IqraIdadat()
        {
            mufaal = Config.Bind(
                "عام",
                "mufaal",
                true,
                "Whether Taarib replaces text at all. Turn this off to play in the game's "
                + "original language without uninstalling the patch.");

            yaltaqit = Config.Bind(
                "عام",
                "iltiqat",
                false,
                "Capture mode: record every string that reaches a takeover point, with the "
                + "rectangle the game drew it in, and replace nothing. For translators "
                + "building a patch.");

            tashkhis = Config.Bind(
                "عام",
                "tashkhis",
                false,
                "Log the full resolution report at startup: which rung of the ladder found "
                + "each method, and why the others declined. Verbose, and the first thing to "
                + "turn on when a game renders no Arabic.");

            budLawha = Config.Bind(
                "لوحة",
                "bud",
                2048,
                "The atlas page size in texels, per side. 4096 is the maximum.");

            mizaniyatLawha = Config.Bind(
                "لوحة",
                "mizaniya",
                64,
                "The atlas budget in mebibytes. When it is reached the atlas evicts glyphs "
                + "it has not drawn recently rather than growing inside the game's memory.");
        }

        private string DalilAlMulhaq()
        {
            string qaida = Paths.PluginPath;
            string maa = Path.Combine(qaida, DalilTaarib);
            return Directory.Exists(maa) ? maa : qaida;
        }

        private void Hammil()
        {
            Muhammil.Tahmeel(dalil);
            Muhammil.Taakkad();
            Log.LogInfo(
                $"jisr {Muhammil.IsdarNass()} حُمِّلت من {Muhammil.MasarMaktaba}. | jisr "
                + $"{Muhammil.IsdarNass()} loaded from {Muhammil.MasarMaktaba}.");
        }

        private Ruqaa IftahRuqaa()
        {
            string? masar = JidRuqaa();
            if (masar is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6601",
                    $"لا توجد رقعة تعريب في {dalil}؛ لم يكتمل التثبيت.",
                    $"There is no Taarib patch in {dalil}; the installation did not complete.",
                    Khutwa.IadatTarkibIttar);
            }

            Ruqaa maftuha = Ruqaa.Iftah(masar);
            Log.LogInfo(
                $"فُتحت الرقعة {Path.GetFileName(masar)}: {maftuha.Nusus.Length} نص، "
                + $"{maftuha.Takhtitat.Length} تخطيط. | Opened {Path.GetFileName(masar)}: "
                + $"{maftuha.Nusus.Length} strings, {maftuha.Takhtitat.Length} layouts.");
            return maftuha;
        }

        private string? JidRuqaa()
        {
            if (!Directory.Exists(dalil))
            {
                return null;
            }
            string[] mawjud = Directory.GetFiles(
                dalil, "*" + ImtidadRuqaa, SearchOption.TopDirectoryOnly);
            if (mawjud.Length == 0)
            {
                return null;
            }
            // Newest first, the name only to break a tie — the Mono twin's rule
            // and for its reason: sorting by name alone loads whichever patch
            // happens to sort first, which in a game that has been patched twice
            // is the older one, and the game then stays in its original language
            // with nothing in the log to say why.
            //
            // Every timestamp is read once, into the array that is then sorted,
            // for the Mono twin's second reason: a filesystem call inside the
            // comparator is a value the sort assumes is fixed and the filesystem
            // does not promise to be, and .NET raises "IComparer.Compare()
            // method returns inconsistent results" when the ordering stops being
            // coherent — inside somebody's game, at load.
            (DateTime Waqt, string Masar)[] murattaba =
                new (DateTime, string)[mawjud.Length];
            for (int i = 0; i < mawjud.Length; i++)
            {
                murattaba[i] = (File.GetLastWriteTimeUtc(mawjud[i]), mawjud[i]);
            }
            Array.Sort(murattaba, (awwal, thani) =>
            {
                int muqarana = thani.Waqt.CompareTo(awwal.Waqt);
                return muqarana != 0
                    ? muqarana
                    : StringComparer.Ordinal.Compare(awwal.Masar, thani.Masar);
            });
            return murattaba[0].Masar;
        }

        private static MaqbadSiyaq InshaSiyaq()
        {
            TaaribKhiyaratSiyaq khiyarat = default;
            khiyarat.MizaniyatMakhzan = (nuint)(4 * 1024 * 1024);
            khiyarat.AdadMakhazin = 1;
            return MaqbadSiyaq.Insha(khiyarat);
        }

        private MaqbadSilsila IbniSilsila(MaqbadSiyaq siyaqHali, Ruqaa maftuha)
        {
            ReadOnlySpan<MadkhalKhatt> sijillat = maftuha.Khutut;
            if (sijillat.Length == 0)
            {
                throw new KhataTaarib(
                    Ramz.KhattMarfud,
                    "TAARIB-E-6602",
                    "الرقعة لا تسمّي أي خط، فلا يمكن تشكيل أي نص بها.",
                    "The patch names no font, so no text can be shaped with it.",
                    Khutwa.IadatTarkibIttar);
            }

            string dalilKhutut = Path.Combine(dalil, DalilKhutut);
            MaqbadKhatt[] muhammala = new MaqbadKhatt[sijillat.Length];
            khutut = muhammala;
            for (int i = 0; i < sijillat.Length; i++)
            {
                muhammala[i] = HammilKhatt(siyaqHali, maftuha, dalilKhutut, i);
            }
            return MaqbadSilsila.Insha(siyaqHali, muhammala);
        }

        private MaqbadKhatt HammilKhatt(
            MaqbadSiyaq siyaqHali, Ruqaa maftuha, string dalilKhutut, int fahras)
        {
            MadkhalKhatt sijill = maftuha.Khutut[fahras];
            string ism = maftuha.IsmKhatt(sijill);
            if (ism.Length == 0
                || ism.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0
                || ism.IndexOf(Path.DirectorySeparatorChar) >= 0
                || ism.IndexOf(Path.AltDirectorySeparatorChar) >= 0
                || ism == "." || ism == "..")
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6603",
                    $"اسم ملف الخط في سجل الخط {fahras} ليس اسمًا بسيطًا؛ رُفض.",
                    $"The font file name in font record {fahras} is not a plain name; it was "
                    + "refused.",
                    Khutwa.IadatTarkibIttar);
            }

            string masar = Path.Combine(dalilKhutut, ism);
            if (!File.Exists(masar))
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6604",
                    $"الخط {ism} الذي تسمّيه الرقعة غير موجود في {dalilKhutut}.",
                    $"The font {ism} the patch names is not present in {dalilKhutut}.",
                    Khutwa.IadatTarkibIttar);
            }

            byte[] bayt = File.ReadAllBytes(masar);
            if (!Ruqaa.TahaqqaqBasma(bayt, maftuha.BasmaKhatt(fahras)))
            {
                throw new KhataTaarib(
                    Ramz.KhattMarfud,
                    "TAARIB-E-6605",
                    $"بصمة الخط {ism} لا تطابق ما سجّلته الرقعة؛ تغيّر الملف بعد التثبيت.",
                    $"The content hash of {ism} does not match what the patch recorded; the "
                    + "file changed after installation.",
                    Khutwa.IadatTarkibIttar);
            }

            // Face zero, always — not `fahras`.
            //
            // `fahras` is this font's position in the patch's chain; the
            // argument is the face index *inside the file*, which only a `.ttc`
            // collection has more than one of. Passing the chain position asked
            // for face 1 of the second font, face 2 of the third, and so on, and
            // every ordinary `.ttf` has exactly one — so a patch carrying a
            // single font worked and a patch carrying four died on the second
            // with TAARIB-E-2501, "the requested face is not in the file". The
            // compiler shapes every face at index zero (`MawridKhatt::jadeed`
            // takes `0`), and nothing in the patch records a face index at all,
            // so zero is not a guess: it is the only value that can be right.
            return MaqbadKhatt.MinDhakira(siyaqHali, bayt, 0, fahsArabi: true);
        }

        private Lawha IbniLawha(MaqbadSiyaq siyaqHali, Ruqaa maftuha)
        {
            int bud = budLawha.Value;
            bud = bud < 256 ? 256 : (bud > Lawha.AqsaBud ? Lawha.AqsaBud : bud);
            int miza = mizaniyatLawha.Value;
            miza = miza < 8 ? 8 : (miza > 512 ? 512 : miza);
            MaqbadLawha maqbad = MaqbadLawha.Insha(
                siyaqHali, (ushort)bud, (ushort)bud, 1, maftuha.Namat,
                (nuint)((long)miza * 1024 * 1024));
            return new Lawha(maqbad, maftuha.Namat, yamlik: true);
        }

        /// <summary>
        /// Builds the resolution ladder, with whichever rungs this process can
        /// actually construct.
        /// </summary>
        /// <remarks>
        /// Rungs one and two need nothing but the running process. Rung three
        /// needs a signature database, and a build shipped without one has two
        /// rungs rather than a broken third — which the ladder reports as a rung
        /// being unavailable, once, with a sentence, instead of failing
        /// identically for every target.
        /// </remarks>
        private Sullam IbniSullam()
        {
            List<IRutbatHall> rutab = new List<IRutbatHall>(3) { new Bayanat(), new Waqt() };

            string masarBasmat = Path.Combine(dalil, MalafBasmat);
            try
            {
                if (Qaida.HawilFath(
                        masarBasmat, out Qaida? qaida, out HalatQaida halat,
                        out string sababQaida)
                    && qaida is not null)
                {
                    Isdar muharrik = Kashf.IsdarMuharrik(out string sababMuharrik);
                    Isdar nusus = Kashf.IsdarNusus();
                    if (sababMuharrik.Length != 0)
                    {
                        Log.LogWarning(
                            $"تعذّر تحديد إصدار المحرك بدقة: {sababMuharrik} | The engine "
                            + $"version could not be determined precisely: {sababMuharrik}");
                    }
                    Mazwad mazwad = new Mazwad(
                        qaida, muharrik, nusus, Kashf.BinyatAlAamaliya(), Kashf.MinassatAlAala());
                    rutab.Add(new Masah(mazwad));
                }
                else
                {
                    // Absent is normal and is logged at info; unreadable or
                    // malformed is a file somebody shipped or edited wrongly and
                    // is worth a warning, because the rung it disables is the
                    // one protected titles depend on.
                    string satr = $"قاعدة البصمات غير متاحة ({halat}): {sababQaida} | The "
                        + $"signature database is unavailable ({halat}): {sababQaida}";
                    if (halat == HalatQaida.Ghaiba)
                    {
                        Log.LogInfo(satr);
                    }
                    else
                    {
                        Log.LogWarning(satr);
                    }
                }
            }
            catch (Exception khata)
            {
                // A broken database is a missing rung, never a failed startup:
                // the overwhelming majority of games resolve on rung one and
                // would be denied a working translation by a file they never
                // needed.
                Log.LogWarning(
                    "تعذّرت قراءة قاعدة البصمات؛ يعمل السلّم بدرجتين. | The signature "
                    + $"database could not be read; the ladder runs with two rungs. "
                    + khata.Message);
            }

            return new Sullam(rutab.ToArray());
        }

        private JalsatIltiqat? IbdaIltiqat()
        {
            if (!yaltaqit.Value)
            {
                return null;
            }
            Log.LogWarning(
                "وضع الالتقاط مفعَّل: تُسجَّل النصوص ولا تُستبدل. | Capture mode is on: strings "
                + "are recorded and nothing is replaced.");
            return new JalsatIltiqat();
        }

        private void NahiIltiqat()
        {
            JalsatIltiqat? hali = jalsa;
            if (hali is null)
            {
                return;
            }
            string masar = Path.Combine(dalil, MalafIltiqat);
            int maktub = hali.Aktub(masar);
            Log.LogInfo(
                $"كُتب {maktub} سطر التقاط إلى {masar}. | Wrote {maktub} capture rows to "
                + $"{masar}.");
            if (!hali.Tam)
            {
                Log.LogWarning(
                    $"امتلأت جلسة الالتقاط وتوقّف التسجيل؛ أُهمل {hali.AdadMuhmal} نص. | The "
                    + $"capture session filled and stopped; {hali.AdadMuhmal} strings were "
                    + "dropped.");
            }
            jalsa = null;
        }

        private void RakkibAnzima(MawaridIl2cpp mushtaraka, Harmony harmuni)
        {
            IReadOnlyList<INizamIl2cpp> murashshahun = Wusul.Kul();
            for (int i = 0; i < murashshahun.Count; i++)
            {
                INizamIl2cpp nizam = murashshahun[i];
                bool mawjud;
                try
                {
                    mawjud = nizam.Mawjud(mushtaraka.Sullam);
                }
                catch (Exception khata)
                {
                    Log.LogWarning(
                        $"تعذّر فحص نظام {nizam.Ism}. | Text system {nizam.Ism} could not be "
                        + $"probed. {khata.Message}");
                    continue;
                }
                if (!mawjud)
                {
                    continue;
                }

                try
                {
                    nizam.Rakkib(mushtaraka, harmuni);
                    anzima.Add(nizam);
                    Log.LogInfo($"رُكّب نظام {nizam.Ism}. | Text system {nizam.Ism} installed.");
                }
                catch (KhataTaarib khata)
                {
                    Log.LogWarning(
                        $"تعطّل نظام {nizam.Ism}: {khata.Arabi} | Text system {nizam.Ism} is "
                        + $"disabled: {khata.Injilizi}");
                    Jarrib(nizam.Fukk, nizam.Ism);
                }
                catch (Exception khata)
                {
                    Log.LogWarning(
                        $"تعطّل نظام {nizam.Ism}. | Text system {nizam.Ism} is disabled. "
                        + khata.Message);
                    Jarrib(nizam.Fukk, nizam.Ism);
                }
            }
        }

        /// <summary>
        /// Says which rung found what, because a takeover that works is not the
        /// same as a takeover that works for the right reason.
        /// </summary>
        /// <remarks>
        /// The summary line is always logged; the per-target detail only under
        /// the diagnostics setting. A game resolving entirely on rung one should
        /// cost one line at startup, and a game that fell to byte patterns
        /// should say so without being asked — because that is the state where
        /// a future engine update silently breaks the install.
        /// </remarks>
        private void BallighAnAlSullam(Sullam sullamHali)
        {
            if (anzima.Count == 0)
            {
                Log.LogWarning(
                    "حُمِّلت الرقعة ولم يُعثر على أي نظام نصوص معروف في هذه اللعبة. | The "
                    + "patch loaded and no text system this build knows was found in this "
                    + "game.");
            }
            else
            {
                Log.LogInfo(
                    $"تعريب يعمل على {anzima.Count} نظام نصوص. | Taarib is running on "
                    + $"{anzima.Count} text system(s).");
            }

            if (!sullamHali.KulluhaMinBayanat)
            {
                Log.LogWarning(
                    $"استُخدمت البصمات الثنائية في {sullamHali.Adad(Rutba.Basmat)} هدف؛ قد "
                    + "يتعطّل هذا مع تحديث اللعبة. | Binary signatures were used for "
                    + $"{sullamHali.Adad(Rutba.Basmat)} target(s); this can break when the "
                    + "game updates.");
            }

            if (!tashkhis.Value)
            {
                return;
            }
            IReadOnlyList<string> taqreer = sullamHali.Taqreer();
            for (int i = 0; i < taqreer.Count; i++)
            {
                Log.LogInfo("  " + taqreer[i]);
            }
        }

        private void Ista(string arabi, string injilizi, string ramz, Exception? khata)
        {
            Halat = HalatTaarib.Faashil;
            SababAlFashal = injilizi;
            string ras = ramz.Length == 0 ? "Taarib" : ramz;
            Log.LogError($"{ras}: {arabi} | {injilizi}");
            if (khata is not null)
            {
                Log.LogDebug(khata.ToString());
            }
            try
            {
                Unload();
            }
            catch (Exception thani)
            {
                Log.LogDebug(thani.ToString());
            }
            Halat = HalatTaarib.Faashil;
        }

        private void Jarrib(Action amal, string ism)
        {
            try
            {
                amal();
            }
            catch (Exception khata)
            {
                Log.LogError($"تعذّر إغلاق {ism}. | {ism} could not be closed. {khata.Message}");
            }
        }
    }
}
