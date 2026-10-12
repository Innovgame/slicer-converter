import { describe, expect, it, vi } from "vitest";

import {
    asSlicerFormatId,
    convertSlicerFileToBlobWithDeps,
    convertSlicerFileToSinkWithDeps,
    convertSlicerFileWithDeps,
    syncSupportedFormatInfosForValidation
} from "../../src/index.js";
import type { PatchableSlicerOutputSink, SlicerConversionDeps, SlicerDocument } from "../../src/index.js";
import { formatFact } from "./support/format-facts.js";

describe("convertSlicerFile / 高层转换 API", () => {
    it("rejects cancellation during document cleanup / 文档清理期间取消时拒绝成功结果", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: [], extensions: ["goo"], is_zip: false })
        ]);
        const deps = createDeps(new Blob([new Uint8Array([1])]));
        const controller = new AbortController();
        vi.mocked(deps.document.close).mockImplementationOnce(async () => {
            controller.abort();
        });
        await expect(
            convertSlicerFileWithDeps(
                { file: deps.document.sourceFile, target: "goo", workerCount: 1, signal: controller.signal },
                deps
            )
        ).rejects.toMatchObject({ name: "AbortError" });
    });
    it("rejects cancellation during opening and closes the opened document / 打开期间取消时拒绝并关闭文档", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: [], extensions: ["goo"], is_zip: false })
        ]);
        const deps = createDeps(new Blob([new Uint8Array([1])]));
        const controller = new AbortController();
        deps.openDocument.mockImplementationOnce(async () => {
            controller.abort();
            return deps.document;
        });
        await expect(
            convertSlicerFileWithDeps(
                { file: deps.document.sourceFile, target: "goo", signal: controller.signal, workerCount: 1 },
                deps
            )
        ).rejects.toMatchObject({ name: "AbortError" });
        expect(deps.document.close).toHaveBeenCalledOnce();
        expect(deps.runJob).not.toHaveBeenCalled();
    });

    it("preserves the primary failure when abort and close also fail / 清理失败时保留原始转换错误", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: [], extensions: ["goo"], is_zip: false })
        ]);
        const deps = createDeps(null);
        deps.runJob.mockRejectedValueOnce(new Error("primary failure"));
        deps.sink.abort.mockRejectedValueOnce(new Error("abort failure"));
        vi.mocked(deps.document.close).mockRejectedValueOnce(new Error("close failure"));
        await expect(
            convertSlicerFileWithDeps({ file: deps.document.sourceFile, target: "goo", workerCount: 1 }, deps)
        ).rejects.toThrow("primary failure");
        expect(deps.document.close).toHaveBeenCalledOnce();
    });

    it("opens a file, reports progress, and returns a named Blob result / 打开文件并返回命名 Blob 结果", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", display_name: "VDT", aliases: ["vdt"], extensions: ["vdt"], is_zip: true })
        ]);
        const progress = vi.fn();
        const blob = new Blob([new Uint8Array([1, 2, 3])]);
        const deps = createDeps(blob);

        const result = await convertSlicerFileWithDeps(
            {
                file: new File([new Uint8Array([1, 2, 3])], "sample.goo"),
                target: "vdt",
                onProgress: progress,
                workerCount: 1
            },
            deps
        );

        expect(deps.init).toHaveBeenCalledTimes(1);
        expect(deps.openDocument).toHaveBeenCalledWith(
            expect.objectContaining({ fileName: undefined, extension: undefined })
        );
        expect(deps.runJob).toHaveBeenCalledWith(
            expect.objectContaining({
                targetFormat: "vdt",
                outputFileName: "sample.vdt",
                workers: deps.workers
            })
        );
        expect(result).toMatchObject({
            blob,
            fileName: "sample.vdt",
            sourceFormat: "goo",
            targetFormat: "vdt",
            warnings: []
        });
        expect(progress).toHaveBeenCalledWith({
            phase: "opening",
            current: 0,
            total: 0,
            label: "Opening file..."
        });
    });

    it("aborts the sink and closes the document when conversion fails / 转换失败时中止 sink 并关闭文档", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const deps = createDeps(null);
        deps.runJob.mockRejectedValueOnce(new Error("worker exploded"));

        await expect(
            convertSlicerFileWithDeps(
                {
                    file: new File([new Uint8Array([1])], "sample.goo"),
                    target: "goo"
                },
                deps
            )
        ).rejects.toThrow("worker exploded");

        expect(deps.sink.abort).toHaveBeenCalledWith("worker exploded");
        expect(deps.document.close).toHaveBeenCalledTimes(1);
        expect(deps.pool.terminateAll).toHaveBeenCalledTimes(1);
    });

    it("converts to Blob with a non-null Blob result / 转换为 Blob 并返回 non-null Blob result", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", display_name: "VDT", aliases: ["vdt"], extensions: ["vdt"], is_zip: true })
        ]);
        const blob = new Blob([new Uint8Array([7, 8, 9])]);
        const deps = createDeps(blob);

        const result = await convertSlicerFileToBlobWithDeps(
            {
                file: new File([new Uint8Array([1])], "sample.goo"),
                target: "vdt",
                worker: { kind: "auto", count: 2 }
            },
            deps
        );

        expect(result.blob).toBe(blob);
        expect(result.fileName).toBe("sample.vdt");
        expect(deps.createDefaultSink).toHaveBeenCalledTimes(1);
        expect(deps.createWorkerPool).toHaveBeenCalledWith(expect.objectContaining({ count: 2 }));
    });

    it("throws when Blob facade receives a null Blob / Blob facade 收到 null Blob 时抛错", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const deps = createDeps(null);

        await expect(
            convertSlicerFileToBlobWithDeps(
                {
                    file: new File([new Uint8Array([1])], "sample.goo"),
                    target: "goo",
                    worker: { kind: "auto", count: 1 }
                },
                deps
            )
        ).rejects.toThrow("Blob conversion completed without a Blob output");
    });

    it("converts to a caller sink and preserves null Blob / 转换到 caller sink 并保持 null Blob", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", display_name: "VDT", aliases: ["vdt"], extensions: ["vdt"], is_zip: true })
        ]);
        const deps = createDeps(null);
        const output: PatchableSlicerOutputSink = {
            streaming: true,
            write: vi.fn(),
            patchHeader: vi.fn(),
            finish: vi.fn(async () => null),
            abort: vi.fn(async () => undefined)
        };

        const result = await convertSlicerFileToSinkWithDeps(
            {
                file: new File([new Uint8Array([1])], "sample.goo"),
                target: "vdt",
                output,
                worker: { kind: "pool", pool: deps.pool }
            },
            deps
        );

        expect(result.blob).toBeNull();
        expect(deps.createDefaultSink).not.toHaveBeenCalled();
        expect(deps.createWorkerPool).not.toHaveBeenCalled();
        expect(deps.runJob).toHaveBeenCalledWith(expect.objectContaining({ sink: output }));
        expect(deps.pool.ensureWorkers).toHaveBeenCalledWith(undefined);
    });
});

function createDeps(blob: Blob | null) {
    const document = createDocument();
    const sink = {
        streaming: false,
        write: vi.fn(),
        patchHeader: vi.fn(),
        finish: vi.fn(async () => blob),
        abort: vi.fn(async (_reason?: string) => undefined)
    };
    const workers = [{ busy: false }] as never[];
    const pool = {
        size: 1,
        workers,
        findIdleWorker: vi.fn(() => workers[0]),
        ensureWorkers: vi.fn(async () => workers),
        pauseIdleTermination: vi.fn(),
        scheduleIdleTermination: vi.fn(),
        terminateAll: vi.fn()
    };
    const deps = {
        init: vi.fn(async () => undefined),
        openDocument: vi.fn(async () => document),
        createDefaultSink: vi.fn(() => sink),
        createWorkerPool: vi.fn(() => pool),
        diagnoseExport: vi.fn(() => ({ warnings: [] })),
        runJob: vi.fn(async () => ({
            outputBlob: blob,
            outputSizeBytes: blob?.size ?? null,
            modeLabel: "workers x1",
            timings: {
                totalMs: 10,
                setupMs: 1,
                layerPhaseMs: 2,
                computeMs: 3,
                assembleMs: 4,
                finalizeOnlyMs: 4
            }
        })),
        document,
        sink,
        pool,
        workers
    } satisfies SlicerConversionDeps & {
        document: SlicerDocument;
        sink: ReturnType<SlicerConversionDeps["createDefaultSink"]>;
        pool: ReturnType<SlicerConversionDeps["createWorkerPool"]>;
        workers: never[];
    };
    return deps;
}

function createDocument(): SlicerDocument {
    return {
        sourceFile: new File([new Uint8Array([1])], "sample.goo"),
        sourceName: "sample.goo",
        sourceSize: 1,
        parsedHeader: {
            format: asSlicerFormatId("goo"),
            metadata: { resolution_x: 1, resolution_y: 1, layer_count: 1 },
            layers: [{ offset: 0, size: 1 }]
        },
        async readLayerPayload() {
            return { kind: "encoded-layer", bytes: new Uint8Array([1]) };
        },
        close: vi.fn(async () => undefined)
    };
}
