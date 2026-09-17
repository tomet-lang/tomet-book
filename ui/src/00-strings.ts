(() => {
  const T = (window.TMT ??= {});

  // ==================== UI STRINGS ====================
  // Injected by base.html from `[ui.strings]`; the fallback keeps this file
  // working if it is ever loaded on a page that did not set them.
  const STRINGS = window.tmtStrings || {};
  const t = (key, fallback = "") => STRINGS[key] ?? fallback;

  Object.assign(T, { t });
})();
