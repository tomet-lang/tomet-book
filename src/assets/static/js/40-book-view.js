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

      // Scrolling is almost always continuous, so the heading under the line
      // right now is nearly always the one found last frame, or its immediate
      // neighbour. Walking out from that cached position -- instead of
      // rescanning from the top of the document every frame -- turns this
      // from one forced-layout read per heading above the fold into one or
      // two, however deep into a long document the reader is.
      let headingCursor = 0;

      const currentHeadingId = () => {
        const n = headingTargets.length;
        if (n === 0) return null;

        const containerTop = scrollContainer.getBoundingClientRect().top;
        const line = containerTop + HEADING_LINE;
        if (headingCursor >= n) headingCursor = n - 1;

        // Retreat: the reader scrolled up past where the cursor thought they were.
        while (headingCursor > 0 && headingTargets[headingCursor].el.getBoundingClientRect().top > line) {
          headingCursor--;
        }
        // Advance: the reader scrolled down past it instead.
        while (
          headingCursor < n - 1 &&
          headingTargets[headingCursor + 1].el.getBoundingClientRect().top <= line
        ) {
          headingCursor++;
        }

        return headingTargets[headingCursor].id;
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
      // Deliberately live: the reader sees the next heading light up while
      // still mid-scroll, which is the whole point of a tab bar you can read
      // ahead on. A scroll-settle version of this traded that liveness away
      // for iPad frame budget and was tried and backed out -- being able to
      // glance at what's coming while scrolling matters more here than
      // matching a plainer wiki's low-end-device ceiling.
      const onScroll = T.rafThrottle(updateScrollSpy);
      scrollContainer.__tmtScrollHandler = onScroll;
      scrollContainer.addEventListener('scroll', onScroll, { passive: true });

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

      // Only a top-level tab ever gets pushed aside by a neighbour's slot
      // opening -- a nested child only ever moves within its own
      // already-open slot -- so only these need their shift tracked.
      const topLevelNodes = Array.from(tabsBar.querySelectorAll(':scope > .sticky-tabs-list > .sticky-tab-node'));

      // How many pixels every tab after a given one shifts when THAT one's
      // children-slot opens, measured once instead of asked of the browser
      // again on every heading crossing. A view-transition-based version of
      // this (since reverted) still forced a real, synchronous layout at the
      // moment of the crossing -- once instead of once per frame, but still
      // once *then*, which on iPad was one large stutter instead of many
      // small ones. Measuring here, up front, means a later crossing only
      // ever plays back an already-known number via `transform`.
      let expandDelta = [];
      function measureExpandDeltas() {
        expandDelta = topLevelNodes.map((node) => {
          const slot = node.querySelector(':scope > .sticky-children-slot');
          if (!slot) return 0;
          const collapsedWidth = node.offsetWidth;
          node.classList.add('is-active-branch');
          const expandedWidth = node.offsetWidth;
          node.classList.remove('is-active-branch');
          return expandedWidth - collapsedWidth;
        });
      }
      measureExpandDeltas();
      // Web fonts can still land after this and reflow the text a size
      // wider or narrower, same as the settle() pass below re-checks the
      // scroll position for the same reason.
      setTimeout(measureExpandDeltas, 150);

      // Orientation change / resize invalidates every measured width. One
      // listener for the page's lifetime (window itself outlives any single
      // SPA navigation, so this must not be re-added every time
      // setupStickyTabs runs) that always calls whichever instance is
      // current.
      window.__tmtRemeasureStickyTabs = measureExpandDeltas;
      if (!window.__tmtStickyResizeBound) {
        window.__tmtStickyResizeBound = true;
        window.addEventListener('resize', () => {
          clearTimeout(window.__tmtStickyResizeTimer);
          window.__tmtStickyResizeTimer = setTimeout(() => window.__tmtRemeasureStickyTabs?.(), 150);
        });
      }

      // Which top-level tab's slot is currently open, so the next crossing
      // knows what it's animating *from* as well as *to*. -1 is "none".
      let activeTopIdx = -1;

      function topLevelIndexOf(node) {
        let n = node;
        while (n && !topLevelNodes.includes(n)) {
          n = n.parentElement ? n.parentElement.closest('.sticky-tab-node') : null;
        }
        return n ? topLevelNodes.indexOf(n) : -1;
      }

      // FLIP the top-level tabs across an accordion open/close using the
      // deltas measured above, instead of asking the browser to lay
      // anything out to find the answer. `apply` makes the real DOM change
      // (synchronously, but nothing here reads a layout property back out
      // of it, so the browser is free to do that work whenever it likes
      // rather than being forced to finish it before the next line runs)
      // and returns whatever `onSettled` needs once the slide finishes.
      function slideTopLevelTabs(fromIdx, toIdx, apply, onSettled) {
        const fromDelta = fromIdx >= 0 ? expandDelta[fromIdx] || 0 : 0;
        const toDelta = toIdx >= 0 ? expandDelta[toIdx] || 0 : 0;

        const affected = [];
        if (fromIdx !== toIdx) {
          topLevelNodes.forEach((node, j) => {
            const oldShift = fromIdx >= 0 && j > fromIdx ? fromDelta : 0;
            const newShift = toIdx >= 0 && j > toIdx ? toDelta : 0;
            if (oldShift !== newShift) affected.push({ node, delta: oldShift - newShift });
          });
        }

        // Invert: jump every affected tab to where it visually still belongs.
        affected.forEach(({ node, delta }) => {
          node.style.transition = 'none';
          node.style.transform = `translateX(${delta}px)`;
        });

        const applied = apply();

        if (affected.length === 0) {
          onSettled(applied);
          return;
        }

        // Play: next frame, ease every tab back to its real (already-applied) spot.
        requestAnimationFrame(() => {
          let remaining = affected.length;
          const settleOne = () => {
            remaining -= 1;
            if (remaining <= 0) onSettled(applied);
          };
          affected.forEach(({ node }) => {
            node.style.transition = 'transform 0.28s cubic-bezier(0.2, 0, 0, 1)';
            node.addEventListener(
              'transitionend',
              () => {
                node.style.transition = '';
                node.style.transform = '';
                settleOne();
              },
              { once: true }
            );
            node.style.transform = '';
          });
        });
      }

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

        const targetTopIdx = topLevelIndexOf(targetNode);
        const fromTopIdx = activeTopIdx;

        const applyActiveState = () => {
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
          if (directBtn) directBtn.classList.add('is-active');

          // 4. Mark all ancestors as is-active-branch so their children slots expand
          let parent = targetNode.parentElement ? targetNode.parentElement.closest('.sticky-tab-node') : null;
          while (parent) {
            parent.classList.add('is-active-branch');
            parent = parent.parentElement ? parent.parentElement.closest('.sticky-tab-node') : null;
          }

          activeTopIdx = targetTopIdx;
          return directBtn;
        };

        // Smooth-scroll the tab bar horizontally to keep the active tab in
        // view once the slide settles -- doing it mid-slide would fight the
        // transform driving the tab there.
        const revealActiveTab = (directBtn) => {
          directBtn?.scrollIntoView({ inline: 'nearest', block: 'nearest', behavior: 'smooth' });
        };

        slideTopLevelTabs(fromTopIdx, targetTopIdx, applyActiveState, revealActiveTab);
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

    // The nav pane, the data pane, and recent-notes history are independent
    // of the scrollspy/sticky-tabs machinery above (no shared state, nothing
    // calls back into it), so they live in their own files -- 36-nav-pane.js,
    // 37-data-pane.js, 38-recent-notes.js -- and this just wires them in.
    T.setupNavPane();
    T.bindDataPaneToggle();

    const container = document.getElementById('wiki-book-view');
    requestAnimationFrame(() => {
      container?.classList.add('is-ready');
    });

    T.updateRecentNotes();
  }

  Object.assign(T, { setupBookViewInteraction, syncHeadingHash, hidePopover });
})();
