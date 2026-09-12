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

  Object.assign(T, { rafThrottle, storage });
})();
