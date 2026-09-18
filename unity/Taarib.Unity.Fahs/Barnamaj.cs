// برنامج — the runner.
//
// One line per case, first failure decides the exit code, no test framework.
// That is not minimalism for its own sake: these checks have to be runnable on
// the machine that builds the plugins, and that machine has a .NET SDK and no
// package feed. A framework here would mean a restore, and a check that cannot
// be run is a check nobody runs.

using System;
using System.Collections.Generic;

namespace Taarib.Unity.Fahs
{
    /// <summary>The entry point.</summary>
    public static class Barnamaj
    {
        /// <summary>Runs every case and reports.</summary>
        /// <returns>Zero when every case passed, one otherwise.</returns>
        public static int Main()
        {
            List<Halat> halat = new List<Halat>();
            FahsLawn.Sajjil(halat);
            FahsSafha.Sajjil(halat);
            FahsRafa.Sajjil(halat);

            int najah = 0;
            int fashal = 0;
            for (int i = 0; i < halat.Count; i++)
            {
                Halat hala = halat[i];
                string? sabab = Jarrib(hala);
                if (sabab is null)
                {
                    najah++;
                    Console.WriteLine("ok   " + hala.Ism);
                }
                else
                {
                    fashal++;
                    Console.WriteLine("FAIL " + hala.Ism + " — " + sabab);
                }
            }

            Console.WriteLine();
            Console.WriteLine(
                fashal == 0
                    ? $"{najah} case(s) passed."
                    : $"{najah} case(s) passed, {fashal} failed.");

            // Printed rather than asserted, because the number a person wants
            // from this run is how much a frame costs and an assertion only
            // shows it when it is wrong.
            Console.WriteLine();
            Console.WriteLine(FahsRafa.Qiyas());
            return fashal == 0 ? 0 : 1;
        }

        /// <summary>
        /// Runs one case. A case that throws is a failure with the exception's
        /// own sentence, rather than a stack trace that ends the run and takes
        /// every case after it with it.
        /// </summary>
        private static string? Jarrib(Halat hala)
        {
            try
            {
                return hala.Tanfidh();
            }
            catch (Exception khata)
            {
                return "threw " + khata.GetType().Name + ": " + khata.Message;
            }
        }
    }

    /// <summary>One case: a name and a body that answers why it failed.</summary>
    public sealed class Halat
    {
        /// <summary>Records one case.</summary>
        /// <param name="ism">Its name, printed as written.</param>
        /// <param name="tanfidh">Its body; <c>null</c> means it passed.</param>
        /// <exception cref="ArgumentNullException">Either argument is null.</exception>
        public Halat(string ism, Func<string?> tanfidh)
        {
            Ism = ism ?? throw new ArgumentNullException(nameof(ism));
            Tanfidh = tanfidh ?? throw new ArgumentNullException(nameof(tanfidh));
        }

        /// <summary>The case's name.</summary>
        public string Ism { get; }

        /// <summary>The case's body.</summary>
        public Func<string?> Tanfidh { get; }
    }
}
