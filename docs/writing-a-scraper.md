# Writing a `GenericScraper` config

This documents the YAML format consumed by `GenericScraper`
(`src/scraper/generic.rs`) and `MangaScraperConfig` (`src/config/`). It's
everything you need to add support for a new manga site, or a new family of
sites that share a template/CMS, without touching Rust code.

If you just want the formal shape, `schema/config.schema.yaml` (plus
`schema/array_selector.schema.yaml` / `string_selector.schema.yaml`) is the
JSON-Schema version editors can validate against. This document explains the
*semantics* behind that shape — what actually happens at scrape time, and the
non-obvious behaviors you'll hit while writing selectors.

## Before you write a config: is this even scrapable?

This engine fetches raw HTML over HTTP and runs CSS selectors against it.
There is no JS engine. Two failure modes look similar from the outside but
are fundamentally different in how fixable they are:

- **Cloudflare "I'm Under Attack Mode"** (HTTP 403, body contains "Just a
  moment..."/"cloudflare"): detected automatically as `ScrapeError::CloudflareIUAM`.
  Not fixable by config — see `tests/manga.rs` for the existing
  `#[ignore = "CloudflareIUAM"]` entries.
- **A page that returns 200 with real HTML, but the data you need (usually
  chapter images) is filled in by client-side JS after the fact** — e.g. a
  `window.location.replace(...)` redirect gate with a signed token, or images
  loaded via a signed/rotating-key AJAX call whose response is itself
  obfuscated/packed JS rather than HTML or JSON. No amount of CSS selector or
  regex cleverness fixes this; the value literally isn't in the document
  `kuchiki` parses.

Before investing time in a config: fetch the manga page **and** an actual
chapter/reader page with `curl` using a real browser User-Agent, and check
that the `<img>` tags in the chapter page already carry real image URLs (in
`src` or a `data-*` attribute) rather than a loading placeholder. If the
image URL only appears after an AJAX call that itself needs a computed key,
this scraper can't handle it — don't build the config, just note the site as
unsupported.

## Top-level file anatomy

```yaml
# yaml-language-server: $schema=../schema/config.schema.yaml
name: my-site          # config name; shows up in error messages
accept: { ... }         # how to recognize URLs this config handles
manga: { ... }          # selectors for the manga "home" page
images: { ... }          # selectors for a chapter reader page
search: [ ... ]          # optional, per-hostname search support
date_formats: [ ... ]     # extra chrono strftime formats this site uses
```

One YAML file can cover many hostnames that share a template (see
`configs/madara.yaml`, which handles a dozen "Madara" WordPress-theme sites
via one shared config), or a single site (`configs/mangakakalot.yaml`). Reuse
an existing config as your starting template whenever the target site's
markup looks like a known CMS/theme — you'll often only need to add a
hostname or tweak a couple of selectors rather than write a config from
scratch.

Configs are compiled into the binary at build time (`configs/*.yaml` via
`include_dir!`) — edit the yaml, rebuild, done. There's no separate runtime
copy to keep in sync.

## The `accept` block

```yaml
accept:
  selectors:
    - "[content*=madara]"
    - body.wp-manga-template-default
  hostnames:
    - isekaiscan.top
```

A URL is routed to a config if **either**:
- its hostname is an exact match in `accept.hostnames`, **or**
- any one of `accept.selectors` matches somewhere in the fetched page (DOM
  fingerprinting — `doc.select_first(selector).is_ok()`, checked against the
  whole document).

Both lists are optional; you can rely purely on hostnames (like
`mangakakalot.yaml`) or purely on DOM fingerprints, or mix both (`hostnames`
as a fast/explicit path for known mirrors, `selectors` to catch new/unlisted
mirrors of the same template automatically).

**Multiple configs can accept the same URL.** `manga()` tries every accepting
config in turn and returns the first one that successfully scrapes *all*
required fields; failures from earlier configs are collected but otherwise
ignored (surfaced together only if every config fails). This means:
- Loose/generic DOM fingerprints are fine — a false-positive match will
  simply fail to find its required selectors and fall through.
- Don't rely on `accept.selectors` being a *unique* fingerprint; it only
  needs to be broad enough to catch real matches for its own selectors to
  then work.

## Selector engine

Selectors are standard CSS (via a `kuchiki` fork with the `selectors`
crate), so normal combinators (`div > p`, `a + span`, `~`), attribute
selectors (`[href*=madara]`, `[data-tip]`, `[og:site_name='Weeb Central']`),
and structural pseudo-classes (`:first-child`, `:nth-child(2)`, `:last-child`)
all work as you'd expect from `document.querySelectorAll`.

On top of that, this fork adds a few pseudo-classes you'll see throughout the
existing configs:

| Pseudo-class | Matches when |
|---|---|
| `:contains("text")` | the element's full text content contains `text` (case-sensitive) |
| `:icontains("text")` | same, case-insensitive |
| `:has(selector)` | the element has a descendant matching `selector` |
| `:has-not(selector)` | the element has **no** descendant matching `selector` |
| `:empty` / `:not-empty` | element has no / has children |

`:has()`/`:has-not()`'s argument is a full selector, so quote it when it
contains its own pseudo-class or attribute syntax:
`div.summary-heading:has('h5:icontains("alternative")')`. Note `:has()`
searches descendants of the candidate element, not siblings — pair it with a
parent selector when you need "the sibling near a label" pattern, e.g.
`td:has('i.info-status') + td` (weebcentral/mangakakalot both use this "find
the label icon, then take the next cell" idiom for label:value table rows).

`:active`, `:hover`, `:focus`, `:visited`, `:checked`, etc. parse but always
evaluate to false against static HTML — don't use them.

**All selection is inclusive-descendant search from whatever node you're
selecting on.** When a selector list has multiple elements matching (e.g.
`chapter.base`), each matched element becomes its own scope: the per-chapter
`title`/`url`/`number`/`date` selectors search within (and including) that
one element, not the whole page.

## `StringSelectors` vs `ArraySelectors`

Every selector field in the config (title, description, chapter.url, images'
`image_selector`, etc.) is one of these two types, and both accept the same
three shapes in YAML:

```yaml
# 1. Bare string — shorthand for {selector: "...", options: <defaults>}
title: div.post-title h1

# 2. A single map with explicit options
title:
  selector: div.post-title h1
  options:
    fix_capitalization: title

# 3. A list of alternatives, tried in order until one produces non-empty text
title:
  - selector: div.post-title h1        # variant A of the shared template
    options:
      fix_capitalization: title
  - selector: div#manga-title h1       # variant B of the shared template
    options:
      text_selection: { type: own-text }
      fix_capitalization: title
```

**The list form is the main tool for supporting multiple site variants under
one config.** It is *not* "pick the best selector" — it's "try selector 1;
if it comes back empty, try selector 2; ...". This is how `madara.yaml`
supports both this-week's Madara markup and slightly older variants with one
`title:` field, and how `chapter.fetch_external`/`images.fetch_external`
entries are tried in order (see below).

The difference between the two types: `StringSelectors` produces one
`Option<String>` (first selector in the list whose *first matched element
(s)* yield non-empty text wins). `ArraySelectors` produces a `Vec<String>`
by collecting text from **every** element the winning selector matches
(e.g. `authors: div.author-content a` collects one string per `<a>`).

## Selector `options`

Both `StringSelectorOptions` and `ArraySelectorOptions` share the same three
knobs, applied in this order — **text extraction → trim → cleanup → fix
capitalization → (array only) split**:

### `text_selection`
How to turn a matched element into a string. Default is `all-text` joined by
a single space.

```yaml
options:
  text_selection:
    type: all-text        # every descendant text node, concatenated
    join_with: " "         # separator between text nodes (default: " ")
```
```yaml
options:
  text_selection:
    type: own-text          # only this element's direct text children,
                              # ignoring nested elements (good for stripping
                              # a trailing "more" link, badges, etc. sitting
                              # inside the same element)
```
```yaml
options:
  text_selection:
    type: attributes
    attributes:               # tried in order, first one present wins
      - data-src
      - src
```

**Gotcha:** for a singular (`StringSelectors`) field, `attributes` reads the
attribute directly off the matched element(s) — it does *not* search that
element's descendants. Your selector must resolve to the actual tag carrying
the attribute (`img.cover`, not `div.cover-wrapper`). Array-selector
per-item lookups (`select_string_array`, used for `authors`/`genres`/
`alt_titles`/`image_selector`) are more forgiving and *do* fall back to
descendant search — but it's still best practice to select the exact
element.

### `cleanup`
An ordered list of regex replacements, applied to the extracted (trimmed)
text before capitalization/splitting:

```yaml
options:
  cleanup:
    - replace_regex: "^Status : "
      replace_with: ""
    - replace_regex: "Scifi"
      replace_with: "Sci-Fi"
```
`replace_with` supports standard `regex` crate replacement syntax
(`$1`, `${name}`, etc. for capture groups).

**Gotcha:** for a singular `StringSelectors` field, if a selector's
extracted text is non-empty *before* cleanup but cleanup reduces it to an
empty string, the whole field resolves to `None` immediately — it does
**not** fall through to try the next selector alternative in the list. Only
an initially-empty match (selector found nothing, or matched an empty
element) triggers fallback to the next alternative. Array selectors don't
have this trap: an item that cleans up to empty is just skipped, and the
per-selector fallback still applies to the selector as a whole.

### `fix_capitalization`
```yaml
options:
  fix_capitalization: title   # Title Case via `convert_case`
  # or: skip (default) — leave text as-is
```
Useful for sites that shout genre/title text in all-caps or all-lowercase.

### `text_split_regex` (array selectors only)
Some sites expose an array field as one blob of delimited text instead of
one element per item (`"martial arts, shonen, school"`). Default splits on
` *[,;\-|]+ *` (comma/semicolon/dash/pipe, with surrounding whitespace).

```yaml
options:
  text_split_regex: ","        # custom delimiter
# or
options:
  text_split_regex: null       # disable splitting — selector already
                                 # yields one matched element per array item
                                 # (e.g. multiple <img> tags for images)
```
Set this to `null` whenever your selector already matches one element per
array item (most `image_selector`s do this); otherwise the default regex
runs against each item's text too, usually harmlessly, but can misfire on
image URLs containing `-`.

## The `manga` block

```yaml
manga:
  title: ...            # required
  description: ...      # required
  cover_url: ...          # optional
  status: ...             # technically optional in the schema — see below
  authors: ...             # optional, ArraySelectors
  genres: ...              # optional, ArraySelectors
  alt_titles: ...           # optional, ArraySelectors
  chapter: { ... }           # required, see below
```

`title`, `description`, and `chapter` are required both by the schema *and*
at runtime — the scraped `Manga` struct has no default for them, so if your
selector fails to find text on a real page, that config's scrape attempt
fails outright for that URL.

**`status` is a trap.** The Rust type is `Option<StringSelectors>`, so the
schema and compiler both let you omit it — but `is_ongoing: bool` on the
resulting `Manga` has no default value either. `is_ongoing` is *only* ever
set as a side effect of successfully resolving a `status` selector
(`generic.rs::full_manga`). Omit `status`, or point it at a selector that
sometimes finds nothing, and the whole manga fails to build with a
`WebScrapingError` on that page — even though every other field scraped
fine. **Always give `status` a selector that reliably matches on every real
manga page for this site.**

Status text is normalized like this (`GenericScraper::manga_status`,
case-insensitive exact match against the *whole* cleaned string):

```
"ongoing" | "on-going" | "updating" | "live"  ->  is_ongoing = true
anything else (including "Completed", "Hiatus", "Dropped", "N/A", ...)
                                                ->  is_ongoing = false
```

If the site's wording doesn't match one of those four, add a `cleanup`
regex to normalize it (e.g. replace `"Updating"` variants some sites use),
rather than trying to extend the matcher — the matcher itself isn't
configurable per-site.

`cover_url`, `authors`, `genres`, `alt_titles` are genuinely optional —
omit them freely if the site doesn't expose the data; the corresponding
test entry in `tests/manga.rs` can list them under `ignore = [...]` too.

## The `chapter` block

```yaml
chapter:
  base: li.wp-manga-chapter        # required — one match per chapter
  title: a                          # required, scoped to each `base` match
  url:                               # required
    selector: a
    options:
      text_selection: { type: attributes, attributes: [href, src] }
  number: span.chapter-number        # optional
  date: span.chapter-release-date i   # optional
  fetch_external: [ ... ]              # optional, see below
```

- `base` selects one element per chapter; every other selector in this block
  runs scoped to that one element (inclusive of itself).
- `url` is resolved relative to the **manga page's URL** (`Url::join`), not
  each chapter's own address.
- `number`: if the selector is missing or finds nothing, falls back to the
  chapter's `title` text. Either way, the first decimal number found via
  regex (`\d+(\.\d+)?`) becomes the chapter number. If no number can be
  parsed at all, chapters get numbered by **reverse list position**
  (`total_chapters - index`) — this assumes `base` lists chapters
  newest-first, which is true of every site currently configured. If a
  site lists oldest-first, this fallback will number backwards; make sure
  `number` (or `title`) actually contains a parseable number for that site.
- `date`: parsed by a fairly extensive built-in date parser
  (`src/util/date.rs`) *before* consulting `date_formats` — it already
  understands ISO-8601 variants, epoch millis, ~20 common `Month Day Year
  [Time]` permutations, and relative phrases like `"2 days ago"`,
  `"yesterday"`, `"3 months ago"`, and `"now"/"today"/"latest"/"hot"`
  (treated as "just now"). Only add to `date_formats` (top-level config
  field, shared with `search.posted`) for formats the built-in list doesn't
  already cover — check `DEFAULT_DATE_FORMATS` in `src/util/date.rs` before
  adding one that might be redundant. Formats use
  [chrono strftime syntax](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

### `fetch_external` (chapters and images)

Some sites don't render the chapter list (or the reader images) directly in
the page you fetch — they load it via a secondary request the page's own JS
would normally trigger. `fetch_external` lets you replicate that *as long as
it's a plain HTTP request whose target URL/id can be read straight out of
the initially-fetched HTML* (not computed by JS).

```yaml
fetch_external:
  - id: script:icontains('mangaid')      # StringSelectors — find some text
    regex: var mangaID = '(?<id>\d+)';    # must contain a named `id` group
    url: /ajax-list-chapter?mangaID={id}    # {id}, {host}, {url} get substituted
    method: get                              # get (default) or post
```

Each entry is tried in order; the **first** one whose `id` selector finds
text *and* whose `regex` matches that text (populating the named `id`
group) wins — its `url` (template-substituted, then resolved relative to
the original page URL) is fetched, and the resulting parsed document
*replaces* the current one for every selector below it in the config
(`chapter.base` for chapter fetch_external, `image_selector` for images'
fetch_external). If no entry matches, the original document is used as-is
— so it's safe to list several fallback strategies for different site
variants under one config (`madara.yaml` has four — three HTML-based, one
JSON-based, see below).

Template variables available in `url`:
- `{id}` — the named capture group from `regex`
- `{host}` — the original page's hostname
- `{url}` — the original page's full URL as a string (note: no separator is
  inserted, so if you need `{url}` immediately followed by a path segment,
  make sure the matched URL already ends in `/`, as `weebcentral.yaml` relies
  on: `url: "{url}ajax/chapters/"`)

A `regex: "(?<id>.*)"` (capture everything) is a common pattern when the
useful "id" is really a full pre-built URL sitting in an attribute (e.g. an
htmx `hx-get` attribute, or a `<link rel="canonical">` href you then append
a suffix to) rather than a numeric ID you need to interpolate.

### `fetch_external` against a JSON API (`json_array` / `json_url_template`)

Some sites (and `search` endpoints — see below) hand back a plain JSON API
response instead of HTML or an HTML AJAX fragment. `fetch_external` (and
`search`) can consume these too, via two extra optional fields alongside
`id`/`regex`/`url`/`method`:

```yaml
fetch_external:
  - id: script:icontains('comicSlug')
    regex: comicSlug = '(?<id>[^']+)';
    url: "https://{host}/api/comics/{id}/chapters"
    json_array: /data/chapters             # RFC 6901 JSON Pointer to an array
    json_url_template: "{url}/{chapter_slug}"  # optional, see below
```

When `json_array` is set, the fetched response is parsed as **JSON instead
of HTML**. The engine navigates to that path (an
[RFC 6901 JSON Pointer](https://datatracker.ietf.org/doc/html/rfc6901) —
`/data/chapters` means "the `chapters` array inside the top-level `data`
object") and requires it to resolve to an array. Each array element (must
itself be a JSON object; non-object elements are skipped) is flattened into
a synthetic element:

```html
<div class="json-item" data-chapter_num="266" data-chapter_name="Chapter 266" data-chapter_slug="chapter-266" data-updated_at="2025-09-29T23:45:54.000000Z"></div>
```

This is parsed through the **exact same HTML/CSS pipeline as everything
else** — nothing downstream needs to know or care that the document was
originally JSON. Point `chapter.base` at `div.json-item` (add it as a
fallback alternative alongside your HTML-based selector, see below) and
read fields with the normal `attributes` text_selection
(`attributes: [data-chapter_num]`, etc.).

Flattening rules: strings pass through as-is; numbers/booleans become their
string form; `null` fields are omitted entirely (so an `attributes` list
with a fallback will correctly skip past them); nested objects/arrays are
JSON-stringified rather than silently dropped, in case you ever need to
regex into them via `cleanup`.

**`json_url_template`** exists for APIs (like the one above) that don't
include a directly usable URL or path for each item — only a slug or ID.
It's a template string substituted the same way `url` is (`{host}`, `{url}`
— the *original page's* hostname/URL, not the JSON API's), plus one more
kind of placeholder: `{<field>}` for any of that same item's own flattened
fields (`{chapter_slug}` in the example above). The computed result is
exposed as one more synthetic attribute, `data-__url`, which you then read
from `chapter.url` like any other attribute.

Because `chapter.base`/`title`/`url`/`number`/`date` are shared across every
`fetch_external` alternative in a config, the usual pattern is to make each
of them a **fallback list**: your existing HTML-based selector first, then
one more alternative reading the matching `data-*` attribute off
`div.json-item`. Since fallback-list alternatives are tried strictly in
order, this is safe to bolt onto an existing config — sites that never hit
the JSON `fetch_external` branch keep matching their original (first)
alternative exactly as before; only a page whose document actually got
replaced by the JSON-flattening step will ever match `div.json-item`. See
`configs/madara.yaml`'s `chapter` block for a real example (manhwafan.com
needs this; every other Madara-family site keeps working off the first
alternative in each list, untouched).

`search` supports the identical `json_array`/`json_url_template` pair for
JSON-backed search endpoints (a common pattern for WordPress "live search"
AJAX actions), using the search page's own hostname/URL as the `{host}`/
`{url}` context instead of a manga page's.

## The `images` block

```yaml
images:
  fetch_external: [ ... ]     # optional, identical mechanism to chapter's
  image_selector:              # required, ArraySelectors
    - selector: img.wp-manga-chapter-img, li.blocks-gallery-item img
      options:
        text_selection: { type: attributes, attributes: [data-src, src] }
        text_split_regex: null   # one <img> per page image — don't split
    - selector: p#arraydata        # fallback: some variants embed all image
      options:                       # URLs as one comma-separated blob in a
        text_split_regex: ","          # hidden element instead of real <img>s
```

This runs against whatever page `chapter_images(chapter_url)` fetches (after
`images.fetch_external`, if any, swaps in a different document). Resulting
URLs are resolved relative to that page's own URL.

## The `search` block (optional)

```yaml
search:
  - hostnames:                     # NOTE: independent of accept.hostnames!
      - lhtranslation.net
      - manhuaplus.com
    search_url: "{hostname}/?s={query}&post_type=wp-manga"
    query_format:                    # cleanup pipeline applied to the query
      - replace_regex: '\+'            # BEFORE it's substituted into search_url
        replace_with: "%2B"
      - replace_regex: " "
        replace_with: "+"
    selectors:
      base: ".c-tabs-item__content"        # required — one match per result
      url: { selector: a, options: {...} }    # required
      title: { selector: h3 a, ... }           # required
      cover_url: img                            # optional
      posted: div.post-on span                   # optional, parsed via date_formats
```

**`search[].hostnames` is a completely separate list from
`accept.hostnames`.** A hostname doesn't automatically get search support
just because a config accepts it for manga scraping — you opt a hostname
into search explicitly here, and a URL missing from every `search[].hostnames`
list simply reports `SearchNotSupported`. `search_url` gets `{hostname}`
and `{query}` substituted (`https://` is prepended automatically if the
result doesn't already start with `http`).

You can list one `search` entry with many hostnames sharing the same search
UI (as `madara.yaml` does), or multiple entries if different hostnames under
one config need different search markup/URLs.

## `date_formats`

A flat list of extra [chrono strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)
patterns, shared by `chapter.date` and `search.posted`. Only needed for
formats the built-in parser doesn't already cover (see `chapter.date`
above) — most existing configs only need two or three entries for a site's
one or two unusual formats.

## Step-by-step: adding a new site

1. **Pick a real manga URL on the target site** with more than one chapter.
2. **Fetch it like a browser and sanity-check it isn't gated:**
   ```sh
   curl -sL -A "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36" \
     "https://example.com/manga/some-title/" -o /tmp/page.html -w "%{http_code}\n"
   grep -io "cloudflare\|just a moment" /tmp/page.html   # empty output = good
   ```
3. **Pretty-print it so you can actually read the structure:**
   ```sh
   python3 -c "from bs4 import BeautifulSoup; print(BeautifulSoup(open('/tmp/page.html'), 'html.parser').prettify())" > /tmp/page_pretty.html
   ```
   (or `xmllint --html --format`, whichever renders more usefully for that
   page).
4. **Also fetch a real chapter/reader page** and confirm the actual image
   URLs are present in `<img src>`/`data-src` right away — this is the one
   check that can't be worked around later. If images only appear via a
   dynamically-computed AJAX call, stop here; this site isn't a fit for
   `GenericScraper`.
5. **Check whether an existing config already fits.** If the site is running
   a known theme (Madara, a Mangakakalot clone, etc.), you may only need to
   add a hostname to an existing `accept.hostnames`/`search.hostnames` list
   rather than write new selectors.
6. **Write the yaml**, using the closest existing config as a template.
   Fill in `manga.title`/`description`/`status`/`chapter` first (the
   required set) and confirm those work before layering on
   `authors`/`genres`/`alt_titles`/`search`.
7. **Add a test entry** in `tests/manga.rs` under the right `test_manga_mod!`
   block (or a new one), with the real URL from step 1:
   ```rust
   mysite: "https://example.com/manga/some-title/";
   ```
8. **Iterate:**
   ```sh
   cargo test --test manga <mod>::mysite -- --nocapture
   ```
   Responses are cached under `http-cacache/` (`CacheMode::ForceCache`) —
   re-running is instant and offline once fetched. If the site's markup
   changes mid-session, delete the relevant cache entry (or the whole
   directory) to force a refetch.
9. Add `RUST_LOG=manga_parser=debug` to see the scraper's own trace logging
   (`fetch_doc_config`, `chapters`, `images`, `do_search` all log the
   selectors they're trying and how many matches/images they found):
   ```sh
   RUST_LOG=manga_parser=debug cargo test --test manga <mod>::mysite -- --nocapture
   ```
10. Once `title`/`description`/`chapters`/first-chapter-images pass, decide
    on `authors`/`genres`/`alt_titles`/`chapter_date`: if the site genuinely
    doesn't expose one of them, add it to that test entry's
    `ignore = [...]` list rather than faking a selector.

## Quick error reference

Errors you'll see while iterating (`src/error.rs`), and what they usually mean:

| Error | Likely cause |
|---|---|
| `SelectorError` | invalid CSS syntax in one of your selector strings |
| `WebScrapingError("Missing required field...")` | a required `StringSelectors` never found non-empty text — check the selector, or a `cleanup` regex wiping it to empty (see the fallback gotcha above) |
| `WebScrapingError("Missing required url...")` | same, for a `url`-typed selector |
| `WebsiteNotSupported` | no config's `accept` matched this hostname/DOM at all |
| `MultipleScrapingErrors` | one or more configs matched `accept` but every one failed to fully build a `Manga`/images list — the map is keyed by config name, check which one and why |
| `CloudflareIUAM` | got an HTTP 403; not config-fixable |
| `SearchNotSupported` | hostname isn't listed in any `search[].hostnames` |
