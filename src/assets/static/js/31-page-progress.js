(() => {
  const T = (window.TMT ??= {});

  // ==================== PAGE LOAD PROGRESS BAR ====================
  function startPageProgress() {
    const bar = document.getElementById('page-progress-bar');
    if (!bar) return;
    bar.classList.remove('is-done');
    bar.classList.remove('is-loading');
    void bar.offsetWidth;
    bar.classList.add('is-loading');
  }

  function finishPageProgress() {
    const bar = document.getElementById('page-progress-bar');
    if (!bar) return;
    bar.classList.remove('is-loading');
    bar.classList.add('is-done');
    setTimeout(() => {
      bar.classList.remove('is-done');
    }, 450);
  }

  Object.assign(T, { startPageProgress, finishPageProgress });
})();
