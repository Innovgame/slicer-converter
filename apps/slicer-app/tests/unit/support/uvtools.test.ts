import { describe, expect, it } from "vitest";

import {
    getUvtoolsCompatibilityIssue,
    getUvtoolsCompatibilityWarnings,
    parseUvtoolsCompatibilitySummary
} from "../../support/uvtools";
import { ANYCUBIC_ARCHIVE_EXTENSIONS, ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS } from "../../support/anycubic-targets";

describe("UVTools compatibility output / UVTools 兼容输出", () => {
    it("reports incorrect image ratio warnings without failing compatibility / 报告图像比例 warning 但不失败", () => {
        const output = [
            "Opening file model.pw0: Done in 4.95s",
            "It looks like this file was sliced with an incorrect image ratio.",
            "Resolution: {Width=8520, Height=4320}",
            "Display: {Width=55.44, Height=98.637}",
            "Ratio: 1:111"
        ].join("\n");

        const result = { output, exitCode: 1 };

        expect(getUvtoolsCompatibilityIssue(result)).toBeNull();
        expect(getUvtoolsCompatibilityWarnings(result)).toContain("incorrect image ratio");
    });

    it("rejects non-zero UVTools exits / 拒绝 UVTools 非零退出", () => {
        const output = "Opening file model.ctb: Done in 1.00s";

        expect(getUvtoolsCompatibilityIssue({ output, exitCode: 1 })).toContain("exit code 1");
    });

    it("accepts print-properties files that opened and decode despite exit code 1 / 接受已打开且可解码但 exit code 为 1 的 print-properties 输出", () => {
        const output = [
            "Opening file model.photons: Done in 0.13s",
            "ResolutionX: 1440",
            "ResolutionY: 2560",
            "IssueManager: UVtools.Core.Managers.IssueManager",
            "CanDecode: True",
            "CanEncode: True",
            "Total properties: 276"
        ].join("\n");

        expect(getUvtoolsCompatibilityIssue({ output, exitCode: 1 })).toBeNull();
    });

    it("accepts successful UVTools output / 接受 UVTools 成功输出", () => {
        const output = "Opening file model.ctb: Done in 1.00s";

        expect(getUvtoolsCompatibilityIssue({ output, exitCode: 0 })).toBeNull();
    });

    it("parses a compact compatibility summary from print-properties output / 从 print-properties 输出解析兼容摘要", () => {
        const output = [
            "Opening file model.pm5s: Done in 5.01s",
            "FileExtension: .pm5s",
            "ResolutionX: 8520",
            "ResolutionY: 4320",
            "DisplayWidth: 153.36",
            "DisplayHeight: 77.76",
            "LayerCount: 1380",
            "MachineName: ELEGOO Mars 5 Ultra(1)",
            "UsingSquarePixels: True",
            "CanDecode: True",
            "CanEncode: True",
            "Total properties: 287"
        ].join("\n");

        expect(parseUvtoolsCompatibilitySummary({ output, exitCode: 1 })).toEqual({
            canDecode: true,
            canEncode: true,
            displayHeight: 77.76,
            displayWidth: 153.36,
            exitCode: 1,
            fileExtension: ".pm5s",
            issue: null,
            layerCount: 1380,
            machineName: "ELEGOO Mars 5 Ultra(1)",
            opened: true,
            resolutionX: 8520,
            resolutionY: 4320,
            usingSquarePixels: true,
            warnings: []
        });
    });

    it("keeps unsupported newer Anycubic targets out of strict UVTools smoke / 不把新版 Anycubic 目标放入 UVTools 严格 smoke", () => {
        expect(ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS).not.toContain("pm4n");
        for (const extension of ANYCUBIC_ARCHIVE_EXTENSIONS) {
            expect(ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS).not.toContain(extension);
        }
    });
});
