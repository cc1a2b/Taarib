// وصول — the seam between the IL2CPP plugin root and the takeovers it installs.
//
// The root asks every text system the same four things — what are you called,
// are you here, install yourself, take yourself back off — and this file is the
// list of systems it asks. It is deliberately thin. Under IL2CPP each takeover
// already answers INizamIl2cpp itself, because the contract's questions map
// exactly onto what a takeover does: Mawjud is one resolution through the
// ladder, Rakkib is the interception, Fukk is the removal. So there is no
// per-system wrapper here of the kind the Mono adapter needs, and Kul() is
// literally the table.
//
// WHAT THIS FILE DOES OWN, AND WHY IT HAS TO. The glyph source. All the
// takeovers draw from ONE atlas, one set of uploaded pages and one set of
// materials. If each built its own, a game with both TextMeshPro and legacy
// uGUI would hold two copies of every rasterized glyph, two textures and two
// materials — and two materials never batch together however identical their
// contents, so the game would pay twice the text draw calls for nothing. Under
// Mono the plugin root builds that source and hands it to every adapter; under
// IL2CPP it cannot, because MawaridIl2cpp is assembled before the resolution
// ladder has been asked for a single engine method and the source cannot exist
// until it has. So the source is built lazily, by whichever takeover installs
// first, and released by whichever detaches last. MasdarMushtarak is that
// counter, and it is the only reason this file is longer than a list.
//
// WHY Kul() TAKES NOTHING. The root calls Wusul.Kul() with no arguments and then
// hands each entry the shared resources in Rakkib. That ordering is not
// cosmetic: Mawjud must be answerable before anything has been built, because
// the root uses it to decide whether to build at all, and a factory that
// required the resources up front would force the root to construct an atlas for
// a game that has no text system this build knows.
//
// THE ORDER, AND WHAT IT MEANS. TextMeshPro first, legacy uGUI second, roughly
// by how much of a modern game's text each system owns: a title with both almost
// always has its dialogue in the former. It matters only for which failure a
// player sees first in the log, because installing one does not prevent
// installing the other — the root catches per system and disables per system.
//
// WHAT IS DELIBERATELY NOT HERE. NGUI, FairyGUI and world-space TextMesh. All
// three are taken over by the Mono adapter and none of them is in this list, and
// that is a decision rather than an omission. NGUI and FairyGUI are Unity 4- and
// 5-era GUI kits: a game shipping either of them predates IL2CPP being the
// default backend by years, and the intersection of "ships NGUI" and "compiled
// with IL2CPP" is small enough that the right answer is to say so in the log
// rather than to carry two more resolvers and two more sets of byte signatures
// that nobody would exercise. World-space TextMesh is a different kind of
// exclusion: the Mono adapter reaches it without patching anything, because
// every member of TextMesh is an extern property with no IL body for HarmonyX to
// weave into, so that adapter walks the scene instead. Walking the scene under
// IL2CPP means resolving and calling FindObjectsOfType, enumerating an
// Il2CppArray of components every frame and rooting each one — a per-frame
// allocation and a per-frame native enumeration, which is precisely what the
// per-frame rules in this project forbid. Doing it properly needs a different
// mechanism from the Mono one, and inventing that mechanism belongs to the phase
// that has a game to test it against. A reader who comes here looking for those
// three should find this paragraph, not silence.

using System;
using System.Collections.Generic;

namespace Taarib.Unity.Il2cpp.Anzimat
{
    /// <summary>
    /// مصدر مشترك — the engine binding and the glyph source, built once for the
    /// whole plugin and released when the last takeover lets go.
    /// </summary>
    /// <remarks>
    /// <para>
    /// <b>Why a counter and not a field on the root.</b> The engine binding
    /// resolves about three dozen UnityEngine members through
    /// <see cref="Hall.Sullam"/>, and the glyph source uploads every compiled
    /// atlas page into a texture and builds a material for it. Neither can be
    /// built before the ladder exists, and neither should be built at all in a
    /// game that has no text system this build knows — which is exactly the
    /// game where every takeover's <see cref="INizamIl2cpp.Mawjud"/> answers
    /// false and <see cref="INizamIl2cpp.Rakkib"/> is never called. Building on
    /// first use and releasing on last release is what gets both properties: one
    /// atlas for however many takeovers install, and no atlas at all when none
    /// does.
    /// </para>
    /// <para>
    /// <b>What a failure here means.</b> <see cref="Ihjiz"/> answers
    /// <c>null</c> when the engine could not be bound or the atlas could not be
    /// prepared, having already written the sentence that names which. Every
    /// takeover turns that into its own refusal with its own permanent code, so
    /// the root logs one line per text system rather than one line per resolved
    /// member. A failure is remembered: a second takeover asking after the first
    /// failed gets the same <c>null</c> without re-running an upload that has
    /// already been refused once.
    /// </para>
    /// <para>
    /// Not thread-safe. Installation and teardown both run on the plugin's own
    /// initialisation and shutdown paths, on one thread, which is the same
    /// assumption every other type in this namespace makes.
    /// </para>
    /// </remarks>
    public sealed class MasdarMushtarak
    {
        private static MasdarMushtarak? hali;
        private static bool khaba;

        private readonly WaslMuharrik wasl;
        private readonly MasdarAshkal masdar;
        private int adadHamalat;

        private MasdarMushtarak(WaslMuharrik wasl, MasdarAshkal masdar)
        {
            this.wasl = wasl;
            this.masdar = masdar;
        }

        /// <summary>The engine binding every takeover calls Unity through.</summary>
        public WaslMuharrik Wasl => wasl;

        /// <summary>The atlas, its uploaded pages and their materials.</summary>
        public MasdarAshkal Masdar => masdar;

        /// <summary>How many takeovers currently hold this source.</summary>
        public int AdadHamalat => adadHamalat;

        /// <summary>
        /// Takes a reference to the shared source, building it on the first call.
        /// </summary>
        /// <param name="mawarid">Everything the plugin root assembled.</param>
        /// <returns>
        /// The shared source, or <c>null</c> when it could not be built — in
        /// which case the reason is already in the log and the caller should
        /// refuse with its own permanent code.
        /// </returns>
        /// <exception cref="ArgumentNullException"><paramref name="mawarid"/> is null.</exception>
        public static MasdarMushtarak? Ihjiz(MawaridIl2cpp mawarid)
        {
            if (mawarid is null)
            {
                throw new ArgumentNullException(nameof(mawarid));
            }
            if (hali is not null)
            {
                hali.adadHamalat++;
                return hali;
            }
            if (khaba)
            {
                // Remembered rather than retried. The two failures this can be —
                // the engine members not resolving, and Taarib's shader not being
                // in the build — are both structural, and re-running the upload
                // for the second takeover would put the same paragraph in the log
                // twice for the same reason.
                return null;
            }

            WaslMuharrik? engine = WaslMuharrik.Iqran(mawarid.Sullam, mawarid.Sijill);
            if (engine is null)
            {
                khaba = true;
                return null;
            }

            MasdarAshkal? source = MasdarAshkal.Insha(
                engine, mawarid.Ruqaa, mawarid.Silsila, mawarid.Lawha, mawarid.Sijill);
            if (source is null)
            {
                engine.Dispose();
                khaba = true;
                return null;
            }

            MasdarMushtarak jadid = new MasdarMushtarak(engine, source);
            jadid.adadHamalat = 1;
            hali = jadid;
            return jadid;
        }

        /// <summary>
        /// Releases one reference, destroying the textures and materials when the
        /// last takeover lets go.
        /// </summary>
        /// <remarks>
        /// Idempotent past zero, because teardown can run after a partial
        /// installation: a takeover that failed halfway through
        /// <see cref="INizamIl2cpp.Rakkib"/> still has its
        /// <see cref="INizamIl2cpp.Fukk"/> called, and a second release must not
        /// destroy an atlas another takeover is still drawing from.
        /// </remarks>
        public void Sarrih()
        {
            if (adadHamalat <= 0)
            {
                return;
            }
            adadHamalat--;
            if (adadHamalat != 0)
            {
                return;
            }
            masdar.Dispose();
            wasl.Dispose();
            if (ReferenceEquals(hali, this))
            {
                hali = null;
            }
            // The failure latch is cleared with the source, so a plugin reloaded
            // in the same process starts from a clean decision rather than
            // inheriting the previous load's refusal.
            khaba = false;
        }
    }

    /// <summary>The text systems this IL2CPP build knows how to take over.</summary>
    /// <remarks>
    /// Two, and the file header says why the Mono adapter's other three are not
    /// here. Read that paragraph before adding a fourth entry: the reasons are
    /// specific to each system and not one blanket judgement.
    /// </remarks>
    public static class Wusul
    {
        /// <summary>
        /// Every system, in the order the plugin root should try them.
        /// </summary>
        /// <returns>One entry per system, freshly constructed.</returns>
        /// <remarks>
        /// <para>
        /// Fresh instances rather than a cached array, because each entry carries
        /// the state of one installation — the interceptions it installed, the
        /// components it owns, the materials it has to give back — and a static
        /// array would carry the previous load's state into the next one. The
        /// call happens once per plugin load and allocates two objects.
        /// </para>
        /// <para>
        /// Nothing here probes, resolves or hooks. Constructing an entry is free
        /// and answers no question; the root asks <see cref="INizamIl2cpp.Mawjud"/>
        /// next, and that is the first thing that touches the ladder.
        /// </para>
        /// </remarks>
        public static IReadOnlyList<INizamIl2cpp> Kul()
        {
            return new INizamIl2cpp[]
            {
                // TextMeshPro is where a modern Unity game's dialogue lives, and
                // it is the harder of the two: it re-shapes from its own font
                // asset's glyph tables, which carry no Arabic OpenType features
                // at all, so its generation is bypassed rather than corrected.
                new NizamTmp(),

                // Legacy uGUI. Under IL2CPP its generated facade is called
                // Il2CppUnityEngine.UI, while the runtime and the binary know it
                // as UnityEngine.UI; the takeover names the native form, because
                // that is what rungs two and three can answer for. Its absence is
                // the ordinary case for a game built entirely on TextMeshPro and
                // costs nothing but a probe.
                new NizamWajiha(),
            };
        }
    }
}
