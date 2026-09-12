(() => {
  const T = (window.TMT ??= {});

  // ==================== PAGE LOAD PROGRESS BAR ====================
  // Bridges the one gap the SPA router (80-router.js) gives no feedback for:
  // click to fetch-response. The View Transition it uses for the actual
  // content swap only starts once that response is already in hand, so on
  // a slow network a click looked like it did nothing until the new page
  // just appeared.

  function startPageProgress() {
    const bar = document.getElementById('page-progress-bar');
    if (!bar) return;
    bar.classList.remove('is-done');
    // A navigation started while the previous one's bar was still mid-fade
    // needs the width transition to actually restart, not get coalesced
    // into a no-op because the class never left in between.
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
    }, 500);
  }

  Object.assign(T, { startPageProgress, finishPageProgress });
})();
