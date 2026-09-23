(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);

  // ==================== SEARCH (NATIVE + PAGEFIND FALLBACK) ====================
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

  interface NativeSearchDocument {
    id: number;
    slug: string;
    title: string;
    url: string;
    path: string;
    aliases?: string[];
    kind?: string;
    section?: string;
    image?: string;
    headings?: string[];
    text: string;
  }

  interface NativeSearchIndex {
    docs: NativeSearchDocument[];
  }

  let nativeIndexPromise: Promise<NativeSearchIndex | null> | null = null;

  async function getNativeIndex(): Promise<NativeSearchIndex | null> {
    if (!nativeIndexPromise) {
      nativeIndexPromise = fetch('/search-index.json')
        .then((res) => (res.ok ? res.json() : null))
        .catch((e) => {
          console.warn('Native search index not found or error loading:', e);
          return null;
        });
    }
    return nativeIndexPromise;
  }

  // Tokenizes a query into lowercase words. Prefers Intl.Segmenter (handles
  // languages without whitespace, e.g. Japanese) and falls back to splitting
  // on whitespace/punctuation where it's unavailable.
  function tokenizeQuery(query: string): string[] {
    const q = query.trim();
    if (!q) return [];

    if (typeof Intl !== 'undefined' && typeof (Intl as any).Segmenter === 'function') {
      const segmenter = new (Intl as any).Segmenter(undefined, { granularity: 'word' });
      const tokens: string[] = [];
      for (const { segment, isWordLike } of segmenter.segment(q)) {
        if (isWordLike) tokens.push(segment.toLowerCase());
      }
      if (tokens.length > 0) return tokens;
    }

    return q
      .toLowerCase()
      .split(/[\s、,，.。]+/)
      .filter(Boolean);
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
    boost?: number;
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

  interface CompiledQuery {
    qStrict: string;
    qFuzzy: string;
    wordPattern: RegExp | null;
  }

  function compileQuery(query: string): CompiledQuery {
    const qStrict = normalizeStrict(query);
    const qFuzzy = normalizeFuzzy(query);
    let wordPattern: RegExp | null = null;
    if (qFuzzy) {
      const escaped = qFuzzy.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&');
      wordPattern = new RegExp(`(^|\\s)${escaped}($|\\s)`);
    }
    return { qStrict, qFuzzy, wordPattern };
  }

  // Shared by the floating search box and the sidebar's lookup pane, so both
  // surfaces rank native-index results the same way Pagefind's tuned ranking
  // approximated: exact/prefix/word-boundary/substring hits on title, alias,
  // filename, URL stem and path, in that priority order.
  function calculateSearchBoost(query: string, item: SearchCandidate, compiled?: CompiledQuery): number {
      const q = compiled || compileQuery(query);
      if (!q.qStrict) return 0;

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
        if (tStrict === q.qStrict || (q.qFuzzy && tFuzzy === q.qFuzzy)) {
          maxBoost = Math.max(maxBoost, 100000 * weight);
          continue;
        }

        // Tier 2: Prefix match (text starts with query)
        // e.g. "Raora Panthera" starts with "Raora"
        if (tStrict.startsWith(q.qStrict) || (q.qFuzzy && tFuzzy.startsWith(q.qFuzzy))) {
          maxBoost = Math.max(maxBoost, 50000 * weight);
          continue;
        }

        // Tier 3: Word-boundary match
        if (q.wordPattern && q.wordPattern.test(tFuzzy)) {
          maxBoost = Math.max(maxBoost, 20000 * weight);
          continue;
        }

        // Tier 4: Substring match anywhere in title/filename/alias
        if (tStrict.includes(q.qStrict) || (q.qFuzzy && tFuzzy.includes(q.qFuzzy))) {
          maxBoost = Math.max(maxBoost, 10000 * weight);
          continue;
        }
      }

      return maxBoost;
  }

  interface CachedDocTarget {
    tStrict: string;
    tFuzzy: string;
    weight: number;
  }

  interface CachedSearchDocument extends NativeSearchDocument {
    _metaHaystack?: string;
    _textLower?: string;
    _fullHaystack?: string;
    _targets?: CachedDocTarget[];
  }

  function getDocTargets(doc: CachedSearchDocument): CachedDocTarget[] {
    if (doc._targets) return doc._targets;

    const title = doc.title || '';
    const path = doc.path || '';
    const filename = path ? path.split('/').pop()?.replace(/\.tmt$/, '') || '' : '';
    const urlStem = (doc.url || '').replace(/\/+$/, '').split('/').pop() || '';
    const rawAliases = doc.aliases || [];

    const rawTargets: Array<{ text: string; weight: number }> = [
      { text: title, weight: 1.0 },
      { text: filename, weight: 1.0 },
      { text: urlStem, weight: 0.9 },
      { text: path, weight: 0.8 },
      ...rawAliases.map((a) => ({ text: a, weight: 0.98 })),
    ];

    doc._targets = rawTargets
      .filter((t) => t.text)
      .map((t) => ({
        tStrict: normalizeStrict(t.text),
        tFuzzy: normalizeFuzzy(t.text),
        weight: t.weight,
      }));

    return doc._targets;
  }

  function calculateDocBoost(compiled: CompiledQuery, doc: CachedSearchDocument): number {
    if (!compiled.qStrict) return 0;
    const targets = getDocTargets(doc);
    let maxBoost = 0;

    for (let i = 0; i < targets.length; i++) {
      const { tStrict, tFuzzy, weight } = targets[i];

      // Tier 1: Exact match
      if (tStrict === compiled.qStrict || (compiled.qFuzzy && tFuzzy === compiled.qFuzzy)) {
        maxBoost = Math.max(maxBoost, 100000 * weight);
        continue;
      }

      // Tier 2: Prefix match
      if (tStrict.startsWith(compiled.qStrict) || (compiled.qFuzzy && tFuzzy.startsWith(compiled.qFuzzy))) {
        maxBoost = Math.max(maxBoost, 50000 * weight);
        continue;
      }

      // Tier 3: Word-boundary match
      if (compiled.wordPattern && compiled.wordPattern.test(tFuzzy)) {
        maxBoost = Math.max(maxBoost, 20000 * weight);
        continue;
      }

      // Tier 4: Substring match
      if (tStrict.includes(compiled.qStrict) || (compiled.qFuzzy && tFuzzy.includes(compiled.qFuzzy))) {
        maxBoost = Math.max(maxBoost, 10000 * weight);
        continue;
      }
    }

    return maxBoost;
  }

  function getDocHaystack(doc: CachedSearchDocument, includeFullText: boolean): string {
    if (doc._metaHaystack === undefined) {
      doc._metaHaystack = [doc.title, ...(doc.aliases || []), ...(doc.headings || []), doc.path]
        .join(' ')
        .toLowerCase();
    }
    if (!includeFullText) {
      return doc._metaHaystack;
    }
    if (doc._fullHaystack === undefined) {
      if (doc._textLower === undefined) {
        doc._textLower = (doc.text || '').toLowerCase();
      }
      doc._fullHaystack = doc._metaHaystack + ' ' + doc._textLower;
    }
    return doc._fullHaystack;
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  // Builds an HTML excerpt around the first token match in `text`,
  // wrapping every token occurrence in <mark>. Output is safe to assign
  // to innerHTML: the source text is escaped before highlighting.
  function buildHighlightedExcerpt(text: string, tokens: string[], maxLen = 160): string {
    if (!text) return '';
    const lowerText = text.toLowerCase();

    let matchIndex = -1;
    for (const tok of tokens) {
      const idx = lowerText.indexOf(tok);
      if (idx !== -1 && (matchIndex === -1 || idx < matchIndex)) matchIndex = idx;
    }

    const anchor = matchIndex === -1 ? 0 : matchIndex;
    const start = Math.max(0, anchor - Math.floor(maxLen / 3));
    const end = Math.min(text.length, start + maxLen);
    const slice = text.slice(start, end);

    const uniqueTokens = Array.from(new Set(tokens.filter(Boolean))).sort((a, b) => b.length - a.length);
    let highlighted = escapeHtml(slice);
    if (uniqueTokens.length > 0) {
      const pattern = uniqueTokens.map((tok) => tok.replace(/[-/\\^$*+?.()|[\]{}]/g, '\\$&')).join('|');
      const re = new RegExp(`(${pattern})`, 'gi');
      highlighted = highlighted.replace(re, '<mark>$1</mark>');
    }

    return `${start > 0 ? '…' : ''}${highlighted}${end < text.length ? '…' : ''}`;
  }

  function toSearchCandidate(doc: NativeSearchDocument, rawScore: number, excerpt?: string): SearchCandidate {
    return {
      url: doc.url,
      rawScore,
      excerpt,
      meta: {
        title: doc.title,
        path: doc.path,
        aliases: (doc.aliases || []).join(', '),
        image: doc.image,
      },
      filters: {
        kind: doc.kind,
        section: doc.section,
      },
    };
  }

  // Generates SearchCandidate with lazy excerpt creation so heavy regex highlighting
  // only runs for the top results actually rendered, not all matching documents.
  function toLazySearchCandidate(
    doc: CachedSearchDocument,
    boost: number,
    rawScore: number,
    tokens: string[]
  ): SearchCandidate {
    let cachedExcerpt: string | undefined = undefined;
    let excerptComputed = false;

    const candidate: SearchCandidate = {
      url: doc.url,
      boost,
      rawScore,
      meta: {
        title: doc.title,
        path: doc.path,
        aliases: (doc.aliases || []).join(', '),
        image: doc.image,
      },
      filters: {
        kind: doc.kind,
        section: doc.section,
      },
      get excerpt(): string | undefined {
        if (!excerptComputed) {
          excerptComputed = true;
          cachedExcerpt = doc.text ? buildHighlightedExcerpt(doc.text, tokens) : '';
        }
        return cachedExcerpt;
      },
      set excerpt(val: string | undefined) {
        cachedExcerpt = val;
        excerptComputed = true;
      },
    };

    return candidate;
  }

  // Searches the in-memory native index, shared by the floating search box
  // and the sidebar's lookup pane. With a query, every token must appear
  // somewhere in the document (AND search) and results are ranked with
  // `calculateSearchBoost`; with an empty query every document (after the
  // optional section filter) is returned in index order, so a caller doing
  // pagination gets the same "browse everything" behavior Pagefind gave the
  // lookup pane when it was called with no term.
  function nativeSearchDocuments(
    query: string,
    index: NativeSearchIndex,
    opts: { sections?: Set<string> | null } = {}
  ): SearchCandidate[] {
    const sections = opts.sections;
    const docs = sections && sections.size > 0 ? index.docs.filter((d) => d.section && sections.has(d.section)) : index.docs;

    const trimmedQuery = query.trim();
    const tokens = tokenizeQuery(trimmedQuery);
    if (tokens.length === 0) {
      return docs.map((doc) => toSearchCandidate(doc, 0));
    }

    // Single-character queries (or empty) search only meta (title/path/alias/heading)
    // to prevent full-text match explosion and UI freeze in large vaults (e.g. 20k docs).
    const searchFullText = trimmedQuery.length >= 2;
    const compiled = compileQuery(trimmedQuery);

    // Sort tokens by descending length for faster early rejection in AND matching
    const sortedTokens = [...tokens].sort((a, b) => b.length - a.length);

    const matches: SearchCandidate[] = [];
    for (let d = 0; d < docs.length; d++) {
      const doc = docs[d] as CachedSearchDocument;
      const haystack = getDocHaystack(doc, searchFullText);

      let allMatch = true;
      for (let i = 0; i < sortedTokens.length; i++) {
        if (!haystack.includes(sortedTokens[i])) {
          allMatch = false;
          break;
        }
      }
      if (!allMatch) continue;

      const boost = calculateDocBoost(compiled, doc);

      let textScore = 0;
      if (searchFullText && doc.text) {
        if (doc._textLower === undefined) {
          doc._textLower = doc.text.toLowerCase();
        }
        const textLower = doc._textLower;
        for (let i = 0; i < tokens.length; i++) {
          const tok = tokens[i];
          let from = 0;
          for (;;) {
            const idx = textLower.indexOf(tok, from);
            if (idx === -1) break;
            textScore += 1;
            from = idx + tok.length;
          }
        }
      }

      // Excerpt is lazily computed when read by the renderer
      matches.push(toLazySearchCandidate(doc, boost, textScore, tokens));
    }

    // Sort with pre-calculated boost and rawScore (pure numerical comparison)
    matches.sort((a, b) => {
      const boostDiff = (b.boost ?? 0) - (a.boost ?? 0);
      if (boostDiff !== 0) return boostDiff;
      return b.rawScore - a.rawScore;
    });

    return matches;
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
      getNativeIndex();
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
        const nativeIndex = await getNativeIndex();

        if (nativeIndex) {
          const topResults = nativeSearchDocuments(query, nativeIndex).slice(0, 10);
          selectedIndex = -1;

          if (topResults.length === 0) {
            showStatus(t('search.empty'));
            showResults();
            return;
          }

          renderResults(topResults);
          showResults();
          return;
        }

        // Native index unavailable (e.g. `search = "pagefind"` or "none"
        // in tmtbook.toml, or this page was served before a build ran) —
        // fall back to Pagefind.
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

        const compiled = compileQuery(query);
        for (let i = 0; i < candidates.length; i++) {
          candidates[i].boost = calculateSearchBoost(query, candidates[i], compiled);
        }

        candidates.sort((a, b) => {
          const boostDiff = (b.boost ?? 0) - (a.boost ?? 0);
          if (boostDiff !== 0) return boostDiff;
          return b.rawScore - a.rawScore;
        });

        const topResults = candidates.slice(0, 10);
        selectedIndex = -1;

        renderResults(topResults);
        showResults();
      }, 200);
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

  Object.assign(T, { setupSearch, getPagefind, getNativeIndex, nativeSearchDocuments });
})();
