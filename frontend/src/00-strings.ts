// ==================== UI STRINGS ====================
// Injected by base.html from `[ui.strings]`; the fallback keeps this file
// working if it is ever loaded on a page that did not set them.

export function t(key: string, fallback = ''): string {
  const strings = window.tmtStrings || {};
  return strings[key] ?? fallback;
}

const T = (window.TMT ??= {} as any);
T.t = t;
