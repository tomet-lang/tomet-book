(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== EDIT DROPDOWN & COPY TOAST ====================
  function showToast(message) {
    let toast = document.getElementById('tmt-global-toast');
    if (!toast) {
      toast = document.createElement('div');
      toast.id = 'tmt-global-toast';
      toast.className = 'tmt-toast';
      document.body.appendChild(toast);
    }
    toast.textContent = message;
    toast.classList.add('is-show');
    if (toast._timer) clearTimeout(toast._timer);
    toast._timer = setTimeout(() => {
      toast.classList.remove('is-show');
    }, 2500);
  }

  function setupEditDropdown() {
    const editBtns = document.querySelectorAll('.btn-edit-open');
    if (editBtns.length === 0) return;

    editBtns.forEach((btn) => {
      const dropdown = btn.closest('.edit-dropdown');
      if (!dropdown) return;
      const menu = dropdown.querySelector('.edit-dropdown-menu');
      if (!menu) return;

      btn.onclick = (e) => {
        e.preventDefault();
        e.stopPropagation();
        const isOpen = dropdown.classList.contains('is-open');
        // Close all other open dropdowns
        document.querySelectorAll('.edit-dropdown.is-open').forEach((d) => {
          d.classList.remove('is-open');
          const m = d.querySelector('.edit-dropdown-menu');
          if (m) m.hidden = true;
        });

        if (!isOpen) {
          dropdown.classList.add('is-open');
          menu.hidden = false;
        }
      };
    });

    // Copy path buttons
    document.querySelectorAll('.btn-copy-path').forEach((btn) => {
      btn.onclick = async (e) => {
        e.preventDefault();
        e.stopPropagation();
        const path = btn.getAttribute('data-path');
        if (path) {
          try {
            await navigator.clipboard.writeText(path);
            showToast(t('toast.copied'));
          } catch {
            showToast(t('toast.copy_failed'));
          }
        }
        const dropdown = btn.closest('.edit-dropdown');
        if (dropdown) {
          dropdown.classList.remove('is-open');
          const menu = dropdown.querySelector('.edit-dropdown-menu');
          if (menu) menu.hidden = true;
        }
      };
    });

    // Close on outside click
    if (!window.__editDropdownOutsideClickBound) {
      window.__editDropdownOutsideClickBound = true;
      document.addEventListener('click', (e) => {
        if (!e.target.closest('.edit-dropdown')) {
          document.querySelectorAll('.edit-dropdown.is-open').forEach((d) => {
            d.classList.remove('is-open');
            const menu = d.querySelector('.edit-dropdown-menu');
            if (menu) menu.hidden = true;
          });
        }
      });
    }
  }

  Object.assign(T, { setupEditDropdown });
})();
