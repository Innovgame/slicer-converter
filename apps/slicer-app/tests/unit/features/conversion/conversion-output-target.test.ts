import { readFile } from "node:fs/promises";

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
    syncFormatFacts,
    type PatchableSlicerOutputSink,
    type ResolvedSlicerExportTarget,
    type SupportedFormatInfo
} from "@slicer-converter/wasm";
import { openConversionOutputTarget } from "../../../../src/features/conversion/model/conversion-output-target";
import { formatFact } from "../../support/format-facts";

const mocks = vi.hoisted(() => ({
    createBlobOutputSink: vi.fn(),
    createBufferedOutputSink: vi.fn(),
    openFileSystemSink: vi.fn()
}));

vi.mock("@slicer-converter/wasm", async (importOriginal) => {
    const actual = await importOriginal<typeof import("@slicer-converter/wasm")>();
    return {
        ...actual,
        createBlobOutputSink: mocks.createBlobOutputSink,
        createBufferedOutputSink: mocks.createBufferedOutputSink,
        openFileSystemSink: mocks.openFileSystemSink
    };
});

describe("openConversionOutputTarget / 转换输出目标", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        syncFormatFacts(createFormatFacts());
        mocks.createBlobOutputSink.mockImplementation(() => createSink(false));
        mocks.createBufferedOutputSink.mockImplementation((sink: PatchableSlicerOutputSink) => ({
            ...sink,
            streaming: true
        }));
    });

    it("falls back to Blob mode when File System Access is unavailable / File System Access 不可用时回退 Blob mode", async () => {
        const blobSink = createSink(false);
        mocks.openFileSystemSink.mockResolvedValueOnce(null);
        mocks.createBlobOutputSink.mockReturnValueOnce(blobSink);

        const target = await openConversionOutputTarget({
            sourceFileName: "sample.goo",
            target: createTarget({ requestedTargetFormat: "ctb", resolvedTargetFormat: "ctb" })
        });

        expect(target).toMatchObject({
            downloadName: "sample.ctb",
            jobName: "sample",
            isZipTarget: false,
            mode: "blob"
        });
        expect(target.sink).toBe(blobSink);
        expect(mocks.openFileSystemSink).toHaveBeenCalledWith("sample.ctb", "application/octet-stream", [".ctb"], {
            onDiagnostic: expect.any(Function)
        });
        expect(mocks.createBlobOutputSink).toHaveBeenCalledTimes(1);
        expect(mocks.createBufferedOutputSink).not.toHaveBeenCalled();
    });

    it("streams through a buffered sink when File System Access is available / File System Access 可用时通过 buffered sink streaming", async () => {
        const fileSystemSink = createSink(true);
        const bufferedSink = createSink(true);
        mocks.openFileSystemSink.mockResolvedValueOnce(fileSystemSink);
        mocks.createBufferedOutputSink.mockReturnValueOnce(bufferedSink);

        const target = await openConversionOutputTarget({
            sourceFileName: "sample.goo",
            target: createTarget({ requestedTargetFormat: "vdt", resolvedTargetFormat: "vdt" })
        });

        expect(target).toMatchObject({
            downloadName: "sample.vdt",
            jobName: "sample",
            isZipTarget: true,
            mode: "streaming"
        });
        expect(target.sink).toBe(bufferedSink);
        expect(mocks.openFileSystemSink).toHaveBeenCalledWith("sample.vdt", "application/zip", [".vdt"], {
            onDiagnostic: expect.any(Function)
        });
        expect(mocks.createBlobOutputSink).not.toHaveBeenCalled();
        expect(mocks.createBufferedOutputSink).toHaveBeenCalledWith(fileSystemSink);
    });

    it("falls back to Blob mode when the file picker throws / file picker 抛错时回退 Blob mode", async () => {
        const blobSink = createSink(false);
        mocks.openFileSystemSink.mockRejectedValueOnce(new Error("picker cancelled"));
        mocks.createBlobOutputSink.mockReturnValueOnce(blobSink);

        const target = await openConversionOutputTarget({
            sourceFileName: "sample.goo",
            target: createTarget({ requestedTargetFormat: "ctb", resolvedTargetFormat: "ctb" })
        });

        expect(target.mode).toBe("blob");
        expect(target.sink).toBe(blobSink);
        expect(mocks.createBlobOutputSink).toHaveBeenCalledTimes(1);
        expect(mocks.createBufferedOutputSink).not.toHaveBeenCalled();
    });

    it("uses resolved manual target extension for download name and picker filters / 使用 resolved manual target extension 构建下载名和 picker filters", async () => {
        mocks.openFileSystemSink.mockResolvedValueOnce(createSink(true));

        const target = await openConversionOutputTarget({
            sourceFileName: "sample.goo",
            target: createTarget({
                requestedTargetFormat: "anycubic-manual",
                resolvedTargetFormat: "anycubic-zip",
                resolvedExtension: "pm7"
            })
        });

        expect(target.downloadName).toBe("sample.pm7");
        expect(target.jobName).toBe("sample");
        expect(target.isZipTarget).toBe(true);
        expect(target.mode).toBe("streaming");
        expect(mocks.openFileSystemSink).toHaveBeenCalledWith("sample.pm7", "application/zip", [".pm7"], {
            onDiagnostic: expect.any(Function)
        });
    });

    it("uses octet-stream for non-ZIP targets and application zip for ZIP targets / 非 ZIP 使用 octet-stream 且 ZIP 使用 application zip", async () => {
        mocks.openFileSystemSink.mockResolvedValue(createSink(true));

        await openConversionOutputTarget({
            sourceFileName: "part.goo",
            target: createTarget({ requestedTargetFormat: "ctb", resolvedTargetFormat: "ctb" })
        });
        await openConversionOutputTarget({
            sourceFileName: "part.goo",
            target: createTarget({ requestedTargetFormat: "vdt", resolvedTargetFormat: "vdt" })
        });

        expect(mocks.openFileSystemSink).toHaveBeenNthCalledWith(1, "part.ctb", "application/octet-stream", [".ctb"], {
            onDiagnostic: expect.any(Function)
        });
        expect(mocks.openFileSystemSink).toHaveBeenNthCalledWith(2, "part.vdt", "application/zip", [".vdt"], {
            onDiagnostic: expect.any(Function)
        });
    });

    it("logs File System output diagnostics from the package sink / 记录 package sink 的 File System output diagnostics", async () => {
        const consoleInfo = vi.spyOn(console, "info").mockImplementation(() => undefined);
        mocks.openFileSystemSink.mockImplementationOnce(async (_name, _mime, _extensions, options) => {
            options.onDiagnostic({
                type: "file-system-picker",
                suggestedName: "sample.vdt",
                pickerWallMs: 12
            });
            return createSink(true);
        });

        await openConversionOutputTarget({
            sourceFileName: "sample.goo",
            target: createTarget({ requestedTargetFormat: "vdt", resolvedTargetFormat: "vdt" })
        });

        expect(consoleInfo).toHaveBeenCalledWith(
            '[slicer-output] file-system-picker {"suggestedName":"sample.vdt","pickerWallMs":12}'
        );
        consoleInfo.mockRestore();
    });

    it("keeps a named internal sink result type / 保持命名的内部 sink result type", async () => {
        const source = await readFile("src/features/conversion/model/conversion-output-target.ts", "utf8");

        expect(source).toContain("interface OpenConversionOutputSinkResult");
        expect(source).toContain("Promise<OpenConversionOutputSinkResult>");
    });

    it("imports the File System sink directly from the wasm package / 直接从 wasm package 导入 File System sink", async () => {
        const source = await readFile("src/features/conversion/model/conversion-output-target.ts", "utf8");

        expect(source).toContain("openFileSystemSink");
        expect(source).toContain('@slicer-converter/wasm";');
        expect(source).not.toContain("../../infrastructure/output/file-system-output-sink");
    });
});

function createTarget(input: {
    requestedTargetFormat: ResolvedSlicerExportTarget["requestedTargetFormat"];
    resolvedTargetFormat: string;
    resolvedExtension?: string;
}): ResolvedSlicerExportTarget {
    return {
        ...input,
        resolvedTargetFormat: input.resolvedTargetFormat as ResolvedSlicerExportTarget["resolvedTargetFormat"],
        statusMessage: null
    };
}

function createSink(streaming: boolean): PatchableSlicerOutputSink {
    return {
        streaming,
        write: vi.fn(),
        patchHeader: vi.fn(),
        finish: vi.fn(async () => (streaming ? null : new Blob())),
        abort: vi.fn(async () => undefined)
    };
}

function createFormatFacts(): SupportedFormatInfo[] {
    return [
        formatFact({
            name: "ctb",
            display_name: "CTB",
            aliases: ["ctb"],
            extensions: ["ctb"],
            default_extension: "ctb",
            is_zip: false
        }),
        formatFact({
            name: "vdt",
            display_name: "VDT",
            aliases: ["vdt"],
            extensions: ["vdt"],
            default_extension: "vdt",
            is_zip: true
        }),
        formatFact({
            name: "anycubic-zip",
            display_name: "Anycubic ZIP",
            aliases: ["anycubic-zip", "pm7"],
            extensions: ["pm7"],
            default_extension: "pm7",
            is_zip: true
        })
    ];
}
