//! Built-in fast-path route registrations.
//!
//! Each route entry maps a concrete source/target profile pair to one or more
//! payload-level transcoders. The engine still owns fallback behavior; this
//! module only declares which direct strategies are worth trying first.
//!
//! 内置快路径 route 注册表。每个 route entry 将具体源/目标 profile 组合映射到一个或多个
//! payload-level transcoder。fallback 行为仍由 engine 负责；本模块只声明哪些直接策略值得先尝试。

pub mod ctb_to_goo;
mod goo_family_to_ctb;
pub mod goo_to_ctb;
pub mod prz_to_ctb;

use crate::formats::{chitubox::specs::CTB_V3_SPEC, goo::specs::GOO_SPEC};

use super::{registry::RouteRegistryEntry, LayerTranscoder};

static GOO_FAMILY_TO_CTB_TRANSCODERS: [&'static dyn LayerTranscoder; 1] =
    [&goo_family_to_ctb::GOO_FAMILY_TO_CTB_TRANSCODER];

static CTB_TO_GOO_TRANSCODERS: [&'static dyn LayerTranscoder; 1] =
    [&ctb_to_goo::CTB_TO_GOO_TRANSCODER];

/// Static fast-path registrations used by the default transcode registry.
///
/// Route keys are derived from these concrete profiles at lookup time, so the
/// entries double as documentation for currently supported direct layer routes.
///
/// 默认 transcode registry 使用的静态快路径注册。lookup 时会从这些具体 profile 推导 route key，
/// 因此这些 entry 也记录了当前支持的直接 layer route。
pub static ROUTE_REGISTRY_ENTRIES: &[RouteRegistryEntry] = &[
    RouteRegistryEntry {
        source: &GOO_SPEC,
        target: &CTB_V3_SPEC,
        transcoders: &GOO_FAMILY_TO_CTB_TRANSCODERS,
    },
    RouteRegistryEntry {
        source: &CTB_V3_SPEC,
        target: &GOO_SPEC,
        transcoders: &CTB_TO_GOO_TRANSCODERS,
    },
];
