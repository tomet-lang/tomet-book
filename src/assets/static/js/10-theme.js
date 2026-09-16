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

  // ==================== SHELL CUSTOMIZATION CONTROLLER ====================
  const SHELL_COLORS = ['slate', 'violet', 'indigo', 'olive'];
  const SHELL_PATTERNS = ['marble', 'cloud', 'seigaiha', 'flourish', 'mesh', 'none'];

  function getShellColor() {
    return document.documentElement.getAttribute('data-shell-color') || 'slate';
  }

  function setShellColor(color) {
    if (!SHELL_COLORS.includes(color)) color = 'slate';
    if (color === 'slate') {
      document.documentElement.removeAttribute('data-shell-color');
    } else {
      document.documentElement.setAttribute('data-shell-color', color);
    }
    T.storage.set('tmtbook-shell-color', color);
    syncShellUI();
  }

  function getShellPattern() {
    return document.documentElement.getAttribute('data-shell-pattern') || 'marble';
  }

  function setShellPattern(pattern) {
    if (!SHELL_PATTERNS.includes(pattern)) pattern = 'marble';
    if (pattern === 'marble') {
      document.documentElement.removeAttribute('data-shell-pattern');
    } else {
      document.documentElement.setAttribute('data-shell-pattern', pattern);
    }
    T.storage.set('tmtbook-shell-pattern', pattern);
    syncShellUI();
  }

  function getShellContrast() {
    return document.documentElement.getAttribute('data-shell-contrast') || 'normal';
  }

  function setShellContrast(contrast) {
    if (contrast === 'high') {
      document.documentElement.setAttribute('data-shell-contrast', 'high');
      T.storage.set('tmtbook-shell-contrast', 'high');
    } else {
      document.documentElement.removeAttribute('data-shell-contrast');
      T.storage.set('tmtbook-shell-contrast', 'normal');
    }
    syncShellUI();
  }

  function toggleShellContrast() {
    const current = getShellContrast();
    setShellContrast(current === 'high' ? 'normal' : 'high');
  }

  // Backwards compatibility for legacy shell theme calls
  function getShellTheme() {
    return getShellColor();
  }

  function setShellTheme(theme) {
    if (SHELL_COLORS.includes(theme)) {
      setShellColor(theme);
    } else if (theme === 'contrast') {
      setShellContrast('high');
    }
  }

  function migrateLegacyShell() {
    const legacy = T.storage.get('tmtbook-shell');
    if (legacy) {
      let color = 'slate';
      let pattern = 'marble';
      let contrast = 'normal';
      if (legacy === 'violet') { color = 'violet'; pattern = 'cloud'; }
      else if (legacy === 'indigo') { color = 'indigo'; pattern = 'seigaiha'; }
      else if (legacy === 'olive') { color = 'olive'; pattern = 'flourish'; }
      else if (legacy === 'contrast') { color = 'slate'; pattern = 'mesh'; contrast = 'high'; }

      if (!T.storage.get('tmtbook-shell-color')) setShellColor(color);
      if (!T.storage.get('tmtbook-shell-pattern')) setShellPattern(pattern);
      if (!T.storage.get('tmtbook-shell-contrast') && contrast === 'high') setShellContrast('high');
      T.storage.remove('tmtbook-shell');
    }
  }

  function syncShellUI() {
    const currentColor = getShellColor();
    const currentPattern = getShellPattern();
    const isHighContrast = getShellContrast() === 'high';

    document.querySelectorAll('.color-btn[data-shell-color]').forEach((btn) => {
      const isSelected = btn.dataset.shellColor === currentColor;
      btn.classList.toggle('is-active', isSelected);
      btn.setAttribute('aria-pressed', isSelected ? 'true' : 'false');
    });

    document.querySelectorAll('.pattern-btn[data-shell-pattern]').forEach((btn) => {
      const isSelected = btn.dataset.shellPattern === currentPattern;
      btn.classList.toggle('is-active', isSelected);
      btn.setAttribute('aria-pressed', isSelected ? 'true' : 'false');
    });

    const contrastBtn = document.getElementById('shell-contrast-toggle');
    if (contrastBtn) {
      contrastBtn.setAttribute('aria-checked', isHighContrast ? 'true' : 'false');
      contrastBtn.classList.toggle('is-active', isHighContrast);
    }
  }

  function setupShellTheme() {
    migrateLegacyShell();

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
        syncShellUI();
      } else {
        closePopover();
      }
    });

    popover.addEventListener('click', (e) => {
      e.stopPropagation();
      const colorBtn = e.target.closest('.color-btn[data-shell-color]');
      if (colorBtn) {
        setShellColor(colorBtn.dataset.shellColor);
        return;
      }
      const patternBtn = e.target.closest('.pattern-btn[data-shell-pattern]');
      if (patternBtn) {
        setShellPattern(patternBtn.dataset.shellPattern);
        return;
      }
      const contrastBtn = e.target.closest('#shell-contrast-toggle');
      if (contrastBtn) {
        toggleShellContrast();
        return;
      }
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

    syncShellUI();
  }

  Object.assign(T, {
    setupThemeToggle,
    getEffectiveTheme,
    setupShellTheme,
    getShellColor,
    setShellColor,
    getShellPattern,
    setShellPattern,
    getShellContrast,
    setShellContrast,
    toggleShellContrast,
    getShellTheme,
    setShellTheme,
  });
})();
