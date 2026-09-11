(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== THEME CONTROLLER ====================
  function getSystemTheme() {
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  }

  function getEffectiveTheme() {
    const forced = document.documentElement.getAttribute('data-theme');
    if (forced === 'dark' || forced === 'light') return forced;
    return getSystemTheme();
  }

  function syncThemeIcons() {
    const isDark = getEffectiveTheme() === 'dark';
    const titleStr = isDark ? t('theme.to_light') : t('theme.to_dark');

    const themeBtns = document.querySelectorAll('.theme-toggle-btn, #btn-rail-theme');
    themeBtns.forEach((btn) => {
      btn.title = titleStr;
    });
  }

  function toggleTheme() {
    const current = getEffectiveTheme();
    const nextTheme = current === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', nextTheme);
    try {
      localStorage.setItem('tmtbook-theme', nextTheme);
    } catch {}
    syncThemeIcons();
  }

  function setupThemeToggle() {
    const themeBtns = document.querySelectorAll('.theme-toggle-btn, #btn-rail-theme');
    themeBtns.forEach((btn) => {
      btn.onclick = (e) => {
        e.preventDefault();
        toggleTheme();
      };
    });
    syncThemeIcons();
  }

  Object.assign(T, { setupThemeToggle });
})();
