(() => {
  const T = (window.TMT ??= {} as any);
  const initPage = (...a) => T.initPage(...a);
  const setupDevServerLiveReload = (...a) => T.setupDevServerLiveReload(...a);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      initPage();
      setupDevServerLiveReload();
    });
  } else {
    initPage();
    setupDevServerLiveReload();
  }
})();
