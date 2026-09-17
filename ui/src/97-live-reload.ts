(() => {
  const T = (window.TMT ??= {});
  const initPage = (...a) => T.initPage(...a);
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== SMART HOT RELOAD (DEV SERVER) ====================
  function setupDevServerLiveReload() {
    if (window.location.protocol !== 'http:' && window.location.protocol !== 'https:') return;

    function reloadCss() {
      const links = document.querySelectorAll('link[rel="stylesheet"]');
      const timestamp = Date.now();
      links.forEach((link) => {
        const url = new URL(link.href, window.location.href);
        url.searchParams.set('_tmt_ts', timestamp);
        link.href = url.toString();
      });
      console.log('[tmtbook] 🎨 CSS hot reloaded');
    }

    async function hotSwapDocument() {
      try {
        const scrollBox = document.getElementById('book-content-scroll');
        const prevScrollTop = scrollBox ? scrollBox.scrollTop : 0;
        const prevWindowScrollY = window.scrollY;

        const res = await fetch(window.location.href, { cache: 'no-store' });
        if (!res.ok) {
          window.location.reload();
          return;
        }
        const htmlText = await res.text();
        const parser = new DOMParser();
        const newDoc = parser.parseFromString(htmlText, 'text/html');

        document.title = newDoc.title;

        // 1. Swap nav pane (TOC)
        const currentNav = document.getElementById('pane-nav');
        const newNav = newDoc.getElementById('pane-nav');
        if (currentNav && newNav) {
          const prevActivePanel = currentNav.dataset.activePanel || 'toc';
          currentNav.innerHTML = newNav.innerHTML;
          currentNav.dataset.activePanel = prevActivePanel;
        }

        // 2. Swap data pane (infobox) if present
        const currentData = document.getElementById('pane-data');
        const newData = newDoc.getElementById('pane-data');
        if (newData) {
          if (currentData) {
            currentData.innerHTML = newData.innerHTML;
          } else {
            const contentPane = document.querySelector('#wiki-book-view .pane-content');
            if (contentPane) {
              contentPane.parentElement.insertBefore(newData.cloneNode(true), contentPane);
            }
          }
        } else if (currentData) {
          currentData.remove();
        }

        // 2b. Swap content header (sticky tabs)
        const currentContentHeader = document.querySelector('.pane-content-header');
        const newContentHeader = newDoc.querySelector('.pane-content-header');
        if (currentContentHeader && newContentHeader) {
          currentContentHeader.innerHTML = newContentHeader.innerHTML;
        }

        // 3. Swap article container (hero + content)
        const currentContainer = document.querySelector('.pane-article-container');
        const newContainer = newDoc.querySelector('.pane-article-container');
        if (currentContainer && newContainer) {
          currentContainer.innerHTML = newContainer.innerHTML;
        }

        // 4. Swap classic view if present
        const currentClassic = document.querySelector('.classic-view');
        const newClassic = newDoc.querySelector('.classic-view');
        if (currentClassic && newClassic) {
          currentClassic.innerHTML = newClassic.innerHTML;
        }

        // 5. Restore scroll positions
        if (scrollBox) scrollBox.scrollTop = prevScrollTop;
        window.scrollTo(0, prevWindowScrollY);

        // 6. Rebind events & interactive components
        initPage();

        console.log('[tmtbook] ⚡ Document hot-reloaded (DOM swapped, scroll preserved)');
      } catch (e) {
        console.error('[tmtbook] Hot swap failed, reloading page:', e);
        window.location.reload();
      }
    }

    function normalizePath(p) {
      if (!p) return '';
      return decodeURIComponent(p)
        .replace(/\/index\.html$/i, '')
        .replace(/\/+$/, '')
        .toLowerCase();
    }

    function isViewingUrl(targetUrl) {
      const currentPath = normalizePath(window.location.pathname);
      const targetPath = normalizePath(targetUrl);
      return currentPath === targetPath;
    }

    try {
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${wsProtocol}//${window.location.host}/live-reload`);

      ws.onmessage = (event) => {
        let data;
        try {
          data = JSON.parse(event.data);
        } catch {
          if (event.data === 'reload') data = { type: 'full' };
        }

        if (!data) return;

        if (data.type === 'css') {
          reloadCss();
        } else if (data.type === 'doc') {
          if (isViewingUrl(data.url)) {
            hotSwapDocument();
          } else {
            console.log(`[tmtbook] ℹ️ Background doc updated: ${data.url}`);
          }
        } else {
          console.log('[tmtbook] 🔄 Full reload triggered');
          window.location.reload();
        }
      };
      ws.onerror = () => {};
    } catch {}
  }
  Object.assign(T, { setupDevServerLiveReload });
})();
