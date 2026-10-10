# Slicer-Converter 项目状态更新

> **更新日期**：2026-04-15  
> **更新目标**：明确哪些事项已完成、哪些未完成、哪些暂不推进

---

## 一、结论速览

| 状态            | 数量 | 说明                          |
| --------------- | ---: | ----------------------------- |
| 已完成          |   14 | 关键性能与工程化项已落地      |
| 未完成（待做）  |    1 | 批量转换/历史记录             |
| 暂不做/低优先级 |    3 | 实施复杂度高或短期 ROI 不明显 |

---

## 二、已完成事项

| 事项                                       | 状态   | 依据                                                                                                                                             |
| ------------------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 层预读队列（prefetch）                     | 已完成 | `createLayerPrefetch(...)` 已接入转换控制器（`apps/slicer-app/src/core/layer-prefetch.ts`、`apps/slicer-app/src/core/conversion-controller.ts`） |
| 输出流式写入能力（File System Access API） | 已完成 | `OutputSink` + `openStreamingSink` 已实现（`apps/slicer-app/src/core/streaming-writer.ts`）                                                      |
| 二进制输出统一装配抽象                     | 已完成 | `assembleBinaryOutput(...)` 通过 `OutputSink` 写入（`apps/slicer-app/src/core/output-assembler.ts`）                                             |
| ZIP 输出接入可流式 sink                    | 已完成 | `assembleZipArchive(entries, sink)` 已支持 sink 输出（`apps/slicer-app/src/core/zip-archive.ts`）                                                |
| Rust release 优化参数（`opt-level = 3`）   | 已完成 | `rust/Cargo.toml`                                                                                                                                |
| wasm-opt 优化开启（`-O3 --all-features`）  | 已完成 | `rust/Cargo.toml`                                                                                                                                |
| CI 分层执行（check/lint/wasm/e2e）         | 已完成 | `.github/workflows/ci.yml`                                                                                                                       |
| 动态预读窗口（替代固定 `workers+2`）       | 已完成 | `createLayerPrefetch` 支持自适应窗口增减（`apps/slicer-app/src/core/layer-prefetch.ts`）                                                         |
| ZIP 中央目录内存占用优化                   | 已完成 | 移除 `centralParts` 字节块缓存，改为轻量描述符后逐条写出（`apps/slicer-app/src/core/zip-archive.ts`）                                            |
| benchmark 指标纳入 CI 门禁                 | 已完成 | 新增 `test:benchmark:ci` 与阈值脚本并接入 CI（`scripts/check-benchmark-threshold.mjs`、`.github/workflows/ci.yml`）                              |
| 拖拽上传（drag & drop）                    | 已完成 | 上传区已支持拖拽事件并调用 `onFileSelect`（`apps/slicer-app/src/app/components/file-summary-card.tsx`）                                          |
| WASM 加载进度提示                          | 已完成 | WASM 初始化阶段通过 `onProgress` 实时更新状态文案（`apps/slicer-app/src/app/hooks/use-converter-controller.ts`）                                 |
| 转换失败后一键重试                         | 已完成 | `ConvertPanel` 在 `status.type === "error"` 时渲染重试按钮（`apps/slicer-app/src/app/components/convert-panel.tsx`）                             |
| 公开 API 文档注释（TSDoc/Rust doc）        | 已完成 | 为 `packages/slicer-wasm/src` 所有公开接口/函数及 `rust/src/lib.rs`、`rust/src/common/metadata.rs` 补充中英双语注释                              |

---

## 三、未完成事项（待做）

| 事项              | 当前状态 | 建议优先级 |
| ----------------- | -------- | :--------: |
| 批量转换/转换历史 | 尚未实现 |     低     |

---

## 四、暂不做或低优先级事项

| 事项                                     | 暂缓原因                                   |
| ---------------------------------------- | ------------------------------------------ |
| SharedArrayBuffer 并行共享内存方案       | 需要 COOP/COEP 与部署策略配套，改造成本高  |
| 运行时插件化格式扩展机制                 | 当前项目规模下收益有限，静态注册足够       |
| 解码路径深度 SIMD 化（尤其状态机型解码） | 实现复杂、收益不稳定，建议基准驱动后再评估 |

---

## 五、建议执行顺序（下一阶段）

> **当前优先落地项（已确认）**：目前专项任务均已完成，剩余低优先级项

1. 批量转换/转换历史（低优先级，实现成本较高）。

---

## 六、附：本次核对基线

- 本地执行 `pnpm test`：通过（174/174）
- 本地统计：Rust 5189 行、TypeScript 4660 行、测试文件 29 个
