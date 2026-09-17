(() => {
  'use strict';

  class TmtIcon extends HTMLElement {
    static get observedAttributes() {
      return ['name', 'size', 'pkg'];
    }

    connectedCallback() {
      if (!this.shadowRoot && this.attachShadow) {
        const shadow = this.attachShadow({ mode: 'open' });
        const name = this.getAttribute('name');
        const pkg = this.getAttribute('pkg') || 'lucide';
        if (name) {
          const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
          svg.setAttribute('aria-hidden', 'true');
          const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
          use.setAttribute('href', `/icons/${pkg}.svg#${name}`);
          svg.appendChild(use);
          shadow.appendChild(svg);
        } else {
          const slot = document.createElement('slot');
          shadow.appendChild(slot);
        }
      }
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

  window.TMT ??= {};
  window.TMT.TmtIcon = TmtIcon;
})();
