import { afterEach, describe, expect, it, vi } from "vitest";

import { isFileSystemWriteSupported, openFileSystemSink } from "../../src/index.js";

const originalPicker = Object.getOwnPropertyDescriptor(globalThis, "showSaveFilePicker");

function restorePicker(): void {
    if (originalPicker) {
        Object.defineProperty(globalThis, "showSaveFilePicker", originalPicker);
        return;
    }
    Reflect.deleteProperty(globalThis, "showSaveFilePicker");
}

describe("file-system-output-sink / File System Access 输出 sink", () => {
    afterEach(() => {
        restorePicker();
    });

    it("returns null when File System Access API is unavailable / API 不可用时返回 null", async () => {
        Reflect.deleteProperty(globalThis, "showSaveFilePicker");

        expect(isFileSystemWriteSupported()).toBe(false);
        await expect(openFileSystemSink("demo.vdt", "application/zip", ["../.vdt"])).resolves.toBeNull();
    });

    it("opens a writable sink and forwards write, patch, and finish / 打开 writable sink 并转发写入、patch 和完成", async () => {
        const writable = {
            write: vi.fn(async (_data: Uint8Array | Blob | string) => undefined),
            seek: vi.fn(async (_position: number) => undefined),
            close: vi.fn(async () => undefined),
            abort: vi.fn(async (_reason?: unknown) => undefined)
        };
        const createWritable = vi.fn(async () => writable);
        const showSaveFilePicker = vi.fn(async () => ({ createWritable }));
        Object.defineProperty(globalThis, "showSaveFilePicker", {
            configurable: true,
            value: showSaveFilePicker
        });

        const sink = await openFileSystemSink("demo.vdt", "application/zip", ["../.vdt"]);

        expect(isFileSystemWriteSupported()).toBe(true);
        expect(showSaveFilePicker).toHaveBeenCalledWith({
            suggestedName: "demo.vdt",
            types: [{ description: "Slicer file", accept: { "application/zip": ["../.vdt"] } }]
        });
        expect(createWritable).toHaveBeenCalledTimes(1);
        expect(sink?.streaming).toBe(true);

        await sink?.write(new Uint8Array([1, 2]));
        await sink?.patchHeader(new Uint8Array([9, 8]));
        await sink?.finish();

        expect(writable.write).toHaveBeenNthCalledWith(1, new Uint8Array([1, 2]));
        expect(writable.seek).toHaveBeenCalledWith(0);
        expect(writable.write).toHaveBeenNthCalledWith(2, new Uint8Array([9, 8]));
        expect(writable.close).toHaveBeenCalledTimes(1);
        expect(writable.abort).not.toHaveBeenCalled();
    });
});
