(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== BOOK VIEW INTERACTIONS ====================

  /** Reflect the currently read heading in the address bar so section URLs can be copied directly. */
  function syncHeadingHash(id) {
    if (!id) return;
    if (readHash() === id) return;
    try {
      // replaceState: Avoid filling browser history while scrolling.
      history.replaceState(null, "", `#${encodeURIComponent(id)}`);
    } catch {}
  }

  /** Heading ID indicated by the current URL hash. Safely handles malformed strings. */
  function readHash() {
    const raw = (window.location.hash || "").slice(1);
    if (!raw) return "";
    try {
      return decodeURIComponent(raw);
    } catch {
      return raw;
    }
  }

  /** Heading indicated by the requested URL that exists on this page. */
  function requestedHeadingId() {
    const id = readHash();
    return id && document.getElementById(id) ? id : null;
  }

  /** Whether the sticky tabs bar is in vertical right-side layout. */
  function isVerticalTabs() {
    return document.documentElement.getAttribute('data-tabs') === 'right';
  }

  // Sticky Tabs Hover Popover Global State (persists across SPA navigations)
  let popoverHoverTimer = null;
  let popoverCloseTimer = null;

  function hidePopover() {
    if (popoverHoverTimer) {
      clearTimeout(popoverHoverTimer);
      popoverHoverTimer = null;
    }
    if (popoverCloseTimer) {
      clearTimeout(popoverCloseTimer);
      popoverCloseTimer = null;
    }
    const popover = document.getElementById('sticky-hover-popover');
    if (popover) {
      popover.hidden = true;
    }
    document
      .querySelectorAll('.sticky-tab-btn.is-popover-open')
      .forEach((el) => el.classList.remove('is-popover-open'));
  }

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
            syncHeadingHash(targetId);
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
      // This reads the whole heading list on every scroll frame rather than
      // watching for crossings with an IntersectionObserver. An observer only
      // reports a change of intersection state, and a fast scroll carries a
      // heading from below the trigger line to above it within one frame --
      // never intersecting, so never reported, leaving the highlight stuck on
      // wherever the reader used to be.
      const HEADING_LINE = 80;

      const currentHeadingId = () => {
        const containerTop = scrollContainer.getBoundingClientRect().top;
        const line = containerTop + HEADING_LINE;

        let currentId = null;
        for (const { id, el } of headingTargets) {
          // Headings are in document order, so the last one above the line wins
          // and the first one below it ends the search.
          if (el.getBoundingClientRect().top > line) break;
          currentId = id;
        }
        // Before the first heading has reached the line, it is still the one
        // the reader is under.
        return currentId ?? headingTargets[0]?.id ?? null;
      };

      const updateScrollSpy = () => {
        if (isClickScrolling) return;

        const currentId = currentHeadingId();
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
        // When classic vertical view is active, its own spy handles the hash.
        if (document.documentElement.getAttribute("data-view") !== "classic") {
          syncHeadingHash(currentId);
        }
      };

      // initPage() runs again on every client-side navigation and hot reload.
      if (scrollContainer.__tmtScrollHandler) {
        scrollContainer.removeEventListener('scroll', scrollContainer.__tmtScrollHandler);
      }
      if (scrollContainer.__tmtScrollEndHandler) {
        scrollContainer.removeEventListener('scrollend', scrollContainer.__tmtScrollEndHandler);
      }
      if (scrollContainer.__tmtScrollSettleTimer) {
        clearTimeout(scrollContainer.__tmtScrollSettleTimer);
      }

      // Sticky tabs update once the reader stops scrolling rather than on every
      // frame while they scroll: an active heading crossing into or out of a
      // branch opens or closes that branch's children-slot, which reflows the
      // tabs after it. Doing that continuously during a fast scroll is what
      // stutters on iPad; settling once per scroll gesture instead means the
      // scroll itself never competes with that layout work for frame budget.
      // Native `scrollend` fires exactly once at rest where it exists; older
      // WebKit falls back to a debounce that resets on every scroll event.
      const supportsScrollEnd = 'onscrollend' in window;
      const onScrollSettled = () => {
        scrollContainer.__tmtScrollSettleTimer = null;
        updateScrollSpy();
      };
      const onScroll = () => {
        if (supportsScrollEnd) return;
        if (scrollContainer.__tmtScrollSettleTimer) {
          clearTimeout(scrollContainer.__tmtScrollSettleTimer);
        }
        scrollContainer.__tmtScrollSettleTimer = setTimeout(onScrollSettled, 120);
      };
      scrollContainer.__tmtScrollHandler = onScroll;
      scrollContainer.addEventListener('scroll', onScroll, { passive: true });
      if (supportsScrollEnd) {
        scrollContainer.__tmtScrollEndHandler = onScrollSettled;
        scrollContainer.addEventListener('scrollend', onScrollSettled, { passive: true });
      }

      // If the opened URL targets a section, start reading from there. Native browser
      // anchor jumping does not work automatically inside an inner scroll container.
      const requested = requestedHeadingId();
      const settle = () => {
        if (requested) {
          const target = document.getElementById(requested);
          if (target) {
            // Immediate jump without smooth scrolling on initial load.
            scrollContainer.scrollTo({
              top: target.offsetTop - scrollContainer.offsetTop - 16,
              behavior: 'auto',
            });
          }
        }
        updateScrollSpy();
      };
      settle();
      // Images and fonts settle after first paint and move the headings.
      setTimeout(settle, 80);
    }

    // Sticky-Tab Headings: inline expanding accordion synchronized with ScrollSpy.
    // Structure: [ H1 ] (H2) (H2) (H3 -> expanded up to the active item) (H2) [ H1 ] [ H1 ] ...
    function setupStickyTabs(scrollContainer, headingTargets, markClickScrolling) {
      const tabsBar = document.getElementById('sticky-tabs-bar');
      if (!tabsBar) return null;

      hidePopover();

      const nodes = Array.from(tabsBar.querySelectorAll('.sticky-tab-node'));
      if (nodes.length === 0) return null;

      function scrollToHeading(id) {
        if (!id || !scrollContainer) return;
        const target = document.getElementById(id);
        if (!target) return;
        const top = target.offsetTop - scrollContainer.offsetTop - 16;
        scrollContainer.scrollTo({ top, behavior: 'smooth' });
        syncHeadingHash(id);
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

        // Scrolling calls this on every frame, and the work below rewrites
        // classes across the whole bar and scrolls it into view. If the bar
        // already shows this heading there is nothing to do -- asking the DOM
        // rather than remembering the last id keeps this right when a click
        // sets the active tab directly.
        if (targetNode.classList.contains('is-active')) return;

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
        popover.innerHTML = `<div class="sticky-popover-tab" role="button"></div><div class="sticky-hover-popover-list"></div>`;
        document.body.appendChild(popover);
      }
      let popoverTab = popover.querySelector('.sticky-popover-tab');
      let popoverList = popover.querySelector('.sticky-hover-popover-list');
      if (!popoverTab) {
        popoverTab = document.createElement('div');
        popoverTab.className = 'sticky-popover-tab';
        popoverTab.setAttribute('role', 'button');
        if (popoverList) {
          popover.insertBefore(popoverTab, popoverList);
        } else {
          popover.appendChild(popoverTab);
        }
      }
      if (!popoverList) {
        popoverList = document.createElement('div');
        popoverList.className = 'sticky-hover-popover-list';
        popover.appendChild(popoverList);
      }

      function showPopoverForNode(node, btn) {
        if (!popover || !popoverList) return;
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

        // The pointer is about to leave the tab for the popover, which would
        // drop :hover and change the tab's colour underneath it. This keeps the
        // tab looking hovered for as long as its popover is open.
        tabsBar
          .querySelectorAll('.sticky-tab-btn.is-popover-open')
          .forEach((el) => el.classList.remove('is-popover-open'));
        btn.classList.add('is-popover-open');

        // Wear the tab's own colours so the two read as one piece of paper.
        // Match the level and active state of the parent node so CSS variables
        // provide the exact same color without transition interpolation mismatch.
        const parentLevel = node.getAttribute('data-level') || (btn.classList.contains('level-1') ? '1' : btn.classList.contains('level-2') ? '2' : btn.classList.contains('level-3') ? '3' : '4');
        const isParentActive = node.classList.contains('is-active') || node.classList.contains('is-active-branch');
        popover.className = `sticky-hover-popover level-${parentLevel}${isParentActive ? ' is-active' : ''}`;

        // Populate the dummy tab clone so the tab and flyout form one seamless L-shaped sheet of paper.
        if (popoverTab) {
          popoverTab.className = `sticky-popover-tab sticky-tab-btn level-${parentLevel}${isParentActive ? ' is-active' : ''}`;
          popoverTab.innerHTML = btn.innerHTML;
          popoverTab.title = btn.title || '';
          popoverTab.onclick = (e) => {
            e.preventDefault();
            e.stopPropagation();
            hidePopover();
            btn.click();
          };
        }

        const rect = btn.getBoundingClientRect();
        popover.style.width = '';
        const popoverWidth = Math.min(320, Math.max(220, popover.offsetWidth || 220));

        if (isVerticalTabs()) {
          // In vertical mode, the popover expands leftward with the dummy tab overlaid on the parent tab.
          popover.style.width = `${popoverWidth}px`;
          const targetHeight = Math.min(280, popoverList.scrollHeight || 200);
          let popoverTop = rect.top;
          if (popoverTop + targetHeight > window.innerHeight - 16) {
            popoverTop = Math.max(12, window.innerHeight - targetHeight - 16);
          }
          popoverList.style.maxHeight = `${Math.min(280, Math.max(120, window.innerHeight - popoverTop - 24))}px`;
          popover.style.top = `${popoverTop}px`;
          popover.style.left = `${Math.max(12, rect.left - popoverWidth)}px`;

          if (popoverTab) {
            popoverTab.style.top = `${rect.top - popoverTop}px`;
            popoverTab.style.width = `${rect.width + 1}px`;
            popoverTab.style.height = `${rect.height}px`;
          }
        } else {
          popover.style.width = '';
          popoverList.style.maxHeight = '280px';
          let left = rect.left;
          if (left + popoverWidth > window.innerWidth - 12) {
            left = Math.max(12, window.innerWidth - popoverWidth - 12);
          }
          // Flush against the tab: any gap breaks the join.
          popover.style.top = `${rect.bottom}px`;
          popover.style.left = `${left}px`;
        }
      }

      if (popover && !popover.__hoverBound) {
        popover.__hoverBound = true;
        popover.addEventListener('mouseenter', () => {
          if (popoverCloseTimer) {
            clearTimeout(popoverCloseTimer);
            popoverCloseTimer = null;
          }
        });
        popover.addEventListener('mouseleave', (e) => {
          const toEl = e.relatedTarget;
          if (toEl && toEl.closest && toEl.closest('.sticky-tab-btn')) {
            return;
          }
          if (popoverCloseTimer) clearTimeout(popoverCloseTimer);
          popoverCloseTimer = setTimeout(hidePopover, 180);
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
          if (popoverCloseTimer) {
            clearTimeout(popoverCloseTimer);
            popoverCloseTimer = null;
          }
          if (popoverHoverTimer) clearTimeout(popoverHoverTimer);
          popoverHoverTimer = setTimeout(() => {
            showPopoverForNode(node, btn);
          }, 120);
        });

        btn?.addEventListener('mouseleave', (e) => {
          if (popoverHoverTimer) {
            clearTimeout(popoverHoverTimer);
            popoverHoverTimer = null;
          }
          const toEl = e.relatedTarget;
          if (popover && toEl && popover.contains(toEl)) {
            return;
          }
          if (popoverCloseTimer) clearTimeout(popoverCloseTimer);
          popoverCloseTimer = setTimeout(hidePopover, 180);
        });
      });

      // Close popover on scroll
      scrollContainer?.addEventListener('scroll', () => {
        hidePopover();
      }, { passive: true });

      // The bar scrolls horizontally but hides its scrollbar, and a wheel only
      // scrolls vertically by default -- which does nothing here. Turn wheel
      // movement in either axis into horizontal travel along the tabs.
      //
      // Assigning scrollLeft per event would step the bar a notch at a time.
      // Instead each notch moves a target, and a frame loop eases towards it,
      // so a burst of notches becomes one continuous glide.
      if (!tabsBar.__wheelBound) {
        tabsBar.__wheelBound = true;

        let wheelTarget = null;
        let wheelFrame = null;
        const maxScrollLeft = () => Math.max(0, tabsBar.scrollWidth - tabsBar.clientWidth);

        const glide = () => {
          const remaining = wheelTarget - tabsBar.scrollLeft;
          if (Math.abs(remaining) < 0.5) {
            tabsBar.scrollLeft = wheelTarget;
            wheelTarget = null;
            wheelFrame = null;
            return;
          }
          tabsBar.scrollLeft += remaining * 0.22;
          wheelFrame = requestAnimationFrame(glide);
        };

        tabsBar.addEventListener(
          'wheel',
          (e) => {
            // In vertical mode the bar scrolls vertically, so standard browser scrolling applies directly.
            if (isVerticalTabs()) return;
            const delta = Math.abs(e.deltaY) > Math.abs(e.deltaX) ? e.deltaY : e.deltaX;
            if (!delta) return;

            const from = wheelTarget ?? tabsBar.scrollLeft;
            const to = Math.max(0, Math.min(maxScrollLeft(), from + delta));
            // At either end, leave the event alone so the page still scrolls.
            if (to === from) return;

            e.preventDefault();
            wheelTarget = to;
            if (wheelFrame === null) wheelFrame = requestAnimationFrame(glide);
          },
          { passive: false }
        );
      }

      // Jump to the start or the end of the document.
      //
      // No markClickScrolling here: a click on a tab suppresses the scrollspy
      // so the tab you picked stays lit while the page travels, but a jump has
      // no heading of its own -- the spy should follow the page as it goes.
      const jumpTo = (position) => {
        if (!scrollContainer) return;
        hidePopover();
        scrollContainer.scrollTo({
          top: position === 'end' ? scrollContainer.scrollHeight : 0,
          behavior: 'smooth',
        });
      };

      [
        ['btn-sticky-top', 'start'],
        ['btn-sticky-bottom', 'end'],
      ].forEach(([id, position]) => {
        const button = document.getElementById(id);
        if (!button || button.__jumpBound) return;
        button.__jumpBound = true;
        button.addEventListener('click', (e) => {
          e.preventDefault();
          jumpTo(position);
        });
      });

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

    // 1. Navigation pane and permanent left rail toggle control
    function setupNavPane() {
      const titles = {
        toc: t('pane.toc'),
        links: t('pane.links'),
        index: t('pane.index'),
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

    // 2. Data pane expand/collapse control
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

    // 4. Recent notes history
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

  Object.assign(T, { setupBookViewInteraction, syncHeadingHash, hidePopover });
})();
