/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './templates/**/*.html',
    './ui/templates/**/*.html',
    './src/**/*.ts',
    './ui/src/**/*.ts',
  ],
  // The app already switches look via `[data-theme="dark"]` on <html> (see
  // 10-theme.js); match that instead of Tailwind's default `.dark` class or
  // `prefers-color-scheme` strategy.
  darkMode: ['selector', '[data-theme="dark"]'],
  theme: {
    // Replaced, not extended: every color utility should resolve through
    // the app's existing CSS custom properties (00-base.css) so shell
    // color/pattern/contrast switching keeps working, rather than
    // introducing a second, hardcoded palette that can drift from them.
    colors: {
      transparent: 'transparent',
      current: 'currentColor',
      bg: 'var(--bg)',
      surface: 'var(--surface)',
      'surface-subtle': 'var(--surface-subtle)',
      border: 'var(--border)',
      'border-subtle': 'var(--border-subtle)',
      text: 'var(--text)',
      'text-muted': 'var(--text-muted)',
      link: 'var(--link)',
      'link-visited': 'var(--link-visited)',
      unresolved: 'var(--unresolved)',
      'shell-bg': 'var(--shell-bg)',
      'shell-border': 'var(--shell-border)',
      'shell-text': 'var(--shell-text)',
      'shell-text-muted': 'var(--shell-text-muted)',
    },
  },
  corePlugins: {
    // 00-base.css already carries a hand-tuned reset; Tailwind's own
    // Preflight would fight it (button/heading defaults, etc.) rather than
    // layer cleanly on top.
    preflight: false,
    // `.container` is already the app's own fluid page-shell class
    // (00-base.css, paired with `.is-fluid`) -- Tailwind's own `.container`
    // utility has the same name but a totally different meaning (fixed
    // max-width per breakpoint) and would silently win the cascade over it.
    container: false,
  },
  plugins: [],
};
