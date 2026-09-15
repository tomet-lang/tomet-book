(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== SEARCH (PAGEFIND) ====================
  let pagefindInstance = null;
  let searchDebounceTimer = null;

  async function getPagefind() {
    if (!pagefindInstance) {
      try {
        const pagefindUrl = '/pagefind/pagefind.js';
        const pf = await import(/* @vite-ignore */ pagefindUrl);
        await pf.init();
        pagefindInstance = pf;
      } catch (e) {
        console.warn('Pagefind index not found or error loading:', e);
      }
    }
    return pagefindInstance;
  }

  function setupSearch() {
    const input = document.getElementById('wiki-search-input');
    const resultsContainer = document.getElementById('wiki-search-results');
    if (!input || !resultsContainer) return;
    if (input.dataset.searchInitialized === 'true') return;
    input.dataset.searchInitialized = 'true';

    function showResults() {
      resultsContainer.hidden = false;
      adjustPosition();
    }

    function adjustPosition() {
      if (window.innerWidth <= 768) {
        resultsContainer.style.left = '';
        return;
      }
      resultsContainer.style.left = '0';
      const rect = resultsContainer.getBoundingClientRect();
      const maxRight = window.innerWidth - 10;
      if (rect.right > maxRight) {
        const overflow = rect.right - maxRight;
        resultsContainer.style.left = `-${overflow}px`;
      }
    }

    window.addEventListener('resize', () => {
      if (!resultsContainer.hidden) adjustPosition();
    });

    input.addEventListener('focus', () => {
      getPagefind();
      if (input.value.trim().length > 0) {
        showResults();
      }
    });

    let selectedIndex = -1;

    input.addEventListener('input', () => {
      clearTimeout(searchDebounceTimer);
      const query = input.value.trim();
      if (!query) {
        resultsContainer.innerHTML = '';
        resultsContainer.hidden = true;
        selectedIndex = -1;
        return;
      }

      searchDebounceTimer = setTimeout(async () => {
        const pf = await getPagefind();
        if (!pf) {
          resultsContainer.innerHTML = `<div class="search-status">${t('search.loading')}</div>`;
          showResults();
          return;
        }

        const search = await pf.search(query);
        if (!search || search.results.length === 0) {
          resultsContainer.innerHTML = `<div class="search-status">${t('search.empty')}</div>`;
          showResults();
          selectedIndex = -1;
          return;
        }

        const topResults = await Promise.all(search.results.slice(0, 10).map((r) => r.data()));
        selectedIndex = -1;

        resultsContainer.innerHTML = topResults
          .map(
            (res, idx) => `
          <a href="${res.url}" class="search-result-item" data-index="${idx}">
            <div class="search-result-row">
              ${res.meta?.image ? `<img src="${res.meta.image}" class="search-result-thumb" alt="" loading="lazy" />` : ''}
              <div class="search-result-main">
                <div class="search-result-title">
                  <span>${res.meta?.title || 'No title'}</span>
                  ${res.filters?.kind ? `<span class="search-result-kind">${res.filters.kind}</span>` : ''}
                  ${res.filters?.section ? `<span class="search-result-section">${res.filters.section}</span>` : ''}
                </div>
                <div class="search-result-excerpt">${res.excerpt || ''}</div>
              </div>
            </div>
          </a>
        `
          )
          .join('');
        showResults();
      }, 120);
    });

    input.addEventListener('keydown', (e) => {
      const items = resultsContainer.querySelectorAll('.search-result-item');
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (items.length > 0) {
          selectedIndex = (selectedIndex + 1) % items.length;
          updateSelected(items);
        }
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (items.length > 0) {
          selectedIndex = (selectedIndex - 1 + items.length) % items.length;
          updateSelected(items);
        }
      } else if (e.key === 'Enter') {
        if (selectedIndex >= 0 && items[selectedIndex]) {
          e.preventDefault();
          items[selectedIndex].click();
        } else if (items.length > 0) {
          e.preventDefault();
          items[0].click();
        }
      } else if (e.key === 'Escape') {
        resultsContainer.hidden = true;
        input.blur();
      }
    });

    function updateSelected(items) {
      items.forEach((item, idx) => {
        if (idx === selectedIndex) {
          item.classList.add('selected');
          item.scrollIntoView({ block: 'nearest' });
        } else {
          item.classList.remove('selected');
        }
      });
    }

    document.addEventListener('click', (e) => {
      if (!input.contains(e.target) && !resultsContainer.contains(e.target)) {
        resultsContainer.hidden = true;
      }
    });
  }

  // Global shortcut (Ctrl+K or /)
  window.addEventListener('keydown', (e) => {
    const target = e.target;
    const isEditing = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      const input = document.getElementById('wiki-search-input');
      if (input) {
        input.focus();
        input.select();
      }
    } else if (e.key === '/' && !isEditing) {
      e.preventDefault();
      const input = document.getElementById('wiki-search-input');
      if (input) {
        input.focus();
        input.select();
      }
    }
  });

  Object.assign(T, { setupSearch, getPagefind });
})();
