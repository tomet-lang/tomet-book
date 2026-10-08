pub mod document;
pub mod renderer;
pub mod slug;

pub use document::{
    HeroChipItem, InfoboxRowItem, ProcessedDoc, SectionTab, TocItem, enhance_internal_links,
    extract_workspace_config_blocks, process_parsed_document, process_parsed_document_with_blocks,
    strip_doc_extension,
};
pub use renderer::{Backlink, BookRenderer, EntrySummary, SectionSummary};
pub use slug::{
    DocRouteInput, RouteTable, clean_doc_slug, clean_segment, normalize_path, normalize_path_str,
};
