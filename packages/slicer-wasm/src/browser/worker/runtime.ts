import type { ConvertZipLayerInput, DecodeZipPreviewInput } from "../archive/zip/worker.js";
import type { LayerCodecRuntime } from "../runtime/codec.js";

import type { ZipArchiveEntry, ZipPreparedEntryWithTimings } from "../archive/zip/archive.js";
import type { SlicerWorkerRequest, SlicerWorkerResponse } from "./protocol.js";
import { createSlicerConversionHandlers } from "./handlers/conversion.js";
import { createSlicerPreviewHandlers } from "./handlers/preview.js";

/**
 * @zh `DispatchableSlicerWorkerRequest` 类型契约。
 * @en Type contract for `DispatchableSlicerWorkerRequest`.
 */
export type DispatchableSlicerWorkerRequest = Exclude<SlicerWorkerRequest, { type: "cancel" }>;
/**
 * @zh `DispatchableSlicerWorkerType` 类型契约。
 * @en Type contract for `DispatchableSlicerWorkerType`.
 */
export type DispatchableSlicerWorkerType = DispatchableSlicerWorkerRequest["type"];
/**
 * @zh `SlicerWorkerHandlerMap` 类型契约。
 * @en Type contract for `SlicerWorkerHandlerMap`.
 */
export type SlicerWorkerHandlerMap = {
    [K in DispatchableSlicerWorkerType]: (message: Extract<DispatchableSlicerWorkerRequest, { type: K }>) => void;
};

/**
 * @zh `SlicerWorkerRuntimeDeps` 接口契约。
 * @en Interface contract for `SlicerWorkerRuntimeDeps`.
 */
export interface SlicerWorkerRuntimeDeps {
    postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) => void;
    now: () => number;
    decodeEncodeLayer: LayerCodecRuntime["decodeEncodeLayer"];
    decodeLayerPreview: LayerCodecRuntime["decodeLayerPreview"];
    inflateRaw: (compressed: Uint8Array, expectedUncompressedSize?: number) => Promise<Uint8Array>;
    prepareZipEntry: (entry: ZipArchiveEntry) => Promise<ZipPreparedEntryWithTimings>;
    convertZipLayerForTarget: (input: ConvertZipLayerInput) => Promise<Uint8Array>;
    decodeZipPreview: (input: DecodeZipPreviewInput) => Promise<{
        pixels: Uint8Array;
        width: number;
        height: number;
        timings?: {
            zipInflateMs: number;
            wasmPreviewDecodeMs: number;
            totalMs: number;
        };
    }>;
}

/**
 * @zh `SlicerWorkerRuntime` 接口契约。
 * @en Interface contract for `SlicerWorkerRuntime`.
 */
export interface SlicerWorkerRuntime {
    handlers: SlicerWorkerHandlerMap;
}

/**
 * @zh `createSlicerWorkerRuntime` 导出函数。
 * @en Exported function `createSlicerWorkerRuntime`.
 */
export function createSlicerWorkerRuntime(deps: SlicerWorkerRuntimeDeps): SlicerWorkerRuntime {
    const handlers: SlicerWorkerHandlerMap = {
        ...createSlicerPreviewHandlers(deps),
        ...createSlicerConversionHandlers(deps)
    };

    return { handlers };
}
