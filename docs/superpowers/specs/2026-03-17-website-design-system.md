# Website Redesign Spec 1: Infrastructure + Design System

**Goal:** Migrate the Sans website from a custom Sans server to GitHub Pages, establish a design system with Catppuccin light/dark theming, reusable web components, and modern typography.

**Scope:** Infrastructure, theming, layout, components, deployment. Content and page updates are Spec 2.

---

## Infrastructure

### GitHub Pages Deployment

- **Source:** `website/` directory on main branch
- **Workflow:** `.github/workflows/deploy-site.yml` triggered on push to main (paths: `website/**`)
- **Steps:** checkout → upload `website/` as pages artifact → deploy
- **Base path:** `/sans` (for `sans-language.github.io/sans`).
  - **URL strategy:** All internal links use relative paths (`./docs/`, `../`). Asset references (CSS, JS, fonts) use root-relative paths (`/sans/css/style.css`). Do NOT use `<base href>` — it breaks anchor links and has known footguns.
  - The web components accept a `base` attribute to construct link hrefs.
- **CNAME:** Not yet — custom domain to be added later. When added, base path becomes `/` and all root-relative asset paths simplify.
- **`.nojekyll`:** Include in `website/` root to prevent Jekyll processing.

### File Structure

```
website/
├── index.html
├── docs/index.html
├── benchmarks/index.html
├── download/index.html
├── 404.html
├── llms.txt
├── .nojekyll
├── css/
│   └── style.css
├── js/
│   └── components.js
└── fonts/
    ├── SourceSans3-Regular.woff2
    ├── SourceSans3-SemiBold.woff2
    ├── SourceSans3-Bold.woff2
    ├── SourceCodePro-Regular.woff2
    └── SourceCodePro-SemiBold.woff2
```

Directory-based routing: each page is an `index.html` inside its directory so GitHub Pages serves clean URLs (`/sans/docs`, `/sans/benchmarks`, `/sans/download`).

### What Gets Removed

- `website/main.sans` — move to `examples/website-server/main.sans`. Must not remain in `website/` or it gets deployed as a downloadable file.
- `website/main` — compiled binary, already gitignored. Delete if present.
- `website/static/` — all files move to the new structure above. Delete the directory after migration.

---

## Design System

### Color Palette — Catppuccin Latte (Light) / Mocha (Dark)

All colors defined as CSS custom properties on `:root` (light) and `[data-theme="dark"]` (dark).

**Light Mode (Latte):**

| Role | Variable | Value |
|------|----------|-------|
| Base (page bg) | `--base` | `#eff1f5` |
| Mantle (nav/footer bg) | `--mantle` | `#e6e9ef` |
| Crust | `--crust` | `#dce0e8` |
| Surface0 (borders) | `--surface0` | `#ccd0da` |
| Surface1 | `--surface1` | `#bcc0cc` |
| Text | `--text` | `#4c4f69` |
| Subtext1 | `--subtext1` | `#5c5f77` |
| Subtext0 | `--subtext0` | `#6c6f85` |
| Overlay0 | `--overlay0` | `#9ca0b0` |
| Mauve (primary: buttons, badges) | `--mauve` | `#8839ef` |
| Lavender (links, secondary) | `--lavender` | `#7287fd` |
| Green (strings) | `--green` | `#40a02b` |
| Peach (numbers) | `--peach` | `#fe640b` |
| Yellow (types) | `--yellow` | `#df8e1d` |
| Red (errors) | `--red` | `#d20f39` |

**Dark Mode (Mocha):**

| Role | Variable | Value |
|------|----------|-------|
| Base | `--base` | `#1e1e2e` |
| Mantle | `--mantle` | `#181825` |
| Crust | `--crust` | `#11111b` |
| Surface0 | `--surface0` | `#313244` |
| Surface1 | `--surface1` | `#45475a` |
| Text | `--text` | `#cdd6f4` |
| Subtext1 | `--subtext1` | `#bac2de` |
| Subtext0 | `--subtext0` | `#a6adc8` |
| Overlay0 | `--overlay0` | `#6c7086` |
| Mauve | `--mauve` | `#cba6f7` |
| Lavender | `--lavender` | `#b4befe` |
| Green | `--green` | `#a6e3a1` |
| Peach | `--peach` | `#fab387` |
| Yellow | `--yellow` | `#f9e2af` |
| Red | `--red` | `#f38ba8` |

### Typography

- **Body/Headings:** Source Sans 3 (weights: 400, 600, 700). Self-hosted woff2 files. `font-display: swap`.
- **Code:** Source Code Pro (weights: 400, 600). Self-hosted woff2 files. `font-display: swap`.
- **Fallback stack:** `-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` for body; `ui-monospace, "SF Mono", Menlo, monospace` for code.
- **Base size:** 16px, line-height 1.6.

### Layout

- **Max width:** 1440px for container.
- **Docs layout:** 1440px container with sticky sidebar TOC (240px) + content area.
- **Responsive breakpoints:**
  - `<= 1024px`: docs layout collapses to single column, TOC becomes static.
  - `<= 768px`: hero adjusts, feature grid becomes single column, nav may wrap.

### Theme Toggle

- **Mechanism:** `data-theme` attribute on `<html>` element. Default is light.
- **Detection:** On first visit, check `localStorage('sans-theme')`. If absent, check `prefers-color-scheme: dark`. If matches, set dark.
- **Persistence:** Save choice to `localStorage('sans-theme')`.
- **No FOUC:** Inline `<script>` in `<head>` (before CSS loads) reads localStorage and sets `data-theme` before first paint.
- **CSS fallback for no-JS:** Include `@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { /* dark vars */ } }` so users with JS disabled get system-appropriate theming.
- **Toggle:** Inline SVG sun/moon icon in nav. Clicking toggles `data-theme` and saves to localStorage. Accessibility: `aria-label="Switch to dark mode"` / `"Switch to light mode"`, `role="button"`, `tabindex="0"`.

---

## Web Components

### `<sans-nav>`

Custom element rendering the sticky navigation bar.

**Attributes:**
- `active` — which nav link to highlight (e.g., `active="docs"`).
- `base` — base path for links (default: `/sans`).

**Renders:**
- Logo: "Sans" with "ALPHA" superscript badge in Mauve.
- Links: Docs, Benchmarks, Download, GitHub.
- Theme toggle: sun/moon icon on far right.
- Sticky: `position: sticky; top: 0; z-index: 100`.
- Background: `var(--mantle)`, bottom border `var(--surface0)`.
- Inner container: 1440px max-width, centered.

### `<sans-footer>`

Custom element rendering the page footer.

**Attributes:**
- `base` — base path for links (default: `/sans`).

**Renders:**
- "Sans Alpha — MIT License — GitHub — Download"
- "Alpha" in Mauve color.
- Links in Lavender.
- Background: `var(--mantle)`, top border `var(--surface0)`.
- **Version:** The component reads `<meta name="sans-version" content="0.3.44">` from the document `<head>`. CI's version-bump workflow updates this meta tag via `sed`. This avoids hardcoding in JS.

### Implementation

Single file: `js/components.js`. Defines both custom elements. Each uses Shadow DOM for style encapsulation, with `<style>` tags inside the shadow root that reference CSS custom properties from the document (CSS custom properties pierce shadow DOM).

**Host element styling:**
- `<sans-nav>`: `:host { display: block; position: sticky; top: 0; z-index: 100; }` — sticky positioning on the host element itself.
- `<sans-footer>`: `:host { display: block; margin-top: 48px; }` — spacing from page content.
- Both components render an inner div with `max-width: 1440px; margin: 0 auto;` for centering.

### Syntax Highlighting Class Mapping

All syntax highlighting uses CSS custom properties so dark mode works automatically:

| Class | Variable | Light | Dark |
|-------|----------|-------|------|
| `.kw` (keywords) | `var(--mauve)` | `#8839ef` | `#cba6f7` |
| `.fn` (functions) | `var(--lavender)` | `#7287fd` | `#b4befe` |
| `.str` (strings) | `var(--green)` | `#40a02b` | `#a6e3a1` |
| `.num` (numbers) | `var(--peach)` | `#fe640b` | `#fab387` |
| `.type` (types) | `var(--yellow)` | `#df8e1d` | `#f9e2af` |
| `.comment` | `var(--overlay0)` | `#9ca0b0` | `#6c7086` |

### Release Workflow Integration

The release workflow (`.github/workflows/release.yml`) must be updated to target the new file paths. The version-bump job's `sed` commands should update:
- `website/index.html` — `<meta name="sans-version">` tag
- `website/docs/index.html` — same meta tag
- `website/benchmarks/index.html` — same meta tag
- `website/download/index.html` — same meta tag

The footer component reads the version from this meta tag at runtime, so only the HTML files need updating (not `components.js`).

---

## CSS Architecture

Single file: `css/style.css`.

**Structure:**
1. `@font-face` declarations (Source Sans 3, Source Code Pro)
2. `:root` — light mode CSS custom properties
3. `[data-theme="dark"]` — dark mode overrides
4. Reset / base styles
5. Layout (container, grid)
6. Navigation (consumed by web component but also available for fallback)
7. Hero section
8. Feature cards
9. Code blocks and syntax highlighting
10. Tables
11. Documentation-specific styles (TOC, headings)
12. Comparison/tabs
13. Install block
14. Buttons
15. Footer
16. Responsive breakpoints

All color values use `var(--name)` — never raw hex in component styles.

---

## Deployment Workflow

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

---

## Out of Scope (Spec 2)

- Page content (hero copy, feature cards, token comparisons, benchmarks)
- Documentation restructuring and ordering
- llms.txt content
- 404 page content
- Examples updates
- Benchmark data and ordering
