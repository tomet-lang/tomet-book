(() => {
  const T = (window.TMT ??= {});

  // ==================== DATA PANE (INFOBOX / METADATA) EXPAND-COLLAPSE ====================
  function bindDataPaneToggle() {
    const pane = document.getElementById('pane-data');
    const btnCollapse = document.getElementById('header-collapse-data');
    const barExpand = document.getElementById('bar-expand-data');
    if (!pane) return;

    const isNarrow = window.innerWidth <= 900;
    const dataSetting = T.storage.get('wiki-pane-data-collapsed');
    const isCollapsed = dataSetting !== null ? dataSetting === 'true' : isNarrow;
    if (isCollapsed) {
      pane.classList.add('is-collapsed');
    } else {
      pane.classList.remove('is-collapsed');
    }
    document.documentElement.removeAttribute('data-data-collapsed');

    const handleCollapse = (e) => {
      e.stopPropagation();
      pane.classList.add('is-collapsed');
      T.storage.set('wiki-pane-data-collapsed', 'true');
    };

    btnCollapse?.addEventListener('click', handleCollapse);
    btnCollapse?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        handleCollapse(e);
      }
    });

    const handleExpand = (e) => {
      e.stopPropagation();
      pane.classList.remove('is-collapsed');
      T.storage.set('wiki-pane-data-collapsed', 'false');
    };

    barExpand?.addEventListener('click', handleExpand);
    barExpand?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        handleExpand(e);
      }
    });

    pane.addEventListener('click', () => {
      if (pane.classList.contains('is-collapsed')) {
        handleExpand(new Event('click'));
      }
    });
  }

  Object.assign(T, { bindDataPaneToggle });
})();
