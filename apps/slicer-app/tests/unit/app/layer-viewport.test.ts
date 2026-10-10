import { describe, expect, it } from "vitest";

import { layerPixelToMm, layerTopMm, type LayerPlate } from "../../../src/features/converter/ui/layer-viewport";

const plate: LayerPlate = {
    nativeWidth: 100,
    nativeHeight: 50,
    platformWidth: 200,
    platformHeight: 100,
    mirrorX: false,
    mirrorY: false
};

describe("layer viewport plate mapping / 层图板面换算", () => {
    it("converts a pixel center around the plate origin / 把像素中心换到板心原点", () => {
        expect(layerPixelToMm(plate, 0, 0)).toEqual({ x: -99, y: 49 });
        expect(layerPixelToMm(plate, 99, 49)).toEqual({ x: 99, y: -49 });
    });

    it("mirrors axes before converting to millimetres / 换算毫米前镜像坐标轴", () => {
        expect(layerPixelToMm({ ...plate, mirrorX: true, mirrorY: true }, 0, 0)).toEqual({ x: 99, y: -49 });
    });

    it("returns null without a plate size / 没有板面尺寸时返回空", () => {
        expect(layerPixelToMm({ ...plate, platformWidth: 0 }, 1, 1)).toBeNull();
    });

    it("estimates layer-top Z from uniform thickness / 按均匀层厚估算层顶 Z", () => {
        expect(layerTopMm(0, 0.05)).toBeCloseTo(0.05);
        expect(layerTopMm(3, undefined)).toBeCloseTo(0.2);
    });
});
