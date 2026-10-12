# 转换可靠性修复与验证

基线：`cbfcc5c`。此次实现覆盖项目审查中已获同意的六项问题。

## 实现

- CTB 解析先检查图层表范围再分配；完整二进制输入单独校验层数和 payload 范围。浏览器完整文件打开执行同类校验，ZIP source 拒绝截断读取和 stored 大小不匹配。
- 转换取消统一清理 Worker 消息监听器，终止并从共享池移除仍在执行的转换 Worker。已完成的 Worker 和正在预览的 Worker 不受该清理影响。Worker 池可补充替代 Worker，初始化请求串行化，池终止后丢弃迟到的就绪结果。
- 转换在预读、打开、组装、输出完成和文档清理边界检查取消；清理失败保留原始转换错误。
- 文件选择使用请求序号保护状态提交；旧解析失败和成功不会覆盖最新选择，替换、淘汰和卸载时关闭文档。
- 两种 WASM 初始化入口自动同步 Rust 格式目录；高层转换共享同一初始化边界。CI 用真实 `dist` 与 bundled WASM 分别验证默认初始化和进度初始化。
- CLI 测试通过独立临时目录隔离同名文件，测试线程退出时清理目录。
- benchmark 脚本和 Rust example 正确读取 `rust/data/`，支持 `BENCH_RLE_INPUT`；缺少必需样本时失败。新增浏览器 GOO → CTB → GOO smoke 被常规 CI 的 Chromium/Firefox 项目收集，检查层数、逐层像素和关键打印参数。

## 边界决策

`parse_document` 仍被现有调用方用于 header 前缀读取。完整二进制验证因此使用显式 `ParsedDocument.validate_complete(file_size)`；CLI 在读取完整文件后调用它。Archive 的层偏移属于解压数据布局，不能按 ZIP 原始文件大小做二进制 payload 校验。

取消同步 WASM 计算时采用终止和替换正在执行的 Worker，避免旧计算继续占用资源或把结果交给下一次任务。输出最终提交若已开始，外部文件流是否能回滚由 sink 决定；此时收到取消会拒绝任务结果，不报告转换成功。

## 验证结果

| 验证                                                         | 结果                                                                                           |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| `pnpm check`                                                 | 通过                                                                                           |
| 默认并行 `pnpm test`                                         | 通过：Rust 537，package 240，app 139，scripts 19                                               |
| Rust fmt、Clippy all-targets/all-features，warnings 为 error | 通过                                                                                           |
| `pnpm build`                                                 | release WASM、TS package 和生产页面构建通过                                                    |
| `pnpm test:wasm`                                             | WASM Node 24 项通过；browser-only suite 不在此命令中执行                                       |
| `node scripts/smoke-package.mjs` / `progress`                | 两种初始化入口的真实包 smoke 通过                                                              |
| benchmark gate                                               | 实际执行：4.32 ms/层，阈值 25 ms/层                                                            |
| 独立代码审阅                                                 | 发现的三个取消时序边界均由先失败后通过的行为测试修复；无遗留 Critical/Important/Minor 代码发现 |
| 浏览器 roundtrip smoke                                       | Chromium/Firefox 均被 CI 模式收集；本地运行受环境限制，未执行浏览器用例                        |

测试先复现后修复：截断图层表、异常 layer count、完整 payload 越界、ZIP 截断、已取消 signal、读取中取消、预读同步取消、旧消息、异步输出取消、输出完成取消、文档清理取消、旧文件结果、卸载、格式目录初始化、临时文件隔离、必需 benchmark 样本缺失、Worker 初始化竞争。

## 真实样本派生与语义验证

源：`rust/data/Untitled_20260413_1125_TI3U.goo`，914 层，8520×4320。

```sh
cargo build --manifest-path rust/Cargo.toml -p slicer-converter-cli --release --offline
rust/target/release/slicer-converter --input rust/data/Untitled_20260413_1125_TI3U.goo --output /tmp/slicer-reliability-validation/real.ctb --threads 4 --timing
rust/target/release/slicer-converter --input /tmp/slicer-reliability-validation/real.ctb --output /tmp/slicer-reliability-validation/roundtrip.goo --threads 4 --timing
```

两次输出均经真实 WASM 解析与反向转换验证，保留 914 层和关键 metadata。对第 0、457、913 层逐像素比较，每层 36,806,400 像素，结果符合 CTB gray7 量化规则：`gray7 = source >> 1; expected = (gray7 << 1) | (gray7 >> 6)`。这是抽样层语义验证，不代表真实文件每一层都作了像素比较。新增小样本浏览器测试则比较所有层。

| 路径      | CLI 内部总耗时 | Fast-path / fallback | 峰值 RSS                      |
| --------- | -------------- | -------------------- | ----------------------------- |
| GOO → CTB | 15.01 ms       | 914 / 0，100% 命中   | 68,354,048 bytes，约 65.2 MiB |
| CTB → GOO | 11.86 ms       | 914 / 0，100% 命中   | 66,043,904 bytes，约 63.0 MiB |

上述是本机 release CLI 单次运行结果，内部 timing 不含进程启动；峰值 RSS 由 Python `resource.getrusage(RUSAGE_CHILDREN)` 获取。它与 benchmark gate 的全像素 decode/encode 口径不同，不用于互相比较。日志和派生文件都位于 `/tmp/slicer-reliability-validation/`。

截断 CTB 的 CLI 回归也经 release 二进制验证：返回错误且未创建 GOO 输出。

## 环境限制

Vite 监听 `localhost:8080` 被当前沙箱拒绝。请求沙箱外执行时，自动审批系统因审批模型账户配置错误（404：`gpt-5.6-luna` 不受支持）未能完成审批，测试没有执行；这不是对测试操作安全性的否定。没有绕过审批。

恢复本地监听权限或在 CI 中运行 `pnpm test:app:e2e` 可执行新增 smoke。本次不能声称实际 Chromium/Firefox 用例通过，也没有进行物理打印或外部 UVTools 程序验证。
