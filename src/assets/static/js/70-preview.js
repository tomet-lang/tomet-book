(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

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

    // Close preview card on scroll and outside click.
    window.addEventListener('scroll', hidePreviewCard, { capture: true, passive: true });
    document.addEventListener('click', (e) => {
      const card = document.getElementById('wiki-page-preview');
      if (card && !card.contains(e.target)) {
        hidePreviewCard();
      }
    });

    const previewCardEl = document.getElementById('wiki-page-preview');
    if (previewCardEl && !previewCardEl.__peekBound) {
      previewCardEl.__peekBound = true;
      previewCardEl.addEventListener('click', () => {
        const href = previewCardEl.dataset.href;
        if (href) {
          hidePreviewCard();
          T.openCenterPeek?.(href);
        }
      });
      previewCardEl.addEventListener('mouseleave', () => {
        clearTimeout(hoverTimer);
        hoverTimer = setTimeout(() => {
          if (!previewCardEl.matches(':hover')) {
            hidePreviewCard();
          }
        }, 150);
      });
    }

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

        card.dataset.href = href;
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

  Object.assign(T, { initPagePreview });
})();
