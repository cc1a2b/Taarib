# Security policy

Taarib modifies files inside games other people paid for, loads code into game processes it
does not own, unpacks archives it did not create, and verifies signatures against a key
compiled into the binary. Those four sentences are what makes this document necessary rather
than decorative.

## Reporting a vulnerability

**Do not open a public issue.**

Use **GitHub's private vulnerability reporting** on this repository — the Security tab,
"Report a vulnerability". That is the preferred channel: it keeps the report private, it
keeps the thread attached to the repository, and it is the one route that does not depend on
a mail provider deciding a message with an attached exploit is spam.

If that is unavailable to you, email `renhusa9@gmail.com` with `[taarib security]` in the
subject. Say only that you have a security report and roughly what area it touches; details
can move to a private channel once contact is established. Do not attach a working exploit to
a first email.

What helps, in rough order:

- **What an attacker gains, concretely.** "Arbitrary file write outside the game directory"
  is a report; "unsafe path handling" is a lead.
- **Which build.** The version string and the trust anchor it was compiled against — the
  library screen's status strip shows `tatwir` or `isdar`, and the diagnostics screen writes
  both into the maintenance bundle. A defect that only exists in a development build is a
  different defect.
- **The smallest reproduction you can manage.** A crafted `.ruqaa`, a crafted registry shard,
  a crafted archive, a crafted compatibility recipe. Attach it, or describe how to build it.
- **What the victim has to have done first.** Whether the attack needs them to have already
  installed something changes the severity by a lot.
- **The platform**, and for anything in the injection or overlay path, the game and its
  launcher.

### What to expect back

This is one person, working on this in their own time, in one timezone. There is no bounty,
no security team, no rotation, and no service-level agreement, and inventing one here would
just be a number nobody is accountable to.

What is actually promised: your report will be read and you will get a reply that is written
by a person rather than generated. If a week goes by with nothing, assume the message did not
arrive — mail from a stranger with a proof-of-concept attached is exactly what a spam filter
is built to eat — and send it again through the other channel. That is not a brush-off; it is
the realistic failure mode of a single-maintainer inbox, and knowing it in advance is worth
more than a promised response time that is not backed by anything.

If it is a real issue, you will be told what the fix is and roughly when it lands, and you
will be credited in the release notes unless you ask not to be.

## What is in scope

Anything that lets an attacker reach past a boundary the product is built around. In rough
order of severity.

### Defeating package signature verification

Every installable patch is an Ed25519-signed `.ruqaa`, and the client is supposed to refuse
anything unsigned, mismatched, or revoked — not by configuration, not by a build flag, not by
a developer mode. A way around that check is the top-severity report in this project.

The anchor it checks against is compiled in, and this is the part worth understanding before
you report against it. `taarib_khatm::MIRSAT_MALIK` is a constant resolved at compile time:
with the environment variable `TAARIB_MIFTAH_ISDAR` set, it is the release key and its
identity is `isdar`; with the variable absent it falls back to `MIFTAH_TATWIR`, the
development key committed in the tree, and its identity is `tatwir`.
`crates/taarib-aman/src/tahaqquq_tawqee.rs` then refuses a package whose signing key is not the
anchor, and refuses the development key by name when the anchor is a release one.

In scope: anything that makes a release build accept a package it should not — a signature
check that can be skipped, an anchor that can be substituted at runtime, a verification path
that returns before it compares, a `.ruqaa` whose signed region does not cover a byte the
installer later acts on, a downgrade or replay that reinstates a revoked patch.

### Getting bytes executed on a user's machine

Through a patch, a registry record, an archive, the update channel, or a staged component.
Anything that turns downloaded data into code.

### Escaping quarantine

Downloaded content is unpacked with path traversal, device names, symlink escapes,
case-collision attacks and decompression bombs all rejected before a byte is written
(`crates/taarib-aman/src/sandooq_fak.rs`). A path that gets a file outside the quarantine root
is in scope.

### Escaping a fingerprint recipe

A compatibility recipe arrives inside a downloaded package and is a list of paths that will
cause the user's computer to open files. It is validated on construction *and* on
deserialization, through the same constructor, so that no stored document can point outside
the game directory. A recipe that reads `~/.ssh/id_ed25519` would turn the installer into a
read primitive driven by a stranger; if you find one, that is the report to send.

### The registry client

`taarib-mustawda` fetches sharded JSON and release assets over a forge's raw-content endpoint,
with a mirror, a bundled offline copy and a LAN share behind it. Index content is supposed to
become readable only by hashing against the manifest's declared entry first — there is meant
to be no constructor that produces a verified shard without that check, so unverified index
content cannot be represented, let alone parsed.

In scope: a route to a parsed shard that skipped the hash; a mirror, LAN share or imported
file that is trusted more than the primary source; a manifest that can be rolled back to
re-offer a withdrawn patch; a shard that can make the client request or write a path it
should not; a response that can exhaust memory or disk before it is rejected.

### Install and restore

Every modified file is backed up byte-exact before it is touched, with the manifest written
before the change it describes. A sequence that leaves a game unrecoverable is in scope,
including interruption, crash and power-loss paths, and including a restore that writes
somewhere the backup manifest did not name.

### The injection surface

`taarib-mudkhal`, `taarib-haqn`, `taarib-tabaqa`, `taarib-jisr`, the overlay, and the engine
adapters. This code is designed to load into a process that is not ours, in a binary the user
paid for, hook its present chain, and draw over its frames.

In scope: memory-safety failures in anything on that path; a hook that can be redirected by
the loaded content rather than by the loader; a payload load path that can be pointed at a
library the signature check did not cover; a `version.dll` proxy that can be made to forward
to something other than the system library; privilege gained during injection.

Worth stating with the rest: **no adapter has ever been observed running inside a real game
process.** `docs/tashghil.md` is the evidence, engine by engine, of where each chain stops.
The injection surface is real code that compiles and is reachable, and it has had far less
adversarial exposure than the desktop paths — which cuts both ways for a reporter. Bugs there
are likely and welcome; they are also, today, mostly not exploitable by a remote attacker,
because the path does not run.

### Getting a game asset into a package

The provenance gate is supposed to make this structurally impossible — see the Legal position
section of the README. A route around it is both a security issue and a legal one.

### Reaching the network from a place that should not

`taarib-saff` contains no `std::fs` and no `std::net`; the adapters do not talk to the network
at all — the one `XMLHttpRequest` in the tree, in the RPG Maker plugin, reads the game's own
files over `file://`, which is how those games load everything. A path that reaches a host is a
bug regardless of what it fetches.

## What is not in scope

- **The anti-cheat refusal being bypassable by the user's own deliberate action.** Taarib
  detects anti-cheat and refuses to install into a protected game. That refusal is a safety
  mechanism *for* the user, not a DRM measure *against* them, and a user who works around it
  on their own machine is not an attacker. Report a hole in the detection as a bug, because a
  game wrongly cleared is genuinely bad — a user can lose an account over it. It is not a
  vulnerability report.
- **Attacks that require the attacker to already control the user's machine.** If the premise
  is local code execution as the same user, the interesting part is what it reaches next, and
  that is what the report should be about.
- **The development signing key being public.** `MIFTAH_TATWIR` is committed on purpose,
  clearly labelled, and a release build refuses signatures made with it by name. A build you
  compiled yourself trusts it — that is what a development build is for.
- **Reports that a community translation is wrong, offensive, or low quality.** That is the
  review queue's job. Use an issue.
- **Vulnerabilities in a game.** Report those to the game's publisher.
- **Dependency advisories with no path to exploitation here.** `cargo deny check` already
  runs. If you have found one it misses, that is welcome, but say how it is reachable.
- **Missing hardening that is not reachable** — a compiler flag, a header, a permission that
  could be narrower — unless you can name what it would have stopped. These are useful and
  belong in an issue; they are not embargo-worthy.

## Known limitations, stated up front

An honest policy names what is already known rather than waiting to be told.

- **macOS builds are unsigned and unnotarized.** No Apple Developer account exists yet, so
  Gatekeeper refuses the first launch and the documented workaround is to clear the quarantine
  attribute manually. Recorded in `docs/tawzee/macos.md` rather than hidden.
- **In portable mode, the contributor identity key is still minted into the machine
  keychain.** Portable mode is supposed to forbid every machine-global write; this is a known
  exception, recorded in `docs/tawzee/adhonat.md`.
- **`core:webview:allow-internal-toggle-devtools` cannot be withdrawn** at the pinned Tauri
  version. The workspace does not enable Tauri's `devtools` feature, so there is no inspector
  to open, but the capability remains declarable.
- **Stock SteamOS provides no Secret Service.** On a Steam Deck there may be no
  `org.freedesktop.secrets` provider, which changes where key material can live.
  `docs/tawzee/steamdeck.md` covers what that means.
- **The one-button pipeline verifies against a run-local anchor.** `tahaqquq` in
  `crates/taarib-aman/src/tahaqquq_tawqee.rs` takes the anchor as an argument, and the
  one-button install verifies the package it just built against the key that just signed it,
  rather than against the compiled-in `MIRSAT_MALIK` — the comment above the check says so.
  Downloaded packages are verified against the compiled-in anchor. Whether a release build
  can be made to reach the run-local path with a package it did not build has not been
  audited; that audit is open, and a finding there is in scope under "an anchor that can be
  substituted at runtime".
- **The in-game half is not yet live.** See the injection surface section above.

## How fixes reach users

A security fix ships through the normal update channel: a signed channel manifest under the
official registry root, verified by the identity the client trusts, with per-target entries
carrying a hash. An interrupted update must leave the previous version launchable, and the
updater never touches the data root, the component store, or any installed game.

If a *patch* rather than the application is the problem, it is revoked. A revocation carries a
machine-readable reason and a written statement that cannot be blank, enters a signed
revocation list every client checks, and tells anyone who already installed it — on next
launch, in Arabic and English — why it was pulled, offering a clean uninstall that restores
the game to its original bytes.

Because the registry is a public Git repository, every revocation is a commit anybody can
audit.

## If the signing key itself is the problem

There is no key escrow and no recovery. If the release key is compromised or lost, the anchor
is retired and a new one published, and every package signed under the old key becomes
uninstallable by clients built against the new one. That is not a gap in the design — it is
the property that makes the anchor worth anything — but it does mean a key compromise is a
coordinated rebuild of every client and a re-signing of the catalogue, not a configuration
change. `docs/mustawda.md` §7.1 has the procedure.
