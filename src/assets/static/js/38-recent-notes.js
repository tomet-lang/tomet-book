(() => {
  const T = (window.TMT ??= {});

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
    if (recentSection && recentList && recents.length > 0) {
      recentList.innerHTML = recents
        .map((r) => {
          const isCurrent = r.path === currentPath;
          // The current page keeps its own tinted card (.is-current in CSS)
          // and stands taller than its title length alone would give it --
          // on a real shelf, the book you're actually reading is the one
          // you'd spot first, not just a differently-colored spine.
          const CURRENT_HEIGHT_BONUS = 20;
          const height = spineHeightFor(r.title) + (isCurrent ? CURRENT_HEIGHT_BONUS : 0);
          const spineStyle = ` style="height: ${height}px"`;
          return `
        <li>
          <a href="${r.path}" class="recent-link ${isCurrent ? 'is-current' : ''}"${spineStyle}>
            <span class="recent-title">${r.title}</span>
          </a>
        </li>
      `;
        })
        .join('');
      recentSection.hidden = false;
    }
  }

  Object.assign(T, { updateRecentNotes });
})();
