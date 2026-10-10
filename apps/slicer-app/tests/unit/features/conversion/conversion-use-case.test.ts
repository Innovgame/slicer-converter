import { beforeEach, describe, expect, it, vi } from "vitest";

import { createSlicerDocument, type SlicerDocument } from "@slicer-converter/wasm";
import type * as SlicerWasm from "@slicer-converter/wasm";
import type {
    ManagedSlicerWorker as ManagedConversionWorker,
    ResolvedSlicerExportTarget,
    SlicerWorkerPoolState as WorkerPoolState
} from "@slicer-converter/wasm";
import { formatFact } from "../../support/format-facts";

type OpenedDocument = SlicerDocument;

const mocks = vi.hoisted(() => ({
    convertSlicerFileToBlob: vi.fn(),
    convertSlicerFileToSink: vi.fn()
}));

vi.mock("@slicer-converter/wasm", async (importOriginal) => {
    const actual = await importOriginal<typeof SlicerWasm>();
    return {
        ...actual,
        convertSlicerFileToBlob: mocks.convertSlicerFileToBlob,
        convertSlicerFileToSink: mocks.convertSlicerFileToSink
    };
});

const originalPicker = Object.getOwnPropertyDescriptor(globalThis, "showSaveFilePicker");

describe("runConversionUseCase / runConversionUseCase 用例", () => {
    beforeEach(async () => {
        restorePicker();
        vi.clearAllMocks();
        await syncFormatFactsForTest();
    });

    it("delegates conversion to the wasm package and downloads Blob output / 委托转换到 wasm package 并下载 Blob 输出", async () => {
        Reflect.deleteProperty(globalThis, "showSaveFilePicker");
        const blob = new Blob([new Uint8Array([1, 2, 3])], { type: "application/octet-stream" });
        mocks.convertSlicerFileToBlob.mockResolvedValueOnce({
            blob,
            fileName: "sample.ctb",
            sourceFormat: "goo",
            targetFormat: "ctb",
            metadata: { layer_count: 1 },
            warnings: [],
            timings: {
                totalMs: 10,
                setupMs: 1,
                layerPhaseMs: 2,
                computeMs: 3,
                assembleMs: 4,
                finalizeOnlyMs: 4
            }
        });
        const downloadBlob = vi.fn();
        const workerPool = createWorkerPool();
        const updateProgress = vi.fn();

        const result = await runUseCase({ workerPool, outputFileName: "custom-name", updateProgress, downloadBlob });

        expect(mocks.convertSlicerFileToBlob).toHaveBeenCalledWith(
            expect.objectContaining({
                file: expect.any(File),
                fileName: "custom-name.ctb",
                target: expect.objectContaining({
                    requestedTargetFormat: "ctb",
                    resolvedTargetFormat: "ctb"
                }),
                signal: expect.any(AbortSignal),
                worker: { kind: "pool", pool: workerPool },
                onProgress: expect.any(Function)
            })
        );
        expect(mocks.convertSlicerFileToBlob).toHaveBeenCalledWith(
            expect.not.objectContaining({ output: expect.anything() })
        );
        expect(mocks.convertSlicerFileToSink).not.toHaveBeenCalled();
        const onProgress = mocks.convertSlicerFileToBlob.mock.calls[0]?.[0].onProgress as (progress: {
            current: number;
            total: number;
            label: string;
        }) => void;
        onProgress({ current: 1, total: 2, label: "Converting layer 1/2..." });
        expect(updateProgress).toHaveBeenCalledWith(1, 2, "Converting layer 1/2...");
        expect(downloadBlob).toHaveBeenCalledWith({ blob, downloadName: "custom-name.ctb" });
        expect(result.statusMessage).toContain("Converted 1 layers to CTB");
    });

    it("falls back to source filename when output filename is blank / 输出文件名为空时回退源文件名", async () => {
        Reflect.deleteProperty(globalThis, "showSaveFilePicker");
        const blob = new Blob([new Uint8Array([1])], { type: "application/octet-stream" });
        mocks.convertSlicerFileToBlob.mockResolvedValueOnce({
            blob,
            fileName: "sample.ctb",
            sourceFormat: "goo",
            targetFormat: "ctb",
            metadata: { layer_count: 1 },
            warnings: [],
            timings: {
                totalMs: 10,
                setupMs: 1,
                layerPhaseMs: 2,
                computeMs: 3,
                assembleMs: 4,
                finalizeOnlyMs: 4
            }
        });
        const downloadBlob = vi.fn();

        await runUseCase({ outputFileName: "   ", downloadBlob });

        expect(mocks.convertSlicerFileToBlob).toHaveBeenCalledWith(expect.objectContaining({ fileName: "sample.ctb" }));
        expect(downloadBlob).toHaveBeenCalledWith({ blob, downloadName: "sample.ctb" });
    });

    it("keeps streaming output on disk without invoking the Blob download adapter / 保持 streaming output on 磁盘 不调用 Blob 下载 adapter", async () => {
        const writable = installStreamingPicker();
        mocks.convertSlicerFileToSink.mockResolvedValueOnce({
            blob: null,
            fileName: "sample.vdt",
            sourceFormat: "goo",
            targetFormat: "vdt",
            metadata: { layer_count: 1 },
            warnings: [],
            timings: {
                totalMs: 12,
                setupMs: 1,
                layerPhaseMs: 2,
                computeMs: 3,
                assembleMs: 4,
                finalizeOnlyMs: 5
            }
        });
        const downloadBlob = vi.fn();

        const result = await runUseCase({
            target: { requestedTargetFormat: "vdt", resolvedTargetFormat: "vdt", statusMessage: null },
            downloadBlob
        });

        expect(mocks.convertSlicerFileToSink).toHaveBeenCalledWith(
            expect.objectContaining({
                output: expect.any(Object),
                worker: expect.objectContaining({ kind: "pool" })
            })
        );
        expect(mocks.convertSlicerFileToBlob).not.toHaveBeenCalled();
        expect(downloadBlob).not.toHaveBeenCalled();
        expect(writable.abort).not.toHaveBeenCalled();
        expect(result.statusMessage).toContain("Output streamed to disk");
        expect(result.statusMessage).toContain("finalize-only 5.0ms");
    });

    it("aborts app-owned output sink when package conversion fails / package 转换失败时中止 app 拥有的 output sink", async () => {
        const writable = installStreamingPicker();
        mocks.convertSlicerFileToSink.mockRejectedValueOnce(new Error("worker exploded"));

        await expect(runUseCase()).rejects.toThrow("worker exploded");

        expect(writable.abort).toHaveBeenCalledWith("worker exploded");
    });
});

async function runUseCase(
    input: {
        document?: OpenedDocument;
        workerPool?: WorkerPoolState<ManagedConversionWorker>;
        target?: ResolvedSlicerExportTarget;
        outputFileName?: string;
        signal?: AbortSignal;
        updateProgress?: (current: number, total: number, label: string) => void;
        downloadBlob?: (download: { blob: Blob; downloadName: string }) => void;
    } = {}
) {
    const { runConversionUseCase } = await import("../../../../src/features/conversion/model/conversion-use-case");
    return runConversionUseCase({
        document: input.document ?? createRawDocument(),
        workerPool: input.workerPool ?? createWorkerPool(),
        target: input.target ?? {
            requestedTargetFormat: "ctb",
            resolvedTargetFormat: "ctb",
            statusMessage: null
        },
        outputFileName: input.outputFileName,
        signal: input.signal ?? new AbortController().signal,
        updateProgress: input.updateProgress ?? vi.fn(),
        downloadBlob: input.downloadBlob ?? vi.fn()
    });
}

function createRawDocument(): OpenedDocument {
    const file = new File([new Uint8Array([1, 2, 3])], "sample.goo");
    return createSlicerDocument({
        sourceFile: file,
        parsedHeader: {
            format: "goo",
            metadata: {
                resolution_x: 64,
                resolution_y: 32,
                layer_count: 1
            },
            layers: [{ offset: 0, size: 3 }]
        } as never,
        layerSource: { kind: "raw-file" }
    });
}

function createWorkerPool(): WorkerPoolState<ManagedConversionWorker> {
    const worker = { busy: false } as ManagedConversionWorker;
    return {
        size: 1,
        workers: [worker],
        findIdleWorker: vi.fn(() => worker),
        ensureWorkers: vi.fn(async () => [worker]),
        pauseIdleTermination: vi.fn(),
        scheduleIdleTermination: vi.fn(),
        terminateAll: vi.fn()
    };
}

function installStreamingPicker() {
    const writable = {
        write: vi.fn(async (_data: Uint8Array | Blob | string) => undefined),
        seek: vi.fn(async (_position: number) => undefined),
        close: vi.fn(async () => undefined),
        abort: vi.fn(async (_reason?: unknown) => undefined)
    };
    Object.defineProperty(globalThis, "showSaveFilePicker", {
        configurable: true,
        value: vi.fn(async () => ({ createWritable: vi.fn(async () => writable) }))
    });
    return writable;
}

function restorePicker(): void {
    if (originalPicker) {
        Object.defineProperty(globalThis, "showSaveFilePicker", originalPicker);
        return;
    }
    Reflect.deleteProperty(globalThis, "showSaveFilePicker");
}

async function syncFormatFactsForTest(): Promise<void> {
    const { syncFormatFacts } = await import("@slicer-converter/wasm");
    syncFormatFacts([
        formatFact({
            name: "ctb",
            display_name: "CTB",
            aliases: ["ctb"],
            extensions: ["ctb"],
            default_extension: "ctb",
            is_zip: false,
            capabilities: {
                can_import: true,
                can_export: true,
                is_archive: false,
                supports_preview: true,
                supports_anti_alias: false,
                supports_per_layer_settings: false,
                requires_full_document_parse: false,
                supports_encryption: false
            }
        }),
        formatFact({
            name: "vdt",
            display_name: "VDT",
            aliases: ["vdt"],
            extensions: ["vdt"],
            default_extension: "vdt",
            is_zip: true,
            capabilities: {
                can_import: true,
                can_export: true,
                is_archive: true,
                supports_preview: true,
                supports_anti_alias: false,
                supports_per_layer_settings: false,
                requires_full_document_parse: true,
                supports_encryption: false
            }
        })
    ]);
}
