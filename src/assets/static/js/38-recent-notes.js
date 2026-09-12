(() => {
  const T = (window.TMT ??= {});

  // ==================== RECENT NOTES HISTORY ====================
  function updateRecentNotes() {
    const currentPath = window.location.pathname;
    const titleEl = document.querySelector('.content-title') || document.querySelector('.article-header h1');
    const currentTitle = titleEl?.textContent?.trim() || document.title.replace(/\s*\|.*$/, '');
    const iconEl = document.querySelector('.avatar-icon') || document.querySelector('.title-inline-icon');
    const currentIcon = iconEl?.textContent?.trim() || '📄';

    if (!currentPath || !currentTitle || currentPath === '/wiki' || currentPath === '/wiki/' || currentPath === '/' || currentPath === '/index.html') return;

    let recents = [];
    try {
      recents = JSON.parse(T.storage.get('wiki-recent-notes') || '[]');
    } catch {}

    recents = recents.filter((r) => r.path !== currentPath);
    recents.unshift({ path: currentPath, title: currentTitle, icon: currentIcon });
    if (recents.length > 8) recents = recents.slice(0, 8);

    T.storage.set('wiki-recent-notes', JSON.stringify(recents));

    const recentSection = document.getElementById('pane-recent-section');
    const recentList = document.getElementById('pane-recent-list');
    if (recentSection && recentList && recents.length > 0) {
      recentList.innerHTML = recents
        .map(
          (r) => `
        <li>
          <a href="${r.path}" class="recent-link ${r.path === currentPath ? 'is-current' : ''}">
            <span class="recent-icon">${r.icon}</span>
            <span class="recent-title">${r.title}</span>
          </a>
        </li>
      `
        )
        .join('');
      recentSection.hidden = false;
    }
  }

  Object.assign(T, { updateRecentNotes });
})();
