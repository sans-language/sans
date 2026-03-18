# Website Design System Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the Sans website from a custom Sans server to GitHub Pages with a Catppuccin light/dark design system, web components, and modern typography.

**Architecture:** Static site served by GitHub Pages. CSS custom properties power the light/dark theme. Two web components (`<sans-nav>`, `<sans-footer>`) provide shared UI. Directory-based routing gives clean URLs.

**Tech Stack:** HTML, CSS (custom properties), vanilla JS (Web Components API), GitHub Actions (deploy), woff2 self-hosted fonts.

**Spec:** `docs/superpowers/specs/2026-03-17-website-design-system.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `website/css/style.css` | Create | Complete stylesheet with Catppuccin Latte/Mocha theming |
| `website/js/components.js` | Create | `<sans-nav>` and `<sans-footer>` web components |
| `website/fonts/*.woff2` | Create | Self-hosted Source Sans 3 + Source Code Pro |
| `website/.nojekyll` | Create | Prevent Jekyll processing |
| `website/index.html` | Migrate | Homepage (from `website/static/index.html`) |
| `website/docs/index.html` | Migrate | Docs (from `website/static/docs.html`) |
| `website/benchmarks/index.html` | Migrate | Benchmarks (from `website/static/benchmarks.html`) |
| `website/download/index.html` | Migrate | Download (from `website/static/download.html`) |
| `website/404.html` | Create | Custom 404 page |
| `.github/workflows/deploy-site.yml` | Create | GitHub Pages deploy workflow |
| `.github/workflows/release.yml` | Modify | Update version-bump sed paths |
| `examples/website-server/main.sans` | Move | Relocate Sans server out of deploy path |

---

## Chunk 1: Foundation (fonts, CSS, components, structure)

### Task 1: Create directory structure and download fonts

**Files:**
- Create: `website/css/` directory
- Create: `website/js/` directory
- Create: `website/fonts/` directory (with 5 woff2 files)
- Create: `website/docs/` directory
- Create: `website/benchmarks/` directory
- Create: `website/download/` directory
- Create: `website/.nojekyll`

- [ ] **Step 1: Create directories**

```bash
cd /Users/sgordon/Development/Repos/sans
mkdir -p website/css website/js website/fonts website/docs website/benchmarks website/download
touch website/.nojekyll
```

- [ ] **Step 2: Download Source Sans 3 fonts**

Fetch the Google Fonts CSS to extract woff2 URLs, then download them. Use a User-Agent header that triggers woff2 format:

```bash
# Fetch CSS with woff2 user-agent to get woff2 URLs
curl -sS "https://fonts.googleapis.com/css2?family=Source+Sans+3:wght@400;600;700&display=swap" \
  -H "User-Agent: Mozilla/5.0" | grep -oP 'url\(\K[^)]+\.woff2' | head -3

# Download each weight (URLs will be from output above)
# Save as:
#   website/fonts/SourceSans3-Regular.woff2
#   website/fonts/SourceSans3-SemiBold.woff2
#   website/fonts/SourceSans3-Bold.woff2
```

If URL extraction is difficult, use the fontsource npm package or download from https://github.com/google/fonts/tree/main/ofl/sourcesans3 directly.

- [ ] **Step 3: Download Source Code Pro fonts**

```bash
curl -sS "https://fonts.googleapis.com/css2?family=Source+Code+Pro:wght@400;600&display=swap" \
  -H "User-Agent: Mozilla/5.0" | grep -oP 'url\(\K[^)]+\.woff2' | head -2

# Save as:
#   website/fonts/SourceCodePro-Regular.woff2
#   website/fonts/SourceCodePro-SemiBold.woff2
```

- [ ] **Step 4: Verify fonts downloaded**

```bash
ls -la website/fonts/
# Should show 5 woff2 files, each 20-80KB
```

- [ ] **Step 5: Commit**

```bash
git add website/css website/js website/fonts website/docs website/benchmarks website/download website/.nojekyll
git commit -m "chore: create website directory structure and download fonts"
```

---

### Task 2: Create the CSS design system

**Files:**
- Create: `website/css/style.css`

This is the largest single file. It contains all styles for all pages, with CSS custom properties for theming.

- [ ] **Step 1: Write the complete stylesheet**

Write `website/css/style.css` with the following structure. The complete file should include:

**Section 1 — Font faces:**
```css
@font-face {
  font-family: 'Source Sans 3';
  src: url('/sans/fonts/SourceSans3-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'Source Sans 3';
  src: url('/sans/fonts/SourceSans3-SemiBold.woff2') format('woff2');
  font-weight: 600;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'Source Sans 3';
  src: url('/sans/fonts/SourceSans3-Bold.woff2') format('woff2');
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'Source Code Pro';
  src: url('/sans/fonts/SourceCodePro-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}
@font-face {
  font-family: 'Source Code Pro';
  src: url('/sans/fonts/SourceCodePro-SemiBold.woff2') format('woff2');
  font-weight: 600;
  font-style: normal;
  font-display: swap;
}
```

**Section 2 — CSS custom properties (Catppuccin Latte — light mode defaults):**
```css
:root {
  --base: #eff1f5;
  --mantle: #e6e9ef;
  --crust: #dce0e8;
  --surface0: #ccd0da;
  --surface1: #bcc0cc;
  --text: #4c4f69;
  --subtext1: #5c5f77;
  --subtext0: #6c6f85;
  --overlay0: #9ca0b0;
  --mauve: #8839ef;
  --lavender: #7287fd;
  --green: #40a02b;
  --peach: #fe640b;
  --yellow: #df8e1d;
  --red: #d20f39;
}
```

**Section 3 — Dark mode overrides (Catppuccin Mocha):**
```css
[data-theme="dark"] {
  --base: #1e1e2e;
  --mantle: #181825;
  --crust: #11111b;
  --surface0: #313244;
  --surface1: #45475a;
  --text: #cdd6f4;
  --subtext1: #bac2de;
  --subtext0: #a6adc8;
  --overlay0: #6c7086;
  --mauve: #cba6f7;
  --lavender: #b4befe;
  --green: #a6e3a1;
  --peach: #fab387;
  --yellow: #f9e2af;
  --red: #f38ba8;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --base: #1e1e2e;
    --mantle: #181825;
    --crust: #11111b;
    --surface0: #313244;
    --surface1: #45475a;
    --text: #cdd6f4;
    --subtext1: #bac2de;
    --subtext0: #a6adc8;
    --overlay0: #6c7086;
    --mauve: #cba6f7;
    --lavender: #b4befe;
    --green: #a6e3a1;
    --peach: #fab387;
    --yellow: #f9e2af;
    --red: #f38ba8;
  }
}
```

**Section 4 — Reset and base styles:**
```css
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  background: var(--base);
  color: var(--text);
  font-family: 'Source Sans 3', -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 16px;
  line-height: 1.6;
}

a { color: var(--lavender); text-decoration: none; }
a:hover { text-decoration: underline; }
```

**Section 5 — Layout:**
```css
.container { max-width: 1440px; margin: 0 auto; padding: 0 32px; }
```

**Section 6 — Hero:**
```css
.hero { padding: 80px 0 64px; text-align: center; }
.hero h1 { font-size: 48px; font-weight: 700; color: var(--text); letter-spacing: -1px; margin-bottom: 16px; }
.tagline { font-size: 22px; color: var(--subtext0); margin-bottom: 12px; max-width: 640px; margin-left: auto; margin-right: auto; }
.subtitle { font-size: 16px; color: var(--overlay0); max-width: 640px; margin-left: auto; margin-right: auto; }
.hero-cta { display: flex; gap: 12px; justify-content: center; margin-top: 32px; }
.hero-version { margin-top: 16px; font-size: 13px; color: var(--overlay0); }
.hero-version a { color: var(--overlay0); text-decoration: underline; }
```

**Section 7 — Buttons:**
```css
.btn-primary {
  display: inline-block; background: var(--mauve); color: #fff;
  padding: 12px 28px; border-radius: 8px; font-size: 15px; font-weight: 600;
  text-decoration: none; transition: opacity 0.15s;
}
.btn-primary:hover { opacity: 0.9; text-decoration: none; }

.btn-secondary {
  display: inline-block; background: var(--surface0); color: var(--text);
  padding: 12px 28px; border-radius: 8px; font-size: 15px; font-weight: 600;
  text-decoration: none; transition: background 0.15s;
}
.btn-secondary:hover { background: var(--surface1); text-decoration: none; }
```

**Section 8 — Feature cards:**
```css
.features {
  display: grid; grid-template-columns: repeat(2, 1fr);
  gap: 20px; padding: 48px 0;
}
.feature-card {
  background: var(--mantle); border: 1px solid var(--surface0);
  border-radius: 12px; padding: 24px 28px;
  transition: transform 0.2s, box-shadow 0.2s;
}
.feature-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.08);
}
.feature-card h3 { font-size: 18px; font-weight: 600; color: var(--text); margin-bottom: 8px; }
.feature-card p { font-size: 15px; color: var(--subtext0); line-height: 1.6; }
```

**Section 9 — Code blocks and syntax highlighting:**
```css
pre {
  background: var(--mantle); border: 1px solid var(--surface0);
  border-radius: 8px; padding: 16px 20px;
  font-size: 14px; line-height: 1.6; overflow-x: auto; margin-bottom: 16px;
}
code {
  font-family: 'Source Code Pro', ui-monospace, "SF Mono", Menlo, monospace;
  color: var(--text);
}
p code, li code, td code {
  background: var(--mantle); border: 1px solid var(--surface0);
  border-radius: 4px; padding: 1px 6px; font-size: 13px;
}

/* Syntax highlighting — uses CSS vars so dark mode works automatically */
.kw { color: var(--mauve); }
.fn { color: var(--lavender); }
.str { color: var(--green); }
.num { color: var(--peach); }
.type { color: var(--yellow); }
.comment { color: var(--overlay0); font-style: italic; }
```

**Section 10 — Tables:**
```css
table { width: 100%; border-collapse: collapse; margin-bottom: 24px; font-size: 14px; }
thead th {
  color: var(--mauve); font-weight: 600; text-align: left;
  padding: 10px 14px; border-bottom: 2px solid var(--surface0);
}
tbody tr:nth-child(odd) { background: var(--mantle); }
tbody tr:nth-child(even) { background: var(--base); }
tbody td { padding: 8px 14px; border-bottom: 1px solid var(--surface0); vertical-align: top; }
td.highlight { color: var(--mauve); font-weight: 600; }
```

**Section 11 — Docs layout:**
```css
.docs { padding: 32px 0; }
.docs h1 { font-size: 32px; margin-bottom: 16px; }
.docs h2 { font-size: 24px; margin: 40px 0 16px; padding-top: 16px; border-top: 1px solid var(--surface0); }
.docs h3 { font-size: 18px; color: var(--subtext1); margin: 24px 0 12px; }
.docs h4 { font-size: 16px; color: var(--subtext1); margin-bottom: 8px; }
.docs ul { padding-left: 24px; margin-bottom: 16px; }
.docs li { margin-bottom: 4px; }

.docs-layout { display: grid; grid-template-columns: 240px 1fr; gap: 32px; max-width: 1440px; margin: 0 auto; padding: 0 32px; }

.toc {
  position: sticky; top: 72px;
  background: var(--mantle); border: 1px solid var(--surface0);
  border-radius: 8px; padding: 16px 20px; margin-top: 32px;
  max-height: calc(100vh - 88px); overflow-y: auto;
}
.toc h3 { font-size: 12px; text-transform: uppercase; color: var(--subtext0); letter-spacing: 0.05em; margin-bottom: 8px; }
.toc a { display: block; font-size: 13px; color: var(--subtext1); padding: 2px 0; }
.toc a:hover { color: var(--lavender); text-decoration: none; }
```

**Section 12 — Comparison tabs:**
```css
.comparison { padding: 48px 0; border-top: 1px solid var(--surface0); }
.tabs > input[type="radio"] { display: none; }
.tab-bar { display: flex; border-bottom: 2px solid var(--surface0); gap: 0; }
.tab-label {
  padding: 8px 16px; font-size: 14px; font-weight: 600; color: var(--subtext0);
  cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -2px;
  transition: color 0.15s, border-color 0.15s;
}
.tab-label:hover { color: var(--text); }
.token-badge {
  font-size: 11px; color: var(--overlay0); background: var(--crust);
  border-radius: 10px; padding: 1px 7px; margin-left: 4px;
}
.tab-panels > .tab-panel { display: none; }

/* Tab activation — requires one rule per tab. Extend for 5 tabs (Sans, Go, Rust, Node, Python). */
/* Example 1 tabs */
.tabs > input:nth-of-type(1):checked ~ .tab-bar > .tab-label:nth-child(1),
.tabs > input:nth-of-type(2):checked ~ .tab-bar > .tab-label:nth-child(2),
.tabs > input:nth-of-type(3):checked ~ .tab-bar > .tab-label:nth-child(3),
.tabs > input:nth-of-type(4):checked ~ .tab-bar > .tab-label:nth-child(4),
.tabs > input:nth-of-type(5):checked ~ .tab-bar > .tab-label:nth-child(5) {
  color: var(--mauve); border-color: var(--mauve);
}
.tabs > input:nth-of-type(1):checked ~ .tab-bar > .tab-label:nth-child(1) .token-badge,
.tabs > input:nth-of-type(2):checked ~ .tab-bar > .tab-label:nth-child(2) .token-badge,
.tabs > input:nth-of-type(3):checked ~ .tab-bar > .tab-label:nth-child(3) .token-badge,
.tabs > input:nth-of-type(4):checked ~ .tab-bar > .tab-label:nth-child(4) .token-badge,
.tabs > input:nth-of-type(5):checked ~ .tab-bar > .tab-label:nth-child(5) .token-badge {
  background: var(--mauve); color: #fff;
}
.tabs > input:nth-of-type(1):checked ~ .tab-panels > .tab-panel:nth-child(1),
.tabs > input:nth-of-type(2):checked ~ .tab-panels > .tab-panel:nth-child(2),
.tabs > input:nth-of-type(3):checked ~ .tab-panels > .tab-panel:nth-child(3),
.tabs > input:nth-of-type(4):checked ~ .tab-panels > .tab-panel:nth-child(4),
.tabs > input:nth-of-type(5):checked ~ .tab-panels > .tab-panel:nth-child(5) {
  display: block;
}
```

**Section 13 — Install block:**
```css
.install-block {
  position: relative; background: var(--mantle); border: 1px solid var(--surface0);
  border-radius: 8px; padding: 16px 56px 16px 20px; margin: 16px 0;
  font-family: 'Source Code Pro', ui-monospace, monospace; font-size: 14px; overflow-x: auto; white-space: nowrap;
}
.install-block pre { margin: 0; background: none; border: none; padding: 0; }
.copy-btn {
  position: absolute; right: 12px; top: 50%; transform: translateY(-50%);
  background: var(--surface0); border: none; border-radius: 5px;
  padding: 4px 10px; font-size: 12px; color: var(--text); cursor: pointer;
}
.copy-btn:hover { background: var(--surface1); }
```

**Section 14 — Footer (fallback for no-JS):**
```css
footer {
  border-top: 1px solid var(--surface0); padding: 32px;
  text-align: center; color: var(--subtext0); font-size: 14px; margin-top: 48px;
}
footer a { color: var(--lavender); }
```

**Section 15 — Responsive breakpoints:**
```css
@media (max-width: 1024px) {
  .docs-layout { grid-template-columns: 1fr; }
  .toc { position: static; max-height: none; }
}
@media (max-width: 768px) {
  .hero h1 { font-size: 32px; }
  .tagline { font-size: 18px; }
  .features { grid-template-columns: 1fr; }
  .hero-cta { flex-direction: column; align-items: center; }
}
```

- [ ] **Step 2: Verify CSS file structure**

```bash
wc -l website/css/style.css
# Should be approximately 180-250 lines
```

- [ ] **Step 3: Commit**

```bash
git add website/css/style.css
git commit -m "feat: add Catppuccin Latte/Mocha CSS design system"
```

---

### Task 3: Create web components

**Files:**
- Create: `website/js/components.js`

- [ ] **Step 1: Write the web components file**

Write `website/js/components.js` with two custom elements:

**`<sans-nav>` component:**
- Attributes: `active` (which link to highlight), `base` (default `/sans`)
- Shadow DOM with internal styles using CSS custom properties
- Host element: `display: block; position: sticky; top: 0; z-index: 100;`
- Renders: logo ("Sans" + "ALPHA" badge in Mauve), nav links (Docs, Benchmarks, Download, GitHub), theme toggle (inline SVG sun/moon)
- Theme toggle: reads/writes `data-theme` on `document.documentElement`, saves to `localStorage('sans-theme')`
- Active link: highlighted with `var(--mauve)` color and bottom border
- Inner container: `max-width: 1440px; margin: 0 auto; padding: 0 32px;`
- Background: `var(--mantle)`, border-bottom: `1px solid var(--surface0)`
- Links use the `base` attribute for href construction
- GitHub link opens in new tab
- Accessibility: toggle has `aria-label`, `role="button"`, `tabindex="0"`, keyboard support (Enter/Space)

**`<sans-footer>` component:**
- Attributes: `base` (default `/sans`)
- Shadow DOM with internal styles
- Host element: `display: block; margin-top: 48px;`
- Reads version from `document.querySelector('meta[name="sans-version"]')?.content || ''`
- Renders: "Sans Alpha — MIT License — GitHub — Download" with version if available
- "Alpha" in `var(--mauve)`, links in `var(--lavender)`
- Background: `var(--mantle)`, border-top: `1px solid var(--surface0)`

**Theme toggle inline SVG icons:**
- Sun icon: simple circle + rays (for "switch to light mode")
- Moon icon: crescent (for "switch to dark mode")
- Both 18px, stroke `currentColor`

- [ ] **Step 2: Commit**

```bash
git add website/js/components.js
git commit -m "feat: add sans-nav and sans-footer web components with theme toggle"
```

---

### Task 4: Migrate homepage

**Files:**
- Create: `website/index.html` (from `website/static/index.html`)

- [ ] **Step 1: Write the new homepage**

Create `website/index.html`. This replaces `website/static/index.html`.

Key changes from old version:
- Add `<meta name="sans-version" content="0.3.44">` in `<head>`
- Add inline theme detection script in `<head>` (before CSS):
  ```html
  <script>
    (function(){var t=localStorage.getItem('sans-theme');if(t)document.documentElement.setAttribute('data-theme',t);else if(matchMedia('(prefers-color-scheme:dark)').matches)document.documentElement.setAttribute('data-theme','dark');})();
  </script>
  ```
- Link to `/sans/css/style.css` (not `/static/style.css`)
- Add `<script src="/sans/js/components.js" defer></script>`
- Replace hardcoded nav with `<sans-nav active="home" base="/sans"></sans-nav>`
- Replace hardcoded footer with `<sans-footer base="/sans"></sans-footer>`
- Keep all existing content sections (hero, features, comparison, benchmarks, quick start)
- Update hero text per Spec 2 (but structure stays the same — content updates are Spec 2's job, just make the skeleton work)
- All internal links use relative paths: `./docs/`, `./benchmarks/`, `./download/`

- [ ] **Step 2: Open in browser and verify**

```bash
cd website && python3 -m http.server 8000 &
# Open http://localhost:8000 in browser
# Verify: nav renders, footer renders, theme toggle works, fonts load
# Kill: kill %1
```

- [ ] **Step 3: Commit**

```bash
git add website/index.html
git commit -m "feat: migrate homepage to new design system with web components"
```

---

### Task 5: Migrate docs, benchmarks, download pages

**Files:**
- Create: `website/docs/index.html` (from `website/static/docs.html`)
- Create: `website/benchmarks/index.html` (from `website/static/benchmarks.html`)
- Create: `website/download/index.html` (from `website/static/download.html`)

- [ ] **Step 1: Migrate docs page**

Copy `website/static/docs.html` to `website/docs/index.html`. Apply the same changes as Task 4:
- Add version meta tag and theme script in `<head>`
- Update CSS/JS paths to `/sans/css/style.css` and `/sans/js/components.js`
- Replace nav with `<sans-nav active="docs" base="/sans"></sans-nav>`
- Replace footer with `<sans-footer base="/sans"></sans-footer>`
- Update internal links to relative paths
- TOC links are anchor links (`#types`, `#variables`) — these stay as-is

- [ ] **Step 2: Migrate benchmarks page**

Same transformation for `website/static/benchmarks.html` → `website/benchmarks/index.html`:
- `<sans-nav active="benchmarks" base="/sans">`
- Update all paths

- [ ] **Step 3: Migrate download page**

Same transformation for `website/static/download.html` → `website/download/index.html`:
- `<sans-nav active="download" base="/sans">`
- Update all paths
- The install command URLs stay pointed at GitHub releases (absolute URLs, no change needed)

- [ ] **Step 4: Verify all pages render**

```bash
# With python server still running from Task 4:
# Open http://localhost:8000/docs/
# Open http://localhost:8000/benchmarks/
# Open http://localhost:8000/download/
# Verify: nav active state, theme toggle, content renders, links work
```

- [ ] **Step 5: Commit**

```bash
git add website/docs/index.html website/benchmarks/index.html website/download/index.html
git commit -m "feat: migrate docs, benchmarks, download pages to new design system"
```

---

### Task 6: Create 404 page

**Files:**
- Create: `website/404.html`

- [ ] **Step 1: Write 404 page**

Create `website/404.html` with:
- Same `<head>` setup (version meta, theme script, CSS, components JS)
- `<sans-nav base="/sans"></sans-nav>` (no active link)
- Centered content:
  - `<h1>404</h1>` (large, using `var(--text)`)
  - `<p>The page you're looking for doesn't exist.</p>` (using `var(--subtext0)`)
  - `<a href="/sans/" class="btn-primary">Go to homepage</a>`
- `<sans-footer base="/sans"></sans-footer>`

- [ ] **Step 2: Commit**

```bash
git add website/404.html
git commit -m "feat: add custom 404 page"
```

---

## Chunk 2: Deployment and cleanup

### Task 7: Create GitHub Pages deploy workflow

**Files:**
- Create: `.github/workflows/deploy-site.yml`

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/deploy-site.yml`:

```yaml
name: Deploy Website

on:
  push:
    branches: [main]
    paths: ['website/**']

permissions:
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: website
      - id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/deploy-site.yml
git commit -m "feat: add GitHub Pages deploy workflow"
```

---

### Task 8: Update release workflow version-bump paths

**Files:**
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Update sed commands in release.yml**

The version-bump job currently targets `website/static/*.html` with footer text. Update to target the new paths and the `<meta name="sans-version">` tag:

Find the sed commands for website files and replace:
```bash
# Old (remove these):
for html in website/static/index.html website/static/docs.html website/static/benchmarks.html website/static/download.html; do
  sed -i "s/Sans v[0-9]*\.[0-9]*\.[0-9]*/Sans v${VERSION}/g" "$html"
done

# New (replace with):
for html in website/index.html website/docs/index.html website/benchmarks/index.html website/download/index.html; do
  sed -i "s/content=\"[0-9]*\.[0-9]*\.[0-9]*\"/content=\"${VERSION}\"/" "$html"
done
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "fix: update release workflow version-bump to target new website paths"
```

---

### Task 9: Move Sans server and clean up old files

**Files:**
- Move: `website/main.sans` → `examples/website-server/main.sans`
- Delete: `website/static/` directory
- Delete: `website/main` (compiled binary, if present)

- [ ] **Step 1: Move the Sans server to examples**

```bash
mkdir -p examples/website-server
mv website/main.sans examples/website-server/main.sans
```

- [ ] **Step 2: Remove old static directory and binary**

```bash
rm -rf website/static
rm -f website/main
```

- [ ] **Step 3: Update .gitignore if needed**

Check if `website/main` is already in `.gitignore`. If not, add it (though it's now removed and won't be regenerated).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: move Sans server to examples/, remove old website/static/"
```

---

### Task 10: Verify complete site locally

- [ ] **Step 1: Start local server and test all pages**

```bash
cd /Users/sgordon/Development/Repos/sans/website
python3 -m http.server 8000 &
```

Test each page:
- `http://localhost:8000/` — homepage loads, nav/footer render, theme toggle works
- `http://localhost:8000/docs/` — docs load with sidebar TOC
- `http://localhost:8000/benchmarks/` — benchmark tables render
- `http://localhost:8000/download/` — download page with install command
- `http://localhost:8000/nonexistent` — 404 page (may not work with python server, but file exists)
- Toggle dark mode — all pages switch correctly
- Check fonts load (Source Sans 3 for text, Source Code Pro for code blocks)
- Check responsive: resize to mobile width, verify single-column layout

```bash
kill %1
```

- [ ] **Step 2: Verify no broken links**

```bash
grep -r "static/" website/ --include="*.html" --include="*.css" --include="*.js"
# Should return NO results — all old /static/ references must be gone
```

- [ ] **Step 3: Final commit if any fixes needed**

```bash
git status
# If clean, skip. If fixes needed, stage and commit.
```
