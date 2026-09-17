(() => {
  const T = (window.TMT ??= {} as any);

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

    function updateBackdrop(isActive) {
      if (window.innerWidth > 768) return;
      const backdrop = document.getElementById('pane-nav-backdrop');
      if (backdrop) backdrop.classList.toggle('is-active', isActive);
    }

    const handleCollapse = (e) => {
      e?.stopPropagation();
      pane.classList.add('is-collapsed');
      updateBackdrop(false);
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
      e?.stopPropagation();
      pane.classList.remove('is-collapsed');
      updateBackdrop(true);
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

    // Expose for router
    T.collapseDataPane = handleCollapse;

    // Mobile backdrop click and Escape handling for data pane
    if (!window.__dataPaneGlobalBound) {
      window.__dataPaneGlobalBound = true;
      document.addEventListener('click', (e) => {
        if (window.innerWidth <= 768 && e.target.closest('#pane-nav-backdrop')) {
          const p = document.getElementById('pane-data');
          if (p && !p.classList.contains('is-collapsed')) {
            handleCollapse(e);
          }
        }
      });
      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
          const p = document.getElementById('pane-data');
          if (p && !p.classList.contains('is-collapsed')) {
            handleCollapse(e);
          }
        }
      });

      // Swipe right to close data pane on mobile
      let touchStartX = 0;
      let touchStartY = 0;
      let touchStartTime = 0;
      document.addEventListener(
        'touchstart',
        (e) => {
          if (window.innerWidth > 768 || e.touches.length !== 1) return;
          touchStartX = e.touches[0].clientX;
          touchStartY = e.touches[0].clientY;
          touchStartTime = Date.now();
        },
        { passive: true }
      );
      document.addEventListener(
        'touchend',
        (e) => {
          if (window.innerWidth > 768 || e.changedTouches.length !== 1) return;
          const deltaX = e.changedTouches[0].clientX - touchStartX;
          const deltaY = e.changedTouches[0].clientY - touchStartY;
          const duration = Date.now() - touchStartTime;
          if (duration > 450 || Math.abs(deltaX) < 45 || Math.abs(deltaX) <= Math.abs(deltaY) * 1.5) {
            return;
          }
          const p = document.getElementById('pane-data');
          // Swipe right on open data pane -> close it
          if (p && !p.classList.contains('is-collapsed') && deltaX > 0) {
            handleCollapse(e);
          }
        },
        { passive: true }
      );
    }
  }

  Object.assign(T, { bindDataPaneToggle });
})();
