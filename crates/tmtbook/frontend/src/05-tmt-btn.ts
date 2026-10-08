(() => {
  'use strict';
  const T = (window.TMT ??= {} as any);

  class TmtBtn extends HTMLElement {
    static get observedAttributes() {
      return ['active', 'disabled', 'title', 'aria-label', 'href', 'class'];
    }

    connectedCallback() {
      T.ensureShadowRoot(this);
      this._btn = this.shadowRoot?.querySelector('.tmt-btn-inner');
      this._syncAttributes();

      if (!this.hasAttribute('tabindex') && !this.hasAttribute('disabled')) {
        this.setAttribute('tabindex', '0');
      }

      this.addEventListener('click', this._handleClick.bind(this));
      this.addEventListener('keydown', this._handleKeyDown.bind(this));
    }

    _isActive() {
      return this.hasAttribute('active') || this.classList.contains('active') || this.classList.contains('is-active');
    }

    _handleClick(e) {
      if (this.hasAttribute('disabled')) {
        e.preventDefault();
        e.stopImmediatePropagation();
        return;
      }
      // `active` only means "already selected" -- it must not swallow the
      // click. Some callers (e.g. a tab that collapses its panel when
      // clicked again) rely on their own listener still seeing it; only
      // href-navigation is skipped, since re-navigating to the current page
      // is never useful.
      const href = this.getAttribute('href');
      if (href && !this._isActive() && !e.defaultPrevented) {
        window.location.href = href;
      }
    }

    _handleKeyDown(e) {
      if (this.hasAttribute('disabled')) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        this.click();
      }
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue === newValue) return;
      this._syncAttributes();
    }

    _syncAttributes() {
      if (!this._btn) return;

      const disabled = this.hasAttribute('disabled');
      const active = this._isActive();
      const title = this.getAttribute('title');
      const ariaLabel = this.getAttribute('aria-label');

      if (disabled) {
        this.setAttribute('aria-disabled', 'true');
        this.setAttribute('tabindex', '-1');
        this._btn.disabled = true;
      } else {
        this.removeAttribute('aria-disabled');
        if (this.getAttribute('tabindex') === '-1') {
          this.setAttribute('tabindex', '0');
        }
        this._btn.disabled = false;
      }

      if (active) {
        this.setAttribute('aria-pressed', 'true');
      } else {
        this.removeAttribute('aria-pressed');
      }

      if (title) this._btn.title = title;
      if (ariaLabel) this._btn.setAttribute('aria-label', ariaLabel);
    }
  }

  if (!customElements.get('tmt-btn')) {
    customElements.define('tmt-btn', TmtBtn);
  }

  window.TMT ??= {} as any;
  window.TMT.TmtBtn = TmtBtn;
})();
