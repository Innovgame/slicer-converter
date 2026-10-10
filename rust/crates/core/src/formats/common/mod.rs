//! Shared format-kernel contracts.
//!
//! This module contains the small traits and descriptors that every concrete
//! slicer format wires together: profile metadata, binary/archive layout,
//! layer codec, crypto, footer, and pipeline orchestration.
//!
//! 这里是格式内核的公共契约层。各具体 slicer format 会把 profile metadata、binary/archive
//! layout、layer codec、crypto、footer 和 pipeline orchestration 这些小型契约组合起来。

pub mod archive;
pub mod binary;
pub mod capabilities;
pub mod composed;
pub mod context;
pub mod crypto;
pub mod footer;
pub mod keys;
pub mod pipeline;
pub mod profile;
pub mod profile_kit;
pub mod resolver;
pub mod spec;

pub(crate) use archive::{
    archive_entry_descriptor, bytes_archive_entry, collect_zip_entries, contains_archive_marker,
    contains_archive_markers, decode_png_thumbnail, encode_png_thumbnail, has_zip_magic,
    json_archive_entry, read_optional_zip_bytes, read_optional_zip_text, read_required_zip_text,
    read_zip_entry_bytes, sorted_layer_offsets, text_archive_entry,
};
pub use archive::{
    ArchiveEncodeContext, ArchiveEntry, ArchiveEntryDescriptor, ArchiveLayout, NoArchiveLayout,
    ZipEntryInfo,
};
pub use capabilities::FormatCapabilities;
pub use composed::{HeaderLayout, LayerCodec};
pub use context::{LayerDecodeContext, LayerEncodeContext, LayerOptions};
pub use crypto::{CryptoPolicy, NoCrypto};
pub use footer::{FooterPolicy, NoFooter};
pub use keys::FormatKey;
pub use pipeline::{
    DefaultArchivePipeline, DefaultBinaryPipeline, DocumentEncodeStats, EncodedLayerStream,
    FormatPipeline, LayerSource, OutputSink, ParsedDocument, VecOutputSink,
};
pub use profile::{ContainerKind, FamilyIdentity, FamilyProfile, FormatCatalog, FormatFamily};
pub use profile_kit::{
    format_spec_from_profile_parts, ProfileCatalogDescriptor, ProfileIdentityDescriptor,
    RuntimeProfileWiring,
};
pub(crate) use resolver::{ensure_export_capable, ExportTargetRequest};
pub use resolver::{
    ExportTargetOptions, HintScopedProfileResolver, ProfileResolver, SimpleProfileResolver,
};
pub use spec::{FormatFamilySpec, FormatIdentity, FormatKeys, FormatSpec};
