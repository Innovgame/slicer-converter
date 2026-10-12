import { describe, expect, it, vi } from "vitest";

import {
    asSlicerFormatId,
    createBlobOutputSink,
    runSlicerConversionJob,
    syncSupportedFormatInfosForValidation
} from "../../src/index.js";
import type {
    ManagedSlicerWorker,
    SlicerDocument,
    SlicerWorkerRequest,
    SlicerWorkerResponse
} from "../../src/index.js";
import { formatFact } from "./support/format-facts.js";

const wasmMocks = vi.hoisted(() => ({
    buildHeader: vi.fn(() => new Uint8Array([1, 2]))
}));

vi.mock("../../lib/slicer_converter.js", () => ({
    default: vi.fn(),
    parse_header: vi.fn(),
    decode_layer: vi.fn(),
    decode_layer_preview: vi.fn(),
    decode_encode_layer: vi.fn(),
    decode_encode_layer_stats: vi.fn(),
    encode_layer: vi.fn(),
    build_header: wasmMocks.buildHeader,
    finalize_header: vi.fn((header: Uint8Array) => header),
    build_footer: vi.fn(() => new Uint8Array([3])),
    build_archive_metadata_entries: vi.fn(() => "[]"),
    describe_archive_layer_entries: vi.fn(() => "[]"),
    build_cxdlpv4_document_header: vi.fn(),
    build_cxdlpv4_layer_def: vi.fn(),
    build_document_from_layers: vi.fn(),
    diagnose_export: vi.fn(),
    crc32_bytes: vi.fn(),
    cxdlp_crc32_update: vi.fn()
}));

describe("conversion job / 转换任务", () => {
    it("rejects cancellation while finishing the output / 输出完成期间取消时拒绝成功结果", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const controller = new AbortController();
        const sink = createBlobOutputSink();
        const finish = sink.finish.bind(sink);
        sink.finish = async () => {
            controller.abort();
            return finish();
        };
        await expect(
            runSlicerConversionJob({
                document: createFakeDocument(),
                targetFormat: asSlicerFormatId("goo"),
                outputFileName: "sample.goo",
                sink,
                workers: [createLayerDoneWorker()],
                signal: controller.signal,
                updateProgress: vi.fn()
            })
        ).rejects.toMatchObject({ name: "AbortError" });
    });
    it("stops assembly before finishing the sink when cancelled / 取消组装后不完成输出 sink", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const controller = new AbortController();
        const sink = createBlobOutputSink();
        const write = sink.write.bind(sink);
        sink.write = async (bytes) => {
            await write(bytes);
            controller.abort();
        };
        const finish = vi.spyOn(sink, "finish");
        await expect(
            runSlicerConversionJob({
                document: createFakeDocument(),
                targetFormat: asSlicerFormatId("goo"),
                outputFileName: "sample.goo",
                sink,
                workers: [createLayerDoneWorker()],
                signal: controller.signal,
                updateProgress: vi.fn()
            })
        ).rejects.toMatchObject({ name: "AbortError" });
        expect(finish).not.toHaveBeenCalled();
    });

    it("converts a binary target into a Blob sink / 转换二进制目标到 Blob sink", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);
        const worker = createLayerDoneWorker();
        const progress = vi.fn();

        const result = await runSlicerConversionJob({
            document: createFakeDocument(),
            targetFormat: asSlicerFormatId("goo"),
            outputFileName: "sample.goo",
            sink: createBlobOutputSink(),
            workers: [worker],
            signal: new AbortController().signal,
            updateProgress: progress
        });

        expect(result.outputBlob).toBeInstanceOf(Blob);
        expect(result.outputSizeBytes).toBeGreaterThan(0);
        expect(result.timings.computeMs).toBe(4);
        expect(result.timings.finalizeOnlyMs).toBeTypeOf("number");
        expect(result.timings.finalizeOnlyMs).toBeGreaterThanOrEqual(0);
        expect(progress).toHaveBeenCalledWith(0, 2, "Converting...");
        expect(progress).toHaveBeenCalledWith(2, 2, "Assembling file...");
    });

    it("passes Anycubic target version into binary assembly / 传递 Anycubic 目标版本到二进制组装", async () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "pwms", aliases: ["pwms"], extensions: ["pwms"], is_zip: false })
        ]);
        const worker = createLayerDoneWorker();

        await runSlicerConversionJob({
            document: createFakeDocument(),
            targetFormat: asSlicerFormatId("pwms"),
            targetAnycubicVersion: 515,
            outputFileName: "sample.pwms",
            sink: createBlobOutputSink(),
            workers: [worker],
            signal: new AbortController().signal,
            updateProgress: vi.fn()
        });

        expect(wasmMocks.buildHeader).toHaveBeenCalledWith(expect.any(String), "pwms", 515);
    });
});

function createFakeDocument(): SlicerDocument {
    return {
        sourceFile: new File([new Uint8Array([1, 2])], "sample.goo"),
        sourceName: "sample.goo",
        sourceSize: 2,
        parsedHeader: {
            format: asSlicerFormatId("goo"),
            metadata: { resolution_x: 2, resolution_y: 2, layer_count: 2 },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 }
            ]
        },
        async readLayerPayload(index) {
            return { kind: "encoded-layer", bytes: new Uint8Array([index]) };
        },
        async close() {}
    };
}

function createLayerDoneWorker(): ManagedSlicerWorker {
    class LayerDoneWorker extends EventTarget {
        busy = false;
        onerror: ((this: AbstractWorker, event: ErrorEvent) => unknown) | null = null;
        onmessage: ((this: Worker, event: MessageEvent) => unknown) | null = null;
        onmessageerror: ((this: Worker, event: MessageEvent) => unknown) | null = null;

        postMessage(request: SlicerWorkerRequest): void {
            queueMicrotask(() => {
                this.dispatchEvent(
                    new MessageEvent("message", {
                        data: {
                            type: "layer-done",
                            index: "index" in request ? request.index : 0,
                            result: new Uint8Array([("index" in request ? request.index : 0) + 10]).buffer,
                            elapsedMs: 2
                        } satisfies SlicerWorkerResponse
                    })
                );
            });
        }

        terminate(): void {}
    }

    return new LayerDoneWorker() as unknown as ManagedSlicerWorker;
}
