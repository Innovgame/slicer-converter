import { describe, expect, it } from "vitest";

import { buildFileMetadataJson, buildLayerMetadataJson } from "../../src/browser/format/header.js";

describe("buildLayerMetadataJson / 构建 layer metadata JSON", () => {
    it("fills missing fields with the same defaults used by the app / 填充 缺失 fields 带有 the 相同 defaults used by the app", () => {
        expect(JSON.parse(buildLayerMetadataJson({ exposure_time: 2.5 }))).toMatchObject({
            exposure_time: 2.5,
            bottom_exposure_time: 2.5,
            light_pwm: 255,
            lift_height: 5
        });
    });
});

describe("buildFileMetadataJson / 构建 file metadata JSON", () => {
    it("serializes thumbnail typed arrays as JSON sequences / 序列化 thumbnail typed arrays as JSON sequences", () => {
        const json = buildFileMetadataJson({
            resolution_x: 64,
            thumbnails: [{ width: 1, height: 1, pixels_rgba: new Uint8Array([1, 2, 3, 255]) }]
        });

        expect(JSON.parse(json)).toEqual({
            resolution_x: 64,
            thumbnails: [{ width: 1, height: 1, pixels_rgba: [1, 2, 3, 255] }]
        });
    });
});
