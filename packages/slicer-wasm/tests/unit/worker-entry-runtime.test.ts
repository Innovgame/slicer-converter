import { afterEach, describe, expect, it, vi } from "vitest";

import { startSlicerWorkerEntryRuntime } from "../../src/browser/worker/entry-runtime.js";
import type { SlicerWorkerRuntime } from "../../src/browser/worker/runtime.js";
import type { SlicerWorkerRequest, SlicerWorkerResponse } from "../../src/browser/worker/protocol.js";

describe("worker entrypoint / worker entrypoint 用例", () => {
    afterEach(() => {
        vi.unstubAllGlobals();
        vi.clearAllMocks();
    });

    it("keeps package entry runtime free of concrete worker construction / package entry runtime 不创建 concrete worker", async () => {
        const source = await import("node:fs/promises").then(({ readFile }) =>
            readFile("src/browser/worker/entry-runtime.ts", "utf8")
        );

        expect(source).toContain("startSlicerWorkerEntryRuntime");
        expect(source).not.toContain("new Worker");
        expect(source).not.toContain("worker.ts");
    });

    it("keeps layer correlation fields when a prepare request arrives before the worker is ready / 保持 layer correlation fields 当 a prepare request arrives 在之前 the worker is ready", () => {
        const { postMessage, scope } = createTestScope();
        const runtime = createTestRuntime();

        startSlicerWorkerEntryRuntime({
            scope,
            runtime,
            initWasm: () => new Promise<void>(() => undefined)
        });

        scope.onmessage?.({
            data: {
                type: "decode-encode-prepare",
                index: 4,
                data: new ArrayBuffer(0),
                sourceFormat: "goo",
                targetFormat: "vdt",
                targetAnycubicVersion: null,
                width: 64,
                height: 32,
                antiAliasLevel: 1,
                encryptionKey: 0,
                layerMetadataJson: null,
                entryName: "0004.png",
                entryCompressionMethod: 8
            }
        } as MessageEvent<SlicerWorkerRequest>);

        expect(postMessage).toHaveBeenCalledWith(
            expect.objectContaining({
                type: "error",
                index: 4,
                error: expect.objectContaining({
                    code: "E_WORKER_NOT_READY",
                    retryable: true
                })
            })
        );
        expect(postMessage.mock.calls[0]?.[0]).not.toHaveProperty("batchStartIndex");
    });

    it("dispatches non-cancel requests after WASM init finishes / WASM init 完成后分发非 cancel 请求", async () => {
        const { scope } = createTestScope();
        const runtime = createTestRuntime();

        startSlicerWorkerEntryRuntime({
            scope,
            runtime,
            initWasm: () => Promise.resolve()
        });
        await Promise.resolve();

        scope.onmessage?.({
            data: {
                type: "decode-preview",
                requestId: 1,
                quality: "interactive",
                index: 2,
                maxPreviewSize: 512,
                payload: {
                    kind: "encoded-layer",
                    data: new ArrayBuffer(0),
                    format: "goo",
                    width: 16,
                    height: 16,
                    antiAliasLevel: 1,
                    encryptionKey: 0
                }
            }
        } as MessageEvent<SlicerWorkerRequest>);

        expect(runtime.handlers["decode-preview"]).toHaveBeenCalledWith(
            expect.objectContaining({
                type: "decode-preview",
                index: 2
            })
        );
    });
});

function createTestScope(): {
    postMessage: ReturnType<typeof vi.fn>;
    scope: {
        postMessage(message: SlicerWorkerResponse, transfer?: Transferable[]): void;
        onmessage: ((event: MessageEvent<SlicerWorkerRequest>) => void) | null;
    };
} {
    const postMessage = vi.fn();
    return {
        postMessage,
        scope: {
            postMessage,
            onmessage: null
        }
    };
}

function createTestRuntime(): SlicerWorkerRuntime {
    return {
        handlers: {
            "decode-preview": vi.fn(),
            "decode-encode": vi.fn(),
            "decode-encode-prepare": vi.fn(),
            "zip-decode-encode": vi.fn(),
            "zip-decode-encode-prepare": vi.fn()
        }
    };
}
