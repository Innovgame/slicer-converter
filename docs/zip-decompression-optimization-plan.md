# 方案 C：SL1/VDT ZIP 解压优化（交付版）

## 1. 目标

将 SL1 / VDT 这类 ZIP-based 格式的层数据处理，从“主线程逐层解包 + 传输解压后 PNG”改为“主线程解析目录并传输压缩数据 + Worker 原生解压”，以降低主线程阻塞、减少传输体积，并将 ZIP 转换路径收口到 `decode_encode_layer()`。

## 2. 交付结论（2026-04-14）

| 项目                                       | 状态   | 说明                                                                                       |
| ------------------------------------------ | ------ | ------------------------------------------------------------------------------------------ |
| JS 解析 ZIP 目录                           | 已完成 | `apps/slicer-app/src/core/zip.ts` 已实现 `parseZipDirectory()` 与 `getCompressedData()`    |
| 主线程发送压缩层数据到 Worker              | 已完成 | ZIP 路径不再发送解压后的 PNG bytes                                                         |
| Worker 使用原生 `DecompressionStream` 解压 | 已完成 | 已支持 ZIP method `0` / `8`                                                                |
| 前端移除 `extract_zip_entry` 依赖          | 已完成 | 当前前端执行链路不再调用 WASM `extract_zip_entry`                                          |
| ZIP 单元测试                               | 已完成 | `apps/slicer-app/tests/unit/zip.test.ts` 已覆盖最小 ZIP fixture 与异常路径                 |
| ZIP 转换路径收口到 `decode_encode_layer()` | 已完成 | `apps/slicer-app/src/core/zip-worker.ts` 与 `apps/slicer-app/src/worker.ts` 已切到组合调用 |
| ZIP 性能复测与结果回填                     | 未完成 | benchmark 页面已支持 ZIP median 统计，但尚未补录 `sample.sl1` / `sample.vdt` 实测值        |

## 3. 优化前后对比

### 优化前

```text
主线程
1. file.arrayBuffer() -> fileBytes
2. WASM parse_header(fileBytes, extension)
3. WASM extract_zip_entry(fileBytes, entry_index) -> PNG bytes
4. postMessage(PNG bytes) -> Worker

Worker
5. decode_encode_layer(PNG bytes, ...)
```

### 当前已落地实现

```text
主线程
1. file.arrayBuffer() -> fileBytes
2. JS parseZipDirectory(fileBytes)
3. WASM parse_header(fileBytes, extension)
4. getCompressedData(fileBytes, entry) -> compressed bytes
5. postMessage(compressed bytes) -> Worker

Worker
6. DecompressionStream("deflate-raw") -> PNG bytes
7. 预览: decode_layer(PNG, "sl1", ...)
8. 转换: decode_encode_layer(PNG, "sl1", targetFormat, ...)
```

## 4. 当前实现边界

### 4.1 ZIP 目录解析

- 入口文件：`apps/slicer-app/src/core/zip.ts`
- 当前返回结构：
    - `ZipDirectory.entries[]`
    - `ZipEntryInfo.index`
    - `ZipEntryInfo.name`
    - `ZipEntryInfo.compressionMethod`
    - `ZipEntryInfo.compressedSize`
    - `ZipEntryInfo.uncompressedSize`
    - `ZipEntryInfo.localHeaderOffset`
- `getCompressedData()` 通过读取 Local File Header 懒计算 payload offset，并返回零拷贝 `subarray`

### 4.2 Worker ZIP 路径

- Worker 消息类型：
    - `"zip-decode-encode"`
    - `"zip-decode-preview"`
- 当前行为：
    - `compressionMethod === 0`：直接使用原始 payload
    - `compressionMethod === 8`：走 `DecompressionStream("deflate-raw")`
    - 其他 method：直接抛错
- 转换与预览职责：
    - ZIP 转换复用 `convertZipLayerForTarget()`，在浏览器原生解压后调用 `decode_encode_layer()`
    - ZIP 预览复用 `decodeZipPreview()`，继续保持 `decode_layer()` 输出像素
- 约束：
    - 不做 fallback
    - `DecompressionStream` 是 ZIP 路径的运行时前提

### 4.3 主线程行为

- ZIP 格式（`sl1` / `vdt`）加载时会同时：
    - 读取完整文件到 `fileBytes`
    - 用 JS 解析 ZIP 目录
    - 用 WASM 解析 header
    - 将 `header.layers[i].offset` 继续解释为 ZIP entry index
- ZIP benchmark 在主 UI 中仍被显式禁用

### 4.4 WASM 兼容性处理

- `extract_zip_entry` 仍保留在 Rust/WASM 导出中
- 该导出当前不属于前端热路径
- 保留原因是避免破坏其他潜在消费者，而不是继续作为主方案的一部分

### 4.5 浏览器 benchmark 工具

- 入口文件：`apps/slicer-app/src/benchmark-page.ts`
- 当前 ZIP benchmark 行为：
    - 识别 `sl1` / `vdt`
    - 解析 ZIP 目录并提取压缩 payload
    - 复用 `inflateRaw()` 做浏览器原生解压
    - 连续运行 5 次并以中位数展示：
        - `ZIP inflate`
        - `ZIP convert (combined)`
        - `ZIP convert (legacy two-step)`
- 当前输出仍缺手工录入的样本实测值和环境说明

## 5. 预期收益

| 指标                               | 优化前            | 当前实现预期         | 状态                   |
| ---------------------------------- | ----------------- | -------------------- | ---------------------- |
| ZIP 解析次数                       | N 次（每层）      | 1 次                 | 已完成                 |
| 单层 inflate                       | ~5-10ms（WASM）   | ~1-3ms（native）     | 已切换到浏览器原生解压 |
| Worker 传输/层                     | ~3.5MB（PNG raw） | ~400KB（compressed） | 已完成                 |
| 主线程阻塞/层                      | ~3-12ms           | ~0ms                 | 已完成                 |
| `decode_encode_layer` 组合调用收益 | 有潜在收益        | ZIP 路径已收口       | 已完成代码接线         |

说明：上表仍是设计估算，不是最终交付级实测数据。

## 6. 已验证项

### 当前会话已执行命令

- `vitest run --pool=threads tests/unit/zip-worker.test.ts`
- `vitest run --pool=threads tests/unit/benchmark.test.ts`
- `vitest run --pool=threads`
- `pnpm typecheck`

### 当前会话已验证结果

- ZIP worker helper 回归测试通过（3 个测试）
- benchmark helper 回归测试通过（7 个测试）
- 前端 unit tests 全量通过（4 个文件，16 个测试）
- TypeScript 类型检查通过

### 尚未在当前会话复验

- Playwright 聚焦 ZIP 的 E2E
- 浏览器 benchmark 页面上的 `sample.sl1` / `sample.vdt` 5 次中位数实测

## 7. 剩余工作

### 7.1 性能复测与文档回填

目标：

- 在浏览器环境下补 ZIP benchmark
- 对 `sample.sl1` / `sample.vdt` 做多次测量
- 回填最终交付文档中的实测数据与环境说明

建议验收口径：

- 同一样本至少运行 5 次
- 记录中位数（median）
- 若 `combined` 中位数优于 `legacy`，或落在 `legacy * 1.05` 以内，则可视为“无性能回退”

## 8. 风险与约束

| 项目                       | 当前处理                                 |
| -------------------------- | ---------------------------------------- |
| ZIP64 / 分卷 / 加密 ZIP    | 不支持，按 SL1 / VDT 实际输入约束处理    |
| `DecompressionStream` 缺失 | 显式报错，不回退到旧 WASM ZIP 解包路径   |
| ZIP benchmark 数据抖动     | 需要按固定样本、多次运行、中位数口径记录 |

## 9. 交付建议

这份文档可作为“对外说明 / 交付摘要”版本使用；若要继续实施剩余工作，请参考更细的执行计划：

- `docs/internal/superpowers/plans/2026-04-14-zip-optimization-remaining-work.md`
