import assert from "node:assert/strict";
import { test } from "node:test";
import {
    buildFormatValidationCases,
    comparePixels,
    quantizePixel,
    resizeAndQuantizePixels,
    verifyArtifactIdentity
} from "./format-validation.mjs";

test("rejects an artifact that ignored the requested version / 拒绝忽略请求版本的生成物", () => {
    const bytes = new Uint8Array(16);
    new DataView(bytes.buffer).setUint32(12, 516, true);
    assert.throws(() => verifyArtifactIdentity({ format: "pwma", version: 515 }, "pwma", bytes), /version/i);
    new DataView(bytes.buffer).setUint32(12, 515, true);
    assert.equal(verifyArtifactIdentity({ format: "pwma", version: 515 }, "pwma", bytes), 515);
    assert.throws(() => verifyArtifactIdentity({ format: "pwma", version: 515 }, "pwmx", bytes), /format/i);
    new DataView(bytes.buffer).setUint32(4, 4, true);
    assert.throws(
        () => verifyArtifactIdentity({ format: "ctb-v5", version: null, headerVersion: 5 }, "ctb-v5", bytes),
        /version/i
    );
});

test("checks an independent resampling and binary image / 独立验证重采样及二值图像", () => {
    const result = resizeAndQuantizePixels(new Uint8Array([0, 127, 128, 255]), 2, 2, 4, 2, "binary");
    assert.deepEqual(Array.from(result), [0, 0, 0, 0, 255, 255, 255, 255]);
});

test("uses independent expected grayscale mappings / 使用独立的灰度量化期望", () => {
    const source = [0, 1, 127, 128, 252, 253, 254, 255];
    assert.deepEqual(
        source.map((value) => quantizePixel(value, "gray7")),
        [0, 0, 126, 129, 253, 253, 255, 255]
    );
    assert.deepEqual(
        source.map((value) => quantizePixel(value, "gray4")),
        [0, 0, 119, 136, 255, 255, 255, 255]
    );
    assert.deepEqual(
        source.map((value) => quantizePixel(value, "binary")),
        [0, 0, 0, 255, 255, 255, 255, 255]
    );
    assert.deepEqual(
        source.map((value) => quantizePixel(value, "cxdlp")),
        [0, 0, 127, 129, 253, 253, 255, 255]
    );
    assert.deepEqual(
        source.map((value) => quantizePixel(value, "phz")),
        [0, 0, 127, 128, 255, 255, 255, 255]
    );
});

test("enumerates catalog versions and archive extensions / 枚举目录版本及 archive 扩展名", () => {
    const cases = buildFormatValidationCases([
        {
            name: "pwma",
            profile_id: "anycubic-pwma",
            default_extension: "pwma",
            extensions: ["pwma"],
            capabilities: { can_export: true },
            specifics: { supported_versions: [515, 516] }
        },
        {
            name: "anycubic-zip",
            profile_id: "zip",
            default_extension: "pm7",
            extensions: ["pm7", "pwsz"],
            is_zip: true,
            capabilities: { can_export: true },
            specifics: {}
        },
        { name: "cxdlp-v1", profile_id: "v1", extensions: ["cxdlp"], capabilities: { can_export: false } }
    ]);
    assert.deepEqual(
        cases.map(({ format, version, extension }) => [format, version, extension]),
        [
            ["pwma", 515, "pwma"],
            ["pwma", 516, "pwma"],
            ["anycubic-zip", null, "pm7"],
            ["anycubic-zip", null, "pwsz"]
        ]
    );
    assert.equal(new Set(cases.map((row) => row.key)).size, 4);
});

test("measures grayscale differences instead of hiding lossy conversion / 记录灰度差异而不掩盖有损转换", () => {
    assert.deepEqual(
        comparePixels(new Uint8Array([0, 1, 127, 128, 254, 255]), new Uint8Array([0, 0, 126, 129, 255, 255])),
        {
            pixels: 6,
            changedPixels: 4,
            maxAbsoluteError: 1,
            meanAbsoluteError: 4 / 6
        }
    );
    assert.throws(() => comparePixels(new Uint8Array(1), new Uint8Array(2)), /pixel count/i);
});
