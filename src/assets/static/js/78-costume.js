(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== COSTUME SWITCHER ====================
  function setupCostumeSwitchers() {
    document.querySelectorAll('.infobox-costume-switcher').forEach((switcher) => {
      const parent = switcher.closest('.infobox');
      if (!parent) return;
      const img = parent.querySelector('.infobox-main-img');
      if (!img) return;

      switcher.querySelectorAll('.costume-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
          const src = btn.getAttribute('data-img-src');
          if (!src) return;
          img.src = src;
          switcher.querySelectorAll('.costume-btn').forEach((b) => b.classList.remove('active'));
          btn.classList.add('active');
        });
      });
    });
  }

  Object.assign(T, { setupCostumeSwitchers });
})();
