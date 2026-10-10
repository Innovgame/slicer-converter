# Deployment Runbook

本文记录 `slicer-converter` 浏览器应用的部署方式、产物位置、缓存策略和 base path 注意事项。

## 目标产物

部署浏览器应用时，核心静态产物来自两个位置：

- `packages/slicer-wasm/lib/`
  wasm-pack 生成的 `*.wasm` 与 JS glue
- `apps/slicer-app/dist/`
  Vite 构建出的最终静态站点

常规部署入口：

```bash
pnpm install
pnpm build:app
```

`pnpm build:app` 会先触发 `prebuild`，因此会先构建 wasm，再产出 app 静态文件。

## GitHub Pages

仓库内置了 Pages workflow：

- [deploy.yml](../.github/workflows/deploy.yml)

站点地址：`https://innovgame.github.io/slicer-converter/`

关键点：

- 仓库 Settings → Pages → Source 选择 GitHub Actions
- 部署工作流设置 `GITHUB_PAGES=true`，Vite 使用 `base: "/slicer-converter/"`
- Pages job 上传的是 `apps/slicer-app/dist/`
- 不需要单独手工搬运 `packages/slicer-wasm/lib/`，因为它已经被打进最终构建产物引用链
- `master` 上的 [CI](../.github/workflows/ci.yml) 成功后会自动部署这次通过的提交
- 也可以在 Actions 里手动运行 `Deploy to GitHub Pages`

自动部署只跟随 `master` 的 push 或手动触发的成功 CI，不会在 pull request 的 CI 结束后发布。

## 非 Pages 静态托管

适用场景：

- Nginx
- Caddy
- Cloudflare Pages
- Netlify
- 任意对象存储静态站点

部署步骤：

```bash
pnpm install
pnpm build:app
```

然后发布：

- 整个 `apps/slicer-app/dist/` 目录

不要只上传 HTML，必须带上同目录中的 JS、CSS、worker chunk 和 wasm 资源。

## 自定义 base path

当前 Vite 配置：

- 本地默认 `/`
- `GITHUB_PAGES=true` 时切到 `/slicer-converter/`

见：

- [vite.config.ts](../apps/slicer-app/vite.config.ts)

如果你部署到自定义子路径，例如 `/tools/slicer-converter/`，需要在构建前设置合适的 `base`，否则会出现：

- 页面能打开，但 JS/wasm 404
- worker 初始化失败

建议做法：

1. 根据部署目标调整 `apps/slicer-app/vite.config.ts`
2. 重新执行 `pnpm build:app`
3. 在浏览器 Network 面板确认 JS、worker、wasm 路径都正确

## Cache Header 建议

推荐策略：

- HTML：`no-cache` 或较短缓存
- JS/CSS/静态 chunk：允许缓存
- `*.wasm`：允许缓存，但发布新版本时要确保 URL 或缓存策略能让客户端拿到新文件

原因：

- HTML 负责引用最新构建产物
- wasm 如果被长期错误缓存，容易出现“页面更新了，但 worker 还在跑旧逻辑”

如果你遇到“构建成功但页面表现仍像旧版本”，先检查 wasm 缓存。

## 运行时健康检查

部署后建议做以下检查：

1. 打开首页，确认应用能加载
2. 打开控制台，确认没有 worker / wasm 初始化错误
3. 选择一个本地文件，确认可解析头部
4. 触发一次 preview，确认 worker 协议正常
5. 执行一次转换，确认下载链路正常

## 常见部署故障

### 页面空白或脚本 404

通常是 base path 不匹配。

先检查：

- 页面 HTML 引用的静态资源 URL
- 实际部署子路径
- `apps/slicer-app/vite.config.ts` 中的 `base`

### worker 初始化失败

通常原因：

- wasm/worker 资源 404
- 站点把 JS 或 wasm 以错误 MIME 类型返回
- 缓存中仍有旧 wasm

## 推荐的部署前检查命令

```bash
pnpm build:app
pnpm test:app
pnpm --filter @innovgame/slicer-converter test:unit
cargo test --manifest-path rust/Cargo.toml
```

如果要严格对齐当前 CI：

```bash
pnpm --filter @slicer-converter/slicer-app exec vitest run --coverage
cargo fmt --check --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
```
