(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== BOOK VIEW INTERACTIONS ====================

  /** Heading indicated by the requested URL that exists on this page. */
  function requestedHeadingId() {
    const id = T.readHash();
    return id && document.getElementById(id) ? id : null;
  }

  /** Whether the sticky tabs bar is in vertical right-side layout. */
  function isVerticalTabs() {
    return document.documentElement.getAttribute('data-tabs') === 'right';
  }

  /** Swap the level-N / is-active state on a *reused* element (the popover
   *  and its tab clone persist across many different hover targets) without
   *  a full `className =` replacement, which would also wipe out any class
   *  not managed here. */
  function setStickyLevelState(el, level, isActive) {
    el.classList.remove('level-1', 'level-2', 'level-3', 'level-4', 'level-5', 'level-6');
    el.classList.add(`level-${level}`);
    el.classList.toggle('is-active', isActive);
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
      let headingTargets: any[] = Array.from(tocLinks)
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

      let updateStickyTabs = null;

      // ScrollSpy: unified cursor-based heading tracker from 00-utils.js
      const spy = T.createScrollSpy({
        container: scrollContainer,
        headingTargets,
        tocLinks,
        headingLine: 80,
        isActiveView: () => document.documentElement.getAttribute('data-view') !== 'classic',
        onActiveChange: (currentId) => {
          if (updateStickyTabs) {
            updateStickyTabs(currentId);
          }
          T.syncHeadingHash(currentId);
        },
      });

      if (window.__tmtBookSpy) {
        window.__tmtBookSpy.destroy();
      }
      window.__tmtBookSpy = spy;

      tocLinks.forEach((link) => {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const targetId = link.getAttribute('data-target');
          if (!targetId) return;
          const targetEl = document.getElementById(targetId);
          if (targetEl && scrollContainer) {
            spy.markClickScrolling(800);
            const top = targetEl.offsetTop - scrollContainer.offsetTop - 16;
            scrollContainer.scrollTo({ top, behavior: 'smooth' });
            tocLinks.forEach((l) => l.classList.remove('is-active'));
            link.classList.add('is-active');
            T.syncHeadingHash(targetId);
          }
        });
      });

      updateStickyTabs = setupStickyTabs(scrollContainer, headingTargets, () => {
        spy.markClickScrolling(800);
      });

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
        spy.update();
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

      const topLevelNodes = Array.from(tabsBar.querySelectorAll(':scope > .sticky-tabs-list > .sticky-tab-node'));

      // Every node whose own children-slot can expand, at any depth -- not
      // just top-level. A sibling row only ever needs to make room when the
      // slot belonging to ONE of its own members opens or closes, and that
      // happens identically at every depth (a level-2 tab's siblings get
      // pushed aside by its own H3 slot opening exactly the way a level-1
      // tab's siblings do by its H2 slot) -- so this is measured everywhere
      // once, and slideAtDivergence below picks whichever single depth
      // actually changed.
      const nodesWithSlots = nodes.filter((n) => n.querySelector(':scope > .sticky-children-slot'));

      // How many pixels every tab after a given one shifts when THAT one's
      // children-slot opens, measured once instead of asked of the browser
      // again on every heading crossing. A view-transition-based version of
      // this (since reverted) still forced a real, synchronous layout at the
      // moment of the crossing -- once instead of once per frame, but still
      // once *then*, which on iPad was one large stutter instead of many
      // small ones. Measuring here, up front, means a later crossing only
      // ever plays back an already-known number via `transform`.
      let expandDeltaByNode = new Map();
      function measureExpandDeltas() {
        // Every read batched together, then every write, then the second
        // read -- not read-write-read-write per node. Reading offsetWidth
        // right after a classList change forces the browser to run layout
        // synchronously right there, so interleaved like that this was one
        // forced reflow *per tab* on every single navigation, not just
        // once. A page with many headings (this vault's pages routinely
        // have eight or more) paid for that many reflows every time, which
        // is cheap to not notice on a fast desktop GPU but very much not on
        // a phone or a weaker laptop. Batched, it's two forced layouts
        // total regardless of how many tabs there are.
        //
        // Whatever branch is genuinely open right now must come out of this
        // the same way it went in: forcing every slot open to measure it
        // would otherwise permanently wipe is-active-branch off whatever
        // tab actually has it, silently collapsing the reader's current
        // branch the next time a resize retriggers this.
        const reallyOpen = nodesWithSlots.filter((n) => n.classList.contains('is-active-branch'));

        // The bar lays its top-level tabs out along one axis and an open
        // slot pushes later siblings aside along that same axis -- sideways
        // (offsetWidth) in the default horizontal bar, downward
        // (offsetHeight) once `data-tabs="right"` turns it into a column.
        // Every nested .sticky-children-slot inherits the same axis (see
        // 35-sticky-tabs-vertical.css), so one axis choice covers every depth.
        const vertical = isVerticalTabs();
        nodesWithSlots.forEach((n) => n.classList.remove('is-active-branch'));
        const collapsedSizes = nodesWithSlots.map((n) => (vertical ? n.offsetHeight : n.offsetWidth));
        nodesWithSlots.forEach((n) => n.classList.add('is-active-branch'));
        const expandedSizes = nodesWithSlots.map((n) => (vertical ? n.offsetHeight : n.offsetWidth));

        nodesWithSlots.forEach((n) => n.classList.remove('is-active-branch'));
        reallyOpen.forEach((n) => n.classList.add('is-active-branch'));

        expandDeltaByNode = new Map();
        nodesWithSlots.forEach((n, i) => expandDeltaByNode.set(n, expandedSizes[i] - collapsedSizes[i]));
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

      // Top-level node down to (and including) `node` itself.
      function chainOf(node) {
        const chain = [];
        let n = node;
        while (n) {
          chain.unshift(n);
          n = n.parentElement ? n.parentElement.closest('.sticky-tab-node') : null;
        }
        return chain;
      }

      // FLIP whichever single sibling row actually changes across an
      // accordion open/close, using the deltas measured above, instead of
      // asking the browser to lay anything out to find the answer. Every
      // ancestor shared by `oldChain` and `newChain` stays open the whole
      // time (nothing to slide there); every node past the divergence
      // point on the new side reveals as a whole, fresh, alongside its own
      // now-open parent (nothing to slide there either -- it was never
      // visible a moment ago). So exactly one sibling row can possibly
      // need to make room: the one right at the first point the two chains
      // disagree. `apply` makes the real DOM change (synchronously, but
      // nothing here reads a layout property back out of it, so the
      // browser is free to do that work whenever it likes rather than
      // being forced to finish it before the next line runs) and returns
      // whatever `onSettled` needs once the slide finishes.
      function slideAtDivergence(oldChain, newChain, apply, onSettled) {
        const axis = isVerticalTabs() ? 'translateY' : 'translateX';

        let k = 0;
        while (k < oldChain.length && k < newChain.length && oldChain[k] === newChain[k]) k++;

        const siblingNodes =
          k === 0
            ? topLevelNodes
            : Array.from(
                oldChain[k - 1].querySelectorAll(':scope > .sticky-children-slot > .sticky-tab-node')
              );

        const fromIdx = oldChain[k] ? siblingNodes.indexOf(oldChain[k]) : -1;
        const toIdx = newChain[k] ? siblingNodes.indexOf(newChain[k]) : -1;
        const fromDelta = fromIdx >= 0 ? expandDeltaByNode.get(siblingNodes[fromIdx]) || 0 : 0;
        const toDelta = toIdx >= 0 ? expandDeltaByNode.get(siblingNodes[toIdx]) || 0 : 0;

        const affected = [];
        if (fromIdx !== toIdx) {
          siblingNodes.forEach((node, j) => {
            const oldShift = fromIdx >= 0 && j > fromIdx ? fromDelta : 0;
            const newShift = toIdx >= 0 && j > toIdx ? toDelta : 0;
            if (oldShift !== newShift) affected.push({ node, delta: oldShift - newShift });
          });
        }

        // Invert: jump every affected tab to where it visually still belongs.
        affected.forEach(({ node, delta }) => {
          node.style.transition = 'none';
          node.style.transform = `${axis}(${delta}px)`;
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
        T.syncHeadingHash(id);
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

        const oldActiveNode = nodes.find((n) => n.classList.contains('is-active')) || null;
        const oldChain = oldActiveNode ? chainOf(oldActiveNode) : [];
        const newChain = chainOf(targetNode);

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

          return directBtn;
        };

        // Smooth-scroll the tab bar horizontally to keep the active tab in
        // view once the slide settles -- doing it mid-slide would fight the
        // transform driving the tab there.
        const revealActiveTab = (directBtn) => {
          directBtn?.scrollIntoView({ inline: 'nearest', block: 'nearest', behavior: 'smooth' });
        };

        slideAtDivergence(oldChain, newChain, applyActiveState, revealActiveTab);
      }

      // Hover popover for child subheadings. The container, its dummy tab,
      // and the list are all rendered directly in base.html, so the shape
      // lives in exactly one place instead of a parallel copy here -- every
      // use below already tolerates `popover`/`popoverTab`/`popoverList`
      // being null, since the popover only ever appears on pages that
      // actually render sticky tabs.
      const popover = document.getElementById('sticky-hover-popover');
      const popoverTab = popover?.querySelector<HTMLElement>('.sticky-popover-tab') ?? null;
      const popoverList = popover?.querySelector<HTMLElement>('.sticky-hover-popover-list') ?? null;

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

        popoverList.replaceChildren();
        const itemTemplate = document.getElementById('sticky-hover-item-template') as HTMLTemplateElement | null;
        if (!itemTemplate) return;
        childNodes.forEach((childNode) => {
          const child = childNode as HTMLElement;
          const childId = child.getAttribute('data-id');
          const childLevel = child.getAttribute('data-level') || '2';
          const childBtn = child.querySelector(':scope > .sticky-tab-btn');
          const childText = childBtn?.querySelector('.sticky-tab-text')?.textContent || childId;
          const isActive = child.classList.contains('is-active');

          const item = itemTemplate.content.firstElementChild?.cloneNode(true) as any;
          if (!item) return;
          item.href = `#${childId}`;
          item.classList.add(`level-${childLevel}`);
          if (isActive) item.classList.add('is-active');
          item.setAttribute('data-target', childId);
          const badge = item.querySelector('.sticky-level-badge');
          badge.classList.add(`level-${childLevel}`);
          badge.textContent = `H${childLevel}`;
          // .textContent, not innerHTML: childText is a heading's own text
          // content, and re-parsing it as HTML here would both risk breaking
          // on stray `<`/`&` and, if a heading ever contained real markup,
          // execute it.
          item.querySelector('.item-text').textContent = childText;

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
        setStickyLevelState(popover, parentLevel, isParentActive);

        // Populate the dummy tab clone so the tab and flyout form one seamless L-shaped sheet of paper.
        if (popoverTab) {
          setStickyLevelState(popoverTab, parentLevel, isParentActive);
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

      // Close popover on scroll. scrollContainer (#book-content-scroll)
      // wraps .pane-article-container rather than being replaced by it, so
      // it survives every SPA navigation -- same reasoning as tabsBar's
      // __wheelBound guard just below, and popover's __hoverBound above.
      // Without this, this specific listener (unlike the main scrollspy
      // handler a bit above, which already removes its old one before
      // adding a new one) piled up one more copy per navigation, each
      // still running on every scroll tick after the page that added it
      // was long gone -- the "gets janky after a while" bug.
      if (scrollContainer && !scrollContainer.__tmtPopoverCloseBound) {
        scrollContainer.__tmtPopoverCloseBound = true;
        scrollContainer.addEventListener('scroll', () => {
          hidePopover();
        }, { passive: true });
      }

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
    T.setupLookupPane();
    T.bindDataPaneToggle();

    const container = document.getElementById('wiki-book-view');
    requestAnimationFrame(() => {
      container?.classList.add('is-ready');
    });

    T.updateRecentNotes();
  }

  Object.assign(T, { setupBookViewInteraction, hidePopover });
})();
