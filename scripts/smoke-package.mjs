import { File } from "node:buffer";
import { URL } from "node:url";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import {
    initSlicerConverter,
    initSlicerConverterWithProgress,
    openSlicerDocumentFile,
    parseBrowserFileHeader
} from "../packages/slicer-wasm/dist/index.js";

const mode = process.argv[2];
if (mode === "progress") await initSlicerConverterWithProgress(() => undefined);
else await initSlicerConverter();
await initSlicerConverter();
const bytes = await readFile(new URL("../rust/crates/core/tests/fixtures/sample.ctb", import.meta.url));
const file = new File([bytes], "sample.ctb");
const document = await openSlicerDocumentFile({ file, parseHeader: parseBrowserFileHeader });
assert.equal(document.parsedHeader.layers.length, 10);
assert.equal(document.parsedHeader.metadata.resolution_x, 64);
await document.close();

const tableOffset = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(64, true);
await assert.rejects(
    openSlicerDocumentFile({
        file: new File([bytes.subarray(0, tableOffset)], "bad.ctb"),
        parseHeader: parseBrowserFileHeader
    }),
    /Buffer too small/i
);
console.log(`Published package smoke passed (${mode ?? "default"} initialization)`);
