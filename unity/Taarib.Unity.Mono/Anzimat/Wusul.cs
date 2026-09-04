// وصول — the seam between the plugin root's contract and the five takeovers.
//
// The root asks every text system the same three questions: are you here,
// install yourself, take yourself back off. The five adapters answer in almost
// the same shape already — a static Ibda factory that returns null when the
// binding failed, and IDisposable to come back off — so this file is thin. What
// it supplies is the two things the adapters cannot: the probe that decides
// whether a system is present at all, and the shared resources every one of
// them draws from.
//
// WHY THE PROBE LIVES HERE AND NOT IN EACH ADAPTER. Ibda both resolves the
// binding and installs the patches, so calling it to find out whether a system
// exists would install a takeover for a game that has none. The probe is one
// type lookup and it has to happen first, separately. Keeping the five lookups
// side by side in one list is also the only place a reader can see the whole
// set of systems this build knows, which is otherwise spread over five files.
//
// WHY ONE WRAPPER AND NOT FIVE. Because the five differ in exactly one way that
// matters — whether Ibda takes a Harmony instance — and a class per system
// would be five copies of the same eight lines to express that. The world-space
// TextMesh takeover is the odd one: TextMesh's members are extern properties
// with no IL body, so there is nothing for HarmonyX to weave into and that
// adapter patches nothing at all, walking the scene instead. Its factory
// ignores the Harmony instance it is handed, which is correct rather than an
// oversight, and the list below says so.
//
// THE GLYPH SOURCE IS SHARED, AND HAS TO BE. All five draw from one atlas and
// one set of materials: MasdarAshkal. If each built its own, a game with both
// TextMeshPro and NGUI would hold two copies of every rasterized glyph, two
// textures and two materials — and two materials never batch together however
// identical their contents, so the game would pay twice the text draw calls for
// nothing.

using System;
using System.Collections.Generic;
using HarmonyLib;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// What every takeover needs, built once by the plugin root and shared.
    /// </summary>
    /// <remarks>
    /// Held rather than copied: the handles inside belong to the plugin root and
    /// outlive every adapter. What this type owns, and disposes, is the glyph
    /// source it built.
    /// </remarks>
    public sealed class MawaridIstila : IDisposable
    {
        private MawaridIstila(
            Ruqaa ruqaa,
            MasdarAshkal masdar,
            MaqbadSiyaq? siyaq,
            MaqbadSilsila? silsila,
            Takhtit takhtit)
        {
            Ruqaa = ruqaa;
            Masdar = masdar;
            Siyaq = siyaq;
            Silsila = silsila;
            Takhtit = takhtit;
        }

        /// <summary>The mapped patch.</summary>
        public Ruqaa Ruqaa { get; }

        /// <summary>The shared glyph source: atlases, pages and materials.</summary>
        public MasdarAshkal Masdar { get; }

        /// <summary>The native layout context, for text the compiler never saw.</summary>
        public MaqbadSiyaq? Siyaq { get; }

        /// <summary>The font chain.</summary>
        public MaqbadSilsila? Silsila { get; }

        /// <summary>
        /// The scratch layout every adapter lays runtime text into.
        /// </summary>
        /// <remarks>
        /// One instance, shared, and that is safe for exactly one reason: every
        /// takeover point runs on Unity's main thread, one string at a time, and
        /// the result is consumed into a mesh before the next call begins. It is
        /// not thread-safe and is not meant to be. Giving each adapter its own
        /// would mean five glyph buffers, each grown to the longest string that
        /// adapter ever met, resident for the life of the process.
        /// </remarks>
        public Takhtit Takhtit { get; }

        /// <summary>
        /// Builds the shared resources, or returns <c>null</c> when the glyph
        /// source could not be prepared.
        /// </summary>
        /// <param name="ruqaa">The mapped patch.</param>
        /// <param name="siyaq">The native layout context.</param>
        /// <param name="silsila">The font chain.</param>
        /// <param name="lawha">The runtime atlas and its residency bookkeeping.</param>
        /// <returns>The resources, or <c>null</c>.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="ruqaa"/> is null.</exception>
        /// <remarks>
        /// A null return is not an exception because whatever refused has
        /// already logged the sentence that names it — a missing shader, an
        /// atlas in a mode this build does not draw. The root turns it into one
        /// refusal for the whole takeover rather than five identical ones.
        /// </remarks>
        public static MawaridIstila? Insha(
            Ruqaa ruqaa, MaqbadSiyaq? siyaq, MaqbadSilsila? silsila, Lawha? lawha)
        {
            if (ruqaa is null)
            {
                throw new ArgumentNullException(nameof(ruqaa));
            }
            MasdarAshkal? masdar = MasdarAshkal.Insha(ruqaa, silsila, lawha);
            if (masdar is null)
            {
                return null;
            }
            return new MawaridIstila(ruqaa, masdar, siyaq, silsila, new Takhtit());
        }

        /// <summary>Releases the glyph source.</summary>
        /// <remarks>
        /// The scratch layout is not released here because there is nothing to
        /// release: it holds managed arrays and no native handle, so the
        /// collector reclaims it once the last adapter has let go.
        /// </remarks>
        public void Dispose()
        {
            Masdar.Dispose();
        }
    }

    /// <summary>
    /// One text system, as the plugin root sees it: a name, a probe, and a
    /// factory.
    /// </summary>
    public sealed class WasilatNizam : INizamNass
    {
        private readonly string naw;
        private readonly string ramz;
        private readonly MawaridIstila mawarid;
        private readonly Func<MawaridIstila, Harmony, IDisposable?> ibda;
        private IDisposable? nizam;

        /// <summary>Describes one system.</summary>
        /// <param name="ism">Its name, for the log and the diagnostics screen.</param>
        /// <param name="naw">
        /// The assembly-qualified-enough type name whose presence means the game
        /// uses this system, or an empty string for a system that is always
        /// present and discovers its own absence.
        /// </param>
        /// <param name="ramz">The permanent code to report when binding fails.</param>
        /// <param name="mawarid">The shared resources.</param>
        /// <param name="ibda">The adapter's factory.</param>
        /// <exception cref="ArgumentNullException">
        /// <paramref name="mawarid"/> or <paramref name="ibda"/> is null.
        /// </exception>
        public WasilatNizam(
            string ism,
            string naw,
            string ramz,
            MawaridIstila mawarid,
            Func<MawaridIstila, Harmony, IDisposable?> ibda)
        {
            Ism = ism ?? string.Empty;
            this.naw = naw ?? string.Empty;
            this.ramz = ramz ?? string.Empty;
            this.mawarid = mawarid ?? throw new ArgumentNullException(nameof(mawarid));
            this.ibda = ibda ?? throw new ArgumentNullException(nameof(ibda));
        }

        /// <inheritdoc/>
        public string Ism { get; }

        /// <inheritdoc/>
        public bool Mawjud()
        {
            // One type lookup, and no log line when it fails. Most games have
            // one or two of these systems, so four of the five probes returning
            // false is the ordinary case and saying so five times every launch
            // would be five lines of noise in every player's log.
            return naw.Length == 0 || Suluk.Rabt.Naw(naw) is not null;
        }

        /// <inheritdoc/>
        public void Rakkib(SiyaqIstila siyaq, Harmony tarqee)
        {
            nizam = ibda(mawarid, tarqee);
            if (nizam is null)
            {
                throw new KhataTaarib(
                    Ramz.GhayrMuhayyaa,
                    ramz,
                    $"تعذّر ربط نظام {Ism}؛ يُترك نصه بلغته الأصلية.",
                    $"The {Ism} text system could not be bound; its text is left in the "
                    + "original language.",
                    Khutwa.FathTashkhis);
            }
        }

        /// <inheritdoc/>
        public void Fukk()
        {
            nizam?.Dispose();
            nizam = null;
        }
    }

    /// <summary>The text systems this build knows how to take over.</summary>
    public static class Wusul
    {
        /// <summary>
        /// Every system, in the order the root should try them.
        /// </summary>
        /// <param name="mawarid">The shared resources.</param>
        /// <returns>One entry per system.</returns>
        /// <exception cref="ArgumentNullException"><paramref name="mawarid"/> is null.</exception>
        /// <remarks>
        /// The order is deliberate and is roughly how much of a game's text each
        /// system is likely to own: a title with both TextMeshPro and legacy
        /// uGUI almost always has its dialogue in the former. It matters only
        /// for which failure a player sees first in the log, since installing
        /// one does not prevent installing the rest.
        /// </remarks>
        public static IReadOnlyList<INizamNass> Kul(MawaridIstila mawarid)
        {
            if (mawarid is null)
            {
                throw new ArgumentNullException(nameof(mawarid));
            }

            return new INizamNass[]
            {
                new WasilatNizam(
                    "TextMeshPro", "TMPro.TMP_Text", "TAARIB-E-6301", mawarid,
                    (m, h) => NizamTmp.Ibda(
                        h, m.Ruqaa, m.Masdar, m.Siyaq, m.Silsila, m.Takhtit)),

                // UnityEngine.UI ships with Unity but is not in the engine
                // module reference package, so it is probed by name like a
                // third-party assembly.
                new WasilatNizam(
                    "UnityEngine.UI.Text", "UnityEngine.UI.Text", "TAARIB-E-6302", mawarid,
                    (m, h) => NizamWajiha.Ibda(
                        h, m.Ruqaa, m.Masdar, m.Siyaq, m.Silsila, m.Takhtit)),

                new WasilatNizam(
                    "NGUI", "UILabel", "TAARIB-E-6303", mawarid,
                    (m, h) => NizamNGui.Ibda(
                        h, m.Ruqaa, m.Masdar, m.Siyaq, m.Silsila, m.Takhtit)),

                new WasilatNizam(
                    "FairyGUI", "FairyGUI.TextField", "TAARIB-E-6304", mawarid,
                    (m, h) => NizamFairyGui.Ibda(
                        h, m.Ruqaa, m.Masdar, m.Siyaq, m.Silsila, m.Takhtit)),

                // TextMesh is an engine type, so it is always present and the
                // probe is empty; whether the game uses it is something the
                // takeover discovers by finding no components to guard. It is
                // also the one adapter that patches nothing — every member of
                // TextMesh is an extern property with no IL body, so there is
                // nothing for HarmonyX to weave into — which is why its factory
                // ignores the Harmony instance rather than taking one.
                new WasilatNizam(
                    "UnityEngine.TextMesh", string.Empty, "TAARIB-E-6305", mawarid,
                    (m, _) => NizamMujassam.Ibda(
                        m.Ruqaa, m.Masdar, m.Siyaq, m.Silsila, m.Takhtit)),
            };
        }
    }
}
