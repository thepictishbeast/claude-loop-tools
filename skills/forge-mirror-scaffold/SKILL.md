---
name: forge-mirror-scaffold
description: Generate a PlausiDen-Forge cms/<slug>.json mirror page from a live URL. Runs crawler --capture-reference, extracts SSR title + meta description + h1-h3 headings, scaffolds an image_hero + paragraph + kv_pair + call_to_action page using existing Loom primitives, pre-resolves label_consistency with site-prefixed hrefs. Compresses what was ~150-line hand-authored CMS into one invocation. Use when the user says "mirror <url>", "scaffold a forge static for <site>", or for the PRIORITY 3 pixel-rep rotation work.
---

# /forge-mirror-scaffold — generate a Forge mirror page from a URL

Walks a live URL via `crawler --capture-reference`, extracts the SSR-visible content, and emits a valid `cms/<slug>.json` that passes `forge build` (0 strict findings). Designed for the PRIORITY 3 rotation (prosperityclub.com / plausiden.com / sacred.vote / Stripe / Linear / Vercel / GitHub / Notion / Anthropic / Render / Fly) where each page needs a working approximation rapidly.

## Why this exists

This-session experience: 11 Forge-static mirror pages hand-authored at ~150 LOC each. Each one followed the same pattern (`image_hero` with eyebrow + verbatim title + verbatim lede + CTA → `paragraph` disclaimer → `pull_quote` from a verbatim live quote → `kv_pair` 4-6 items mapping the live features → `heading` + `paragraph` → final `call_to_action`). Each one tripped the same `label_consistency` gate that needed site-prefixed hrefs.

That's ~1,600 LOC of mechanical CMS authoring that compresses to one command per site if a skill walks the SSR + emits the scaffold.

## Steps

### 1. Capture the live site

```sh
sudo -u paul /home/paul/projects/PlausiDen-Crawler/target/debug/crawler \
  --capture-reference "$URL" \
  --out-dir /tmp/captures
```

The runner emits `<slug>.png`, `<slug>.html`, `<slug>.styles.json`, and `manifest.json` in `/tmp/captures/<slug>/`. Confirms multi-viewport capture works against the target.

### 2. Extract content from the captured HTML

Read `/tmp/captures/<slug>/1280.html` and extract:

| Field | Source |
|-------|--------|
| `title` | `<title>` element body |
| `description` | `<meta name="description" content="…">` |
| Hero title | First `<h1>` (or `<title>` if no h1) |
| Hero lede | Same description as above |
| Features | First 4-6 `<h2>` / `<h3>` headings + adjacent `<p>` excerpts |
| Pull-quote | Longest distinct `<p>` over 100 chars |

If site is a JS-only SPA (Sacred Vote-class), fall back to `<meta property="og:title">` / `og:description` / `<title>` only — operator extends manually.

### 3. Emit the cms/<slug>.json scaffold

Standard 7-section shape:

```jsonc
{
  "$schema": "../cms-schema.json",
  "title":      "<extracted title>",
  "description":"<extracted description>",
  "brand":      "<derived from host>",
  "chrome":     "page_shell",
  "theme":      "<picked from {warm, light, dark, ocean}>",
  "nav_actions":[{ "label": "<host>", "href": "<URL>", "data_backend": "cta-platform" }],
  "path":       "/<slug>.html",
  "site_origin":"https://dev.plausiden.com",
  "footer":     { /* boilerplate + extracted footer-link headings */ },
  "nav_links":  [ /* 3-5 top-nav links scraped from live */ ],
  "sections":   [
    { "kind": "image_hero", "align": "start",
      "eyebrow": "<brand> — Forge static mirror",
      "title": "<hero title>", "lede": "<description>",
      "cta": { "label": "<host>", "href": "<URL>", "data_backend": "cta-platform" },
      "background": { "kind": "photo", "src": "/assets/photos/stock-66.jpg",
        "alt": "Editorial photograph illustrating <topic>.",
        "overlay": "dark" },
      "height": "compact"
    },
    { "kind": "paragraph",
      "text": "Forge-static approximation of <host> authored from SSR-extractable headings + meta description. Pixel-rep rotation target per PlausiDen-Forge #222. Not affiliated with <legal-name>."
    },
    { "kind": "pull_quote",
      "body": "<longest live <p> ≥ 100 chars>",
      "attribution": "<host>, SSR-extracted",
      "emphasis": "display"
    },
    { "kind": "kv_pair",
      "heading": "<section heading derived from h2>",
      "density": "comfortable",
      "items": [ /* 4-6 items from h3/h4 → title + adjacent <p> excerpt → hint */ ]
    },
    { "kind": "heading", "level": 2, "text": "<second h2>" },
    { "kind": "paragraph", "text": "<adjacent <p> body, substrate-correct reframe>" },
    { "kind": "call_to_action", "align": "start",
      "eyebrow": "Reference",
      "title": "Open the live <brand> site.",
      "lede": "This is the Forge-static approximation. The actual product lives at the canonical URL.",
      "cta": { "label": "<host>", "href": "<URL>", "data_backend": "cta-platform" }
    }
  ]
}
```

### 4. Pre-resolve label_consistency

The cross-page `label_consistency` gate (forge-phases) flags multiple distinct labels for the same href. Empirically: 7 of 11 rotation pages this session tripped it. Pre-resolutions the scaffolder applies:

- Site-prefixed hrefs for nav_links / footer.columns when paths would collide with prosperityclub.com (e.g. `/<slug>-contact/` instead of `/contact/`).
- Unify all anchors targeting the live root URL on label = `<host>` (drop "Open", "Visit", "View" etc. prefixes).
- For shared official paths (`/about/` / `/contact/` etc.), namespace under `/<slug>-about/` / `/<slug>-contact/`.

### 5. forge build + deploy verification

```sh
cd /home/paul/projects/PlausiDen-Forge
sudo -u paul ./target/release/forge build 2>&1 | tail -5
# expect: strict findings: 0 + advocacy coverage: N/N (100%)

rsync -a /home/paul/projects/PlausiDen-Forge/static/ /var/www/dev.plausiden.com/
chown -R caddy:caddy /var/www/dev.plausiden.com/
curl -sS -w 'http=%{http_code}\n' -o /dev/null "https://dev.plausiden.com/<slug>.html"
```

If forge build fails on `label_consistency`: drop the conflicting label by uniformizing to the host name.

### 6. Commit + push (optional, operator-driven)

The skill output to stdout is the JSON content; the operator decides where to write it. Recommended path: `cms/<slug>.json` under the Forge repo root.

```sh
sudo -u paul git -C /home/paul/projects/PlausiDen-Forge add "cms/<slug>.json"
sudo -u paul git -C /home/paul/projects/PlausiDen-Forge commit -m "cms/<slug>.json: Forge-static approximation of <host>"
sudo -u paul git -C /home/paul/projects/PlausiDen-Forge push origin main
```

## Out of scope

- Image assets — uses the existing `/assets/photos/stock-66.jpg` placeholder. Bespoke imagery requires a separate asset request to the operator.
- Pixel-level layout match — the scaffold produces working content; visual polish (column counts, theme overrides, custom primitives) lives in subsequent iterations.
- JS-required SPA sites — if the live page has thin SSR (Sacred Vote / Fly.io), the scaffolder produces a minimal version from og: metadata; the operator extends manually.
- LFI repo content — out of scope per [[feedback_lfi_out_of_scope_for_this_instance]].

## Related skills

- `workspace-tests` — verify the post-scaffold build doesn't regress
- `fmt-sweep` — canonicalize any auxiliary code changes the scaffold drove

## Don't

- Don't fabricate live content — every section's verbatim copy must come from the captured HTML or be marked as a substrate-correct reframe in a disclaimer.
- Don't omit the disclaimer paragraph + colophon — the rotation pages are not affiliated with the target brands; the disclaimer is doctrinal not optional.
- Don't ship a scaffolded page without running `forge build` to catch label_consistency / path_consistency / phantom_button gates.
