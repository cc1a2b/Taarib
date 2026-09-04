# التقديم — the submission transport, and the one thing it is waiting for

A contributor finishes a patch, the pre-flight gate passes, and the submission
travels to the registry as a pull request with the package attached to a staging
release. Every part of that is written and every part of it is tested. One value
is missing, and only the person who owns the registry account can produce it: an
OAuth **client identifier**. Until it exists the application says so, in both
languages, and keeps the submission on the contributor's machine.

This file records exactly what that step is, exactly where the value goes, and
exactly what the transport does on either side of it. It is derived from
`crates/taarib-taqdeem/src/irsal.rs`, from `docs/mustawda.md` §7, and from a set
of round trips against real servers on 2026‑09‑05 (§7).

---

## 1. The owner's remaining step

### 1.1 Register the application

Open **`https://github.com/settings/applications/new`**.

That URL is the one GitHub's own documentation links to as "the client ID you
received from GitHub when you registered". The click path to the same page is
profile picture → **Settings** → **Developer settings**
(`https://github.com/settings/developers`) → **OAuth apps** → **New OAuth App**
(the button reads **Register a new application** if this is the first one).

There is no organization equivalent for OAuth Apps. `github.com/organizations/<org>/settings/applications/new`
does not exist; an org-owned OAuth App is made by registering it personally and
then using **Transfer ownership**. If the registry should be owned by an
organization from the start, register a **GitHub App** instead — the device flow
works identically — at `https://github.com/settings/apps/new` or
`https://github.com/organizations/<ORG>/settings/apps/new`.

Fill the form as follows.

| field | value | why |
| --- | --- | --- |
| Application name | `Taarib` | Shown to the contributor on the authorisation page. |
| Homepage URL | `https://github.com/cc1a2b/taarib` | Shown on the same page. |
| Application description | optional | |
| Authorization callback URL | `https://github.com/cc1a2b/taarib` | See §1.2. |
| **Enable Device Flow** | **ticked** | See §1.3. This is the one that is easy to miss. |

Then press **Register application**. The **Client ID** is on the page that
follows. **Do not generate a client secret**: the device flow does not use one,
and this application never holds one.

### 1.2 About the callback URL

GitHub's documentation does not mark this field optional for an OAuth App, and
every other optional field on that form is explicitly prefixed "Optionally", so
assume it is required and put something valid in it. It is functionally inert
here: `redirect_uri` belongs to the web application flow, and the device flow's
token exchange sends only `client_id`, `device_code` and `grant_type` — nothing
in this codebase reads, sends or validates a callback URL. Any `https` address
you control is fine; the repository's own page is the least surprising choice.

For a **GitHub App** the field is documented as optional and ignored outright:
"If your app uses device flow instead of web application flow to generate a user
access token, this field will be ignored."

### 1.3 The device-flow tick box, which is not optional

> "Before you can use the device flow to authorize and identify users, you must
> first enable it in your app's settings."

For an **OAuth App** it is a checkbox labelled **Enable Device Flow**, sitting
between "Authorization callback URL" and "Expire user access tokens" on the
registration form, and editable afterwards under Settings → Developer settings →
OAuth apps → the app.

For a **GitHub App** it is under the heading **Identifying and authorizing
users**, same label.

If it is not ticked, the device-code endpoint answers `400` and the token
endpoint answers `device_flow_disabled`. This transport recognises that code by
name and refuses with a sentence naming the operator rather than the
contributor:

> the registered application does not have the device flow enabled; the registry
> operator has to turn it on in the application's settings

### 1.4 Scopes

Nothing is chosen at registration time — an OAuth App has no scope list in its
settings. The scope is requested per authorisation, and this build requests
exactly one:

```
public_repo
```

`IdadatTawthiq.nitaq`, set in `apps/studio/src-tauri/src/taqdeem_awamir.rs::idadat_tawthiq`.

`public_repo` is documented as "read/write access to code, commit statuses,
repository projects, collaborators, and deployment statuses **for public
repositories**", which covers all four things a submission does:

| step | endpoint | covered by |
| --- | --- | --- |
| fork the registry | `POST /repos/{owner}/{repo}/forks` | `public_repo` |
| push the branch | `git push` over HTTPS to the fork | `public_repo` |
| upload the package | `POST /repos/{owner}/{repo}/releases/{id}/assets` | `public_repo` |
| open the request | `POST /repos/{owner}/{repo}/pulls` | `public_repo` |

Two caveats worth knowing before the first live run:

- GitHub no longer publishes a classic-scope line for the fork, pull-request and
  release-asset endpoints — the fine-grained permissions are documented and the
  OAuth scope is not. The answer above comes from the scope table itself, and
  from the pattern every endpoint that *does* still publish one follows: it is
  repository **visibility**, not ownership, that decides between `public_repo`
  and `repo`. The registry is public, so `public_repo` is the right ask. If the
  registry is ever made private, this becomes `repo`.
- Add `workflow` **only** if a submission branch could ever touch
  `.github/workflows/`. It cannot: a submission writes one file,
  `sharaih/{shard}.json`.

At runtime GitHub answers every REST call with `x-oauth-scopes` (what the token
has) and `x-accepted-oauth-scopes` (what the endpoint wanted). If a step refuses
with `403`, read those two headers before changing anything else.

### 1.5 Where the client identifier goes

It is a **setting**, not a compiled constant. Nothing has to be rebuilt.

| | |
| --- | --- |
| field | `muarrif_amil` |
| declared in | `crates/taarib-usus/src/idadat.rs` → `IdadatMasadir::muarrif_amil: Option<String>` |
| on disk | `idadat.json`, under `masadir.muarrif_amil` — `%APPDATA%\Taarib\` on Windows, `~/Library/Application Support/Taarib/` on macOS, `$XDG_CONFIG_HOME/taarib/` on Linux |
| in the interface | Settings → **Sources** (`QismIdadat::Masadir`) |
| read by | `apps/studio/src-tauri/src/taqdeem_awamir.rs`, in `abda_tawthiq_taqdeem` and `sallim_taqdeem` |

```jsonc
{
  "masadir": {
    "rasmi": "https://github.com/cc1a2b/taarib-registry",
    "muarrif_amil": "Ov23li………",          // ← the Client ID from §1.1
    "rabt_tajheez": "https://uploads.github.com/repos/cc1a2b/taarib-registry/releases/<id>/assets?name=tajheez-{ism}"
  }
}
```

To ship it as a default for every install instead, change the `None` in
`IdadatMasadir::default()` to `Some("Ov23li…".to_owned())`. A client identifier
is public by design — it appears in every device-code request — so it is not a
secret and committing it is not a leak. **The client *secret* is, and there
should not be one.**

### 1.6 The second missing field, `rabt_tajheez`

The client identifier alone is not enough to complete a submission; the staging
upload endpoint has to exist too, and it depends on the registry repository,
which is a separate piece of work. Its rule is in `docs/mustawda.md` §7.2 and is
enforced by `IdadatMustawda::tahaqquq`:

- an `https` address containing **`tajheez`**,
- and never containing `isdar`,
- and never a `…/releases/download/…` address, which is not writable at all.

On GitHub the upload address carries a numeric release identifier and has
nowhere in its path to put a name, so the marker rides in the asset name:

```
https://uploads.github.com/repos/<owner>/taarib-registry/releases/<release id>/assets?name=tajheez-{ism}
```

Create a release on the registry repository tagged `tajheez`, read its numeric
`id` from `GET /repos/<owner>/taarib-registry/releases/tags/tajheez`, and paste
the address above with that id. The asset comes back as
`https://github.com/<owner>/taarib-registry/releases/download/tajheez/tajheez-<file>.ruqaa`,
which is what `irfa_ila_tajheez` checks before it lets the record point at it.

### 1.7 Confirming it worked

1. Put the client identifier in Settings → Sources and restart nothing.
2. Start a submission. The device-authorisation card should show a short code
   and `https://github.com/login/device`, valid for 15 minutes.
3. If instead you see *"the registry upload channel is not provisioned yet"*, the
   field is still empty or still holds a placeholder — see §3.
4. If you see *"does not have the device flow enabled"*, go back to §1.3.
5. If you see *"the forge does not recognise the client identifier"*, the value
   was pasted wrong or belongs to a deleted application.

---

## 2. Where the line falls

This is the whole point of the current state of the crate: the client identifier
gates a very small part of the transport, and everything else is finished.

| needs the client identifier | why |
| --- | --- |
| `ibda_tawthiq` | posts `client_id` to `https://github.com/login/device/code` |
| `istitla` | posts `client_id` + `device_code` to the token endpoint |
| `akmil_tawthiq`, `wathiq`, `ramz_mukhazzan_aw_tawthiq` | wrap those two |

| does **not** need it | what it needs instead |
| --- | --- |
| `bina_amil` | nothing — TLS, timeouts, the redirect ceiling, `https_only` |
| `QalabRabt::mila` | nothing — substitution and refusal of anything that is not a path segment |
| `IdadatMustawda::tahaqquq`, `qabul_tajheez` | nothing |
| `arsil`, `iqra_radd`, `taraju`, `RaddMahdud::{hadd_muadal,aarid,intizar}` | nothing — retry, backoff, rate-limit headers |
| `iqtibas`, `jalb_json` | nothing — error mapping and parsing |
| `hala_min_khata` | nothing — the device flow's *vocabulary*, as opposed to its requests |
| `tahaqquq_basma` | the package bytes |
| `mulakhkhas`, `wasf_talab_damj`, `tahdheerat`, `adif_ila_shareeha` | the compile's own output |
| `hawiya_muwaththaqa`, `shawka`, `irfa_ila_tajheez`, `iftah_talab_damj`, `adfa` | **a token** — which the device flow is one way to obtain and not the only way |
| `RamzWusul`, `khzin_ramz`, `hat_ramz`, `imsah_ramz` | the OS keychain |
| `SijillIrsal`, `DaftarIrsal` | the data root |

The last row of the second table is why the transport could be proved at all: the
four API calls take a `RamzWusul` and do not care where it came from.

---

## 3. The refusal when it is not provisioned

`KhataTaqdeem::IrsalGhayrMuhayya { naqis }`, code `7110`, severity `Tanbeeh`,
next step `FathIdadat { qism: Masadir }`.

> **English** — The registry upload channel is not provisioned yet: the
> `muarrif_amil` field is empty in Settings. Your submission stays recorded
> locally and is sent the moment the registry operator provisions the channel.
>
> **العربية** — قناة الرفع إلى السجلّ غير مجهّزة بعد: الحقل `muarrif_amil` فارغ
> في الإعدادات. بقي تقديمك مسجّلًا محليًا، ويُرفع تلقائيًا متى جهّز مشغّل
> السجلّ القناة.

The wording is the Studio's own, from
`KhataTaqdeemAmr::IrsalGhayrMuhayya`, so a contributor sees one sentence whether
the refusal is raised by the command layer before any work starts or by the
crate when the configuration is validated.

It is deliberately **not** `BayanNaqis` ("the submission cannot be sent without
…"), which is the draft missing something the contributor can supply. Telling a
contributor to fix a field only the operator can fill sends them looking for a
setting that was never theirs.

A blank value is not the only thing that raises it. `muarrif_muhayya` also
refuses a value shorter than eight characters, one carrying whitespace, a
control character or `{}<>`, and the words a half-filled settings file is left
holding (`TODO`, `changeme`, `your-…`, a run of one repeated character). A
placeholder that reached the forge would come back as a generic
`unauthorized_client` after a network round trip, and a contributor reads that
as "my account is wrong".

---

## 4. The sequence

`irsal::irsal` takes the gate proof by value, so a submission that did not pass
every blocking check is not representable at the call. Each step advances the
local ledger at `<data root>/taqdeem/irsal.json` before the next begins, so an
interrupted submission says where it stopped.

| step | `MarhalatIrsal` | what happens |
| --- | --- | --- |
| 1 | `Khatm` | the package is opened and its content hash compared to the record's — see §6 |
| 2 | `Bayanat` | the shard index and the asset file name are computed |
| 3 | `Shawka` | `GET /user` for the account name, then the fork is found or created and waited for |
| 4 | `Tajheez` | the package is uploaded to the staging release in 1 MiB chunks |
| 5 | `Far` | the base branch is fetched, the shard rewritten, committed and pushed to the fork |
| 6 | `TalabDamj` | `POST /pulls` opens the request against the base branch |

Steps 3, 4 and 6 go through `arsil`; step 5 is `git2` on a blocking thread with
the token as the HTTPS password.

Every API request carries:

```
Authorization: <scheme> <token>
Accept: application/json
X-GitHub-Api-Version: 2026-03-10
User-Agent: Taarib/<version> (+https://github.com/cc1a2b/taarib)
```

`Accept` is not decoration: without it the device endpoints answer
`key=value&key=value`, not JSON. `User-Agent` is not decoration either — GitHub
rejects a REST request that has none and answers `403` to one it considers
invalid. `X-GitHub-Api-Version` is optional and pinned anyway: unsent, the API
falls back to `2022-11-28`, whose support ends in March 2028.

---

## 5. Retries and rate limits

`SiyasatItada` is per call site, not a global knob.

| policy | used by | retries on |
| --- | --- | --- |
| `Aid` | every `GET`, the fork creation, the device-code and token polls | rate limits, `408/500/502/503/504`, and transport failures |
| `HaddFaqat` | the package upload, `POST /pulls` | rate limits only |

The distinction is about ambiguity, not caution. A rate-limit refusal means the
forge did not look at the request. A `502` from `POST /pulls` might mean the
request went through and the answer was lost, and sending it again would put a
second thing in the owner's queue for a person to read and close. Forking is in
the first group because forking an already-forked repository returns the
existing fork rather than making a second one.

Four attempts, backoff doubling from 500 ms, capped at 120 s — the same numbers
`taarib_tarjama::muzawwidun` uses for the same decision. No jitter: this is one
desktop application making one submission, not a fleet.

A refusal is read as a rate limit when it is `429`, or `403` carrying either
`retry-after` or `x-ratelimit-remaining: 0`. A `403` with neither is a
permissions refusal and is not repeated.

The wait, in the order the forge's own guidance gives:

1. `retry-after`, in whole seconds, when present.
2. Otherwise `x-ratelimit-reset` — but **only** when `x-ratelimit-remaining` is
   `0`. That header rides on every answer, spent window or not, and reading it
   off a gateway failure would park a submission until the top of the hour.
3. Otherwise, for a rate limit that named no wait at all — which is the
   secondary limiter, and there is no way to query it — one minute.
4. Never less than the exponential schedule, and never more than 120 s. Past
   that the transport stops and reports rather than holding a submission open in
   silence.

Only the delta-seconds form of `retry-after` is read. The HTTP-date form would
mean comparing a clock that is not ours against one that is, and misreading it
as an enormous wait would hang a submission; GitHub sends whole seconds.

---

## 6. Two invariants worth stating

**The package and its record are from the same compile.** `tahaqquq_basma` opens
the container about to be uploaded and compares its own content hash to
`record.basmat_muhtawa` before anything is sent. Without it, a record from one
compile beside a package file from another is a corruption nobody notices: the
upload succeeds, the pull request reads correctly, and every install of it fails
its integrity check months later.

**A submitted package never lands in the public release area.** Two checks. The
endpoint template is refused unless it names `tajheez` and never names `isdar`
(§1.6). And the address the forge answers with is checked the same way — that
one is binding, because by then the forge has said where the bytes actually
landed rather than where they were sent.

---

## 7. What was proved against real servers, and what was not

Run on 2026‑09‑05 from this working tree. Nothing was created, modified or
deleted on any GitHub account: every GitHub call below is a `GET`, and every
call that writes went to an echo service.

| what | how | result |
| --- | --- | --- |
| TLS, `User-Agent`, status handling | `GET https://api.github.com/rate_limit` through `bina_amil` + `arsil` | `200`, 427-byte body read under the ceiling |
| rate-limit envelope | the same response | `x-ratelimit-remaining: 27`, `x-ratelimit-reset: 1788560275` — 1 612 s ahead of the clock at the time — and `hadd_muadal()` correctly `false` on a `200` |
| error mapping | `GET https://api.github.com/user` with an unusable token | `MustawdaRafad { amal: "reading the authenticated identity", sabab: "HTTP 401: {\"message\": \"Bad credentials\", …}" }` |
| request shape of the upload | `POST https://httpbin.org/post` with the real chunked body, 2 097 159 bytes | the echo shows `content-length: 2097159`, `application/octet-stream`, the `Authorization`, `x-github-api-version: 2026-03-10`, the `Taarib/1.0.0 (+…)` agent — and **no** `transfer-encoding`, so the streamed body still arrived as one sized object |
| redirect ceiling | `httpbin.org/redirect/2` and `/redirect/6` | `200`, then refused at the fourth hop with "error following redirect" |
| retry and backoff | `httpbin.org/status/503` under both policies | `HaddFaqat` returned `503` in 327 ms; `Aid` returned `503` in 4.25 s, against a 3.5 s schedule of 0.5 + 1 + 2 |
| the response shapes | fixtures captured from live reads of `cli/cli` and `api.github.com/user` | see below |

The response fixtures matter more than they look. GitHub answers a pull request
with **both** `url` and `html_url`, and a release-asset upload with **both**
`url` and `browser_download_url`. Reading each pair as two spellings of one
field — `#[serde(rename = "html_url", alias = "url")]` — makes serde reject the
body as a *duplicate field*, so every successful call came back as a parse
failure. That is now two named fields and an explicit preference order, and both
real bodies are in the test suite as regressions.

**Not proved, and cannot be until §1 is done:** the device-code request, the
token poll, and therefore the four authenticated writes as one sequence. What is
proved about the device flow is its vocabulary — every documented error code maps
to the right outcome, and the two that mean "the application was never finished
being registered" say so in those words.

**Also not proved:** the `git2` half (fetch, commit, push) against a real fork,
because that needs a repository that does not exist yet, and creating it belongs
to a different piece of work.

To re-run the network tests:

```
cargo test -p taarib-taqdeem shabaka -- --ignored --test-threads 1
```

They need no credential. Everything a credential would unlock is the device
flow, and the device flow is the part that is waiting.
