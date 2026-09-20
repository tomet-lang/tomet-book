(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

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
  function buildBookIndexItem(data, hereUrl) {
    const itemTemplate = document.getElementById('book-index-item-template') as HTMLTemplateElement | null;
    if (!itemTemplate) return null;
    const li = itemTemplate.content.firstElementChild?.cloneNode(true) as HTMLElement;
    const link = li.querySelector<HTMLAnchorElement>('.book-index-link');
    if (!link) return li;

    const isHere = Boolean(hereUrl && data.url === hereUrl);
    const isVisited = T.isVisited ? T.isVisited(data.url) : false;
    link.href = data.url;
    link.classList.toggle('is-here', isHere);
    link.classList.toggle('is-visited', Boolean(isVisited));
    if (isHere) link.setAttribute('aria-current', 'page');

    const bg = li.querySelector<HTMLElement>('.book-index-bg');
    const bgImg = bg?.querySelector('img');
    if (data.meta?.image && bg && bgImg) {
      bgImg.src = data.meta.image;
      bgImg.addEventListener('error', () => bg.remove(), { once: true });
      bg.hidden = false;
    } else {
      bg?.remove();
    }

    const title = li.querySelector('.book-index-title');
    if (title) title.textContent = data.meta?.title || data.url;

    const body = li.querySelector<HTMLElement>('.book-index-body');
    const hasBody = Boolean(data.filters?.section || data.meta?.aliases || data.excerpt);
    if (!hasBody || !body) {
      body?.remove();
      return li;
    }
    body.hidden = false;

    const meta = body.querySelector<HTMLElement>('.book-index-meta');
    if (data.filters?.section && meta) {
      meta.textContent = data.filters.section;
      meta.hidden = false;
    } else {
      meta?.remove();
    }

    const aliases = body.querySelector<HTMLElement>('.search-result-aliases');
    if (data.meta?.aliases && aliases) {
      const label = aliases.querySelector('.alias-label');
      const text = aliases.querySelector('.alias-text');
      if (label) label.textContent = t('search.aliases', '別名:');
      if (text) text.textContent = data.meta.aliases;
      aliases.hidden = false;
    } else {
      aliases?.remove();
    }

    const excerpt = body.querySelector<HTMLElement>('.search-result-excerpt');
    if (data.excerpt && excerpt) {
      // Pagefind's excerpt carries its own <mark> highlighting, so it has
      // to stay real markup here, not escaped text.
      excerpt.innerHTML = data.excerpt;
      excerpt.hidden = false;
    } else {
      excerpt?.remove();
    }

    return li;
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
    const paneScroll = panelEl?.closest('.pane-scroll') || document.querySelector('#pane-nav .pane-scroll');

    // Restore state from sessionStorage if available
    let savedState = null;
    try {
      savedState = JSON.parse(T.session?.get('wiki-lookup-state') || 'null');
    } catch {}

    const activeTags = new Set(Array.isArray(savedState?.tags) ? savedState.tags : []);
    let searchToken = 0;
    let searchPage = typeof savedState?.page === 'number' ? savedState.page : 0;
    let pendingScrollTop = typeof savedState?.scrollTop === 'number' ? savedState.scrollTop : null;

    if (savedState?.query) {
      input.value = savedState.query;
    }

    function saveLookupState() {
      if (!T.session) return;
      const q = input.value.trim();
      const tags = [...activeTags];
      const sTop = paneScroll ? paneScroll.scrollTop : 0;
      const state = {
        query: q,
        tags,
        page: searchPage,
        scrollTop: sTop,
      };
      T.session.set('wiki-lookup-state', JSON.stringify(state));
    }

    // Capture scroll position before navigating or during scroll
    if (paneScroll) {
      const onScrollThrottled = T.rafThrottle
        ? T.rafThrottle(() => {
            if (panelEl && !panelEl.hidden) {
              saveLookupState();
            }
          })
        : () => {
            if (panelEl && !panelEl.hidden) saveLookupState();
          };
      paneScroll.addEventListener('scroll', onScrollThrottled, { passive: true });
    }

    // Save on result link click
    results.addEventListener('click', (e) => {
      if (e.target.closest('a')) {
        saveLookupState();
      }
    });

    function showPlaceholder(message) {
      const box = document.createElement('div');
      box.className = 'pane-placeholder-box';
      const span = document.createElement('span');
      span.className = 'placeholder-text';
      span.textContent = message;
      box.appendChild(span);
      results.replaceChildren(box);
    }

    async function renderSearchPage(query) {
      const myToken = ++searchToken;
      const nativeIndex = await T.getNativeIndex?.();
      if (myToken !== searchToken) return; // a newer keystroke/tag click superseded this one

      let pageData;
      let totalCount;

      if (nativeIndex) {
        const sections = activeTags.size > 0 ? activeTags : null;
        const allResults = T.nativeSearchDocuments(query || '', nativeIndex, { sections });
        totalCount = allResults.length;
        if (totalCount === 0) {
          showPlaceholder(t('search.empty'));
          return;
        }

        const totalPages = Math.ceil(totalCount / SEARCH_PAGE_SIZE);
        searchPage = Math.min(Math.max(0, searchPage), Math.max(0, totalPages - 1));
        pageData = allResults.slice(searchPage * SEARCH_PAGE_SIZE, (searchPage + 1) * SEARCH_PAGE_SIZE);
      } else {
        // Native index unavailable (e.g. `search = "pagefind"` or "none" in
        // tmtbook.toml) -- fall back to Pagefind, as this pane always did.
        const pf = await T.getPagefind?.();
        if (myToken !== searchToken) return;
        if (!pf) {
          showPlaceholder(t('search.loading'));
          return;
        }

        const filters = activeTags.size > 0 ? { section: [...activeTags] } : undefined;
        // Pagefind treats '' as a literal (zero-match) query; null is what
        // actually means "no text term" -- with no filters either, that lists
        // every note, which is exactly the "paginated from the start" state.
        const search = await pf.search(query || null, filters ? { filters } : undefined);
        if (myToken !== searchToken) return;
        const searchResultRefs = search?.results || [];
        totalCount = searchResultRefs.length;

        if (totalCount === 0) {
          showPlaceholder(t('search.empty'));
          return;
        }

        const totalPages = Math.ceil(totalCount / SEARCH_PAGE_SIZE);
        searchPage = Math.min(Math.max(0, searchPage), Math.max(0, totalPages - 1));
        const pageRefs = searchResultRefs.slice(
          searchPage * SEARCH_PAGE_SIZE,
          (searchPage + 1) * SEARCH_PAGE_SIZE
        );
        pageData = await Promise.all(pageRefs.map((r) => r.data()));
        if (myToken !== searchToken) return;
      }

      const totalPages = Math.ceil(totalCount / SEARCH_PAGE_SIZE);
      const list = document.createElement('ul');
      list.className = 'book-index-items';
      pageData.forEach((d) => {
        const item = buildBookIndexItem(d, hereUrl);
        if (item) list.appendChild(item);
      });

      let pagination: HTMLDivElement | null = null;
      if (totalPages > 1) {
        pagination = document.createElement('div');
        pagination.className = 'lookup-pagination';

        const prevBtn = document.createElement('button');
        prevBtn.type = 'button';
        prevBtn.className = 'lookup-page-btn';
        prevBtn.dataset.dir = '-1';
        prevBtn.disabled = searchPage === 0;
        prevBtn.textContent = t('search.prev', '◀ 前へ');

        const status = document.createElement('span');
        status.className = 'lookup-page-status';
        status.textContent = `${searchPage + 1} / ${totalPages}`;

        const nextBtn = document.createElement('button');
        nextBtn.type = 'button';
        nextBtn.className = 'lookup-page-btn';
        nextBtn.dataset.dir = '1';
        nextBtn.disabled = searchPage === totalPages - 1;
        nextBtn.textContent = t('search.next', '次へ ▶');

        pagination.append(prevBtn, status, nextBtn);
      }

      results.replaceChildren(list, ...(pagination ? [pagination] : []));

      results.querySelectorAll<HTMLButtonElement>('.lookup-page-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
          searchPage += Number(btn.dataset.dir);
          saveLookupState();
          renderSearchPage(query);
        });
      });

      // Restore scroll position once results are rendered into the DOM
      if (pendingScrollTop !== null && paneScroll) {
        const top = pendingScrollTop;
        pendingScrollTop = null;
        requestAnimationFrame(() => {
          paneScroll.scrollTop = top;
        });
      }
    }

    function render(resetPage = true) {
      if (resetPage) {
        searchPage = 0;
      }
      saveLookupState();
      renderSearchPage(input.value.trim());
    }

    let inputDebounceTimer = null;
    input.addEventListener('input', () => {
      clearTimeout(inputDebounceTimer);
      inputDebounceTimer = setTimeout(() => render(true), 150);
    });

    input.addEventListener('search', () => {
      if (!input.value) {
        clearTimeout(inputDebounceTimer);
        render(true);
      }
    });

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && input.value) {
        e.preventDefault();
        input.value = '';
        render(true);
      }
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
          const chips = sections.map((s) => {
            const chip = document.createElement('button');
            chip.type = 'button';
            chip.className = activeTags.has(s.name) ? 'lookup-tag-chip is-active' : 'lookup-tag-chip';
            chip.dataset.tag = s.name;
            chip.textContent = s.name;
            return chip;
          });
          tagsBox.replaceChildren(...chips);
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
            render(true);
          });
        }
        render(false);
      });
    });
  }

  Object.assign(T, { setupLookupPane });
})();
