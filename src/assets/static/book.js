(() => {
  // ==================== VIEW MODE SWITCHER ====================
  function syncViewMode(doc = document) {
    try {
      const savedView = localStorage.getItem('wiki-view-mode');
      if (savedView === 'classic' || savedView === 'book') {
        doc.documentElement.setAttribute('data-view', savedView);
      }
    } catch {}
  }

  function setupViewSwitcher() {
    const btnBook = document.getElementById('btn-view-book');
    const btnClassic = document.getElementById('btn-view-classic');
    if (!btnBook || !btnClassic) return;

    function setView(mode) {
      document.documentElement.setAttribute('data-view', mode);
      try {
        localStorage.setItem('wiki-view-mode', mode);
      } catch {}
    }

    btnBook.onclick = () => setView('book');
    btnClassic.onclick = () => setView('classic');

    window.addEventListener('keydown', (e) => {
      if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable)) return;
      if (e.key.toLowerCase() === 'v' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const current = document.documentElement.getAttribute('data-view');
        setView(current === 'book' ? 'classic' : 'book');
      }
    });
  }

  // ==================== BOOK VIEW INTERACTIONS ====================
  function setupBookViewInteraction() {
    const tocLinks = document.querySelectorAll('.book-toc-link');
    const scrollContainer = document.getElementById('book-content-scroll');

    tocLinks.forEach((link) => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const targetId = link.getAttribute('data-target');
        if (!targetId) return;
        const targetEl = document.getElementById(targetId);
        if (targetEl && scrollContainer) {
          const top = targetEl.offsetTop - scrollContainer.offsetTop - 16;
          scrollContainer.scrollTo({ top, behavior: 'smooth' });
          tocLinks.forEach((l) => l.classList.remove('is-active'));
          link.classList.add('is-active');
        }
      });
    });

    // 1. 目次ペインと常設左縦バーの開閉制御
    function setupNavPane() {
      const paneNav = document.getElementById('pane-nav');
      const btnRailToc = document.getElementById('btn-rail-toc');
      const btnCollapseNav = document.getElementById('header-collapse-nav');

      function updateNavState(isCollapsed) {
        if (!paneNav) return;
        if (isCollapsed) {
          paneNav.classList.add('is-collapsed');
          if (btnRailToc) btnRailToc.classList.remove('is-active');
        } else {
          paneNav.classList.remove('is-collapsed');
          if (btnRailToc) btnRailToc.classList.add('is-active');
        }
      }

      try {
        const isNarrow = window.innerWidth <= 900;
        const navSetting = localStorage.getItem('wiki-pane-nav-collapsed');
        const isCollapsed = navSetting !== null ? navSetting === 'true' : isNarrow;
        updateNavState(isCollapsed);
      } catch (e) {}

      btnRailToc?.addEventListener('click', (e) => {
        e.stopPropagation();
        const currentlyCollapsed = paneNav?.classList.contains('is-collapsed') ?? false;
        const next = !currentlyCollapsed;
        updateNavState(next);
        try {
          localStorage.setItem('wiki-pane-nav-collapsed', String(next));
        } catch (e) {}
      });

      const handleNavCollapse = (e) => {
        e.stopPropagation();
        updateNavState(true);
        try {
          localStorage.setItem('wiki-pane-nav-collapsed', 'true');
        } catch (e) {}
      };

      btnCollapseNav?.addEventListener('click', handleNavCollapse);
      btnCollapseNav?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          handleNavCollapse(e);
        }
      });
    }

    setupNavPane();

    // 2. データペインの開閉制御
    function bindDataPaneToggle() {
      const pane = document.getElementById('pane-data');
      const btnCollapse = document.getElementById('header-collapse-data');
      const barExpand = document.getElementById('bar-expand-data');
      if (!pane) return;

      try {
        const isNarrow = window.innerWidth <= 900;
        const dataSetting = localStorage.getItem('wiki-pane-data-collapsed');
        const isCollapsed = dataSetting !== null ? dataSetting === 'true' : isNarrow;
        if (isCollapsed) {
          pane.classList.add('is-collapsed');
        } else {
          pane.classList.remove('is-collapsed');
        }
      } catch (e) {}

      const handleCollapse = (e) => {
        e.stopPropagation();
        pane.classList.add('is-collapsed');
        try {
          localStorage.setItem('wiki-pane-data-collapsed', 'true');
        } catch (e) {}
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
        try {
          localStorage.setItem('wiki-pane-data-collapsed', 'false');
        } catch (e) {}
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

    bindDataPaneToggle();

    // 3. 目次の現在位置ハイライト (ScrollSpy)
    function setupTocScrollSpy() {
      if (!scrollContainer || tocLinks.length === 0) return;

      const headingEntries = [];
      tocLinks.forEach((link) => {
        const targetId = link.getAttribute('data-target');
        if (!targetId) return;
        const el = document.getElementById(targetId);
        if (el) headingEntries.push({ id: targetId, el, link });
      });

      if (headingEntries.length === 0) return;

      let ticking = false;
      function updateActiveHeading() {
        if (!scrollContainer) return;
        const containerRect = scrollContainer.getBoundingClientRect();
        const threshold = containerRect.top + 60;

        let activeEntry = headingEntries[0];
        for (let i = 0; i < headingEntries.length; i++) {
          const rect = headingEntries[i].el.getBoundingClientRect();
          if (rect.top <= threshold) {
            activeEntry = headingEntries[i];
          } else {
            break;
          }
        }

        headingEntries.forEach((entry) => {
          if (entry === activeEntry) {
            entry.link.classList.add('is-active');
          } else {
            entry.link.classList.remove('is-active');
          }
        });
        ticking = false;
      }

      scrollContainer.addEventListener(
        'scroll',
        () => {
          if (!ticking) {
            requestAnimationFrame(updateActiveHeading);
            ticking = true;
          }
        },
        { passive: true }
      );

      updateActiveHeading();
    }

    setupTocScrollSpy();

    const container = document.getElementById('wiki-book-view');
    requestAnimationFrame(() => {
      container?.classList.add('is-ready');
    });

    // 4. 最近開いたノート
    function updateRecentNotes() {
      const currentPath = window.location.pathname;
      const titleEl = document.querySelector('.content-title') || document.querySelector('.article-header h1');
      const currentTitle = titleEl?.textContent?.trim() || document.title.replace(/\s*\|.*$/, '');
      const iconEl = document.querySelector('.avatar-icon') || document.querySelector('.title-inline-icon');
      const currentIcon = iconEl?.textContent?.trim() || '📄';

      if (!currentPath || !currentTitle || currentPath === '/wiki' || currentPath === '/wiki/' || currentPath === '/' || currentPath === '/index.html') return;

      let recents = [];
      try {
        recents = JSON.parse(localStorage.getItem('wiki-recent-notes') || '[]');
      } catch {}

      recents = recents.filter((r) => r.path !== currentPath);
      recents.unshift({ path: currentPath, title: currentTitle, icon: currentIcon });
      if (recents.length > 8) recents = recents.slice(0, 8);

      try {
        localStorage.setItem('wiki-recent-notes', JSON.stringify(recents));
      } catch {}

      const recentSection = document.getElementById('pane-recent-section');
      const recentList = document.getElementById('pane-recent-list');
      if (recentSection && recentList && recents.length > 0) {
        recentList.innerHTML = recents
          .map(
            (r) => `
          <li>
            <a href="${r.path}" class="recent-link ${r.path === currentPath ? 'is-current' : ''}">
              <span class="recent-icon">${r.icon}</span>
              <span class="recent-title">${r.title}</span>
            </a>
          </li>
        `
          )
          .join('');
        recentSection.hidden = false;
      }
    }

    updateRecentNotes();
  }

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

  // ==================== SEARCH (PAGEFIND) ====================
  let pagefindInstance = null;
  let searchDebounceTimer = null;

  async function getPagefind() {
    if (!pagefindInstance) {
      try {
        const pagefindUrl = '/pagefind/pagefind.js';
        const pf = await import(/* @vite-ignore */ pagefindUrl);
        await pf.init();
        pagefindInstance = pf;
      } catch (e) {
        console.warn('Pagefind index not found or error loading:', e);
      }
    }
    return pagefindInstance;
  }

  function setupSearch() {
    const input = document.getElementById('wiki-search-input');
    const resultsContainer = document.getElementById('wiki-search-results');
    if (!input || !resultsContainer) return;
    if (input.dataset.searchInitialized === 'true') return;
    input.dataset.searchInitialized = 'true';

    input.addEventListener('focus', () => {
      getPagefind();
      if (input.value.trim().length > 0) {
        resultsContainer.hidden = false;
      }
    });

    let selectedIndex = -1;

    input.addEventListener('input', () => {
      clearTimeout(searchDebounceTimer);
      const query = input.value.trim();
      if (!query) {
        resultsContainer.innerHTML = '';
        resultsContainer.hidden = true;
        selectedIndex = -1;
        return;
      }

      searchDebounceTimer = setTimeout(async () => {
        const pf = await getPagefind();
        if (!pf) {
          resultsContainer.innerHTML = '<div class="search-status">検索インデックスを準備中...</div>';
          resultsContainer.hidden = false;
          return;
        }

        const search = await pf.search(query);
        if (!search || search.results.length === 0) {
          resultsContainer.innerHTML = '<div class="search-status">見つかりませんでした</div>';
          resultsContainer.hidden = false;
          selectedIndex = -1;
          return;
        }

        const topResults = await Promise.all(search.results.slice(0, 10).map((r) => r.data()));
        selectedIndex = -1;

        resultsContainer.innerHTML = topResults
          .map(
            (res, idx) => `
          <a href="${res.url}" class="search-result-item" data-index="${idx}">
            <div class="search-result-row">
              ${res.meta?.image ? `<img src="${res.meta.image}" class="search-result-thumb" alt="" loading="lazy" />` : ''}
              <div class="search-result-main">
                <div class="search-result-title">
                  <span>${res.meta?.title || 'No title'}</span>
                  ${res.filters?.kind ? `<span class="search-result-kind">${res.filters.kind}</span>` : ''}
                  ${res.filters?.section ? `<span class="search-result-section">${res.filters.section}</span>` : ''}
                </div>
                <div class="search-result-excerpt">${res.excerpt || ''}</div>
              </div>
            </div>
          </a>
        `
          )
          .join('');
        resultsContainer.hidden = false;
      }, 120);
    });

    input.addEventListener('keydown', (e) => {
      const items = resultsContainer.querySelectorAll('.search-result-item');
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (items.length > 0) {
          selectedIndex = (selectedIndex + 1) % items.length;
          updateSelected(items);
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (items.length > 0) {
          selectedIndex = (selectedIndex - 1 + items.length) % items.length;
          updateSelected(items);
        }
      } else if (e.key === 'Enter') {
        if (selectedIndex >= 0 && items[selectedIndex]) {
          e.preventDefault();
          items[selectedIndex].click();
        } else if (items.length > 0) {
          e.preventDefault();
          items[0].click();
        }
      } else if (e.key === 'Escape') {
        resultsContainer.hidden = true;
        input.blur();
      }
    });

    function updateSelected(items) {
      items.forEach((item, idx) => {
        if (idx === selectedIndex) {
          item.classList.add('selected');
          item.scrollIntoView({ block: 'nearest' });
        } else {
          item.classList.remove('selected');
        }
      });
    }

    document.addEventListener('click', (e) => {
      if (!input.contains(e.target) && !resultsContainer.contains(e.target)) {
        resultsContainer.hidden = true;
      }
    });
  }

  // Global shortcut (Ctrl+K or /)
  window.addEventListener('keydown', (e) => {
    const target = e.target;
    const isEditing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      const input = document.getElementById('wiki-search-input');
      if (input) {
        input.focus();
        input.select();
      }
    } else if (e.key === '/' && !isEditing) {
      e.preventDefault();
      const input = document.getElementById('wiki-search-input');
      if (input) {
        input.focus();
        input.select();
      }
    }
  });

  // ==================== PAGE PREVIEW ====================
  const previewCache = new Map();
  let hoverTimer = null;
  let activeLink = null;

  function hidePreviewCard() {
    const card = document.getElementById('wiki-page-preview');
    if (card) card.hidden = true;
  }

  function initPagePreview() {
    const card = document.getElementById('wiki-page-preview');
    if (!card) return;

    card.addEventListener('mouseenter', () => clearTimeout(hoverTimer));
    card.addEventListener('mouseleave', hidePreviewCard);

    const titleEl = card.querySelector('.preview-title');
    const sectionEl = card.querySelector('.preview-section');
    const bodyEl = card.querySelector('.preview-body');
    const thumbEl = card.querySelector('.preview-thumb');

    document.querySelectorAll('article a').forEach((a) => {
      const href = a.getAttribute('href');
      if (!href || href.startsWith('#') || href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:')) {
        return;
      }

      a.addEventListener('mouseenter', () => {
        clearTimeout(hoverTimer);
        activeLink = a;
        hoverTimer = setTimeout(async () => {
          if (activeLink !== a) return;
          let data = previewCache.get(href);
          if (!data) {
            try {
              const res = await fetch(href);
              if (!res.ok) return;
              const text = await res.text();
              const parser = new DOMParser();
              const doc = parser.parseFromString(text, 'text/html');

              const docTitle = doc.querySelector('.content-title')?.textContent?.trim() || doc.querySelector('h1')?.textContent?.trim() || 'No Title';
              const docSection = doc.querySelector('.crumb-val')?.textContent?.trim() || '';
              const docBody = doc.querySelector('article p')?.textContent?.trim() || '';
              const docImage = doc.querySelector('.hero-banner-img')?.getAttribute('src') || doc.querySelector('article img')?.getAttribute('src') || '';

              data = { title: docTitle, section: docSection, excerpt: docBody, image: docImage };
              previewCache.set(href, data);
            } catch (e) {
              return;
            }
          }

          if (activeLink !== a) return;
          if (titleEl) titleEl.textContent = data.title;
          if (sectionEl) sectionEl.textContent = data.section;
          if (bodyEl) bodyEl.textContent = data.excerpt;
          if (thumbEl) {
            if (data.image) {
              thumbEl.src = data.image;
              thumbEl.hidden = false;
            } else {
              thumbEl.hidden = true;
            }
          }

          const rect = a.getBoundingClientRect();
          card.hidden = false;
          let top = rect.bottom + 8;
          let left = rect.left;
          if (left + 320 > window.innerWidth - 16) {
            left = window.innerWidth - 320 - 16;
          }
          if (top + card.offsetHeight > window.innerHeight - 16) {
            top = rect.top - card.offsetHeight - 8;
          }
          card.style.top = `${Math.max(10, top)}px`;
          card.style.left = `${Math.max(10, left)}px`;
        }, 300);
      });

      a.addEventListener('mouseleave', () => {
        clearTimeout(hoverTimer);
        hoverTimer = setTimeout(hidePreviewCard, 200);
      });
    });
  }

  // ==================== IMAGE LIGHTBOX ====================
  function initImageLightbox() {
    const overlay = document.getElementById('wiki-image-lightbox');
    const lightboxImg = document.getElementById('lightbox-img');
    const lightboxCaption = document.getElementById('lightbox-caption');
    const closeBtn = overlay?.querySelector('.lightbox-close');
    if (!overlay || !lightboxImg) return;

    function openLightbox(src, alt = '') {
      lightboxImg.src = src;
      lightboxImg.alt = alt;
      if (lightboxCaption) lightboxCaption.textContent = alt;
      overlay.hidden = false;
      document.body.style.overflow = 'hidden';
    }

    function closeLightbox() {
      overlay.hidden = true;
      lightboxImg.src = '';
      document.body.style.overflow = '';
    }

    closeBtn?.addEventListener('click', closeLightbox);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) closeLightbox();
    });

    window.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && !overlay.hidden) closeLightbox();
    });

    document.querySelectorAll('article img').forEach((img) => {
      img.addEventListener('click', () => {
        const src = img.getAttribute('src');
        if (src) openLightbox(src, img.getAttribute('alt') || '');
      });
    });

    const heroZoomBtn = document.querySelector('.hero-zoom-btn');
    const heroBannerImg = document.querySelector('.hero-banner-img');
    if (heroZoomBtn && heroBannerImg) {
      heroZoomBtn.addEventListener('click', () => {
        const src = heroBannerImg.getAttribute('src');
        if (src) openLightbox(src, heroBannerImg.getAttribute('alt') || 'Banner');
      });
    }
  }

  // ==================== VIEW TRANSITIONS ROUTER ====================
  // Intercepts in-app link clicks and performs smooth slide animations
  async function navigateTo(url, pushState = true) {
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
        const navSetting = localStorage.getItem('wiki-pane-nav-collapsed');
        const navCollapsed = navSetting !== null ? navSetting === 'true' : isNarrow;
        const newPaneNav = newDoc.getElementById('pane-nav');
        const newBtnToc = newDoc.getElementById('btn-rail-toc');
        if (navCollapsed) {
          newPaneNav?.classList.add('is-collapsed');
          newBtnToc?.classList.remove('is-active');
        } else {
          newPaneNav?.classList.remove('is-collapsed');
          newBtnToc?.classList.add('is-active');
        }

        const dataSetting = localStorage.getItem('wiki-pane-data-collapsed');
        const dataCollapsed = dataSetting !== null ? dataSetting === 'true' : isNarrow;
        if (dataCollapsed) {
          newDoc.getElementById('pane-data')?.classList.add('is-collapsed');
        }
      } catch (e) {}

      const updateDOM = () => {
        document.title = newDoc.title;
        // Swap content container
        const currentContainer = document.querySelector('.pane-article-container') || document.querySelector('.classic-view');
        const newContainer = newDoc.querySelector('.pane-article-container') || newDoc.querySelector('.classic-view');

        // Swap data pane if exists
        const currentDataPane = document.getElementById('pane-data');
        const newDataPane = newDoc.getElementById('pane-data');
        if (currentDataPane && newDataPane) {
          currentDataPane.innerHTML = newDataPane.innerHTML;
        }

        // Swap nav pane (breadcrumbs and TOC)
        const currentNavPane = document.getElementById('pane-nav');
        const newNavPane = newDoc.getElementById('pane-nav');
        if (currentNavPane && newNavPane) {
          currentNavPane.innerHTML = newNavPane.innerHTML;
        }

        if (currentContainer && newContainer) {
          currentContainer.innerHTML = newContainer.innerHTML;
        } else {
          document.body.innerHTML = newDoc.body.innerHTML;
        }

        if (pushState) {
          history.pushState(null, newDoc.title, url);
        }

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
      navigateTo(window.location.href, false);
    });
  }

  // ==================== INITIALIZE ====================
  function initPage() {
    syncViewMode();
    setupViewSwitcher();
    setupBookViewInteraction();
    initKeyboardNavigation();
    setupSearch();
    initPagePreview();
    initImageLightbox();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initPage();
      setupClientRouter();
    });
  } else {
    initPage();
    setupClientRouter();
  }
})();

// ==================== LIVE RELOAD (DEV SERVER) ====================
(() => {
  if (window.location.protocol === 'http:' || window.location.protocol === 'https:') {
    try {
      const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const ws = new WebSocket(`${wsProtocol}//${window.location.host}/live-reload`);
      ws.onmessage = (event) => {
        if (event.data === 'reload') {
          console.log('[tmtbook] Rebuild detected, reloading...');
          window.location.reload();
        }
      };
      ws.onerror = () => {}; // Silently ignore on production static hosts
    } catch {}
  }
})();
