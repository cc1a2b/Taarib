# The Taarib Native ABI — `taarib_jisr`

This is the reference for the C surface of `taarib_jisr`, the shared library
that carries the Arabic text engine (`taarib-saff`) and the glyph atlas
(`taarib-lawha`) into processes that are not Rust. It is the contract the
Unity adapters bind through P/Invoke, the Unreal module links directly, the
Ren'Py adapter reaches through `ctypes`, and the RPG Maker VX Ace adapter
reaches through RGSS3's `Win32API`. There is no second entry point into the
engine from outside Rust, and no engine-specific extension of this one —
an adapter that needed a private ABI call would be an adapter making policy,
which adapters do not do.

The library ships as `taarib_jisr.dll` / `libtaarib_jisr.so` /
`libtaarib_jisr.dylib`, built for `x86_64` and `aarch64` on Windows, Linux
and macOS, plus `i686` on Windows and Linux, because a 32-bit game process
needs a 32-bit library and the injector selects by the target image, not by
the host. The generated header is `crates/taarib-jisr/include/taarib.h`; it
is produced by cbindgen and committed, so a consumer never needs the Rust
toolchain. The frozen structure layouts live in
`crates/taarib-jisr/src/anwa.rs` — except the two appended types, which
live beside the functions they serve so that an append stays visible as
an append: `TaaribIhsaatKhazina`, the layout cache's counters, in
`awamir.rs`, and the capture callback type `TaaribIltiqatFn`, the
nullable form of `hayat.rs`'s `TaaribRaddIltiqat` — and the status code
space in `crates/taarib-jisr/src/khata_c.rs`; this document states the
same contract in longer form, and where the two could ever disagree, the
Rust source is the truth.

## The one rule that shapes everything

**No Rust type crosses the boundary, no panic crosses the boundary, and
every function returns a status.**

Everything else in this document is a consequence of that rule:

- Every function is `extern "C"`, unmangled, returns an `int32_t` status,
  and writes its results through out-parameters. Zero is success; every
  failure is negative, so a caller in any language writes `if (r < 0)` and
  is right.
- Every structure that crosses is `repr(C)`, made only of fixed-width
  integers, `float`, raw pointers and `size_t`, ordered widest-first so
  that no structure carries implicit padding an uninitialised byte could
  hide in. Enumerations cross as `uint32_t`, never as C enums, because a C
  enum's width is implementation-defined and a value outside the declared
  set is undefined behaviour in C++ — and an adapter compiled against a
  newer header will eventually send exactly such a value. Every decoder
  treats an unrecognised number as the documented default instead.
- Every entry point wraps its body in `catch_unwind`. A panic that unwound
  into a game's C# or C++ frame would be a crash the player blames on the
  game, so a panic here becomes `TAARIB_INHIYAR` plus a logged diagnostic,
  and the game keeps running to report it.
- Because an `int32_t` is not enough to act on, the full error — permanent
  code, Arabic sentence, English sentence, next action — is stashed in
  thread-local storage on the way out, and retrieved with the
  `taarib_khata_*` functions. Thread-local, not global: a render thread and
  a background loader can both be in this library at once, and a shared
  last-error slot would let one thread's failure be reported as the
  other's, which sends whoever reads it looking in the wrong place.

## Status codes

Success is `0`. Every other value is negative and permanent: a code that has
shipped never changes meaning. `TAARIB_SIAT_QASIRA` is the one code that is
not a failure in the usual sense — it is the capacity negotiation described
below, and a caller that treats it as an error has misread the protocol.

| Code | Value | Meaning | What the caller does |
| --- | --- | --- | --- |
| `TAARIB_NAJAH` | 0 | The operation succeeded. | Continue. |
| `TAARIB_KHATA_AAM` | -1 | Something failed that no more specific code describes. The stashed error still names it exactly. | Retrieve the stashed error and surface it. |
| `TAARIB_MUASHIR_BATIL` | -2 | A required pointer argument was null. | Fix the call site; this is the caller's bug. |
| `TAARIB_MAQBAD_BATIL` | -3 | A handle was not one this library issued, or was destroyed already. | Fix the handle's lifetime; the use-after-free was detected, not executed. |
| `TAARIB_SIAT_QASIRA` | -4 | The caller's buffer is too small. The required capacity was written to the out-parameter; nothing else was written. | Grow to the reported capacity and retry — once is enough. |
| `TAARIB_ISDAR_GHAYR_MUTAWAFIQ` | -5 | The caller was built against a different major version of this ABI. | Refuse to proceed and say which versions disagreed. |
| `TAARIB_TARMIZ_BATIL` | -6 | A string argument was not valid UTF-8. | Fix the encoding at the call site; do not strip and retry silently. |
| `TAARIB_KHATT_MARFUD` | -7 | A font was rejected, or could not be read. The stashed error names the missing table or feature. | Show the sentence; offer a different font. A font without `GSUB` does not degrade — it silently fails to join, which is why rejection is loud. |
| `TAARIB_TASHKEEL_FASHIL` | -8 | Shaping produced nothing for text that is not empty. | A different font is the answer, not a retry. |
| `TAARIB_DHAKIRA` | -9 | An allocation failed inside the library. Reported rather than aborted, because this library runs inside somebody else's process and does not get to decide that the process ends. | Free what can be freed (destroy an atlas, clear the cache) and retry, or degrade. |
| `TAARIB_QEEMA_BATILA` | -10 | An argument was structurally fine but its value is not usable — a negative width, a size of zero, a style span pointing outside its text. | Fix the value; the stashed error says which one. |
| `TAARIB_INHIYAR` | -11 | A panic was caught at the boundary. | The library has a bug worth reporting — and the game is still running to report it. Log the stashed diagnostic and continue without the failed result. |
| `TAARIB_LAWHA_MUMTALIA` | -12 | The atlas is full and nothing in it may be evicted, because everything is referenced by the frame being drawn. | Finish the frame, call `taarib_lawha_ibda_itar`, retry; or raise the byte budget. |
| `TAARIB_GHAYR_MADUM` | -13 | The operation is not available in this build. | Take the documented fallback path; do not retry. |
| `TAARIB_GHAYR_MUHAYYAA` | -14 | The library has not been initialised, or was shut down. | Fix the call ordering; this is the caller's bug. |

### Retrieving the failure behind a code

Four functions read the thread-local stash:

- `taarib_khata_akhir()` — the last status code for this thread.
- `taarib_khata_khutwa()` — the next-action number (see below).
- `taarib_khata_ramz(hadaf, siaa, matlub)` — the permanent machine code as
  text, e.g. `TAARIB-E-2500`. Empty when the failure carried no structured
  error (a null pointer, a stale handle, a caught panic are recorded as a
  bare status).
- `taarib_khata_nass(lugha, hadaf, siaa, matlub)` — the sentence, with
  `lugha` 0 for Arabic and 1 for English. Any other number resolves to
  Arabic, because Arabic is the primary text of this product and a wrong
  discriminant should still produce the real sentence.

The retrieval functions are the only entry points that never modify the
stash. **Every other entry point clears the stash for its thread on
success**, so a caller must read all four parts immediately after the
failing call, before making any other call on that thread. The two string
retrievals use the standard capacity negotiation and can themselves return
`TAARIB_SIAT_QASIRA`; retrieving an error never overwrites it.

The next-action number is the `Khutwa` from the error model, frozen and
additive — a new action takes the next free number, and a caller that does
not recognise a number shows the sentence without a button rather than
showing nothing:

| Number | Action |
| --- | --- |
| 0 | Nothing to do; the message is complete on its own. |
| 1 | Try the same operation again. |
| 2 | Rescan the library. |
| 3 | Re-probe this game's engine. |
| 4 | Point Taarib at a path it could not find. |
| 5 | Choose a different font. |
| 6 | Open a section of Settings. |
| 7 | Open Diagnostics. |
| 8 | Open the overflow report. |
| 9 | Open the offending strings in the workspace. |
| 10 | Update Taarib itself. |
| 11 | Reinstall the framework for this game. |
| 12 | Uninstall the patch and restore the game. |
| 13 | Re-match the patch against the game's new build. |
| 14 | Verify the game's files through its launcher. |
| 15 | Contact the patch's contributor. |
| 16 | Send a diagnostics bundle to the project owner. |
| 17 | Free disk space and retry. |
| 18 | Grant the permission the operating system refused. |

## The handle model

Four opaque handle kinds cross the boundary, each a distinct pointer type
so that C callers get type safety between kinds:

| Handle | What it is | Created by | Destroyed by |
| --- | --- | --- | --- |
| `TaaribSiyaq` | An engine context: the caches, the pooled buffers, the capture channel. | `taarib_siyaq_insha` | `taarib_siyaq_ihdham` |
| `TaaribKhatt` | A loaded, validated font. | `taarib_khatt_min_dhakira` | `taarib_khatt_ihdham` |
| `TaaribSilsila` | An ordered fallback chain of fonts. | `taarib_silsila_insha` | `taarib_silsila_ihdham` |
| `TaaribLawha` | A glyph atlas that grows and evicts at runtime. | `taarib_lawha_insha` | `taarib_lawha_ihdham` |

A handle is pointer-shaped but is not a pointer into anything. The value is
a **generation-tagged index** into the library's own table: some bits name
the slot, the rest carry the generation the slot was in when the handle was
issued. Destroying a handle bumps the slot's generation, so a stale handle
presented later fails the generation comparison and the call returns
`TAARIB_MAQBAD_BATIL`. Vulkan's non-dispatchable handles work the same way,
for the same reason.

Why not a plain pointer? Because a pointer alone cannot detect
use-after-free. Dereferencing a freed pointer is undefined behaviour that
usually *works* — the memory is still mapped, the bytes are still warm —
until an allocation reuses the block and the crash appears somewhere
unrelated, in somebody else's game, with a stack that points at nothing.
Worse, a reallocated slot could belong to a *different* live object, and
the call would succeed against the wrong font. The generation tag turns
both outcomes into a reported status: the difference between a bug report
naming a stale handle and a crash dump in a player's game.

Note what is *not* a handle: a layout. `taarib_takhtit` writes glyphs and
lines into a caller-owned buffer and retains nothing, so there is nothing
to destroy and no lifetime to get wrong. A layout handle would have been
one more create/destroy pair for every adapter to leak; a buffer the caller
already owns cannot be leaked by this library.

## The ownership contract

Stated once, completely. Every ownership question in an ABI is a leak or a
double free waiting for the one caller who reads the documentation
differently, so the surface is designed to leave as few questions as
possible:

1. **The caller allocates every buffer; the library frees none of them.**
   Glyph arrays, line arrays, string buffers, out-structures — all of them
   are caller memory, read and written only for the duration of the call.
   The library never keeps a pointer to caller memory past the return.
   There is no function anywhere in the surface that returns memory the
   caller must free.

2. **Input pointers are borrowed for the call only.** The text, the span
   array and the feature array inside `TaaribTalab`, and the handle array
   given to `taarib_silsila_insha`, must stay valid until the call
   returns, and are not touched afterwards. A garbage-collected caller
   must keep those objects referenced across the call and may release
   them the instant it returns.

3. **Font bytes are copied.** `taarib_khatt_min_dhakira` copies the bytes
   into the library's own shared, reference-counted buffer — the single
   buffer of Decision 4 that shaping, metrics and rasterization all read.
   The caller may free its copy immediately. This is the only sane
   contract when the callers include garbage-collected runtimes whose
   strings can be collected or moved; a borrowing variant would be an
   invitation to a use-after-free the library could not detect.

4. **Destroying a font does not tear a chain.** A chain holds its own
   references to the font resources, so `taarib_khatt_ihdham` on a font
   that a live chain uses only drops the handle; the chain keeps working.
   The reverse is also true: destroying a chain releases its references
   and leaves the font handles valid. Destroy in any order.

5. **Exactly one function lends memory out: `taarib_lawha_safha`.** The
   `bayt` pointer in the returned `TaaribSafha` points into the atlas's
   own pages. It stays valid until the next call that can change the
   atlas — any `taarib_lawha_shakl`, or destroying the atlas. The
   contract is upload-and-forget: copy the texels into a texture in the
   same breath, then let the pointer go. Holding it across a lookup is
   reading a rectangle that may have been reassigned.

6. **Strings come back through caller buffers, never as returned
   pointers.** Every text-returning function (`taarib_isdar_nass`,
   `taarib_khata_ramz`, `taarib_khata_nass`, `taarib_khatt_aila`) writes
   UTF-8 into a caller buffer using the capacity negotiation below, and
   writes a trailing NUL when there is room, so C callers can treat the
   buffer as a string without consulting the length.

7. **Handles are owned by the caller and returned to the library.** Each
   create pairs with exactly one destroy, listed in the handle table
   above. Destroying a context is the one cascade: it takes down every
   font, chain and atlas handle issued from it, and a child handle
   presented afterwards is reported stale rather than tolerated. Orderly
   adapters still destroy children first, context last, mirroring the
   create order — a teardown that names each resource is a teardown a
   leak cannot hide in.

## The capacity negotiation

One protocol serves every variable-length result: positioned glyphs, lines
and strings. The caller states a capacity; when it is short, the call
writes the *required* capacity into the out-parameter, writes nothing else,
and returns `TAARIB_SIAT_QASIRA`. The caller grows once and retries. The
loop, written out:

```c
size_t siaa = 256;
uint8_t *hadaf = malloc(siaa);
for (;;) {
    size_t matlub = 0;
    int32_t halat = taarib_khata_nass(1, hadaf, siaa, &matlub);
    if (halat == TAARIB_NAJAH) {
        break;                      /* hadaf holds the NUL-terminated text */
    }
    if (halat == TAARIB_SIAT_QASIRA && matlub > siaa) {
        uint8_t *jadeed = realloc(hadaf, matlub);
        if (jadeed == NULL) { /* handle OOM */ break; }
        hadaf = jadeed;
        siaa = matlub;
        continue;                   /* exactly one more round */
    }
    break;                          /* a real failure; stop negotiating */
}
```

Two properties make this the shape of the allocation-free hot path rather
than a chore:

- **The reported requirement is exact and stable for the same input**, so
  the second call succeeds. A caller that loops more than twice for one
  input has found a library bug, and all three shipped bindings bound
  their loops and report rather than spin — inside a game, a hang is
  worse than a missing line.
- **The buffer never shrinks.** A game's text sizes plateau within the
  first frames; after the buffer has grown to the high-water mark, every
  later call succeeds on the first attempt and allocates nothing. That is
  what "allocation-free in practice and not merely in principle" means:
  the protocol front-loads all allocation into the first seconds.

For layout, the same protocol runs through `TaaribMakhzanTakhtit`: the
caller fills `huruf`/`siaat_huruf` and `sutur`/`siaat_sutur`; on
`TAARIB_SIAT_QASIRA` the required counts are in `adad_huruf` and
`adad_sutur` (both are always written, so one retry fixes both arrays),
and nothing was written to the arrays themselves.

## Thread safety, per function group

| Group | Functions | Guarantee |
| --- | --- | --- |
| Version | `taarib_abi_isdar`, `taarib_isdar_nass` | Callable from any thread at any time, concurrently. |
| Error retrieval | `taarib_khata_akhir`, `taarib_khata_khutwa`, `taarib_khata_ramz`, `taarib_khata_nass` | Callable from any thread; each thread reads only its own stash. An error must be retrieved on the thread that received the failing status — another thread's stash is another thread's failure. |
| Context lifecycle and use | `taarib_siyaq_*`, `taarib_takhtit`, `taarib_qiyas`, `taarib_makhzan_ihsaat` | A context may be created on one thread and used on another (it is `Send`), and any thread may call in at any time: every call that reaches a context takes the library's **context lock** exactly once — one process-wide lock over the context table, held for the duration of the call and released before it returns — so concurrent calls serialise instead of racing. No caller-side lock is needed; two threads sharing one context are safe and merely slow, because their calls interleave at call granularity rather than run in parallel. |
| Fonts and chains | `taarib_khatt_*`, `taarib_silsila_*` | Serialise on the same context lock — but `taarib_khatt_min_dhakira` copies and validates the font bytes *before* taking it, so a render thread sharing the context never stalls behind megabytes of font parsing. The font *resources* underneath are immutable once loaded and reference-counted, which is why fonts and chains can be destroyed in any order. |
| Atlas | `taarib_lawha_*` | Serialise on the same context lock. `taarib_lawha_shakl` and `taarib_lawha_ibda_itar` are safe on a render thread because that lock is the only one they take, it is taken per call, and no caller can hold it across calls — let alone across a frame. The one piece of caller code that ever runs under it, the capture sink, is bound by its registration contract to return promptly. |
| Capture | `taarib_iltiqat_shaghghil`, `taarib_iltiqat_awqif` | Serialise on the same context lock, which is what makes their guarantees exact: registering replaces any previous sink atomically with respect to layout calls, and `taarib_iltiqat_awqif` returns only after any in-flight delivery has finished, because delivery runs under the lock this call must first take. The sink itself runs on whichever thread called `taarib_takhtit`. |

What keeps the cached case cheap enough to sit on a render thread is not
the locking but what happens under it: a hit copies a finished layout
straight out of the cache into the caller's buffer and touches the
allocator zero times. The lock is real and taken once per call — cheap
because the one-context-per-thread arrangement every adapter uses leaves
it uncontended, not because it is absent.

## Runtime string capture

Phase 12's problem: some games' containers cannot be read statically —
the strings live in encrypted archives, in code, in formats no extractor
understands. Capture solves it from the other end. Every string a game
draws passes through `taarib_takhtit` on its way to the screen, so a sink
registered on the context records what the game *actually draws*, and the
extraction pipeline is fed by the one source that cannot be wrong about
which strings exist: the running game itself.

Two functions carry the whole surface. `taarib_iltiqat_shaghghil`
registers the sink and an opaque pointer that travels with it;
`taarib_iltiqat_awqif` unregisters them. The sink crosses as a
`TaaribIltiqatFn` — the nullable form of `TaaribRaddIltiqat`, nullable so
that the null function pointer is representable and refused at
registration rather than called — and the function the host implements
has this shape:

```c
void radd(void *mustakhdim, const uint8_t *nass, size_t tul);
```

`mustakhdim` is the registered pointer, handed back untouched and never
dereferenced by the library; it must simply stay meaningful to the sink
until capture stops. `nass` and `tul` are the request's UTF-8 text with
an explicit length, **not** NUL-terminated, and **borrowed for the
duration of the invocation only** — a sink that wants the text copies it
before returning.

**Where capture fires: on a cache miss in `taarib_takhtit`**, after the
layout is finished and before the call returns, on whichever thread made
the call. Only there — `taarib_qiyas` measures and never reports. A miss
is precisely "text this context has not seen", so the stream
self-deduplicates: the first frame that shows a line reports it, every
later frame hits the cache and reports nothing, and steady-state play
costs the sink no calls at all — which is what makes capture safe to
leave on for a whole session. The report happens whatever the buffer
negotiation said: a `TAARIB_SIAT_QASIRA` return still captures, because a
short buffer does not make the text less new, and the retry that follows
hits the cache and does not report again.

**The sink runs while the context lock is held.** That is what makes
registration and delivery race-free without a second lock, and both
halves of the registration contract follow from it. The sink returns
promptly, because a frame is waiting on the layout call it is running
inside. And it never calls back into this library — not on any handle,
not even a version query on a context — because the lock it would need is
the lock it is already running under, the lock is not re-entrant, and the
result is not a status code but a deadlocked thread, which inside a game
is the render thread. Hand the bytes to the host's own transport and
return.

Teardown is exact. Delivery happens under the same lock
`taarib_iltiqat_awqif` must take, so **when it returns, no delivery is in
flight** and none will follow: the sink and its `mustakhdim` may be torn
down immediately. Stopping capture that was never started succeeds and
does nothing, and destroying the context drops the registration the same
way.

## Versioning

`taarib_abi_isdar` returns a major and a minor. The rule a caller applies
is the one the version constants in `awamir.rs` document: **refuse to
proceed unless the majors are equal**, because past a major mismatch the
two sides no longer agree what the bytes between them mean. The minor is
different in kind. A minor bump only appends, so everything an old caller
compiled against keeps working, and a minor mismatch **in either
direction is not a reason to refuse** — it is only a reason not to call
what the running library does not have. A caller built against a newer
minor treats an older library's missing exports as absent features and
takes its documented fallback, exactly as it would for
`TAARIB_GHAYR_MADUM`. All three shipped bindings — Unity's P/Invoke
loader, the Ren'Py `ctypes` binding, the RPG Maker VX Ace `Win32API`
binding — do exactly this at load: they compare the major only, refuse a
mismatch with a sentence naming both versions in the host's own log, and
never gate on the minor.

What a **minor** bump may do — all of it additive, none of it able to
break a caller built against an older minor:

- Add new exported functions, and with them the new structures and
  callback types they need — `taarib_makhzan_ihsaat` with
  `TaaribIhsaatKhazina`, and the capture pair with `TaaribIltiqatFn`,
  are appends of exactly this shape.
- Add new status codes, new next-action numbers, new flag bits in
  existing `alam` fields, and new discriminant values for existing
  `uint32_t` options — all spaces every decoder already treats as
  "unrecognised means the documented default".
- Improve behaviour behind an existing signature (better kashida ranking,
  a smarter cache) without changing any structure or any documented
  meaning.

What a minor may never do is change anything already shipped. Any of the
following forces a **major** bump:

- Any change to a frozen structure: a field added, removed, reordered or
  resized — even appending, because structures cross by value and by
  pointer with sizes compiled into every caller.
- Any change to an existing function's signature or to the meaning of an
  existing status code, flag, discriminant or next-action number.
- Any change to the ownership contract or the negotiation protocol.
- Removing a function.

Codes, flags and action numbers are permanent across majors too: a value
that has shipped is never reused with a different meaning, because users
paste codes into bug reports and search for them years later.

## A worked example in C

The complete shape of a session: check the version, make a context, load a
font, build a chain, lay out a line of Arabic into a fixed width with the
negotiation loop written out, walk the glyphs, and tear everything down in
reverse. Error handling is real — every status is checked, and a failure
prints the stashed sentence before cleanup.

```c
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "taarib.h"

/* Prints this thread's stashed failure: code, English sentence, action.
   The string functions take uint8_t*, the header's spelling of a UTF-8
   byte; printf wants char*, C's other name for the same byte, so the
   cast sits at the print and nowhere else. */
static void itbaa_khata(const char *ayn, int32_t halat) {
    uint8_t ramz[32] = {0};
    uint8_t nass[512] = {0};
    size_t matlub = 0;
    taarib_khata_ramz(ramz, sizeof ramz, &matlub);
    taarib_khata_nass(1, nass, sizeof nass, &matlub); /* 1 = English */
    fprintf(stderr, "%s failed: %d %s %s (next action %d)\n",
            ayn, halat, (const char *)ramz, (const char *)nass,
            taarib_khata_khutwa());
}

int main(int adad, char **wasail) {
    if (adad < 2) {
        fprintf(stderr, "usage: %s <font.ttf>\n", wasail[0]);
        return 1;
    }

    /* 1. The version gate. A mismatched major never gets further. */
    uint32_t kabir = 0, sagheer = 0;
    int32_t halat = taarib_abi_isdar(&kabir, &sagheer);
    if (halat != TAARIB_NAJAH || kabir != 1) {
        fprintf(stderr, "ABI major %u, need 1: refusing to continue\n", kabir);
        return 1;
    }

    /* 2. The font's bytes. The library copies; this buffer is ours to
       free the moment the load returns. */
    FILE *malaf = fopen(wasail[1], "rb");
    if (malaf == NULL) { perror("fopen"); return 1; }
    fseek(malaf, 0, SEEK_END);
    long tul_malaf = ftell(malaf);
    fseek(malaf, 0, SEEK_SET);
    uint8_t *bayt = malloc((size_t)tul_malaf);
    if (bayt == NULL || fread(bayt, 1, (size_t)tul_malaf, malaf) != (size_t)tul_malaf) {
        fprintf(stderr, "could not read %s\n", wasail[1]);
        fclose(malaf);
        free(bayt);
        return 1;
    }
    fclose(malaf);

    /* 3. A context: 4 MiB layout cache, two pooled buffers. */
    TaaribKhiyaratSiyaq khiyarat_siyaq = {0};
    khiyarat_siyaq.mizaniyat_makhzan = 4u * 1024u * 1024u;
    khiyarat_siyaq.adad_makhazin = 2;
    TaaribSiyaq siyaq = NULL;
    halat = taarib_siyaq_insha(&khiyarat_siyaq, &siyaq);
    if (halat != TAARIB_NAJAH) {
        itbaa_khata("taarib_siyaq_insha", halat);
        free(bayt);
        return 1;
    }

    /* 4. The font, validated for Arabic (face 0, validation on). A font
       without the joining tables is rejected here, loudly, instead of
       drawing disconnected letters silently later. */
    TaaribKhatt khatt = NULL;
    halat = taarib_khatt_min_dhakira(siyaq, bayt, (size_t)tul_malaf,
                                     0, 1, &khatt);
    free(bayt); /* copied by the library; ours ends here */
    if (halat != TAARIB_NAJAH) {
        itbaa_khata("taarib_khatt_min_dhakira", halat);
        taarib_siyaq_ihdham(siyaq);
        return 1;
    }

    /* 5. A chain of one. Real adapters chain an Arabic font and a Latin
       fallback; the request below still works with one. */
    TaaribKhatt khutut[1] = {khatt};
    TaaribSilsila silsila = NULL;
    halat = taarib_silsila_insha(siyaq, khutut, 1, &silsila);
    if (halat != TAARIB_NAJAH) {
        itbaa_khata("taarib_silsila_insha", halat);
        taarib_khatt_ihdham(siyaq, khatt);
        taarib_siyaq_ihdham(siyaq);
        return 1;
    }

    /* 6. The request: logical-order UTF-8, 24 px, 320 px wide, kashida
       justification, everything else the documented defaults (zero). */
    static const char nass[] =
        "\xD9\x85\xD8\xB1\xD8\xAD\xD8\xA8\xD9\x8B\xD8\xA7 "
        "\xD8\xA8\xD8\xA7\xD9\x84\xD8\xB9\xD8\xA7\xD9\x84\xD9\x85";
        /* "مرحبًا بالعالم", written as bytes so this file survives any
           source encoding */
    TaaribTalab talab = {0};
    talab.nass = (const uint8_t *)nass;
    talab.tul_nass = sizeof nass - 1;
    talab.silsila = silsila;
    talab.hajm = 24.0f;
    talab.ard_mutah = 320.0f;
    talab.khiyarat.dabt = 3;      /* kashida, then spaces */
    talab.khiyarat.muhadhaha = 3; /* filled */

    /* 7. The negotiation loop, in full. Start deliberately small to show
       the retry; a real adapter starts near its high-water mark. */
    size_t siaat_huruf = 8, siaat_sutur = 2;
    TaaribHarf *huruf = malloc(siaat_huruf * sizeof *huruf);
    TaaribSatr *sutur = malloc(siaat_sutur * sizeof *sutur);
    TaaribMakhzanTakhtit makhzan;
    for (;;) {
        memset(&makhzan, 0, sizeof makhzan);
        makhzan.huruf = huruf;
        makhzan.siaat_huruf = siaat_huruf;
        makhzan.sutur = sutur;
        makhzan.siaat_sutur = siaat_sutur;
        halat = taarib_takhtit(siyaq, &talab, &makhzan);
        if (halat == TAARIB_NAJAH) {
            break;
        }
        if (halat != TAARIB_SIAT_QASIRA) {
            itbaa_khata("taarib_takhtit", halat);
            goto tanzheef;
        }
        if (makhzan.adad_huruf > siaat_huruf) {
            siaat_huruf = makhzan.adad_huruf;
            huruf = realloc(huruf, siaat_huruf * sizeof *huruf);
        }
        if (makhzan.adad_sutur > siaat_sutur) {
            siaat_sutur = makhzan.adad_sutur;
            sutur = realloc(sutur, siaat_sutur * sizeof *sutur);
        }
        /* Grown to the exact reported requirement: the retry succeeds. */
    }

    /* 8. Walk the result. Glyph ids and positions — never codepoints. */
    printf("%zu glyphs on %zu line(s), %.1f x %.1f px, %s\n",
           makhzan.adad_huruf, makhzan.adad_sutur,
           (double)makhzan.ard, (double)makhzan.irtifa,
           (makhzan.alam & TAARIB_TAKHTIT_YAMEEN) ? "RTL" : "LTR");
    for (size_t s = 0; s < makhzan.adad_sutur; s++) {
        const TaaribSatr *satr = &sutur[s];
        for (uint32_t h = 0; h < satr->adad_huruf; h++) {
            const TaaribHarf *harf = &huruf[satr->awwal_harf + h];
            printf("  glyph %5u  x %7.2f  y %7.2f  advance %6.2f  font %u%s\n",
                   harf->muarrif, (double)harf->s, (double)harf->a,
                   (double)harf->taqaddum, harf->khatt,
                   (harf->alam & TAARIB_HARF_ALAMA) ? "  (mark)" : "");
        }
    }

tanzheef:
    /* 9. Reverse order: children before context. The buffers are ours. */
    free(huruf);
    free(sutur);
    taarib_silsila_ihdham(siyaq, silsila);
    taarib_khatt_ihdham(siyaq, khatt);
    taarib_siyaq_ihdham(siyaq);
    return halat == TAARIB_NAJAH ? 0 : 1;
}
```

## Every exported function

All twenty-eight exported functions. Arguments are shown with their
header types and names; every function returns `int32_t`. "Caller buffer"
means memory the caller allocated and keeps; "copied" means the library
took its own copy before returning; "borrowed" means valid only as
documented in the ownership note.

| Function | Purpose | Ownership note |
| --- | --- | --- |
| `taarib_abi_isdar(uint32_t *kabir, uint32_t *sagheer)` | The ABI's major and minor. The first call a caller makes; a mismatched major means stop. | Writes two caller-owned integers. |
| `taarib_isdar_nass(uint8_t *hadaf, size_t siaa, size_t *matlub)` | The library's human-readable version, for logs and diagnostics. | Caller buffer, negotiated capacity, NUL-terminated when it fits. |
| `taarib_khata_akhir(void)` | This thread's last status code. | Reads the thread-local stash; touches nothing. |
| `taarib_khata_khutwa(void)` | This thread's last next-action number. | Reads the stash; touches nothing. |
| `taarib_khata_ramz(uint8_t *hadaf, size_t siaa, size_t *matlub)` | The permanent code of the last failure, as text. | Caller buffer, negotiated; never modifies the stash. |
| `taarib_khata_nass(uint32_t lugha, uint8_t *hadaf, size_t siaa, size_t *matlub)` | The last failure's sentence, Arabic (0) or English (1). | Caller buffer, negotiated; never modifies the stash. |
| `taarib_siyaq_insha(const TaaribKhiyaratSiyaq *khiyarat, TaaribSiyaq *khuruj)` | Creates an engine context: cache budget, buffer pool. | Options are read during the call only. The handle is owned by the caller until `taarib_siyaq_ihdham`. |
| `taarib_siyaq_ihdham(TaaribSiyaq siyaq)` | Destroys a context and everything it owns. | Invalidates the handle; child handles used afterwards are reported stale. Destroy children first. |
| `taarib_siyaq_amsah(TaaribSiyaq siyaq)` | Empties the context's caches — the layout cache, the pooled layout buffers, the prepared shaper state — and returns their memory. | No ownership change: every handle stays valid, a registered capture sink stays registered, and the atlas table is deliberately untouched — pages stay open, and page pointers lent by `taarib_lawha_safha` are not invalidated. The only cost is that the next layouts shape again. |
| `taarib_khatt_min_dhakira(TaaribSiyaq siyaq, const uint8_t *bayt, size_t tul, uint32_t fahras, uint32_t fahs_arabi, TaaribKhatt *khuruj)` | Loads face `fahras` from font bytes, validating Arabic tables when `fahs_arabi` is non-zero. | **Bytes are copied**; the caller may free them immediately. Handle owned until `taarib_khatt_ihdham`. |
| `taarib_khatt_ihdham(TaaribSiyaq siyaq, TaaribKhatt khatt)` | Destroys a font handle. | Chains holding the font keep their own references and survive. |
| `taarib_khatt_huwiya(TaaribSiyaq siyaq, TaaribKhatt khatt, uint64_t *khuruj)` | The font's stable identity — the hash of its bytes, as carried in every glyph key and cache key. | Writes one caller-owned integer. |
| `taarib_khatt_qiyasat(TaaribSiyaq siyaq, TaaribKhatt khatt, float hajm, TaaribQiyasatKhatt *khuruj)` | The font's metrics scaled to `hajm` pixels. | Writes a caller-owned structure. |
| `taarib_khatt_aila(TaaribSiyaq siyaq, TaaribKhatt khatt, uint8_t *hadaf, size_t siaa, size_t *matlub)` | The font's family name. | Caller buffer, negotiated capacity. |
| `taarib_silsila_insha(TaaribSiyaq siyaq, const TaaribKhatt *khutut, size_t adad, TaaribSilsila *khuruj)` | Builds a fallback chain from `adad` font handles, tried in order per character. | The handle array is read during the call only. The chain takes its own references to the fonts. Handle owned until `taarib_silsila_ihdham`. |
| `taarib_silsila_ihdham(TaaribSiyaq siyaq, TaaribSilsila silsila)` | Destroys a chain. | Releases the chain's references; the font handles remain valid. |
| `taarib_takhtit(TaaribSiyaq siyaq, const TaaribTalab *talab, TaaribMakhzanTakhtit *makhzan)` | The hot path: lays out one request into the caller's buffer, allocation-free, cache-first. When capture is on, a cache miss reports its text to the registered sink before the call returns. | Request pointers are read during the call only. The glyph and line arrays are caller memory; on `TAARIB_SIAT_QASIRA` the required counts are written and the arrays are untouched. No pointer is retained. |
| `taarib_qiyas(TaaribSiyaq siyaq, const TaaribTalab *talab, TaaribQiyasNass *khuruj)` | Measures a request without positioning glyphs — same pipeline, same policies, so the number agrees with what layout would draw. | Writes a caller-owned structure. |
| `taarib_makhzan_ihsaat(TaaribSiyaq siyaq, TaaribIhsaatKhazina *khuruj)` | The layout cache's counters: hits, misses, evictions, bytes held, the byte budget, entries. Diagnostics, not control — a cache that never hits is a cache whose key is wrong, and this is how the Diagnostics screen says so with numbers. The counters are cumulative for the context's life and survive `taarib_siyaq_amsah`. | Writes a caller-owned structure. |
| `taarib_lawha_insha(TaaribSiyaq siyaq, uint16_t aqsa_ard, uint16_t aqsa_irtifa, uint16_t hashw, uint32_t namat, size_t mizaniya, TaaribLawha *khuruj)` | Creates a runtime atlas: page limits, padding, coverage or SDF, byte budget. | Handle owned until `taarib_lawha_ihdham`. |
| `taarib_lawha_ihdham(TaaribSiyaq siyaq, TaaribLawha lawha)` | Destroys an atlas. | Invalidates every page pointer previously lent by `taarib_lawha_safha`. |
| `taarib_lawha_ibda_itar(TaaribSiyaq siyaq, TaaribLawha lawha)` | Marks a frame boundary, so eviction can reclaim rectangles from previous frames but never one promised to the current frame. | No ownership change. Call once per frame before the frame's lookups. |
| `taarib_lawha_shakl(TaaribSiyaq siyaq, TaaribLawha lawha, TaaribSilsila silsila, const TaaribMiftahShakl *miftah, TaaribMawdiShakl *khuruj)` | Where a glyph lives — rasterizing, packing, and possibly evicting or opening a page on a miss. | Writes a caller-owned structure. May invalidate previously lent page pointers. |
| `taarib_lawha_safha(TaaribSiyaq siyaq, TaaribLawha lawha, uint16_t fahras, TaaribSafha *khuruj)` | Page `fahras`: texel pointer, dimensions, mode. | **The one borrowed pointer in the surface.** `khuruj->bayt` points into the atlas and is valid until the next `taarib_lawha_shakl` or `taarib_lawha_ihdham`. Upload and let go. |
| `taarib_lawha_adad_safahat(TaaribSiyaq siyaq, TaaribLawha lawha, uint32_t *khuruj)` | How many pages are open. | Writes one caller-owned integer. |
| `taarib_lawha_ihsaat(TaaribSiyaq siyaq, TaaribLawha lawha, TaaribIhsaatLawha *khuruj)` | The atlas's counters: hits, misses, evictions, growth, bytes against budget. A miss count that keeps climbing after the first minutes of play means the patch compiler missed strings. | Writes a caller-owned structure. |
| `taarib_iltiqat_shaghghil(TaaribSiyaq siyaq, TaaribIltiqatFn radd, void *mustakhdim)` | Turns runtime string capture on: registers the sink that receives every layout request this context has not seen. A null `radd` is refused; registering again replaces the previous sink atomically with respect to layout calls. | `radd` and `mustakhdim` must stay valid until capture stops or the context dies. `mustakhdim` is carried through untouched and never dereferenced. The text later handed to the sink is borrowed for the duration of each invocation only. |
| `taarib_iltiqat_awqif(TaaribSiyaq siyaq)` | Turns capture off and drops the sink. When it returns, no delivery is in flight and none will follow, so the sink and its `mustakhdim` may be torn down immediately. Stopping capture that was never started succeeds. | Releases the library's use of the registered pair; nothing else changes. |
