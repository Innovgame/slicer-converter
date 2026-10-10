# Troubleshooting FAQ

本文集中记录本仓库最常见的本地构建、测试、WASM 与浏览器运行问题。

## `wasm-pack` 未安装或版本不兼容

现象：

- `pnpm build:wasm`
- `pnpm dev`
- `pnpm test:wasm`

报错里出现 `wasm-pack: command not found` 或 `failed to execute wasm-pack`。

处理：

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
wasm-pack --version
```

如果是 CI 或新机器，确认 `wasm-pack` 已进入 `PATH`。

## 缺少 `wasm32-unknown-unknown` target

现象：

- `pnpm build:wasm`
- `pnpm test:wasm`

提示缺少 wasm target。

处理：

```bash
rustup target add wasm32-unknown-unknown
```

## `pnpm build:wasm` 成功但浏览器仍加载旧产物

现象：

- 页面里仍然表现出旧逻辑
- Network 面板里看到旧的 `slicer_converter_bg.wasm`

处理：

1. 重新执行 `pnpm build:wasm`
2. 刷新浏览器缓存，必要时使用 hard reload
3. 确认部署环境没有把 `*.wasm` 长时间缓存成 immutable

仓库里默认 wasm 入口来自 `packages/slicer-wasm/lib/`，而不是单独的外部 CDN。

## Playwright fixture 加载失败

现象：

- `pnpm test:app:e2e` 上传样本后解析失败
- Playwright 报 `rust/tests/fixtures/sample.*` 不存在

处理：

1. 先执行示例生成：

```bash
pnpm fixtures
```

2. 确认文件已生成到 `rust/tests/fixtures/`
3. 重新运行 `pnpm test:app:e2e`

产品 UI 不再暴露内置 fixture shortcut；浏览器回归测试通过真实 file input 上传 fixture。

## Playwright 浏览器未安装

现象：

- `pnpm test:app:e2e`
- `pnpm test:browser`

提示缺少 Chromium 或对应浏览器依赖。

处理：

```bash
pnpm --filter @slicer-converter/slicer-app exec playwright install --with-deps chromium
```

如果后续扩到 Firefox，也用同一命令安装对应 browser。

## `pnpm test:wasm:browser` 失败，提示 ChromeDriver / 浏览器问题

现象：

- browser runner 无法启动
- 提示浏览器驱动不可用或连接失败

说明：

`pnpm test:wasm:browser` 和 `pnpm benchmark:wasm:browser` 依赖本地浏览器环境，比 Node runner 更脆弱。

处理：

1. 优先先跑 `pnpm test:wasm`，确认 wasm contract 本身正常
2. 检查本地 Chrome/Chromium 是否可启动
3. 清理本地浏览器缓存和驱动缓存后重试

如果只是做日常开发验证，优先使用 `pnpm test:wasm`。

## `pnpm --filter @slicer-converter/slicer-app exec vitest run --coverage` 缺 provider

现象：

- 提示 `Cannot find dependency '@vitest/coverage-v8'`

当前仓库已经把 provider 纳入 `apps/slicer-app` 的 devDependencies。如果你在老分支或旧 lockfile 上遇到这个问题，执行：

```bash
pnpm install
```

然后重新运行 coverage 命令。

## `cargo fmt --check` 或 `cargo clippy -D warnings` 失败

现象：

- CI `check` job 失败
- 本地 Rust 测试能过，但静态门禁不过

处理：

```bash
cargo fmt --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
```

建议在提交前把这两条和 `cargo test --manifest-path rust/Cargo.toml` 一起跑。

## 格式识别失败

现象：

- UI 报 `Parse error`
- 或 worker / wasm 错误里带 `E_UNKNOWN_FORMAT`、`E_INVALID_MAGIC`

处理顺序：

1. 确认文件扩展名与真实格式一致
2. 如果是 ZIP 类格式，确认文件不是被二次压缩或损坏
3. 如果是新格式样本，先确认 Rust registry 是否已支持，并且前端 adapter 已同步
4. 查看结构化错误里的 `code`、`detail`、`format`、`layerIndex`

`Phase 1B` 之后，WASM 与 worker 错误会尽量保留结构化字段，不需要再做字符串正则匹配。

## Worker 初始化失败或页面显示 `Worker init failed`

现象：

- 页面启动后直接进入 error 状态
- 不再出现 `Ready - prewarmed 0 worker(s)`，而是明确失败

处理：

1. 打开浏览器控制台查看 worker 初始化错误
2. 确认 wasm 产物已成功构建且可被 worker 访问
3. 确认部署环境没有错误的 base path
4. 如果是临时错误，刷新页面重试；`Phase 1A` 后仅 retryable 路径会自动重试

## 我只想快速验证一轮，应该跑哪些命令

日常最小集合：

```bash
pnpm test:app
pnpm --filter @innovgame/slicer-converter test:unit
cargo test --manifest-path rust/Cargo.toml
```

提交前建议补上：

```bash
pnpm --filter @slicer-converter/slicer-app exec vitest run --coverage
cargo fmt --check --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
```
