# 格式矩阵、性能与发布准备

基线：`dcc9c95`。范围：继续浏览器验证、格式/profile/版本矩阵、大样本基线和打包产物验证。

## 验证口径

- 合成源由现有 WASM codec/assembly 生成两层 64×64 GOO，覆盖 0–255 灰度和反向梯度；再次解码逐像素验证输入梯度。
- 由 Rust catalog facts 枚举导出 profile、Anycubic 支持版本和 ZIP 扩展名，共 73 个组合。通过真实 CLI 导出、真实 WASM 文档 API 打开和解码，再由 CLI 转回 GOO。
- 独立检查解析后的格式身份、Anycubic/CTB/加密 CTB 的物理 header 版本；正向像素按格式量化规则比较，PHOTONS 独立验证 nearest-neighbor 重采样与二值化；反向图层像素必须与目标解码像素完全相同。
- metadata 差异单独记录。通过表示上述内部一致性与投影检查通过，不能代替外部 UVTools 可打开、机器 profile 匹配或物理打印验证。

## 矩阵结果

73/73 组合通过，覆盖 46 个可导出 canonical profiles。下表汇总，逐组合命令、header 版本、像素差异和 timing 见 `/tmp/slicer-format-validation/synthetic-report.json`。

| 格式             | Rust profile ID              | 产物版本      | 扩展名                           | 组合数 | 已记录的 metadata 投影     |
| ---------------- | ---------------------------- | ------------- | -------------------------------- | ------ | -------------------------- |
| ctb-v1           | chitubox-ctb-v1              | 1             | ctb                              | 1      | 无上述字段差异             |
| ctb              | chitubox-ctb-v2              | 2             | ctb                              | 1      | 无上述字段差异             |
| ctb-v3           | chitubox-ctb-v3              | 3             | ctb                              | 1      | 无上述字段差异             |
| ctb-v4           | chitubox-ctb-v4              | 4             | ctb                              | 1      | 无上述字段差异             |
| ctb-v5           | chitubox-ctb-v5              | 5             | ctb                              | 1      | 无上述字段差异             |
| gktwo-ctb-v1     | chitubox-gktwo-ctb-v1        | 1             | gktwo.ctb                        | 1      | 无上述字段差异             |
| gktwo-ctb-v2     | chitubox-gktwo-ctb-v2        | 2             | gktwo.ctb                        | 1      | 无上述字段差异             |
| gktwo-ctb-v3     | chitubox-gktwo-ctb-v3        | 3             | gktwo.ctb                        | 1      | 无上述字段差异             |
| gktwo-ctb        | chitubox-gktwo-ctb-v4        | 4             | gktwo.ctb                        | 1      | 无上述字段差异             |
| gktwo-ctb-v5     | chitubox-gktwo-ctb-v5        | 5             | gktwo.ctb                        | 1      | 无上述字段差异             |
| cbddlp-v1        | chitubox-cbddlp-v1           | 1             | cbddlp                           | 1      | 无上述字段差异             |
| cbddlp           | chitubox-cbddlp              | 2             | cbddlp                           | 1      | 无上述字段差异             |
| photon           | chitubox-photon              | 2             | photon                           | 1      | 无上述字段差异             |
| phz              | chitubox-phz                 | default       | phz                              | 1      | 无上述字段差异             |
| fdg              | chitubox-fdg                 | default       | fdg                              | 1      | 无上述字段差异             |
| photons          | chitubox-photons             | default       | photons                          | 1      | resolution_x, resolution_y |
| encrypted-ctb    | chitubox-ctb-encrypted-v5    | 5             | encrypted.ctb                    | 1      | 无上述字段差异             |
| encrypted-ctb-v4 | chitubox-ctb-encrypted-v4    | 4             | encrypted.ctb                    | 1      | 无上述字段差异             |
| chitubox-zip     | chitubox-zip-archive-catalog | default       | zip                              | 1      | 无上述字段差异             |
| pw0              | anycubic-pw0                 | 1             | pw0                              | 1      | display_height_mm          |
| pws              | anycubic-pws                 | 1             | pws                              | 1      | display_height_mm          |
| pwx              | anycubic-pwx                 | 1             | pwx                              | 1      | display_height_mm          |
| dlp              | anycubic-dlp                 | 1, 515, 516   | dlp                              | 3      | display_height_mm          |
| pwmo             | anycubic-pwmo                | 1, 515, 516   | pwmo                             | 3      | display_height_mm          |
| pwms             | anycubic-pwms                | 1, 515, 516   | pwms                             | 3      | display_height_mm          |
| pwma             | anycubic-pwma                | 515, 516      | pwma                             | 2      | display_height_mm          |
| pwmx             | anycubic-pwmx                | 1, 515, 516   | pwmx                             | 3      | display_height_mm          |
| pmsq             | anycubic-pmsq                | 1, 515, 516   | pmsq                             | 3      | display_height_mm          |
| pm3              | anycubic-pm3                 | 515, 516      | pm3                              | 2      | display_height_mm          |
| pm3m             | anycubic-pm3m                | 515, 516      | pm3m                             | 2      | display_height_mm          |
| dl2p             | anycubic-dl2p                | 515, 516, 517 | dl2p                             | 3      | display_height_mm          |
| pmx2             | anycubic-pmx2                | 515, 516, 517 | pmx2                             | 3      | display_height_mm          |
| pwmb             | anycubic-pwmb                | 515, 516, 517 | pwmb                             | 3      | display_height_mm          |
| pm3r             | anycubic-pm3r                | 515, 516, 517 | pm3r                             | 3      | display_height_mm          |
| pm3n             | anycubic-pm3n                | 517           | pm3n                             | 1      | display_height_mm          |
| pm4n             | anycubic-pm4n                | 517           | pm4n                             | 1      | display_height_mm          |
| px6s             | anycubic-px6s                | 517           | px6s                             | 1      | display_height_mm          |
| pm5              | anycubic-pm5                 | 517           | pm5                              | 1      | display_height_mm          |
| pm5s             | anycubic-pm5s                | 518           | pm5s                             | 1      | display_height_mm          |
| m5sp             | anycubic-m5sp                | 518           | m5sp                             | 1      | display_height_mm          |
| anycubic-zip     | anycubic-zip-archive-catalog | default       | pm4u, pm7, pm7m, pp1, pp1m, pwsz | 6      | 无上述字段差异             |
| goo              | goo-v3                       | default       | goo                              | 1      | 无上述字段差异             |
| prz              | phrozen-prz-v3               | default       | prz                              | 1      | 无上述字段差异             |
| cxdlpv4          | creality-cxdlpv4             | default       | cxdlpv4                          | 1      | 无上述字段差异             |
| sl1              | prusa-sl1-archive            | default       | sl1, sl1s                        | 2      | lift_height, lift_speed    |
| vdt              | voxeldance-vdt-archive       | default       | vdt                              | 1      | 无上述字段差异             |

非 CTB/Anycubic 的 default 表示无需 `--version`，格式自身版本以 registry 实现为准；profile ID 用于目录追踪，CLI `--profile` 使用 canonical format name。

三种旧 Creality CXDLP v1/v2/v3 的 `can_export=false`，无法由当前项目导出生成，仍需要外部真实样本。Anycubic Zip 的六个扩展名均已生成并反向验证，但此次导出为 PW0 image codec；PWSZ contour image 的外部样本验证不包含在本矩阵内。

已记录的损失：CTB 为 gray7；PHZ/FDG 有自己的 gray7 扩展与高灰阶截断规则；CXDLPv4 非零灰阶补最低位；Anycubic PW0 family 为 nibble 投影；CBDDLP/Photon/PWS 为二值化；PhotonS 还会固定分辨率重采样。Anycubic 二进制格式的非方形物理像素可能投影 display_height；SL1 本次回读缺少 lift_height/lift_speed。以上不等同于无损保留原打印设置。

## 本次发现并修复的问题

1. Creality 原先将需要整文件 checksum 的 parser 声明为可读前缀，914 层 CXDLPv4 的浏览器文档 API 因此误报 checksum mismatch；Rust registry 现要求完整文档读取。生成物本身原已可由 CLI 和完整 WASM parser 解析。
2. Anycubic Zip 原导出空 machine_extern，底层层数/曝光既未写入 active resin，也未在 Rust/TS 导入恢复；现在依照 UVTools 的 active_resins→user_resins/factory_resins→property.name→slicepara 选择规则写出和读取，层曝光 fallback 也区分底层与普通层。
3. 按 UVTools 对齐 slicepara 总抬升高度与 normal_0/normal_1 两阶段高度之和，并用非零第二阶段测试保护。

Rust 与 TS 的新增回归均先失败后通过。新增浏览器回归：取消已进入 worker 调度的转换后立即重试，以及延迟旧文件读取时保持最新文件；两项已在 Chromium/Firefox 实际执行并通过。

## 真实样本基线

源：`rust/data/Untitled_20260413_1125_TI3U.goo`，914 层，8520×4320。输出和反向 GOO 保存在 `/tmp/slicer-format-validation/real-*`。CLI 使用 release build、4 threads；对图层 0/457/913 的每个像素做正向投影及反向一致性比较，三层各 36,806,400 像素。

6/6 优先路线通过，源→目标的上述关键 metadata 字段均无差异。真实样本这些抽样层的图像只包含能被各编码精确保留的值；灰度损失边界仍以合成梯度验证为准。

| 目标         | 正向 CLI 内部总耗时 ms | 反向 CLI 内部总耗时 ms | 正向 fast/fallback | 正向峰值 RSS MiB |
| ------------ | ---------------------- | ---------------------- | ------------------ | ---------------- |
| ctb-v4       | 15.25                  | 12.38                  | 914/0              | 85.0             |
| ctb-v5       | 15.03                  | 12.63                  | 914/0              | 79.1             |
| chitubox-zip | 1253.13                | 2992.80                | 0/914              | 384.2            |
| anycubic-zip | 2483.93                | 550.63                 | 0/914              | 212.7            |
| prz          | 417.90                 | 439.19                 | 0/914              | 201.2            |
| cxdlpv4      | 2518.52                | 562.79                 | 0/914              | 204.9            |

总耗时为 CLI 内部 timing，排除进程启动；RSS 是独立 Python helper 的单个 CLI 子进程 `RUSAGE_CHILDREN.ru_maxrss`，本机 macOS 返回字节。RSS 与 timing 来自不同次运行；单次结果用于基线，不作为跨机器性能保证。详见 real-report.json 和 real-rss-report.json。

实际 Node/WASM preview codec：914 层源的第 0 层→1024×519 RGBA，5 次耗时 3.76/2.36/1.30/1.25/1.26 ms，产物 2,125,824 RGBA bytes。这是 codec 耗时，不包含浏览器 File I/O、worker 传输、ImageBitmap 或绘制；采样前后的进程 RSS 也不是浏览器峰值内存。浏览器实测结果见下文；Node/WASM codec 数据保留为独立口径。

## 可复现命令

```sh
pnpm build:wasm
cargo build --manifest-path rust/Cargo.toml -p slicer-converter-cli --release --offline
node scripts/verify-format-matrix.mjs
SC_MATRIX_MODE=real node scripts/verify-format-matrix.mjs
node scripts/verify-packed-package.mjs
CI=true pnpm test:app:e2e
```

`SC_MATRIX_OUTPUT` 设置矩阵生成物目录，`SC_MATRIX_SOURCE` 指定真实 GOO，`SC_MATRIX_CLI` 指定 CLI；`SC_PACK_SMOKE_OUTPUT` 指定打包/消费项目目录。默认产物写临时目录，不提交大样本。

## 浏览器验证与性能

环境权限恢复后，`CI=true PLAYWRIGHT_USE_EXISTING_SERVER=1 pnpm test:app:e2e` 在 Chromium 147.0.7727.15 和 Firefox 148.0.2 上实际通过 62/62 项，包括 GOO→CTB→GOO、取消已进入 worker 调度的转换后立即重试、旧文件延迟完成时保持新文件、各格式上传/下载、预览滑块、语言和 onboarding。

闭环 smoke 显式选 CTB v2，避免把默认 v1 误当成 v2。WASM 浏览器测试 `pnpm test:wasm:browser` 也通过（1 项）；此前失败由本地缓存 ChromeDriver147 与系统 Chrome154 不匹配造成，刷新忽略目录中的 driver 后恢复，不改生产代码。

914 层真实 GOO 在两个浏览器导出 VDT，下载产物分别保存在 `/tmp/slicer-browser-baseline/chromium-real.vdt` 和 `firefox-real.vdt`。真实 WASM 文档 API 重新打开、检查914层及关键 metadata，并在0/457/913层逐像素比较，两个实际浏览器产物均与源一致。

| 浏览器   | 上传开始至首次 preview canvas draw（单次） | VDT 转换（页面内部 timing，单次） | 转换期间主线程 JS heap 采样峰值 | 隔离浏览器进程树 RSS 合计采样峰值 |
| -------- | ------------------------------------------ | --------------------------------- | ------------------------------- | --------------------------------- |
| Chromium | 522.4 ms                                   | 1.27 s                            | 约49.2 MiB                      | 约2023.8 MiB                      |
| Firefox  | 514 ms                                     | 7.36 s                            | API不支持，未观测               | 约2517.4 MiB                      |

首帧从开始上传到实际调用 preview canvas 的 drawImage计时，包含文件读取、解析、worker通信及render调用，但不是屏幕像素显示的精确present时间。JS heap每25ms采样，RSS每100ms采样；RSS是隔离浏览器所有子进程之和，包含renderer/worker、浏览器和GPU等开销，共享页可能重复计数，不能当成独占物理内存。单次测量用于定位热点，不作为跨机器或浏览器性能保证。浏览器大文件内存与Firefox转换速度是下一轮优化重点。

另外，Chromium Blob VDT三轮基线：1.28/1.34/1.25 s，中位数1.28 s；主线程heap采样峰值中位数65.2 MiB。PRZ/VDT各三轮的Blob和真实OPFS诊断benchmark均通过1500ms finalize-only门禁。

旧diagnostics benchmark要求已不再发出的内部complete事件，实测曾失败；现以显式选择的公开完成状态和耗时做验证，保留实际事件数0、内部transfer/WASM/sink数据null，不伪造观测。每次运行缺少请求阈值的测量都会失败，不能由其他运行的测量掩盖。

原localhost sandbox/自动审批404阻塞已经解除。外部UVTools executable和旧CXDLP真实样本仍未验证，本次浏览器通过不能替代这些兼容性检查。

## 0.1.5 发布准备

- 包版本、README和changelog已对齐0.1.5，页面版本来自包元数据。
- 实际构建0.1.5 npm tarball，再离线安装到全新的外部consumer；两个新进程验证默认/进度初始化、打开样本、读取单层和拒绝截断输入，均通过。
- 产物位于系统临时目录的`slicer-package-verification/innovgame-slicer-converter-0.1.5.tgz`，脚本写出report.json和integrity。
- release准备完成；尚未tag、push或publish。多平台CLI release构建和外部打印机验证不在本地浏览器门禁内。

## 最终检查与审阅

- `pnpm check`、`pnpm build`、Rust fmt/Clippy通过；完整测试和最终浏览器验证已执行。
- 矩阵：合成73/73、真实6/6；0.1.5独立消费包两个入口通过。
- 独立审阅提出的正向重采样、产物版本、取消同步门控和两阶段抬升高度已落实。后续benchmark审阅发现的跨运行缺失测量掩盖也经过RED→GREEN修复。
- 新测试／benchmark与生产API的边界保持清晰；没有添加生产遥测以满足旧测试假设。
- 本轮改动随提交归档；按用户要求暂不发布，未tag、push或publish。

最终完整测试：Rust539项、package241项、app142项、scripts24项均通过；浏览器E2E62项与WASM浏览器1项通过。
