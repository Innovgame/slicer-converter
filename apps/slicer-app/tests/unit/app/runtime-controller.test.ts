import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
    initSlicerConverterWithProgress: vi.fn(),
    getSupportedFormats: vi.fn(() => []),
    syncSupportedFormatInfosForValidation: vi.fn(),
    validateBrowserFormatBehaviors: vi.fn(),
    initWorkerPool: vi.fn()
}));

vi.mock("@slicer-converter/wasm", async (importOriginal) => ({
    ...(await importOriginal<typeof import("@slicer-converter/wasm")>()),
    getSupportedFormats: mocks.getSupportedFormats,
    initSlicerConverterWithProgress: mocks.initSlicerConverterWithProgress,
    initSlicerWorkerPool: mocks.initWorkerPool,
    syncSupportedFormatInfosForValidation: mocks.syncSupportedFormatInfosForValidation,
    validateBrowserFormatBehaviors: mocks.validateBrowserFormatBehaviors
}));

describe("useRuntimeController / runtime controller", () => {
    afterEach(() => {
        vi.clearAllMocks();
    });

    it("fails bootstrap when prewarm completes with zero ready workers / 预热没有可用 worker 时启动失败", async () => {
        mocks.initSlicerConverterWithProgress.mockResolvedValue(undefined);
        mocks.initWorkerPool.mockResolvedValue({
            size: 0,
            workers: [],
            findIdleWorker: vi.fn(),
            ensureWorkers: vi.fn(),
            pauseIdleTermination: vi.fn(),
            scheduleIdleTermination: vi.fn(),
            terminateAll: vi.fn()
        });

        const { useRuntimeController } = await import("../../../src/app/runtime/use-runtime-controller");
        const { result } = renderHook(() => useRuntimeController());

        await waitFor(() => {
            expect(result.current.status?.type).toBe("error");
        });

        expect(result.current.status?.message).toContain("Worker init failed");
        expect(result.current.wasmReady).toBe(false);
        expect(mocks.syncSupportedFormatInfosForValidation).toHaveBeenCalledWith([]);
        expect(mocks.validateBrowserFormatBehaviors).toHaveBeenCalledWith([]);
    });
});
