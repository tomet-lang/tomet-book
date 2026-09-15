(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== SEARCH ====================
  // Pagefind-backed and always paginated, even before anything is typed (an
  // empty query with no tags still lists every note -- Pagefind treats "no
  // term, no filter" as "everything" -- just 20 at a time). Results reuse
  // the sidebar's book-index-* card styling (see 50-nav-infobox.css) so a
  // hit still reads as a small book.

  /** lookup/manifest.json lists the vault's section names for the tag
   *  chips, without any of the vault's actual notes. Not re-fetched on
   *  every SPA navigation, since setupLookupPane() itself reruns on every
   *  one (the router replaces #pane-nav's innerHTML, so its inputs are
   *  fresh DOM nodes each time and need rebinding regardless). */
  let manifestPromise = null;
  function loadManifest() {
    if (!manifestPromise) {
      manifestPromise = fetch('/lookup/manifest.json')
        .then((res) => (res.ok ? res.json() : { sections: [] }))
        .catch(() => ({ sections: [] }));
    }
    return manifestPromise;
  }

  /** Runs `onFirstShow` once, the first time `panelEl`'s `hidden` attribute
   *  goes false (immediately, if it already isn't hidden). A rail-tab panel
   *  starts hidden -- every panel but the default "toc" one does -- so a
   *  setup function that fetched data or loaded a runtime dependency
   *  (Pagefind's index, a lookup manifest) unconditionally at setup time
   *  was paying for that on every single page view, not just the ones
   *  where the reader actually opened this tab. Toggling `hidden` fires no
   *  scroll/resize/visibility event of its own, so this has to watch the
   *  attribute directly rather than hook into whatever rail-tab code flips it. */
  function onPanelVisible(panelEl, onFirstShow) {
    if (!panelEl) {
      onFirstShow();
      return;
    }
    let started = false;
    const showOnce = () => {
      if (started) return;
      started = true;
      onFirstShow();
    };
    if (!panelEl.hidden) showOnce();
    new MutationObserver(() => {
      if (!panelEl.hidden) showOnce();
    }).observe(panelEl, { attributes: true, attributeFilter: ['hidden'] });
  }

  // Search results come from Pagefind's hydrated result data.
  function searchEntryHtml(data, hereUrl) {
    const isHere = hereUrl && data.url === hereUrl;
    return `
      <li class="book-index-item">
        <a href="${data.url}" class="book-index-link${isHere ? ' is-here' : ''}"${isHere ? ' aria-current="page"' : ''}>
          <span class="book-index-title">${data.meta?.title || data.url}</span>
          ${data.filters?.section ? `<span class="book-index-meta">${data.filters.section}</span>` : ''}
          ${data.meta?.aliases ? `<span class="search-result-aliases"><span class="alias-label">${t('search.aliases', '別名:')}</span> ${data.meta.aliases}</span>` : ''}
          ${data.excerpt ? `<span class="search-result-excerpt">${data.excerpt}</span>` : ''}
        </a>
      </li>
    `;
  }

  const SEARCH_PAGE_SIZE = 20;

  function setupLookupPane() {
    const input = document.getElementById('lookup-filter-input');
    const tagsBox = document.getElementById('lookup-tags');
    const results = document.getElementById('lookup-results');
    if (!input || !tagsBox || !results) return;
    if (input.dataset.lookupInitialized === 'true') return;
    input.dataset.lookupInitialized = 'true';

    const hereUrl = window.tmtPageUrl || null;
    const panelEl = document.getElementById('pane-panel-lookup');
    const activeTags = new Set();
    let searchToken = 0;
    let searchPage = 0;
    let searchResultRefs = null;

    async function renderSearchPage(query) {
      const myToken = ++searchToken;
      const pf = await T.getPagefind?.();
      if (myToken !== searchToken) return; // a newer keystroke/tag click superseded this one
      if (!pf) {
        results.innerHTML = `<div class="pane-placeholder-box"><span class="placeholder-text">${t('search.loading')}</span></div>`;
        return;
      }

      const filters = activeTags.size > 0 ? { section: [...activeTags] } : undefined;
      // Pagefind treats '' as a literal (zero-match) query; null is what
      // actually means "no text term" -- with no filters either, that lists
      // every note, which is exactly the "paginated from the start" state.
      const search = await pf.search(query || null, filters ? { filters } : undefined);
      if (myToken !== searchToken) return;
      searchResultRefs = search?.results || [];

      if (searchResultRefs.length === 0) {
        results.innerHTML = `<div class="pane-placeholder-box"><span class="placeholder-text">${t('search.empty')}</span></div>`;
        return;
      }

      const totalPages = Math.ceil(searchResultRefs.length / SEARCH_PAGE_SIZE);
      searchPage = Math.min(searchPage, totalPages - 1);
      const pageRefs = searchResultRefs.slice(
        searchPage * SEARCH_PAGE_SIZE,
        (searchPage + 1) * SEARCH_PAGE_SIZE
      );
      const pageData = await Promise.all(pageRefs.map((r) => r.data()));
      if (myToken !== searchToken) return;

      const list = `<ul class="book-index-items">${pageData.map((d) => searchEntryHtml(d, hereUrl)).join('')}</ul>`;
      const pagination =
        totalPages > 1
          ? `
        <div class="lookup-pagination">
          <button type="button" class="lookup-page-btn" data-dir="-1" ${searchPage === 0 ? 'disabled' : ''}>${t('search.prev', '◀ 前へ')}</button>
          <span class="lookup-page-status">${searchPage + 1} / ${totalPages}</span>
          <button type="button" class="lookup-page-btn" data-dir="1" ${searchPage === totalPages - 1 ? 'disabled' : ''}>${t('search.next', '次へ ▶')}</button>
        </div>
      `
          : '';
      results.innerHTML = list + pagination;

      results.querySelectorAll('.lookup-page-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
          searchPage += Number(btn.dataset.dir);
          renderSearchPage(query);
        });
      });
    }

    function render() {
      searchPage = 0;
      renderSearchPage(input.value.trim());
    }

    let inputDebounceTimer = null;
    input.addEventListener('input', () => {
      clearTimeout(inputDebounceTimer);
      inputDebounceTimer = setTimeout(render, 150);
    });

    // The manifest fetch, the tag chips it fills in, and the first Pagefind
    // search (renderSearchPage -> T.getPagefind(), which loads and inits
    // Pagefind's whole index) only happen once the reader actually opens
    // this tab -- not on every page view regardless of whether they ever do.
    onPanelVisible(panelEl, () => {
      loadManifest().then((manifest) => {
        const sections = manifest.sections || [];
        if (sections.length > 0) {
          tagsBox.hidden = false;
          tagsBox.innerHTML = sections
            .map((s) => `<button type="button" class="lookup-tag-chip" data-tag="${s.name}">${s.name}</button>`)
            .join('');
          tagsBox.addEventListener('click', (e) => {
            const chip = e.target.closest('.lookup-tag-chip');
            if (!chip) return;
            const tag = chip.dataset.tag;
            if (activeTags.has(tag)) {
              activeTags.delete(tag);
              chip.classList.remove('is-active');
            } else {
              activeTags.add(tag);
              chip.classList.add('is-active');
            }
            render();
          });
        }
        render();
      });
    });
  }

  Object.assign(T, { setupLookupPane });
})();
