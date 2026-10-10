import { readFileSync } from "node:fs";
import path from "node:path";

import { beforeEach, describe, expect, it, vi } from "vitest";

import { createBlobOutputSink } from "../../src/browser/output/sink.js";

const mocks = vi.hoisted(() => ({
    mode: "streaming" as "streaming" | "buffered" | "flat",
    assembly: {
        buildHeader: vi.fn(() => new Uint8Array([1])),
        buildFooter: vi.fn(() => new Uint8Array([9])),
        buildDocument: vi.fn(
            (
                _metadataJson: string,
                _targetFormat: string,
                _targetAnycubicVersion: number | undefined,
                layers: Uint8Array[]
            ) => new Uint8Array([30, layers.length, ...Array.from(layers[0]), ...Array.from(layers[1])])
        ),
        finalizeHeader: vi.fn(() => new Uint8Array([7])),
        buildPrefix: vi.fn(() => [new Uint8Array([10]), new Uint8Array([20]), new Uint8Array([21])]),
        checksumUpdate: vi.fn((previous: number, chunk: Uint8Array) => previous + chunk[0]),
        buildChecksumTrailer: vi.fn(() => new Uint8Array([0, 0, 0, 58]))
    }
}));

vi.mock("../../src/browser/output/document-assembly.js", () => ({
    createStreamingDocumentAssemblyHelpers: () => ({
        buildHeader: mocks.assembly.buildHeader,
        finalizeHeader: mocks.assembly.finalizeHeader,
        buildFooter: mocks.assembly.buildFooter,
        streamingDocument:
            mocks.mode === "flat"
                ? undefined
                : mocks.mode === "buffered"
                  ? {
                        buildDocument: mocks.assembly.buildDocument
                    }
                  : {
                        buildPrefix: mocks.assembly.buildPrefix,
                        checksumUpdate: mocks.assembly.checksumUpdate,
                        buildChecksumTrailer: mocks.assembly.buildChecksumTrailer
                    }
    })
}));

vi.mock("../../src/browser/format/header.js", () => ({
    buildFileMetadataJson: (metadata: unknown) => JSON.stringify(metadata)
}));

beforeEach(() => {
    mocks.mode = "streaming";
    vi.clearAllMocks();
    vi.resetModules();
});

describe("wasm binary output assembly / WASM 二进制输出 assembly", () => {
    it("routes CXDLPv4 through streaming document helpers / 路由 CXDLPv4 through streaming document helpers", async () => {
        const { assembleWasmBinaryOutputFromLayerStream } = await import("../../src/browser/output/wasm-binary.js");

        async function* layerStream(): AsyncGenerator<Uint8Array, void, void> {
            yield new Uint8Array([3]);
            yield new Uint8Array([4, 5]);
        }

        const result = await assembleWasmBinaryOutputFromLayerStream({
            metadata: { resolution_x: 1, layer_count: 2 },
            targetFormat: "cxdlpv4",
            layerCount: 2,
            layerStream: layerStream(),
            sink: createBlobOutputSink()
        });

        expect(mocks.assembly.buildHeader).not.toHaveBeenCalled();
        expect(mocks.assembly.finalizeHeader).not.toHaveBeenCalled();
        expect(mocks.assembly.buildFooter).not.toHaveBeenCalled();
        expect(mocks.assembly.buildPrefix).toHaveBeenCalledWith(
            '{"resolution_x":1,"layer_count":2}',
            "cxdlpv4",
            new Uint32Array([1, 2])
        );
        expect(mocks.assembly.checksumUpdate).toHaveBeenCalledTimes(5);
        expect(mocks.assembly.buildChecksumTrailer).toHaveBeenCalledWith(58);
        expect(result).not.toBeNull();
        expect(Array.from(new Uint8Array(await result!.arrayBuffer()))).toEqual([10, 20, 21, 3, 4, 5, 0, 0, 0, 58]);
    });

    it("routes encrypted CTB through buffered document helpers / 路由 encrypted CTB through buffered document helpers", async () => {
        mocks.mode = "buffered";
        const { assembleWasmBinaryOutputFromLayerStream } = await import("../../src/browser/output/wasm-binary.js");

        async function* layerStream(): AsyncGenerator<Uint8Array, void, void> {
            yield new Uint8Array([3]);
            yield new Uint8Array([4, 5]);
        }

        const result = await assembleWasmBinaryOutputFromLayerStream({
            metadata: { resolution_x: 1, layer_count: 2 },
            targetFormat: "encrypted-ctb",
            layerCount: 2,
            layerStream: layerStream(),
            sink: createBlobOutputSink()
        });

        expect(mocks.assembly.buildHeader).not.toHaveBeenCalled();
        expect(mocks.assembly.finalizeHeader).not.toHaveBeenCalled();
        expect(mocks.assembly.buildFooter).not.toHaveBeenCalled();
        expect(mocks.assembly.buildDocument).toHaveBeenCalledWith(
            '{"resolution_x":1,"layer_count":2}',
            "encrypted-ctb",
            undefined,
            [new Uint8Array([3]), new Uint8Array([4, 5])]
        );
        expect(result).not.toBeNull();
        expect(Array.from(new Uint8Array(await result!.arrayBuffer()))).toEqual([30, 2, 3, 4, 5]);
    });

    it("passes Anycubic target version to header helpers / 传递 Anycubic target version to header helpers", async () => {
        mocks.mode = "flat";
        const { assembleWasmBinaryOutputFromLayerStream } = await import("../../src/browser/output/wasm-binary.js");

        async function* layerStream(): AsyncGenerator<Uint8Array, void, void> {
            yield new Uint8Array([3]);
            yield new Uint8Array([4, 5]);
        }

        const result = await assembleWasmBinaryOutputFromLayerStream({
            metadata: { resolution_x: 1, layer_count: 2 },
            targetFormat: "pm5s",
            targetAnycubicVersion: 515,
            layerCount: 2,
            layerStream: layerStream(),
            sink: createBlobOutputSink()
        });

        expect(mocks.assembly.buildHeader).toHaveBeenCalledWith('{"resolution_x":1,"layer_count":2}', "pm5s", 515);
        expect(mocks.assembly.finalizeHeader).toHaveBeenCalledWith(
            new Uint8Array([1]),
            "pm5s",
            new Uint32Array([1, 2]),
            515
        );
        expect(result).not.toBeNull();
        expect(Array.from(new Uint8Array(await result!.arrayBuffer()))).toEqual([7, 3, 4, 5, 9]);
    });

    it("keeps raw WASM ABI names out of binary output source / 保持 raw WASM ABI names outside binary output source", () => {
        const sources = ["src/browser/output/binary.ts", "src/browser/output/wasm-binary.ts"]
            .map((relativePath) => readFileSync(path.join(process.cwd(), relativePath), "utf8"))
            .join("\n");

        expect(sources).not.toContain("build_cxdlpv4_document_header");
        expect(sources).not.toContain("build_cxdlpv4_layer_def");
        expect(sources).not.toContain("build_document_from_layers");
        expect(sources).not.toContain("cxdlp_crc32_update");
    });
});
