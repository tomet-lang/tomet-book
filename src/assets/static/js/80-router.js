(() => {
  const T = (window.TMT ??= {});

  let isNavigating = false;
  let currentPath = normalizePath(window.location.href);

  function normalizePath(url) {
    if (!url) return '';
    try {
      const u = new URL(url, window.location.origin);
      return u.pathname.replace(/\/index\.html$/i, '').replace(/(.)\/+$/, '$1');
    } catch {
      return url.split('#')[0].replace(/\/index\.html$/i, '').replace(/(.)\/+$/, '$1');
    }
  }

  function saveCurrentScrollState() {
    const scrollContainer = document.getElementById('book-content-scroll');
    const scrollTop = scrollContainer ? scrollContainer.scrollTop : window.scrollY;
    try {
      history.replaceState(
        {
          ...(history.state || {}),
          scrollTop,
          path: window.location.href,
        },
        document.title
      );
    } catch {}
  }

  function swapContent(newDoc, url, cleanUrl, hash, pushState, savedScrollTop) {
    // 1. Title & URL
    document.title = newDoc.title;
    const scriptEl = Array.from(newDoc.querySelectorAll('script')).find((s) =>
      s.textContent.includes('window.tmtPageUrl')
    );
    if (scriptEl) {
      const m = scriptEl.textContent.match(/window\.tmtPageUrl\s*=\s*(".*?"|'.*?');/);
      if (m) {
        try {
          window.tmtPageUrl = JSON.parse(m[1]);
        } catch {}
      }
    } else {
      window.tmtPageUrl = cleanUrl;
    }

    if (pushState) {
      history.pushState({ path: url, scrollTop: 0 }, newDoc.title, url);
    }

    currentPath = normalizePath(cleanUrl);

    // 2. Mark visited
    T.markVisited?.(cleanUrl);

    // 3. Breadcrumbs in TOC panel
    const currentBreadcrumb = document.querySelector('#pane-panel-toc .nav-breadcrumbs');
    const newBreadcrumb = newDoc.querySelector('#pane-panel-toc .nav-breadcrumbs');
    if (currentBreadcrumb && newBreadcrumb) {
      currentBreadcrumb.innerHTML = newBreadcrumb.innerHTML;
    }

    // 4. TOC in TOC panel
    const currentToc = document.querySelector('#pane-panel-toc .pane-toc');
    const newToc = newDoc.querySelector('#pane-panel-toc .pane-toc');
    if (newToc) {
      if (currentToc) {
        currentToc.innerHTML = newToc.innerHTML;
      } else {
        const tocPanelInner = document.querySelector('#pane-panel-toc .pane-tab-inner');
        if (tocPanelInner) {
          const div = document.createElement('div');
          div.className = 'pane-toc';
          div.innerHTML = newToc.innerHTML;
          tocPanelInner.appendChild(div);
        }
      }
    } else if (currentToc) {
      currentToc.remove();
    }

    // 5. Backlinks panel
    const currentLinks = document.getElementById('pane-panel-links');
    const newLinks = newDoc.getElementById('pane-panel-links');
    if (currentLinks && newLinks) {
      currentLinks.innerHTML = newLinks.innerHTML;
    }

    // 6. Update .is-here in sidebar
    const targetNorm = normalizePath(cleanUrl);
    document.querySelectorAll('#pane-nav .book-index-link.is-here').forEach((el) => {
      el.classList.remove('is-here');
      el.removeAttribute('aria-current');
    });
    document.querySelectorAll('#pane-nav .book-index-link').forEach((el) => {
      const href = normalizePath(el.getAttribute('href'));
      if (href && href === targetNorm) {
        el.classList.add('is-here');
        el.setAttribute('aria-current', 'page');
      }
    });

    // 7. Data pane (infobox)
    const currentDataPane = document.getElementById('pane-data');
    const newDataPane = newDoc.getElementById('pane-data');
    if (newDataPane) {
      if (currentDataPane) {
        currentDataPane.innerHTML = newDataPane.innerHTML;
      } else {
        const contentPane = document.querySelector('.pane-content');
        if (contentPane && contentPane.parentNode) {
          const cloned = newDataPane.cloneNode(true);
          contentPane.parentNode.insertBefore(cloned, contentPane);
          T.bindDataPaneToggle?.();
        }
      }
    } else if (currentDataPane) {
      currentDataPane.remove();
    }

    // 8. Sticky tabs header
    const currentHeader = document.querySelector('.pane-content-header');
    const newHeader = newDoc.querySelector('.pane-content-header');
    if (currentHeader && newHeader) {
      currentHeader.innerHTML = newHeader.innerHTML;
    } else if (currentHeader) {
      currentHeader.innerHTML = '';
    }

    // 9. Article container
    const currentArticle = document.querySelector('.pane-article-container');
    const newArticle = newDoc.querySelector('.pane-article-container');
    if (currentArticle && newArticle) {
      currentArticle.innerHTML = newArticle.innerHTML;
    }

    // 10. Classic view
    const currentClassic = document.querySelector('.classic-view');
    const newClassic = newDoc.querySelector('.classic-view');
    if (currentClassic && newClassic) {
      currentClassic.innerHTML = newClassic.innerHTML;
    }

    // 11. Scroll article to hash, saved position, or top
    const scrollContainer = document.getElementById('book-content-scroll');
    const isClassic = document.documentElement.getAttribute('data-view') === 'classic';

    if (hash) {
      const target = document.getElementById(hash);
      if (target && scrollContainer) {
        scrollContainer.scrollTop = target.offsetTop - scrollContainer.offsetTop - 16;
      } else if (scrollContainer) {
        scrollContainer.scrollTop = 0;
      }
      if (isClassic) {
        if (target) {
          window.scrollTo({ top: target.offsetTop - 32, behavior: 'auto' });
        } else {
          window.scrollTo(0, 0);
        }
      }
    } else if (savedScrollTop !== null && savedScrollTop !== undefined) {
      if (scrollContainer) {
        scrollContainer.scrollTop = savedScrollTop;
      }
      if (isClassic) {
        window.scrollTo({ top: savedScrollTop, behavior: 'auto' });
      }
    } else {
      if (scrollContainer) {
        scrollContainer.scrollTop = 0;
      }
      if (isClassic) {
        window.scrollTo(0, 0);
      }
    }

    // 12. Rebind interactions
    T.setupBookViewInteraction?.();
    if (isClassic) {
      T.setupClassicViewInteraction?.();
    }
    T.setupCostumeSwitchers?.();
    T.setupEditDropdown?.();
    T.updateRecentNotes?.();
    T.syncVisitedClasses?.();

    // 13. Finish progress bar
    T.finishPageProgress?.();
  }

  async function navigateTo(url, pushState = true, savedScrollTop = null) {
    if (isNavigating) return;

    const targetNorm = normalizePath(url);
    const hash = url.includes('#') ? url.split('#')[1] : '';

    // Same document jump (heading anchor or top)
    if (currentPath === targetNorm) {
      if (hash) {
        const targetEl = document.getElementById(hash);
        const scrollContainer = document.getElementById('book-content-scroll');
        if (targetEl) {
          if (document.documentElement.getAttribute('data-view') === 'classic') {
            window.scrollTo({ top: targetEl.offsetTop - 32, behavior: 'smooth' });
          } else if (scrollContainer) {
            scrollContainer.scrollTo({
              top: targetEl.offsetTop - scrollContainer.offsetTop - 16,
              behavior: 'smooth',
            });
          }
        }
      } else {
        const scrollContainer = document.getElementById('book-content-scroll');
        if (document.documentElement.getAttribute('data-view') === 'classic') {
          window.scrollTo({ top: 0, behavior: 'smooth' });
        } else if (scrollContainer) {
          scrollContainer.scrollTo({ top: 0, behavior: 'smooth' });
        }
      }
      if (pushState) {
        history.pushState({ path: url, scrollTop: 0 }, '', url);
      }
      return;
    }

    // Save scroll position for the page we are about to leave
    if (pushState) {
      saveCurrentScrollState();
    }

    isNavigating = true;
    T.startPageProgress?.();

    // Close floating UI
    T.closeCenterPeek?.();
    T.hidePreviewCard?.();
    T.hidePopover?.();
    const searchResults = document.getElementById('wiki-search-results');
    if (searchResults) searchResults.hidden = true;

    try {
      const cleanUrl = url.split('#')[0];
      const newDoc = await T.fetchDocument?.(cleanUrl);
      if (!newDoc) {
        T.finishPageProgress?.();
        if (!pushState) {
          window.location.reload();
        } else {
          window.location.href = url;
        }
        return;
      }

      const currentBookView = document.getElementById('wiki-book-view');
      const newBookView = newDoc.getElementById('wiki-book-view');

      // If either page is not a note (e.g. index catalog or different layout), fallback to full navigation
      if (!currentBookView || !newBookView) {
        T.finishPageProgress?.();
        if (!pushState) {
          window.location.reload();
        } else {
          window.location.href = url;
        }
        return;
      }

      if (document.startViewTransition) {
        document.startViewTransition(() => {
          swapContent(newDoc, url, cleanUrl, hash, pushState, savedScrollTop);
        });
      } else {
        swapContent(newDoc, url, cleanUrl, hash, pushState, savedScrollTop);
      }
    } catch (err) {
      console.warn('PJAX navigation failed, falling back:', err);
      T.finishPageProgress?.();
      if (!pushState) {
        window.location.reload();
      } else {
        window.location.href = url;
      }
    } finally {
      isNavigating = false;
    }
  }

  function setupClientRouter() {
    if (window.__clientRouterInitialized) return;
    window.__clientRouterInitialized = true;

    currentPath = normalizePath(window.location.href);

    // Seed initial history state with scroll position
    try {
      if (!history.state || history.state.scrollTop === undefined) {
        const scrollContainer = document.getElementById('book-content-scroll');
        const scrollTop = scrollContainer ? scrollContainer.scrollTop : window.scrollY;
        history.replaceState(
          { ...(history.state || {}), path: window.location.href, scrollTop },
          document.title
        );
      }
    } catch {}

    document.addEventListener('click', (e) => {
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button !== 0) return;

      const a = e.target.closest('a');
      if (!a) return;

      const href = a.getAttribute('href');
      if (!href) return;

      // Ignore external, mailto, target="_blank", or hash-only links
      if (
        href.startsWith('http://') ||
        href.startsWith('https://') ||
        href.startsWith('//') ||
        href.startsWith('mailto:') ||
        a.getAttribute('target') === '_blank' ||
        href.startsWith('#')
      ) {
        return;
      }

      // Only handle in-app wiki note links
      if (!href.startsWith('/wiki') && !href.startsWith('wiki')) {
        return;
      }

      e.preventDefault();
      navigateTo(href, true);
    });

    window.addEventListener('popstate', (e) => {
      const savedScroll = e.state?.scrollTop;
      navigateTo(window.location.href, false, savedScroll);
    });
  }

  Object.assign(T, { setupClientRouter, navigateTo });
})();
