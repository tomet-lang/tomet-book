(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== IMAGE LIGHTBOX ====================
  function openLightbox(src, alt = '') {
    const overlay = document.getElementById('wiki-image-lightbox');
    const lightboxImg = document.getElementById('lightbox-img');
    const lightboxCaption = document.getElementById('lightbox-caption');
    if (!overlay || !lightboxImg) return;

    lightboxImg.src = src;
    lightboxImg.alt = alt;
    if (lightboxCaption) lightboxCaption.textContent = alt;
    overlay.hidden = false;
    document.body.style.overflow = 'hidden';
  }

  function closeLightbox() {
    const overlay = document.getElementById('wiki-image-lightbox');
    const lightboxImg = document.getElementById('lightbox-img');
    if (!overlay) return;

    overlay.hidden = true;
    if (lightboxImg) lightboxImg.src = '';
    document.body.style.overflow = '';
  }

  function initImageLightbox() {
    const overlay = document.getElementById('wiki-image-lightbox');
    const closeBtn = overlay?.querySelector('.lightbox-close');
    if (!overlay) return;

    // The overlay lives in base.html and survives every DOM swap, so these
    // must be bound once -- initPage() runs again on each navigation.
    if (!overlay.__lightboxBound) {
      overlay.__lightboxBound = true;

      closeBtn?.addEventListener('click', closeLightbox);
      overlay.addEventListener('click', (e) => {
        if (e.target === overlay) closeLightbox();
      });
      window.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && !overlay.hidden) closeLightbox();
      });
    }

    // These, by contrast, are page content: new nodes every time.
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

  Object.assign(T, { initImageLightbox, openLightbox, closeLightbox });
})();
