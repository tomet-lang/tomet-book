(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

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

      // ScrollSpy: the single source of truth for "which heading am I on".
      //
      // An IntersectionObserver reports each heading as it crosses a line 80px
      // below the top of the scroll container, and carries the rectangle with
      // it -- so nothing here reads layout, and nothing runs while scrolling
      // through a long section. `above` is the heading state we care about:
      // the current heading is the last one that has passed the line.
      const above = new Map();

      const applyCurrent = () => {
        if (isClickScrolling) return;

        let currentId = null;
        for (const { id } of headingTargets) {
          if (above.get(id)) currentId = id;
        }
        // Before the first heading has scrolled past, the first one is current.
        if (!currentId && headingTargets.length > 0) {
          currentId = headingTargets[0].id;
        }
        if (!currentId) return;

        const active = headingTargets.find((item) => item.id === currentId);
        if (active?.link && !active.link.classList.contains('is-active')) {
          tocLinks.forEach((l) => l.classList.remove('is-active'));
          active.link.classList.add('is-active');
          active.link.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
        if (updateStickyTabs) {
          updateStickyTabs(currentId);
        }
      };

      // initPage() runs again on every client-side navigation and hot reload.
      scrollContainer.__tmtSpyObserver?.disconnect();
      const spy = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            // rootBounds is the band below, so its top edge *is* the line.
            const line = entry.rootBounds?.top ?? 0;
            above.set(entry.target.id, entry.boundingClientRect.top <= line);
          }
          applyCurrent();
        },
        {
          root: scrollContainer,
          // Shrink the observed area to a band starting 80px down, so crossings
          // are reported at that line rather than at the container's edge.
          rootMargin: '-80px 0px -85% 0px',
          threshold: 0,
        }
      );
      headingTargets.forEach(({ el }) => spy.observe(el));
      scrollContainer.__tmtSpyObserver = spy;
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

  Object.assign(T, { setupBookViewInteraction });
})();
