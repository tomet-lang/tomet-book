(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== DRAGGABLE FLOATING CONTROLS ====================
  function setupDraggableFloatingControls() {
    const floating = document.getElementById('floating-controls') || document.querySelector('.floating-controls');
    if (!floating) return;

    const handle = floating.querySelector('.floating-drag-handle');
    if (!handle) return;

    function restorePosition() {
      try {
        const saved = localStorage.getItem('wiki-floating-controls-pos');
        if (saved) {
          const { x, y } = JSON.parse(saved);
          const width = floating.offsetWidth || 280;
          const height = floating.offsetHeight || 38;
          const maxX = Math.max(8, window.innerWidth - width - 8);
          const maxY = Math.max(8, window.innerHeight - height - 8);
          const clampedX = Math.max(8, Math.min(maxX, x));
          const clampedY = Math.max(8, Math.min(maxY, y));
          floating.style.left = `${clampedX}px`;
          floating.style.top = `${clampedY}px`;
          floating.style.right = 'auto';
        }
      } catch (e) {}
    }

    restorePosition();
    requestAnimationFrame(restorePosition);

    if (floating.__dragBound) return;
    floating.__dragBound = true;

    let isDragging = false;
    let startX = 0;
    let startY = 0;
    let startLeft = 0;
    let startTop = 0;

    const onPointerDown = (e) => {
      if (e.button !== 0) return;
      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;

      const rect = floating.getBoundingClientRect();
      startLeft = rect.left;
      startTop = rect.top;

      floating.style.left = `${startLeft}px`;
      floating.style.top = `${startTop}px`;
      floating.style.right = 'auto';

      floating.classList.add('is-dragging');
      try {
        handle.setPointerCapture(e.pointerId);
      } catch (err) {}
      e.preventDefault();
      e.stopPropagation();
    };

    const onPointerMove = (e) => {
      if (!isDragging) return;
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;

      let newLeft = startLeft + dx;
      let newTop = startTop + dy;

      const minX = 8;
      const maxX = Math.max(8, window.innerWidth - floating.offsetWidth - 8);
      const minY = 8;
      const maxY = Math.max(8, window.innerHeight - floating.offsetHeight - 8);

      newLeft = Math.max(minX, Math.min(maxX, newLeft));
      newTop = Math.max(minY, Math.min(maxY, newTop));

      floating.style.left = `${newLeft}px`;
      floating.style.top = `${newTop}px`;
    };

    const onPointerUp = (e) => {
      if (!isDragging) return;
      isDragging = false;
      floating.classList.remove('is-dragging');
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch (err) {}

      const rect = floating.getBoundingClientRect();
      try {
        localStorage.setItem(
          'wiki-floating-controls-pos',
          JSON.stringify({ x: Math.round(rect.left), y: Math.round(rect.top) })
        );
      } catch (err) {}
    };

    handle.addEventListener('pointerdown', onPointerDown);
    handle.addEventListener('pointermove', onPointerMove);
    handle.addEventListener('pointerup', onPointerUp);
    handle.addEventListener('pointercancel', onPointerUp);

    window.addEventListener('resize', () => {
      if (floating.style.left && floating.style.left !== 'auto') {
        const rect = floating.getBoundingClientRect();
        const maxX = Math.max(8, window.innerWidth - floating.offsetWidth - 8);
        const maxY = Math.max(8, window.innerHeight - floating.offsetHeight - 8);
        const clampedX = Math.max(8, Math.min(maxX, rect.left));
        const clampedY = Math.max(8, Math.min(maxY, rect.top));
        floating.style.left = `${clampedX}px`;
        floating.style.top = `${clampedY}px`;
      }
    });
  }

  function setupViewSwitcher() {
    const btnBook = document.getElementById('btn-view-book');
    const btnClassic = document.getElementById('btn-view-classic');
    if (!btnBook || !btnClassic) return;

    function setView(mode) {
      document.documentElement.setAttribute('data-view', mode);
      try {
        localStorage.setItem('wiki-view-mode', mode);
      } catch {}
    }

    btnBook.onclick = () => setView('book');
    btnClassic.onclick = () => setView('classic');

    if (!window.__viewSwitcherKeyBound) {
      window.__viewSwitcherKeyBound = true;
      window.addEventListener('keydown', (e) => {
        if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable)) return;
        if (e.key.toLowerCase() === 'v' && !e.ctrlKey && !e.metaKey && !e.altKey) {
          const current = document.documentElement.getAttribute('data-view');
          setView(current === 'book' ? 'classic' : 'book');
        }
      });
    }
  }

  Object.assign(T, { setupDraggableFloatingControls, setupViewSwitcher });
})();
