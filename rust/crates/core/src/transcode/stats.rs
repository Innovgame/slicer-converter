use std::collections::BTreeMap;

use serde::Serialize;

use super::route::TranscodeRouteKey;

/// Per-layer transcode decision counters.
///
/// Returned by the engine so callers can distinguish direct fast-path hits from
/// generic decode/encode fallback without parsing logs.
///
/// 单层转码决策计数。engine 返回它，让调用方无需解析日志即可区分快路径命中与通用
/// decode/encode fallback。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LayerTranscodeStats {
    /// Number of fast-path strategies attempted.
    ///
    /// 尝试过的快路径策略数量。
    pub strategy_attempts: u32,
    /// Number of strategies that cleanly declined.
    ///
    /// 正常拒绝的策略数量。
    pub strategy_declines: u32,
    /// Whether a fast path produced the target payload.
    ///
    /// 是否由快路径产出目标 payload。
    pub fast_path_applied: bool,
    /// Whether generic fallback was used.
    ///
    /// 是否使用了通用 fallback。
    pub fallback_used: bool,
}

/// Aggregate stats for one route key.
///
/// These counters are useful for route-level hit-rate summaries in CLI/bench
/// output without exposing the internal `BTreeMap` used by [`TranscodeStats`].
///
/// 单个 route key 的聚合统计。
/// 这些计数用于 CLI/bench 输出 route-level hit-rate summary，同时不暴露 [`TranscodeStats`]
/// 内部使用的 `BTreeMap`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteTranscodeStats {
    /// Total layers processed for this route.
    ///
    /// 该 route 已处理的总层数。
    pub layers_total: u64,
    /// Layers completed by a fast path.
    ///
    /// 由快路径完成的层数。
    pub fast_path_hits: u64,
    /// Layers completed by generic fallback.
    ///
    /// 由通用 fallback 完成的层数。
    pub fallback_hits: u64,
    /// Total strategy attempts across layers.
    ///
    /// 跨层累计策略尝试次数。
    pub strategy_attempts: u64,
    /// Total clean strategy declines across layers.
    ///
    /// 跨层累计策略正常拒绝次数。
    pub strategy_declines: u64,
}

impl RouteTranscodeStats {
    fn record_layer(&mut self, layer: LayerTranscodeStats) {
        self.layers_total += 1;
        self.strategy_attempts += layer.strategy_attempts as u64;
        self.strategy_declines += layer.strategy_declines as u64;
        if layer.fast_path_applied {
            self.fast_path_hits += 1;
        }
        if layer.fallback_used {
            self.fallback_hits += 1;
        }
    }

    fn merge(&mut self, other: Self) {
        self.layers_total += other.layers_total;
        self.fast_path_hits += other.fast_path_hits;
        self.fallback_hits += other.fallback_hits;
        self.strategy_attempts += other.strategy_attempts;
        self.strategy_declines += other.strategy_declines;
    }
}

/// Aggregate transcode stats across all routes.
///
/// route_breakdown stays private so callers read it through stable iterator and
/// lookup helpers.
///
/// 跨所有 route 的聚合转码统计。route_breakdown 保持私有，调用方通过稳定 iterator 与 lookup helper
/// 读取。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranscodeStats {
    /// Total layers processed.
    ///
    /// 已处理总层数。
    pub layers_total: u64,
    /// Layers completed by fast paths.
    ///
    /// 由快路径完成的层数。
    pub fast_path_hits: u64,
    /// Layers completed by generic fallback.
    ///
    /// 由通用 fallback 完成的层数。
    pub fallback_hits: u64,
    /// Total fast-path strategy attempts.
    ///
    /// 快路径策略累计尝试次数。
    pub strategy_attempts: u64,
    /// Total clean fast-path strategy declines.
    ///
    /// 快路径策略累计正常拒绝次数。
    pub strategy_declines: u64,
    route_breakdown: BTreeMap<TranscodeRouteKey, RouteTranscodeStats>,
}

impl TranscodeStats {
    /// Records one layer result under a route key.
    ///
    /// 在指定 route key 下记录单层结果。
    pub fn record_layer_for_route(&mut self, route: TranscodeRouteKey, layer: LayerTranscodeStats) {
        self.layers_total += 1;
        self.strategy_attempts += layer.strategy_attempts as u64;
        self.strategy_declines += layer.strategy_declines as u64;
        if layer.fast_path_applied {
            self.fast_path_hits += 1;
        }
        if layer.fallback_used {
            self.fallback_hits += 1;
        }
        self.route_breakdown
            .entry(route)
            .or_default()
            .record_layer(layer);
    }

    /// Returns aggregate stats for a route when present.
    ///
    /// 返回指定 route 的聚合统计。
    pub fn route_stats(&self, route: TranscodeRouteKey) -> Option<RouteTranscodeStats> {
        self.route_breakdown.get(&route).copied()
    }

    /// Iterates route breakdown in deterministic key order.
    ///
    /// 以确定性 key 顺序遍历 route breakdown。
    pub fn routes(&self) -> impl Iterator<Item = (TranscodeRouteKey, RouteTranscodeStats)> + '_ {
        self.route_breakdown
            .iter()
            .map(|(route, stats)| (*route, *stats))
    }

    /// Merges another stats value into this one.
    ///
    /// 将另一个 stats 值合并到当前值。
    pub fn merge(&mut self, other: Self) {
        self.layers_total += other.layers_total;
        self.fast_path_hits += other.fast_path_hits;
        self.fallback_hits += other.fallback_hits;
        self.strategy_attempts += other.strategy_attempts;
        self.strategy_declines += other.strategy_declines;
        for (route, stats) in other.route_breakdown {
            self.route_breakdown.entry(route).or_default().merge(stats);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayerTranscodeStats, TranscodeRouteKey, TranscodeStats};

    fn route_key(source: &str, target: &str) -> TranscodeRouteKey {
        let registry = crate::formats::global_registry();
        let source = registry
            .spec_by_name(source)
            .unwrap_or_else(|| panic!("{source} spec"));
        let target = registry
            .spec_by_name(target)
            .unwrap_or_else(|| panic!("{target} spec"));
        TranscodeRouteKey::from_specs(source, target)
    }

    #[test]
    fn record_layer_updates_aggregate_and_route_stats() {
        let mut stats = TranscodeStats::default();
        let goo_to_ctb = route_key("goo", "ctb");

        stats.record_layer_for_route(
            goo_to_ctb,
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 0,
                fast_path_applied: true,
                fallback_used: false,
            },
        );
        stats.record_layer_for_route(
            goo_to_ctb,
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 1,
                fast_path_applied: false,
                fallback_used: true,
            },
        );

        assert_eq!(stats.layers_total, 2);
        assert_eq!(stats.fast_path_hits, 1);
        assert_eq!(stats.fallback_hits, 1);
        assert_eq!(stats.strategy_attempts, 2);
        assert_eq!(stats.strategy_declines, 1);

        let route_stats = stats
            .route_stats(goo_to_ctb)
            .expect("route stats should exist");
        assert_eq!(route_stats.layers_total, 2);
        assert_eq!(route_stats.fast_path_hits, 1);
        assert_eq!(route_stats.fallback_hits, 1);
    }

    #[test]
    fn merge_combines_route_breakdown_across_routes() {
        let mut left = TranscodeStats::default();
        let goo_to_ctb = route_key("goo", "ctb");
        let ctb_to_goo = route_key("ctb", "goo");
        left.record_layer_for_route(
            goo_to_ctb,
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 0,
                fast_path_applied: true,
                fallback_used: false,
            },
        );

        let mut right = TranscodeStats::default();
        right.record_layer_for_route(
            goo_to_ctb,
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 1,
                fast_path_applied: false,
                fallback_used: true,
            },
        );
        right.record_layer_for_route(
            ctb_to_goo,
            LayerTranscodeStats {
                strategy_attempts: 1,
                strategy_declines: 0,
                fast_path_applied: true,
                fallback_used: false,
            },
        );

        left.merge(right);

        let routes = left.routes().collect::<Vec<_>>();
        assert_eq!(routes.len(), 2);

        let goo_route = left.route_stats(goo_to_ctb).expect("goo route stats");
        assert_eq!(goo_route.layers_total, 2);
        assert_eq!(goo_route.fast_path_hits, 1);
        assert_eq!(goo_route.fallback_hits, 1);

        let ctb_route = left.route_stats(ctb_to_goo).expect("ctb route stats");
        assert_eq!(ctb_route.layers_total, 1);
        assert_eq!(ctb_route.fast_path_hits, 1);
        assert_eq!(ctb_route.fallback_hits, 0);
    }
}
