(() => {
  'use strict';

  class TmtBadge extends HTMLElement {
    static get observedAttributes() {
      return ['href', 'title', 'aria-label'];
    }

    connectedCallback() {
      if (this.hasAttribute('href') && !this.hasAttribute('tabindex')) {
        this.setAttribute('tabindex', '0');
        this.setAttribute('role', 'link');
      }

      this.addEventListener('click', this._handleClick.bind(this));
      this.addEventListener('keydown', this._handleKeyDown.bind(this));
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue === newValue) return;
      if (name === 'href') {
        if (newValue) {
          if (!this.hasAttribute('tabindex')) this.setAttribute('tabindex', '0');
          if (!this.hasAttribute('role')) this.setAttribute('role', 'link');
        } else {
          if (this.getAttribute('role') === 'link') this.removeAttribute('role');
          if (this.getAttribute('tabindex') === '0') this.removeAttribute('tabindex');
        }
        const innerLink = this.shadowRoot?.querySelector('a.tmt-badge-inner');
        if (innerLink) {
          if (newValue) innerLink.setAttribute('href', newValue);
          else innerLink.removeAttribute('href');
        }
      }
    }

    _handleClick(e) {
      if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
      if (e.composedPath().some(el => el.tagName === 'A')) return;

      const href = this.getAttribute('href');
      if (href) {
        window.location.href = href;
      }
    }

    _handleKeyDown(e) {
      const href = this.getAttribute('href');
      if (!href) return;
      if (e.key === 'Enter') {
        e.preventDefault();
        window.location.href = href;
      }
    }
  }

  if (!customElements.get('tmt-badge')) {
    customElements.define('tmt-badge', TmtBadge);
  }

  window.TMT ??= {};
  window.TMT.TmtBadge = TmtBadge;
})();
