import { describe, expect, it } from "vitest";

import {
    buildBinaryParts,
    buildDownloadFilename,
    toOwnedArrayBuffer
} from "../../../src/shared/lib/output";

describe("download helpers / 下载 helpers", () => {
    it("builds download filenames from source names / 构建 下载 filenames 来自 source names", () => {
        expect(buildDownloadFilename("sample.goo", "ctb")).toBe("sample.ctb");
        expect(buildDownloadFilename("archive.backup.goo", "sl1")).toBe("archive.backup.sl1");
        expect(buildDownloadFilename(null, "prz")).toBe("converted.prz");
        expect(buildDownloadFilename("model.goo", "anycubic-manual", "pwmx")).toBe("model.pwmx");
        expect(buildDownloadFilename("model.goo", "chitubox-manual", "ctb")).toBe("model.ctb");
        expect(buildDownloadFilename("model.goo", "chitubox-manual", "gktwo.ctb")).toBe("model.gktwo.ctb");
        expect(buildDownloadFilename("model.goo", "anycubic-zip", "pm7")).toBe("model.pm7");
    });

    it("copies typed array slices into owned array buffers / copies typed array slices 到 owned array buffers", () => {
        const source = new Uint8Array([10, 20, 30, 40]);
        const slice = source.subarray(1, 3);
        const owned = toOwnedArrayBuffer(slice);

        expect(Array.from(new Uint8Array(owned))).toEqual([20, 30]);
    });

    it("builds binary parts and omits empty footer / 构建 binary parts 并且 omits 空 footer", () => {
        const header = new Uint8Array([1, 2]);
        const chunks = [new Uint8Array([3]), new Uint8Array([4, 5])];
        const footer = new Uint8Array([]);
        const parts = buildBinaryParts(header, chunks, footer);

        expect(parts).toHaveLength(3);
        expect(Array.from(new Uint8Array(parts[0]))).toEqual([1, 2]);
        expect(Array.from(new Uint8Array(parts[2]))).toEqual([4, 5]);
    });

    it("includes non-empty footer in binary parts / includes non-空 footer in binary parts", () => {
        const parts = buildBinaryParts(new Uint8Array([1]), [new Uint8Array([2])], new Uint8Array([9, 9]));

        expect(parts).toHaveLength(3);
        expect(Array.from(new Uint8Array(parts[2]))).toEqual([9, 9]);
    });
});
