(() => {
  const T = (window.TMT ??= {});

  // ==================== SHARED HELPERS ====================

  /** Coalesce repeated calls into at most one `fn()` invocation per animation
   *  frame. The returned function also exposes `.cancel()`, for a caller that
   *  needs to drop a still-pending call (e.g. a drag that just ended). */
  function rafThrottle(fn) {
    let frame = null;
    const throttled = () => {
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        fn();
      });
    };
    throttled.cancel = () => {
      if (frame !== null) {
        cancelAnimationFrame(frame);
        frame = null;
      }
    };
    return throttled;
  }

  // localStorage throws in Safari private browsing (and wherever a user has
  // disabled site data), so every call site used to wrap itself in its own
  // try/catch. Centralising it here means a read that fails just reads as
  // "nothing saved" rather than a script error breaking the caller.
  const storage = {
    get(key) {
      try {
        return localStorage.getItem(key);
      } catch {
        return null;
      }
    },
    set(key, value) {
      try {
        localStorage.setItem(key, value);
      } catch {}
    },
  };

  // Synchronize URL hash with current heading without polluting history.
  function syncHeadingHash(id) {
    if (!id) return;
    const newHash = '#' + id;
    if (window.location.hash !== newHash) {
      history.replaceState(null, '', newHash);
    }
  }

  /**
   * ScrollSpy: cursor-based heading tracker and TOC synchronizer.
   * Walks bidirectionally from a cached cursor using bounding client rects.
   * Exactly identical algorithm across book view and classic view:
   * reads headings relative to the trigger line without artificial bottom-of-page patches.
   *
   * @param {Object} options
   * @param {Element|Window|null} options.container - Scroll container (Element or window)
   * @param {Array<{id: string, el: Element, link?: Element}>} options.headingTargets
   * @param {NodeList|Array<Element>} [options.tocLinks]
   * @param {number} [options.headingLine=80] - Distance in px from container top
   * @param {Function} [options.isActiveView] - Predicate whether current view is active
   * @param {Function} [options.onActiveChange] - Callback when active heading changes
   * @param {Function} [options.onScroll] - Callback invoked on every scroll event
   * @returns {{ update: Function, currentHeadingId: Function, markClickScrolling: Function, destroy: Function }}
   */
  function createScrollSpy({
    container,
    headingTargets,
    tocLinks = [],
    headingLine = 80,
    isActiveView = () => true,
    onActiveChange,
    onScroll: onScrollCallback,
  }) {
    let headingCursor = 0;
    let isClickScrolling = false;
    let clickTimer = null;

    const scrollTarget = container && container !== window ? container : window;

    const markClickScrolling = (duration = 800) => {
      isClickScrolling = true;
      if (clickTimer) clearTimeout(clickTimer);
      clickTimer = setTimeout(() => {
        isClickScrolling = false;
      }, duration);
    };

    const currentHeadingId = () => {
      const n = headingTargets?.length || 0;
      if (n === 0) return null;

      const containerTop =
        container && container !== window
          ? container.getBoundingClientRect().top
          : 0;
      const line = containerTop + headingLine;

      if (headingCursor >= n) headingCursor = n - 1;

      // Retreat: the reader scrolled up past where the cursor thought they were.
      while (
        headingCursor > 0 &&
        headingTargets[headingCursor].el.getBoundingClientRect().top > line
      ) {
        headingCursor--;
      }
      // Advance: the reader scrolled down past it instead.
      while (
        headingCursor < n - 1 &&
        headingTargets[headingCursor + 1].el.getBoundingClientRect().top <= line
      ) {
        headingCursor++;
      }

      return headingTargets[headingCursor].id;
    };

    const update = () => {
      if (!isActiveView()) return;
      if (isClickScrolling) return;

      const currentId = currentHeadingId();
      if (!currentId) return;

      const active = headingTargets.find((item) => item.id === currentId);
      if (active?.link && !active.link.classList.contains('is-active')) {
        tocLinks.forEach((l) => l.classList.remove('is-active'));
        active.link.classList.add('is-active');
        active.link.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }

      if (onActiveChange) {
        onActiveChange(currentId, active);
      }
    };

    const throttledUpdate = rafThrottle(() => {
      if (!isActiveView()) return;
      if (onScrollCallback) onScrollCallback();
      update();
    });

    if (scrollTarget.__tmtScrollHandler) {
      scrollTarget.removeEventListener('scroll', scrollTarget.__tmtScrollHandler);
    }
    scrollTarget.__tmtScrollHandler = throttledUpdate;
    scrollTarget.addEventListener('scroll', throttledUpdate, { passive: true });

    return {
      update,
      currentHeadingId,
      markClickScrolling,
      destroy: () => {
        if (scrollTarget.__tmtScrollHandler === throttledUpdate) {
          scrollTarget.removeEventListener('scroll', throttledUpdate);
          scrollTarget.__tmtScrollHandler = null;
        }
        if (clickTimer) clearTimeout(clickTimer);
      },
    };
  }

  Object.assign(T, { rafThrottle, storage, syncHeadingHash, createScrollSpy });
})();
