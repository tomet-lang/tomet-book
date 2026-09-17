// tmtbook UI runtime entry point.
// Preserves the load order defined in assets.rs. Each module registers onto
// window.TMT or acts as a self-contained custom element / listener.

import './00-utils';
import './00-strings';
import './05-tmt-btn';
import './06-tmt-badge';
import './07-tmt-icon';
import './08-tmt-swatch';
import './09-tmt-switch';
import './10-theme';
import './20-view-mode';
import './30-floating-controls';
import './31-page-progress';
import './36-nav-pane';
import './37-data-pane';
import './38-recent-notes';
import './39-lookup-pane';
import './40-book-view';
import './50-keyboard';
import './60-search';
import './70-preview';
import './72-center-peek';
import './75-lightbox';
import './78-costume';
import './80-router';
import './85-classic-view';
import './90-edit-menu';
import './92-code-copy';
import './95-init';
import './97-live-reload';
import './99-bootstrap';
