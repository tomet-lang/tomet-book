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
      document.querySelectorAll('.rail-tab-nav').forEach((tab) => tab.classList.remove('is-active'));
      T.storage.set('wiki-pane-nav-collapsed', 'true');
    }

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

      // Track scroll position per active panel
      const paneScroll = pNav.querySelector('.pane-scroll');
      if (paneScroll && !paneScroll.dataset.scrollBound) {
        paneScroll.dataset.scrollBound = 'true';
        const onScrollThrottled = T.rafThrottle
          ? T.rafThrottle(() => {
              const active = pNav.dataset.activePanel || 'toc';
              if (active !== 'lookup') {
                T.session?.set(`wiki-pane-nav-scroll-${active}`, String(paneScroll.scrollTop));
              }
            })
          : () => {
              const active = pNav.dataset.activePanel || 'toc';
              if (active !== 'lookup') {
                T.session?.set(`wiki-pane-nav-scroll-${active}`, String(paneScroll.scrollTop));
              }
            };
        paneScroll.addEventListener('scroll', onScrollThrottled, { passive: true });
      }
    }

    // Global delegation for rail tabs and collapse buttons
    if (!window.__navPaneDelegated) {
      window.__navPaneDelegated = true;

      document.addEventListener('click', (e) => {
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

        // Nav pane links (save scroll position before navigation)
        const navLink = e.target.closest('#pane-nav a');
        if (navLink) {
          const currentPane = document.getElementById('pane-nav');
          const paneScroll = currentPane?.querySelector('.pane-scroll');
          const active = currentPane?.dataset.activePanel || 'toc';
          if (paneScroll && active !== 'lookup') {
            T.session?.set(`wiki-pane-nav-scroll-${active}`, String(paneScroll.scrollTop));
          }
        }
      });

      document.addEventListener('keydown', (e) => {
        if ((e.key === 'Enter' || e.key === ' ') && e.target.closest('#header-collapse-nav')) {
          e.preventDefault();
          collapsePane();
        }
      });
    }
  }

  Object.assign(T, { setupNavPane });
})();
