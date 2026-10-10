import { afterEach, describe, expect, it, vi } from "vitest";

import {
    createBlobOutputSink,
    createSlicerDocument,
    parseChituboxZipFile,
    parseSl1File,
    parseVdtFile,
    type ZipPreparedEntry
} from "../../src/index.js";
import { assembleZipArchiveForTest } from "./support/zip-test-archive.js";

describe("zip header parser / ZIP header parser", () => {
    afterEach(() => {
        vi.unstubAllGlobals();
        vi.restoreAllMocks();
    });

    it("orders VDT numeric layer names by layer index / 按 layer index 排序 VDT numeric layer names", async () => {
        const archiveBytes = await buildStoredVdtArchive([0, 1, 10, 11, 2, 3, 4, 5, 6, 7, 8, 9]);
        const sourceFile = new File([toArrayBuffer(archiveBytes)], "sample.vdt");

        const source = await parseVdtFile(sourceFile);
        const document = createSlicerDocument({ sourceFile, ...source });

        for (let layerIndex = 0; layerIndex < 12; layerIndex += 1) {
            await expect(document.readLayerPayload(layerIndex)).resolves.toMatchObject({
                kind: "zip-entry",
                bytes: new Uint8Array([layerIndex])
            });
        }
    });

    it("returns a browser document source for ZIP formats / 返回 ZIP 格式的 browser document source", async () => {
        const archiveBytes = await buildStoredVdtArchive([0, 1]);
        const sourceFile = new File([toArrayBuffer(archiveBytes)], "sample.vdt");

        const source = await parseVdtFile(sourceFile);

        expect(source.parsedHeader).toMatchObject({
            format: "vdt",
            metadata: { layer_count: 2 },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 }
            ]
        });
        expect(source.layerSource.kind).toBe("zip");

        const document = createSlicerDocument({ sourceFile, ...source });
        await expect(document.readLayerPayload(0)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([0]),
            compressionMethod: 0,
            uncompressedSize: 1
        });
    });

    it("orders SL1 five-digit layer suffixes by layer index / 按 layer index 排序 SL1 五位 layer suffix", async () => {
        const archiveBytes = await buildStoredSl1Archive([0, 2, 1]);
        const sourceFile = new File([toArrayBuffer(archiveBytes)], "sample.sl1");

        const source = await parseSl1File(sourceFile);
        const document = createSlicerDocument({ sourceFile, ...source });

        expect(source.parsedHeader).toMatchObject({
            format: "sl1",
            metadata: { layer_count: 3, layer_height_mm: 0.05 },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 },
                { offset: 2, size: 1 }
            ]
        });

        for (let layerIndex = 0; layerIndex < 3; layerIndex += 1) {
            await expect(document.readLayerPayload(layerIndex)).resolves.toMatchObject({
                kind: "zip-entry",
                bytes: new Uint8Array([layerIndex])
            });
        }
    });

    it("opens Chitubox ZIP from its central directory and run.gcode / 从 central directory 和 run.gcode 打开 Chitubox ZIP", async () => {
        const archiveBytes = await buildStoredChituboxZipArchive([2, 1]);
        const sourceFile = new File([toArrayBuffer(archiveBytes)], "sample.zip");

        const source = await parseChituboxZipFile(sourceFile);
        const document = createSlicerDocument({ sourceFile, ...source });

        expect(source.parsedHeader).toMatchObject({
            format: "chitubox-zip",
            metadata: {
                machine_name: "Mini 4K",
                resolution_x: 4,
                resolution_y: 2,
                display_width_mm: 80,
                display_height_mm: 40,
                machine_z_mm: 160,
                layer_height_mm: 0.05,
                layer_count: 2,
                bottom_layer_count: 1,
                anti_alias_level: 4,
                exposure_time: 2.5,
                bottom_exposure_time: 30,
                light_off_delay: 1.25,
                bottom_light_off_delay: 0.5,
                light_pwm: 180,
                bottom_light_pwm: 230,
                lift_height: 6,
                lift_speed: 60,
                bottom_lift_height: 7,
                bottom_lift_speed: 45,
                retract_speed: 150,
                mirror_x: true,
                print_time_sec: 1234,
                volume_ml: 3.5,
                weight_g: 4.2,
                cost: 1.2
            },
            layers: [
                { offset: 0, size: 1 },
                { offset: 1, size: 1 }
            ]
        });

        await expect(document.readLayerPayload(0)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([1]),
            compressionMethod: 0,
            uncompressedSize: 1
        });
        await expect(document.readLayerPayload(1)).resolves.toMatchObject({
            kind: "zip-entry",
            bytes: new Uint8Array([2])
        });
    });

    it("decodes Chitubox ZIP preview PNG entries into thumbnails / 将 Chitubox ZIP preview PNG entries 解码为 thumbnails", async () => {
        installThumbnailDecodeMocks();
        const archiveBytes = await buildStoredChituboxZipArchive([1, 2]);
        const sourceFile = new File([toArrayBuffer(archiveBytes)], "sample.zip");

        const source = await parseChituboxZipFile(sourceFile);

        expect(source.parsedHeader.metadata.thumbnails).toHaveLength(2);
        expect(source.parsedHeader.metadata.thumbnails?.[0]).toMatchObject({ width: 2, height: 1 });
        expect(source.parsedHeader.metadata.thumbnails?.[0]?.pixels_rgba).toBeInstanceOf(Uint8Array);
        expect(Array.from(source.parsedHeader.metadata.thumbnails?.[0]?.pixels_rgba ?? [])).toEqual([
            10, 20, 30, 255, 40, 50, 60, 255
        ]);
        expect(source.parsedHeader.metadata.thumbnails?.[1]).toMatchObject({ width: 2, height: 1 });
        expect(Array.from(source.parsedHeader.metadata.thumbnails?.[1]?.pixels_rgba ?? [])).toEqual([
            70, 80, 90, 255, 100, 110, 120, 255
        ]);
    });
});

async function buildStoredVdtArchive(layerWriteOrder: number[]): Promise<Uint8Array> {
    const manifest = {
        machine: { resolution_x: 64, resolution_y: 32 },
        print: { layer_thickness: 0.05 },
        layers: Array.from({ length: layerWriteOrder.length }, (_, index) => ({
            height: (index + 1) * 0.05
        }))
    };
    const entries: ZipPreparedEntry[] = [
        storedEntry("manifest.json", new TextEncoder().encode(JSON.stringify(manifest))),
        ...layerWriteOrder.map((layerIndex) => storedEntry(`${layerIndex}.png`, new Uint8Array([layerIndex])))
    ];
    return assembleStoredArchive(entries);
}

async function buildStoredSl1Archive(layerWriteOrder: number[]): Promise<Uint8Array> {
    const entries: ZipPreparedEntry[] = [
        storedEntry("config.ini", new TextEncoder().encode("layer_height = 0.05\nexp_time = 2\n")),
        ...layerWriteOrder.map((layerIndex) =>
            storedEntry(`layers/slice_${layerIndex.toString().padStart(5, "0")}.png`, new Uint8Array([layerIndex]))
        )
    ];
    return assembleStoredArchive(entries);
}

async function buildStoredChituboxZipArchive(layerWriteOrder: number[]): Promise<Uint8Array> {
    const entries: ZipPreparedEntry[] = [
        storedEntry("run.gcode", new TextEncoder().encode(chituboxZipGcode())),
        storedEntry("preview.png", new Uint8Array([99])),
        storedEntry("preview_cropping.png", new Uint8Array([98])),
        ...layerWriteOrder.map((layerNumber) => storedEntry(`${layerNumber}.png`, new Uint8Array([layerNumber])))
    ];
    return assembleStoredArchive(entries);
}

function chituboxZipGcode(): string {
    return `;(****Build and Slicing Parameters****)
;fileName: generated.zip
;machineType: Mini 4K
;estimatedPrintTime: 1234
;volume: 3.5
;weight: 4.2
;price: 1.2
;layerHeight: 0.05
;resolutionX: 4
;resolutionY: 2
;machineX: 80
;machineY: 40
;machineZ: 160
;normalExposureTime: 2.5
;bottomLayerExposureTime: 30
;normalDropSpeed: 150
;normalLayerLiftSpeed: 60
;normalLayerLiftHeight: 6
;bottomLayerCount: 1
;totalLayer: 2
;bottomLayerLiftHeight: 7
;bottomLayerLiftSpeed: 45
;bottomLightOffTime: 0.5
;lightOffTime: 1.25
;bottomPWMLight: 230
;PWMLight: 180
;antiAliasLevel: 4
;mirror: 1
`;
}

async function assembleStoredArchive(entries: ZipPreparedEntry[]): Promise<Uint8Array> {
    const result = await assembleZipArchiveForTest(entries, createBlobOutputSink());
    return new Uint8Array(await result.blob!.arrayBuffer());
}

function storedEntry(name: string, data: Uint8Array): ZipPreparedEntry {
    return {
        name,
        compressionMethod: 0,
        compressedData: data,
        uncompressedSize: data.length,
        crc32: 0
    };
}

function toArrayBuffer(data: Uint8Array): ArrayBuffer {
    const buffer = new ArrayBuffer(data.byteLength);
    new Uint8Array(buffer).set(data);
    return buffer;
}

function installThumbnailDecodeMocks(): void {
    const imagePixels = [
        new Uint8ClampedArray([10, 20, 30, 255, 40, 50, 60, 255]),
        new Uint8ClampedArray([70, 80, 90, 255, 100, 110, 120, 255])
    ];

    vi.stubGlobal(
        "createImageBitmap",
        vi.fn(async () => ({ width: 2, height: 1, close: vi.fn() }))
    );

    vi.stubGlobal("document", {
        createElement: vi.fn((tagName: string) => {
            if (tagName !== "canvas") {
                throw new Error(`Unexpected element request ${tagName}`);
            }

            const canvas = {
                width: 0,
                height: 0,
                getContext: vi.fn(() => ({
                    drawImage: vi.fn(),
                    getImageData: vi.fn(() => ({ data: imagePixels.shift() ?? new Uint8ClampedArray() }))
                }))
            };
            return canvas as unknown as HTMLCanvasElement;
        })
    });
}
