(() => {
  const T = (window.TMT ??= {} as any);
  const t = (key: string, fallback = "") => T.t(key, fallback);
  const showToast = (...a) => T.showToast(...a);

  // ==================== DEV-ONLY SOURCE VIEWER ====================
  //
  // tomet-book is for reading and publishing, not writing -- authoring
  // stays the job of a real editor (VSCode/Cursor, both already one click
  // away in this same dropdown) or a dedicated tool like tomet-web-editor.
  // This is just a read-only look at a page's raw `.tmt` source, for
  // whenever the rendered HTML doesn't make it obvious what the source
  // actually says.

  let overlay: HTMLElement | null = null;

  async function fetchSource(path: string): Promise<string | null> {
    try {
      const res = await fetch(`/__tmtbook/source?path=${encodeURIComponent(path)}`, {
        cache: "no-store",
      });
      if (!res.ok) return null;
      return await res.text();
    } catch {
      return null;
    }
  }

  function closeOverlay() {
    overlay?.remove();
    overlay = null;
    document.removeEventListener("keydown", onOverlayKeydown, true);
  }

  function onOverlayKeydown(e: KeyboardEvent) {
    if (!overlay) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeOverlay();
    }
  }

  function openOverlay(source: string) {
    overlay = document.createElement("div");
    overlay.className = "tmt-source-viewer-overlay";

    const panel = document.createElement("div");
    panel.className = "tmt-source-viewer-panel";

    const toolbar = document.createElement("div");
    toolbar.className = "tmt-source-viewer-toolbar";

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "tmt-source-viewer-btn";
    closeBtn.textContent = t("edit.close");
    closeBtn.onclick = closeOverlay;

    toolbar.append(closeBtn);

    const textarea = document.createElement("textarea");
    textarea.className = "tmt-source-viewer-textarea";
    textarea.value = source;
    textarea.readOnly = true;
    textarea.spellcheck = false;

    panel.append(toolbar, textarea);
    overlay.appendChild(panel);
    document.body.appendChild(overlay);

    overlay.addEventListener("mousedown", (e) => {
      if (e.target === overlay) closeOverlay();
    });
    document.addEventListener("keydown", onOverlayKeydown, true);

    textarea.focus();
    textarea.setSelectionRange(0, 0);
  }

  function setupSourceViewer() {
    const toggle = document.querySelector<HTMLButtonElement>(".btn-source-view-toggle");
    if (!toggle) return;

    toggle.onclick = async (e) => {
      e.preventDefault();
      e.stopPropagation();
      const dropdown = toggle.closest(".edit-dropdown");
      if (dropdown) {
        dropdown.classList.remove("is-open");
        const menu = dropdown.querySelector<HTMLElement>(".edit-dropdown-menu");
        if (menu) menu.hidden = true;
      }

      const path = toggle.dataset.sourcePath || "";
      if (!path) return;
      const source = await fetchSource(path);
      if (source == null) {
        showToast(t("toast.source_load_failed"));
        return;
      }
      openOverlay(source);
    };
  }

  Object.assign(T, { setupSourceViewer });
})();
