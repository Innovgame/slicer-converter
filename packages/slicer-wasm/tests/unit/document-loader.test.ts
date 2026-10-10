import { deflateRawSync } from "node:zlib";

import { describe, expect, it, vi } from "vitest";

import type { ParsedHeader } from "../../src/browser/types.js";
import {
    loadSlicerDocumentFileSource,
    openSlicerDocument,
    openSlicerDocumentFile
} from "../../src/browser/document/source.js";
import { syncFormatFacts } from "../../src/browser/format/format.js";
import { parseZipCentralDirectoryEntries } from "../../src/browser/archive/zip/directory.js";
import { readZipEntryBytes } from "../../src/browser/archive/zip/source.js";
import { formatFact } from "./support/format-facts.js";

describe("openSlicerDocument / 打开 slicer 文档", () => {
    it("derives the extension from the browser File name / 从浏览器 File 名称推导 extension", async () => {
        syncFormatFacts([formatFact({ name: "ctb", aliases: ["ctb"], extensions: ["ctb"], is_zip: false })]);
        const file = new File([new Uint8Array([1, 2, 3])], "Sample.CTB");
        const parseHeader = vi.fn(
            (_data: Uint8Array, extension: string): ParsedHeader => ({
                format: extension as never,
                metadata: { layer_count: 0 },
                layers: []
            })
        );

        const document = await openSlicerDocumentFile({ file, parseHeader });

        expect(parseHeader).toHaveBeenCalledWith(expect.any(Uint8Array), "ctb");
        expect(document.sourceName).toBe("Sample.CTB");
        expect(document.parsedHeader.format).toBe("ctb");
    });

    it("loads a source from the browser File name when no extension is provided / 没有 extension 时从浏览器 File 名称加载 source", async () => {
        syncFormatFacts([formatFact({ name: "phz", aliases: ["phz"], extensions: ["phz"], is_zip: false })]);
        const file = new File([new Uint8Array([4, 5, 6])], "part.PHZ");
        const parseHeader = vi.fn(
            (_data: Uint8Array, extension: string): ParsedHeader => ({
                format: extension as never,
                metadata: { layer_count: 0 },
                layers: []
            })
        );

        const source = await loadSlicerDocumentFileSource({ file, parseHeader });

        expect(parseHeader).toHaveBeenCalledWith(expect.any(Uint8Array), "phz");
        expect(source.layerSource).toEqual({ kind: "raw-file" });
    });

    it("opens Chitubox ZIP without falling back to WASM prefix parsing / 打开 Chitubox ZIP 不回退到 WASM prefix parsing", async () => {
        syncFormatFacts([
            formatFact({
                name: "chitubox-zip",
                aliases: ["chitubox-zip", "chituzip"],
                extensions: ["zip"],
                is_zip: true
            })
        ]);
        const parseHeader = vi.fn((): ParsedHeader => {
            throw new Error("Parse error: ZIP error: invalid Zip archive: Could not find EOCD");
        });

        const opened = await openSlicerDocument({
            file: new File([toArrayBuffer(buildStoredZipArchive(chituboxZipEntries()))], "sample.zip"),
            extension: "zip",
            parseHeader
        });

        expect(parseHeader).not.toHaveBeenCalled();
        expect(opened.parsedHeader).toMatchObject({
            format: "chitubox-zip",
            metadata: { layer_count: 2, resolution_x: 4, resolution_y: 2 },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 }
            ]
        });
        await expect(opened.readLayerPayload(0)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([1]),
            compressionMethod: 0,
            uncompressedSize: 1
        });
    });

    it("opens Anycubic ZIP as ZIP-backed layers for preview / 打开 Anycubic ZIP 为 preview 提供 ZIP-backed layers", async () => {
        syncFormatFacts([
            formatFact({
                name: "anycubic-zip",
                aliases: ["anycubic-zip", "pm7"],
                extensions: ["anycubic-zip", "pm7"],
                is_zip: true,
                specifics: {
                    family: "anycubic-archive",
                    settings_manifest_entry: "anycubic_photon_resins.pwsp",
                    layers_manifest_entry: "layers_controller.conf",
                    layer_entry_prefix: "layer_images/layer_",
                    default_layer_image_extension: "pw0Img",
                    layer_image_extensions: ["pw0Img", "pwszImg"],
                    required_manifest_entries: ["anycubic_photon_resins.pwsp", "layers_controller.conf"]
                }
            })
        ]);
        const parseHeader = vi.fn((): ParsedHeader => {
            throw new Error("Parse error: ZIP prefix parser should not be used");
        });

        const opened = await openSlicerDocument({
            file: new File([toArrayBuffer(buildStoredZipArchive(anycubicZipEntries()))], "sample.anycubic-zip"),
            extension: "anycubic-zip",
            parseHeader
        });

        expect(parseHeader).not.toHaveBeenCalled();
        expect(opened.parsedHeader).toMatchObject({
            format: "anycubic-zip",
            metadata: { layer_count: 2, resolution_x: 4, resolution_y: 2 },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 }
            ]
        });
        await expect(opened.readLayerPayload(1)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([2]),
            compressionMethod: 0,
            uncompressedSize: 1
        });
    });

    it("falls back to header retry parsing when no browser opener exists / 无 browser opener 时回退到 header retry parsing", async () => {
        syncFormatFacts([formatFact({ name: "phz", aliases: ["phz"], extensions: ["phz"], is_zip: false })]);
        const requiredBytes = 65 * 1024 * 1024;
        const file = new File([new Blob([new Uint8Array(requiredBytes + 8)])], "sample.phz");
        const parseHeader = vi.fn((data: Uint8Array): ParsedHeader => {
            if (data.byteLength < requiredBytes) {
                throw new Error(`Parse error: Buffer too small: need ${requiredBytes} bytes, got ${data.byteLength}`);
            }
            return {
                format: "phz" as never,
                metadata: { layer_count: 0 },
                layers: []
            };
        });

        const document = await openSlicerDocument({ file, extension: "phz", parseHeader });

        expect(parseHeader).toHaveBeenCalledTimes(2);
        expect(document.parsedHeader.format).toBe("phz");
    });

    it("reads the full file when format facts require document-level parsing / format facts 需要 document-level parsing 时读取完整文件", async () => {
        syncFormatFacts([
            formatFact({
                name: "photons",
                aliases: ["photons", "photon-s"],
                extensions: ["photons"],
                is_zip: false,
                capabilities: {
                    can_import: true,
                    can_export: true,
                    is_archive: false,
                    supports_preview: false,
                    supports_anti_alias: false,
                    supports_per_layer_settings: true,
                    supports_encryption: false,
                    requires_full_document_parse: true
                }
            })
        ]);
        const fileBytes = new Uint8Array(2 * 1024 * 1024);
        fileBytes[fileBytes.length - 1] = 9;
        const parseHeader = vi.fn(
            (data: Uint8Array): ParsedHeader => ({
                format: "photons" as never,
                metadata: { layer_count: 1 },
                layers: [{ offset: data.byteLength - 1, size: 1 }]
            })
        );

        const document = await openSlicerDocument({
            file: new File([fileBytes], "sample.photons"),
            extension: "photons",
            parseHeader
        });

        expect(parseHeader).toHaveBeenCalledTimes(1);
        expect(parseHeader.mock.calls[0]?.[0].byteLength).toBe(fileBytes.byteLength);
        await expect(document.readLayerPayload(0)).resolves.toMatchObject({
            kind: "encoded-layer",
            bytes: new Uint8Array([9])
        });
    });
});

describe("ZIP document opener malformed archives / ZIP 文档 opener malformed archives", () => {
    it("rejects truncated Central Directory variable-length records / 拒绝 truncated Central Directory variable-length records", () => {
        const centralDirectory = new Uint8Array(46);
        const view = new DataView(centralDirectory.buffer);
        view.setUint32(0, 0x02014b50, true);
        view.setUint16(28, 4, true);

        expect(() => parseZipCentralDirectoryEntries(centralDirectory, 1)).toThrow(
            /ZIP Central Directory truncated at entry 0/
        );
    });

    it("rejects deflated entries when inflated bytes exceed expected size / inflated bytes 超过 expected size 时拒绝 deflated entries", async () => {
        const compressed = new Uint8Array(deflateRawSync(new Uint8Array([1, 2, 3])));

        await expect(
            readZipEntryBytes(
                {
                    async readRange() {
                        return new Uint8Array();
                    },
                    async readCompressedEntryPayload() {
                        return compressed;
                    }
                },
                {
                    index: 0,
                    name: "1.png",
                    compressionMethod: 8,
                    compressedSize: compressed.byteLength,
                    uncompressedSize: 2,
                    localHeaderOffset: 0,
                    usesDataDescriptor: false
                }
            )
        ).rejects.toThrow(/Inflated data exceeded expected uncompressed size: expected 2 bytes/);
    });
});

interface StoredZipEntry {
    readonly name: string;
    readonly data: Uint8Array;
}

function chituboxZipEntries(): StoredZipEntry[] {
    return [
        storedEntry("run.gcode", encodeText(chituboxZipGcode())),
        storedEntry("preview.png", new Uint8Array([99])),
        storedEntry("preview_cropping.png", new Uint8Array([98])),
        storedEntry("1.png", new Uint8Array([1])),
        storedEntry("2.png", new Uint8Array([2]))
    ];
}

function anycubicZipEntries(): StoredZipEntry[] {
    return [
        storedEntry("anycubic_photon_resins.pwsp", encodeText(anycubicSettingsJson())),
        storedEntry("layers_controller.conf", encodeText(anycubicLayersJson())),
        storedEntry("print_info.json", encodeText('{"print_time":12,"volume":1.5,"weight":2.5,"cost":3.5}')),
        storedEntry("layer_images/layer_0.pw0Img", new Uint8Array([1])),
        storedEntry("layer_images/layer_1.pw0Img", new Uint8Array([2]))
    ];
}

function storedEntry(name: string, data: Uint8Array): StoredZipEntry {
    return { name, data };
}

function buildStoredZipArchive(entries: readonly StoredZipEntry[]): Uint8Array {
    const chunks: Uint8Array[] = [];
    const centralDirectory: Uint8Array[] = [];
    let offset = 0;

    for (const entry of entries) {
        const nameBytes = encodeText(entry.name);
        const localHeader = new Uint8Array(30 + nameBytes.length);
        const localView = new DataView(localHeader.buffer);
        localView.setUint32(0, 0x04034b50, true);
        localView.setUint16(26, nameBytes.length, true);
        localHeader.set(nameBytes, 30);
        chunks.push(localHeader, entry.data);

        const centralHeader = new Uint8Array(46 + nameBytes.length);
        const centralView = new DataView(centralHeader.buffer);
        centralView.setUint32(0, 0x02014b50, true);
        centralView.setUint16(10, 0, true);
        centralView.setUint32(20, entry.data.length, true);
        centralView.setUint32(24, entry.data.length, true);
        centralView.setUint16(28, nameBytes.length, true);
        centralView.setUint32(42, offset, true);
        centralHeader.set(nameBytes, 46);
        centralDirectory.push(centralHeader);
        offset += localHeader.length + entry.data.length;
    }

    const centralDirectoryOffset = offset;
    const centralDirectorySize = centralDirectory.reduce((sum, chunk) => sum + chunk.length, 0);
    const eocd = new Uint8Array(22);
    const eocdView = new DataView(eocd.buffer);
    eocdView.setUint32(0, 0x06054b50, true);
    eocdView.setUint16(8, entries.length, true);
    eocdView.setUint16(10, entries.length, true);
    eocdView.setUint32(12, centralDirectorySize, true);
    eocdView.setUint32(16, centralDirectoryOffset, true);

    return concatBytes([...chunks, ...centralDirectory, eocd]);
}

function concatBytes(chunks: readonly Uint8Array[]): Uint8Array {
    const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
    const result = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
        result.set(chunk, offset);
        offset += chunk.length;
    }
    return result;
}

function chituboxZipGcode(): string {
    return `;machineType: Mini 4K
;layerHeight: 0.05
;resolutionX: 4
;resolutionY: 2
;totalLayer: 2
;bottomLayerCount: 1
`;
}

function anycubicSettingsJson(): string {
    return JSON.stringify({
        machine_type: {
            name: "Photon",
            res_x: 4,
            res_y: 2,
            print_xsize: 10,
            print_ysize: 5,
            print_zsize: 100,
            raster_antialiasing: 1
        }
    });
}

function anycubicLayersJson(): string {
    return JSON.stringify({
        paras: [
            { layer_index: 0, layer_thickness: 0.05, exposure_time: 2, zup_height: 5, zup_speed: 60 },
            { layer_index: 1, layer_thickness: 0.05, exposure_time: 2, zup_height: 5, zup_speed: 60 }
        ]
    });
}

function encodeText(value: string): Uint8Array {
    return new TextEncoder().encode(value);
}

function toArrayBuffer(data: Uint8Array): ArrayBuffer {
    const buffer = new ArrayBuffer(data.byteLength);
    new Uint8Array(buffer).set(data);
    return buffer;
}
