(() => {
  'use strict';
  const T = (window.TMT ??= {} as any);

  class TmtIcon extends HTMLElement {
    static get observedAttributes() {
      return ['name', 'size', 'pkg'];
    }

    connectedCallback() {
      T.ensureShadowRoot(this);
    }

    attributeChangedCallback(name, oldValue, newValue) {
      if (oldValue === newValue) return;
      if (name === 'name' && this.shadowRoot) {
        const use = this.shadowRoot.querySelector('use');
        const pkg = this.getAttribute('pkg') || 'lucide';
        if (use && newValue) {
          use.setAttribute('href', `/icons/${pkg}.svg#${newValue}`);
        }
      }
    }
  }

  if (!customElements.get('tmt-icon')) {
    customElements.define('tmt-icon', TmtIcon);
  }

  window.TMT ??= {} as any;
  window.TMT.TmtIcon = TmtIcon;
})();
