(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== COSTUME SWITCHER ====================
  function setupCostumeSwitchers() {
    document.querySelectorAll('.infobox-image-section').forEach((section) => {
      if (section.__costumeBound) return;
      section.__costumeBound = true;

      const img = section.querySelector('.infobox-main-img');
      const switcher = section.querySelector('.infobox-costume-switcher');
      const wrapper = section.querySelector('.infobox-image-wrapper');
      if (!img) return;

      const buttons = switcher ? Array.from(switcher.querySelectorAll('.costume-btn')) : [];

      const selectCostume = (index) => {
        if (!buttons.length) return;
        const targetBtn = buttons[(index + buttons.length) % buttons.length];
        if (!targetBtn || targetBtn.classList.contains('active')) return;
        const src = targetBtn.getAttribute('data-img-src');
        if (!src) return;

        img.style.opacity = '0.4';
        setTimeout(() => {
          img.src = src;
          const orig = targetBtn.getAttribute('data-original');
          if (orig) {
            img.setAttribute('data-original', orig);
          } else {
            img.removeAttribute('data-original');
          }
          img.style.opacity = '1';
        }, 80);

        buttons.forEach((b) => b.classList.remove('active'));
        targetBtn.classList.add('active');
      };

      const getCurrentIndex = () => {
        const idx = buttons.findIndex((b) => b.classList.contains('active'));
        return idx >= 0 ? idx : 0;
      };

      buttons.forEach((btn, idx) => {
        btn.addEventListener('click', () => selectCostume(idx));
      });

      // Prev / Next arrow buttons
      const btnPrev = section.querySelector('.costume-arrow.prev');
      const btnNext = section.querySelector('.costume-arrow.next');
      btnPrev?.addEventListener('click', (e) => {
        e.stopPropagation();
        selectCostume(getCurrentIndex() - 1);
      });
      btnNext?.addEventListener('click', (e) => {
        e.stopPropagation();
        selectCostume(getCurrentIndex() + 1);
      });

      // Mobile Touch swipe on wrapper
      if (wrapper) {
        let touchStartX = 0;
        let touchStartY = 0;
        wrapper.addEventListener(
          'touchstart',
          (e) => {
            if (e.touches.length === 1) {
              touchStartX = e.touches[0].clientX;
              touchStartY = e.touches[0].clientY;
            }
          },
          { passive: true }
        );
        wrapper.addEventListener(
          'touchend',
          (e) => {
            if (e.changedTouches.length === 1 && buttons.length > 1) {
              const deltaX = e.changedTouches[0].clientX - touchStartX;
              const deltaY = e.changedTouches[0].clientY - touchStartY;
              if (Math.abs(deltaX) > 40 && Math.abs(deltaX) > Math.abs(deltaY) * 1.4) {
                if (deltaX > 0) {
                  selectCostume(getCurrentIndex() - 1);
                } else {
                  selectCostume(getCurrentIndex() + 1);
                }
              }
            }
          },
          { passive: true }
        );
      }
    });
  }

  // ==================== INFOBOX COPY BUTTON ====================
  if (!window.__infoboxCopyBound) {
    window.__infoboxCopyBound = true;
    document.addEventListener('click', async (e) => {
      const btn = e.target.closest('.infobox-copy-btn');
      if (!btn) return;
      e.preventDefault();
      e.stopPropagation();

      const td = btn.closest('.infobox-data');
      if (!td) return;

      const valEl = td.querySelector('.infobox-data-val') || td;
      const clone = valEl.cloneNode(true);
      clone.querySelectorAll('.infobox-copy-btn').forEach((b) => b.remove());
      const text = (clone.textContent || '').trim();
      if (!text) return;

      try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(text);
        } else {
          const textarea = document.createElement('textarea');
          textarea.value = text;
          textarea.style.position = 'fixed';
          textarea.style.opacity = '0';
          document.body.appendChild(textarea);
          textarea.select();
          document.execCommand('copy');
          document.body.removeChild(textarea);
        }

        const originalHtml = btn.innerHTML;
        btn.classList.add('is-copied');
        btn.innerHTML = `<svg viewBox="0 0 24 24" width="12" height="12" stroke="#10b981" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
        btn.setAttribute('title', t('infobox.copied', 'コピーしました'));
        btn.setAttribute('aria-label', t('infobox.copied', 'コピーしました'));

        if (btn._resetTimer) clearTimeout(btn._resetTimer);
        btn._resetTimer = setTimeout(() => {
          btn.classList.remove('is-copied');
          btn.innerHTML = originalHtml;
          btn.setAttribute('title', t('infobox.copy', '値をコピー'));
          btn.setAttribute('aria-label', t('infobox.copy', '値をコピー'));
          btn._resetTimer = null;
        }, 1800);
      } catch (err) {
        console.error('Failed to copy value:', err);
      }
    });
  }

  Object.assign(T, { setupCostumeSwitchers });
})();
