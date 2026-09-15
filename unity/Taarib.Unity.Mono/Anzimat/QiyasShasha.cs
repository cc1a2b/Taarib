// قياس الشاشة — how many screen pixels one layout unit of a text component
// covers, on the Mono backend.
//
// Every text system in this plugin lays a string out in the units its
// component measures in, and then asks the atlas for a bitmap. Those two
// things are only the same number on an unscaled screen-space-overlay canvas.
// On a canvas with a scaler one layout unit is more than one pixel; on a
// world-space component — a monitor in a room, a sign on a wall, a floating
// label — the component's size field is in world units, and a heading that
// declares a size of eight can cover a third of the screen. Rasterizing that
// at eight pixels and letting the GPU magnify it is the difference between
// Arabic a player can read and a smear.
//
// So this file answers one question, the same way for every text system:
// project one layout unit of the component's own up axis into screen space and
// measure it. Nasij.HajmLawhaMulaim turns the answer into a rasterization
// size, and Nasij.Ibni scales the resulting glyph rectangles back down by
// Hajm / HajmLawha, so the geometry is untouched and only the sampled bitmap
// changes.
//
// It deliberately measures rather than classifies. Asking "is this a canvas,
// and which render mode" and then applying a formula per mode is the version
// that breaks on the sixth arrangement somebody ships: a canvas nested under a
// scaled parent, a camera-space canvas with a plane distance, a world-space
// canvas on a moving object. Projection handles all of them because it is what
// the renderer itself does.

using System;
using UnityEngine;

namespace Taarib.Unity.Mono.Anzimat
{
    /// <summary>
    /// Screen pixels per layout unit, for text components.
    /// </summary>
    internal static class QiyasShasha
    {
        /// <summary>
        /// The answer when nothing can be measured: one unit, one pixel, which
        /// is the arrangement every interface label has always been drawn under
        /// and the one value that cannot make anything worse.
        /// </summary>
        public const float Muhayad = 1f;

        /// <summary>
        /// The largest factor that is reported rather than treated as a
        /// degenerate transform. Well past any real arrangement, and small
        /// enough that the product of it and a size stays finite.
        /// </summary>
        private const float Saqf = 4096f;

        private static Camera? kamiraMakhbua;
        private static int itarKamira = -1;

        /// <summary>
        /// How many screen pixels one layout unit of this component covers.
        /// </summary>
        /// <param name="juz">
        /// The text component, which every text system holds as an untyped
        /// reference because the type belongs to the game's own assemblies.
        /// Anything that is not a Unity component — a FairyGUI text field, for
        /// one — has no transform to project and reports the neutral factor.
        /// </param>
        /// <returns>
        /// A factor above zero, or <see cref="Muhayad"/> when the component is
        /// behind the camera, degenerate, destroyed, not a Unity component, or
        /// has no camera to be projected through.
        /// </returns>
        public static float BikselLilWahda(object? juz)
        {
            Transform? tahwil = (juz as Component)?.transform
                ?? (juz as GameObject)?.transform;
            if (tahwil is null || tahwil == null)
            {
                return Muhayad;
            }

            try
            {
                return Qis(tahwil);
            }
            catch (MissingReferenceException)
            {
                // The component was destroyed between the takeover deciding to
                // draw it and this measurement. Neutral, so the string draws
                // exactly as it did before this file existed.
                return Muhayad;
            }
            catch (NullReferenceException)
            {
                return Muhayad;
            }
        }

        private static float Qis(Transform tahwil)
        {
            // The component's own up axis, one local unit long, carried into
            // world space with every scale and rotation between here and the
            // root already applied. The up axis rather than a magnitude of
            // lossyScale because a text component's size field measures
            // vertically, and a non-uniformly scaled parent stretches the two
            // axes differently.
            Vector3 wahda = tahwil.TransformVector(new Vector3(0f, 1f, 0f));
            float tul = wahda.magnitude;
            if (!(tul > 0f) || float.IsNaN(tul) || float.IsInfinity(tul))
            {
                return Muhayad;
            }

            Canvas? lawh = tahwil.GetComponentInParent<Canvas>();
            if (lawh is not null && lawh != null)
            {
                if (lawh.renderMode == RenderMode.ScreenSpaceOverlay)
                {
                    // An overlay canvas is drawn straight into screen space
                    // with no camera at all: its world units are already
                    // pixels, and the scaler expresses itself as the canvas
                    // root's scale, which is exactly what TransformVector just
                    // accumulated.
                    return Mahdud(tul);
                }
            }

            Camera? kamira = KamiraLawh(lawh) ?? KamiraRaisiya();
            if (kamira is null)
            {
                return Mahdud(tul);
            }

            Vector3 asl = tahwil.position;
            Vector3 bidaya = kamira.WorldToScreenPoint(asl);
            Vector3 nihaya = kamira.WorldToScreenPoint(asl + wahda);
            // Behind the near plane the projection is a reflection, not a
            // measurement: the returned pixels are on the wrong side of the
            // screen and their distance means nothing.
            if (!(bidaya.z > 0f) || !(nihaya.z > 0f))
            {
                return Mahdud(tul);
            }

            float ds = nihaya.x - bidaya.x;
            float da = nihaya.y - bidaya.y;
            return Mahdud(Mathf.Sqrt((ds * ds) + (da * da)));
        }

        /// <summary>The camera a non-overlay canvas is rendered by, if it names one.</summary>
        private static Camera? KamiraLawh(Canvas? lawh)
        {
            if (lawh is null || lawh == null)
            {
                return null;
            }
            Camera? kamira = lawh.worldCamera;
            return kamira is not null && kamira != null ? kamira : null;
        }

        /// <summary>
        /// The scene's main camera, looked up at most once a frame.
        /// </summary>
        /// <remarks>
        /// <c>Camera.main</c> is a tagged-object search on the Unity versions
        /// this plugin still has to run under, and text can regenerate many
        /// times in one frame. One lookup per frame, and the cache is dropped
        /// the moment the camera it holds has been destroyed — which is what
        /// every scene change does to it.
        /// </remarks>
        private static Camera? KamiraRaisiya()
        {
            Camera? makhbua = kamiraMakhbua;
            if (itarKamira == Time.frameCount && makhbua is not null && makhbua != null)
            {
                return makhbua;
            }

            itarKamira = Time.frameCount;
            Camera? raisiya = Camera.main;
            kamiraMakhbua = raisiya is not null && raisiya != null ? raisiya : null;
            return kamiraMakhbua;
        }

        /// <summary>
        /// Keeps a measured factor inside the range the atlas ladder can serve,
        /// so a degenerate transform cannot ask for a rung that does not exist.
        /// </summary>
        private static float Mahdud(float biksel)
        {
            if (float.IsNaN(biksel) || !(biksel > 0f))
            {
                return Muhayad;
            }
            return biksel > Saqf ? Saqf : biksel;
        }
    }
}
