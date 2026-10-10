# @slicer-converter/wasm

Browser TypeScript API for opening, previewing, and converting resin slicer files with the Rust/WASM converter.

## Install And Build

This package is currently consumed from the workspace:

```ts
import { convertSlicerFileToBlob, openSlicerDocumentFile } from "@slicer-converter/wasm";
```

Before running an app, build the WASM artifact:

```powershell
pnpm --filter @slicer-converter/wasm build
```

## Open A File

```ts
import { openSlicerDocumentFile, parseBrowserFileHeader } from "@slicer-converter/wasm";

const document = await openSlicerDocumentFile({
    file,
    parseHeader: parseBrowserFileHeader
});

console.log(document.parsedHeader.format, document.parsedHeader.layers.length);
```

`openSlicerDocumentFile()` chooses the browser-side opener for archive formats and falls back to the WASM header parser for raw formats.

## Convert A File

```ts
import { convertSlicerFileToBlob } from "@slicer-converter/wasm";

const result = await convertSlicerFileToBlob({
    file,
    target: "ctb",
    signal: abortController.signal,
    onProgress(progress) {
        console.log(progress.current, progress.total, progress.label);
    }
});

download(result.blob, result.fileName);
```

`convertSlicerFileToBlob()` initializes WASM, opens the document, resolves the target, creates the default Blob output sink, runs the conversion, and returns a non-null `Blob`.

## External Browser App: Goo To Target

An external browser app can keep its own Goo generation pipeline and use this package only for the reusable slicer conversion step:

```ts
import {
    convertSlicerFileToBlob,
    initSlicerConverter,
    prepareSlicerExportTargetFromMatrix,
    syncFormatFacts,
    type ManagedSlicerWorker,
    type SlicerConversionProgress
} from "@slicer-converter/wasm";

export async function convertGeneratedGoo(input: {
    gooBlob: Blob;
    fileNameStem: string;
    formatId: string;
    suffixId: string;
    versionId: string;
    signal?: AbortSignal;
    onProgress?: (progress: SlicerConversionProgress) => void;
}): Promise<{ blob: Blob; fileName: string }> {
    const api = await initSlicerConverter();
    syncFormatFacts(api.getSupportedFormats());

    const prepared = prepareSlicerExportTargetFromMatrix({
        sourceFormat: "goo",
        fileNameStem: input.fileNameStem,
        formatId: input.formatId,
        suffixId: input.suffixId,
        versionId: input.versionId
    });
    if (!prepared) {
        throw new Error("No slicer-converter export target selected.");
    }

    const workers: ManagedSlicerWorker[] = [];
    try {
        const result = await convertSlicerFileToBlob({
            file: new File([input.gooBlob], `${input.fileNameStem}.goo`),
            fileName: `${input.fileNameStem}.goo`,
            sourceExtension: "goo",
            target: prepared.target,
            signal: input.signal,
            worker: {
                kind: "auto",
                createWorker() {
                    const worker = Object.assign(
                        new Worker(new URL("./SlicerConverter.worker.ts", import.meta.url), { type: "module" }),
                        { busy: false }
                    );
                    workers.push(worker);
                    return worker;
                }
            },
            onProgress: input.onProgress
        });

        return { blob: result.blob, fileName: prepared.fileName };
    } finally {
        workers.forEach((worker) => worker.terminate());
    }
}
```

Build the package before using a local link:

```powershell
pnpm --filter @slicer-converter/wasm build
```

## Advanced Output

Use `convertSlicerFileToSink()` when the caller owns the output sink:

```ts
import {
    convertSlicerFileToSink,
    createBufferedOutputSink,
    openFileSystemSink
} from "@slicer-converter/wasm";

const fileSystemSink = await openFileSystemSink({ suggestedName: "plate.ctb" });
const output = createBufferedOutputSink(fileSystemSink);

const result = await convertSlicerFileToSink({
    file,
    target: "ctb",
    output,
    onProgress
});

console.log(result.fileName, result.blob);
```

For lower-level control, `convertSlicerFile()` remains available and returns `blob: Blob | null` depending on the sink.

## Preview Layers

```ts
import { createSlicerPreviewSession } from "@slicer-converter/wasm";

const session = createSlicerPreviewSession({ document });
const payload = await session.buildDispatchPayload({
    layerIndex: 0,
    requestId: 1,
    quality: "interactive",
    maxPreviewSize: 512
});

worker.postMessage(payload.request, [payload.transfer]);
```

The preview session builds worker payloads for raw encoded layers and ZIP/archive layers.

## Worker Runtime

```ts
import {
    createSlicerWorkerRuntime,
    startSlicerWorkerEntryRuntime
} from "@slicer-converter/wasm";

const runtime = createSlicerWorkerRuntime({
    postMessage: (message, transfer) => self.postMessage(message, transfer ?? []),
    now: () => performance.now(),
    decodeEncodeLayer,
    decodeLayerPreview,
    inflateRaw,
    prepareZipEntry,
    convertZipLayerForTarget,
    decodeZipPreview
});

startSlicerWorkerEntryRuntime({
    scope: self as DedicatedWorkerGlobalScope,
    runtime,
    initWasm
});
```

Use `initSlicerWorkerPool()` on the main thread when you want managed worker prewarming and idle termination.

## ZIP And Output Diagnostics

```ts
import {
    createInstrumentedPatchableOutputSink,
    parseZipDirectory,
    snapshotOutputSinkStats
} from "@slicer-converter/wasm";

const directory = parseZipDirectory(bytes);
const { sink, stats } = createInstrumentedPatchableOutputSink(outputSink);

await sink.write(bytes);
await sink.finish();

console.log(directory.entries.length, snapshotOutputSinkStats(stats));
```

Archive naming and metadata planning APIs are available through `buildArchiveMetadataEntries()`, `describeArchiveLayerEntry()`, `describeArchiveLayerEntries()`, and `createArchiveLayerDescriptorProvider()`.

## Format Facts

```ts
import {
    listExportTargetOptions,
    listImportableFileExtensions,
    validateBrowserFormatBehaviors
} from "@slicer-converter/wasm";
```

Use these helpers for UI option lists and browser format validation. Format behavior, archive layout, manifest/config fields, and layer codec behavior should stay aligned with UVTools before changing them.
