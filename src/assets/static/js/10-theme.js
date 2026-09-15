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
    T.storage.set('tmtbook-theme', nextTheme);
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
    setupShellTheme();
  }

  // ==================== SHELL THEME CONTROLLER ====================
  const SHELL_THEMES = ['slate', 'violet', 'indigo', 'olive', 'contrast'];

  function getShellTheme() {
    return document.documentElement.getAttribute('data-shell') || 'slate';
  }

  function setShellTheme(theme) {
    if (!SHELL_THEMES.includes(theme)) theme = 'slate';
    if (theme === 'slate') {
      document.documentElement.removeAttribute('data-shell');
    } else {
      document.documentElement.setAttribute('data-shell', theme);
    }
    T.storage.set('tmtbook-shell', theme);
    syncShellSwatches();
  }

  function syncShellSwatches() {
    const current = getShellTheme();
    document.querySelectorAll('.swatch-btn').forEach((btn) => {
      const isSelected = btn.dataset.shell === current;
      btn.classList.toggle('is-active', isSelected);
      btn.setAttribute('aria-pressed', isSelected ? 'true' : 'false');
    });
  }

  function setupShellTheme() {
    const paletteBtn = document.getElementById('btn-rail-palette');
    const popover = document.getElementById('shell-palette-popover');
    if (!paletteBtn || !popover) return;

    function closePopover() {
      popover.hidden = true;
      paletteBtn.setAttribute('aria-expanded', 'false');
    }

    paletteBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const willOpen = popover.hidden;
      if (willOpen) {
        popover.hidden = false;
        paletteBtn.setAttribute('aria-expanded', 'true');
      } else {
        closePopover();
      }
    });

    popover.querySelectorAll('.swatch-btn').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const targetShell = btn.dataset.shell;
        if (targetShell) {
          setShellTheme(targetShell);
        }
        closePopover();
      });
    });

    document.addEventListener('click', (e) => {
      if (!e.target.closest('.rail-palette-container')) {
        closePopover();
      }
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        closePopover();
      }
    });

    syncShellSwatches();
  }

  Object.assign(T, {
    setupThemeToggle,
    getEffectiveTheme,
    setupShellTheme,
    getShellTheme,
    setShellTheme,
  });
})();
