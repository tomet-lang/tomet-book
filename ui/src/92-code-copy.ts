(() => {
  const t = (key: string, fallback = "") =>
    window.TMT?.t ? window.TMT.t(key, fallback) : (window.tmtStrings?.[key] ?? fallback ?? key);

  document.addEventListener('click', async (e) => {
    const btn = e.target.closest('.code-copy-btn');
    if (!btn) return;
    const wrapper = btn.closest('.code-block-wrapper');
    const code = wrapper ? (wrapper.querySelector('pre code') || wrapper.querySelector('pre')) : null;
    if (!code) return;

    const text = code.textContent || '';
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      }

      const use = btn.querySelector('use');
      if (use) use.setAttribute('href', '/icons/lucide.svg#check');
      const icon = btn.querySelector('tmt-icon');
      if (icon) icon.setAttribute('name', 'check');
      btn.classList.add('is-copied');
      btn.setAttribute('title', t('code.copied', 'コピーしました'));
      btn.setAttribute('aria-label', t('code.copied', 'コピーしました'));

      if (btn._resetTimer) clearTimeout(btn._resetTimer);
      btn._resetTimer = setTimeout(() => {
        if (use) use.setAttribute('href', '/icons/lucide.svg#copy');
        if (icon) icon.setAttribute('name', 'copy');
        btn.classList.remove('is-copied');
        btn.setAttribute('title', t('code.copy', 'コードをコピー'));
        btn.setAttribute('aria-label', t('code.copy', 'コードをコピー'));
        btn._resetTimer = null;
      }, 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  });
})();
