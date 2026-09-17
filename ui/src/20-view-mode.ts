(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== VIEW MODE SWITCHER ====================
  function syncViewMode(doc = document) {
    const savedView = T.storage.get('wiki-view-mode');
    if (savedView === 'classic' || savedView === 'book') {
      doc.documentElement.setAttribute('data-view', savedView);
    } else if (window.innerWidth < 768) {
      doc.documentElement.setAttribute('data-view', 'classic');
    }
  }

  Object.assign(T, { syncViewMode });
})();
