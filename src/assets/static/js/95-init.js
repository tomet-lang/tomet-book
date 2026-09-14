(() => {
  const T = (window.TMT ??= {});
  const syncViewMode = (...a) => T.syncViewMode(...a);
  const setupThemeToggle = (...a) => T.setupThemeToggle(...a);
  const setupDraggableFloatingControls = (...a) => T.setupDraggableFloatingControls(...a);
  const setupViewSwitcher = (...a) => T.setupViewSwitcher(...a);
  const setupTabsSideSwitcher = (...a) => T.setupTabsSideSwitcher(...a);
  const setupBookViewInteraction = (...a) => T.setupBookViewInteraction(...a);
  const setupClassicViewInteraction = (...a) => T.setupClassicViewInteraction(...a);
  const initKeyboardNavigation = (...a) => T.initKeyboardNavigation(...a);
  const setupSearch = (...a) => T.setupSearch(...a);
  const initPagePreview = (...a) => T.initPagePreview(...a);
  const initCenterPeek = (...a) => T.initCenterPeek(...a);
  const initImageLightbox = (...a) => T.initImageLightbox(...a);
  const setupCostumeSwitchers = (...a) => T.setupCostumeSwitchers(...a);
  const setupEditDropdown = (...a) => T.setupEditDropdown(...a);
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== INITIALIZE ====================
  function initPage() {
    syncViewMode();
    setupThemeToggle();
    setupDraggableFloatingControls();
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
  }

  Object.assign(T, { initPage });
})();
