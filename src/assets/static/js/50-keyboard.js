(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== KEYBOARD NAVIGATION ====================
  let isKeyNavInitialized = false;
  function initKeyboardNavigation() {
    if (isKeyNavInitialized) return;
    isKeyNavInitialized = true;

    window.addEventListener('keydown', (e) => {
      if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable)) return;
      if (document.documentElement.getAttribute('data-view') !== 'book') return;

      const c = document.getElementById('wiki-book-view');
      if (e.key === ']' || (e.key === 'ArrowRight' && e.altKey)) {
        c?.scrollBy({ left: 360, behavior: 'smooth' });
      } else if (e.key === '[' || (e.key === 'ArrowLeft' && e.altKey)) {
        c?.scrollBy({ left: -360, behavior: 'smooth' });
      } else if (e.key.toLowerCase() === 't' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        document.getElementById('btn-rail-toc')?.click();
      } else if (e.key.toLowerCase() === 'd' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const paneData = document.getElementById('pane-data');
        if (paneData) {
          if (paneData.classList.contains('is-collapsed')) {
            document.getElementById('bar-expand-data')?.click();
          } else {
            document.getElementById('header-collapse-data')?.click();
          }
        }
      }
    });
  }

  Object.assign(T, { initKeyboardNavigation });
})();
