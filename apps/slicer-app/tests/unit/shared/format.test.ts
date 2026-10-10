import { describe, expect, it } from "vitest";

import { formatBytes, formatDuration } from "../../../src/shared/lib/format";

describe("format helpers / 格式化工具", () => {
    it("formats durations / 格式化 durations", () => {
        expect(formatDuration(12.34)).toBe("12.3ms");
        expect(formatDuration(1250)).toBe("1.25s");
    });

    it("formats bytes for kb and mb / 格式化 字节 for kb 并且 mb", () => {
        expect(formatBytes(900)).toBe("900 B");
        expect(formatBytes(1536)).toBe("1.5 KB");
        expect(formatBytes(2 * 1048576)).toBe("2.0 MB");
    });
});
