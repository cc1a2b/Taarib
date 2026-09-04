// توسعة — container reflow and right-to-left interface mirroring: the
// rectangle geometry a translated string needs, computed as values, and
// applied only where the patch says it may be.
//
// THE PROBLEM. Arabic is not the same size as the English it replaces. Some
// strings are shorter — "Settings" against "إعدادات" — and many are longer,
// because Arabic spells out what English abbreviates and because a UI font at
// the same nominal size draws Arabic taller. A button sized for its English
// label clips its Arabic one; a paragraph that fit in three lines wraps to
// four and loses the fourth off the bottom of its box.
//
// WHAT THIS FILE IS ALLOWED TO DO ABOUT IT, AND WHAT IT IS NOT. It can widen
// the box, heighten it, move it so it grows away from its own leading edge,
// and mirror it for a right-to-left interface. It may do each of those only
// where the patch's constraint record carries the matching permission bit,
// and that restriction is the most important line in the file.
//
//   THE SECOND HARD REQUIREMENT, STATED FIRST BECAUSE IT IS THE ONE PEOPLE
//   TALK THEMSELVES OUT OF: NEVER GROW A CONTAINER WHOSE PERMISSION BIT IS
//   NOT SET. A RectTransform in a shipped game is not a free variable. It is
//   an element of a layout somebody authored, and it is very often load
//   bearing: a health bar's background whose width the bar's fill is driven
//   from; a panel inside a horizontal layout group whose siblings redistribute
//   the moment it changes; a tooltip whose own script positions it from its
//   own size every frame. Growing one because a string inside it did not fit
//   fixes the string and breaks the screen, in ways that appear three menus
//   away and are never attributed to a translation patch. The patch compiler
//   measured which elements are safe to grow and recorded the answer as
//   ALAM_QAYD_NAMU_ARD and ALAM_QAYD_NAMU_IRTIFA; a reviewer looked at it.
//   Where the bit is absent, the honest response is not to grow: the layout
//   options' own overflow policy applies instead — report, shrink or truncate,
//   whichever the constraint records — and the string is written into this
//   object's refusal list by name, so it appears in the overflow report as
//   work for a translator rather than as a silently mangled screen.
//
//   THE FIRST HARD REQUIREMENT: EVERY ADJUSTMENT IS RECORDED SO IT CAN BE
//   UNDONE EXACTLY. A patch can be switched off at run time, and switching it
//   off has to put the interface back where it was — not approximately, not
//   as recomputed from the current values, but exactly. The distinction is not
//   pedantry. The tempting shape is to record only the delta, or to invert the
//   mirror by mirroring again, or to shrink the width back by the amount it
//   grew. Every one of those recomputes the original from the current state,
//   and every one of them drifts: a width restored as `hali - numu` differs
//   from the original by whatever else touched the rect in between, an anchor
//   restored by `1 - x` is exact only if nothing else moved it, and a float
//   round-tripped through the two loses its low bits. Toggle a patch three
//   times with any of those and the layout is visibly wrong, with no single
//   step that was wrong. So SijillTarajua stores the ORIGINAL VALUES, captured
//   once, the first time an element is touched and never re-captured
//   afterwards — a second adjustment of an already-adjusted element writes new
//   values over the adjusted ones and leaves the record alone, because the
//   record already holds the truth.
//
// WHY THE GEOMETRY IS COMPUTED AS VALUES BEFORE ANYTHING IS APPLIED. Ihsib is
// a pure function of numbers: no RectTransform, no component, no side effect.
// It can be run to ask "what would this need" without changing anything, which
// is what a diagnostics overlay and the capture path both want, and it means
// the permission checks are decided in one readable place instead of being
// scattered through a sequence of property writes.
//
// MIRRORING IS THREE DIFFERENT OPERATIONS AND THEY ARE GATED DIFFERENTLY.
// Flipping anchors and pivot re-pins the element, so it needs both the mirror
// bit and ALAM_QAYD_IADAT_TATHBIT — the compiler records those separately
// because an element can be safe to reverse and unsafe to re-anchor, typically
// when its position is driven by a script that writes anchoredPosition itself.
// Reversing a horizontal layout group and swapping directional padding change
// no anchor and need only the mirror bit.
//
// WHAT ALLOCATES. Ihsib allocates nothing and never touches Unity. Wassi and
// Amrih allocate exactly one undo record the first time they touch a given
// element, and nothing on any later call for the same element; the refusal
// list allocates once per distinct string that overflowed, capped. None of
// these is a per-frame path — reflow runs when the string in an element
// changes, which is when a menu opens or a line of dialogue advances.

using System;
using System.Collections.Generic;
using Taarib.Unity.Jisr;
using Taarib.Unity.Mushtarak;
using UnityEngine;

namespace Taarib.Unity.Mono.Suluk
{
    /// <summary>
    /// The bits of <see cref="NatijatTawseea.Alam"/>: what the computation
    /// decided, including what it decided not to do.
    /// </summary>
    /// <remarks>
    /// The refusals are bits rather than an absence, because "the width did not
    /// change" and "the width needed to change and was not allowed to" are
    /// different facts and only the second one belongs in an overflow report.
    /// </remarks>
    public static class AlamatTawseea
    {
        /// <summary>The width grew.</summary>
        public const uint NamuArd = 1u << 0;

        /// <summary>The height grew.</summary>
        public const uint NamuIrtifa = 1u << 1;

        /// <summary>
        /// The element was shifted so its leading edge stayed where it was
        /// while it grew.
        /// </summary>
        public const uint Izaha = 1u << 2;

        /// <summary>
        /// The width needed to grow and the constraint does not permit it. The
        /// overflow policy applies and the string is named in the refusal list.
        /// </summary>
        public const uint RafdArd = 1u << 3;

        /// <summary>The height needed to grow and the constraint does not permit it.</summary>
        public const uint RafdIrtifa = 1u << 4;

        /// <summary>
        /// Something grew, but re-anchoring is not permitted, so the growth was
        /// taken symmetrically about the element's own pivot instead of away
        /// from its leading edge. Not an error — a centred element that widens
        /// about its centre is usually right — but worth recording, because a
        /// left-pinned element that widens about its centre creeps left.
        /// </summary>
        public const uint RafdTathbit = 1u << 5;
    }

    /// <summary>
    /// The bits of a <see cref="SijillTarajua"/>: which of the recorded
    /// originals were actually changed, and therefore which are restored.
    /// </summary>
    /// <remarks>
    /// Undo restores only what it changed. Restoring every recorded value
    /// unconditionally would be simpler and would also undo whatever the game's
    /// own scripts did to the element in the meantime — a tooltip that
    /// repositions itself every frame would be snapped back to where it was
    /// when the patch first touched it, which is a bug introduced by turning
    /// the patch off.
    /// </remarks>
    public static class AlamatTarajua
    {
        /// <summary><c>sizeDelta</c> was changed.</summary>
        public const uint Hajm = 1u << 0;

        /// <summary><c>anchoredPosition</c> was changed.</summary>
        public const uint Mawqi = 1u << 1;

        /// <summary><c>anchorMin</c> and <c>anchorMax</c> were changed.</summary>
        public const uint Tathbit = 1u << 2;

        /// <summary><c>pivot</c> was changed.</summary>
        public const uint Mihwar = 1u << 3;

        /// <summary>A horizontal layout group's arrangement was reversed.</summary>
        public const uint AksTarteeb = 1u << 4;

        /// <summary>The children's sibling order was reversed.</summary>
        public const uint AksAbna = 1u << 5;

        /// <summary>A layout group's left and right padding were swapped.</summary>
        public const uint BadalHashw = 1u << 6;
    }

    /// <summary>
    /// طلب التوسعة — everything the reflow computation reads: what the text
    /// measured, what the box currently is, and what the patch permits.
    /// </summary>
    public struct TalabTawseea
    {
        /// <summary>
        /// The measured width of the widest line, in layout pixels —
        /// <see cref="TaaribQiyasNass.Ard"/> or <see cref="Takhtit.Ard"/>
        /// verbatim.
        /// </summary>
        public float ArdNass;

        /// <summary>The measured total height, in layout pixels.</summary>
        public float IrtifaNass;

        /// <summary>The rectangle's current width, in the element's own units.</summary>
        public float ArdHali;

        /// <summary>The rectangle's current height, in the element's own units.</summary>
        public float IrtifaHali;

        /// <summary>
        /// The element's pivot: zero at the left and bottom edges, one at the
        /// right and top. Unity's own order, passed through unchanged.
        /// </summary>
        public Vector2 Mihwar;

        /// <summary>
        /// Element units per layout pixel. One for a canvas whose units are
        /// pixels, which is the ordinary case; the component's own ratio for
        /// world-space text. Zero or less is read as one.
        /// </summary>
        public float Qiyas;

        /// <summary>
        /// Whether the interface direction is right to left, which decides
        /// which edge is the leading one and therefore which way a widening
        /// element extends.
        /// </summary>
        public bool Yameen;

        /// <summary>The permissions and the box the compiler measured.</summary>
        public MadkhalQayd Qayd;
    }

    /// <summary>
    /// نتيجة التوسعة — the geometry the element should have, as values.
    /// Applying it is a separate decision, made by <see cref="Tawseea.Wassi"/>.
    /// </summary>
    public struct NatijatTawseea
    {
        /// <summary>The target width, in the element's own units.</summary>
        public float Ard;

        /// <summary>The target height, in the element's own units.</summary>
        public float Irtifa;

        /// <summary>
        /// How far to move the element horizontally so its leading edge stays
        /// put while it grows. Zero when nothing grew or re-anchoring is not
        /// permitted.
        /// </summary>
        public float IzahaS;

        /// <summary>How far to move it vertically, positive upward.</summary>
        public float IzahaA;

        /// <summary>
        /// Whether the text fits after the adjustment. False means growth was
        /// refused or was not enough, and the overflow policy applies.
        /// </summary>
        public bool Yalaim;

        /// <summary>
        /// How much width the text still needs beyond the box, in the element's
        /// units. Zero when it fits.
        /// </summary>
        public float ZaidArd;

        /// <summary>How much height it still needs beyond the box.</summary>
        public float ZaidIrtifa;

        /// <summary>What was decided, per <see cref="AlamatTawseea"/>.</summary>
        public uint Alam;

        /// <summary>Whether anything at all would change.</summary>
        public bool Yughayyir =>
            (Alam & (AlamatTawseea.NamuArd | AlamatTawseea.NamuIrtifa | AlamatTawseea.Izaha)) != 0;

        /// <summary>Whether a permitted growth was refused by the constraint.</summary>
        public bool Marfud =>
            (Alam & (AlamatTawseea.RafdArd | AlamatTawseea.RafdIrtifa)) != 0;
    }

    /// <summary>
    /// سجل رفض — one string that did not fit an element it is not permitted to
    /// grow.
    /// </summary>
    /// <remarks>
    /// The diagnostics record the prompt for this file demands, and the reason
    /// it names the string rather than the element: an element is a path in a
    /// scene hierarchy that means nothing to a translator, while a string is
    /// the thing they can shorten. This is the runtime half of the overflow
    /// report the patch compiler produces offline, and the two are read side by
    /// side — the compiler's covers every string it measured, this one covers
    /// the strings that were composed at run time and that the compiler
    /// therefore never saw.
    /// </remarks>
    public struct SijillRafd
    {
        /// <summary>
        /// The string, or the key a caller identifies it by. Never empty in a
        /// recorded entry.
        /// </summary>
        public string Muarrif;

        /// <summary>The width the text needed, in the element's units.</summary>
        public float ArdMatlub;

        /// <summary>The width it had.</summary>
        public float ArdMutah;

        /// <summary>The height the text needed.</summary>
        public float IrtifaMatlub;

        /// <summary>The height it had.</summary>
        public float IrtifaMutah;

        /// <summary>Which growth was refused, per <see cref="AlamatTawseea"/>.</summary>
        public uint Alam;

        /// <summary>
        /// How many times this string was refused. A label redrawn every frame
        /// raises this rather than adding a row, for the same reason the capture
        /// session deduplicates.
        /// </summary>
        public int Marrat;
    }

    /// <summary>
    /// ربط المجموعة — the layout-group members mirroring needs, resolved once
    /// at startup and held as delegates.
    /// </summary>
    /// <remarks>
    /// <para>
    /// Everything here is reflected rather than named, and not for the reason
    /// Mushtarak reflects. <c>RectTransform</c>, <c>Vector2</c> and
    /// <c>RectOffset</c> are engine types this assembly is allowed to name and
    /// does name; <c>UnityEngine.UI.LayoutGroup</c> and
    /// <c>HorizontalLayoutGroup</c> are uGUI, which is a package a game may
    /// simply not have — a title built entirely on TextMeshPro over a bare
    /// canvas, or on NGUI, has no uGUI assembly at all. A compile-time
    /// reference would make the plugin fail to load in those games over a
    /// feature they do not use.
    /// </para>
    /// <para>
    /// <c>reverseArrangement</c> is the other reason. It arrived in Unity
    /// 2020.1; a game on 2019 has a <c>HorizontalLayoutGroup</c> without it.
    /// Binding it optionally means that game gets the sibling-order fallback
    /// and one log line, instead of no mirroring at all.
    /// </para>
    /// </remarks>
    public sealed class RabtMajmua
    {
        private RabtMajmua()
        {
            NawMajmua = Rabt.Naw("UnityEngine.UI.LayoutGroup");
            NawUfuqi = Rabt.Naw("UnityEngine.UI.HorizontalLayoutGroup");
            Type? nawHashw = Rabt.Naw("UnityEngine.RectOffset");

            QariAks = Rabt.Qari<bool>(NawUfuqi, "reverseArrangement");
            KatibAks = Rabt.Katib<bool>(NawUfuqi, "reverseArrangement");
            QariHashw = Rabt.Qari<object>(NawMajmua, "padding");
            AmrIttisakh = Rabt.Amr(NawMajmua, "SetDirty");

            QariYasar = Rabt.Qari<int>(nawHashw, "left");
            KatibYasar = Rabt.Katib<int>(nawHashw, "left");
            QariYameen = Rabt.Qari<int>(nawHashw, "right");
            KatibYameen = Rabt.Katib<int>(nawHashw, "right");
        }

        /// <summary>
        /// <c>UnityEngine.UI.LayoutGroup</c>, or <c>null</c> when this game has
        /// no uGUI.
        /// </summary>
        public Type? NawMajmua { get; }

        /// <summary><c>UnityEngine.UI.HorizontalLayoutGroup</c>, or <c>null</c>.</summary>
        public Type? NawUfuqi { get; }

        /// <summary>Reads a horizontal group's arrangement direction.</summary>
        public Func<object, bool>? QariAks { get; }

        /// <summary>Writes it.</summary>
        public Action<object, bool>? KatibAks { get; }

        /// <summary>
        /// Reads a layout group's padding. Typed as <see cref="object"/> because
        /// the property's own type is <c>RectOffset</c>, and a delegate bound to
        /// return the base type is the one direction the runtime permits — the
        /// setter is deliberately not bound for the mirror of that reason, and
        /// is not needed: <c>RectOffset</c> is a reference type, so its fields
        /// are edited in place and <see cref="AmrIttisakh"/> tells the layout
        /// system that they changed.
        /// </summary>
        public Func<object, object>? QariHashw { get; }

        /// <summary>
        /// <c>LayoutGroup.SetDirty</c> — the notification that makes an
        /// in-place padding edit take effect. Without it the numbers change and
        /// the screen does not, until something else happens to dirty the same
        /// layout, which reads as mirroring that works intermittently.
        /// </summary>
        public Action<object>? AmrIttisakh { get; }

        /// <summary>Reads a <c>RectOffset</c>'s left inset.</summary>
        public Func<object, int>? QariYasar { get; }

        /// <summary>Writes it.</summary>
        public Action<object, int>? KatibYasar { get; }

        /// <summary>Reads a <c>RectOffset</c>'s right inset.</summary>
        public Func<object, int>? QariYameen { get; }

        /// <summary>Writes it.</summary>
        public Action<object, int>? KatibYameen { get; }

        /// <summary>Whether this game has uGUI layout groups at all.</summary>
        public bool LahuMajmuat => NawMajmua is not null;

        /// <summary>
        /// Whether a horizontal group's arrangement can be reversed directly,
        /// rather than through the sibling-order fallback.
        /// </summary>
        public bool YaqlibTarteeb => QariAks is not null && KatibAks is not null;

        /// <summary>Whether directional padding can be swapped.</summary>
        public bool YubadilHashw =>
            QariHashw is not null && QariYasar is not null && KatibYasar is not null
            && QariYameen is not null && KatibYameen is not null;

        /// <summary>
        /// Resolves the binding once, reporting each capability that is absent
        /// and what its absence costs. Always returns an instance: mirroring an
        /// element's own anchors works with none of these bound, and a game
        /// without uGUI still gets that half.
        /// </summary>
        /// <returns>The binding.</returns>
        public static RabtMajmua Iqran()
        {
            RabtMajmua rabt = new RabtMajmua();
            if (!rabt.LahuMajmuat)
            {
                // Not a degradation worth a line: a game with no uGUI has no
                // layout groups to reverse, and saying so on every launch would
                // train people to ignore the log.
                return rabt;
            }
            if (!rabt.YaqlibTarteeb)
            {
                Rabt.Ballagh(
                    "HorizontalLayoutGroup has no reverseArrangement in this build; "
                    + "horizontal groups are mirrored by reversing their children's sibling "
                    + "order instead, which is exact but also reorders anything else that "
                    + "reads sibling index.");
            }
            if (!rabt.YubadilHashw)
            {
                Rabt.Ballagh(
                    "LayoutGroup.padding or RectOffset.left/right did not resolve; directional "
                    + "padding is not mirrored, so a group padded on one side keeps that side "
                    + "in a right-to-left interface. Nothing else is affected.");
            }
            return rabt;
        }
    }

    /// <summary>
    /// سجل التراجع — one element's original geometry, captured before it was
    /// first touched, so turning the patch off puts it back exactly.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The values here are captured once and are never written again. That is
    /// the whole point of the type: a record that re-captured on every
    /// adjustment would, from the second adjustment onward, be recording the
    /// adjusted state as the original, and undo would restore an element to a
    /// position the patch put it in.
    /// </para>
    /// <para>
    /// It holds a strong reference to the <see cref="RectTransform"/>, which is
    /// deliberate and bounded: <see cref="Tawseea"/> keys its table on the
    /// instance identifier rather than on the object, so a scene unload that
    /// destroys the element leaves a record whose target compares equal to
    /// <c>null</c> through Unity's own operator, which
    /// <see cref="Tarajaa"/> detects and discards. Keying the table on the
    /// object itself would instead keep every destroyed element's managed shell
    /// alive for the lifetime of the process.
    /// </para>
    /// </remarks>
    public sealed class SijillTarajua
    {
        private bool munfadh;

        /// <summary>
        /// Captures one element's geometry. Called once per element, by
        /// <see cref="Tawseea"/>, before the first change.
        /// </summary>
        /// <param name="mustatil">The element.</param>
        /// <exception cref="ArgumentNullException"><paramref name="mustatil"/> is null.</exception>
        public SijillTarajua(RectTransform mustatil)
        {
            Mustatil = mustatil ?? throw new ArgumentNullException(nameof(mustatil));
            Muarrif = mustatil.GetInstanceID();
            HajmAsli = mustatil.sizeDelta;
            MawqiAsli = mustatil.anchoredPosition;
            TathbitAdnaAsli = mustatil.anchorMin;
            TathbitAqsaAsli = mustatil.anchorMax;
            MihwarAsli = mustatil.pivot;
        }

        /// <summary>The element this record belongs to.</summary>
        public RectTransform Mustatil { get; }

        /// <summary>
        /// Its instance identifier, captured at the same moment. Read after the
        /// object is destroyed, when calling <c>GetInstanceID</c> would throw.
        /// </summary>
        public int Muarrif { get; }

        /// <summary>The original <c>sizeDelta</c>.</summary>
        public Vector2 HajmAsli { get; }

        /// <summary>The original <c>anchoredPosition</c>.</summary>
        public Vector2 MawqiAsli { get; }

        /// <summary>The original <c>anchorMin</c>.</summary>
        public Vector2 TathbitAdnaAsli { get; }

        /// <summary>The original <c>anchorMax</c>.</summary>
        public Vector2 TathbitAqsaAsli { get; }

        /// <summary>The original <c>pivot</c>.</summary>
        public Vector2 MihwarAsli { get; }

        /// <summary>What has been changed since, per <see cref="AlamatTarajua"/>.</summary>
        public uint Alam { get; internal set; }

        /// <summary>
        /// The layout group whose padding was swapped, when one was — the
        /// <c>LayoutGroup</c> instance, of whatever concrete kind.
        /// </summary>
        public object? Majmua { get; internal set; }

        /// <summary>
        /// The horizontal layout group whose arrangement was reversed, when one
        /// was.
        /// </summary>
        /// <remarks>
        /// Held separately from <see cref="Majmua"/> even though the two are
        /// the same object whenever both are set. <c>reverseArrangement</c> is
        /// declared on <c>HorizontalLayoutGroup</c> and the delegate bound to
        /// it casts its target to that type, while <c>padding</c> and
        /// <c>SetDirty</c> are declared on the base — so one field serving both
        /// would be an invariant held by nothing but this comment, and the
        /// first vertical group to acquire a reversal would throw an
        /// InvalidCastException inside undo.
        /// </remarks>
        public object? MajmuaUfuqiya { get; internal set; }

        /// <summary>
        /// The element's child count at the moment its children were reversed.
        /// Undo compares against it: reversal is exactly its own inverse only
        /// while the count is unchanged, and re-reversing a group the game has
        /// since added a child to would scramble it rather than restore it.
        /// </summary>
        public int AdadAbna { get; internal set; }

        /// <summary>Whether this record has already been spent.</summary>
        public bool Munfadh => munfadh;

        /// <summary>
        /// Puts the element back exactly as it was, restoring only what was
        /// changed.
        /// </summary>
        /// <param name="rabt">
        /// The layout-group binding, for the group half of the restoration.
        /// </param>
        /// <returns>
        /// Whether anything was restored. <c>false</c> for a record that was
        /// already spent, and for an element the game has destroyed — a
        /// destroyed element needs no restoring, and writing to one throws.
        /// </returns>
        public bool Tarajaa(RabtMajmua? rabt)
        {
            if (munfadh)
            {
                return false;
            }
            munfadh = true;

            RectTransform mustatil = Mustatil;
            if (mustatil == null)
            {
                return false;
            }

            try
            {
                // Anchors before position and size: assigning anchorMin or
                // anchorMax recomputes sizeDelta and anchoredPosition against
                // the new anchoring, so restoring them in the other order would
                // have the anchor write overwrite the two values just restored.
                if ((Alam & AlamatTarajua.Tathbit) != 0)
                {
                    mustatil.anchorMin = TathbitAdnaAsli;
                    mustatil.anchorMax = TathbitAqsaAsli;
                }
                if ((Alam & AlamatTarajua.Mihwar) != 0)
                {
                    mustatil.pivot = MihwarAsli;
                }
                if ((Alam & AlamatTarajua.Hajm) != 0)
                {
                    mustatil.sizeDelta = HajmAsli;
                }
                if ((Alam & AlamatTarajua.Mawqi) != 0)
                {
                    mustatil.anchoredPosition = MawqiAsli;
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Tawseea: restoring an element's rectangle threw; that element "
                    + "keeps the geometry the patch gave it", khata);
            }

            TarajuaMajmua(rabt, mustatil);
            return true;
        }

        private void TarajuaMajmua(RabtMajmua? rabt, RectTransform mustatil)
        {
            if (rabt is null)
            {
                return;
            }
            try
            {
                object? majmua = Majmua;
                object? ufuqi = MajmuaUfuqiya;
                Func<object, bool>? qariAks = rabt.QariAks;
                Action<object, bool>? katibAks = rabt.KatibAks;
                if ((Alam & AlamatTarajua.AksTarteeb) != 0
                    && ufuqi is not null && qariAks is not null && katibAks is not null)
                {
                    katibAks(ufuqi, !qariAks(ufuqi));
                }
                if ((Alam & AlamatTarajua.AksAbna) != 0)
                {
                    if (mustatil.childCount == AdadAbna)
                    {
                        AksAbna(mustatil);
                    }
                    else
                    {
                        Rabt.Ballagh(
                            "Tawseea: a mirrored layout group's child count changed from "
                            + AdadAbna.ToString(System.Globalization.CultureInfo.InvariantCulture)
                            + " to "
                            + mustatil.childCount.ToString(
                                System.Globalization.CultureInfo.InvariantCulture)
                            + " while the patch was on; its children are left in their current "
                            + "order rather than re-reversed, because re-reversing a group that "
                            + "gained or lost a child scrambles it instead of restoring it.");
                    }
                }
                if ((Alam & AlamatTarajua.BadalHashw) != 0 && majmua is not null)
                {
                    BadilHashw(rabt, majmua);
                }
                if (majmua is not null && rabt.AmrIttisakh is not null
                    && (Alam & (AlamatTarajua.AksTarteeb | AlamatTarajua.BadalHashw)) != 0)
                {
                    rabt.AmrIttisakh(majmua);
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Tawseea: restoring a layout group threw; that group keeps the "
                    + "arrangement the patch gave it", khata);
            }
        }

        /// <summary>
        /// Reverses a transform's children by repeatedly moving the last one
        /// forward. Exactly its own inverse for a fixed child count, which is
        /// what lets undo re-run it instead of storing every index.
        /// </summary>
        internal static void AksAbna(Transform ab)
        {
            int adad = ab.childCount;
            for (int i = 0; i < adad; i++)
            {
                ab.GetChild(adad - 1).SetSiblingIndex(i);
            }
        }

        /// <summary>Swaps a layout group's left and right padding in place.</summary>
        internal static bool BadilHashw(RabtMajmua rabt, object majmua)
        {
            if (!rabt.YubadilHashw)
            {
                return false;
            }
            object? hashw = rabt.QariHashw!(majmua);
            if (hashw is null)
            {
                return false;
            }
            int yasar = rabt.QariYasar!(hashw);
            int yameen = rabt.QariYameen!(hashw);
            if (yasar == yameen)
            {
                return false;
            }
            rabt.KatibYasar!(hashw, yameen);
            rabt.KatibYameen!(hashw, yasar);
            return true;
        }
    }

    /// <summary>
    /// توسعة — the reflow and mirroring behaviour: compute the geometry,
    /// apply what the patch permits, record every change, and be able to put
    /// all of it back.
    /// </summary>
    /// <remarks>
    /// One instance per adapter, shared by every element it manages, because
    /// the undo table has to be shared: a patch is switched off once and every
    /// element it touched has to come back.
    /// </remarks>
    public sealed class Tawseea
    {
        /// <summary>
        /// The slack a measurement is allowed against its box, in element
        /// units. A line that exceeds its box by a hundredth of a unit has
        /// accumulated rounding, not overflowed, and growing a container for it
        /// would be a visible change for an invisible cause.
        /// </summary>
        public const float Samaha = 0.01f;

        /// <summary>
        /// How many distinct refused strings are kept. A game whose patch
        /// refuses ten thousand strings has a compilation problem, not a
        /// diagnostics problem, and a list that grew without bound inside it
        /// would turn that into a memory problem as well.
        /// </summary>
        public const int AqsaRafdat = 256;

        private readonly RabtMajmua rabt;
        private readonly Dictionary<int, SijillTarajua> sijillat;
        private readonly Dictionary<string, int> fahrasRafdat;
        private readonly List<SijillRafd> rafdat;
        private long rafdatMuhmala;

        /// <summary>
        /// Creates the behaviour over a layout-group binding.
        /// </summary>
        /// <param name="rabt">
        /// The binding, from <see cref="RabtMajmua.Iqran"/>. Passed in rather
        /// than resolved here so the whole namespace's reflection happens once,
        /// at startup, on the thread the plugin loads on.
        /// </param>
        /// <exception cref="ArgumentNullException"><paramref name="rabt"/> is null.</exception>
        public Tawseea(RabtMajmua rabt)
        {
            this.rabt = rabt ?? throw new ArgumentNullException(nameof(rabt));
            sijillat = new Dictionary<int, SijillTarajua>(64);
            fahrasRafdat = new Dictionary<string, int>(32, StringComparer.Ordinal);
            rafdat = new List<SijillRafd>(16);
        }

        /// <summary>How many elements this object is holding an undo record for.</summary>
        public int AdadSijillat => sijillat.Count;

        /// <summary>How many distinct strings have been refused a growth.</summary>
        public int AdadRafdat => rafdat.Count;

        /// <summary>
        /// How many refusals were dropped because the list was full. Non-zero
        /// means the report is a sample rather than a census, which a reader
        /// has to know.
        /// </summary>
        public long RafdatMuhmala => rafdatMuhmala;

        /// <summary>
        /// The geometry a string needs, as values. Pure: it reads no component,
        /// writes nothing, and allocates nothing.
        /// </summary>
        /// <remarks>
        /// <para>
        /// <b>Which way an element grows.</b> Changing <c>sizeDelta</c> grows a
        /// rectangle about its own pivot: a width increase of <c>d</c> moves the
        /// left edge by <c>-p.x * d</c> and the right edge by
        /// <c>(1 - p.x) * d</c>. Left alone, a centre-pivoted element therefore
        /// grows into its neighbours on both sides. What is wanted is for the
        /// leading edge — the right edge in a right-to-left interface, the left
        /// edge otherwise — to stay exactly where the designer put it while the
        /// element extends away from it, because the leading edge is where the
        /// eye starts and where the text begins. That is one translation:
        /// </para>
        /// <code>
        ///     right-to-left    IzahaS = -(1 - p.x) * d      the right edge holds
        ///     left-to-right    IzahaS =      p.x  * d       the left edge holds
        ///     height           IzahaA = -(1 - p.y) * dh     the top edge holds
        /// </code>
        /// <para>
        /// Height always holds the top edge, in both directions: text flows
        /// downward, so a paragraph that gained a line gained it at the bottom.
        /// </para>
        /// <para>
        /// <b>Why the shift is gated on re-anchoring.</b> Moving the element is
        /// a change to <c>anchoredPosition</c>, which is exactly the value a
        /// game's own positioning script writes. Where the constraint does not
        /// permit re-anchoring, the growth is taken about the pivot instead and
        /// <see cref="AlamatTawseea.RafdTathbit"/> records that it was.
        /// </para>
        /// </remarks>
        /// <param name="talab">What the text measured and what the box is.</param>
        /// <returns>The geometry, and what was refused.</returns>
        public static NatijatTawseea Ihsib(in TalabTawseea talab)
        {
            NatijatTawseea natija = default;
            natija.Ard = talab.ArdHali;
            natija.Irtifa = talab.IrtifaHali;

            float k = talab.Qiyas > 0f ? talab.Qiyas : 1f;
            float matlubArd = talab.ArdNass * k;
            float matlubIrtifa = talab.IrtifaNass * k;

            if (matlubArd > talab.ArdHali + Samaha)
            {
                if (talab.Qayd.YanmuArdan)
                {
                    natija.Ard = matlubArd;
                    natija.Alam |= AlamatTawseea.NamuArd;
                }
                else
                {
                    natija.Alam |= AlamatTawseea.RafdArd;
                    natija.ZaidArd = matlubArd - talab.ArdHali;
                }
            }

            if (matlubIrtifa > talab.IrtifaHali + Samaha)
            {
                if (talab.Qayd.YanmuIrtifaan)
                {
                    natija.Irtifa = matlubIrtifa;
                    natija.Alam |= AlamatTawseea.NamuIrtifa;
                }
                else
                {
                    natija.Alam |= AlamatTawseea.RafdIrtifa;
                    natija.ZaidIrtifa = matlubIrtifa - talab.IrtifaHali;
                }
            }

            float d = natija.Ard - talab.ArdHali;
            float dh = natija.Irtifa - talab.IrtifaHali;
            if (d != 0f || dh != 0f)
            {
                if (talab.Qayd.YuadTathbituh)
                {
                    natija.IzahaS = talab.Yameen
                        ? -(1f - talab.Mihwar.x) * d
                        : talab.Mihwar.x * d;
                    natija.IzahaA = -(1f - talab.Mihwar.y) * dh;
                    if (natija.IzahaS != 0f || natija.IzahaA != 0f)
                    {
                        natija.Alam |= AlamatTawseea.Izaha;
                    }
                }
                else
                {
                    natija.Alam |= AlamatTawseea.RafdTathbit;
                }
            }

            natija.Yalaim = natija.ZaidArd <= 0f && natija.ZaidIrtifa <= 0f;
            return natija;
        }

        /// <summary>
        /// Computes the geometry and applies whatever the constraint permits,
        /// recording the element's originals the first time it is touched.
        /// </summary>
        /// <param name="mustatil">The element to reflow.</param>
        /// <param name="talab">What the text measured and what the box is.</param>
        /// <param name="muarrif">
        /// The string, or a key identifying it, for the refusal list. May be
        /// <c>null</c>, in which case a refusal is still counted but not named —
        /// which is worth avoiding, because an unnamed refusal is a number a
        /// translator cannot act on.
        /// </param>
        /// <returns>
        /// The decision, including the residual overflow when growth was
        /// refused. The caller applies the overflow policy from that.
        /// </returns>
        public NatijatTawseea Wassi(
            RectTransform mustatil, in TalabTawseea talab, string? muarrif)
        {
            NatijatTawseea natija = Ihsib(in talab);
            if (mustatil == null)
            {
                return natija;
            }

            if (natija.Marfud)
            {
                SajjilRafd(muarrif, in talab, in natija);
            }
            if (!natija.Yughayyir)
            {
                return natija;
            }

            SijillTarajua sijill = Sajjil(mustatil);
            try
            {
                if ((natija.Alam & (AlamatTawseea.NamuArd | AlamatTawseea.NamuIrtifa)) != 0)
                {
                    Vector2 hajm = mustatil.sizeDelta;
                    // Only the axes that grew are written. A rectangle stretched
                    // on one axis carries a sizeDelta that is an offset from its
                    // parent rather than a size on that axis, and writing a
                    // measured height into it would resize the element by the
                    // parent's height on top of its own.
                    if ((natija.Alam & AlamatTawseea.NamuArd) != 0)
                    {
                        hajm.x += natija.Ard - talab.ArdHali;
                    }
                    if ((natija.Alam & AlamatTawseea.NamuIrtifa) != 0)
                    {
                        hajm.y += natija.Irtifa - talab.IrtifaHali;
                    }
                    mustatil.sizeDelta = hajm;
                    sijill.Alam |= AlamatTarajua.Hajm;
                }
                if ((natija.Alam & AlamatTawseea.Izaha) != 0)
                {
                    Vector2 mawqi = mustatil.anchoredPosition;
                    mawqi.x += natija.IzahaS;
                    mawqi.y += natija.IzahaA;
                    mustatil.anchoredPosition = mawqi;
                    sijill.Alam |= AlamatTarajua.Mawqi;
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Tawseea: applying a reflow to an element threw; that element "
                    + "keeps the geometry the game gave it", khata);
            }
            return natija;
        }

        /// <summary>
        /// Mirrors one element for a right-to-left interface: its anchors, its
        /// pivot and its offset where re-anchoring is permitted, and its layout
        /// group's arrangement and directional padding where the group exists.
        /// </summary>
        /// <remarks>
        /// <para>
        /// <b>The three writes are one operation and have to happen together.</b>
        /// Mirroring the anchors moves the frame the element is positioned
        /// inside; mirroring the pivot moves the point inside the element that
        /// the offset is measured from; negating <c>anchoredPosition.x</c> then
        /// carries the element the same distance in the other direction. Do any
        /// two of the three and the element lands somewhere neither the original
        /// nor the mirror puts it — most visibly, an element anchored to the
        /// left edge and offset twenty units inward ends up twenty units off the
        /// right edge of the screen.
        /// </para>
        /// <para>
        /// <b>Only the horizontal axis is touched.</b> Mirroring is a left-right
        /// operation; a right-to-left interface reads top to bottom exactly like
        /// a left-to-right one, and flipping Y would put a title bar at the
        /// bottom of its panel.
        /// </para>
        /// <para>
        /// <b>Vertical and grid groups.</b> A vertical group's order does not
        /// mirror, so only its padding is swapped. A <c>GridLayoutGroup</c>'s
        /// <c>startCorner</c> is not touched at all: it is a genuine mirroring
        /// axis this file does not handle, and a grid in a mirrored interface
        /// therefore keeps filling from the corner the game authored, which is
        /// wrong and is a smaller wrong than reordering an inventory.
        /// </para>
        /// </remarks>
        /// <param name="mustatil">The element to mirror.</param>
        /// <param name="qayd">The constraint carrying the permission bits.</param>
        /// <returns>Whether anything was changed.</returns>
        public bool Amrih(RectTransform mustatil, in MadkhalQayd qayd)
        {
            if (mustatil == null || !qayd.Yumrah)
            {
                return false;
            }

            SijillTarajua sijill = Sajjil(mustatil);
            bool ghayyar = false;

            if (qayd.YuadTathbituh)
            {
                try
                {
                    Vector2 adna = mustatil.anchorMin;
                    Vector2 aqsa = mustatil.anchorMax;
                    Vector2 jadeedAdna = adna;
                    Vector2 jadeedAqsa = aqsa;
                    jadeedAdna.x = 1f - aqsa.x;
                    jadeedAqsa.x = 1f - adna.x;
                    mustatil.anchorMin = jadeedAdna;
                    mustatil.anchorMax = jadeedAqsa;
                    sijill.Alam |= AlamatTarajua.Tathbit;

                    Vector2 mihwar = mustatil.pivot;
                    mihwar.x = 1f - mihwar.x;
                    mustatil.pivot = mihwar;
                    sijill.Alam |= AlamatTarajua.Mihwar;

                    Vector2 mawqi = mustatil.anchoredPosition;
                    mawqi.x = -mawqi.x;
                    mustatil.anchoredPosition = mawqi;
                    sijill.Alam |= AlamatTarajua.Mawqi;
                    ghayyar = true;
                }
                catch (Exception khata)
                {
                    Rabt.Ballagh("Tawseea: mirroring an element's anchoring threw; that "
                        + "element keeps the anchoring the game gave it", khata);
                }
            }

            return AmrihMajmua(mustatil, sijill) || ghayyar;
        }

        /// <summary>
        /// Puts one element back exactly as it was and forgets it.
        /// </summary>
        /// <param name="mustatil">The element.</param>
        /// <returns>Whether a record existed and was applied.</returns>
        public bool Tarajaa(RectTransform mustatil)
        {
            if (mustatil == null)
            {
                return false;
            }
            int muarrif = mustatil.GetInstanceID();
            if (!sijillat.TryGetValue(muarrif, out SijillTarajua sijill))
            {
                return false;
            }
            sijillat.Remove(muarrif);
            return sijill.Tarajaa(rabt);
        }

        /// <summary>
        /// Puts every touched element back and forgets all of them — what
        /// switching the patch off at run time calls.
        /// </summary>
        /// <returns>How many elements were restored.</returns>
        public int TarajuaKull()
        {
            int adad = 0;
            foreach (KeyValuePair<int, SijillTarajua> madkhal in sijillat)
            {
                if (madkhal.Value.Tarajaa(rabt))
                {
                    adad++;
                }
            }
            sijillat.Clear();
            return adad;
        }

        /// <summary>
        /// Drops the record for an element without restoring it — for an
        /// element the game destroyed, where there is nothing left to restore
        /// and the record is only holding its managed shell alive.
        /// </summary>
        /// <param name="muarrif">The instance identifier the record was made under.</param>
        /// <returns>Whether a record was dropped.</returns>
        public bool Ansa(int muarrif)
        {
            return sijillat.Remove(muarrif);
        }

        /// <summary>
        /// Drops every record whose element the game has since destroyed. A
        /// scene change is when this is worth calling: without it, one record
        /// per destroyed element accumulates for the life of the process.
        /// </summary>
        /// <returns>How many were dropped.</returns>
        public int Nazzif()
        {
            int adad = 0;
            List<int>? maytat = null;
            foreach (KeyValuePair<int, SijillTarajua> madkhal in sijillat)
            {
                if (madkhal.Value.Mustatil == null)
                {
                    maytat ??= new List<int>(16);
                    maytat.Add(madkhal.Key);
                }
            }
            if (maytat is not null)
            {
                for (int i = 0; i < maytat.Count; i++)
                {
                    if (sijillat.Remove(maytat[i]))
                    {
                        adad++;
                    }
                }
            }
            return adad;
        }

        /// <summary>One refused string.</summary>
        /// <param name="fahras">Its position in the list, below <see cref="AdadRafdat"/>.</param>
        /// <param name="sijill">The record.</param>
        /// <returns>Whether the position is in range.</returns>
        public bool JidRafd(int fahras, out SijillRafd sijill)
        {
            if ((uint)fahras >= (uint)rafdat.Count)
            {
                sijill = default;
                return false;
            }
            sijill = rafdat[fahras];
            return true;
        }

        /// <summary>
        /// Empties the refusal list — what a diagnostics console does after
        /// writing a report, so the next session's list describes the next
        /// session.
        /// </summary>
        public void AfrighRafdat()
        {
            rafdat.Clear();
            fahrasRafdat.Clear();
            rafdatMuhmala = 0;
        }

        /// <summary>
        /// The record for an element, capturing its originals the first time
        /// and returning the existing one every time after.
        /// </summary>
        /// <remarks>
        /// The second half of that sentence is the requirement. An element whose
        /// string changes twice is adjusted twice, and the second adjustment
        /// must not re-capture: the values it would capture are the ones the
        /// first adjustment wrote, and an undo built on them would restore the
        /// element to a state the patch created rather than to the state the
        /// game shipped. Three toggles of a patch that re-captured are three
        /// compounded adjustments and a layout that is visibly, unaccountably
        /// wrong.
        /// </remarks>
        private SijillTarajua Sajjil(RectTransform mustatil)
        {
            int muarrif = mustatil.GetInstanceID();
            if (sijillat.TryGetValue(muarrif, out SijillTarajua mawjud))
            {
                return mawjud;
            }
            SijillTarajua sijill = new SijillTarajua(mustatil);
            sijillat[muarrif] = sijill;
            return sijill;
        }

        /// <summary>
        /// Mirrors the layout group on an element, where it has one: the
        /// arrangement of a horizontal group, and the directional padding of any
        /// group.
        /// </summary>
        private bool AmrihMajmua(RectTransform mustatil, SijillTarajua sijill)
        {
            if (!rabt.LahuMajmuat)
            {
                return false;
            }
            bool ghayyar = false;
            try
            {
                // Compared with Unity's own equality operator before the value
                // is widened to object: a component on a destroyed GameObject
                // is a live managed reference that only that operator reports
                // as absent, and writing a property on one throws.
                Component? ufuqiKhaam = rabt.NawUfuqi is null
                    ? null
                    : mustatil.GetComponent(rabt.NawUfuqi);
                Component? majmuaKhaam = mustatil.GetComponent(rabt.NawMajmua!);
                if (majmuaKhaam == null)
                {
                    return false;
                }
                object majmua = majmuaKhaam;
                object? ufuqi = ufuqiKhaam == null ? null : ufuqiKhaam;
                sijill.Majmua = majmua;
                sijill.MajmuaUfuqiya = ufuqi;

                Func<object, bool>? qariAks = rabt.QariAks;
                Action<object, bool>? katibAks = rabt.KatibAks;
                if (ufuqi is not null)
                {
                    if (qariAks is not null && katibAks is not null)
                    {
                        katibAks(ufuqi, !qariAks(ufuqi));
                        sijill.Alam |= AlamatTarajua.AksTarteeb;
                        ghayyar = true;
                    }
                    else
                    {
                        SijillTarajua.AksAbna(mustatil);
                        sijill.AdadAbna = mustatil.childCount;
                        sijill.Alam |= AlamatTarajua.AksAbna;
                        ghayyar = true;
                    }
                }

                if (SijillTarajua.BadilHashw(rabt, majmua))
                {
                    sijill.Alam |= AlamatTarajua.BadalHashw;
                    ghayyar = true;
                }
                if (ghayyar && rabt.AmrIttisakh is not null)
                {
                    rabt.AmrIttisakh(majmua);
                }
            }
            catch (Exception khata)
            {
                Rabt.Ballagh("Tawseea: mirroring a layout group threw; that group keeps the "
                    + "arrangement the game gave it", khata);
            }
            return ghayyar;
        }

        /// <summary>
        /// Records that a string did not fit an element it may not grow, or
        /// raises the count on a string already recorded.
        /// </summary>
        private void SajjilRafd(
            string? muarrif, in TalabTawseea talab, in NatijatTawseea natija)
        {
            if (string.IsNullOrEmpty(muarrif))
            {
                // Counted, not named. A refusal with no identifier cannot be
                // acted on, and putting an empty row in the report would only
                // make the report look complete when it is not.
                rafdatMuhmala++;
                return;
            }

            string ism = muarrif!;
            if (fahrasRafdat.TryGetValue(ism, out int mawdi))
            {
                SijillRafd hali = rafdat[mawdi];
                hali.Marrat++;
                // The worst sighting wins. The same string can reach the same
                // element in a wider and a narrower state — a panel mid-tween,
                // a list before and after its scrollbar appears — and the report
                // has to describe the case that actually clipped.
                if (natija.ZaidArd > hali.ArdMatlub - hali.ArdMutah)
                {
                    hali.ArdMatlub = talab.ArdNass * (talab.Qiyas > 0f ? talab.Qiyas : 1f);
                    hali.ArdMutah = talab.ArdHali;
                }
                if (natija.ZaidIrtifa > hali.IrtifaMatlub - hali.IrtifaMutah)
                {
                    hali.IrtifaMatlub = talab.IrtifaNass * (talab.Qiyas > 0f ? talab.Qiyas : 1f);
                    hali.IrtifaMutah = talab.IrtifaHali;
                }
                hali.Alam |= natija.Alam
                    & (AlamatTawseea.RafdArd | AlamatTawseea.RafdIrtifa);
                rafdat[mawdi] = hali;
                return;
            }

            if (rafdat.Count >= AqsaRafdat)
            {
                rafdatMuhmala++;
                return;
            }

            float k = talab.Qiyas > 0f ? talab.Qiyas : 1f;
            SijillRafd sijill;
            sijill.Muarrif = ism;
            sijill.ArdMatlub = talab.ArdNass * k;
            sijill.ArdMutah = talab.ArdHali;
            sijill.IrtifaMatlub = talab.IrtifaNass * k;
            sijill.IrtifaMutah = talab.IrtifaHali;
            sijill.Alam = natija.Alam & (AlamatTawseea.RafdArd | AlamatTawseea.RafdIrtifa);
            sijill.Marrat = 1;
            fahrasRafdat[ism] = rafdat.Count;
            rafdat.Add(sijill);
        }
    }
}
