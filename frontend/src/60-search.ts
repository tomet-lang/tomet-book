(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== SEARCH (PAGEFIND) ====================
  let pagefindInstance = null;
  let searchDebounceTimer = null;

  async function getPagefind() {
    if (!pagefindInstance) {
      try {
        const pagefindUrl = '/pagefind/pagefind.js';
        const pf = await import(/* @vite-ignore */ pagefindUrl);
        await pf.options({
          ranking: {
            pageLength: 0.1,
          },
        });
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

    function showStatus(message) {
      const status = document.createElement('div');
      status.className = 'search-status';
      status.textContent = message;
      resultsContainer.replaceChildren(status);
    }

    function normalizeStrict(s: string): string {
      return (s || '').toLowerCase().trim();
    }

    function normalizeFuzzy(s: string): string {
      return (s || '')
        .toLowerCase()
        .replace(/[@#\-_/\\.()[\]{}:;,+*~]/g, ' ')
        .replace(/\s+/g, ' ')
        .trim();
    }

    interface SearchCandidate {
      url: string;
      rawScore: number;
      excerpt?: string;
      meta?: {
        title?: string;
        path?: string;
        aliases?: string;
        image?: string;
        [key: string]: any;
      };
      filters?: {
        kind?: string;
        section?: string;
        [key: string]: any;
      };
      [key: string]: any;
    }

    function calculateSearchBoost(query: string, item: SearchCandidate): number {
      const qStrict = normalizeStrict(query);
      const qFuzzy = normalizeFuzzy(query);
      if (!qStrict) return 0;

      const title = item.meta?.title || '';
      const path = item.meta?.path || '';
      const filename = path ? path.split('/').pop()?.replace(/\.tmt$/, '') || '' : '';
      const urlStem = (item.url || '').replace(/\/+$/, '').split('/').pop() || '';

      const rawAliases = item.meta?.aliases
        ? item.meta.aliases
            .split(/[,、]/)
            .map((s) => s.trim())
            .filter(Boolean)
        : [];

      const targets = [
        { text: title, weight: 1.0 },
        { text: filename, weight: 1.0 },
        { text: urlStem, weight: 0.9 },
        { text: path, weight: 0.8 },
        ...rawAliases.map((a) => ({ text: a, weight: 0.98 })),
      ];

      let maxBoost = 0;

      for (const { text, weight } of targets) {
        if (!text) continue;
        const tStrict = normalizeStrict(text);
        const tFuzzy = normalizeFuzzy(text);

        // Tier 1: Exact match (either strict or fuzzy)
        // e.g. "@Raora-Panthera" === "@Raora-Panthera" or "Raora Panthera" === "Raora Panthera"
        if (tStrict === qStrict || (qFuzzy && tFuzzy === qFuzzy)) {
          maxBoost = Math.max(maxBoost, 100000 * weight);
          continue;
        }

        // Tier 2: Prefix match (text starts with query)
        // e.g. "Raora Panthera" starts with "Raora"
        if (tStrict.startsWith(qStrict) || (qFuzzy && tFuzzy.startsWith(qFuzzy))) {
          maxBoost = Math.max(maxBoost, 50000 * weight);
          continue;
        }

        // Tier 3: Word-boundary match
        if (qFuzzy) {
          const escaped = qFuzzy.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&');
          const wordPattern = new RegExp(`(^|\\s)${escaped}($|\\s)`);
          if (wordPattern.test(tFuzzy)) {
            maxBoost = Math.max(maxBoost, 20000 * weight);
            continue;
          }
        }

        // Tier 4: Substring match anywhere in title/filename/alias
        if (tStrict.includes(qStrict) || (qFuzzy && tFuzzy.includes(qFuzzy))) {
          maxBoost = Math.max(maxBoost, 10000 * weight);
          continue;
        }
      }

      return maxBoost;
    }

    function renderResults(results: SearchCandidate[]) {
      const itemTemplate = document.getElementById('search-result-item-template') as HTMLTemplateElement | null;
      if (!itemTemplate) return;

      const items = results.map((res, idx) => {
        const item = itemTemplate.content.firstElementChild?.cloneNode(true) as HTMLAnchorElement;
        item.href = res.url;
        item.dataset.index = String(idx);

        const thumb = item.querySelector<HTMLImageElement>('.search-result-thumb');
        if (res.meta?.image && thumb) {
          thumb.src = res.meta.image;
          thumb.hidden = false;
          thumb.addEventListener('error', () => thumb.remove(), { once: true });
        } else {
          thumb?.remove();
        }

        const titleText = item.querySelector('.result-title-text');
        if (titleText) titleText.textContent = res.meta?.title || 'No title';

        const kind = item.querySelector<HTMLElement>('.search-result-kind');
        if (res.filters?.kind && kind) {
          kind.textContent = res.filters.kind;
          kind.hidden = false;
        } else {
          kind?.remove();
        }

        const section = item.querySelector<HTMLElement>('.search-result-section');
        if (res.filters?.section && section) {
          section.textContent = res.filters.section;
          section.hidden = false;
        } else {
          section?.remove();
        }

        const pathEl = item.querySelector<HTMLElement>('.search-result-path');
        const displayPath =
          res.meta?.path || (res.url ? res.url.replace(/^\/wiki\//, '').replace(/\/+$/, '') : '');
        if (displayPath && pathEl) {
          pathEl.textContent = displayPath;
          pathEl.hidden = false;
        } else {
          pathEl?.remove();
        }

        const aliases = item.querySelector<HTMLElement>('.search-result-aliases');
        if (res.meta?.aliases && aliases) {
          const label = aliases.querySelector('.alias-label');
          const text = aliases.querySelector('.alias-text');
          if (label) label.textContent = t('search.aliases', '別名:');
          if (text) text.textContent = res.meta.aliases;
          aliases.hidden = false;
        } else {
          aliases?.remove();
        }

        const excerpt = item.querySelector<HTMLElement>('.search-result-excerpt');
        if (res.excerpt && excerpt) {
          // If the excerpt is essentially just repeating the path, hide it to keep results clean
          const cleanExcerptText = res.excerpt.replace(/<[^>]+>/g, '').trim();
          const cleanPath = (displayPath || '').trim();
          if (
            cleanExcerptText &&
            cleanPath &&
            (cleanExcerptText === cleanPath || cleanExcerptText.startsWith(cleanPath))
          ) {
            excerpt.remove();
          } else {
            // Pagefind's excerpt carries its own <mark> highlighting, so it
            // has to stay real markup here, not escaped text.
            excerpt.innerHTML = res.excerpt;
            excerpt.hidden = false;
          }
        } else {
          excerpt?.remove();
        }

        return item;
      });

      resultsContainer.replaceChildren(...items);
    }

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
        resultsContainer.replaceChildren();
        resultsContainer.hidden = true;
        selectedIndex = -1;
        return;
      }

      searchDebounceTimer = setTimeout(async () => {
        const pf = await getPagefind();
        if (!pf) {
          showStatus(t('search.loading'));
          showResults();
          return;
        }

        const search = await pf.search(query);
        if (!search || search.results.length === 0) {
          showStatus(t('search.empty'));
          showResults();
          selectedIndex = -1;
          return;
        }

        // Fetch up to 30 candidates to re-rank with exact / prefix / title / path boosts
        const candidateCount = Math.min(search.results.length, 30);
        const candidates: SearchCandidate[] = await Promise.all(
          search.results.slice(0, candidateCount).map(async (r: any) => {
            const data = await r.data();
            return {
              ...data,
              rawScore: r.score,
            };
          })
        );

        candidates.sort((a, b) => {
          const boostA = calculateSearchBoost(query, a);
          const boostB = calculateSearchBoost(query, b);
          if (boostA !== boostB) {
            return boostB - boostA;
          }
          return b.rawScore - a.rawScore;
        });

        const topResults = candidates.slice(0, 10);
        selectedIndex = -1;

        renderResults(topResults);
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

    resultsContainer.addEventListener('click', (e) => {
      if (e.target.closest('a')) {
        resultsContainer.hidden = true;
        input.blur();
      }
    });

    document.addEventListener('click', (e) => {
      const targetNode = e.target as Node | null;
      if (!input.contains(targetNode) && !resultsContainer.contains(targetNode)) {
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
