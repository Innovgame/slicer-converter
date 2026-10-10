# Browser Integration Reference

本文记录 slicer-converter 浏览器侧集成边界。推荐外部调用方优先使用 package 根入口 `@slicer-converter/wasm`，不要直接依赖 `apps/slicer-app` 内部文件。

## 1. 推荐入口

主入口：

- `packages/slicer-wasm/src/index.ts`
- `packages/slicer-wasm/src/api.ts`

`@slicer-converter/wasm` 的根入口保持兼容导出：初始化 API、document opening、conversion、preview、worker runtime、archive/output helper、format helper 和 runtime adapter 都继续从 root 暴露。内部源码已按模块分组，但调用方不需要改成深路径导入。

初始化函数：

- `initSlicerConverter(wasmUrl?)`
- `initSlicerConverterWithProgress(onProgress, wasmUrl?)`

二者都会返回 `SlicerConverterApi`。当前 `SlicerConverterApi` 是初始化后的轻量 facade，包含常用文档打开、转换、目标选择和格式查询方法；worker runtime、archive/output helper、preview session 等仍作为 root helper export 暴露，不放进初始化返回对象。

常用示例：

```ts
import { convertSlicerFile, decodeSlicerLayerPreview, openSlicerDocument } from "@slicer-converter/wasm";

const document = await openSlicerDocument({ file, extension: "goo", parseHeader });
const preview = await decodeSlicerLayerPreview({
    document,
    layerIndex: 0,
    quality: "settled",
    maxPreviewSize: 1024
});

const converted = await convertSlicerFile({
    file,
    target: "vdt",
    onProgress(progress) {
        console.log(progress.phase, progress.current, progress.total);
    }
});
```

## 2. browser 源码分组

`packages/slicer-wasm/src/browser/` 下的实现文件已从旧平铺路径整理为以下分组：

- `conversion/`：转换 pipeline、job/session/strategy/controller、导出目标和目标矩阵。
- `worker/`：worker protocol、runtime、entry runtime、errors、pool、dispatch、handlers、layer 调度。
- `archive/`：archive plan、ZIP 读写、ZIP worker helper。
- `output/`：document assembly、output sink、diagnostics、stream、binary output、File System Access sink、layer spool。
- `document/`：`SlicerDocument` model、document source/opener。
- `format/`：format facts、header parser、GOO header parser、browser format behavior registry。
- `preview/`：preview facade 和 preview session。
- `runtime/`：layer codec runtime adapter 和 runtime metrics。

每个分组都有自己的 `index.ts` barrel，根入口 `packages/slicer-wasm/src/index.ts` 从这些分组 barrel re-export，以维持历史 root exports。分组内实现文件应直接导入具体实现文件，不从同级 `index.ts` 或 package root barrel 回流导入。

## 3. 高层 browser API

这些 API 面向浏览器项目，调用方不需要 import `apps/slicer-app/*`：

- `openSlicerDocument({ file, extension, parseHeader })`
  打开浏览器 `File` 并返回 `SlicerDocument`。ZIP-backed layer 默认保持 compressed payload，读取 layer 时才按需触达对应 entry。
- `openSlicerDocumentFile({ file, parseHeader })`
  从 `file.name` 推导扩展名后打开文档。
- `convertSlicerFile({ file, target, ... })`
  从浏览器文件直接转换为目标格式。默认返回 `Blob`，也可传入 `SlicerOutputSink` / `PatchableSlicerOutputSink` 自行接收输出。
- `decodeSlicerLayerPreview({ document, layerIndex, quality, maxPreviewSize })`
  从已打开的 `SlicerDocument` 解码单层 preview。
- `createSlicerPreviewSession({ document })`
  给 worker-backed UI 使用的低层 helper，把 document layer payload 构造成 preview worker request。

preview 路由会通过 `PreviewTimings.path` 暴露实际路径：

- `encoded-layer-preview`
- `archive-layer-preview`
- `archive-original-png`

## 4. Worker Protocol

协议文件现在属于 package worker module：

- `packages/slicer-wasm/src/browser/worker/protocol.ts`
- `packages/slicer-wasm/src/browser/worker/errors.ts`
- `packages/slicer-wasm/src/browser/worker/runtime.ts`
- `packages/slicer-wasm/src/browser/worker/entry-runtime.ts`

不再以旧 app `types.ts` 作为 worker protocol 真相源。

request 类型：

- `decode-encode`
- `decode-encode-prepare`
- `decode-preview`
- `zip-decode-encode`
- `zip-decode-encode-prepare`
- `cancel`

response 类型：

- `ready`
- `layer-done`
- `preview-done`
- `zip-layer-prepared`
- `error`

结构化 worker 错误使用 `SlicerWorkerErrorMessage`：

- `type: "error"`
- `error: SlicerWorkerError`
- `index?: number`

`SlicerWorkerError` 的关键字段：

- `code`
- `message`
- `stage`
- `retryable`
- `timestamp`
- `format?`
- `layerIndex?`
- `detail?`

app 侧消费路径：

- preview: `apps/slicer-app/src/features/preview/model/use-preview-controller.ts`
- conversion: `apps/slicer-app/src/features/conversion/model/use-conversion-controller.ts`
- runtime bootstrap: `apps/slicer-app/src/app/runtime/use-runtime-controller.ts`
- worker entry: `apps/slicer-app/src/worker/entry.ts`

## 5. BrowserFormatBehavior Capability

能力定义在 package document/format 模块中：

- `packages/slicer-wasm/src/browser/document/source.ts`
- `packages/slicer-wasm/src/browser/format/browser-format-behaviors.ts`
- `packages/slicer-wasm/src/browser/format/behaviors/registry.ts`

`BrowserFormatBehavior` 是浏览器侧的补充能力层，不承载完整格式事实真相，也不承载 ZIP 输出策略。目前只保留：

- `documentOpener?`

格式事实通过 typed `getSupportedFormats()` 从 WASM registry 同步；ZIP 输出的 entry plan 通过 `buildArchiveMetadataEntries()` / `describeArchiveLayerEntries()` 获取。app 启动时在 `apps/slicer-app/src/app/runtime/use-runtime-controller.ts` 调用 `getSupportedFormats()` 和 `validateBrowserFormatBehaviors()`，让缺失的 browser behavior / WASM facts 尽早暴露。

## 6. 错误语义

browser facade 会把 WASM 侧错误规范化成真正的 `Error` 实例，并尽量保留：

- `message`
- `code`
- `detail`
- `retryable`

相关 helper：

- `getWasmErrorMessage(error)`
- `toWasmError(error)`
- `getWorkerErrorMessage(error)`
- `toSlicerWorkerError(error, fallback)`
- `toSlicerWorkerErrorMessage(input)`

上层 UI 通常展示 `message`，日志和调试层使用完整结构化 envelope，不应只靠字符串正则匹配去猜错误类型。

## 7. 推荐阅读顺序

第一次进入浏览器侧代码时，建议按这个顺序看：

1. `packages/slicer-wasm/src/api.ts`
2. `packages/slicer-wasm/src/index.ts`
3. `packages/slicer-wasm/src/browser/types.ts`
4. `packages/slicer-wasm/src/browser/document/source.ts`
5. `packages/slicer-wasm/src/browser/format/browser-format-behaviors.ts`
6. `packages/slicer-wasm/src/browser/worker/protocol.ts`
7. `apps/slicer-app/src/app/runtime/use-runtime-controller.ts`
8. `apps/slicer-app/src/features/preview/model/use-preview-controller.ts`
9. `apps/slicer-app/src/features/conversion/model/use-conversion-controller.ts`
