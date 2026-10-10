import { describe, expect, it } from "vitest";

import { parseGooHeaderStreaming } from "../../src/browser/format/goo-header-parser.js";

const SMALL_PREVIEW_W = 116;
const SMALL_PREVIEW_H = 116;
const SMALL_PREVIEW_BYTES = SMALL_PREVIEW_W * SMALL_PREVIEW_H * 2;
const FIXED_HEADER_BYTES = 195_477;

describe("parseGooHeaderStreaming / parseGooHeaderStreaming 用例", () => {
    it("stores decoded thumbnail rgba pixels in Uint8Array buffers / stores decoded thumbnail rgba pixels in Uint8Array buffers 用例", async () => {
        const file = createMinimalGooFile();

        const parsed = await parseGooHeaderStreaming(file);

        expect(parsed.metadata.thumbnails).toHaveLength(2);
        expect(parsed.metadata.thumbnails?.[0]?.pixels_rgba).toBeInstanceOf(Uint8Array);
        expect(Array.from(parsed.metadata.thumbnails?.[0]?.pixels_rgba?.slice(0, 4) ?? [])).toEqual([248, 0, 0, 255]);
        expect(parsed.metadata.thumbnails?.[1]?.pixels_rgba).toBeInstanceOf(Uint8Array);
        expect(Array.from(parsed.metadata.thumbnails?.[1]?.pixels_rgba?.slice(0, 4) ?? [])).toEqual([0, 252, 0, 255]);
    });
});

function createMinimalGooFile(): File {
    const bytes = new Uint8Array(FIXED_HEADER_BYTES);

    bytes.set([0x07, 0x00, 0x00, 0x00], 4);
    bytes.set([0x44, 0x4c, 0x50, 0x00], 8);

    let pos = 0;
    pos += 4 + 8;
    pos += 32 + 24 + 24;
    pos += 32;
    pos += 32 + 32;
    pos += 2 + 2 + 2;

    const smallPreviewOffset = pos;
    bytes[smallPreviewOffset] = 0xf8;
    bytes[smallPreviewOffset + 1] = 0x00;
    pos += SMALL_PREVIEW_BYTES;
    pos += 2;

    const bigPreviewOffset = pos;
    bytes[bigPreviewOffset] = 0x07;
    bytes[bigPreviewOffset + 1] = 0xe0;

    return new File([bytes], "sample.goo", { type: "application/octet-stream" });
}
