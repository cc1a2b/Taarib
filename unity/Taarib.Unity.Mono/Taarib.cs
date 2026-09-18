// تعريب — the plugin root for Unity games on the Mono scripting backend.
//
// BepInEx loads this. Everything the adapter does begins here and is torn down
// here, in the reverse order it was built, so a game that unloads the plugin
// gets its own text rendering back rather than a half-detached takeover.
//
// THE ONE RULE THIS FILE EXISTS TO ENFORCE: nothing thrown here reaches
// BepInEx. A plugin that throws out of Awake can stop a game from starting, and
// a player who installed a translation and got a game that will not launch has
// been harmed by Taarib far more than they were helped. So every step below is
// individually guarded, every failure is logged in both languages with its
// permanent code, and any failure at all leaves the game in its original
// language and running. There is no path through this file that ends in an
// unhandled exception, and the ordering below is arranged so that each step's
// failure is survivable by the steps that did succeed.
//
// WHAT IT BUILDS, IN ORDER, AND WHY THAT ORDER.
//
//   1. Configuration. Read first because it decides whether to do anything at
//      all, and because a player who has turned the patch off should not pay
//      for a memory-mapped file they asked not to load.
//
//   2. The native library. Loaded from the plugin folder rather than the
//      system path — see Muhammil — because a Taarib built for one game must
//      not be shadowed by a different build sitting in PATH.
//
//   3. The patch container. Mapped and validated before anything else touches
//      it, and the validation includes a full BLAKE3 pass over the body. That
//      pass costs a fraction of the time the game spends loading its first
//      scene and it happens once.
//
//   4. The font chain. The container names fonts; it does not carry them.
//      Each file is read from khutut/, hashed, and compared against the hash
//      the container recorded — a font replaced on disk after installation is
//      refused here rather than shaped with, because a font swapped for one
//      without Arabic OpenType tables would silently produce isolated letters,
//      which is the exact failure this whole product exists to prevent.
//
//   5. The layout context and the atlas, sized from the patch's own header so
//      the atlas mode matches what the glyph map was built against.
//
//   6. The text systems. Discovered, not configured: each adapter asks whether
//      its types are in the loaded assemblies and disables itself quietly when
//      they are not. Most games have one or two of these, and a game without
//      NGUI is not a game with a problem.
//
// WHY DISCOVERY AND NOT A SETTING. A setting would have to be right in a file
// nobody edits, for a game nobody anticipated, and being wrong in either
// direction is bad: naming a system the game does not have wastes startup and
// logs a scary line, and failing to name one it does have leaves that text in
// English with no explanation. The assemblies already know the answer.

using System;
using System.Collections.Generic;
using System.IO;
using BepInEx;
using BepInEx.Configuration;
using BepInEx.Logging;
// BaseUnityPlugin lives in the BepInEx root namespace on the 5.x line this
// assembly now targets; the 6.x line moved it to BepInEx.Unity.Mono, and a
// build against that namespace is one BepInEx 5 cannot load at all.
using HarmonyLib;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mono.Anzimat;
using Taarib.Unity.Mono.Suluk;
using Taarib.Unity.Mushtarak;
using UnityEngine;

namespace Taarib.Unity.Mono
{
    /// <summary>
    /// What state the takeover reached, which is what the diagnostics screen
    /// and the BepInEx log both report.
    /// </summary>
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
    /// The contract every text-system adapter satisfies.
    /// </summary>
    /// <remarks>
    /// An interface rather than a base class because the adapters share no
    /// state and no behaviour — TextMeshPro and NGUI have nothing in common
    /// except that both are asked the same three questions. Inheritance here
    /// would be a base class with nothing in it.
    /// </remarks>
    public interface INizamNass
    {
        /// <summary>The system's name, for the log and the diagnostics screen.</summary>
        string Ism { get; }

        /// <summary>
        /// Whether this system's types are present in the loaded assemblies.
        /// </summary>
        /// <returns>Whether the game uses this system at all.</returns>
        /// <remarks>
        /// Must not throw and must not patch anything. Called before any
        /// decision to install, on a game that may have none of these systems.
        /// </remarks>
        bool Mawjud();

        /// <summary>Installs the takeover.</summary>
        /// <param name="siyaq">Everything the adapter needs to draw.</param>
        /// <param name="tarqee">The Harmony instance every patch is registered under.</param>
        /// <exception cref="KhataTaarib">
        /// A member could not be resolved. The root catches it, disables this
        /// system alone, and leaves the others running.
        /// </exception>
        void Rakkib(SiyaqIstila siyaq, Harmony tarqee);

        /// <summary>Removes the takeover and restores whatever it changed.</summary>
        /// <remarks>
        /// Must not throw. Called during teardown, possibly after a partial
        /// installation, and an exception here would leave later adapters
        /// un-torn-down.
        /// </remarks>
        void Fukk();
    }

    /// <summary>
    /// Everything an adapter needs in order to replace one string.
    /// </summary>
    /// <remarks>
    /// Passed by reference and never copied: the handles inside it are owned by
    /// the plugin root and outlive every adapter. An adapter that stored a copy
    /// and outlived the root would be drawing from a disposed atlas.
    /// </remarks>
    public sealed class SiyaqIstila
    {
        /// <summary>Builds the context. Called once, by the plugin root.</summary>
        /// <param name="ruqaa">The mapped patch.</param>
        /// <param name="siyaq">The native layout context.</param>
        /// <param name="silsila">The font chain.</param>
        /// <param name="lawha">The atlas and its residency bookkeeping.</param>
        /// <param name="jalsa">The capture session, or null when not capturing.</param>
        /// <param name="sijill">Where an adapter writes its log lines.</param>
        public SiyaqIstila(
            Ruqaa ruqaa,
            MaqbadSiyaq siyaq,
            MaqbadSilsila silsila,
            Lawha lawha,
            JalsatIltiqat? jalsa,
            ManualLogSource sijill)
        {
            Ruqaa = ruqaa ?? throw new ArgumentNullException(nameof(ruqaa));
            Siyaq = siyaq ?? throw new ArgumentNullException(nameof(siyaq));
            Silsila = silsila ?? throw new ArgumentNullException(nameof(silsila));
            Lawha = lawha ?? throw new ArgumentNullException(nameof(lawha));
            Jalsa = jalsa;
            Sijill = sijill ?? throw new ArgumentNullException(nameof(sijill));
        }

        /// <summary>The mapped patch.</summary>
        public Ruqaa Ruqaa { get; }

        /// <summary>The native layout context, for text the compiler never saw.</summary>
        public MaqbadSiyaq Siyaq { get; }

        /// <summary>The font chain to shape and draw with.</summary>
        public MaqbadSilsila Silsila { get; }

        /// <summary>The atlas and its residency bookkeeping.</summary>
        public Lawha Lawha { get; }

        /// <summary>
        /// The capture session, or <c>null</c> when the plugin is replacing text
        /// rather than recording it.
        /// </summary>
        /// <remarks>
        /// Non-null is what puts an adapter into observe-only posture: record
        /// the string with its measured rectangle and let the game draw its own
        /// text. The two are mutually exclusive by design — a capture taken
        /// while Taarib was drawing would record Taarib's own rectangles, not
        /// the game's, and every constraint in the resulting patch would be
        /// measured against the wrong box.
        /// </remarks>
        public JalsatIltiqat? Jalsa { get; }

        /// <summary>Whether the plugin is recording rather than replacing.</summary>
        public bool Yaltaqit => Jalsa != null;

        /// <summary>Where an adapter writes its log lines.</summary>
        public ManualLogSource Sijill { get; }
    }

    /// <summary>
    /// The BepInEx plugin: the Mono half of the Unity takeover.
    /// </summary>
    /// <remarks>
    /// One instance, created by BepInEx. Everything it owns is disposed by
    /// <see cref="Fakkik"/> in the reverse order it was built, on a quit or a
    /// failure — not when the game destroys the host object, which R.E.P.O.
    /// does on its first scene load.
    /// </remarks>
    [BepInPlugin(Muarrif, IsmMaruud, Isdar)]
    public sealed class MulhaqTaarib : BaseUnityPlugin
    {
        /// <summary>The plugin's GUID, as BepInEx and other plugins see it.</summary>
        public const string Muarrif = "com.cc1a2b.taarib.unity.mono";

        /// <summary>The plugin's display name.</summary>
        public const string IsmMaruud = "Taarib";

        /// <summary>The plugin's version.</summary>
        public const string Isdar = "1.0.1";

        /// <summary>The folder, beside the plugin, a hand-assembled layout may use.</summary>
        public const string DalilTaarib = "Taarib";

        /// <summary>
        /// The folder under the game root the installer writes package content
        /// into: <c>taarib-tathbeet</c>'s <c>MUJALLAD_TAARIB</c>, lower-case, and
        /// the same for every engine.
        /// </summary>
        public const string DalilRuqaaFiAlLuba = "taarib";

        /// <summary>The subfolder holding the font files the patch names.</summary>
        public const string DalilKhutut = "khutut";

        /// <summary>The extension of a patch container.</summary>
        public const string ImtidadRuqaa = ".ruqaa";

        /// <summary>The name of the capture file a recording session writes.</summary>
        public const string MalafIltiqat = "iltiqat.jsonl";

        /// <summary>
        /// The gutter the runtime atlas leaves around every glyph, in texels.
        /// </summary>
        /// <remarks>
        /// One on each side, which puts two texels of zeros between any two
        /// neighbouring glyphs — exactly what a bilinear tap at a rectangle's
        /// edge can reach, and the packer's own default. It is named once
        /// because two things need it and they must not disagree: the native
        /// packer, which reserves it, and <see cref="Mushtarak.Lawha"/>, which
        /// has to include it in every rectangle it sends to the GPU. A
        /// <see cref="Mushtarak.Lawha"/> told the gutter is narrower than it is
        /// uploads the glyph and not the zeros beside it, and the letter next
        /// door bleeds a sliver into it.
        /// </remarks>
        private const ushort HashwLawha = 1;

        private readonly List<INizamNass> anzima = new List<INizamNass>();
        // Assigned by IqraIdadat, which is the first thing Awake does and the
        // only thing that runs before anything reads them. Declared with the
        // null-forgiving initializer rather than as nullable because making them
        // nullable would put a `?.` on every read of a value that is never
        // actually absent, and a `?.` that can never be taken is a check a
        // reader has to reason about for nothing.
        private ConfigEntry<bool> mufaal = null!;
        private ConfigEntry<bool> yaltaqit = null!;
        private ConfigEntry<bool> tashkhis = null!;
        private ConfigEntry<int> mizaniyatLawha = null!;
        private ConfigEntry<int> budLawha = null!;

        // These are genuinely absent until each step of Awake succeeds, and
        // absent again after teardown. Nullable, and null-checked everywhere.
        private Harmony? tarqee;
        private Ruqaa? ruqaa;
        private MaqbadSiyaq? siyaq;
        private MaqbadSilsila? silsila;
        private MaqbadKhatt[]? khutut;
        private Lawha? lawha;
        private JalsatIltiqat? jalsa;
        private SiyaqIstila? istila;
        private MawaridIstila? mawarid;
        private string dalil = string.Empty;
        private string dalilMulhaq = string.Empty;

        /// <summary>What state the takeover reached.</summary>
        public HalatTaarib Halat { get; private set; } = HalatTaarib.Bila;

        /// <summary>
        /// The one sentence describing why the takeover is not running, or an
        /// empty string when it is.
        /// </summary>
        /// <remarks>
        /// Kept so that a diagnostics screen or another plugin can ask, rather
        /// than requiring somebody to find the line in a log file that has since
        /// scrolled past several thousand entries.
        /// </remarks>
        public string SababAlFashal { get; private set; } = string.Empty;

        /// <summary>Brings the takeover up. Never throws.</summary>
        private void Awake()
        {
            try
            {
                // Every behaviour and adapter reports why it switched itself
                // off through this one sink. It was never assigned, so every
                // such sentence — including the one naming the missing shader
                // that stopped every Unity game — was computed and dropped.
                Rabt.Sijill = sabab => Logger.LogWarning(sabab);
                IqraIdadat();
                // The miss line the setting has always described. Each string
                // the game draws that the patch has no entry for is named once,
                // so a patch that covers less than expected shows exactly what
                // it missed instead of a screen that stays in English for no
                // stated reason.
                Rabt.SijillFawt = tashkhis.Value ? satr => Logger.LogInfo(satr) : null;
                if (!mufaal.Value)
                {
                    Halat = HalatTaarib.Muattal;
                    Logger.LogInfo("تعريب معطَّل في الإعدادات. | Taarib is disabled in configuration.");
                    return;
                }

                dalilMulhaq = DalilAlMulhaq();
                dalil = DalilAlRuqaa(dalilMulhaq);
                Hammil();

                // Built into locals and published to the fields afterwards. The
                // fields exist for teardown, which has to cope with a partial
                // build; the locals are what this method reasons about, and
                // reasoning about a field that another method might have left
                // null is how a teardown path ends up dereferencing one.
                Ruqaa maftuha = IftahRuqaa();
                ruqaa = maftuha;
                MaqbadSiyaq siyaqJadid = InshaSiyaq();
                siyaq = siyaqJadid;
                MaqbadSilsila silsilaJadida = IbniSilsila(siyaqJadid, maftuha);
                silsila = silsilaJadida;
                Lawha lawhaJadida = IbniLawha(siyaqJadid, maftuha);
                lawha = lawhaJadida;
                jalsa = IbdaIltiqat();

                SiyaqIstila siyaqIstila = new SiyaqIstila(
                    maftuha, siyaqJadid, silsilaJadida, lawhaJadida, jalsa, Logger);
                istila = siyaqIstila;

                // One glyph source for all five adapters. Two would mean two
                // copies of every rasterized glyph and, worse, two materials —
                // and two materials never batch together however identical they
                // are, so a game with both TextMeshPro and NGUI would double its
                // text draw calls for nothing.
                MawaridIstila? mushtaraka = MawaridIstila.Insha(
                    maftuha, siyaqJadid, silsilaJadida, lawhaJadida, jalsa);
                if (mushtaraka is null)
                {
                    throw new KhataTaarib(
                        Ramz.GhayrMuhayyaa,
                        "TAARIB-E-6300",
                        "تعذّر تجهيز مصدر الأشكال؛ لا يمكن رسم أي نص.",
                        "The glyph source could not be prepared; no text can be drawn.",
                        Khutwa.FathTashkhis);
                }
                mawarid = mushtaraka;

                Harmony harmuni = new Harmony(Muarrif);
                tarqee = harmuni;
                RakkibAnzima(siyaqIstila, harmuni, mushtaraka);

                // Subscribed once the takeover is up, so a failure before this
                // point leaves no handler pointing at a half-built plugin.
                Application.quitting += AlaKhurujTatbiq;
                Halat = istila.Yaltaqit ? HalatTaarib.Iltiqat : HalatTaarib.Amil;
                if (anzima.Count == 0)
                {
                    // Not a failure, and worth saying plainly: the patch loaded
                    // and nothing in this game draws text through a system this
                    // build knows. Silence here would look like a broken install.
                    Logger.LogWarning(
                        "حُمِّلت الرقعة ولم يُعثر على أي نظام نصوص معروف في هذه اللعبة؛ لن "
                        + "يُستبدل أي نص. | The patch loaded and no text system this build "
                        + "knows was found in this game; no text will be replaced.");
                }
                else
                {
                    Logger.LogInfo(
                        $"تعريب يعمل على {anzima.Count} نظام نصوص. | Taarib is running on "
                        + $"{anzima.Count} text system(s).");
                }
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

        /// <summary>
        /// Set when the process is ending, so the teardown below can tell the
        /// end of the run from the loss of its host object.
        /// </summary>
        private bool yukhrij;

        /// <summary>
        /// Unity's own end-of-run signal, taken from the static event rather
        /// than the message.
        /// </summary>
        /// <remarks>
        /// <c>OnApplicationQuit</c> is delivered to live components, and this
        /// one is routinely dead: R.E.P.O. destroys BepInEx's manager object at
        /// frame 0 and the takeover deliberately outlives it. So the message
        /// never arrived, <see cref="Fakkik"/> never ran, and everything that
        /// happens at teardown never happened — including writing the capture
        /// file, which is the whole output of a capture session. A translator
        /// played the game, the log said strings were being recorded, and the
        /// file was not there afterwards.
        /// <para>
        /// <c>Application.quitting</c> is static, fires once for the process,
        /// and does not care whether any particular object is alive.
        /// </para>
        /// </remarks>
        private void AlaKhurujTatbiq()
        {
            yukhrij = true;
            Fakkik();
        }

        /// <summary>
        /// The host object's death is not the takeover's.
        /// </summary>
        /// <remarks>
        /// R.E.P.O. destroys BepInEx's manager object at frame 0, on its first
        /// scene load, and every plugin component with it. The takeover does
        /// not live on that component: the Harmony prefixes, the atlas and the
        /// mapped patch are static and keep working after it is gone, and the
        /// only thing tearing them down here achieved was a game that loaded
        /// the patch, said so, and then drew every string in English. So the
        /// teardown runs on a quit — and on a failure, through
        /// <see cref="Ista"/> — and a host destroyed mid-game is logged and
        /// otherwise ignored.
        /// </remarks>
        private void OnDestroy()
        {
            if (!yukhrij && (Halat == HalatTaarib.Amil || Halat == HalatTaarib.Iltiqat))
            {
                Logger.LogInfo(
                    "دُمّر كائن المضيف في الإطار " + Time.frameCount
                    + " وبقي التعريب يعمل. | The host object was destroyed at frame "
                    + Time.frameCount + "; the takeover stays up.");
                return;
            }
            Fakkik();
        }

        /// <summary>Tears the takeover down. Never throws.</summary>
        private void Fakkik()
        {
            // Removed first: the event is static and a handler left pointing at
            // a torn-down plugin would run the whole teardown a second time at
            // the end of the process.
            Application.quitting -= AlaKhurujTatbiq;
            // The run's own answer to "how much of this game is Arabic", written
            // once at the end whether or not the interval was ever crossed.
            Rabt.TaqreerNihai();

            for (int i = anzima.Count - 1; i >= 0; i--)
            {
                try
                {
                    anzima[i].Fukk();
                }
                catch (Exception khata)
                {
                    Logger.LogError(
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
            lawha = null;
            silsila = null;
            khutut = null;
            siyaq = null;
            ruqaa = null;
            istila = null;
            Halat = HalatTaarib.Bila;
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
                + "building a patch. Mutually exclusive with replacing text, because a "
                + "capture taken while Taarib was drawing would record Taarib's rectangles "
                + "instead of the game's.");

            tashkhis = Config.Bind(
                "عام",
                "tashkhis",
                false,
                "Log a line for every string that was looked up and missed. Useful when a "
                + "patch covers less of a game than expected; very noisy otherwise.");

            budLawha = Config.Bind(
                "لوحة",
                "bud",
                2048,
                "The atlas page size in texels, per side. Larger pages mean fewer draw call "
                + "changes and more memory. 4096 is the maximum.");

            mizaniyatLawha = Config.Bind(
                "لوحة",
                "mizaniya",
                64,
                "The atlas budget in mebibytes. When it is reached the atlas evicts glyphs "
                + "it has not drawn recently rather than growing inside the game's memory.");
        }

        /// <summary>The directory this plugin assembly was loaded from.</summary>
        /// <remarks>
        /// Where the native library lives — <c>jisr/&lt;arch&gt;/</c> is staged
        /// beside the managed assemblies and nowhere else — and deliberately not
        /// where the patch is looked for. The two were one value once, and that
        /// is one of the reasons nothing ever reached the screen: the installer
        /// places the package under the game root and this plugin looked beside
        /// itself.
        /// </remarks>
        private string DalilAlMulhaq()
        {
            string? mawdi = Info?.Location;
            string? qaida = string.IsNullOrEmpty(mawdi)
                ? Paths.PluginPath
                : Path.GetDirectoryName(mawdi);
            return string.IsNullOrEmpty(qaida) ? Paths.PluginPath : qaida;
        }

        /// <summary>The directory the installed patch and its fonts are read from.</summary>
        /// <remarks>
        /// The installer writes package content to <c>&lt;game root&gt;/taarib/</c>
        /// for every engine it patches at tier one — that is the directory its
        /// manifest records and its restore path deletes — so that is looked at
        /// first. The two plugin-relative locations are kept for a hand-assembled
        /// layout, in the order the original design named them. The first that
        /// holds a patch file wins; when none does, the game-root location is
        /// returned so the refusal names the place the installer should have
        /// written to.
        /// </remarks>
        private static string DalilAlRuqaa(string dalilMulhaqHali)
        {
            string[] murashshaha =
            {
                Path.Combine(Paths.GameRootPath, DalilRuqaaFiAlLuba),
                Path.Combine(dalilMulhaqHali, DalilTaarib),
                dalilMulhaqHali,
            };
            foreach (string murashshah in murashshaha)
            {
                if (Directory.Exists(murashshah)
                    && Directory.GetFiles(murashshah, "*" + ImtidadRuqaa, SearchOption.TopDirectoryOnly).Length > 0)
                {
                    return murashshah;
                }
            }
            return murashshaha[0];
        }

        private void Hammil()
        {
            Muhammil.Tahmeel(dalilMulhaq);
            Muhammil.Taakkad();
            Logger.LogInfo(
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
                    "TAARIB-E-6201",
                    $"لا توجد رقعة تعريب في {dalil}؛ لم يكتمل التثبيت.",
                    $"There is no Taarib patch in {dalil}; the installation did not complete.",
                    Khutwa.IadatTarkibIttar);
            }

            Ruqaa maftuha = Ruqaa.Iftah(masar);
            Logger.LogInfo(
                $"فُتحت الرقعة {Path.GetFileName(masar)}: {maftuha.Nusus.Length} نص، "
                + $"{maftuha.Takhtitat.Length} تخطيط، {maftuha.MafatihAshkal.Length} شكل. | "
                + $"Opened {Path.GetFileName(masar)}: {maftuha.Nusus.Length} strings, "
                + $"{maftuha.Takhtitat.Length} layouts, {maftuha.MafatihAshkal.Length} glyphs.");
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
            // Newest first, and the name only to break a tie.
            //
            // It used to be the name alone, which is deterministic and picks an
            // arbitrary patch: a game holding a patch from an earlier install
            // and the one just written loaded whichever name sorted first, and
            // that was the old one. The install reported success, the log said a
            // patch had opened, and the game stayed in its original language —
            // the failure with no visible cause. Whatever else two patches mean,
            // the one written last is the one somebody just asked for.
            //
            // Every timestamp is read once, into the array that is then sorted.
            // Read inside the comparator instead, it is a filesystem call the
            // sort takes for a fixed value: the answer may change between two
            // comparisons of the same pair, and .NET raises
            // "IComparer.Compare() method returns inconsistent results" when the
            // ordering stops being coherent — inside somebody's game, at load,
            // with the patch about to be opened.
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
            string ahdath = murattaba[0].Masar;
            if (murattaba.Length > 1)
            {
                Logger.LogWarning(
                    $"يوجد {murattaba.Length} ملف رقعة في {dalil}؛ استُخدم الأحدث كتابةً: "
                    + $"{Path.GetFileName(ahdath)}. احذف ما لم يعد مستعملًا. | "
                    + $"{murattaba.Length} patch files are in {dalil}; the most recently written "
                    + $"was used: {Path.GetFileName(ahdath)}. Delete the ones you no longer "
                    + "want.");
            }
            return ahdath;
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
                    "TAARIB-E-6202",
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
            if (string.IsNullOrEmpty(ism))
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6203",
                    $"سجل الخط {fahras} في الرقعة لا يحمل اسم ملف.",
                    $"Font record {fahras} in the patch carries no file name.",
                    Khutwa.IadatTarkibIttar);
            }

            // The name is a file name, never a path. A container that named
            // ../../something would otherwise reach outside the folder the
            // installer owns, and the container is a file that arrived from a
            // stranger.
            if (ism.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0
                || ism.IndexOf(Path.DirectorySeparatorChar) >= 0
                || ism.IndexOf(Path.AltDirectorySeparatorChar) >= 0
                || ism == "." || ism == "..")
            {
                throw new KhataTaarib(
                    Ramz.QeemaBatila,
                    "TAARIB-E-6204",
                    $"اسم ملف الخط \"{ism}\" في الرقعة ليس اسمًا بسيطًا؛ رُفض.",
                    $"The font file name \"{ism}\" in the patch is not a plain name; it was "
                    + "refused.",
                    Khutwa.IadatTarkibIttar);
            }

            string masar = Path.Combine(dalilKhutut, ism);
            if (!File.Exists(masar))
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    "TAARIB-E-6205",
                    $"الخط {ism} الذي تسمّيه الرقعة غير موجود في {dalilKhutut}.",
                    $"The font {ism} the patch names is not present in {dalilKhutut}.",
                    Khutwa.IadatTarkibIttar);
            }

            byte[] bayt = File.ReadAllBytes(masar);
            if (!Ruqaa.TahaqqaqBasma(bayt, maftuha.BasmaKhatt(fahras)))
            {
                // Refused rather than warned about. A font swapped after
                // installation may have no Arabic OpenType tables at all, and
                // shaping with it would produce isolated, unjoined letters —
                // which is precisely the failure this product exists to prevent,
                // and which would look to a player like Taarib being broken.
                throw new KhataTaarib(
                    Ramz.KhattMarfud,
                    "TAARIB-E-6206",
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
            int bud = Mathf.Clamp(budLawha.Value, 256, Mushtarak.Lawha.AqsaBud);
            long mizaniya = (long)Mathf.Clamp(mizaniyatLawha.Value, 8, 512) * 1024 * 1024;
            MaqbadLawha maqbad = MaqbadLawha.Insha(
                siyaqHali, (ushort)bud, (ushort)bud, HashwLawha, maftuha.Namat, (nuint)mizaniya);

            // The mode comes from the patch's own header rather than from
            // configuration, because the glyph map was built against one of them
            // and drawing it as the other produces text that is legible in a
            // screenshot and wrong in motion.
            Lawha mabniya = new Lawha(maqbad, maftuha.Namat, HashwLawha, yamlik: true);

            return mabniya;
        }

        private JalsatIltiqat? IbdaIltiqat()
        {
            if (!yaltaqit.Value)
            {
                return null;
            }
            Logger.LogWarning(
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
            Logger.LogInfo(
                $"كُتب {maktub} سطر التقاط إلى {masar}. | Wrote {maktub} capture rows to "
                + $"{masar}.");
            if (!hali.Tam)
            {
                // The session filled and stopped recording. Said loudly, because
                // a capture that is silently short produces a patch that is
                // silently missing strings and a translator would never know.
                Logger.LogWarning(
                    $"امتلأت جلسة الالتقاط وتوقّف التسجيل؛ أُهمل {hali.AdadMuhmal} نص. | The "
                    + $"capture session filled and stopped; {hali.AdadMuhmal} strings were "
                    + "dropped.");
            }
            jalsa = null;
        }

        private void RakkibAnzima(
            SiyaqIstila siyaqIstila, Harmony harmuni, MawaridIstila mushtaraka)
        {
            IReadOnlyList<INizamNass> murashshahun = Wusul.Kul(mushtaraka);

            for (int i = 0; i < murashshahun.Count; i++)
            {
                INizamNass nizam = murashshahun[i];
                bool mawjud;
                try
                {
                    mawjud = nizam.Mawjud();
                }
                catch (Exception khata)
                {
                    Logger.LogWarning(
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
                    nizam.Rakkib(siyaqIstila, harmuni);
                    anzima.Add(nizam);
                    Logger.LogInfo($"رُكّب نظام {nizam.Ism}. | Text system {nizam.Ism} installed.");
                }
                catch (KhataTaarib khata)
                {
                    // One system failing disables that system alone. A game with
                    // TextMeshPro and NGUI whose NGUI binding broke should still
                    // have its TextMeshPro text translated.
                    Logger.LogWarning(
                        $"تعطّل نظام {nizam.Ism}: {khata.Arabi} | Text system {nizam.Ism} is "
                        + $"disabled: {khata.Injilizi}");
                    Jarrib(nizam.Fukk, nizam.Ism);
                }
                catch (Exception khata)
                {
                    Logger.LogWarning(
                        $"تعطّل نظام {nizam.Ism}. | Text system {nizam.Ism} is disabled. "
                        + khata.Message);
                    Jarrib(nizam.Fukk, nizam.Ism);
                }
            }
        }

        private void Ista(string arabi, string injilizi, string ramz, Exception? khata)
        {
            Halat = HalatTaarib.Faashil;
            SababAlFashal = injilizi;
            string ras = string.IsNullOrEmpty(ramz) ? "Taarib" : ramz;
            Logger.LogError($"{ras}: {arabi} | {injilizi}");
            if (khata != null)
            {
                Logger.LogDebug(khata.ToString());
            }

            // Whatever came up before the failure is taken back down, so a
            // half-built takeover does not sit in the process holding a mapping
            // and an atlas for a game it is not translating.
            try
            {
                Fakkik();
            }
            catch (Exception thani)
            {
                Logger.LogDebug(thani.ToString());
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
                Logger.LogError($"تعذّر إغلاق {ism}. | {ism} could not be closed. {khata.Message}");
            }
        }
    }
}
