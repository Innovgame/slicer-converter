import { beforeEach, describe, expect, it, vi } from "vitest";
import { formatFact } from "./support/format-facts.js";

const mocks = vi.hoisted(() => ({
    releaseInit: vi.fn().mockResolvedValue(undefined),
    crc32Bytes: vi.fn(),
    listSupportedFormats: vi.fn()
}));

vi.mock("../../lib/slicer_converter.js", () => ({
    default: mocks.releaseInit,
    crc32_bytes: mocks.crc32Bytes,
    list_supported_formats: mocks.listSupportedFormats
}));

beforeEach(() => {
    vi.resetModules();
    mocks.releaseInit.mockClear();
    mocks.listSupportedFormats.mockReturnValue(
        JSON.stringify([formatFact({ name: "ctb", aliases: [], extensions: ["ctb"], is_zip: false })])
    );
    vi.unstubAllGlobals();
});

describe("wasm init wrappers / WASM 初始化包装器", () => {
    it.each([false, true])(
        "makes format facts available after initialization (progress=%s) / 初始化后可查询格式事实",
        async (progress) => {
            const api = await import("../../src/api.js");
            const { getFormatFacts, listImportableFileExtensions } = await import("../../src/browser/format/facts.js");
            if (progress) await api.initSlicerConverterWithProgress(() => undefined);
            else await api.initSlicerConverter();
            expect(getFormatFacts("ctb").capabilities.canImport).toBe(true);
            expect(listImportableFileExtensions()).toContain(".ctb");
            await api.initSlicerConverter();
            expect(getFormatFacts("ctb").format).toBe("ctb");
            expect(mocks.releaseInit).toHaveBeenCalledOnce();
        }
    );

    it("wraps release wasm input in the new module_or_path object / 包装 release wasm input in the new module_or_path 对象", async () => {
        const mod = await import("../../src/api.js");
        const wasmUrl = mod.getSlicerConverterWasmUrl();

        await mod.initSlicerConverter();

        const moduleOrPath = mocks.releaseInit.mock.calls[0]?.[0]?.module_or_path;
        if (wasmUrl.protocol === "file:") {
            expect(moduleOrPath).toBeInstanceOf(Uint8Array);
            expect((moduleOrPath as Uint8Array).byteLength).toBeGreaterThan(0);
            return;
        }

        expect(mocks.releaseInit).toHaveBeenCalledWith({ module_or_path: wasmUrl });
    });

    it("reports download progress and initializes from streamed bytes / 报告 下载 progress 并且 initializes 来自 streamed 字节", async () => {
        const read = vi
            .fn()
            .mockResolvedValueOnce({ done: false, value: new Uint8Array([1, 2]) })
            .mockResolvedValueOnce({ done: false, value: new Uint8Array([3, 4, 5]) })
            .mockResolvedValueOnce({ done: true, value: undefined });
        const fetchMock = vi.fn().mockResolvedValue({
            ok: true,
            status: 200,
            headers: new Headers({ "content-length": "5" }),
            body: { getReader: () => ({ read }) },
            arrayBuffer: vi.fn()
        });
        vi.stubGlobal("fetch", fetchMock);

        const mod = await import("../../src/api.js");
        const onProgress = vi.fn();
        const wasmUrl = new URL("https://example.com/slicer_converter_bg.wasm");

        await mod.initSlicerConverterWithProgress(onProgress, wasmUrl);

        expect(fetchMock).toHaveBeenCalledWith(wasmUrl);
        expect(onProgress).toHaveBeenCalledWith({
            phase: "downloading",
            loadedBytes: 0,
            totalBytes: 5,
            percent: 0
        });
        expect(onProgress).toHaveBeenCalledWith({
            phase: "compiling",
            loadedBytes: 5,
            totalBytes: 5,
            percent: 100
        });
        expect(onProgress).toHaveBeenLastCalledWith({
            phase: "ready",
            loadedBytes: 0,
            totalBytes: null,
            percent: 100
        });

        const moduleOrPath = mocks.releaseInit.mock.calls[0]?.[0]?.module_or_path;
        expect(moduleOrPath).toBeInstanceOf(Uint8Array);
        expect(Array.from(moduleOrPath as Uint8Array)).toEqual([1, 2, 3, 4, 5]);
    });

    it("normalizes structured init failures into Error instances without losing code / 规范化 结构化 init failures 到 Error instances 不通过 losing code", async () => {
        mocks.releaseInit.mockRejectedValueOnce({
            code: "E_WASM_INIT",
            message: "WASM bootstrap failed",
            retryable: false
        });

        const mod = await import("../../src/api.js");

        try {
            await mod.initSlicerConverter();
            throw new Error("Expected initSlicerConverter to reject");
        } catch (error: unknown) {
            expect(error).toBeInstanceOf(Error);
            expect((error as Error).message).toBe("WASM bootstrap failed");
            expect((error as { code?: string }).code).toBe("E_WASM_INIT");
        }
    });
});
