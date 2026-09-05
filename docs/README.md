# Taarib documentation

Start at the root [`README.md`](../README.md); it carries the honest status of
the product and links into everything here. This page is the index.

## Read these in order if you are new

| document | what it answers |
| --- | --- |
| [`mimar.md`](mimar.md) | How Taarib is put together: the twenty-seven crates, the four boundaries, the path a string takes from a game's data files to Arabic on screen. |
| [`taqdimiya.md`](taqdimiya.md) | Why Taarib never produces a Unicode presentation form. **The decision a newcomer is most likely to want to undo.** Read before touching shaping, the atlas, or an adapter's draw path. |
| [`bina.md`](bina.md) | Building all of it from source, including the Windows bundle and the payload feature flag that silently produces a do-nothing module if you forget it. |

## Contracts

These are frozen specifications that implementation tasks build against. They are
written to be obeyed, not summarised.

| document | what it governs |
| --- | --- |
| [`abi.md`](abi.md) | The `jisr` C ABI in full — every type, every ownership rule, versioning, and the error model. The only way into the engine from outside Rust. |
| [`bidaya.md`](bidaya.md) | The payload bootstrap contract: the `taarib_bidaya` symbol, the order its steps run in, and the refusal discipline. **Section 6 is the honest record of what each in-game path reaches today and where it stops.** |
| [`tawzee.md`](tawzee.md) | Packaging and distribution: the two signing identities, the five shipped targets, the twenty-row artifact matrix the staging tool enforces, runtime resolution, self-update, and uninstall. |

## What actually runs

| document | what it answers |
| --- | --- |
| [`tashghil.md`](tashghil.md) | Engine by engine, what the in-game half reaches and the exact function where it stops — the evidence behind `TaqreerImkaniyat::jahiziya`. **Read it before believing any claim about an engine being supported.** It separates the three outcomes an install can have, including the two where the game changes for the worse. |

## The registry, and getting a patch into it

| document | what it governs |
| --- | --- |
| [`mustawda.md`](mustawda.md) | The registry is a Git repository and nothing else. The tree, the manifest, the 256 shards and why every one of them is published, the revocation list, the publishing checklist, and the two values only the owner can provision. |
| [`taqdeem.md`](taqdeem.md) | How a finished patch travels to the registry as a pull request, the one OAuth value that step is still waiting for, exactly where it goes, and what the transport does on either side of it. |

## Per-platform notes

Each of these is written from the live configuration and, where relevant, from
the pinned Tauri sources — not from intention.

| document | covers |
| --- | --- |
| [`tawzee/windows.md`](tawzee/windows.md) | The NSIS installer: the template, the five deviations from stock, the per-user on-disk tree and registry keys, WebView2 bootstrapping, upgrade-in-place, and the full installer and uninstaller flag table. |
| [`tawzee/linux.md`](tawzee/linux.md) | AppImage and `.deb`: what the AppImage carries versus what the host must supply, the desktop entry, resource paths, the portable marker, and how each format updates. |
| [`tawzee/steamdeck.md`](tawzee/steamdeck.md) | A verification audit against SteamOS — the read-only root, the absent Secret Service, Steam and Proton discovery — plus the steps that work on a Deck today. |
| [`tawzee/macos.md`](tawzee/macos.md) | Two per-architecture `.dmg`s and why there is no universal binary, what SIP does to `DYLD_INSERT_LIBRARIES`, the Gatekeeper position, and the entitlements granted and deliberately withheld. |
| [`tawzee/adhonat.md`](tawzee/adhonat.md) | Permissions: the webview capability set, what each OS actually needs, what the product must never ask for, how it degrades when refused, and portable mode. |
| [`tawzee/tahaqquq.md`](tawzee/tahaqquq.md) | What was actually built, installed and run on real hardware — not what the configuration says should happen. The Windows binary and installer, the Linux glibc floor and the two rebuilds that moved it, the fonts proved rather than assumed, and a plain statement of which targets that machine could not reach at all. |

### A note on staleness

`tawzee/steamdeck.md` and `tawzee/windows.md` are audit records written at a
point in time, and parts of them have been overtaken. `tawzee/windows.md` still
describes `msi` and `rpm` as present in `bundle.targets` and requiring removal;
they were removed, and the live value is
`["deb", "appimage", "nsis", "app", "dmg"]`. Both files also predate several
things they record as absent — `bundle.resources`, the `mawarid/` directory, and
the `taarib-tajmee` / `taarib-tahdith` / `taarib-mudkhal` crates all exist now —
and the Steam Deck document contradicts itself on some of these between its
checklist and its later findings, because the findings were amended as they
closed and the checklist was not.

They are left as written because they are records of an audit rather than living
descriptions, and rewriting an audit erases what it found. Where one disagrees
with the tree, the tree wins.

## Elsewhere in the repository

- [`../ROADMAP.md`](../ROADMAP.md) — the build contract for the whole product.
  Twenty-four phases; section 2 holds the eight settled architectural decisions
  and section 4.1 holds the naming law.
- [`../schemas/README.md`](../schemas/README.md) — the generated JSON Schemas and
  what generates them.
- [`../unity/README.md`](../unity/README.md) — the C# plugin projects.
- [`../adapters-script/README.md`](../adapters-script/README.md) — the in-game
  JavaScript, Python and Ruby sides.
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md) — how to submit a translation, how
  review works, and what a code contribution has to pass. Bilingual.
- [`../SECURITY.md`](../SECURITY.md) — how to report a vulnerability, what is in
  scope, and the limitations already known.
