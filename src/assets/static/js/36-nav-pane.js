(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== NAV PANE (TOC / LINKS / INDEX / GRAPH) ====================
  function setupNavPane() {
    const titles = {
      toc: t('pane.toc'),
      links: t('pane.links'),
      index: t('pane.index'),
      lookup: t('pane.lookup'),
      graph: t('pane.graph'),
    };

    function updateBackdrop(isCollapsed) {
      const backdrop = document.getElementById('pane-nav-backdrop');
      if (!backdrop) return;
      if (isCollapsed) {
        backdrop.classList.remove('is-active');
      } else {
        backdrop.classList.add('is-active');
      }
    }

    function switchPanel(panelName, expandIfCollapsed = true) {
      const pNav = document.getElementById('pane-nav');
      if (!pNav) return;
      pNav.dataset.activePanel = panelName;
      T.storage.set('wiki-pane-nav-active-panel', panelName);

      if (expandIfCollapsed) {
        pNav.classList.remove('is-collapsed');
        T.storage.set('wiki-pane-nav-collapsed', 'false');
      }

      const isCollapsed = pNav.classList.contains('is-collapsed');
      updateBackdrop(isCollapsed);

      document.querySelectorAll('.rail-tab-nav').forEach((tab) => {
        if (tab.getAttribute('data-panel') === panelName && !isCollapsed) {
          tab.classList.add('is-active');
        } else {
          tab.classList.remove('is-active');
        }
      });

      document.querySelectorAll('.pane-panel-view').forEach((view) => {
        if (view.id === `pane-panel-${panelName}`) {
          view.hidden = false;
          view.classList.add('is-active');
        } else {
          view.hidden = true;
          view.classList.remove('is-active');
        }
      });

      const paneTitle = document.getElementById('pane-nav-title');
      if (paneTitle && titles[panelName]) {
        paneTitle.textContent = titles[panelName];
      }
    }

    function collapsePane() {
      const pNav = document.getElementById('pane-nav');
      if (!pNav) return;
      pNav.classList.add('is-collapsed');
      updateBackdrop(true);
      document.querySelectorAll('.rail-tab-nav').forEach((tab) => tab.classList.remove('is-active'));
      T.storage.set('wiki-pane-nav-collapsed', 'true');
    }

    // Expose collapseNavPane for other components (e.g. router)
    T.collapseNavPane = collapsePane;

    // Restore collapsed and active state on page load/transition
    const pNav = document.getElementById('pane-nav');
    if (pNav) {
      const isNarrow = window.innerWidth <= 900;
      const navSetting = T.storage.get('wiki-pane-nav-collapsed');
      const isCollapsed = navSetting !== null ? navSetting === 'true' : isNarrow;
      const savedPanel = T.storage.get('wiki-pane-nav-active-panel');
      const currentPanel = savedPanel || pNav.dataset.activePanel || 'toc';
      if (isCollapsed) {
        collapsePane();
      } else {
        switchPanel(currentPanel, false);
      }
      // The pre-paint hint has done its job; the class is authoritative now.
      document.documentElement.removeAttribute('data-nav-collapsed');

      // Restore scroll position for non-lookup panels (e.g. index)
      if (!isCollapsed && currentPanel !== 'lookup') {
        const savedScroll = T.session?.get(`wiki-pane-nav-scroll-${currentPanel}`);
        if (savedScroll) {
          const paneScroll = pNav.querySelector('.pane-scroll');
          if (paneScroll) {
            requestAnimationFrame(() => {
              paneScroll.scrollTop = parseInt(savedScroll, 10) || 0;
            });
          }
        }
      }

      // Track scroll position per active panel (debounced to avoid synchronous storage write lag)
      const paneScroll = pNav.querySelector('.pane-scroll');
      if (paneScroll && !paneScroll.dataset.scrollBound) {
        paneScroll.dataset.scrollBound = 'true';
        let scrollTimeout = null;
        const saveScroll = () => {
          const active = pNav.dataset.activePanel || 'toc';
          if (active !== 'lookup') {
            T.session?.set(`wiki-pane-nav-scroll-${active}`, String(paneScroll.scrollTop));
          }
        };
        paneScroll.addEventListener(
          'scroll',
          () => {
            if (scrollTimeout) clearTimeout(scrollTimeout);
            scrollTimeout = setTimeout(saveScroll, 150);
          },
          { passive: true }
        );
      }
    }

    // Global delegation for rail tabs, backdrop, and collapse buttons
    if (!window.__navPaneDelegated) {
      window.__navPaneDelegated = true;

      document.addEventListener('click', (e) => {
        // Backdrop click (closes mobile drawer)
        const backdrop = e.target.closest('#pane-nav-backdrop');
        if (backdrop) {
          e.stopPropagation();
          collapsePane();
          return;
        }

        // Rail tabs
        const tab = e.target.closest('.rail-tab-nav');
        if (tab) {
          e.stopPropagation();
          const panel = tab.getAttribute('data-panel');
          if (!panel) return;
          const currentPane = document.getElementById('pane-nav');
          if (!currentPane) return;

          const isCollapsed = currentPane.classList.contains('is-collapsed');
          const active = currentPane.dataset.activePanel || 'toc';

          if (isCollapsed) {
            switchPanel(panel, true);
          } else if (active === panel) {
            collapsePane();
          } else {
            switchPanel(panel, true);
          }
          return;
        }

        // Header collapse button
        const collapseBtn = e.target.closest('#header-collapse-nav');
        if (collapseBtn) {
          e.stopPropagation();
          collapsePane();
          return;
        }

        // Nav pane links (save scroll position before navigation & close drawer on mobile)
        const navLink = e.target.closest('#pane-nav a');
        if (navLink) {
          const currentPane = document.getElementById('pane-nav');
          const paneScroll = currentPane?.querySelector('.pane-scroll');
          const active = currentPane?.dataset.activePanel || 'toc';
          if (paneScroll && active !== 'lookup') {
            T.session?.set(`wiki-pane-nav-scroll-${active}`, String(paneScroll.scrollTop));
          }
          if (window.innerWidth <= 768) {
            collapsePane();
          }
        }
      });

      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
          const currentPane = document.getElementById('pane-nav');
          if (currentPane && !currentPane.classList.contains('is-collapsed')) {
            collapsePane();
          }
        }
        if ((e.key === 'Enter' || e.key === ' ') && e.target.closest('#header-collapse-nav')) {
          e.preventDefault();
          collapsePane();
        }
      });

      // Touch swipe gestures for mobile nav drawer (edge swipe right to open, swipe left to close)
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

          // Require quick horizontal swipe (< 450ms, |deltaX| >= 45px, dominantly horizontal)
          if (duration > 450 || Math.abs(deltaX) < 45 || Math.abs(deltaX) <= Math.abs(deltaY) * 1.5) {
            return;
          }

          const currentPane = document.getElementById('pane-nav');
          if (!currentPane) return;
          const isCollapsed = currentPane.classList.contains('is-collapsed');

          // Edge swipe right from rail area (<= 60px) -> open drawer
          if (deltaX > 0 && touchStartX <= 60 && isCollapsed) {
            switchPanel(currentPane.dataset.activePanel || 'toc', true);
          }
          // Swipe left on open drawer or backdrop -> close drawer
          else if (deltaX < 0 && !isCollapsed) {
            collapsePane();
          }
        },
        { passive: true }
      );
    }
  }

  Object.assign(T, { setupNavPane });
})();
