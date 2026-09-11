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
    // 掴んだ時点の寸法を控える。ドラッグ中に offsetWidth を読むと、直前に書いた
    // left/top のせいでその場でレイアウトを計算させられ (強制同期レイアウト)、
    // それがマウスに数フレーム遅れて付いてくる原因になる。
    let dragWidth = 0;
    let dragHeight = 0;
    // pointermove は毎フレームより細かく飛んでくるので、描画は 1 フレーム 1 回に畳む。
    let dragFrame = null;
    let offsetLeft = 0;
    let offsetTop = 0;

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

      // innerWidth/innerHeight と控えた寸法だけで済ませる。要素からは何も読まない。
      const maxX = Math.max(8, window.innerWidth - dragWidth - 8);
      const maxY = Math.max(8, window.innerHeight - dragHeight - 8);
      const left = Math.max(8, Math.min(maxX, startLeft + (e.clientX - startX)));
      const top = Math.max(8, Math.min(maxY, startTop + (e.clientY - startY)));
      offsetLeft = left - startLeft;
      offsetTop = top - startTop;

      if (dragFrame !== null) return;
      dragFrame = requestAnimationFrame(() => {
        dragFrame = null;
        // transform はレイアウトを起こさずコンポジタだけで動く。left/top を
        // 毎フレーム書き換えるとそのたびにレイアウトが走る。
        floating.style.transform = `translate3d(${offsetLeft}px, ${offsetTop}px, 0)`;
      });
    };

    const onPointerUp = (e) => {
      if (!isDragging) return;
      isDragging = false;
      floating.classList.remove('is-dragging');
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch (err) {}

      if (dragFrame !== null) {
        cancelAnimationFrame(dragFrame);
        dragFrame = null;
      }

      // 掴んでいる間の transform を、本来の left/top に畳んで確定する。
      const finalLeft = Math.round(startLeft + offsetLeft);
      const finalTop = Math.round(startTop + offsetTop);
      floating.style.transform = '';
      floating.style.left = `${finalLeft}px`;
      floating.style.top = `${finalTop}px`;

      try {
        localStorage.setItem(
          'wiki-floating-controls-pos',
          JSON.stringify({ x: finalLeft, y: finalTop })
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

  // ==================== STICKY TAB SIDE (上 / 右) ====================
  // base.html が描画前に data-tabs を立てているので、ここは押されたときに
  // 付け替えて覚えるだけ。ちらつきはテーマ切り替えと同じ仕組みで防いでいる。
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
      try {
        localStorage.setItem('wiki-tabs-side', side);
      } catch {}
      sync();
    };

    btnTop.onclick = () => setSide('top');
    btnRight.onclick = () => setSide('right');
    sync();
  }

  Object.assign(T, { setupDraggableFloatingControls, setupViewSwitcher, setupTabsSideSwitcher });
})();
