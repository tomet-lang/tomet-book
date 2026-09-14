(() => {
  const T = (window.TMT ??= {});
  const syncHeadingHash = (...a) => T.syncHeadingHash(...a);
  const t = (key, fallback) => T.t(key, fallback);

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

    // Setup ScrollSpy targets from headings within classic-view
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
      if (window.__tmtClassicScrollHandler) {
        window.removeEventListener('scroll', window.__tmtClassicScrollHandler);
        window.__tmtClassicScrollHandler = null;
      }
      return;
    }

    const headingTargets = headingEls.map((el) => ({
      id: el.id,
      el,
      link: linkById.get(el.id) || null,
    }));

    const classicSpy = T.createScrollSpy({
      container: window,
      headingTargets,
      tocLinks: classicTocLinks,
      headingLine: 80,
      isActiveView: () => document.documentElement.getAttribute('data-view') === 'classic',
      onActiveChange: (currentId) => {
        syncHeadingHash(currentId);
      },
    });

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
