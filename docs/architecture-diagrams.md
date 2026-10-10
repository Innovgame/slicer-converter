# 项目架构图

## 1. 项目架构总览

```mermaid
graph TB
    subgraph Browser["🌐 Browser Runtime"]
        UI["React 19 UI<br/>(converter-app, panels, hooks)"]
        MainThread["Main Thread"]
        WorkerPool["Worker Pool<br/>(cores-1, max 8)"]
        W1["Worker 1<br/>WASM Instance"]
        W2["Worker 2<br/>WASM Instance"]
        Wn["Worker N<br/>WASM Instance"]
    end

    subgraph Frontend["📦 apps/slicer-app/src/"]
        direction TB
        AppLayer["app/<br/>React Components + Hooks"]
        CoreLayer["core/<br/>Business Logic"]
        WorkerLayer["worker/<br/>Handlers + Runtime"]

        AppLayer --> CoreLayer
        CoreLayer --> WorkerLayer
    end

    subgraph CoreModules["core/ 模块"]
        direction TB
        DocLoader["document-loader<br/>文件加载/解析"]
        ConvCtrl["conversion-controller<br/>转换编排"]
        ConvSession["conversion-session<br/>转换会话/层负载构建"]
        PreviewSession["preview-session<br/>预览派发会话"]
        TargetCatalog["format-target-catalog<br/>目标格式目录"]
        FmtRegistry["format-registry<br/>格式注册表"]
        FmtPlugins["formats/<br/>插件 + documentOpener"]
        ArchivePlan["archive-output-plan<br/>Rust/WASM entry plan"]
        ZipAssembler["zip-stream-assembler<br/>流式 ZIP 组装"]
        OutputSink["streaming-writer<br/>OutputSink 接口"]
        WPool["worker-pool<br/>生命周期管理"]
        Prefetch["layer-prefetch<br/>自适应预取"]
    end

    subgraph WasmPkg["📦 packages/slicer-wasm/"]
        WasmAPI["api.ts<br/>WASM 初始化 & 绑定"]
        WasmTypes["types.ts<br/>TypeScript 类型定义"]
    end

    subgraph RustCore["🦀 rust/src/"]
        direction TB
        LibRS["lib.rs<br/>wasm-bindgen 导出"]

        subgraph Formats["formats/"]
            FmtMod["mod.rs<br/>FormatRegistry trait<br/>FORMAT_FAMILIES"]
            SpecKernel["common/<br/>FormatSpec + strategy traits"]
            CTB["ctb/<br/>CTB specs"]
            GOO["goo/<br/>GOO spec"]
            SL1["sl1/<br/>SL1 archive spec"]
            VDT["vdt/<br/>VDT archive spec"]
            PRZ["prz/<br/>PRZ spec"]

            subgraph Anycubic["anycubic/ (21 变体)"]
                AnycubicFmt["Anycubic FormatSpec profiles"]
                Codec["AnycubicLayerCodec trait<br/>(Pw0Codec, PwsCodec)"]
                Layout["AnycubicLayoutStrategy trait<br/>(V1, V516, V517, V518)"]
            end
        end

        subgraph Common["common/"]
            Metadata["metadata.rs<br/>FileMetadata (6 子结构体)<br/>Builder 模式"]
            Error["error.rs<br/>ConvertError 枚举"]
            Layer["layer.rs<br/>LayerImage / PreviewLayerImage"]
        end

        LibRS --> FmtMod
        FmtMod --> SpecKernel
        FmtMod --> CTB & GOO & SL1 & VDT & PRZ & AnycubicFmt
        AnycubicFmt --> Codec & Layout
        LibRS --> Common
    end

    UI -->|"用户操作"| MainThread
    MainThread -->|"postMessage"| WorkerPool
    WorkerPool --> W1 & W2 & Wn
    W1 & W2 & Wn -->|"wasm-bindgen FFI"| LibRS

    FmtRegistry -->|"syncFromWasm"| WasmAPI
    WasmAPI -->|".wasm 加载"| LibRS

    AppLayer --> TargetCatalog
    AppLayer --> PreviewSession
    DocLoader --> FmtRegistry
    FmtRegistry --> FmtPlugins
    ConvCtrl --> WPool
    ConvCtrl --> ConvSession
    ConvCtrl --> ArchivePlan
    ConvCtrl --> ZipAssembler
    ConvCtrl --> OutputSink
    ConvCtrl --> Prefetch
    ArchivePlan --> WasmAPI

    style Browser fill:#e1f5fe,stroke:#0288d1
    style RustCore fill:#fff3e0,stroke:#ef6c00
    style Frontend fill:#e8f5e9,stroke:#388e3c
    style WasmPkg fill:#f3e5f5,stroke:#7b1fa2
```

补充说明：

- 源文件加载优先经 `document-loader` 与格式插件的 `documentOpener` 处理；只有通用路径才回退到 WASM 头部解析。
- ZIP 输出的 entry naming、metadata entries 和 compression method 经 `archive-output-plan` 从 Rust/WASM 获取，浏览器侧只负责流式写出。
- Rust/WASM 已是格式事实的主来源，但浏览器侧当前仍保留 `KNOWN_FORMAT_INFOS` 作为启动期预热回退。
- worker 侧通过 `createWorkerRuntime(deps)` 组合 preview / conversion handler map，主线程只依赖消息协议，不直接耦合具体处理实现。

---

## 2. 格式转换完整数据流

```mermaid
sequenceDiagram
    participant User as 👤 用户
    participant UI as React UI
    participant Ctrl as conversion-controller
    participant Prefetch as layer-prefetch
    participant Pool as worker-pool
    participant Worker as Worker (×N)
    participant WASM as Rust WASM
    participant Assembler as zip-stream-assembler
    participant Sink as OutputSink

    User->>UI: 选择文件 + 目标格式
    UI->>Ctrl: onConvert(file, targetFormat)

    Note over Ctrl: 解析源文件
    Ctrl->>WASM: parse_header(data, ext)
    WASM-->>Ctrl: ParsedHeader {metadata, layers[]}

    Note over Ctrl: 构建目标 header
    Ctrl->>WASM: build_header(metadata, format)
    WASM-->>Ctrl: headerBytes

    Note over Ctrl: 初始化输出
    Ctrl->>Sink: write(headerBytes)
    Ctrl->>Prefetch: 创建预取队列(totalLayers)

    loop 每个 layer (并行 N workers)
        Prefetch->>Prefetch: File.slice() 预取层数据
        Ctrl->>Pool: findIdleWorker()
        Pool-->>Ctrl: Worker ref

        alt 非 ZIP 格式 (CTB/GOO/PRZ)
            Ctrl->>Worker: DecodeEncodeRequest {data, srcFmt, tgtFmt}
            Worker->>WASM: decode_encode_layer(data, ...)
            Note over WASM: decode (RLE/SIMD) → pixels → encode (target RLE)
            WASM-->>Worker: encodedBytes
            Worker-->>Ctrl: layer-done {result, elapsedMs}
            Ctrl->>Sink: write(encodedBytes)
        else ZIP 格式 (SL1/VDT) — 单层 prepare + 流式组装
            Ctrl->>Worker: DecodeEncodePrepareRequest {data, index}
            Worker->>WASM: decode_encode_layer(data, ...)
            Note over WASM: 单层 decode+encode，内部 buffer 复用
            WASM-->>Worker: encodedBytes
            Worker->>Worker: prepareZipEntry (CRC32 + deflate)
            Worker-->>Ctrl: zip-layer-prepared {entry}
            Ctrl->>Assembler: pushLayerEntry(index, entry)
            Note over Assembler: 乱序缓冲 → 前缀有序时自动 flush
            Assembler->>Sink: write(zipLocalHeader + data)
        end
    end

    Note over Ctrl: 完成
    Ctrl->>WASM: finalize_header(header, layerSizes)
    WASM-->>Ctrl: patchedHeader
    Ctrl->>Sink: patchHeader(patchedHeader)

    alt ZIP 格式
        Ctrl->>Assembler: finalize()
        Assembler->>Sink: write(centralDirectory + EOCD)
    end

    Ctrl->>Sink: finish()
    Sink-->>User: 下载文件 (Blob 或 Stream)
```

---

## 3. Rust FormatSpec 与策略组合关系

```mermaid
classDiagram
    class FormatSpec {
        +identity FormatIdentity
        +profile FormatProfile
        +capabilities FormatCapabilities
        +keys FormatKeys
        +header HeaderLayout
        +layer_codec LayerCodec
        +crypto CryptoPolicy
        +footer FooterPolicy
        +archive ArchiveLayout
        +pipeline FormatPipeline
        +parse_header(data) Result~FileMetadata, LayerOffset~
        +decode_layer_into(data, ctx, output) Result
        +encode_layer_into(pixels, ctx, output) Result
        +build_header(metadata) Result~Vec u8~
        +finalize_header(header, sizes) Result~Vec u8~
        +build_footer() Vec u8
        +try_detect(data) bool
    }

    class FormatRegistry {
        <<trait>>
        +families() [FormatFamilySpec]
        +all_specs() [FormatSpec]
        +spec_by_name(name) Option~FormatSpec~
        +spec_by_extension(ext) Option~FormatSpec~
        +detect_spec(data, hint) Option~FormatSpec~
    }

    class FormatFamilySpec {
        +identity FamilyIdentity
        +family FormatFamily
        +profiles [FormatSpec]
        +resolver ProfileResolver
    }

    class HeaderLayout {
        <<trait>>
        +try_detect(data, hint) bool
        +parse_header(data, spec) Result
        +build_header(metadata, spec) Result
        +finalize_header(header, sizes, spec) Result
    }

    class LayerCodec {
        <<trait>>
        +decode_layer_into(data, ctx, output) Result
        +encode_layer_into(pixels, ctx, output) Result
        +decode_preview_rgba(data, ctx, max) Result
    }

    class CryptoPolicy {
        <<trait>>
        +decrypt_layer_in_place(...)
        +encrypt_layer_in_place(...)
    }

    class FooterPolicy {
        <<trait>>
        +build_footer(metadata, spec) Result
    }

    class ArchiveLayout {
        <<trait>>
        +entries(data, spec) Result
        +describe_layer(index, spec) Result
    }

    class FormatPipeline {
        <<trait>>
        +parse_document(spec, data) Result
        +encode_document_into(spec, metadata, layers, sink) Result
    }

    class FileMetadata {
        +printer: PrinterSpec
        +slicing: SlicingSpec
        +exposure: ExposurePlan
        +motion: MotionPlan
        +waits: WaitTimes
        +stats: PrintStatistics
        +per_layer_settings: Vec
        +thumbnails: Vec
    }

    class ConvertError {
        <<enum>>
        InvalidMagic
        UnsupportedVersion
        UnknownFormat
        BufferTooSmall
        RleDecodeError
        PngError
        ZipError
        IoError
        MetadataValidation
    }

    FormatRegistry --> FormatFamilySpec : exposes
    FormatFamilySpec --> FormatSpec : profiles
    FormatSpec *-- HeaderLayout : header
    FormatSpec *-- LayerCodec : layer_codec
    FormatSpec *-- CryptoPolicy : crypto
    FormatSpec *-- FooterPolicy : footer
    FormatSpec *-- ArchiveLayout : archive
    FormatSpec *-- FormatPipeline : pipeline

    FileMetadata --> ConvertError : Result
    FormatSpec --> FileMetadata : parse/build
    FormatSpec --> ConvertError : Result
```
