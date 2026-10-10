import { describe, expect, it } from "vitest";

import { toWorkerErrorMessage } from "../../../src/shared/lib/errors";

describe("worker error normalization / worker 错误 normalization", () => {
    it("preserves structured wasm error fields when wrapping worker failures / preserves 结构化 wasm 错误 fields 当 wrapping worker failures", () => {
        const wasmError = Object.assign(new Error("Invalid magic number"), {
            code: "E_INVALID_MAGIC",
            detail: "expected 0x12345678, got 0x00000000",
            retryable: true
        });

        const message = toWorkerErrorMessage({
            error: wasmError,
            code: "E_WORKER_LAYER_CONVERT",
            stage: "worker-convert",
            retryable: false,
            index: 2,
            format: "ctb",
            layerIndex: 2
        });

        expect(message).toMatchObject({
            type: "error",
            index: 2,
            error: {
                code: "E_INVALID_MAGIC",
                message: "Invalid magic number",
                stage: "worker-convert",
                retryable: true,
                detail: "expected 0x12345678, got 0x00000000",
                format: "ctb",
                layerIndex: 2
            }
        });
        expect(typeof message.error.timestamp).toBe("number");
    });
});
