# 测试体系建设 — 技术规范

> **配套执行计划**：[testing-strategy-plan.md](testing-strategy-plan.md)

## 1. 背景与判断

当前仓库的 Rust 核心测试并不薄弱：

- `rust/src/**` 内已有较多模块级单元测试，覆盖 CTB / GOO / SL1 的 RLE、preview、decoder、config 等纯逻辑。
- `rust/tests/cross_format.rs` 已覆盖 32 个跨格式转换场景。
- `rust/tests/fixture_files.rs` 已覆盖小型 fixture 文件解析和端到端验证。

当前真正缺少的是以下两层：

1. **WASM 边界测试**
   目前 `rust/Cargo.toml` 已引入 `wasm-bindgen-test`，但仓库没有正式的浏览器运行时 WASM API 测试，`parse_header`、`decode_layer`、`encode_layer`、`build_header`、`finalize_header` 等导出函数没有在 JS/WASM 边界上被持续验证。
2. **浏览器端单元 / 集成测试**
   `apps/slicer-app/` 目前没有测试框架依赖，`apps/slicer-app/src/main.ts` 同时承担 DOM 绑定、文件读取、worker 调度、转换编排、benchmark 展示等职责，导致浏览器行为只能靠手工验证。

结论：测试体系建设应以 **补齐 WASM 合约层和浏览器层** 为主，而不是继续堆叠 Rust 核心单测数量。

---

## 2. 目标

建立一套适合当前 monorepo 的分层测试体系，满足以下目标：

1. **快速默认测试**：本地和普通 PR 能在较短时间内跑完核心正确性验证。
2. **边界问题前移**：JS/WASM typed array、JSON、worker message、Blob 组装等问题可通过自动化测试提前暴露。
3. **浏览器主链路可回归**：文件上传、预览、转换、下载等主流程可在真实浏览器环境下验证。
4. **性能验证与正确性解耦**：性能 benchmark 继续保留，但不把不稳定的耗时断言塞进主 CI。

---

## 3. 非目标

以下内容不属于本轮测试体系建设目标：

1. 不引入覆盖率门槛作为第一阶段约束。
2. 不把 `rust/data/` 中的大真实样本纳入默认 CI。
3. 不对 benchmark 的具体毫秒数做硬断言。
4. 不在第一阶段重写现有 Rust 测试结构。

---

## 4. 分层策略

采用四层测试结构：

| 层级                  | 目标                                  | 运行环境         | 建议工具                | 当前状态                              | 本轮动作                      |
| --------------------- | ------------------------------------- | ---------------- | ----------------------- | ------------------------------------- | ----------------------------- |
| Rust 单元测试         | 校验纯算法与格式局部逻辑              | native           | `#[test]`               | 已有较多覆盖                          | 保持现状，少量按需补点        |
| Rust 集成测试         | 校验跨模块转换与小型 fixture 行为     | native           | `rust/tests/*.rs`       | 已有 `cross_format` / `fixture_files` | 保持现状，后续补 fixture 分层 |
| WASM 合约测试         | 校验导出 API 在浏览器运行时下的正确性 | wasm32 + browser | `wasm-bindgen-test`     | 基本缺失                              | 新增                          |
| 浏览器单元 / 集成测试 | 校验 app 逻辑、worker 编排、UI 主链路 | Node + browser   | `Vitest` + `Playwright` | 缺失                                  | 新增                          |

---

## 5. 推荐目录结构

建议目标结构如下：

```text
rust/
├── src/
│   └── ...                      # 保持现有模块单测风格
├── tests/
│   ├── cross_format.rs
│   ├── fixture_files.rs
│   └── wasm_api.rs             # 新增：WASM 导出 API 合约测试
└── examples/
    └── bench_rle.rs

apps/slicer-app/
├── src/
│   ├── main.ts                 # 保留 DOM 绑定与启动逻辑
│   ├── worker.ts
│   ├── types.ts
│   ├── core/
│   │   ├── conversion.ts       # 新增：纯转换编排逻辑
│   │   ├── parsing.ts          # 新增：文件头解析与 metadata 归一化
│   │   ├── benchmark.ts        # 新增：benchmark 计算逻辑
│   │   └── download.ts         # 新增：下载名 / Blob 组装辅助
├── tests/
│   └── unit/
│       ├── benchmark.test.ts   # Vitest 单元测试
│       └── download.test.ts
├── vitest.config.ts
└── tests/
    └── e2e/
        ├── fixture-upload.spec.ts
        ├── convert-download.spec.ts
        └── benchmark.spec.ts
```

原则：

1. **纯逻辑进 `src/core/`**，优先可直接单测。
2. **浏览器交互留在 `main.ts`**，尽量瘦身。
3. **真实浏览器行为放 `tests/e2e/`**，不与纯单测混用。

---

## 6. Fixture 策略

样本分三档：

### 6.1 Tier A：代码内合成样本

用于：

- RLE roundtrip
- header / footer patch
- typed array 边界
- metadata 映射

特点：

- 最快
- 完全可控
- 适合单元测试与 wasm 合约测试

### 6.2 Tier B：小型仓库内样本

建议引入一个独立的测试 fixture 目录作为**源数据**：

```text
rust/tests/fixtures/
├── sample.goo
├── sample.ctb
└── sample.sl1
```

浏览器侧 E2E 通过真实 file input 上传 `rust/tests/fixtures/*`，不再依赖生产静态资源中的样本副本。

用途：

- Rust 集成测试
- 浏览器 E2E smoke tests
- 上传 / 预览 / 转换 / 下载主链路

### 6.3 Tier C：大真实样本

使用现有：

- `rust/data/Untitled_20260413_1125_TI3U.goo`
- `rust/data/converted_real.ctb`

用途：

- 手工 benchmark
- 定位大文件性能问题

约束：

- 不进入默认 `pnpm test`
- 不作为主 CI 必跑项
- 如浏览器测试需要大文件，只作为手工或专项测试，不进入默认回归

---

## 7. WASM 合约测试规范

### 7.1 目标

保证 Rust 导出 API 在浏览器运行时下行为稳定，而不是只在 native Rust 调用路径上正确。

### 7.2 覆盖范围

首批用例至少覆盖：

1. `parse_header` 能解析小型 `sample.goo` / `sample.ctb`。
2. `decode_layer` 返回的 `Uint8Array` 长度与分辨率一致。
3. `encode_layer` + `decode_layer` 在 Goo / CTB 小层数据上 roundtrip 正常。
4. `build_header` + `finalize_header` 能完成 layer size patch。
5. 扩展名提示和 magic detection 在 wasm 环境下一致。

这些用例应优先使用：

- 代码内合成字节数组
- 或 `include_bytes!` 嵌入的 `rust/tests/fixtures/*`

不要依赖 `std::fs::read()` 在浏览器运行时读取样本。

### 7.3 运行方式

建议通过 `wasm-pack test` 维护一套**稳定默认 runner + 可选浏览器 runner**：

默认命令：

```bash
wasm-pack test --node rust
```

可选浏览器命令：

```bash
wasm-pack test --headless --chrome rust
```

原因：

1. 当前这层测试主要校验导出 API、typed array 和 JSON 边界，Node runner 已足够。
2. 浏览器 driver 兼容性问题不应阻塞默认回归。
3. 真正的浏览器行为差异应由后续 Playwright E2E 覆盖。

---

## 8. 前端单元测试规范

### 8.1 原则

前端单测只覆盖 **纯函数和轻状态编排**，不直接在第一阶段测试复杂 DOM 行为。

### 8.2 优先拆分模块

优先从 `apps/slicer-app/src/main.ts` 中拆出：

1. `parsing.ts`
   文件头解析扩容策略、扩展名归一化、metadata 转换。
2. `conversion.ts`
   layer 遍历、结果收敛、进度聚合、main-thread / worker 模式共用逻辑。
3. `benchmark.ts`
   benchmark warmup、layer limit、结果格式化。
4. `download.ts`
   输出文件名、Blob parts 组装辅助。

### 8.3 建议覆盖点

1. 目标格式切换时禁用源格式选项。
2. `formatBytes` / benchmark 展示格式稳定。
3. layer chunk 缺失时能稳定抛错。
4. metadata fallback 逻辑与默认值稳定。
5. benchmark 层数限制逻辑稳定。

---

## 9. 浏览器集成测试规范

### 9.1 目标

验证用户真正会走的主链路，而非函数级细节。

### 9.2 工具

建议使用 `Playwright`：

- 能稳定处理文件上传
- 能驱动真实浏览器
- 能观察下载行为
- 能覆盖 worker / canvas / wasm 这类浏览器特性

### 9.3 首批必备场景

1. **Fixture 上传 smoke**
   通过 file input 上传 `sample.goo`，断言 metadata 与 layer slider 正常出现。
2. **上传 + 预览**
   上传一个 sample 文件，断言 preview canvas 更新。
3. **转换 + 下载**
   从 `sample.goo` 转到 `ctb`，断言下载触发、文件名后缀正确。
4. **benchmark 可执行**
   通过 Playwright benchmark harness 跑转换诊断，断言输出包含转换耗时与 route 统计。

### 9.4 Server 策略

Playwright 配置中应显式声明 `webServer`，避免测试依赖手工先起服务。

建议区分两种模式：

1. **普通 E2E**：启动 `pnpm dev` 或等价预览服务，验证默认浏览器主链路。
2. **benchmark E2E**：使用 `apps/slicer-app/playwright.bench.config.ts` 的专用配置，验证转换诊断和性能观测链路。

### 9.5 约束

1. E2E 测试只使用 Tier B 小型 fixture。
2. 不对 canvas 像素做大面积 snapshot。
3. 不在主 CI 中引入对 benchmark 毫秒数的硬阈值。

---

## 10. 脚本设计

建议根脚本最终收敛为：

```json
{
    "scripts": {
        "test:rust": "cargo test --manifest-path rust/Cargo.toml",
        "test:wasm": "wasm-pack test --headless --chrome rust",
        "test:app": "pnpm --filter @slicer-converter/slicer-app test:unit",
        "test:app:e2e": "pnpm --filter @slicer-converter/slicer-app test:e2e",
        "test": "pnpm typecheck && pnpm test:rust && pnpm test:app",
        "test:browser": "pnpm test:wasm:browser && pnpm test:app:e2e",
        "test:full": "pnpm test && pnpm test:browser"
    }
}
```

原则：

1. `test` 保持快速，适合开发和普通 PR。
2. `test:browser` 用于所有浏览器相关回归验证。
3. `test:full` 用于合并前或 CI 扩展流水线。
4. WASM browser 和 E2E 默认不阻塞高频本地迭代。

---

## 11. CI 建议

建议两级流水线：

### 11.1 PR 必跑

1. `pnpm typecheck`
2. `pnpm test:rust`
3. `pnpm test:app`

### 11.2 扩展流水线 / nightly

1. `pnpm test:wasm`
2. `pnpm test:app:e2e`

这样可以兼顾反馈速度和真实浏览器回归能力。

---

## 12. 风险与约束

| 风险                             | 说明                                 | 缓解方式                               |
| -------------------------------- | ------------------------------------ | -------------------------------------- |
| `main.ts` 过于臃肿导致单测难落地 | 当前 DOM、worker、wasm、下载逻辑耦合 | 先拆 `src/core/` 再补单测              |
| WASM 测试在 CI 环境不稳定        | 浏览器与 wasm runner 依赖较强        | 先在本地 / nightly 跑，后续再纳入主 CI |
| 浏览器 benchmark 结果易波动      | 与机器 / 浏览器状态相关              | 仅验证可执行，不断言耗时阈值           |
| 大样本拖慢流水线                 | `rust/data` 体积和转换成本都高       | 保持只用于手工 benchmark               |

---

## 13. 验收标准

完成第一阶段后，应达到：

1. Rust 原有 68 项测试继续通过。
2. 新增一组可执行的 wasm 合约测试。
3. `apps/slicer-app` 具备基础 Vitest 单测框架。
4. `apps/slicer-app` 具备最少 2-3 条 Playwright 主链路测试。
5. 根脚本具备 `test:rust` / `test:wasm` / `test:app` / `test:app:e2e` / `test:browser` / `test:full` 的分层入口。

---

## 14. 推荐实施顺序

按收益 / 风险比，推荐顺序如下：

1. 先补 **WASM 合约测试**。
2. 再拆前端纯逻辑并补 **Vitest 单测**。
3. 最后补 **Playwright 浏览器集成测试**。

原因：

- WASM 合约层是当前最容易出错、但测试成本最低的一层。
- 浏览器侧如果不先拆 `main.ts`，单元测试会很快变成一堆脆弱 mock。
- E2E 收益大，但应放在前两层稳定后补上。
