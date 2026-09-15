(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== NOTION-STYLE CENTER PEEK ====================
  let activePeekUrl = null;
  let isCenterPeekBound = false;

  function closeCenterPeek() {
    const overlay = document.getElementById('wiki-center-peek');
    if (!overlay || overlay.hidden) return;
    overlay.hidden = true;
    document.body.style.overflow = '';
    activePeekUrl = null;
  }

  async function openCenterPeek(url) {
    const overlay = document.getElementById('wiki-center-peek');
    const crumbsEl = document.getElementById('peek-crumbs');
    const bodyEl = document.getElementById('peek-body');
    if (!overlay || !crumbsEl || !bodyEl) return;

    activePeekUrl = url;
    overlay.hidden = false;
    document.body.style.overflow = 'hidden';

    // Temporary loading state
    crumbsEl.innerHTML = `<span class="crumb-title">${t('search.loading', '読み込み中...')}</span>`;
    bodyEl.innerHTML = `<div class="peek-loading">${t('search.loading', '読み込み中...')}</div>`;

    const doc = await T.fetchDocument(url);
    if (!doc) {
      if (activePeekUrl !== url) return;
      bodyEl.innerHTML = `<div class="peek-loading" style="color: var(--unresolved);">${t('peek.error', 'ページの読み込みに失敗しました')}</div>`;
      return;
    }

    if (activePeekUrl !== url) return;

    // 1. Extract metadata & breadcrumbs
    const title = doc.querySelector('.content-title')?.textContent?.trim() || doc.querySelector('h1')?.textContent?.trim() || '';
    const section = doc.querySelector('.crumb-val')?.textContent?.trim() || doc.querySelector('.breadcrumb-section')?.textContent?.trim() || '';
    const icon = doc.querySelector('.hero-avatar .avatar-icon, .infobox-title span:first-child')?.textContent?.trim() || '';

    // Build header crumbs
    let crumbHtml = '';
    if (icon) {
      crumbHtml += `<span class="crumb-icon">${icon}</span>`;
    }
    if (section) {
      crumbHtml += `<span>${section}</span><span style="opacity:0.4;">/</span>`;
    }
    crumbHtml += `<span class="crumb-title">${title || url}</span>`;
    crumbsEl.innerHTML = crumbHtml;

    // 2. Extract article container
    const articleContainer = doc.querySelector('.pane-article-container');
    if (articleContainer) {
      const cloned = articleContainer.cloneNode(true);

      // Remove in-article edit menus if dev actions shouldn't pollute the modal
      cloned.querySelectorAll('.hero-actions').forEach(el => el.remove());

      bodyEl.innerHTML = '';
      bodyEl.appendChild(cloned);
      bodyEl.scrollTop = 0;

      // Handle internal links within Center Peek:
      // Clicking an in-article link performs normal navigation and closes the modal.
      cloned.querySelectorAll('a').forEach((link) => {
        const href = link.getAttribute('href');
        if (!href) return;

        // Hash anchors: smooth scroll inside peek-body
        if (href.startsWith('#')) {
          link.addEventListener('click', (e) => {
            e.preventDefault();
            const targetId = href.slice(1);
            const targetEl = bodyEl.querySelector(`[id="${CSS.escape(targetId)}"]`);
            if (targetEl) {
              targetEl.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
          });
          return;
        }

        // External links: open in new window
        if (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:')) {
          link.setAttribute('target', '_blank');
          link.setAttribute('rel', 'noopener noreferrer');
          return;
        }

        // Internal note links: navigate normally and close peek
        link.addEventListener('click', (e) => {
          e.preventDefault();
          closeCenterPeek();
          window.location.href = href;
        });
      });

      // Handle images inside Center Peek for lightbox compatibility
      cloned.querySelectorAll('article img, .hero-banner-img, .infobox-main-img').forEach((img) => {
        img.style.cursor = 'zoom-in';
        img.addEventListener('click', () => {
          const src = img.getAttribute('src');
          if (src && typeof T.openLightbox === 'function') {
            T.openLightbox(src, img.getAttribute('alt') || '');
          }
        });
      });
    } else {
      bodyEl.innerHTML = `<div class="peek-loading">本文が見つかりませんでした</div>`;
    }
  }

  function initCenterPeek() {
    const overlay = document.getElementById('wiki-center-peek');
    if (!overlay) return;

    if (!isCenterPeekBound) {
      isCenterPeekBound = true;

      // Close button
      const closeBtn = document.getElementById('peek-btn-close');
      closeBtn?.addEventListener('click', closeCenterPeek);

      // Fullscreen (open as full page) button
      const fullscreenBtn = document.getElementById('peek-btn-fullscreen');
      fullscreenBtn?.addEventListener('click', () => {
        if (!activePeekUrl) return;
        const targetUrl = activePeekUrl;
        closeCenterPeek();
        window.location.href = targetUrl;
      });

      // Close on clicking backdrop
      overlay.addEventListener('click', (e) => {
        if (e.target === overlay) {
          closeCenterPeek();
        }
      });

      // Close on Escape key
      window.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && !overlay.hidden) {
          // If lightbox is open on top, don't close center peek yet
          const lightbox = document.getElementById('wiki-image-lightbox');
          if (lightbox && !lightbox.hidden) return;

          closeCenterPeek();
        }
      });
    }
  }

  Object.assign(T, {
    initCenterPeek,
    openCenterPeek,
    closeCenterPeek,
  });
})();
