import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
    FileMetadata,
    ManagedSlicerWorker as ManagedConversionWorker,
    ParsedHeader,
    ResolvedSlicerExportTarget,
    SlicerWorkerPoolState as WorkerPoolState
} from "@slicer-converter/wasm";
import { createSlicerDocument } from "@slicer-converter/wasm";
import type * as SlicerWasm from "@slicer-converter/wasm";

import { anycubicFormatFact } from "../support/format-facts";

const mocks = vi.hoisted(() => ({
    diagnoseExport: vi.fn(),
    runConversionUseCase: vi.fn()
}));

vi.mock("../../../src/features/conversion/model/conversion-use-case", () => ({
    runConversionUseCase: mocks.runConversionUseCase
}));

vi.mock("@slicer-converter/wasm", async (importOriginal) => ({
    ...(await importOriginal<typeof SlicerWasm>()),
    diagnoseExport: mocks.diagnoseExport
}));

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

function createDocument(metadata: FileMetadata) {
    return createSlicerDocument({
        sourceFile: new File([new Uint8Array([1, 2, 3])], "sample.goo"),
        parsedHeader: {
            format: "goo",
            metadata,
            layers: [{ offset: 0, size: 3 }]
        } as ParsedHeader,
        layerSource: { kind: "raw-file" }
    });
}

describe("useConversionController warnings / 转换 controller warning", () => {
    afterEach(() => {
        vi.clearAllMocks();
    });

    it("warns after exporting to a mismatched Anycubic machine target / 导出到不匹配 Anycubic 机器目标后提示 warning", async () => {
        const { syncFormatFacts } = await import("@slicer-converter/wasm");
        syncFormatFacts([
            anycubicFormatFact({
                machine_id: "PhotonZero",
                display_name: "Photon Zero",
                extension: "pw0",
                default_version: 1,
                supported_versions: [1],
                table_layout: "V1",
                resolution_x: 480,
                resolution_y: 854,
                display_width_mm: 55.4,
                display_height_mm: 98.63,
                machine_z_mm: 150
            })
        ]);
        mocks.runConversionUseCase.mockResolvedValueOnce({ statusMessage: "Converted 1 layer." });
        mocks.diagnoseExport.mockReturnValue({
            warnings: [
                {
                    code: "ANYCUBIC_MACHINE_PROFILE_MISMATCH",
                    severity: "warning",
                    message:
                        "Warning: Source machine ELEGOO Mars 5 Ultra(1) geometry differs from target machine Photon Zero (PW0). Export continues without layer resampling. UVTools may report non-square pixels or an incorrect image ratio, and the model may print with stretched proportions.",
                    source: {
                        format: null,
                        display_name: "ELEGOO Mars 5 Ultra(1)",
                        extension: null,
                        resolution_x: 8520,
                        resolution_y: 4320,
                        display_width_mm: 55.44,
                        display_height_mm: 98.637
                    },
                    target: {
                        format: "pw0",
                        display_name: "Photon Zero",
                        extension: "pw0",
                        resolution_x: 480,
                        resolution_y: 854,
                        display_width_mm: 55.4,
                        display_height_mm: 98.63
                    }
                }
            ]
        });

        const { useConversionController } =
            await import("../../../src/features/conversion/model/use-conversion-controller");
        const setStatus = vi.fn();
        const { result } = renderHook(() =>
            useConversionController({
                document: createDocument({
                    resolution_x: 8520,
                    resolution_y: 4320,
                    display_width_mm: 55.44,
                    display_height_mm: 98.637,
                    layer_count: 1
                }),
                workerPool: createWorkerPool(),
                wasmReady: true,
                outputFileName: "sample",
                conversionTarget: {
                    requestedTargetFormat: "anycubic-manual",
                    resolvedTargetFormat: "pw0",
                    resolvedExtension: "pw0",
                    resolvedAnycubicVersion: 1,
                    statusMessage: "Using manual Anycubic target: Photon Zero (PW0)"
                } satisfies ResolvedSlicerExportTarget,
                setStatus
            })
        );

        expect(result.current.exportWarning).toEqual(
            expect.stringContaining(
                "Source machine ELEGOO Mars 5 Ultra(1) geometry differs from target machine Photon Zero"
            )
        );

        await act(async () => {
            await result.current.onConvert();
        });

        expect(setStatus).toHaveBeenLastCalledWith({
            type: "warning",
            message: expect.stringContaining(
                "Source machine ELEGOO Mars 5 Ultra(1) geometry differs from target machine Photon Zero"
            )
        });
        expect(mocks.diagnoseExport).toHaveBeenCalledWith({
            metadata: {
                resolution_x: 8520,
                resolution_y: 4320,
                display_width_mm: 55.44,
                display_height_mm: 98.637,
                layer_count: 1
            },
            targetFormat: "pw0"
        });
    });
});
