// مزود — the database, as the scanning rung consumes it.
//
// Qaida knows about entries, version ranges, architectures, platforms and
// specificity. Masah knows about modules, executable sections and the addresses
// inside them. Neither should have to know the other's vocabulary, and this file
// is the twenty lines that keep it that way: it answers Masah's question — "what
// patterns should I try for this key, best first?" — by asking Qaida's — "which
// entries match this engine version, this package version, this architecture and
// this platform, ordered by how specifically they say so?".
//
// WHY THE ENGINE AND PACKAGE VERSIONS ARE FIXED AT CONSTRUCTION. They are
// properties of the process, not of the target: the game that is running was
// built with one Unity version and ships one TextMeshPro. Passing them per
// lookup would let two targets in one process resolve against two different
// engine versions, which is not a situation that exists and is a bug that could
// not be reproduced.
//
// WHAT THIS FILE REFUSES TO DO. It does not scan, it does not validate an
// address, and it does not decide that a candidate is good. Those belong to
// Namat, which owns the search and its uniqueness rule, and to Masah, which owns
// the ranges. A provider that also scanned would be a third place the rules
// lived.

using System;
using System.Collections.Generic;
using Taarib.Unity.Il2cpp.Hall;

namespace Taarib.Unity.Il2cpp.Basmat
{
    /// <summary>
    /// The signature database presented as the scanning rung's provider.
    /// </summary>
    /// <remarks>
    /// Build one at startup from a loaded <see cref="Qaida"/> and the versions
    /// this process is running, and hand it to <see cref="Masah"/>. Every lookup
    /// is a dictionary probe and a sort over a handful of candidates; nothing
    /// here is on a frame path, and every target is resolved once.
    /// </remarks>
    public sealed class Mazwad : IMazwadNamat
    {
        private readonly Qaida qaida;
        private readonly Isdar muharrik;
        private readonly Isdar nusus;
        private readonly Binya binya;
        private readonly Minassa minassa;
        private readonly Dictionary<string, SijillBasma> sijillat =
            new Dictionary<string, SijillBasma>(StringComparer.Ordinal);

        /// <summary>
        /// Binds a loaded database to the versions this process is running.
        /// </summary>
        /// <param name="qaida">The loaded database.</param>
        /// <param name="muharrik">The Unity version this game was built with.</param>
        /// <param name="nusus">The text package version it ships.</param>
        /// <param name="binya">The architecture this process is executing as.</param>
        /// <param name="minassa">The platform it is running on.</param>
        /// <exception cref="ArgumentNullException"><paramref name="qaida"/> is null.</exception>
        public Mazwad(Qaida qaida, Isdar muharrik, Isdar nusus, Binya binya, Minassa minassa)
        {
            this.qaida = qaida ?? throw new ArgumentNullException(nameof(qaida));
            this.muharrik = muharrik;
            this.nusus = nusus;
            this.binya = binya;
            this.minassa = minassa;
        }

        /// <summary>
        /// Which database record produced each resolved target, by the target's
        /// qualified name.
        /// </summary>
        /// <remarks>
        /// Written when a candidate is offered rather than when it matches,
        /// because this object never learns whether a candidate matched — Masah
        /// does, and pairs its own match record against this one when the
        /// diagnostics bundle is written. Keeping the two separate is what lets
        /// a bundle say "the database offered three variants and the second one
        /// matched" rather than only naming the winner.
        /// </remarks>
        public IReadOnlyDictionary<string, SijillBasma> Sijillat => sijillat;

        /// <inheritdoc/>
        public bool Mutah(out string sabab)
        {
            return qaida.Mutah(out sabab);
        }

        /// <inheritdoc/>
        public IReadOnlyList<BasmaMurashaha> Murashahat(string miftahBasma)
        {
            if (string.IsNullOrEmpty(miftahBasma))
            {
                return Array.Empty<BasmaMurashaha>();
            }

            IReadOnlyList<MurashshahBasma> khaam =
                qaida.Bahth(miftahBasma, in muharrik, in nusus, binya, minassa);
            if (khaam.Count == 0)
            {
                return Array.Empty<BasmaMurashaha>();
            }

            BasmaMurashaha[] natija = new BasmaMurashaha[khaam.Count];
            for (int i = 0; i < khaam.Count; i++)
            {
                MurashshahBasma wahid = khaam[i];
                string ism = wahid.Sijill.Wasf();
                natija[i] = new BasmaMurashaha(ism, wahid.Namat, wahid.Wahda);
                sijillat[ism] = wahid.Sijill;
            }
            return natija;
        }

        /// <summary>
        /// The database's own report, for the diagnostics bundle.
        /// </summary>
        /// <returns>One line per entry loaded, plus the schema and file it came from.</returns>
        /// <remarks>
        /// Allocates, and is meant to: it runs once, when a bundle is written.
        /// </remarks>
        public IReadOnlyList<string> Taqreer()
        {
            return qaida.Taqreer();
        }
    }
}
