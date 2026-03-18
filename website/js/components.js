// Sans language website web components

const MOON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`;
const SUN_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`;

class SansNav extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._observer = null;
  }

  connectedCallback() {
    this.style.display = 'block';
    this.style.position = 'sticky';
    this.style.top = '0';
    this.style.zIndex = '100';

    this._render();
    this._bindToggle();
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
    const base = this.getAttribute('base') || '/sans';
    const dark = this._isDark();

    const links = [
      { key: 'docs', label: 'Docs', href: `${base}/docs/` },
      { key: 'benchmarks', label: 'Benchmarks', href: `${base}/benchmarks/` },
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
          background: var(--mantle);
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
          font-size: 18px;
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

        .nav-link {
          font-size: 14px;
          color: var(--subtext1);
          text-decoration: none;
          padding: 6px 12px;
          border-radius: 6px;
          transition: background 0.15s, color 0.15s;
        }

        .nav-link:hover {
          background: var(--surface0);
          color: var(--text);
        }

        .nav-link.active {
          color: var(--mauve);
          background: var(--crust);
        }

        .github-link {
          font-size: 14px;
          color: var(--subtext1);
          text-decoration: none;
          padding: 6px 12px;
          border-radius: 6px;
          transition: background 0.15s, color 0.15s;
        }

        .github-link:hover {
          background: var(--surface0);
          color: var(--text);
        }

        .theme-toggle {
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

        .theme-toggle:hover {
          color: var(--text);
          background: var(--surface0);
        }
      </style>
      <div class="nav-bar">
        <div class="nav-inner">
          <a href="${base}/" class="brand">Sans <sup class="alpha">ALPHA</sup></a>
          ${navLinks}
          <a href="https://github.com/sans-language/sans" target="_blank" rel="noopener" class="github-link">GitHub</a>
          <button
            class="theme-toggle"
            aria-label="${dark ? 'Switch to light mode' : 'Switch to dark mode'}"
            role="button"
            tabindex="0"
          >${dark ? SUN_SVG : MOON_SVG}</button>
        </div>
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
    const base = this.getAttribute('base') || '/sans';
    const versionMeta = document.querySelector('meta[name="sans-version"]');
    const version = versionMeta?.content ? ` v${versionMeta.content}` : '';

    this.shadowRoot.innerHTML = `
      <style>
        *, *::before, *::after { box-sizing: border-box; }

        .footer-bar {
          background: var(--mantle);
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
      </style>
      <div class="footer-bar">
        <div class="footer-inner">
          Sans <span style="color: var(--mauve); font-weight: 600;">Alpha</span>${version} — MIT License —
          <a href="https://github.com/sans-language/sans">GitHub</a> —
          <a href="${base}/download/">Download</a>
        </div>
      </div>
    `;
  }
}

customElements.define('sans-nav', SansNav);
customElements.define('sans-footer', SansFooter);
