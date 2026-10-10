use crate::formats::common::FormatSpec;

use super::{paths, LayerTranscoder, TranscodeRouteKey};

/// Registry mapping route keys to fast-path layer transcoders.
///
/// The registry does not perform fallback; it only returns strategies that the
/// engine may attempt before falling back to decode/encode.
///
/// route key 到快路径 layer transcoder 的 registry。registry 不执行 fallback；它只返回
/// engine 在通用 decode/encode fallback 前可尝试的策略。
pub trait TranscodeRegistry: Sync {
    /// Looks up strategies for a compact route key.
    ///
    /// 查找紧凑 route key 对应的策略列表。
    fn lookup_route(&self, route: TranscodeRouteKey) -> &[&'static dyn LayerTranscoder];
}

/// Static route entry used by the built-in registry.
///
/// `source` and `target` are concrete profiles, not just families. Their compact
/// route key is derived during lookup so the registry can keep the declaration
/// close to the profile facts while the engine dispatches on the narrow key.
///
/// 内置 registry 使用的静态 route entry。
/// `source` 与 `target` 是具体 profile，而不只是 family。lookup 时会从它们推导 compact
/// route key，让 registry declaration 靠近 profile facts，同时 engine 仍按 narrow key
/// dispatch。
#[derive(Clone, Copy)]
pub struct RouteRegistryEntry {
    /// Source profile.
    ///
    /// 源 profile。
    pub source: &'static FormatSpec,
    /// Target profile.
    ///
    /// 目标 profile。
    pub target: &'static FormatSpec,
    /// Fast-path strategies for this source/target pair.
    ///
    /// 该源/目标组合的快路径策略列表。
    pub transcoders: &'static [&'static dyn LayerTranscoder],
}

/// Static transcode registry backed by route entries.
///
/// 由静态 route entries 支撑的 transcode registry。
pub struct StaticTranscodeRegistry {
    route_entries: &'static [RouteRegistryEntry],
}

impl StaticTranscodeRegistry {
    /// Creates a registry from static route entries.
    ///
    /// 从静态 route entries 创建 registry。
    pub const fn new(route_entries: &'static [RouteRegistryEntry]) -> Self {
        Self { route_entries }
    }

    fn lookup_route_direct(
        &self,
        route: TranscodeRouteKey,
    ) -> Option<&'static [&'static dyn LayerTranscoder]> {
        self.route_entries
            .iter()
            .find(|entry| TranscodeRouteKey::from_specs(entry.source, entry.target) == route)
            .map(|entry| entry.transcoders)
    }
}

impl TranscodeRegistry for StaticTranscodeRegistry {
    fn lookup_route(&self, route: TranscodeRouteKey) -> &[&'static dyn LayerTranscoder] {
        self.lookup_route_direct(route).unwrap_or(&[])
    }
}

/// Registry implementation with no fast paths.
///
/// Useful for tests that want to force generic fallback.
///
/// 不包含任何快路径的 registry。适合需要强制走通用 fallback 的测试。
pub struct EmptyTranscodeRegistry;

impl TranscodeRegistry for EmptyTranscodeRegistry {
    fn lookup_route(&self, _route: TranscodeRouteKey) -> &[&'static dyn LayerTranscoder] {
        &[]
    }
}

pub(crate) static DEFAULT_REGISTRY: StaticTranscodeRegistry =
    StaticTranscodeRegistry::new(paths::ROUTE_REGISTRY_ENTRIES);

/// Returns the process-wide built-in transcode registry.
///
/// 返回进程级内置 transcode registry。
pub fn default_registry() -> &'static dyn TranscodeRegistry {
    &DEFAULT_REGISTRY
}
