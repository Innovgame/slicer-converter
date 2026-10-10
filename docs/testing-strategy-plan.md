# 测试体系建设 — 执行计划

> **前置文档**：[testing-strategy-spec.md](testing-strategy-spec.md)
> **目标**：在不破坏现有 Rust 测试稳定性的前提下，补齐 WASM 边界测试、前端单元测试与浏览器集成测试。

## 1. 当前基线

已存在：

- Rust 核心模块单元测试
- `rust/tests/cross_format.rs`
- `rust/tests/fixture_files.rs`
- 根命令 `pnpm test` = `pnpm typecheck && cargo test --manifest-path rust/Cargo.toml`

缺少：

- 正式 wasm 合约测试入口
- `apps/slicer-app` 的 unit test 框架
- 浏览器端 E2E 测试框架和脚本

---

## 2. 执行策略

采用 **由内向外** 的顺序：

1. 先补 WASM 合约测试，锁住 JS/WASM 边界。
2. 再拆前端纯逻辑，补 Vitest 单测。
3. 最后补 Playwright 浏览器集成测试。

这样可以避免一开始就把重心放到 UI automation，而核心边界仍然裸奔。

---

## 3. Phase 1：补 WASM 合约测试

### 3.1 目标

新增一组能在浏览器运行时下执行的 `wasm-bindgen-test` 用例。

### 3.2 预期改动

文件：

- `rust/tests/fixtures/*`（新增：小型 canonical fixtures）
- `rust/tests/wasm_api.rs` 或等价新文件
- `package.json`（增加 `test:wasm`）
- 如有必要，补充 README 测试命令说明

### 3.3 用例范围

首批只做高价值覆盖：

1. `parse_header` 解析 sample goo / ctb。
2. `decode_layer` 返回长度与分辨率一致。
3. `encode_layer` + `decode_layer` roundtrip。
4. `build_header` + `finalize_header` 能完成 layer size patch。

实现约束：

1. fixture 通过 `include_bytes!` 或代码生成方式引入。
2. 不使用运行时文件系统读取浏览器样本。

### 3.4 验证命令

```bash
wasm-pack test --node rust
```

可选扩展命令：

```bash
wasm-pack test --headless --chrome rust
```

### 3.5 完成标准

1. 本地能稳定跑通 `test:wasm`。
2. 至少覆盖 4 个关键导出 API。

---

## 4. Phase 2：拆前端纯逻辑并补 Vitest

### 4.1 目标

把 `apps/slicer-app/src/main.ts` 中可纯测的逻辑拆出去，建立前端 unit test 入口。

### 4.2 预期改动

新增 / 调整目录：

```text
apps/slicer-app/src/core/
├── conversion.ts
├── parsing.ts
├── benchmark.ts
└── download.ts
```

新增测试文件：

```text
apps/slicer-app/tests/unit/*.test.ts
apps/slicer-app/vitest.config.ts
```

调整文件：

- `apps/slicer-app/package.json`
- 根 `package.json`
- 可能新增 `vitest.config.ts`

### 4.3 推荐拆分顺序

1. `benchmark.ts`
   最纯，依赖最少，适合先落 Vitest。
2. `download.ts`
   测文件名和 parts 组装策略。
3. `parsing.ts`
   测扩容读取、fallback、格式归一化。
4. `conversion.ts`
   测 layer chunk 收敛、错误传播、进度聚合。

### 4.4 建议脚本

```json
{
    "scripts": {
        "test:unit": "vitest run"
    }
}
```

### 4.5 完成标准

1. `apps/slicer-app` 拥有独立 unit test 入口。
2. 至少补 6-10 个纯逻辑单元测试。
3. `main.ts` 明显变瘦，不再承载全部纯逻辑。

---

## 5. Phase 3：补浏览器集成测试

### 5.1 目标

建立基于真实浏览器的 smoke 和主链路测试。

### 5.2 工具

推荐：

- `@playwright/test`

### 5.3 预期改动

新增：

```text
apps/slicer-app/tests/e2e/
├── fixture-upload.spec.ts
├── convert-download.spec.ts
└── benchmark.spec.ts
```

以及：

- `playwright.config.ts`
- `apps/slicer-app/package.json` 脚本

### 5.4 首批用例

1. Fixture 上传：通过 file input 上传 sample.goo fixture，断言 metadata 出现。
2. 预览显示：断言 preview 区块和 slider 可用。
3. 转换下载：断言下载被触发，后缀为目标格式。
4. benchmark：通过 Playwright benchmark harness 验证转换诊断可执行。

### 5.5 验证方式

执行：

```bash
pnpm --filter @slicer-converter/slicer-app test:e2e
```

正式落地时，应把这一步收口到 Playwright `webServer` 配置里，不要求人工先起服务。

### 5.6 完成标准

1. 至少 3 条主链路测试可稳定运行。
2. 不依赖大真实样本。
3. 下载与 benchmark 行为均有自动化回归。

---

## 6. Phase 4：整理根脚本与 CI 分层

### 6.1 目标

让测试命令具备明确层次，不再只有一个 `pnpm test`。

### 6.2 目标脚本

根 `package.json` 目标状态：

```json
{
    "scripts": {
        "test:rust": "cargo test --manifest-path rust/Cargo.toml",
        "test:wasm": "wasm-pack test --node rust",
        "test:wasm:browser": "wasm-pack test --headless --chrome rust",
        "test:app": "pnpm --filter @slicer-converter/slicer-app test:unit",
        "test:app:e2e": "pnpm --filter @slicer-converter/slicer-app test:e2e",
        "test": "pnpm typecheck && pnpm test:rust && pnpm test:app",
        "test:browser": "pnpm test:wasm:browser && pnpm test:app:e2e",
        "test:full": "pnpm test && pnpm test:browser"
    }
}
```

> **实施说明**：`test:wasm` 默认使用 `--node` runner（更快更稳定），`test:wasm:browser` 保留浏览器 runner 选项。

### 6.3 CI 分层（已实施）

`.github/workflows/ci.yml` — 4 个 job：

| Job       | 依赖  | 内容                                                 |
| --------- | ----- | ---------------------------------------------------- |
| **check** | —     | typecheck → Rust tests → app unit tests              |
| **lint**  | —     | ESLint + Prettier format check                       |
| **wasm**  | check | wasm-pack test --node                                |
| **e2e**   | check | Build WASM → install Playwright Chromium → E2E tests |

PR 必跑（并行）：

1. `check`：typecheck + Rust tests + app unit tests
2. `lint`：ESLint + Prettier

通过后扩展：

1. `wasm`：WASM 合约测试
2. `e2e`：Playwright 浏览器 E2E 测试

---

## 7. 落地顺序与检查点

### Step 1：WASM 合约测试骨架

检查点：

- `test:wasm` 可运行
- 关键导出 API 有最小覆盖

### Step 2：引入 Vitest

检查点：

- `apps/slicer-app` 能独立跑 `test:unit`
- 至少一个 `tests/unit/*.test.ts` 跑通

### Step 3：拆 `main.ts`

检查点：

- `main.ts` 中纯逻辑减少
- 新模块测试通过

### Step 4：引入 Playwright

检查点：

- Fixture 上传 / 转换下载 smoke test 跑通

### Step 5：根脚本和 CI 收口

检查点：

- `pnpm test`
- `pnpm test:browser`
- `pnpm test:full`
- 文档说明同步完成

---

## 8. 风险控制

| 风险                                 | 发生阶段 | 应对                                  |
| ------------------------------------ | -------- | ------------------------------------- |
| `main.ts` 太大导致拆分过程引入回归   | Phase 2  | 每拆一个模块就先补对应测试，再回接 UI |
| wasm browser runner 在某些环境不可用 | Phase 1  | 先保证本地可跑，再决定是否进入默认 CI |
| Playwright 启动成本高                | Phase 3  | 只保留少量高价值主链路测试            |
| benchmark 测试波动                   | Phase 3  | 只验证可执行，不断言耗时阈值          |

---

## 9. 推荐第一批实施范围

如果只做第一轮最有价值的工作，建议压缩为：

1. 新增 `test:wasm`
2. 新增 `rust/tests/wasm_api.rs`
3. 引入 Vitest
4. 从 `main.ts` 中仅拆 `benchmark.ts` 和 `download.ts`
5. 新增 2 条 Playwright smoke tests

这样能最快把当前最薄弱的三层边界补上，同时把工作量控制在可 review 范围内。

---

## 10. 完成标志

当以下条件同时满足时，可认为第一阶段测试体系建设完成：

1. Rust 现有测试保持全绿。
2. `test:wasm` 可执行且有稳定用例。
3. `test:app` 可执行且覆盖已拆出的纯逻辑模块。
4. `test:app:e2e` 至少覆盖 fixture 上传和转换下载。
5. README 或相关文档已说明测试分层入口。

---

## 11. 实施记录

| Phase                    | 状态    | 提交      | 备注                                                |
| ------------------------ | ------- | --------- | --------------------------------------------------- |
| Phase 1：WASM 合约测试   | ✅ 完成 | `caa49ba` | 5 条 wasm-bindgen-test，使用 `--node` runner        |
| Phase 2：Vitest 单元测试 | ✅ 完成 | `d2652dc` | 抽取 benchmark.ts / download.ts，9 条单测           |
| 测试目录重构             | ✅ 完成 | `25d0db1` | 测试文件移至 `tests/unit/`                          |
| Phase 3：Playwright E2E  | ✅ 完成 | `d0f6273` | 8 条 E2E 测试，使用系统 Chrome（channel: 'chrome'） |
| Phase 4：CI / 脚本收口   | ✅ 完成 | `70ae868` | GitHub Actions 4-job CI：check / lint / wasm / e2e  |
