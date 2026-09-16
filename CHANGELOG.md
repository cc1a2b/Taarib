# Changelog

Every released version of Taarib, newest first. Each entry says what changed for
somebody using the product, not what moved in the tree; the commit bodies carry
the engineering detail and are unusually long on purpose.

Versions follow [semantic versioning](https://semver.org/spec/v2.0.0.html). The
format below is close to [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
but groups by what a reader is looking for rather than by the standard's five
headings, because "Fixed" covering both an unreadable glyph and a refused
install tells nobody which one they hit.

## [1.0.1] — 2026-09-15

Arabic a player can read, on games the tool patches by itself.

### Rendering

- **Glyphs are rasterised at the size they are drawn on screen, not the size the
  component declares.** Those are the same number only on an unscaled overlay
  canvas. On a world-space surface — a monitor in a room, a sign on a wall — the
  size field is in world units, so a heading declaring `8` could cover a third
  of the screen and was being rasterised at eight pixels and magnified. The
  drawn size is now measured by projecting one layout unit into screen space and
  quantised to a ten-rung ladder, so a moving camera cannot mint a fresh glyph
  set per frame. The layout is untouched, so text never re-flows as a player
  walks toward a sign. Scaled canvases get sharper text from the same change.
- **The font chain loads face zero.** A chain position was being passed where a
  face index belonged, so any font whose file holds one face was refused with
  `TAARIB-E-2501`.
- **The compiler lays out at six sizes rather than three.** 18, 24, 32, 48, 64
  and 96 pixels. The old top of 32 meant every title and every large menu label
  was a magnified 32-pixel bitmap sitting beside the engine's own crisp outline
  text.
- Five defects that kept the Unity takeover off the screen entirely: a Harmony
  prefix that skipped TextMeshPro's generation, a mesh drawn against the wrong
  atlas, a whole-buffer vertex handoff, bottom-up texture row order, and the
  BepInEx manager being destroyed at frame zero.

### Getting the text out of a game

- **Runtime capture works on the Mono backend**, which is the only route to the
  text a Unity release build's absent type tree hides. Capture is now a normal
  step in the one-button run rather than a debugging tool, it writes one format
  both sides agree on, and a merged session survives a resumed run instead of
  being discarded.
- **The extraction table and the workshop are the same table.** A finished run
  publishes its rows into the workshop's project store, and a workshop with no
  project recovers one from a run that already finished — previously the rows
  sat under the run's identifier while the workshop looked under the game's and
  reported that no project existed, to somebody who had just finished a
  translation. Corrections made in the workshop now feed the next patch instead
  of being overwritten by a fresh machine pass.

### Translation

- **The free provider sends up to fifty strings per request instead of one.** A
  4,265-string game went from roughly 4,265 requests to 86. Pacing was widened
  so a run finishes rather than meeting `TAARIB-E-8307` on the first request.
- The official registry source points at an endpoint that serves files;
  the previous default returned 404 for every fetch.

### Installing

- **The one-button run deploys the framework.** Without it the run wrote the
  patch and its fonts into the game and no loader, so a game that had never been
  patched started in its original language with the whole translation sitting on
  disk beside it and the run reporting success.
- The install writes the uncompressed working copy the Unity plugin can read,
  and rebuilds the package when the string table has moved rather than reusing a
  sealed one that no longer matches.
- **The first-run statement is asked for at the door**, before extraction and
  before any money is spent, rather than at the install stage after both. It can
  now be read and accepted on the screen that needs it, and it ships in English
  as well as Arabic — a session running in English was being asked to accept
  words it might not be able to read.

### Safety and correctness

- A cross-language boundary audit, and the defects it found fixed.
- An uninstall that could not be finished, and a right-to-left override that
  could spoof a URL.
- 66 interface strings that existed in neither language file.
- `#[serde(other)]` on a bound type silently froze the whole TypeScript binding
  generator, leaving the committed bindings stale.

## [1.0.0] — 2026-09-12

The first version the product declared.

A text engine that renders Arabic into a running PC game without borrowing
anything from the game: HarfRust for shaping, skrifa for rasterisation, no
Unicode presentation forms anywhere, and Taarib's own bundled fonts. Around it:
the signed `.ruqaa` patch container, engine detection with a capability report
that tells a user which of three tiers their game gets, a safety gate that
refuses anti-cheat titles outright and warns before a multiplayer one, eight
graphics APIs behind the universal overlay, and the Studio that builds a patch
end to end.

No binaries are published for this version. Every 1.0.0 build that still exists
was compiled against the development trust anchor, which reports `tatwir` in its
provenance and refuses every patch the owner signed.

[1.0.1]: https://github.com/cc1a2b/Taarib/releases/tag/v1.0.1
[1.0.0]: https://github.com/cc1a2b/Taarib/releases/tag/v1.0.0
