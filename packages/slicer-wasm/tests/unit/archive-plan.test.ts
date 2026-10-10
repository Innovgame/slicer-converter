import { describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
    describeArchiveLayerEntries: vi
        .fn()
        .mockReturnValue(
            JSON.stringify([
                { index: 3, name: "demo00003.png", compression_method: 8 },
                { index: 4, name: "demo00004.png", compression_method: 8 }
            ])
        )
}));

vi.mock("../../lib/slicer_converter.js", async (importOriginal) => ({
    ...((await importOriginal()) as object),
    describe_archive_layer_entries: mocks.describeArchiveLayerEntries
}));

describe("archive layer descriptor provider / archive layer descriptor provider", () => {
    it("creates an indexed descriptor provider for prepared ZIP batches / 创建 prepared ZIP batch 的 indexed descriptor provider", async () => {
        const { createArchiveLayerDescriptorProvider } = await import("../../src/index.js");

        const describeLayer = createArchiveLayerDescriptorProvider({
            targetFormat: "sl1",
            jobName: "demo",
            startIndex: 3,
            count: 2
        });

        expect(describeLayer(4)).toEqual({ index: 4, name: "demo00004.png", compressionMethod: 8 });
        expect(() => describeLayer(5)).toThrow("Missing ZIP layer descriptor for target format sl1 layer 5");
        expect(mocks.describeArchiveLayerEntries).toHaveBeenCalledWith("sl1", "demo", 3, 2);
    });

    it("returns one descriptor for a requested layer / 返回 requested layer 的单个 descriptor", async () => {
        const { describeArchiveLayerEntry } = await import("../../src/index.js");

        const descriptor = describeArchiveLayerEntry({
            targetFormat: "sl1",
            jobName: "demo",
            index: 3
        });

        expect(descriptor).toEqual({ index: 3, name: "demo00003.png", compressionMethod: 8 });
        expect(mocks.describeArchiveLayerEntries).toHaveBeenCalledWith("sl1", "demo", 3, 1);
    });
});
