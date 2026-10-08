// ==================== SHARED HELPERS ====================

/** Declarative Shadow DOM (`<template shadowrootmode>`) only auto-attaches
 *  when the browser's HTML parser meets it during real document parsing --
 *  the legacy `Element.innerHTML` setter's fragment-parsing algorithm
 *  deliberately skips it. Content swapped in that way (80-router.js's
 *  `swapContent` replaces whole panes via `.innerHTML =`) leaves the
 *  template sitting inert as a plain child instead of a live shadow root,
 *  so every tmt-* element inside falls back to unstyled light-DOM content.
 *  Call this at the top of every tmt-* custom element's connectedCallback:
 *  a no-op when the browser already attached the shadow root declaratively,
 *  and otherwise finishes the job by hand from the same inert template. */
export function ensureShadowRoot(el: HTMLElement): void {
  if (el.shadowRoot) return;
  const tpl = el.querySelector(':scope > template[shadowrootmode]') as HTMLTemplateElement | null;
  if (!tpl) return;
  const mode = tpl.getAttribute('shadowrootmode') === 'closed' ? 'closed' : 'open';
  const shadow = el.attachShadow({ mode });
  shadow.appendChild(tpl.content.cloneNode(true));
  tpl.remove();
}

/** Coalesce repeated calls into at most one `fn()` invocation per animation
 *  frame. The returned function also exposes `.cancel()`, for a caller that
 *  needs to drop a still-pending call (e.g. a drag that just ended). */
export function rafThrottle<T extends (...args: any[]) => void>(fn: T): { (...args: Parameters<T>): void; cancel(): void } {
  let frame: number | null = null;
  const throttled = (...args: Parameters<T>) => {
    if (frame !== null) return;
    frame = requestAnimationFrame(() => {
      frame = null;
      fn(...args);
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
export const storage: TmtStorage = {
  get(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key: string, value: string): void {
    try {
      localStorage.setItem(key, value);
    } catch {}
  },
  remove(key: string): void {
    try {
      localStorage.removeItem(key);
    } catch {}
  },
};

export const session: TmtStorage = {
  get(key: string): string | null {
    try {
      return sessionStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key: string, value: string): void {
    try {
      sessionStorage.setItem(key, value);
    } catch {}
  },
  remove(key: string): void {
    try {
      sessionStorage.removeItem(key);
    } catch {}
  },
};

/** Heading ID indicated by the current URL hash. Safely handles malformed strings. */
export function readHash(): string {
  const raw = (window.location.hash || '').slice(1);
  if (!raw) return '';
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

// Synchronize URL hash with current heading without polluting history.
export function syncHeadingHash(id: string): void {
  if (!id) return;
  if (readHash() === id) return;
  try {
    history.replaceState(null, '', `#${encodeURIComponent(id)}`);
  } catch {}
}

/** Shared document cache and fetcher across hover preview and center peek. */
const docCache = new Map<string, Document>();

export async function fetchDocument(url: string | null | undefined): Promise<Document | null> {
  if (!url) return null;
  const cleanUrl = url.split('#')[0];
  let cached = docCache.get(cleanUrl);
  if (!cached) {
    try {
      const res = await fetch(cleanUrl);
      if (!res.ok) return null;
      const html = await res.text();
      const doc = new DOMParser().parseFromString(html, 'text/html');
      docCache.set(cleanUrl, doc);
      cached = doc;
    } catch {
      return null;
    }
  }
  return cached;
}

/**
 * ScrollSpy: cursor-based heading tracker and TOC synchronizer.
 * Walks bidirectionally from a cached cursor using bounding client rects.
 * Exactly identical algorithm across book view and classic view:
 * reads headings relative to the trigger line without artificial bottom-of-page patches.
 */
export function createScrollSpy({
  container,
  headingTargets,
  tocLinks = [],
  headingLine = 80,
  isActiveView = () => true,
  onActiveChange,
  onScroll: onScrollCallback,
}: ScrollSpyOptions): ScrollSpyInstance {
  let headingCursor = 0;
  let isClickScrolling = false;
  let clickTimer: any = null;

  const scrollTarget: any = container && container !== window ? container : window;

  const markClickScrolling = (duration = 800) => {
    isClickScrolling = true;
    if (clickTimer) clearTimeout(clickTimer);
    clickTimer = setTimeout(() => {
      isClickScrolling = false;
    }, duration);
  };

  const currentHeadingId = (): string | null => {
    const n = headingTargets?.length || 0;
    if (n === 0) return null;

    const containerTop =
      container && container !== window
        ? (container as Element).getBoundingClientRect().top
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
      (tocLinks as any).forEach((l: Element) => l.classList.remove('is-active'));
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

// Backward-compatibility registration onto window.TMT
const T = (window.TMT ??= {} as any);
Object.assign(T, {
  ensureShadowRoot,
  rafThrottle,
  storage,
  session,
  readHash,
  syncHeadingHash,
  createScrollSpy,
  fetchDocument,
});
