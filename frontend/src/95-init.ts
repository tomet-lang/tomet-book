(() => {
  const T = (window.TMT ??= {} as any);
  const syncViewMode = (...a) => T.syncViewMode(...a);
  const setupThemeToggle = (...a) => T.setupThemeToggle(...a);
  const setupDraggableFloatingControls = (...a) => T.setupDraggableFloatingControls(...a);
  const setupViewSwitcher = (...a) => T.setupViewSwitcher(...a);
  const setupTabsSideSwitcher = (...a) => T.setupTabsSideSwitcher(...a);
  const setupFloatingCollapse = (...a) => T.setupFloatingCollapse?.(...a);
  const setupBookViewInteraction = (...a) => T.setupBookViewInteraction(...a);
  const setupClassicViewInteraction = (...a) => T.setupClassicViewInteraction(...a);
  const initKeyboardNavigation = (...a) => T.initKeyboardNavigation(...a);
  const setupSearch = (...a) => T.setupSearch(...a);
  const initPagePreview = (...a) => T.initPagePreview(...a);
  const initCenterPeek = (...a) => T.initCenterPeek(...a);
  const initImageLightbox = (...a) => T.initImageLightbox(...a);
  const setupCostumeSwitchers = (...a) => T.setupCostumeSwitchers(...a);
  const setupEditDropdown = (...a) => T.setupEditDropdown(...a);
  const setupSourceViewer = (...a) => T.setupSourceViewer(...a);
  const setupClientRouter = (...a) => T.setupClientRouter?.(...a);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== INITIALIZE ====================
  function initPage() {
    syncViewMode();
    setupThemeToggle();
    setupDraggableFloatingControls();
    setupFloatingCollapse();
    setupViewSwitcher();
    setupTabsSideSwitcher();
    setupBookViewInteraction();
    setupClassicViewInteraction();
    initKeyboardNavigation();
    setupSearch();
    initPagePreview();
    initCenterPeek();
    initImageLightbox();
    setupCostumeSwitchers();
    setupEditDropdown();
    setupSourceViewer();
    setupClientRouter();
  }

  Object.assign(T, { initPage });
})();
