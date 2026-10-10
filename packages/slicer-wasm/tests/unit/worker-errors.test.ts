import { describe, expect, it } from "vitest";

import {
    getSlicerWorkerRequestErrorContext,
    getWorkerErrorMessage,
    isRetryableWorkerError,
    isSlicerWorkerError,
    toSlicerWorkerError,
    toSlicerWorkerErrorMessage
} from "../../src/browser/worker/errors.js";
import type { SlicerWorkerRequest } from "../../src/browser/worker/protocol.js";

describe("slicer worker errors / slicer worker 错误", () => {
    it("normalizes unknown errors into structured worker errors / 将 unknown errors 规范化为 structured worker errors", () => {
        const error = toSlicerWorkerError(new Error("boom"), {
            code: "E_WORKER_LAYER_CONVERT",
            stage: "worker-convert",
            retryable: false,
            timestamp: 1,
            format: "goo",
            layerIndex: 3
        });

        expect(error).toEqual({
            code: "E_WORKER_LAYER_CONVERT",
            message: "boom",
            stage: "worker-convert",
            retryable: false,
            timestamp: 1,
            format: "goo",
            layerIndex: 3,
            detail: undefined
        });
        expect(isSlicerWorkerError(error)).toBe(true);
        expect(getWorkerErrorMessage(error)).toBe("boom");
    });

    it("preserves retryable metadata from structured errors / 保留 structured errors 的 retryable metadata", () => {
        const error = toSlicerWorkerError(
            {
                code: "E_WORKER_INIT_TRANSIENT",
                message: "try again",
                stage: "worker-init",
                retryable: true,
                timestamp: 2
            },
            {
                code: "E_FALLBACK",
                stage: "fallback",
                retryable: false
            }
        );

        expect(error.code).toBe("E_WORKER_INIT_TRANSIENT");
        expect(error.stage).toBe("worker-init");
        expect(error.retryable).toBe(true);
        expect(isRetryableWorkerError(error)).toBe(true);
    });

    it("builds worker error messages with layer context / 使用 layer context 构建 worker error messages", () => {
        const message = toSlicerWorkerErrorMessage({
            error: "bad layer",
            code: "E_WORKER_ZIP_PREVIEW",
            stage: "worker-preview",
            retryable: false,
            index: 4,
            layerIndex: 4,
            format: "vdt"
        });

        expect(message).toMatchObject({
            type: "error",
            index: 4,
            error: {
                code: "E_WORKER_ZIP_PREVIEW",
                message: "bad layer",
                stage: "worker-preview",
                retryable: false,
                layerIndex: 4,
                format: "vdt"
            }
        });
    });

    it("extracts request index for worker-not-ready errors / 为 worker-not-ready errors 提取 request index", () => {
        const request = {
            type: "decode-encode",
            index: 7,
            data: new ArrayBuffer(0),
            sourceFormat: "goo",
            targetFormat: "ctb",
            targetAnycubicVersion: null,
            width: 1,
            height: 1,
            antiAliasLevel: 1,
            encryptionKey: 0,
            layerMetadataJson: null
        } satisfies SlicerWorkerRequest;

        expect(getSlicerWorkerRequestErrorContext(request)).toEqual({ index: 7 });
        expect(getSlicerWorkerRequestErrorContext({ type: "cancel" })).toEqual({ index: undefined });
    });
});
