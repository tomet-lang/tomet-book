(() => {
  const T = (window.TMT ??= {} as any);
  const syncHeadingHash = (...a) => T.syncHeadingHash(...a);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== CLASSIC VIEW INTERACTIONS ====================
  function setupClassicViewInteraction() {
    const classicView = document.getElementById('wiki-classic-view');
    if (!classicView) return;

    // Helper to get heading element strictly within classic-view (to avoid matching the hidden book-shell duplicates)
    const getClassicHeadingEl = (id) => {
      if (!id) return null;
      try {
        return classicView.querySelector(`[id="${CSS.escape(id)}"]`);
      } catch {
        return null;
      }
    };

    // Heading targets & TOC links
    const classicTocLinks = Array.from(classicView.querySelectorAll('.toc a'));
    const linkById = new Map();
    classicTocLinks.forEach((link) => {
      const href = link.getAttribute('href');
      if (href && href.startsWith('#')) {
        linkById.set(href.slice(1), link);
      }
    });

    const headingEls = Array.from(
      classicView.querySelectorAll('article h1[id], article h2[id], article h3[id], article h4[id]')
    );

    if (headingEls.length === 0) {
      if (window.__tmtClassicSpy) {
        window.__tmtClassicSpy.destroy();
        window.__tmtClassicSpy = null;
      }
      return;
    }

    const headingTargets = headingEls.map((el) => ({
      id: el.id,
      el,
      link: linkById.get(el.id) || null,
    }));

    // 1. Setup Classic Sticky Tabs
    const classicTabs = document.getElementById('classic-sticky-tabs');
    const tabsBar = document.getElementById('classic-sticky-tabs-bar');
    let classicTabsHelper = null;
    let classicSpy = null;

    if (classicTabs && tabsBar) {
      let scrollFadeTimer = null;
      let isInteracting = false;

      const showTabsTemporarily = () => {
        if (document.documentElement.getAttribute('data-view') !== 'classic') return;
        classicTabs.classList.add('is-scrolling');
        if (scrollFadeTimer) {
          clearTimeout(scrollFadeTimer);
          scrollFadeTimer = null;
        }
        if (!isInteracting) {
          scrollFadeTimer = setTimeout(() => {
            classicTabs.classList.remove('is-scrolling');
            scrollFadeTimer = null;
          }, 2000);
        }
      };

      const onEnter = () => {
        if (document.documentElement.getAttribute('data-view') !== 'classic') return;
        isInteracting = true;
        classicTabs.classList.add('is-hovered');
        if (scrollFadeTimer) {
          clearTimeout(scrollFadeTimer);
          scrollFadeTimer = null;
        }
      };

      const onLeave = () => {
        if (document.documentElement.getAttribute('data-view') !== 'classic') return;
        isInteracting = false;
        classicTabs.classList.remove('is-hovered');
        if (scrollFadeTimer) clearTimeout(scrollFadeTimer);
        scrollFadeTimer = setTimeout(() => {
          classicTabs.classList.remove('is-scrolling');
          scrollFadeTimer = null;
        }, 2000);
      };

      if (!classicTabs.__interactionBound) {
        classicTabs.__interactionBound = true;
        classicTabs.addEventListener('mouseenter', onEnter);
        classicTabs.addEventListener('mouseleave', onLeave);
        classicTabs.addEventListener('touchstart', onEnter, { passive: true });
        classicTabs.addEventListener('touchend', onLeave, { passive: true });
      }

      const nodes = Array.from(tabsBar.querySelectorAll('.sticky-tab-node'));

      // Bind tab clicks
      nodes.forEach((node) => {
        const btn = node.querySelector(':scope > .sticky-tab-btn');
        const targetId = node.getAttribute('data-id');
        btn?.addEventListener('click', (e) => {
          e.preventDefault();
          const targetEl = getClassicHeadingEl(targetId);
          if (targetEl) {
            if (classicSpy) classicSpy.markClickScrolling();
            const top = targetEl.getBoundingClientRect().top + window.pageYOffset - 32;
            window.scrollTo({ top: Math.max(0, top), behavior: 'smooth' });
            syncHeadingHash(targetId);
            setActiveTab(targetId);
            showTabsTemporarily();
          }
        });
      });

      // Bind jump buttons
      classicTabs.querySelectorAll('.sticky-edge-btn').forEach((btn) => {
        if (btn.__jumpBound) return;
        btn.__jumpBound = true;
        const dir = btn.getAttribute('data-direction');
        btn.addEventListener('click', (e) => {
          e.preventDefault();
          window.scrollTo({
            top: dir === 'bottom' ? document.documentElement.scrollHeight : 0,
            behavior: 'smooth',
          });
          showTabsTemporarily();
        });
      });

      // Active tab synchronization
      function setActiveTab(activeId) {
        if (!activeId) return;
        let targetNode = nodes.find((n) => n.getAttribute('data-id') === activeId);

        // Fallback: If activeId is deeper (H4+), find closest preceding heading that exists in tabsBar
        if (!targetNode && headingTargets.length > 0) {
          const idx = headingTargets.findIndex((h) => h.id === activeId);
          if (idx > 0) {
            for (let i = idx - 1; i >= 0; i--) {
              const prev = nodes.find((n) => n.getAttribute('data-id') === headingTargets[i].id);
              if (prev) {
                targetNode = prev;
                break;
              }
            }
          }
        }

        if (!targetNode) return;

        // Check if already active
        if (targetNode.classList.contains('is-active')) return;

        // Update active classes
        nodes.forEach((n) => {
          n.classList.remove('is-active', 'is-active-branch');
          const b = n.querySelector(':scope > .sticky-tab-btn');
          b?.classList.remove('is-active');
        });

        targetNode.classList.add('is-active');
        const activeBtn = targetNode.querySelector(':scope > .sticky-tab-btn');
        activeBtn?.classList.add('is-active');

        // Expand parent branches
        let parent = targetNode.parentElement.closest('.sticky-tab-node');
        while (parent) {
          parent.classList.add('is-active-branch');
          const pb = parent.querySelector(':scope > .sticky-tab-btn');
          pb?.classList.add('is-active');
          parent = parent.parentElement.closest('.sticky-tab-node');
        }

        // Keep active tab in view inside the tabs bar
        targetNode.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }

      // Initial active tab
      if (nodes.length > 0 && !tabsBar.querySelector('.sticky-tab-node.is-active')) {
        const firstId = nodes[0].getAttribute('data-id');
        setActiveTab(firstId);
      }

      // Show initially on page load in classic view
      if (document.documentElement.getAttribute('data-view') === 'classic') {
        showTabsTemporarily();
      }

      // Listen to view mode changes
      if (!document.__classicTabsViewBound) {
        document.__classicTabsViewBound = true;
        document.addEventListener('tmt:view-mode-changed', (e) => {
          if (e.detail?.view === 'classic') {
            showTabsTemporarily();
            window.__tmtClassicSpy?.update();
          } else {
            const tabs = document.getElementById('classic-sticky-tabs');
            tabs?.classList.remove('is-scrolling', 'is-hovered');
            if (scrollFadeTimer) {
              clearTimeout(scrollFadeTimer);
              scrollFadeTimer = null;
            }
          }
        });
      }

      classicTabsHelper = {
        setActiveTab,
        showTemporarily: showTabsTemporarily,
      };
    }

    // 2. Setup ScrollSpy using T.createScrollSpy
    classicSpy = T.createScrollSpy({
      container: window,
      headingTargets,
      tocLinks: classicTocLinks,
      headingLine: 80,
      isActiveView: () => document.documentElement.getAttribute('data-view') === 'classic',
      onActiveChange: (currentId) => {
        syncHeadingHash(currentId);
        if (classicTabsHelper) {
          classicTabsHelper.setActiveTab(currentId);
        }
      },
      onScroll: () => {
        if (classicTabsHelper) {
          classicTabsHelper.showTemporarily();
        }
      },
    });

    if (window.__tmtClassicSpy) {
      window.__tmtClassicSpy.destroy();
    }
    window.__tmtClassicSpy = classicSpy;

    // TOC link smooth scrolling
    classicTocLinks.forEach((link) => {
      link.addEventListener('click', (e) => {
        const href = link.getAttribute('href');
        if (!href || !href.startsWith('#')) return;
        const targetId = href.slice(1);
        const targetEl = getClassicHeadingEl(targetId);
        if (targetEl) {
          e.preventDefault();
          classicSpy.markClickScrolling();
          const top = targetEl.getBoundingClientRect().top + window.pageYOffset - 32;
          window.scrollTo({ top: Math.max(0, top), behavior: 'smooth' });
          classicTocLinks.forEach((l) => l.classList.remove('is-active'));
          link.classList.add('is-active');
          history.pushState(null, '', href);
          if (classicTabsHelper) {
            classicTabsHelper.setActiveTab(targetId);
            classicTabsHelper.showTemporarily();
          }
        }
      });
    });

    // Initial run
    if (document.documentElement.getAttribute('data-view') === 'classic') {
      classicSpy.update();
    }
  }

  Object.assign(T, { setupClassicViewInteraction });
})();
