import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

import { describe, it, expect, vi } from "vitest";

/**
 * @zh 在导入被测模块前 mock wasm-pack 产物，避免 unit test 加载真实 .wasm。
 * @en Mocks the wasm-pack generated module before importing the module under test, avoiding real .wasm loading in unit tests.
 */
const mockInitWasm = vi.fn().mockResolvedValue(undefined);
const mockParseHeader = vi.fn();
const mockDecodeLayer = vi.fn();
const mockDecodeLayerPreview = vi.fn();
const mockDecodeEncodeLayer = vi.fn();
const mockDecodeEncodeLayerStats = vi.fn();
const mockEncodeLayer = vi.fn();
const mockBuildHeader = vi.fn();
const mockDiagnoseExport = vi.fn();
const mockBuildDocumentFromLayers = vi.fn();
const mockBuildArchiveMetadataEntries = vi.fn();
const mockBuildCxdLpV4DocumentHeader = vi.fn();
const mockBuildCxdLpV4LayerDef = vi.fn();
const mockBuildFooter = vi.fn();
const mockFinalizeHeader = vi.fn();
const mockDescribeArchiveLayerEntries = vi.fn();
const mockCrc32Bytes = vi.fn();
const mockCxdLpCrc32Update = vi.fn();
const mockListSupportedFormats = vi.fn();
const EXPECTED_INIT_API_METHODS = [
    "convertSlicerFile",
    "crc32Bytes",
    "diagnoseExport",
    "findSlicerExportTargetMatrixSelection",
    "getSupportedFormats",
    "listExportTargetOptions",
    "listSlicerExportTargetMatrix",
    "loadSlicerDocumentFileSource",
    "loadSlicerDocumentSource",
    "openSlicerDocument",
    "openSlicerDocumentFile",
    "resolveSlicerExportTarget",
    "selectDefaultExportTarget"
].sort();
const EXPECTED_ROOT_RUNTIME_EXPORTS = [
    "BUILTIN_BROWSER_FORMAT_BEHAVIORS",
    "DEFAULT_ANYCUBIC_MANUAL_TARGET",
    "DEFAULT_CHITUBOX_MANUAL_TARGET",
    "SLICER_WORKER_REQUEST_TYPES",
    "SLICER_WORKER_RESPONSE_TYPES",
    "asSlicerFormatId",
    "assembleBinaryOutput",
    "assembleOutputFileFromLayerStream",
    "assembleWasmBinaryOutputFromLayerStream",
    "buildArchiveMetadataEntries",
    "buildCentralHeader",
    "buildDataDescriptor",
    "buildEocd",
    "buildFileMetadataJson",
    "buildLayerMetadataJson",
    "buildLocalHeader",
    "buildSlicerOutputFilename",
    "buildSlicerOutputJobName",
    "buildSlicerZipOutputWithWorkers",
    "convertSlicerFile",
    "convertSlicerFileToBlob",
    "convertSlicerFileToBlobWithDeps",
    "convertSlicerFileToSink",
    "convertSlicerFileToSinkWithDeps",
    "convertSlicerFileWithDeps",
    "BufferedSlicerOutputSink",
    "convertSlicerLayersWithWorkers",
    "convertZipLayerForTarget",
    "createBlobOutputSink",
    "createBufferedOutputSink",
    "createFileZipSource",
    "createArchiveLayerDescriptorProvider",
    "createInstrumentedOutputSink",
    "createInstrumentedPatchableOutputSink",
    "createInMemoryZipSource",
    "createLayerSpool",
    "createOrderedLayerQueue",
    "createSlicerBinaryLayerQueue",
    "createSlicerDocument",
    "createSlicerConversionHandlers",
    "createSlicerConversionSession",
    "createSlicerLayerPrefetch",
    "createSlicerPreviewSession",
    "createSlicerPreviewHandlers",
    "createSlicerRuntimeMetrics",
    "createSlicerWorkerPoolState",
    "createSlicerWorkerRuntime",
    "createStreamingDocumentAssemblyHelpers",
    "createZipSource",
    "crc32Bytes",
    "decodeZipPreview",
    "decodeSlicerLayerPreview",
    "diagnoseExport",
    "describeArchiveLayerEntries",
    "describeArchiveLayerEntry",
    "encodeZipEntryName",
    "findSlicerExportTargetMatrixSelection",
    "getFormatFacts",
    "getDocumentOpenerForFormat",
    "getSlicerConverterWasmUrl",
    "getSlicerWorkerRequestErrorContext",
    "getSupportedFormats",
    "getWorkerErrorMessage",
    "inflateRaw",
    "initSlicerConverter",
    "initSlicerConverterWithProgress",
    "initSlicerWorkerPool",
    "isFileSystemWriteSupported",
    "isRetryableWorkerError",
    "isSlicerWorkerError",
    "listAnycubicManualTargetOptions",
    "listChituboxManualTargetGroups",
    "listChituboxManualTargetOptions",
    "listExportTargetOptions",
    "listFormatFacts",
    "listImportableFileExtensions",
    "listSlicerExportTargetMatrix",
    "loadSlicerDocumentFileSource",
    "loadSlicerDocumentSource",
    "openSlicerDocument",
    "openSlicerDocumentFile",
    "openFileSystemSink",
    "parseAnycubicZipFile",
    "parseGooHeaderStreaming",
    "parseChituboxZipFile",
    "parseSl1File",
    "parseVdtFile",
    "parseZipCentralDirectoryEntries",
    "parseZipDirectoryFromReader",
    "parseBrowserFileHeader",
    "prepareSlicerExportTargetFromMatrix",
    "prepareZipEntry",
    "prepareZipEntryWithTimings",
    "readZipEntryBytes",
    "readZipEntryText",
    "resolveConcreteFormatId",
    "resolveSlicerExportTarget",
    "runSlicerConversionJob",
    "runSlicerConversionStrategy",
    "runSlicerLayerWorkerScheduler",
    "selectDefaultExportTarget",
    "slicerLayerCodecRuntime",
    "snapshotOutputSinkStats",
    "snapshotBufferedOutputSinkStats",
    "startSlicerWorkerEntryRuntime",
    "syncFormatFacts",
    "syncSupportedFormatInfosForValidation",
    "toSlicerWorkerError",
    "toSlicerWorkerErrorMessage",
    "validateBrowserFormatBehaviors",
    "waitForSlicerWorkerReady",
    "ZipStreamAssembler"
].sort();
const BROWSER_MODULE_DIRECTORIES = [
    "archive",
    "conversion",
    "document",
    "format",
    "output",
    "preview",
    "runtime",
    "worker"
];

vi.mock("../../lib/slicer_converter.js", () => ({
    default: mockInitWasm,
    parse_header: mockParseHeader,
    decode_layer: mockDecodeLayer,
    decode_layer_preview: mockDecodeLayerPreview,
    decode_encode_layer: mockDecodeEncodeLayer,
    decode_encode_layer_stats: mockDecodeEncodeLayerStats,
    encode_layer: mockEncodeLayer,
    build_document_from_layers: mockBuildDocumentFromLayers,
    diagnose_export: mockDiagnoseExport,
    build_archive_metadata_entries: mockBuildArchiveMetadataEntries,
    build_cxdlpv4_document_header: mockBuildCxdLpV4DocumentHeader,
    build_cxdlpv4_layer_def: mockBuildCxdLpV4LayerDef,
    build_header: mockBuildHeader,
    build_footer: mockBuildFooter,
    finalize_header: mockFinalizeHeader,
    describe_archive_layer_entries: mockDescribeArchiveLayerEntries,
    crc32_bytes: mockCrc32Bytes,
    cxdlp_crc32_update: mockCxdLpCrc32Update,
    list_supported_formats: mockListSupportedFormats
}));
const { initSlicerConverter, getSlicerConverterWasmUrl } = await import("../../src/index.js");

describe("initSlicerConverter / 初始化 slicer converter", () => {
    it("returns an API object with all expected methods and calls initWasm / 返回 an API 对象 带有 全部 expected 方法 并且 calls initWasm", async () => {
        const api = await initSlicerConverter();

        expect(mockInitWasm).toHaveBeenCalled();
        expect(Object.keys(api).sort()).toEqual(EXPECTED_INIT_API_METHODS);
    });

    it("is idempotent — subsequent calls return the same API shape without re-initializing / is idempotent — subsequent calls 返回 the 相同 API shape 不通过 re-initializing", async () => {
        const callCountBefore = mockInitWasm.mock.calls.length;
        const api1 = await initSlicerConverter();
        const api2 = await initSlicerConverter();
        const callCountAfter = mockInitWasm.mock.calls.length;
        expect(callCountAfter).toBe(callCountBefore);
        expect(Object.keys(api1).sort()).toEqual(EXPECTED_INIT_API_METHODS);
        expect(Object.keys(api2).sort()).toEqual(EXPECTED_INIT_API_METHODS);
    });
});

describe("getSlicerConverterWasmUrl / 获取 WASM URL", () => {
    it("returns a URL ending with .wasm / 返回 a URL ending 带有 .wasm", () => {
        const url = getSlicerConverterWasmUrl();
        expect(url).toBeInstanceOf(URL);
        expect(url.pathname).toMatch(/\.wasm$/);
    });

    it("points to the release wasm file / 指向 the release wasm 文件", () => {
        const url = getSlicerConverterWasmUrl();
        expect(url.pathname).toContain("slicer_converter_bg.wasm");
        expect(url.pathname).not.toContain("/debug/");
    });
});

describe("re-exports / 重新导出", () => {
    it("exports codec helpers through typed adapters instead of direct raw root exports / exports codec helpers through typed adapters 而不是 direct raw root exports", async () => {
        const mod = await import("../../src/index.js");
        const previewResult = { pixels: new Uint8Array([1, 2]), width: 2, height: 1 };
        const packedPreview = new Uint8Array([2, 0, 0, 0, 1, 0, 0, 0, 1, 2]);
        mockDecodeLayerPreview.mockReturnValueOnce(packedPreview).mockReturnValueOnce(packedPreview);

        expect(mod).not.toHaveProperty("parse_header");
        expect(mod).not.toHaveProperty("decode_layer");
        expect(mod).not.toHaveProperty("decode_layer_preview");
        expect(mod).not.toHaveProperty("decode_encode_layer");
        expect(mod).not.toHaveProperty("decode_encode_layer_stats");
        expect(mod).not.toHaveProperty("encode_layer");
        expect(mod.slicerLayerCodecRuntime.decodeEncodeLayer).toBe(mockDecodeEncodeLayer);
        expect(mod.slicerLayerCodecRuntime.decodeEncodeLayerStats).toBe(mockDecodeEncodeLayerStats);
        expect(mod.slicerLayerCodecRuntime.decodeLayer).toBe(mockDecodeLayer);
        expect(mod.slicerLayerCodecRuntime.decodeLayerPreview("data" as never, 1, "ctb", 64, 32, 1, 0, 32)).toEqual(
            previewResult
        );
        expect(mod.slicerLayerCodecRuntime.encodeLayer).toBe(mockEncodeLayer);
        mockBuildArchiveMetadataEntries.mockReturnValueOnce(
            JSON.stringify([{ name: "config.ini", data: [1, 2, 3], compression_method: 8 }])
        );
        expect(
            mod.buildArchiveMetadataEntries({
                metadata: { resolution_x: 64, resolution_y: 32 },
                targetFormat: "sl1",
                jobName: "demo",
                totalLayers: 2
            })
        ).toEqual([{ name: "config.ini", data: new Uint8Array([1, 2, 3]), compressionMethod: 8 }]);
        expect(mockBuildArchiveMetadataEntries).toHaveBeenCalledWith(expect.any(String), "sl1", "demo", 2);
        mockDescribeArchiveLayerEntries.mockReturnValueOnce(
            JSON.stringify([
                { index: 3, name: "demo00003.png", compression_method: 8 },
                { index: 4, name: "demo00004.png", compression_method: 0 }
            ])
        );
        expect(
            mod.describeArchiveLayerEntries({
                targetFormat: "sl1",
                jobName: "demo",
                startIndex: 3,
                count: 2
            })
        ).toEqual([
            { index: 3, name: "demo00003.png", compressionMethod: 8 },
            { index: 4, name: "demo00004.png", compressionMethod: 0 }
        ]);
        expect(mockDescribeArchiveLayerEntries).toHaveBeenCalledWith("sl1", "demo", 3, 2);
        expect(mod.crc32Bytes(new Uint8Array([1, 2]))).toBeUndefined();
        expect(mockCrc32Bytes).toHaveBeenCalledWith(new Uint8Array([1, 2]));
        const cxdlpAssembly = mod.createStreamingDocumentAssemblyHelpers("cxdlpv4");
        const cxdlpDocument = cxdlpAssembly.streamingDocument;
        expect(cxdlpDocument?.buildPrefix).toBeTypeOf("function");
        expect(cxdlpDocument?.checksumUpdate).toBeTypeOf("function");
        expect(cxdlpDocument?.buildChecksumTrailer).toBeTypeOf("function");
        mockBuildCxdLpV4DocumentHeader.mockReturnValueOnce(new Uint8Array([10]));
        mockBuildCxdLpV4LayerDef.mockReturnValueOnce(new Uint8Array([20]));
        expect(cxdlpDocument!.buildPrefix!("{}", "cxdlpv4", new Uint32Array([1]))).toEqual([
            new Uint8Array([10]),
            new Uint8Array([20])
        ]);
        cxdlpDocument!.checksumUpdate!(1, new Uint8Array([2]));
        expect(cxdlpDocument!.buildChecksumTrailer!(0x01020304)).toEqual(new Uint8Array([1, 2, 3, 4]));
        expect(mockBuildCxdLpV4DocumentHeader).toHaveBeenCalledWith("{}", "cxdlpv4", new Uint32Array([1]));
        expect(mockBuildCxdLpV4LayerDef).toHaveBeenCalledWith("{}", 0, 41, 1);
        expect(mockCxdLpCrc32Update).toHaveBeenCalledWith(1, new Uint8Array([2]));
        const encryptedCtbAssembly = mod.createStreamingDocumentAssemblyHelpers("encrypted-ctb");
        mockBuildDocumentFromLayers.mockReturnValueOnce(new Uint8Array([30]));
        expect(
            encryptedCtbAssembly.streamingDocument?.buildDocument?.("{}", "encrypted-ctb", undefined, [
                new Uint8Array([3])
            ])
        ).toEqual(new Uint8Array([30]));
        expect(mockBuildDocumentFromLayers).toHaveBeenCalledWith(
            "{}",
            "encrypted-ctb",
            [new Uint8Array([3])],
            undefined
        );
        mockBuildDocumentFromLayers.mockReturnValueOnce(new Uint8Array([31]));
        encryptedCtbAssembly.streamingDocument?.buildDocument?.("{}", "encrypted-ctb", 515, [new Uint8Array([4])]);
        expect(mockBuildDocumentFromLayers).toHaveBeenCalledWith("{}", "encrypted-ctb", [new Uint8Array([4])], 515);
        mockListSupportedFormats.mockReturnValueOnce('[{"name":"ctb"}]');
        expect(mod.getSupportedFormats()).toEqual([{ name: "ctb" }]);
        mockDiagnoseExport.mockReturnValueOnce(
            JSON.stringify({
                warnings: [
                    {
                        code: "ANYCUBIC_MACHINE_PROFILE_MISMATCH",
                        severity: "warning",
                        message: "Warning: mismatch",
                        source: {
                            format: null,
                            display_name: null,
                            extension: null,
                            resolution_x: 8520,
                            resolution_y: 4320,
                            display_width_mm: 55.44,
                            display_height_mm: 98.637
                        },
                        target: {
                            format: "pw0",
                            display_name: "Photon Zero",
                            extension: "pw0",
                            resolution_x: 480,
                            resolution_y: 854,
                            display_width_mm: 55.4,
                            display_height_mm: 98.63
                        }
                    }
                ]
            })
        );
        expect(
            mod.diagnoseExport({ metadata: { resolution_x: 8520, resolution_y: 4320 }, targetFormat: "pw0" })
                .warnings[0]?.code
        ).toBe("ANYCUBIC_MACHINE_PROFILE_MISMATCH");
        expect(mockDiagnoseExport).toHaveBeenCalledWith(expect.any(String), "pw0");
    });

    it("normalizes structured parse errors from browser helpers into Error instances / 规范化 结构化 parse 错误 来自 browser helpers 到 Error instances", async () => {
        const mod = await import("../../src/index.js");
        mockParseHeader.mockImplementationOnce(() => {
            throw {
                code: "E_INVALID_MAGIC",
                message: "Invalid magic number",
                retryable: false
            };
        });

        try {
            mod.parseBrowserFileHeader(new Uint8Array([1, 2, 3]), "ctb");
            throw new Error("Expected parseBrowserFileHeader to throw");
        } catch (error: unknown) {
            expect(error).toBeInstanceOf(Error);
            expect((error as Error).message).toBe("Invalid magic number");
            expect((error as { code?: string }).code).toBe("E_INVALID_MAGIC");
        }
    });
});

describe("public package surface / 公开 package surface", () => {
    it("keeps compatibility exports available / 保留兼容导出", async () => {
        const mod = await import("../../src/index.js");

        expect(typeof mod.initSlicerConverter).toBe("function");
        expect(typeof mod.convertSlicerFile).toBe("function");
        expect(typeof mod.listSlicerExportTargetMatrix).toBe("function");
        expect(typeof mod.createSlicerWorkerRuntime).toBe("function");
        expect(typeof mod.prepareZipEntryWithTimings).toBe("function");
        expect(typeof mod.slicerLayerCodecRuntime).toBe("object");
    });

    it("keeps root runtime exports explicit / 保持 root runtime exports explicit", async () => {
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(indexSource).not.toContain('export * from "./browser/index.js"');

        const mod = await import("../../src/index.js");
        const runtimeExports = Object.keys(mod).sort();

        expect(runtimeExports).toEqual(EXPECTED_ROOT_RUNTIME_EXPORTS);
        expect(runtimeExports).not.toContain("default");
        expect(runtimeExports).not.toContain("getCompressedData");
        expect(runtimeExports).not.toContain("parseZipDirectory");
        expect(runtimeExports.filter((name) => name.toLowerCase().includes("benchmark"))).toEqual([]);
    });

    it("does not keep a default initializer export / 不保留 default initializer export", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).not.toContain("export default initSlicerConverter");
        expect(indexSource).not.toContain("default,");
    });

    it("does not expose the full raw runtime singleton / 不暴露 full raw runtime singleton", () => {
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(existsSync(path.join(process.cwd(), "src/browser/runtime.ts"))).toBe(false);
        expect(indexSource).not.toContain("slicerWasmRuntime");
    });

    it("does not keep the thin package runtime initializer wrapper / 不保留 thin package runtime initializer wrapper", () => {
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(indexSource).not.toContain("initSlicerConverterRuntime");
    });

    it("exposes typed supported-format helpers instead of the raw JSON export / 暴露 typed supported-format helpers 而不是 raw JSON export", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).not.toContain("list_supported_formats: typeof list_supported_formats");
        expect(indexSource).not.toMatch(/\blist_supported_formats\b/);
    });

    it("exposes typed archive-plan helpers instead of raw JSON exports / 暴露 typed archive-plan helpers 而不是 raw JSON exports", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");

        expect(apiSource).not.toContain("build_archive_metadata_entries: typeof build_archive_metadata_entries");
        expect(apiSource).not.toContain("describe_archive_layer_entries: typeof describe_archive_layer_entries");
    });

    it("exposes document assembly helpers instead of raw header/footer exports / 暴露 document assembly helpers 而不是 raw header/footer exports", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const assemblySource = readFileSync(
            path.join(process.cwd(), "src/browser/output/document-assembly.ts"),
            "utf8"
        );

        expect(apiSource).not.toContain("build_header: typeof build_header");
        expect(apiSource).not.toContain("diagnose_export: typeof diagnose_export");
        expect(apiSource).not.toContain("finalize_header: typeof finalize_header");
        expect(apiSource).not.toContain("build_footer: typeof build_footer");
        expect(assemblySource).not.toContain("=> build_header(");
        expect(assemblySource).not.toContain("=> finalize_header(");
        expect(assemblySource).not.toContain("=> build_footer(");
    });

    it("keeps CXDLPv4 raw streaming helpers behind typed document assembly helpers / 保持 CXDLPv4 raw streaming helpers behind typed document assembly helpers", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const assemblySource = readFileSync(
            path.join(process.cwd(), "src/browser/output/document-assembly.ts"),
            "utf8"
        );

        expect(apiSource).not.toContain("build_cxdlpv4_document_header: typeof build_cxdlpv4_document_header");
        expect(apiSource).not.toContain("build_cxdlpv4_layer_def: typeof build_cxdlpv4_layer_def");
        expect(apiSource).not.toContain("build_document_from_layers: typeof build_document_from_layers");
        expect(apiSource).not.toContain("cxdlp_crc32_update: typeof cxdlp_crc32_update");
        expect(assemblySource).not.toContain("=> cxdlp_crc32_update(");
        expect(assemblySource).not.toContain("= build_cxdlpv4_document_header(");
        expect(assemblySource).not.toContain("chunks.push(build_cxdlpv4_layer_def(");
    });

    it("does not expose the CXDLPv4-specific raw CRC helper / 不暴露 CXDLPv4-specific raw CRC helper", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).not.toContain("cxdlp_crc32_bytes: typeof cxdlp_crc32_bytes");
        expect(indexSource).not.toMatch(/\bcxdlp_crc32_bytes\b/);
    });

    it("does not expose the CTB-specific raw bitplane merge helper / 不暴露 CTB-specific raw bitplane merge helper", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).not.toContain("merge_bitplanes: typeof merge_bitplanes");
        expect(indexSource).not.toMatch(/\bmerge_bitplanes\b/);
    });

    it("does not expose raw codec functions outside the layer codec adapter / 不在 layer codec adapter 外暴露 raw codec functions", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).not.toContain("decode_layer: typeof decode_layer");
        expect(apiSource).not.toContain("decode_layer_preview: typeof decode_layer_preview");
        expect(apiSource).not.toContain("decode_encode_layer: typeof decode_encode_layer");
        expect(apiSource).not.toContain("decode_encode_layer_stats: typeof decode_encode_layer_stats");
        expect(apiSource).not.toContain("encode_layer: typeof encode_layer");
        expect(indexSource).not.toMatch(/\bdecode_layer\b/);
        expect(indexSource).not.toMatch(/\bdecode_layer_preview\b/);
        expect(indexSource).not.toMatch(/\bdecode_encode_layer\b/);
        expect(indexSource).not.toMatch(/\bdecode_encode_layer_stats\b/);
        expect(indexSource).not.toMatch(/\bencode_layer\b/);
    });

    it("keeps layer codec adapter methods camelCase / 保持 layer codec adapter methods 为 camelCase", async () => {
        const codecSource = readFileSync(path.join(process.cwd(), "src/browser/runtime/codec.ts"), "utf8");
        const mod = await import("../../src/index.js");

        expect(Object.keys(mod.slicerLayerCodecRuntime).sort()).toEqual(
            ["decodeEncodeLayer", "decodeEncodeLayerStats", "decodeLayer", "decodeLayerPreview", "encodeLayer"].sort()
        );
        expect(codecSource).not.toContain("decode_encode_layer: typeof decode_encode_layer");
        expect(codecSource).not.toContain("decode_encode_layer_stats: typeof decode_encode_layer_stats");
        expect(codecSource).not.toContain("decode_layer: typeof decode_layer");
        expect(codecSource).not.toContain("decode_layer_preview: typeof decode_layer_preview");
        expect(codecSource).not.toContain("encode_layer: typeof encode_layer");
        expect(codecSource).not.toContain("decodeEncodeLayer: typeof decode_encode_layer");
        expect(codecSource).not.toContain("decodeEncodeLayerStats: typeof decode_encode_layer_stats");
        expect(codecSource).not.toContain("decodeLayer: typeof decode_layer");
        expect(codecSource).not.toContain("encodeLayer: typeof encode_layer");
        expect(codecSource).not.toContain("Parameters<typeof decode_layer_preview_raw>");
        expect(codecSource).not.toContain("decodeEncodeLayer: decode_encode_layer");
        expect(codecSource).not.toContain("decodeEncodeLayerStats: decode_encode_layer_stats");
        expect(codecSource).not.toContain("decodeLayer: decode_layer");
        expect(codecSource).not.toContain("encodeLayer: encode_layer");
    });

    it("exposes browser header parsing helper instead of the raw parse export / 暴露 browser header parsing helper 而不是 raw parse export", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");
        const headerSource = readFileSync(path.join(process.cwd(), "src/browser/format/header.ts"), "utf8");

        expect(apiSource).not.toContain("parse_header: typeof parse_header");
        expect(indexSource).not.toMatch(/\bparse_header\b/);
        expect(headerSource).not.toContain("JSON.parse(parse_header(");
    });

    it("keeps public browser helper docs on typed package names / 保持 public browser helper docs 使用 typed package names", () => {
        const headerSource = readFileSync(path.join(process.cwd(), "src/browser/format/header.ts"), "utf8");
        const typesSource = readFileSync(path.join(process.cwd(), "src/browser/types.ts"), "utf8");
        const publicDocs = [headerSource, typesSource].join("\n");

        expect(publicDocs).not.toContain("WASM `parse_header`");
        expect(publicDocs).not.toContain("WASM `encode_layer`");
        expect(publicDocs).not.toContain("WASM `build_header`");
    });

    it("exposes a typed CRC helper instead of the raw snake-case CRC export / 暴露 typed CRC helper 而不是 raw snake-case CRC export", () => {
        const apiSource = readFileSync(path.join(process.cwd(), "src/api.ts"), "utf8");
        const indexSource = readFileSync(path.join(process.cwd(), "src/index.ts"), "utf8");

        expect(apiSource).toContain("crc32Bytes");
        expect(apiSource).not.toContain("crc32_bytes: typeof crc32_bytes");
        expect(apiSource).not.toContain("return crc32_bytes(data)");
        expect(indexSource).toContain("crc32Bytes");
        expect(indexSource).not.toMatch(/\bcrc32_bytes\b/);
    });

    it("does not keep an unused browser barrel / 不保留 unused browser barrel", () => {
        const packageJson = JSON.parse(readFileSync(path.join(process.cwd(), "package.json"), "utf8")) as {
            exports?: Record<string, unknown>;
        };

        expect(existsSync(path.join(process.cwd(), "src/browser/index.ts"))).toBe(false);
        expect(packageJson.exports).not.toHaveProperty("./browser");
        expect(packageJson.exports).not.toHaveProperty("./browser/index");
    });

    it("keeps worker entry internal during document-opening sinkdown / document-opening 下沉期间保持 worker entry internal", () => {
        const packageJson = JSON.parse(readFileSync(path.join(process.cwd(), "package.json"), "utf8")) as {
            exports?: Record<string, unknown>;
        };

        expect(packageJson.exports).not.toHaveProperty("./browser-worker");
    });

    it("does not keep a standalone debug WASM facade / 不保留 standalone debug WASM facade", () => {
        const packageJson = JSON.parse(readFileSync(path.join(process.cwd(), "package.json"), "utf8")) as {
            exports?: Record<string, unknown>;
            scripts?: Record<string, string>;
        };
        const rootPackageJson = JSON.parse(readFileSync(path.join(process.cwd(), "../../package.json"), "utf8")) as {
            scripts?: Record<string, string>;
        };

        expect(existsSync(path.join(process.cwd(), "src/debug.ts"))).toBe(false);
        expect(packageJson.exports).not.toHaveProperty("./debug");
        expect(packageJson.scripts).not.toHaveProperty("build:dev");
        expect(packageJson.scripts).not.toHaveProperty("build:all");
        expect(rootPackageJson.scripts).not.toHaveProperty("build:wasm:all");
    });

    it("keeps init parameter tests free of unrelated raw runtime mocks / 保持 init parameter tests 不 mock unrelated raw runtime", () => {
        const initParamsSource = readFileSync(path.join(process.cwd(), "tests/unit/init-params.test.ts"), "utf8");

        for (const rawExport of [
            "parse_header",
            "decode_layer",
            "decode_layer_preview",
            "decode_encode_layer",
            "decode_encode_layer_stats",
            "encode_layer",
            "build_archive_metadata_entries",
            "build_cxdlpv4_document_header",
            "build_cxdlpv4_layer_def",
            "build_document_from_layers",
            "build_header",
            "build_footer",
            "finalize_header",
            "describe_archive_layer_entries",
            "cxdlp_crc32_update"
        ]) {
            expect(initParamsSource).not.toContain(`${rawExport}:`);
        }
    });
});

describe("browser module structure / 浏览器模块结构", () => {
    it("keeps conversion pipeline below the public API facade / 保持 conversion pipeline 位于 public API facade 下层", () => {
        const pipelineSource = readFileSync(path.join(process.cwd(), "src/browser/conversion/pipeline.ts"), "utf8");

        expect(pipelineSource).not.toContain('from "../../api.js"');
    });

    it("keeps browser implementation files below the public API facade / 保持 browser 实现文件位于 public API facade 下层", () => {
        const srcRoot = path.join(process.cwd(), "src");
        const browserRoot = path.join(srcRoot, "browser");
        const publicApiFacade = path.join(srcRoot, "api.ts");
        const offenders = listTypeScriptImplementationFiles(browserRoot).flatMap((filePath) => {
            const source = readFileSync(filePath, "utf8");
            const imports = findImportSpecifiers(source);
            const forbiddenImports = imports.filter((specifier) => {
                const resolvedPath = resolveRelativeTypeScriptImport(filePath, specifier);
                return resolvedPath === publicApiFacade;
            });

            return forbiddenImports.length > 0
                ? [
                      {
                          file: toPackageRelativePath(filePath),
                          imports: forbiddenImports
                      }
                  ]
                : [];
        });

        expect(offenders).toEqual([]);
    });

    it("keeps format behavior contracts in the format module / 保持 format behavior contract 归属 format 模块", () => {
        const documentSource = readFileSync(path.join(process.cwd(), "src/browser/document/source.ts"), "utf8");

        expect(documentSource).not.toContain("export interface DocumentOpener");
        expect(documentSource).not.toContain("export interface BrowserFormatBehavior");
    });

    it("keeps ZIP format-specific parsers grouped by format / 保持 ZIP 格式专属 parser 按格式聚合", () => {
        const zipFormatsRoot = path.join(process.cwd(), "src/browser/archive/zip/formats");

        expect(existsSync(path.join(zipFormatsRoot, "anycubic-zip.ts"))).toBe(true);
        expect(existsSync(path.join(zipFormatsRoot, "chitubox-zip.ts"))).toBe(true);
        expect(existsSync(path.join(zipFormatsRoot, "sl1.ts"))).toBe(true);
        expect(existsSync(path.join(zipFormatsRoot, "vdt.ts"))).toBe(true);
    });

    it("names the built-in format behavior registry explicitly / 显式命名内建 format behavior registry", () => {
        const formatIndexSource = readFileSync(path.join(process.cwd(), "src/browser/format/index.ts"), "utf8");
        const browserBehaviorsSource = readFileSync(
            path.join(process.cwd(), "src/browser/format/browser-format-behaviors.ts"),
            "utf8"
        );

        expect(existsSync(path.join(process.cwd(), "src/browser/format/behaviors/registry.ts"))).toBe(true);
        expect(formatIndexSource).not.toContain("./behaviors/index.js");
        expect(browserBehaviorsSource).not.toContain("./behaviors/index.js");
    });

    it("keeps document assembly in the output module / 保持 document assembly 归属 output 模块", () => {
        const wasmBinaryOutputSource = readFileSync(
            path.join(process.cwd(), "src/browser/output/wasm-binary.ts"),
            "utf8"
        );
        const binaryOutputSource = readFileSync(path.join(process.cwd(), "src/browser/output/binary.ts"), "utf8");

        expect(existsSync(path.join(process.cwd(), "src/browser/output/document-assembly.ts"))).toBe(true);
        expect(existsSync(path.join(process.cwd(), "src/browser/archive/document-assembly.ts"))).toBe(false);
        expect(wasmBinaryOutputSource).not.toContain("../archive/document-assembly.js");
        expect(binaryOutputSource).not.toContain("../archive/document-assembly.js");
    });

    it("keeps browser root limited to module directories and shared primitives / 保持 browser 根目录只包含模块目录和共享 primitives", () => {
        const browserRoot = path.join(process.cwd(), "src/browser");
        const allowedRootFiles = new Set(["crc.ts", "types.ts"]);
        const entries = readdirSync(browserRoot, { withFileTypes: true });
        const unexpectedFiles = entries
            .filter((entry) => entry.isFile())
            .map((entry) => entry.name)
            .filter((entryName) => !allowedRootFiles.has(entryName))
            .sort();
        const expectedDirectories = entries
            .filter((entry) => entry.isDirectory())
            .map((entry) => entry.name)
            .sort();

        expect(unexpectedFiles).toEqual([]);
        expect(expectedDirectories).toEqual(BROWSER_MODULE_DIRECTORIES);
    });

    it("keeps browser implementations from importing local barrels or root package barrel / 禁止 browser 实现回流导入本地 barrel 或 package root barrel", () => {
        const srcRoot = path.join(process.cwd(), "src");
        const browserRoot = path.join(srcRoot, "browser");
        const packageRootBarrel = path.join(srcRoot, "index.ts");
        const guardedModuleBarrels = new Set(
            BROWSER_MODULE_DIRECTORIES.map((directory) => path.join(browserRoot, directory, "index.ts"))
        );
        const offenders = BROWSER_MODULE_DIRECTORIES.flatMap((directory) =>
            listTypeScriptImplementationFiles(path.join(browserRoot, directory)).flatMap((filePath) => {
                const source = readFileSync(filePath, "utf8");
                const imports = findImportSpecifiers(source);
                const forbiddenImports = imports.flatMap((specifier) => {
                    if (specifier === "@innovgame/slicer-converter") {
                        return [specifier];
                    }

                    const resolvedPath = resolveRelativeTypeScriptImport(filePath, specifier);
                    if (!resolvedPath) {
                        return [];
                    }

                    if (resolvedPath === packageRootBarrel || guardedModuleBarrels.has(resolvedPath)) {
                        return [specifier];
                    }

                    return [];
                });

                return forbiddenImports.length > 0
                    ? [
                          {
                              file: toPackageRelativePath(filePath),
                              imports: forbiddenImports
                          }
                      ]
                    : [];
            })
        );

        expect(offenders).toEqual([]);
    });
});

function listTypeScriptImplementationFiles(directory: string): string[] {
    return readdirSync(directory).flatMap((entry) => {
        const entryPath = path.join(directory, entry);
        const stats = statSync(entryPath);
        if (stats.isDirectory()) {
            return listTypeScriptImplementationFiles(entryPath);
        }

        return entry.endsWith(".ts") && entry !== "index.ts" ? [entryPath] : [];
    });
}

function findImportSpecifiers(source: string): string[] {
    const specifiers: string[] = [];
    const importExportPattern = /\b(?:import|export)\s+(?:type\s+)?(?:[^"']*?\s+from\s+)?["'](?<specifier>[^"']+)["']/g;
    const dynamicImportPattern = /\bimport\s*\(\s*["'](?<specifier>[^"']+)["']\s*\)/g;

    for (const match of source.matchAll(importExportPattern)) {
        if (match.groups?.specifier) {
            specifiers.push(match.groups.specifier);
        }
    }

    for (const match of source.matchAll(dynamicImportPattern)) {
        if (match.groups?.specifier) {
            specifiers.push(match.groups.specifier);
        }
    }

    return specifiers;
}

function resolveRelativeTypeScriptImport(importingFile: string, specifier: string): string | null {
    if (!specifier.startsWith(".")) {
        return null;
    }

    const withoutExtension = specifier.replace(/\.(?:mjs|cjs|js|ts)$/, "");
    return path.normalize(path.join(path.dirname(importingFile), `${withoutExtension}.ts`));
}

function toPackageRelativePath(filePath: string): string {
    return path.relative(process.cwd(), filePath).replaceAll(path.sep, "/");
}
