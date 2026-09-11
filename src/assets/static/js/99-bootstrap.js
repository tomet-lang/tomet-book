(() => {
  const T = (window.TMT ??= {});
  const initPage = (...a) => T.initPage(...a);
  const setupClientRouter = (...a) => T.setupClientRouter(...a);
  const setupDevServerLiveReload = (...a) => T.setupDevServerLiveReload(...a);
  const t = (key, fallback) => T.t(key, fallback);

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initPage();
      setupClientRouter();
      setupDevServerLiveReload();
    });
  } else {
    initPage();
    setupClientRouter();
    setupDevServerLiveReload();
  }
})();
