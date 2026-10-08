(() => {
  'use strict';
  const T = (window.TMT ??= {} as any);

  class TmtSwitch extends HTMLElement {
    static get observedAttributes() {
      return ['checked', 'disabled', 'title'];
    }

    connectedCallback() {
      T.ensureShadowRoot(this);
      if (!this.hasAttribute('role')) this.setAttribute('role', 'switch');
      if (!this.hasAttribute('tabindex') && !this.hasAttribute('disabled')) {
        this.setAttribute('tabindex', '0');
      }
      this._syncAttributes();
      this.addEventListener('click', this._handleClick.bind(this));
      this.addEventListener('keydown', this._handleKeyDown.bind(this));
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue === newValue) return;
      this._syncAttributes();
    }

    _syncAttributes() {
      const checked = this.hasAttribute('checked');
      const disabled = this.hasAttribute('disabled');
      this.setAttribute('aria-checked', checked ? 'true' : 'false');

      if (disabled) {
        this.setAttribute('aria-disabled', 'true');
        this.setAttribute('tabindex', '-1');
      } else {
        this.removeAttribute('aria-disabled');
        if (this.getAttribute('tabindex') === '-1') this.setAttribute('tabindex', '0');
      }
    }

    _handleClick(e) {
      if (this.hasAttribute('disabled')) {
        e.preventDefault();
      }
    }

    _handleKeyDown(e) {
      if (this.hasAttribute('disabled')) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        this.click();
      }
    }
  }

  if (!customElements.get('tmt-switch')) {
    customElements.define('tmt-switch', TmtSwitch);
  }

  window.TMT ??= {} as any;
  window.TMT.TmtSwitch = TmtSwitch;
})();
