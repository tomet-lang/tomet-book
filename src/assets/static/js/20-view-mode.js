(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== VIEW MODE SWITCHER ====================
  function syncViewMode(doc = document) {
    try {
      const savedView = localStorage.getItem('wiki-view-mode');
      if (savedView === 'classic' || savedView === 'book') {
        doc.documentElement.setAttribute('data-view', savedView);
      } else if (window.innerWidth < 768) {
        doc.documentElement.setAttribute('data-view', 'classic');
      }
    } catch {}
  }

  Object.assign(T, { syncViewMode });
})();
