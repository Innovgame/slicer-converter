import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type * as SlicerWasm from "@innovgame/slicer-converter";

const mocks = vi.hoisted(() => ({
    openSlicerDocumentFile: vi.fn()
}));

vi.mock("@innovgame/slicer-converter", async (importOriginal) => ({
    ...(await importOriginal<typeof SlicerWasm>()),
    openSlicerDocumentFile: mocks.openSlicerDocumentFile
}));

describe("useDocumentController / document controller", () => {
    afterEach(() => {
        vi.resetAllMocks();
    });

    it("keeps the latest selection when older parsing finishes later / 旧解析后完成时保留最新选择", async () => {
        const older = Promise.withResolvers<SlicerWasm.SlicerDocument>();
        const newer = Promise.withResolvers<SlicerWasm.SlicerDocument>();
        mocks.openSlicerDocumentFile.mockReturnValueOnce(older.promise).mockReturnValueOnce(newer.promise);
        const { useDocumentController } = await import("../../../src/features/document-opening");
        const { result } = renderHook(() => useDocumentController({ wasmReady: true, setStatus: vi.fn() }));
        const a = makeDocument("A.goo");
        const b = makeDocument("B.goo");
        let first!: Promise<void>;
        let second!: Promise<void>;
        act(() => {
            first = result.current.onFileSelect(a.sourceFile);
            second = result.current.onFileSelect(b.sourceFile);
        });
        await act(async () => {
            newer.resolve(b);
            await second;
        });
        await act(async () => {
            older.resolve(a);
            await first;
        });
        expect(result.current.sourceFile?.name).toBe("B.goo");
        expect(result.current.document?.sourceName).toBe("B.goo");
        expect(a.close).toHaveBeenCalledOnce();
    });

    it("ignores an obsolete parse failure / 忽略已过期的解析失败", async () => {
        const older = Promise.withResolvers<SlicerWasm.SlicerDocument>();
        const b = makeDocument("B.goo");
        mocks.openSlicerDocumentFile.mockReturnValueOnce(older.promise).mockResolvedValueOnce(b);
        const { useDocumentController } = await import("../../../src/features/document-opening");
        const setStatus = vi.fn();
        const { result } = renderHook(() => useDocumentController({ wasmReady: true, setStatus }));
        let first!: Promise<void>;
        act(() => {
            first = result.current.onFileSelect(makeDocument("A.goo").sourceFile);
        });
        await act(async () => {
            await result.current.onFileSelect(b.sourceFile);
        });
        await act(async () => {
            older.reject(new Error("obsolete failure"));
            await first;
        });
        expect(result.current.document?.sourceName).toBe("B.goo");
        expect(setStatus).not.toHaveBeenCalledWith(expect.objectContaining({ type: "error" }));
    });

    it("closes a document that finishes opening after unmount / 卸载后关闭迟到的文档", async () => {
        const opening = Promise.withResolvers<SlicerWasm.SlicerDocument>();
        mocks.openSlicerDocumentFile.mockReturnValueOnce(opening.promise);
        const { useDocumentController } = await import("../../../src/features/document-opening");
        const setStatus = vi.fn();
        const { result, unmount } = renderHook(() => useDocumentController({ wasmReady: true, setStatus }));
        const document = makeDocument("late.goo");
        let run!: Promise<void>;
        act(() => {
            run = result.current.onFileSelect(document.sourceFile);
        });
        unmount();
        opening.resolve(document);
        await run;
        expect(document.close).toHaveBeenCalledOnce();
        expect(setStatus).not.toHaveBeenCalled();
    });

    it("closes the current document when replaced and on unmount / 替换和卸载时关闭当前文档", async () => {
        const a = makeDocument("A.goo");
        const b = makeDocument("B.goo");
        mocks.openSlicerDocumentFile.mockResolvedValueOnce(a).mockResolvedValueOnce(b);
        const { useDocumentController } = await import("../../../src/features/document-opening");
        const { result, unmount } = renderHook(() => useDocumentController({ wasmReady: true, setStatus: vi.fn() }));
        await act(async () => {
            await result.current.onFileSelect(a.sourceFile);
        });
        await act(async () => {
            await result.current.onFileSelect(b.sourceFile);
        });
        expect(a.close).toHaveBeenCalledOnce();
        unmount();
        expect(b.close).toHaveBeenCalledOnce();
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

function makeDocument(name: string): SlicerWasm.SlicerDocument {
    return {
        sourceFile: new File([new Uint8Array([1])], name),
        sourceName: name,
        sourceSize: 1,
        parsedHeader: {
            format: "goo" as never,
            metadata: { resolution_x: 1, resolution_y: 1, layer_count: 1 },
            layers: [{ offset: 0, size: 1 }]
        },
        readLayerPayload: async () => ({ kind: "encoded-layer", bytes: new Uint8Array([1]) }),
        close: vi.fn(async () => undefined)
    };
}
