(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== DRAGGABLE FLOATING CONTROLS ====================
  function setupDraggableFloatingControls() {
    const floating = document.getElementById('floating-controls') || document.querySelector('.floating-controls');
    if (!floating) return;

    const handle = floating.querySelector('.floating-drag-handle');
    if (!handle) return;

    // Matches 90-responsive.css's breakpoint, where .floating-controls
    // docks to the bottom edge with !important instead of floating.
    // Positioning it with an inline left/top from a desktop drag -- saved
    // from back when it *was* a float -- would win over that CSS (inline
    // style always beats a stylesheet rule, !important or not) and leave
    // it stranded wherever it last got dragged on a wide screen.
    const isNarrowViewport = () => window.innerWidth <= 768;

    function restorePosition() {
      if (isNarrowViewport()) return;
      try {
        const saved = T.storage.get('wiki-floating-controls-pos');
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
    // Cache dimensions at the time drag starts. Reading offsetWidth during drag
    // forces synchronous layout recalculation due to recently modified left/top,
    // causing the element to trail behind the pointer by several frames.
    let dragWidth = 0;
    let dragHeight = 0;
    let offsetLeft = 0;
    let offsetTop = 0;
    // Coalesce renders to once per frame since pointermove fires faster than rAF.
    // Use transform during drag so updates run entirely on the compositor
    // without triggering layout on every frame.
    const applyDragTransform = T.rafThrottle(() => {
      floating.style.transform = `translate3d(${offsetLeft}px, ${offsetTop}px, 0)`;
    });

    const onPointerDown = (e) => {
      if (e.button !== 0) return;
      isDragging = true;
      startX = e.clientX;
      startY = e.clientY;

      const rect = floating.getBoundingClientRect();
      startLeft = rect.left;
      startTop = rect.top;
      dragWidth = rect.width;
      dragHeight = rect.height;
      offsetLeft = 0;
      offsetTop = 0;

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

      // Bound using only window dimensions and cached element size; avoid reading from the DOM.
      const maxX = Math.max(8, window.innerWidth - dragWidth - 8);
      const maxY = Math.max(8, window.innerHeight - dragHeight - 8);
      const left = Math.max(8, Math.min(maxX, startLeft + (e.clientX - startX)));
      const top = Math.max(8, Math.min(maxY, startTop + (e.clientY - startY)));
      offsetLeft = left - startLeft;
      offsetTop = top - startTop;
      applyDragTransform();
    };

    const onPointerUp = (e) => {
      if (!isDragging) return;
      isDragging = false;
      floating.classList.remove('is-dragging');
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch (err) {}

      applyDragTransform.cancel();

      // Commit the drag transform into definitive left/top positions.
      const finalLeft = Math.round(startLeft + offsetLeft);
      const finalTop = Math.round(startTop + offsetTop);
      floating.style.transform = '';
      floating.style.left = `${finalLeft}px`;
      floating.style.top = `${finalTop}px`;

      T.storage.set('wiki-floating-controls-pos', JSON.stringify({ x: finalLeft, y: finalTop }));
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
      T.storage.set('wiki-view-mode', mode);
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

  // ==================== STICKY TAB SIDE (TOP / RIGHT) ====================
  // base.html sets data-tabs before first paint; this switcher simply updates
  // and persists the attribute when clicked.
  function setupTabsSideSwitcher() {
    const btnTop = document.getElementById('btn-tabs-top');
    const btnRight = document.getElementById('btn-tabs-right');
    if (!btnTop || !btnRight) return;

    const sync = () => {
      const side = document.documentElement.getAttribute('data-tabs') === 'right' ? 'right' : 'top';
      btnTop.classList.toggle('is-active', side === 'top');
      btnRight.classList.toggle('is-active', side === 'right');
    };

    const setSide = (side) => {
      document.documentElement.setAttribute('data-tabs', side);
      T.storage.set('wiki-tabs-side', side);
      sync();
    };

    btnTop.onclick = () => setSide('top');
    btnRight.onclick = () => setSide('right');
    sync();
  }

  Object.assign(T, { setupDraggableFloatingControls, setupViewSwitcher, setupTabsSideSwitcher });
})();
