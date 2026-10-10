import { afterEach, describe, expect, it, vi } from "vitest";

import { inflateRaw } from "../../src/index.js";

describe("inflateRaw / raw deflate inflater", () => {
    afterEach(() => {
        vi.unstubAllGlobals();
    });

    it("fills a preallocated buffer when the expected size is known / 填充 a preallocated buffer 当 the expected size is known", async () => {
        const reader = {
            read: vi
                .fn()
                .mockResolvedValueOnce({ done: false, value: new Uint8Array([1, 2]) })
                .mockResolvedValueOnce({ done: false, value: new Uint8Array([3, 4]) })
                .mockResolvedValueOnce({ done: true, value: undefined }),
            cancel: vi.fn(async () => undefined)
        };
        const writer = {
            write: vi.fn(async () => undefined),
            close: vi.fn(async () => undefined)
        };

        class MockDecompressionStream {
            readable = { getReader: () => reader };
            writable = { getWriter: () => writer };

            constructor(format: string) {
                expect(format).toBe("deflate-raw");
            }
        }

        vi.stubGlobal("DecompressionStream", MockDecompressionStream as unknown as typeof DecompressionStream);

        const result = await inflateRaw(new Uint8Array([9, 8]), 4);

        expect(writer.write).toHaveBeenCalledOnce();
        expect(writer.close).toHaveBeenCalledOnce();
        expect(Array.from(result)).toEqual([1, 2, 3, 4]);
    });

    it("concatenates chunks when the expected size is omitted / expected size 省略时拼接 chunks", async () => {
        const reader = {
            read: vi
                .fn()
                .mockResolvedValueOnce({ done: false, value: new Uint8Array([1]) })
                .mockResolvedValueOnce({ done: false, value: new Uint8Array([2, 3]) })
                .mockResolvedValueOnce({ done: true, value: undefined }),
            cancel: vi.fn(async () => undefined)
        };
        const writer = {
            write: vi.fn(async () => undefined),
            close: vi.fn(async () => undefined)
        };

        class MockDecompressionStream {
            readable = { getReader: () => reader };
            writable = { getWriter: () => writer };
        }

        vi.stubGlobal("DecompressionStream", MockDecompressionStream as unknown as typeof DecompressionStream);

        const result = await inflateRaw(new Uint8Array([9, 8]));

        expect(Array.from(result)).toEqual([1, 2, 3]);
    });

    it("throws when decompressed data exceeds the expected size / 抛出 当 decompressed data exceeds the expected size", async () => {
        const reader = {
            read: vi
                .fn()
                .mockResolvedValueOnce({ done: false, value: new Uint8Array([1, 2, 3]) })
                .mockResolvedValueOnce({ done: true, value: undefined }),
            cancel: vi.fn(async () => undefined)
        };
        const writer = {
            write: vi.fn(async () => undefined),
            close: vi.fn(async () => undefined)
        };

        class MockDecompressionStream {
            readable = { getReader: () => reader };
            writable = { getWriter: () => writer };
        }

        vi.stubGlobal("DecompressionStream", MockDecompressionStream as unknown as typeof DecompressionStream);

        await expect(inflateRaw(new Uint8Array([9, 8]), 2)).rejects.toThrow(/expected uncompressed size/i);
        expect(reader.cancel).toHaveBeenCalledOnce();
    });
});
