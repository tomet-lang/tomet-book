(() => {
  const T = (window.TMT ??= {});
  const syncHeadingHash = (...a) => T.syncHeadingHash(...a);
  const t = (key, fallback) => T.t(key, fallback);

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
            syncHeadingHash(currentId);
          }
        });
      },
      { passive: true }
    );
  }

  Object.assign(T, { setupClassicViewInteraction });
})();
