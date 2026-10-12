import { describe, expect, it } from "vitest";
import { parseAnycubicZipFile, createBlobOutputSink, type ZipPreparedEntry } from "../../src/index.js";
import { assembleZipArchiveForTest } from "./support/zip-test-archive.js";

describe("Anycubic active resin metadata / Anycubic 活动树脂参数", () => {
    it("reads bottom settings from the named active resin / 从指定活动树脂读取底层参数", async () => {
        const settings = {
            machine_type: { res_x: 4, res_y: 2 },
            machine_extern: {
                active_resins: ["selected"],
                user_resins: [
                    { property: { name: "other" }, slicepara: { bott_layers: 99, bott_time: 99, exposure_time: 99 } },
                    { property: { name: "selected" }, slicepara: { bott_layers: 1, bott_time: 30, exposure_time: 2.5 } }
                ]
            }
        };
        const entry = (name: string, value: unknown): ZipPreparedEntry => {
            const bytes = new TextEncoder().encode(JSON.stringify(value));
            return { name, compressedData: bytes, compressionMethod: 0, uncompressedSize: bytes.length, crc32: 0 };
        };
        const { blob } = await assembleZipArchiveForTest(
            [
                entry("anycubic_photon_resins.pwsp", settings),
                entry("layers_controller.conf", {
                    count: 2,
                    paras: [
                        { layer_index: 0, exposure_time: 30, layer_thickness: 0.05 },
                        { layer_index: 1, exposure_time: 2.5, layer_thickness: 0.05 }
                    ]
                }),
                entry("layer_images/layer_0.pw0Img", [0, 8]),
                entry("layer_images/layer_1.pw0Img", [0, 8])
            ],
            createBlobOutputSink()
        );
        const parsed = await parseAnycubicZipFile(new File([await blob!.arrayBuffer()], "resin.pm7"));
        expect(parsed.parsedHeader.metadata).toMatchObject({
            bottom_layer_count: 1,
            bottom_exposure_time: 30,
            exposure_time: 2.5
        });
    });
});
