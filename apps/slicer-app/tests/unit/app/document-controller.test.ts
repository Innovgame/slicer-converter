import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type * as SlicerWasm from "@slicer-converter/wasm";

const mocks = vi.hoisted(() => ({
    openSlicerDocumentFile: vi.fn()
}));

vi.mock("@slicer-converter/wasm", async (importOriginal) => ({
    ...(await importOriginal<typeof SlicerWasm>()),
    openSlicerDocumentFile: mocks.openSlicerDocumentFile
}));

describe("useDocumentController / document controller", () => {
    afterEach(() => {
        vi.clearAllMocks();
    });

    it("formats structured parse failures with the embedded message / 使用结构化错误中的消息", async () => {
        mocks.openSlicerDocumentFile.mockRejectedValue({
            code: "E_PARSE_HEADER",
            message: "bad header",
            stage: "document-parse",
            retryable: false,
            timestamp: 1
        });

        const { useDocumentController } = await import("../../../src/features/document-opening");
        const setStatus = vi.fn();
        const { result } = renderHook(() =>
            useDocumentController({
                wasmReady: true,
                setStatus
            })
        );

        await act(async () => {
            await result.current.onFileSelect(new File([new Uint8Array([1, 2, 3])], "sample.goo"));
        });

        expect(setStatus).toHaveBeenCalledWith({
            type: "error",
            message: "Parse error: bad header"
        });
    });
});
