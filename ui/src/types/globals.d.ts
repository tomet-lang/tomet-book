// Global type definitions for tmtbook UI runtime.

interface Window {
  TMT: TmtRuntime;
  tmtStrings?: Record<string, string>;
  tmtPageUrl?: string;
  __tmtClassicSpy?: {
    update(): void;
    destroy(): void;
  };
  __editDropdownOutsideClickBound?: boolean;
  [key: string]: any;
}

interface Document {
  __classicTabsViewBound?: boolean;
  [key: string]: any;
}

interface HTMLElement {
  _timer?: any;
  __interactionBound?: boolean;
  __jumpBound?: boolean;
  [key: string]: any;
}

interface Element {
  __jumpBound?: boolean;
  [key: string]: any;
}

interface EventTarget {
  closest?(selector: string): Element | null;
  tagName?: string;
  isContentEditable?: boolean;
  [key: string]: any;
}

interface Event {
  touches?: any;
  changedTouches?: any;
  detail?: any;
  [key: string]: any;
}

interface TmtStorage {
  get(key: string): string | null;
  set(key: string, value: string): void;
  remove?(key: string): void;
}

interface ScrollSpyInstance {
  update(): void;
  currentHeadingId(): string | null;
  markClickScrolling(duration?: number): void;
  destroy(): void;
}

interface ScrollSpyOptions {
  container: Element | Window | null;
  headingTargets: Array<{ id: string; el: Element; link?: Element }>;
  tocLinks?: NodeListOf<Element> | Element[];
  headingLine?: number;
  isActiveView?: () => boolean;
  onActiveChange?: (currentId: string, active?: { id: string; el: Element; link?: Element }) => void;
  onScroll?: () => void;
}

interface TmtRuntime {
  // i18n
  t(key: string, fallback?: string): string;

  // Storage
  storage?: TmtStorage;
  session?: TmtStorage;

  // Utils
  escapeHtml?(str: unknown): string;
  rafThrottle?<T extends (...args: any[]) => void>(fn: T): { (...args: Parameters<T>): void; cancel(): void };
  markVisited?(url: string): void;
  isVisited?(url: string): boolean;
  readHash?(): string;
  syncHeadingHash?(id: string): void;
  createScrollSpy?(options: ScrollSpyOptions): ScrollSpyInstance;
  fetchDocument?(url: string | null | undefined): Promise<Document | null>;
  getPagefind?(): Promise<any>;

  // Panes & Views
  collapseDataPane?(): void;
  expandDataPane?(): void;
  syncViewMode?(mode?: string): void;

  // Inits & Setups
  initPage?(doc?: Document): void;
  setupDevServerLiveReload?(): void;
  setupThemeToggle?(): void;
  setupDraggableFloatingControls?(): void;
  setupViewSwitcher?(): void;
  setupTabsSideSwitcher?(): void;
  setupFloatingCollapse?(): void;
  setupBookViewInteraction?(): void;
  setupClassicViewInteraction?(): void;
  initKeyboardNavigation?(): void;
  setupSearch?(): void;
  initPagePreview?(): void;
  initCenterPeek?(): void;
  initImageLightbox?(): void;
  setupCostumeSwitchers?(): void;
  setupEditDropdown?(): void;
  setupClientRouter?(): void;

  // Components
  TmtBtn?: any;

  // Flexible property map
  [key: string]: any;
}

// Module declaration for tailwindcss to satisfy tailwind.config.ts
declare module 'tailwindcss' {
  export interface Config {
    content: string[];
    darkMode?: string | [string, string];
    theme?: Record<string, any>;
    corePlugins?: Record<string, boolean>;
    plugins?: any[];
    [key: string]: any;
  }
}
