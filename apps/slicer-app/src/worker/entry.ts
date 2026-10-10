import {
    convertZipLayerForTarget,
    createSlicerWorkerRuntime,
    decodeZipPreview,
    inflateRaw,
    initSlicerConverter,
    prepareZipEntryWithTimings,
    slicerLayerCodecRuntime,
    startSlicerWorkerEntryRuntime
} from "@slicer-converter/wasm";

const workerScope = self as DedicatedWorkerGlobalScope;

const runtime = createSlicerWorkerRuntime({
    postMessage(message, transfer) {
        workerScope.postMessage(message, transfer ?? []);
    },
    now: () => performance.now(),
    decodeEncodeLayer: slicerLayerCodecRuntime.decodeEncodeLayer,
    decodeLayerPreview: slicerLayerCodecRuntime.decodeLayerPreview,
    inflateRaw,
    prepareZipEntry: (entry) => prepareZipEntryWithTimings(entry, () => performance.now()),
    convertZipLayerForTarget,
    decodeZipPreview
});

startSlicerWorkerEntryRuntime({
    scope: workerScope,
    runtime,
    initWasm: initSlicerConverter
});
