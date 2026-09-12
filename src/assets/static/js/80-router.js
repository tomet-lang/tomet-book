(() => {
  const T = (window.TMT ??= {});

  // Path of the current page, used to tell whether popstate is only moving between sections.
  let lastPath = window.location.pathname;
  const initPage = (...a) => T.initPage(...a);
  const syncViewMode = (...a) => T.syncViewMode(...a);
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== VIEW TRANSITIONS ROUTER ====================
  // Intercepts in-app link clicks and performs smooth slide animations
  async function navigateTo(url, pushState = true) {
    T.hidePopover?.();
    try {
      const res = await fetch(url);
      if (!res.ok) {
        window.location.href = url;
        return;
      }
      const htmlText = await res.text();
      const parser = new DOMParser();
      const newDoc = parser.parseFromString(htmlText, 'text/html');

      // Preserve pane collapse states from local storage into new doc
      syncViewMode(newDoc);
      try {
        const isNarrow = window.innerWidth <= 900;
        const navSetting = T.storage.get('wiki-pane-nav-collapsed');
        const navCollapsed = navSetting !== null ? navSetting === 'true' : isNarrow;
        const newPaneNav = newDoc.getElementById('pane-nav');
        if (navCollapsed) {
          newPaneNav?.classList.add('is-collapsed');
        } else {
          newPaneNav?.classList.remove('is-collapsed');
        }

        const dataSetting = T.storage.get('wiki-pane-data-collapsed');
        const dataCollapsed = dataSetting !== null ? dataSetting === 'true' : isNarrow;
        if (dataCollapsed) {
          newDoc.getElementById('pane-data')?.classList.add('is-collapsed');
        }
      } catch (e) {}

      const updateDOM = () => {
        document.title = newDoc.title;

        const currentBookView = document.getElementById('wiki-book-view');
        const newBookView = newDoc.getElementById('wiki-book-view');

        // Case 1: Both pages have wiki-book-view (note to note transition)
        if (currentBookView && newBookView) {
          // Swap nav pane (breadcrumbs and TOC)
          const currentNavPane = document.getElementById('pane-nav');
          const newNavPane = newDoc.getElementById('pane-nav');
          if (currentNavPane && newNavPane) {
            const prevActivePanel = currentNavPane.dataset.activePanel || 'toc';
            currentNavPane.innerHTML = newNavPane.innerHTML;
            currentNavPane.dataset.activePanel = prevActivePanel;
          }

          // Swap data pane if exists in newDoc, or remove if not in newDoc
          const currentDataPane = document.getElementById('pane-data');
          const newDataPane = newDoc.getElementById('pane-data');
          if (newDataPane) {
            if (currentDataPane) {
              currentDataPane.innerHTML = newDataPane.innerHTML;
              currentDataPane.classList.remove('is-collapsed');
            } else {
              const contentPane = currentBookView.querySelector('.pane-content');
              if (contentPane) {
                const cloned = newDataPane.cloneNode(true);
                currentBookView.insertBefore(cloned, contentPane);
              }
            }
          } else if (currentDataPane) {
            currentDataPane.remove();
          }

          // Swap content header (sticky tabs)
          const currentContentHeader = document.querySelector('.pane-content-header');
          const newContentHeader = newDoc.querySelector('.pane-content-header');
          if (currentContentHeader && newContentHeader) {
            currentContentHeader.innerHTML = newContentHeader.innerHTML;
          }

          // Swap content container
          const currentContainer = document.querySelector('.pane-article-container');
          const newContainer = newDoc.querySelector('.pane-article-container');
          if (currentContainer && newContainer) {
            currentContainer.innerHTML = newContainer.innerHTML;
          }

          // Swap classic view
          const currentClassic = document.querySelector('.classic-view');
          const newClassic = newDoc.querySelector('.classic-view');
          if (currentClassic && newClassic) {
            currentClassic.innerHTML = newClassic.innerHTML;
          }
        } else {
          // Case 2: Transitioning to/from top page (index.html) or full structural change!
          const currentContainer = document.querySelector('.container.is-fluid');
          const newContainer = newDoc.querySelector('.container.is-fluid');
          if (currentContainer && newContainer) {
            const currentFloating = currentContainer.querySelector('.floating-controls');
            const newFloating = newContainer.querySelector('.floating-controls');
            if (newFloating) newFloating.remove();

            Array.from(currentContainer.children).forEach((child) => {
              if (child !== currentFloating) {
                child.remove();
              }
            });

            Array.from(newContainer.children).forEach((child) => {
              currentContainer.appendChild(child);
            });
          } else {
            document.body.innerHTML = newDoc.body.innerHTML;
          }
        }

        if (pushState) {
          history.pushState(null, newDoc.title, url);
        }
        lastPath = window.location.pathname;

        // Scroll article container to top
        const scrollBox = document.getElementById('book-content-scroll');
        if (scrollBox) scrollBox.scrollTop = 0;
        window.scrollTo(0, 0);

        // Rebind page events
        initPage();
      };

      if (document.startViewTransition) {
        document.startViewTransition(updateDOM);
      } else {
        updateDOM();
      }
    } catch (e) {
      console.error('Navigation failed, falling back:', e);
      window.location.href = url;
    }
  }

  function setupClientRouter() {
    document.addEventListener('click', (e) => {
      // Find nearest anchor
      let target = e.target;
      while (target && target.tagName !== 'A') {
        target = target.parentElement;
      }
      if (!target || target.tagName !== 'A') return;

      const href = target.getAttribute('href');
      if (!href) return;

      // Ignore hash links, external links, mailto, etc.
      if (href.startsWith('#') || href.startsWith('http://') || href.startsWith('https://') || href.startsWith('//') || href.startsWith('mailto:')) {
        return;
      }
      if (target.getAttribute('target') === '_blank') return;
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button !== 0) return;

      e.preventDefault();
      navigateTo(href, true);
    });

    window.addEventListener('popstate', () => {
      // If only navigating between sections on the same page, keep the existing DOM
      // and scroll to the matching section instead of rebuilding.
      if (window.location.pathname === lastPath) {
        initPage();
        return;
      }
      lastPath = window.location.pathname;
      navigateTo(window.location.href, false);
    });
  }

  Object.assign(T, { setupClientRouter });
})();
