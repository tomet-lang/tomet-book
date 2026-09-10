(() => {
  // ==================== THEME CONTROLLER ====================
  function getSystemTheme() {
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  }

  function getEffectiveTheme() {
    const forced = document.documentElement.getAttribute('data-theme');
    if (forced === 'dark' || forced === 'light') return forced;
    return getSystemTheme();
  }

  function syncThemeIcons() {
    const isDark = getEffectiveTheme() === 'dark';
    const titleStr = isDark ? 'ライトテーマに切り替え' : 'ダークテーマに切り替え';

    const themeBtns = document.querySelectorAll('.theme-toggle-btn, #btn-rail-theme');
    themeBtns.forEach((btn) => {
      btn.title = titleStr;
    });
  }

  function toggleTheme() {
    const current = getEffectiveTheme();
    const nextTheme = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', nextTheme);
    try {
      localStorage.setItem('tmtbook-theme', nextTheme);
    } catch {}
    syncThemeIcons();
  }

  function setupThemeToggle() {
    const themeBtns = document.querySelectorAll('.theme-toggle-btn, #btn-rail-theme');
    themeBtns.forEach((btn) => {
      btn.onclick = (e) => {
        e.preventDefault();
        toggleTheme();
      };
    });
    syncThemeIcons();
  }

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

    if (!window.__viewSwitcherKeyBound) {
      window.__viewSwitcherKeyBound = true;
      window.addEventListener('keydown', (e) => {
        if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable)) return;
        if (e.key.toLowerCase() === 'v' && !e.ctrlKey && !e.metaKey && !e.altKey) {
          const current = document.documentElement.getAttribute('data-view');
          setView(current === 'book' ? 'classic' : 'book');
        }
      });
    }
  }

  // ==================== BOOK VIEW INTERACTIONS ====================
  function setupBookViewInteraction() {
    const tocLinks = document.querySelectorAll('.book-toc-link');
    const scrollContainer = document.getElementById('book-content-scroll');

    const hasTabs = !!document.getElementById('sticky-tabs-bar');
    if (scrollContainer && (tocLinks.length > 0 || hasTabs)) {
      let headingTargets = Array.from(tocLinks)
        .map((link) => {
          const id = link.getAttribute('data-target');
          const el = id ? document.getElementById(id) : null;
          return el ? { id, el, link } : null;
        })
        .filter(Boolean);

      if (headingTargets.length === 0) {
        const headingEls = scrollContainer.querySelectorAll('h1[id], h2[id], h3[id], h4[id]');
        headingTargets = Array.from(headingEls).map((el) => ({
          id: el.id,
          el,
          link: null,
        }));
      }

      let isClickScrolling = false;
      let clickTimer = null;

      tocLinks.forEach((link) => {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const targetId = link.getAttribute('data-target');
          if (!targetId) return;
          const targetEl = document.getElementById(targetId);
          if (targetEl && scrollContainer) {
            isClickScrolling = true;
            if (clickTimer) clearTimeout(clickTimer);
            clickTimer = setTimeout(() => {
              isClickScrolling = false;
            }, 800);

            const top = targetEl.offsetTop - scrollContainer.offsetTop - 16;
            scrollContainer.scrollTo({ top, behavior: 'smooth' });
            tocLinks.forEach((l) => l.classList.remove('is-active'));
            link.classList.add('is-active');
          }
        });
      });

      const updateStickyTabs = setupStickyTabs(scrollContainer);

      // ScrollSpy: auto-highlight TOC item on scroll
      let scrollTicking = false;
      const updateScrollSpy = () => {
        if (isClickScrolling) return;
        const containerRect = scrollContainer.getBoundingClientRect();
        const topThreshold = containerRect.top + 80;

        let currentId = null;
        for (let i = 0; i < headingTargets.length; i++) {
          const item = headingTargets[i];
          const rect = item.el.getBoundingClientRect();
          if (rect.top <= topThreshold) {
            currentId = item.id;
          } else {
            break;
          }
        }

        if (!currentId && headingTargets.length > 0) {
          const firstRect = headingTargets[0].el.getBoundingClientRect();
          if (firstRect.top <= containerRect.bottom) {
            currentId = headingTargets[0].id;
          }
        }

        if (currentId) {
          headingTargets.forEach(({ id, link }) => {
            if (id === currentId && link) {
              if (!link.classList.contains('is-active')) {
                tocLinks.forEach((l) => l.classList.remove('is-active'));
                link.classList.add('is-active');
                link.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
              }
            }
          });
          if (updateStickyTabs) {
            updateStickyTabs(currentId);
          }
        }
      };

      scrollContainer.addEventListener(
        'scroll',
        () => {
          if (isClickScrolling || scrollTicking) return;
          scrollTicking = true;
          requestAnimationFrame(() => {
            scrollTicking = false;
            updateScrollSpy();
          });
        },
        { passive: true }
      );

      setTimeout(updateScrollSpy, 150);
    }

    // 付箋見出し (Sticky-Tab Headings)
    function setupStickyTabs(scrollContainer) {
      const tabsBar = document.getElementById('sticky-tabs-bar');
      if (!tabsBar) return null;

      const tabItems = Array.from(tabsBar.querySelectorAll('.sticky-tab-item'));
      if (tabItems.length === 0) return null;

      function closeAllDropdowns() {
        tabItems.forEach((item) => {
          item.classList.remove('is-open');
          const dd = item.querySelector('.sticky-tab-dropdown');
          if (dd) dd.hidden = true;
        });
      }

      function scrollToHeading(id) {
        if (!id || !scrollContainer) return;
        const target = document.getElementById(id);
        if (!target) return;
        const top = target.offsetTop - scrollContainer.offsetTop - 16;
        scrollContainer.scrollTo({ top, behavior: 'smooth' });
      }

      tabItems.forEach((item) => {
        const btn = item.querySelector('.sticky-tab-btn');
        const dropdown = item.querySelector('.sticky-tab-dropdown');
        const targetId = item.getAttribute('data-target');

        btn?.addEventListener('click', (e) => {
          e.stopPropagation();
          const hasChildren = !!dropdown;
          const isOpen = item.classList.contains('is-open');

          if (hasChildren) {
            if (isOpen) {
              closeAllDropdowns();
            } else {
              closeAllDropdowns();
              item.classList.add('is-open');
              dropdown.hidden = false;
            }
          } else {
            closeAllDropdowns();
            scrollToHeading(targetId);
          }
        });

        dropdown?.querySelectorAll('.sticky-dropdown-link').forEach((link) => {
          link.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            const childId = link.getAttribute('data-target');
            closeAllDropdowns();
            scrollToHeading(childId);
          });
        });
      });

      if (!window.__stickyTabsOutsideClickBound) {
        window.__stickyTabsOutsideClickBound = true;
        document.addEventListener('click', (e) => {
          if (!e.target.closest('.sticky-tab-item')) {
            document.querySelectorAll('.sticky-tab-item.is-open').forEach((item) => {
              item.classList.remove('is-open');
              const dd = item.querySelector('.sticky-tab-dropdown');
              if (dd) dd.hidden = true;
            });
          }
        });
      }

      return function updateStickyTabsOnScroll(currentId) {
        if (!currentId) return;

        let activeTabItem = null;
        let activeChildText = null;

        for (const item of tabItems) {
          const tabTarget = item.getAttribute('data-target');
          if (tabTarget === currentId) {
            activeTabItem = item;
            activeChildText = null;
            break;
          }

          const matchingChild = item.querySelector(`.sticky-dropdown-link[data-target="${currentId}"]`);
          if (matchingChild) {
            activeTabItem = item;
            const textEl = matchingChild.querySelector('.dropdown-item-text');
            activeChildText = textEl ? textEl.textContent.trim() : matchingChild.textContent.trim();
            break;
          }
        }

        if (activeTabItem) {
          tabItems.forEach((item) => {
            const subLabel = item.querySelector('.sticky-sub-active-label');
            if (item === activeTabItem) {
              if (!item.classList.contains('is-active')) {
                item.classList.add('is-active');
                item.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
              }
              if (subLabel) {
                if (activeChildText) {
                  subLabel.textContent = ` › ${activeChildText}`;
                  subLabel.hidden = false;
                } else {
                  subLabel.textContent = '';
                  subLabel.hidden = true;
                }
              }
            } else {
              item.classList.remove('is-active');
              if (subLabel) {
                subLabel.textContent = '';
                subLabel.hidden = true;
              }
            }

            item.querySelectorAll('.sticky-dropdown-link').forEach((link) => {
              if (link.getAttribute('data-target') === currentId) {
                link.classList.add('is-active');
              } else {
                link.classList.remove('is-active');
              }
            });
          });
        }
      };
    }

    // 1. 目次ペインと常設左縦バーの開閉制御
    function setupNavPane() {
      const titles = {
        toc: '目次',
        links: 'リンク',
        graph: 'グラフ',
      };

      function switchPanel(panelName, expandIfCollapsed = true) {
        const pNav = document.getElementById('pane-nav');
        if (!pNav) return;
        pNav.dataset.activePanel = panelName;

        if (expandIfCollapsed) {
          pNav.classList.remove('is-collapsed');
          try {
            localStorage.setItem('wiki-pane-nav-collapsed', 'false');
          } catch (e) {}
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
        try {
          localStorage.setItem('wiki-pane-nav-collapsed', 'true');
        } catch (e) {}
      }

      // Restore collapsed and active state on page load/transition
      const pNav = document.getElementById('pane-nav');
      if (pNav) {
        try {
          const isNarrow = window.innerWidth <= 900;
          const navSetting = localStorage.getItem('wiki-pane-nav-collapsed');
          const isCollapsed = navSetting !== null ? navSetting === 'true' : isNarrow;
          const currentPanel = pNav.dataset.activePanel || 'toc';
          if (isCollapsed) {
            collapsePane();
          } else {
            switchPanel(currentPanel, false);
          }
        } catch (e) {}
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
        });

        document.addEventListener('keydown', (e) => {
          if ((e.key === 'Enter' || e.key === ' ') && e.target.closest('#header-collapse-nav')) {
            e.preventDefault();
            collapsePane();
          }
        });
      }
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
  let isPreviewGlobalInitialized = false;

  function hidePreviewCard() {
    const card = document.getElementById('wiki-page-preview');
    if (card) card.hidden = true;
  }

  function initPagePreview() {
    if (isPreviewGlobalInitialized) return;
    isPreviewGlobalInitialized = true;

    // スクロール時および画面クリック時にプレビューを閉じる
    window.addEventListener('scroll', hidePreviewCard, { capture: true, passive: true });
    document.addEventListener('click', (e) => {
      const card = document.getElementById('wiki-page-preview');
      if (card && !card.contains(e.target)) {
        hidePreviewCard();
      }
    });

    document.addEventListener('mouseover', (e) => {
      const card = document.getElementById('wiki-page-preview');
      if (!card) return;

      const target = e.target.closest('a');
      if (!target || !target.closest('article')) return;

      const href = target.getAttribute('href');
      if (!href || href.startsWith('#') || href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:')) {
        return;
      }

      activeLink = target;
      clearTimeout(hoverTimer);

      hoverTimer = setTimeout(async () => {
        if (activeLink !== target) return;

        let data = previewCache.get(href);
        if (!data) {
          try {
            const res = await fetch(href);
            if (!res.ok) return;
            const text = await res.text();
            const doc = new DOMParser().parseFromString(text, 'text/html');

            const title = doc.querySelector('.content-title')?.textContent?.trim() || doc.querySelector('h1')?.textContent?.trim() || '';
            const section = doc.querySelector('.crumb-val')?.textContent?.trim() || doc.querySelector('.breadcrumb-section')?.textContent?.trim() || '';
            const excerpt = doc.querySelector('article p, article li, article blockquote')?.textContent?.trim() || '';
            const imgEl = doc.querySelector('.hero-banner-img, .infobox-main-img, article img');
            const image = imgEl?.getAttribute('src') || '';

            data = { title, section, excerpt, image };
            previewCache.set(href, data);
          } catch (err) {
            return;
          }
        }

        if (activeLink !== target) return;
        if (!data.excerpt && !data.title) return;

        const titleEl = card.querySelector('.preview-title');
        const sectionEl = card.querySelector('.preview-section');
        const bodyEl = card.querySelector('.preview-body');
        const thumbEl = card.querySelector('.preview-thumb');

        if (titleEl) titleEl.textContent = data.title;
        if (sectionEl) sectionEl.textContent = data.section ? `(${data.section})` : '';
        if (bodyEl) bodyEl.textContent = data.excerpt;

        if (thumbEl) {
          if (data.image) {
            thumbEl.src = data.image;
            thumbEl.hidden = false;
          } else {
            thumbEl.src = '';
            thumbEl.hidden = true;
          }
        }

        const rect = target.getBoundingClientRect();
        const cardWidth = 320;
        let left = rect.left;
        if (left + cardWidth > window.innerWidth - 16) {
          left = window.innerWidth - cardWidth - 16;
        }
        if (left < 16) left = 16;

        let top = rect.bottom + 8;
        if (top + 140 > window.innerHeight && rect.top > 150) {
          top = rect.top - 140;
        }

        card.style.left = `${Math.round(left)}px`;
        card.style.top = `${Math.round(top)}px`;
        card.hidden = false;
      }, 250);
    });

    document.addEventListener('mouseout', (e) => {
      const card = document.getElementById('wiki-page-preview');
      if (!card) return;

      const target = e.target.closest('a');
      if (target && target === activeLink) {
        activeLink = null;
        clearTimeout(hoverTimer);
        hoverTimer = setTimeout(() => {
          if (!card.matches(':hover')) {
            hidePreviewCard();
          }
        }, 150);
      }
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

    document.querySelectorAll('article img, .infobox-main-img, .hero-banner-img').forEach((img) => {
      img.addEventListener('click', () => {
        const src = img.getAttribute('src');
        if (src) openLightbox(src, img.getAttribute('alt') || '');
      });
    });

    document.querySelectorAll('.hero-zoom-btn').forEach((btn) => {
      const wrapper = btn.closest('.hero-banner-wrapper');
      const img = wrapper ? wrapper.querySelector('.hero-banner-img') : document.querySelector('.hero-banner-img');
      btn.addEventListener('click', () => {
        const src = img?.getAttribute('src');
        if (src) openLightbox(src, img.getAttribute('alt') || 'Banner');
      });
    });
  }

  // ==================== COSTUME SWITCHER ====================
  function setupCostumeSwitchers() {
    document.querySelectorAll('.infobox-costume-switcher').forEach((switcher) => {
      const parent = switcher.closest('.infobox');
      if (!parent) return;
      const img = parent.querySelector('.infobox-main-img');
      if (!img) return;

      switcher.querySelectorAll('.costume-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
          const src = btn.getAttribute('data-img-src');
          if (!src) return;
          img.src = src;
          switcher.querySelectorAll('.costume-btn').forEach((b) => b.classList.remove('active'));
          btn.classList.add('active');
        });
      });
    });
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
        if (navCollapsed) {
          newPaneNav?.classList.add('is-collapsed');
        } else {
          newPaneNav?.classList.remove('is-collapsed');
        }

        const dataSetting = localStorage.getItem('wiki-pane-data-collapsed');
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

  // ==================== CLASSIC VIEW INTERACTIONS ====================
  function setupClassicViewInteraction() {
    const classicTocLinks = document.querySelectorAll('.classic-view .toc a');
    if (classicTocLinks.length === 0) return;

    const headingTargets = Array.from(classicTocLinks)
      .map((link) => {
        const href = link.getAttribute('href');
        if (!href || !href.startsWith('#')) return null;
        const id = href.slice(1);
        const el = document.getElementById(id);
        return el ? { id, el, link } : null;
      })
      .filter(Boolean);

    let isClicking = false;
    let clickTimer = null;

    classicTocLinks.forEach((link) => {
      link.addEventListener('click', (e) => {
        const href = link.getAttribute('href');
        if (!href || !href.startsWith('#')) return;
        const targetEl = document.getElementById(href.slice(1));
        if (targetEl) {
          e.preventDefault();
          isClicking = true;
          if (clickTimer) clearTimeout(clickTimer);
          clickTimer = setTimeout(() => {
            isClicking = false;
          }, 800);
          targetEl.scrollIntoView({ behavior: 'smooth' });
          classicTocLinks.forEach((l) => l.classList.remove('is-active'));
          link.classList.add('is-active');
          history.pushState(null, '', href);
        }
      });
    });

    let ticking = false;
    window.addEventListener(
      'scroll',
      () => {
        if (document.documentElement.getAttribute('data-view') !== 'classic' || isClicking || ticking) return;
        ticking = true;
        requestAnimationFrame(() => {
          ticking = false;
          let currentId = null;

          for (let i = 0; i < headingTargets.length; i++) {
            const item = headingTargets[i];
            const rect = item.el.getBoundingClientRect();
            if (rect.top <= 100) {
              currentId = item.id;
            } else {
              break;
            }
          }

          if (!currentId && headingTargets.length > 0) {
            currentId = headingTargets[0].id;
          }

          if (currentId) {
            headingTargets.forEach(({ id, link }) => {
              if (id === currentId) {
                link.classList.add('is-active');
              } else {
                link.classList.remove('is-active');
              }
            });
          }
        });
      },
      { passive: true }
    );
  }

  // ==================== EDIT DROPDOWN & COPY TOAST ====================
  function showToast(message) {
    let toast = document.getElementById('tmt-global-toast');
    if (!toast) {
      toast = document.createElement('div');
      toast.id = 'tmt-global-toast';
      toast.className = 'tmt-toast';
      document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.classList.add('is-show');
    if (toast._timer) clearTimeout(toast._timer);
    toast._timer = setTimeout(() => {
      toast.classList.remove('is-show');
    }, 2500);
  }

  function setupEditDropdown() {
    const editBtns = document.querySelectorAll('.btn-edit-open');
    if (editBtns.length === 0) return;

    editBtns.forEach((btn) => {
      const dropdown = btn.closest('.edit-dropdown');
      if (!dropdown) return;
      const menu = dropdown.querySelector('.edit-dropdown-menu');
      if (!menu) return;

      btn.onclick = (e) => {
        e.preventDefault();
        e.stopPropagation();
        const isOpen = dropdown.classList.contains('is-open');
        // Close all other open dropdowns
        document.querySelectorAll('.edit-dropdown.is-open').forEach((d) => {
          d.classList.remove('is-open');
          const m = d.querySelector('.edit-dropdown-menu');
          if (m) m.hidden = true;
        });

        if (!isOpen) {
          dropdown.classList.add('is-open');
          menu.hidden = false;
        }
      };
    });

    // Copy path buttons
    document.querySelectorAll('.btn-copy-path').forEach((btn) => {
      btn.onclick = async (e) => {
        e.preventDefault();
        e.stopPropagation();
        const path = btn.getAttribute('data-path');
        if (path) {
          try {
            await navigator.clipboard.writeText(path);
            showToast('📋 パスをクリップボードにコピーしました');
          } catch {
            showToast('❌ コピーに失敗しました');
          }
        }
        const dropdown = btn.closest('.edit-dropdown');
        if (dropdown) {
          dropdown.classList.remove('is-open');
          const menu = dropdown.querySelector('.edit-dropdown-menu');
          if (menu) menu.hidden = true;
        }
      };
    });

    // Close on outside click
    if (!window.__editDropdownOutsideClickBound) {
      window.__editDropdownOutsideClickBound = true;
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.edit-dropdown')) {
          document.querySelectorAll('.edit-dropdown.is-open').forEach((d) => {
            d.classList.remove('is-open');
            const menu = d.querySelector('.edit-dropdown-menu');
            if (menu) menu.hidden = true;
          });
        }
      });
    }
  }

  // ==================== INITIALIZE ====================
  function initPage() {
    syncViewMode();
    setupThemeToggle();
    setupViewSwitcher();
    setupBookViewInteraction();
    setupClassicViewInteraction();
    initKeyboardNavigation();
    setupSearch();
    initPagePreview();
    initImageLightbox();
    setupCostumeSwitchers();
    setupEditDropdown();
  }

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

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initPage();
      setupClientRouter();
      setupDevServerLiveReload();
    });
  } else {
    initPage();
    setupClientRouter();
    setupDevServerLiveReload();
  }
})();
