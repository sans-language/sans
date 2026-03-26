// Sans language website web components

const MOON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`;
const SUN_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`;
const MENU_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>`;
const CLOSE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`;

class SansNav extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._observer = null;
    this._menuOpen = false;
  }

  connectedCallback() {
    this.style.display = 'block';
    this.style.position = 'sticky';
    this.style.top = '0';
    this.style.zIndex = '100';

    this._render();
    this._bindToggle();
    this._bindMenu();
    this._watchTheme();
  }

  disconnectedCallback() {
    if (this._observer) {
      this._observer.disconnect();
      this._observer = null;
    }
  }

  _isDark() {
    return document.documentElement.getAttribute('data-theme') === 'dark';
  }

  _render() {
    const active = this.getAttribute('active') || '';
    const base = this.hasAttribute('base') ? this.getAttribute('base') : '';
    const dark = this._isDark();

    const links = [
      { key: 'playground', label: 'Playground', href: `${base}/play/` },
      { key: 'examples', label: 'Examples', href: `${base}/examples/` },
      { key: 'docs', label: 'Documentation', href: `${base}/docs/` },
      { key: 'benchmarks', label: 'Comparisons', href: `${base}/benchmarks/` },
      { key: 'download', label: 'Download', href: `${base}/download/` },
    ];

    const navLinks = links.map(({ key, label, href }) => {
      const isActive = active === key;
      return `<a href="${href}" class="nav-link${isActive ? ' active' : ''}">${label}</a>`;
    }).join('');

    this.shadowRoot.innerHTML = `
      <style>
        *, *::before, *::after { box-sizing: border-box; }

        .nav-bar {
          background: color-mix(in srgb, var(--base) 80%, transparent);
          backdrop-filter: blur(12px);
          -webkit-backdrop-filter: blur(12px);
          border-bottom: 1px solid var(--surface0);
        }

        .nav-inner {
          max-width: 1440px;
          margin: 0 auto;
          padding: 0 32px;
          display: flex;
          align-items: center;
          height: 56px;
          gap: 24px;
        }

        .brand {
          font-weight: 700;
          font-size: 32px;
          color: var(--text);
          text-decoration: none;
          margin-right: auto;
        }

        .alpha {
          font-size: 9px;
          color: var(--mauve);
          font-weight: 600;
          vertical-align: super;
        }

        .desktop-links {
          display: flex;
          align-items: center;
          gap: 4px;
        }

        .nav-link, .github-link {
          font-size: 14px;
          color: var(--subtext1);
          text-decoration: none;
          padding: 6px 12px;
          border-radius: 6px;
          transition: background 0.15s, color 0.15s;
        }

        .nav-link:hover, .github-link:hover {
          background: var(--surface0);
          color: var(--text);
        }

        .nav-link.active {
          color: var(--mauve);
          background: var(--crust);
        }

        .theme-toggle, .menu-toggle {
          background: none;
          border: none;
          cursor: pointer;
          padding: 6px;
          color: var(--subtext0);
          display: flex;
          align-items: center;
          justify-content: center;
          border-radius: 6px;
          transition: color 0.15s, background 0.15s;
        }

        .theme-toggle:hover, .menu-toggle:hover {
          color: var(--text);
          background: var(--surface0);
        }

        .menu-toggle { display: none; }

        /* Mobile menu */
        .mobile-menu {
          display: none;
          background: var(--mantle);
          border-bottom: 1px solid var(--surface0);
          padding: 8px 16px 16px;
        }

        .mobile-menu.open { display: block; }

        .mobile-menu a {
          display: block;
          font-size: 16px;
          color: var(--subtext1);
          text-decoration: none;
          padding: 10px 12px;
          border-radius: 6px;
          transition: background 0.15s, color 0.15s;
        }

        .mobile-menu a:hover {
          background: var(--surface0);
          color: var(--text);
        }

        .mobile-menu a.active {
          color: var(--mauve);
          background: var(--crust);
        }

        @media (max-width: 768px) {
          .nav-inner { padding: 0 16px; gap: 8px; }
          .desktop-links { display: none; }
          .menu-toggle { display: flex; }
        }
      </style>
      <div class="nav-bar">
        <div class="nav-inner">
          <a href="${base}/" class="brand">Sans <sup class="alpha">ALPHA</sup></a>
          <div class="desktop-links">
            ${navLinks}
            <a href="https://github.com/sans-language/sans" target="_blank" rel="noopener" class="github-link">GitHub</a>
          </div>
          <button
            class="theme-toggle"
            aria-label="${dark ? 'Switch to light mode' : 'Switch to dark mode'}"
            role="button"
            tabindex="0"
          >${dark ? SUN_SVG : MOON_SVG}</button>
          <button class="menu-toggle" aria-label="Open menu" tabindex="0">${MENU_SVG}</button>
        </div>
      </div>
      <div class="mobile-menu">
        ${links.map(({ key, label, href }) =>
          `<a href="${href}" class="${active === key ? 'active' : ''}">${label}</a>`
        ).join('')}
        <a href="https://github.com/sans-language/sans" target="_blank" rel="noopener">GitHub</a>
      </div>
    `;
  }

  _bindToggle() {
    const btn = this.shadowRoot.querySelector('.theme-toggle');
    if (!btn) return;

    const toggle = () => {
      const current = document.documentElement.getAttribute('data-theme') || 'light';
      const next = current === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', next);
      localStorage.setItem('sans-theme', next);
    };

    btn.addEventListener('click', toggle);
    btn.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    });
  }

  _bindMenu() {
    const btn = this.shadowRoot.querySelector('.menu-toggle');
    const menu = this.shadowRoot.querySelector('.mobile-menu');
    if (!btn || !menu) return;

    btn.addEventListener('click', () => {
      this._menuOpen = !this._menuOpen;
      menu.classList.toggle('open', this._menuOpen);
      btn.innerHTML = this._menuOpen ? CLOSE_SVG : MENU_SVG;
      btn.setAttribute('aria-label', this._menuOpen ? 'Close menu' : 'Open menu');
    });
  }

  _watchTheme() {
    this._observer = new MutationObserver(() => {
      this._updateToggle();
    });
    this._observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
  }

  _updateToggle() {
    const btn = this.shadowRoot.querySelector('.theme-toggle');
    if (!btn) return;
    const dark = this._isDark();
    btn.innerHTML = dark ? SUN_SVG : MOON_SVG;
    btn.setAttribute('aria-label', dark ? 'Switch to light mode' : 'Switch to dark mode');
  }
}

class SansFooter extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.style.display = 'block';
    this.style.marginTop = '48px';
    this._render();
  }

  _render() {
    const base = this.hasAttribute('base') ? this.getAttribute('base') : '';
    // Version is tracked in meta tags for CI but not displayed in the footer

    this.shadowRoot.innerHTML = `
      <style>
        *, *::before, *::after { box-sizing: border-box; }

        .footer-bar {
          background: var(--base);
          border-top: 1px solid var(--surface0);
          padding: 24px 32px;
          text-align: center;
        }

        .footer-inner {
          max-width: 1440px;
          margin: 0 auto;
          font-size: 14px;
          color: var(--subtext0);
        }

        a {
          color: var(--lavender);
          text-decoration: none;
        }

        a:hover {
          text-decoration: underline;
        }

        @media (max-width: 768px) {
          .footer-bar { padding: 20px 16px; }
          .footer-inner { font-size: 13px; }
        }
      </style>
      <div class="footer-bar">
        <div class="footer-inner">
          Sans <span style="color: var(--mauve); font-weight: 600;">Alpha</span> — MIT License —
          <a href="https://github.com/sans-language/sans">GitHub</a> —
          <a href="${base}/download/">Download</a>
        </div>
      </div>
    `;
  }
}

customElements.define('sans-nav', SansNav);

// Auto-wrap code lines in <span class="line"> for line numbers
document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('pre > code').forEach(code => {
    const html = code.innerHTML;
    const lines = html.split('\n');
    // Remove trailing empty line from split
    if (lines.length && lines[lines.length - 1].trim() === '') lines.pop();
    code.innerHTML = lines.map(l => `<span class="line">${l}</span>`).join('\n');
  });
});
customElements.define('sans-footer', SansFooter);
