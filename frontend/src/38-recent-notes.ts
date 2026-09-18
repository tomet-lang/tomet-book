(() => {
  const T = (window.TMT ??= {} as any);

  // ==================== RECENT NOTES HISTORY ====================

  /** Every internal link (book_index, all_entries, backlinks, ...) points to
   *  a slug path with no trailing slash -- see EntrySummary::url in
   *  renderer.rs. A URL typed, bookmarked, or pasted with a trailing slash
   *  still loads the same page but gives `location.pathname` a different
   *  string, which used to mean a different dedup key AND a different
   *  spine color for what is the same note. */
  function normalizePath(path) {
    if (!path) return path;
    return path.replace(/\/index\.html$/i, '').replace(/(.)\/+$/, '$1');
  }

  function markVisited(path) {
    const p = normalizePath(path);
    if (!p || p === '/wiki' || p === '/') return;
    try {
      const set = new Set(JSON.parse(T.storage.get('wiki-visited-notes') || '[]'));
      set.add(p);
      T.storage.set('wiki-visited-notes', JSON.stringify([...set]));
    } catch {}
  }

  function isVisited(path) {
    const p = normalizePath(path);
    if (!p) return false;
    try {
      const list = JSON.parse(T.storage.get('wiki-visited-notes') || '[]');
      return list.includes(p);
    } catch {
      return false;
    }
  }

  function syncVisitedClasses() {
    try {
      const list = JSON.parse(T.storage.get('wiki-visited-notes') || '[]');
      if (!list.length) return;
      const set = new Set(list);
      document.querySelectorAll('.book-index-link, .search-result-item').forEach((a) => {
        const href = normalizePath(a.getAttribute('href'));
        if (href && set.has(href)) {
          a.classList.add('is-visited');
        }
      });
    } catch {}
  }

  document.addEventListener('click', (e) => {
    const a = e.target.closest('a');
    const href = a?.getAttribute('href');
    if (href && (href.startsWith('/wiki/') || href.startsWith('wiki/'))) {
      markVisited(href);
      a.classList.add('is-visited');
    }
  });

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', syncVisitedClasses);
  } else {
    syncVisitedClasses();
  }

  /** A note's spine height: mostly a fixed shelf height, nudged by a few
   *  pixels either way so the shelf still reads as a row of individual books
   *  rather than identical tiles -- but never so that a shorter title ends
   *  up taller than a longer one. title.length is linearly mapped onto the
   *  +/-VARIANCE band and clamped at both ends (titles at or below MIN_LEN
   *  bottom out at BASE_HEIGHT - VARIANCE, at or above MAX_LEN top out at
   *  BASE_HEIGHT + VARIANCE), so it's monotonic by construction rather than
   *  hashed -- a hash gave two titles differing only by a few characters no
   *  guarantee about which came out taller. */
  function spineHeightFor(title) {
    const BASE_HEIGHT = 150;
    const VARIANCE = 5; // +/- px
    const MIN_LEN = 4; // titles at/under this length bottom out
    const MAX_LEN = 18; // titles at/over this length top out
    const t = Math.min(1, Math.max(0, (title.length - MIN_LEN) / (MAX_LEN - MIN_LEN)));
    return Math.round(BASE_HEIGHT - VARIANCE + t * VARIANCE * 2);
  }

  function updateRecentNotes() {
    const currentPath = normalizePath(window.location.pathname);
    const titleEl = document.querySelector('.content-title') || document.querySelector('.article-header h1');
    const currentTitle = titleEl?.textContent?.trim() || document.title.replace(/\s*\|.*$/, '');

    if (!currentPath || !currentTitle || currentPath === '/wiki' || currentPath === '/') return;

    markVisited(currentPath);

    let recents = [];
    try {
      recents = JSON.parse(T.storage.get('wiki-recent-notes') || '[]');
    } catch {}

    // Re-normalize on read too, so an entry saved before this fix (or from a
    // trailing-slash URL saved before the fix existed) still dedupes and
    // colors the same as the same page reached the canonical way.
    recents = recents
      .map((r) => ({ ...r, path: normalizePath(r.path) }))
      .filter((r) => r.path !== currentPath);
    recents.unshift({ path: currentPath, title: currentTitle });
    if (recents.length > 8) recents = recents.slice(0, 8);

    T.storage.set('wiki-recent-notes', JSON.stringify(recents));

    const recentSection = document.getElementById('pane-recent-section');
    const recentList = document.getElementById('pane-recent-list');
    const itemTemplate = document.getElementById('recent-note-item-template') as HTMLTemplateElement | null;
    if (recentSection && recentList && itemTemplate && recents.length > 0) {
      const items = recents.map((r) => {
        const isCurrent = r.path === currentPath;
        // The current page keeps its own tinted card (.is-current in CSS)
        // and stands taller than its title length alone would give it --
        // on a real shelf, the book you're actually reading is the one
        // you'd spot first, not just a differently-colored spine.
        const CURRENT_HEIGHT_BONUS = 20;
        const height = spineHeightFor(r.title) + (isCurrent ? CURRENT_HEIGHT_BONUS : 0);

        const li = itemTemplate.content.firstElementChild?.cloneNode(true) as HTMLElement;
        const link = li.querySelector<HTMLAnchorElement>('.recent-link');
        // `.href`/`.style`/`.textContent` are DOM property assignments, not
        // markup parsing, so r.path/r.title never need escaping here the
        // way the old innerHTML-string version did.
        if (link) {
          link.href = r.path;
          link.classList.toggle('is-current', isCurrent);
          link.style.height = `${height}px`;
          const title = link.querySelector('.recent-title');
          if (title) title.textContent = r.title;
        }
        return li;
      });
      recentList.replaceChildren(...items);
      recentSection.hidden = false;
    }
  }

  Object.assign(T, { updateRecentNotes, normalizePath, markVisited, isVisited, syncVisitedClasses });
})();
