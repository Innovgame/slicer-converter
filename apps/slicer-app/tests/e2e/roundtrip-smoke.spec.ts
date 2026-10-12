import { readFile } from "node:fs/promises";
import { expect, test } from "@playwright/test";
import { initSlicerConverter, openSlicerDocumentFile, parseBrowserFileHeader } from "@innovgame/slicer-converter";
import { decode_layer } from "../../../../packages/slicer-wasm/lib/slicer_converter.js";
import { fixtureFilePath, loadFilePath } from "../support/fixture-files";

test("preserves layers, pixels and print settings through GOO → CTB → GOO / GOO → CTB → GOO 保留图层像素及打印参数", async ({
    page
}, testInfo) => {
    test.setTimeout(90_000);
    await page.addInitScript(() => {
        Object.defineProperty(window, "showSaveFilePicker", { configurable: true, value: undefined });
        localStorage.setItem("slicer-language", "en");
        localStorage.setItem("slicer-onboarding-v1", "dismissed");
    });
    await page.goto("/");
    await expect(page.locator("#statusMsg")).toContainText("Ready", { timeout: 30_000 });
    const sourcePath = fixtureFilePath("sample.goo");
    await loadFilePath(page, sourcePath, "GOO");
    await page.selectOption("#slicerConverterFormat", "chitubox-manual");
    await page.selectOption("#slicerConverterSuffix", "ctb");
    const ctbDownload = page.waitForEvent("download");
    await page.click("#convertBtn");
    const ctbPath = testInfo.outputPath("converted.ctb");
    await (await ctbDownload).saveAs(ctbPath);
    await expect(page.locator("#convertBtn")).toBeEnabled();
    await loadFilePath(page, ctbPath, "CTB");
    await page.selectOption("#slicerConverterFormat", "goo");
    const gooDownload = page.waitForEvent("download");
    await page.click("#convertBtn");
    const roundtripPath = testInfo.outputPath("roundtrip.goo");
    await (await gooDownload).saveAs(roundtripPath);

    await initSlicerConverter();
    const open = async (filePath: string) =>
        openSlicerDocumentFile({
            file: new File([await readFile(filePath)], filePath.split(/[\\/]/).pop()!),
            parseHeader: parseBrowserFileHeader
        });
    const source = await open(sourcePath);
    const intermediate = await open(ctbPath);
    const roundtrip = await open(roundtripPath);
    try {
        const metadata = source.parsedHeader.metadata;
        for (const document of [intermediate, roundtrip]) {
            expect(document.parsedHeader.layers.length).toBe(source.parsedHeader.layers.length);
            for (const key of [
                "resolution_x",
                "resolution_y",
                "layer_height_mm",
                "exposure_time",
                "bottom_exposure_time",
                "bottom_layer_count"
            ] as const) {
                expect(document.parsedHeader.metadata[key], key).toBeCloseTo(metadata[key]!, 5);
            }
            for (let index = 0; index < source.parsedHeader.layers.length; index++) {
                const original = await source.readLayerPayload(index);
                const converted = await document.readLayerPayload(index);
                const decode = (bytes: Uint8Array, header: typeof source.parsedHeader) =>
                    decode_layer(
                        bytes,
                        index,
                        header.format,
                        metadata.resolution_x!,
                        metadata.resolution_y!,
                        header.metadata.anti_alias_level ?? 1,
                        header.metadata.encryption_key ?? 0
                    );
                expect(decode(converted.bytes, document.parsedHeader)).toEqual(
                    decode(original.bytes, source.parsedHeader)
                );
            }
        }
    } finally {
        await Promise.all([source.close(), intermediate.close(), roundtrip.close()]);
    }
});
