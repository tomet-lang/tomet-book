(() => {
  const T = (window.TMT ??= {});
  const t = (key, fallback) => T.t(key, fallback);

  // ==================== LOOKUP PANE (A-Z / TAG BROWSE + FILTER) ====================
  // Reuses the sidebar's book-index-* card styling (see 50-nav-infobox.css)
  // so a result still reads as a small book, not a plain search hit.

  /** /all-entries.json is one file for the whole vault, written once per
   *  build (see build_book in mod.rs) -- not re-fetched on every SPA
   *  navigation, since setupLookupPane() itself reruns on every one (the
   *  router replaces #pane-nav's innerHTML, so its inputs are fresh DOM
   *  nodes each time and need rebinding regardless). */
  let entriesPromise = null;
  function loadAllEntries() {
    if (!entriesPromise) {
      entriesPromise = fetch('/all-entries.json')
        .then((res) => (res.ok ? res.json() : []))
        .catch(() => []);
    }
    return entriesPromise;
  }

  /** The character a title sorts under: uppercased for ASCII letters, left
   *  as-is otherwise -- there is no reading/furigana field to bucket kana
   *  into proper gojuon rows, so a title starting with a kanji or kana
   *  groups under that literal character. */
  function groupKey(title) {
    const ch = title.trim().charAt(0) || '#';
    return /[a-z]/i.test(ch) ? ch.toUpperCase() : ch;
  }

  function entryHtml(entry, hereUrl) {
    const isHere = hereUrl && entry.url === hereUrl;
    return `
      <li class="book-index-item">
        <a href="${entry.url}" class="book-index-link${isHere ? ' is-here' : ''}"${isHere ? ' aria-current="page"' : ''}>
          <span class="book-index-title">${entry.title}</span>
          ${entry.section ? `<span class="book-index-meta">${entry.section}</span>` : ''}
        </a>
      </li>
    `;
  }

  function setupLookupPane() {
    const input = document.getElementById('lookup-filter-input');
    const tagsBox = document.getElementById('lookup-tags');
    const results = document.getElementById('lookup-results');
    if (!input || !tagsBox || !results) return;
    if (input.dataset.lookupInitialized === 'true') return;
    input.dataset.lookupInitialized = 'true';

    const hereUrl = window.tmtPageUrl || null;
    const activeTags = new Set();
    let sorted = [];

    function render() {
      const query = input.value.trim().toLowerCase();
      const filtered = sorted.filter((entry) => {
        if (activeTags.size > 0 && !activeTags.has(entry.section)) return false;
        return !query || entry.title.toLowerCase().includes(query);
      });

      if (filtered.length === 0) {
        results.innerHTML = `<div class="pane-placeholder-box"><span class="placeholder-text">${t('search.empty')}</span></div>`;
        return;
      }

      // Browsing (nothing typed, no tag picked): a real A-Z/あ-ん index,
      // grouped by first character. Once the reader is filtering, letter
      // groups would mostly hold a single entry each, so switch to a flat
      // result list instead.
      if (!query && activeTags.size === 0) {
        let html = '';
        let currentGroup = null;
        for (const entry of filtered) {
          const key = groupKey(entry.title);
          if (key !== currentGroup) {
            if (currentGroup !== null) html += '</ul>';
            html += `<div class="lookup-group-heading">${key}</div><ul class="book-index-items">`;
            currentGroup = key;
          }
          html += entryHtml(entry, hereUrl);
        }
        html += '</ul>';
        results.innerHTML = html;
      } else {
        results.innerHTML = `<ul class="book-index-items">${filtered.map((e) => entryHtml(e, hereUrl)).join('')}</ul>`;
      }
    }

    input.addEventListener('input', render);

    loadAllEntries().then((entries) => {
      sorted = [...entries].sort((a, b) => a.title.localeCompare(b.title, 'ja'));
      const sections = [...new Set(entries.map((e) => e.section).filter(Boolean))].sort((a, b) =>
        a.localeCompare(b, 'ja')
      );
      if (sections.length > 0) {
        tagsBox.hidden = false;
        tagsBox.innerHTML = sections
          .map((s) => `<button type="button" class="lookup-tag-chip" data-tag="${s}">${s}</button>`)
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
  }

  Object.assign(T, { setupLookupPane });
})();
