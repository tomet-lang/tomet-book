(() => {
  // ==================== UI STRINGS ====================
  // Injected by base.html from `[ui.strings]`; the fallback keeps this file
  // working if it is ever loaded on a page that did not set them.
  const STRINGS = window.tmtStrings || {};
  const t = (key, fallback = "") => STRINGS[key] ?? fallback;

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
    const titleStr = isDark ? t('theme.to_light') : t('theme.to_dark');

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

  // ==================== DRAGGABLE FLOATING CONTROLS ====================
  function setupDraggableFloatingControls() {
    const floating = document.getElementById('floating-controls') || document.querySelector('.floating-controls');
    if (!floating) return;

    const handle = floating.querySelector('.floating-drag-handle');
    if (!handle) return;

    function restorePosition() {
      try {
        const saved = localStorage.getItem('wiki-floating-controls-pos');
        if (saved) {
          const { x, y } = JSON.parse(saved);
          const width = floating.offsetWidth || 280;
          const height = floating.offsetHeight || 38;
          const maxX = Math.max(8, window.innerWidth - width - 8);
          const maxY = Math.max(8, window.innerHeight - height - 8);
          const clampedX = Math.max(8, Math.min(maxX, x));
          const clampedY = Math.max(8, Math.min(maxY, y));
          floating.style.left = `${clampedX}px`;
          floating.style.top = `${clampedY}px`;
          floating.style.right = 'auto';
        }
      } catch (e) {}
    }

    restorePosition();
    requestAnimationFrame(restorePosition);

    if (floating.__dragBound) return;
    floating.__dragBound = true;

    let isDragging = false;
    let startX = 0;
    let startY = 0;
    let startLeft = 0;
    let startTop = 0;

    const onPointerDown = (e) => {
      if (e.button !== 0) return;
      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;

      const rect = floating.getBoundingClientRect();
      startLeft = rect.left;
      startTop = rect.top;

      floating.style.left = `${startLeft}px`;
      floating.style.top = `${startTop}px`;
      floating.style.right = 'auto';

      floating.classList.add('is-dragging');
      try {
        handle.setPointerCapture(e.pointerId);
      } catch (err) {}
      e.preventDefault();
      e.stopPropagation();
    };

    const onPointerMove = (e) => {
      if (!isDragging) return;
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;

      let newLeft = startLeft + dx;
      let newTop = startTop + dy;

      const minX = 8;
      const maxX = Math.max(8, window.innerWidth - floating.offsetWidth - 8);
      const minY = 8;
      const maxY = Math.max(8, window.innerHeight - floating.offsetHeight - 8);

      newLeft = Math.max(minX, Math.min(maxX, newLeft));
      newTop = Math.max(minY, Math.min(maxY, newTop));

      floating.style.left = `${newLeft}px`;
      floating.style.top = `${newTop}px`;
    };

    const onPointerUp = (e) => {
      if (!isDragging) return;
      isDragging = false;
      floating.classList.remove('is-dragging');
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch (err) {}

      const rect = floating.getBoundingClientRect();
      try {
        localStorage.setItem(
          'wiki-floating-controls-pos',
          JSON.stringify({ x: Math.round(rect.left), y: Math.round(rect.top) })
        );
      } catch (err) {}
    };

    handle.addEventListener('pointerdown', onPointerDown);
    handle.addEventListener('pointermove', onPointerMove);
    handle.addEventListener('pointerup', onPointerUp);
    handle.addEventListener('pointercancel', onPointerUp);

    window.addEventListener('resize', () => {
      if (floating.style.left && floating.style.left !== 'auto') {
        const rect = floating.getBoundingClientRect();
        const maxX = Math.max(8, window.innerWidth - floating.offsetWidth - 8);
        const maxY = Math.max(8, window.innerHeight - floating.offsetHeight - 8);
        const clampedX = Math.max(8, Math.min(maxX, rect.left));
        const clampedY = Math.max(8, Math.min(maxY, rect.top));
        floating.style.left = `${clampedX}px`;
        floating.style.top = `${clampedY}px`;
      }
    });
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

      const updateStickyTabs = setupStickyTabs(scrollContainer, headingTargets, () => {
        isClickScrolling = true;
        if (clickTimer) clearTimeout(clickTimer);
        clickTimer = setTimeout(() => {
          isClickScrolling = false;
        }, 800);
      });

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

      if (scrollContainer.__tmtScrollHandler) {
        scrollContainer.removeEventListener('scroll', scrollContainer.__tmtScrollHandler);
      }
      const onScroll = () => {
        if (isClickScrolling || scrollTicking) return;
        scrollTicking = true;
        requestAnimationFrame(() => {
          scrollTicking = false;
          updateScrollSpy();
        });
      };
      scrollContainer.__tmtScrollHandler = onScroll;
      scrollContainer.addEventListener('scroll', onScroll, { passive: true });

      // Immediate init
      updateScrollSpy();
      setTimeout(updateScrollSpy, 80);
    }

    // 付箋見出し (Sticky-Tab Headings: インライン横展開アコーディオン & ScrollSpy完全同期)
    // 構造: [ H1 ] (H2) (H2) (H3->いまアクティブなやつまでが展開される) (H2) [ H1 ] [ H1 ] ...
    function setupStickyTabs(scrollContainer, headingTargets, markClickScrolling) {
      const tabsBar = document.getElementById('sticky-tabs-bar');
      if (!tabsBar) return null;

      const nodes = Array.from(tabsBar.querySelectorAll('.sticky-tab-node'));
      if (nodes.length === 0) return null;

      function scrollToHeading(id) {
        if (!id || !scrollContainer) return;
        const target = document.getElementById(id);
        if (!target) return;
        const top = target.offsetTop - scrollContainer.offsetTop - 16;
        scrollContainer.scrollTo({ top, behavior: 'smooth' });
      }

      function setActiveHeading(currentId) {
        if (!currentId) return;

        // 1. Target node search
        let targetNode = nodes.find((n) => n.getAttribute('data-id') === currentId);

        // Fallback: If currentId is deeper (H4+), find closest preceding heading in headingTargets that exists in tabsBar
        if (!targetNode && headingTargets && headingTargets.length > 0) {
          const idx = headingTargets.findIndex((h) => h.id === currentId);
          if (idx > 0) {
            for (let i = idx - 1; i >= 0; i--) {
              const prevId = headingTargets[i].id;
              targetNode = nodes.find((n) => n.getAttribute('data-id') === prevId);
              if (targetNode) break;
            }
          }
        }

        if (!targetNode) return;

        // 2. Clear previous active/branch states
        nodes.forEach((n) => {
          n.classList.remove('is-active', 'is-active-branch');
        });
        tabsBar.querySelectorAll('.sticky-tab-btn.is-active').forEach((btn) => {
          btn.classList.remove('is-active');
        });

        // 3. Mark current target as active
        targetNode.classList.add('is-active');
        const directBtn = Array.from(targetNode.children).find((el) => el.classList.contains('sticky-tab-btn'));
        if (directBtn) {
          directBtn.classList.add('is-active');
          // Smooth scroll the tab bar horizontally to keep active tab in view
          directBtn.scrollIntoView({ inline: 'nearest', block: 'nearest', behavior: 'smooth' });
        }

        // 4. Mark all ancestors as is-active-branch so their children slots expand
        let parent = targetNode.parentElement ? targetNode.parentElement.closest('.sticky-tab-node') : null;
        while (parent) {
          parent.classList.add('is-active-branch');
          parent = parent.parentElement ? parent.parentElement.closest('.sticky-tab-node') : null;
        }
      }

      // Hover popover for child subheadings
      let popover = document.getElementById('sticky-hover-popover');
      if (!popover) {
        popover = document.createElement('div');
        popover.id = 'sticky-hover-popover';
        popover.className = 'sticky-hover-popover';
        popover.hidden = true;
        popover.setAttribute('data-pagefind-ignore', '');
        popover.innerHTML = `
          <div class="sticky-hover-popover-header">
            <span class="sticky-hover-popover-title"></span>
          </div>
          <div class="sticky-hover-popover-list"></div>
        `;
        document.body.appendChild(popover);
      }
      const popoverTitle = popover.querySelector('.sticky-hover-popover-title');
      const popoverList = popover.querySelector('.sticky-hover-popover-list');
      let hoverTimer = null;
      let closeTimer = null;

      function hidePopover() {
        if (hoverTimer) clearTimeout(hoverTimer);
        if (closeTimer) clearTimeout(closeTimer);
        if (popover) popover.hidden = true;
      }

      function showPopoverForNode(node, btn) {
        if (!popover || !popoverTitle || !popoverList) return;
        const childrenSlot = node.querySelector(':scope > .sticky-children-slot');
        if (!childrenSlot) {
          hidePopover();
          return;
        }

        const childNodes = Array.from(childrenSlot.querySelectorAll(':scope > .sticky-tab-node'));
        if (childNodes.length === 0) {
          hidePopover();
          return;
        }

        const parentText = btn.querySelector('.sticky-tab-text')?.textContent || '';
        const parentLevel = node.getAttribute('data-level') || '1';

        popoverTitle.innerHTML = `<span class="sticky-level-badge level-${parentLevel}">H${parentLevel}</span> <span>${parentText} ${t("sticky.children")} (${childNodes.length})</span>`;

        popoverList.innerHTML = '';
        childNodes.forEach((child) => {
          const childId = child.getAttribute('data-id');
          const childLevel = child.getAttribute('data-level') || '2';
          const childBtn = child.querySelector(':scope > .sticky-tab-btn');
          const childText = childBtn?.querySelector('.sticky-tab-text')?.textContent || childId;
          const isActive = child.classList.contains('is-active');

          const item = document.createElement('a');
          item.href = `#${childId}`;
          item.className = `sticky-hover-item level-${childLevel}${isActive ? ' is-active' : ''}`;
          item.setAttribute('data-target', childId);
          item.innerHTML = `
            <span class="sticky-level-badge level-${childLevel}">H${childLevel}</span>
            <span class="item-text">${childText}</span>
          `;

          item.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            hidePopover();
            if (markClickScrolling) markClickScrolling();
            setActiveHeading(childId);
            scrollToHeading(childId);
          });

          popoverList.appendChild(item);
        });

        popover.hidden = false;
        const rect = btn.getBoundingClientRect();
        const popoverWidth = Math.min(320, Math.max(200, popover.offsetWidth || 220));
        let left = rect.left;
        if (left + popoverWidth > window.innerWidth - 12) {
          left = Math.max(12, window.innerWidth - popoverWidth - 12);
        }
        popover.style.top = `${rect.bottom + 2}px`;
        popover.style.left = `${left}px`;
      }

      if (popover && !popover.__hoverBound) {
        popover.__hoverBound = true;
        popover.addEventListener('mouseenter', () => {
          if (closeTimer) clearTimeout(closeTimer);
        });
        popover.addEventListener('mouseleave', () => {
          closeTimer = setTimeout(hidePopover, 180);
        });
      }

      // Bind click & hover on all tab buttons
      nodes.forEach((node) => {
        const btn = Array.from(node.children).find((el) => el.classList.contains('sticky-tab-btn'));
        const targetId = node.getAttribute('data-id');

        btn?.addEventListener('click', (e) => {
          e.preventDefault();
          e.stopPropagation();
          hidePopover();
          if (markClickScrolling) markClickScrolling();
          setActiveHeading(targetId);
          scrollToHeading(targetId);
        });

        btn?.addEventListener('mouseenter', () => {
          if (closeTimer) clearTimeout(closeTimer);
          if (hoverTimer) clearTimeout(hoverTimer);
          hoverTimer = setTimeout(() => {
            showPopoverForNode(node, btn);
          }, 120);
        });

        btn?.addEventListener('mouseleave', () => {
          if (hoverTimer) clearTimeout(hoverTimer);
          closeTimer = setTimeout(hidePopover, 180);
        });
      });

      // Close popover on scroll
      scrollContainer?.addEventListener('scroll', () => {
        hidePopover();
      }, { passive: true });

      // Initial active state: activate the first root node if nothing is active yet
      const firstNode = nodes[0];
      if (firstNode && !tabsBar.querySelector('.sticky-tab-node.is-active')) {
        const firstId = firstNode.getAttribute('data-id');
        setActiveHeading(firstId);
      }

      // Return update function for ScrollSpy
      return function updateStickyTabsOnScroll(currentId) {
        setActiveHeading(currentId);
      };
    }

    // 1. 目次ペインと常設左縦バーの開閉制御
    function setupNavPane() {
      const titles = {
        toc: t('pane.toc'),
        links: t('pane.links'),
        graph: t('pane.graph'),
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
          // The pre-paint hint has done its job; the class is authoritative now.
          document.documentElement.removeAttribute('data-nav-collapsed');
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
        document.documentElement.removeAttribute('data-data-collapsed');
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
          resultsContainer.innerHTML = `<div class="search-status">${t('search.loading')}</div>`;
          resultsContainer.hidden = false;
          return;
        }

        const search = await pf.search(query);
        if (!search || search.results.length === 0) {
          resultsContainer.innerHTML = `<div class="search-status">${t('search.empty')}</div>`;
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
            showToast(t('toast.copied'));
          } catch {
            showToast(t('toast.copy_failed'));
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
    setupDraggableFloatingControls();
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
