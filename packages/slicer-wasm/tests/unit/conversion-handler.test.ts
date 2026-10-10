import { readFile } from "node:fs/promises";
import { describe, expect, it, vi } from "vitest";

import type { SlicerWorkerResponse } from "../../src/browser/worker/protocol.js";

describe("conversion worker handlers / 转换 worker handlers", () => {
    it("centralizes ZIP prepared-entry response packing / centralizes ZIP prepared-entry response packing 用例", async () => {
        const source = await readFile("src/browser/worker/handlers/conversion.ts", "utf8");

        expect(source).toContain("function buildZipLayerPreparedResponse");
        expect(source.match(/type:\s*"zip-layer-prepared"/g) ?? []).toHaveLength(1);
    });

    it("posts one prepared ZIP layer message for single-layer prepare requests / posts one prepared ZIP layer message for single-layer prepare requests 用例", async () => {
        const { createSlicerConversionHandlers } = await import("../../src/browser/worker/handlers/conversion.js");
        const posted: Array<{ message: unknown; transfer?: Transferable[] }> = [];
        const decodeEncodeLayer = vi.fn(() => new Uint8Array([0xaa, 0xbb, 0xcc]));
        const prepareZipEntry = vi.fn(async () => ({
            entry: {
                name: "0.png",
                compressionMethod: 0 as const,
                compressedData: new Uint8Array([0xaa, 0xbb, 0xcc]),
                uncompressedSize: 3,
                crc32: 0x12345678
            },
            timings: {
                crcMs: 1,
                deflateMs: 2,
                packMs: 3,
                totalMs: 6
            }
        }));
        let now = 10;
        const handlers = createSlicerConversionHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) => posted.push({ message, transfer }),
            now: () => {
                now += 5;
                return now;
            },
            decodeEncodeLayer,
            decodeLayerPreview: vi.fn(),
            inflateRaw: vi.fn(),
            prepareZipEntry,
            convertZipLayerForTarget: vi.fn(),
            decodeZipPreview: vi.fn()
        } as never);

        handlers["decode-encode-prepare"]({
            type: "decode-encode-prepare",
            index: 0,
            data: new Uint8Array([1, 2, 3]).buffer as ArrayBuffer,
            sourceFormat: "goo",
            targetFormat: "vdt",
            targetAnycubicVersion: null,
            width: 64,
            height: 32,
            antiAliasLevel: 1,
            encryptionKey: 0,
            layerMetadataJson: "{}",
            entryName: "0.png",
            entryCompressionMethod: 0
        });

        await waitForPostedMessages(posted, 1);

        expect(posted[0]!.message).toMatchObject({
            type: "zip-layer-prepared",
            index: 0,
            name: "0.png",
            compressionMethod: 0,
            crc32: 0x12345678,
            uncompressedSize: 3,
            computeMs: expect.any(Number),
            timings: {
                encodeMs: expect.any(Number),
                zipPrepareMs: expect.any(Number),
                wasmInputBytes: 3,
                wasmOutputBytes: 3,
                crcMs: 1,
                deflateMs: 2,
                zipPackMs: 3,
                totalMs: expect.any(Number)
            }
        });
        expect(new Uint8Array((posted[0]!.message as { data: ArrayBuffer }).data)).toEqual(
            new Uint8Array([0xaa, 0xbb, 0xcc])
        );
        expect(posted[0]!.transfer).toHaveLength(1);
        expect(prepareZipEntry).toHaveBeenCalledWith({
            name: "0.png",
            data: new Uint8Array([0xaa, 0xbb, 0xcc]),
            compressionMethod: 0
        });
    });

    it("passes Anycubic target version to ZIP layer conversion / 将 Anycubic 目标版本传给 ZIP 层转换", async () => {
        const { createSlicerConversionHandlers } = await import("../../src/browser/worker/handlers/conversion.js");
        const posted: Array<{ message: unknown; transfer?: Transferable[] }> = [];
        const convertZipLayerForTarget = vi.fn(async () => new Uint8Array([0x51, 0x55]));
        const handlers = createSlicerConversionHandlers({
            postMessage: (message: SlicerWorkerResponse, transfer?: Transferable[]) => posted.push({ message, transfer }),
            now: vi.fn(() => 10),
            decodeEncodeLayer: vi.fn(),
            decodeLayerPreview: vi.fn(),
            inflateRaw: vi.fn(),
            prepareZipEntry: vi.fn(),
            convertZipLayerForTarget,
            decodeZipPreview: vi.fn()
        } as never);

        handlers["zip-decode-encode"]({
            type: "zip-decode-encode",
            index: 0,
            data: new Uint8Array([1, 2, 3]).buffer as ArrayBuffer,
            compressionMethod: 8,
            uncompressedSize: 3,
            sourceFormat: "vdt",
            targetFormat: "pwms",
            targetAnycubicVersion: 515,
            width: 64,
            height: 32,
            layerMetadataJson: "{}"
        });

        await waitForPostedMessages(posted, 1);

        expect(convertZipLayerForTarget).toHaveBeenCalledWith(
            expect.objectContaining({
                targetFormat: "pwms",
                targetAnycubicVersion: 515
            })
        );
        expect(posted[0]!.message).toMatchObject({ type: "layer-done", index: 0 });
    });
});

async function waitForPostedMessages(
    posted: Array<{ message: unknown; transfer?: Transferable[] }>,
    count: number
): Promise<void> {
    const startedAt = performance.now();
    while (posted.length < count) {
        if (performance.now() - startedAt > 250) {
            throw new Error(`Timed out waiting for ${count} posted messages; got ${posted.length}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 0));
    }
}
