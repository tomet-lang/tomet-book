(() => {
  'use strict';
  const T = (window.TMT ??= {} as any);

  class TmtSwatch extends HTMLElement {
    static get observedAttributes() {
      return ['checked', 'disabled', 'title'];
    }

    connectedCallback() {
      T.ensureShadowRoot(this);
      if (!this.hasAttribute('role')) this.setAttribute('role', 'radio');
      this._syncAttributes();
      this.addEventListener('click', this._handleClick.bind(this));
      this.addEventListener('keydown', this._handleKeyDown.bind(this));
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue === newValue) return;
      this._syncAttributes();
    }

    _group() {
      return this.closest('[role="radiogroup"]');
    }

    _siblings() {
      const group = this._group();
      if (!group) return [this];
      return Array.from(group.querySelectorAll('tmt-swatch'));
    }

    _syncAttributes() {
      const checked = this.hasAttribute('checked');
      const disabled = this.hasAttribute('disabled');
      this.setAttribute('aria-checked', checked ? 'true' : 'false');

      if (disabled) {
        this.setAttribute('aria-disabled', 'true');
        this.setAttribute('tabindex', '-1');
        return;
      }
      this.removeAttribute('aria-disabled');

      // Roving tabindex: the checked swatch is the tab stop; if none in the
      // group is checked yet, the first one is, so the group stays reachable.
      const siblings = this._siblings();
      const anyChecked = siblings.some((s) => s.hasAttribute('checked'));
      const isTabStop = anyChecked ? checked : siblings[0] === this;
      this.setAttribute('tabindex', isTabStop ? '0' : '-1');
    }

    _handleClick(e) {
      if (this.hasAttribute('disabled')) {
        e.preventDefault();
        return;
      }
      this.focus();
    }

    _handleKeyDown(e) {
      if (this.hasAttribute('disabled')) return;
      const moveKeys = ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'];
      if (!moveKeys.includes(e.key)) return;
      e.preventDefault();

      const siblings = this._siblings().filter((s) => !s.hasAttribute('disabled'));
      if (siblings.length === 0) return;
      const idx = siblings.indexOf(this);

      let nextIdx;
      if (e.key === 'Home') nextIdx = 0;
      else if (e.key === 'End') nextIdx = siblings.length - 1;
      else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
        nextIdx = (idx - 1 + siblings.length) % siblings.length;
      } else {
        nextIdx = (idx + 1) % siblings.length;
      }

      // Native <input type="radio"> both moves focus and changes the
      // selection on arrow keys; click() re-enters the same path a mouse
      // click would, so callers only need one selection handler.
      const next = siblings[nextIdx];
      next.click();
      next.focus();
    }
  }

  if (!customElements.get('tmt-swatch')) {
    customElements.define('tmt-swatch', TmtSwatch);
  }

  window.TMT ??= {} as any;
  window.TMT.TmtSwatch = TmtSwatch;
})();
