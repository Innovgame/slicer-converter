import { describe, expect, it } from "vitest";

import {
    buildFilePickerAccept,
    type FilePickerEnvironment
} from "../../../../src/features/converter/ui/file-picker-accept";

const catalogExtensions = [".ctb", ".v1.ctb", ".gktwo.ctb", ".encrypted.ctb", ".goo", ".v1.cxdlp", ".zip"];

function environment(
    input: Partial<FilePickerEnvironment> & Pick<FilePickerEnvironment, "userAgent">
): FilePickerEnvironment {
    return {
        platform: "",
        maxTouchPoints: 0,
        ...input
    };
}

describe("buildFilePickerAccept / 文件选择器 accept", () => {
    it("lets Apple file dialogs select unregistered slicer extensions / 让 Apple 文件框可选中未注册的切片扩展名", () => {
        const cases = [
            environment({
                userAgent:
                    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
                platform: "iPhone",
                maxTouchPoints: 5
            }),
            environment({
                userAgent:
                    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/119.0.6045.109 Mobile/15E148 Safari/604.1",
                platform: "iPhone",
                maxTouchPoints: 5
            }),
            environment({
                userAgent:
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
                platform: "MacIntel",
                maxTouchPoints: 5
            }),
            environment({
                userAgent:
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15",
                platform: "MacIntel"
            }),
            environment({
                userAgent:
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                platform: "MacIntel"
            }),
            environment({
                userAgent:
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
                userAgentDataPlatform: "macOS"
            })
        ];

        for (const picker of cases) {
            expect(buildFilePickerAccept(catalogExtensions, picker)).toBe("application/octet-stream");
        }
    });

    it("keeps a single-segment extension filter on other platforms / 其它平台保留单段扩展名过滤", () => {
        const windows = environment({
            userAgent:
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            platform: "Win32",
            userAgentDataPlatform: "Windows"
        });
        const linux = environment({
            userAgent: "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
            platform: "Linux x86_64"
        });

        expect(buildFilePickerAccept(catalogExtensions, windows)).toBe(".ctb,.cxdlp,.goo,.zip");
        expect(buildFilePickerAccept(catalogExtensions, linux)).toBe(".ctb,.cxdlp,.goo,.zip");
        expect(buildFilePickerAccept(["ctb", ".V1.CTB", " goo ", ".v1.cxdlp"], windows)).toBe(".ctb,.cxdlp,.goo");
    });

    it("allows every file before the format catalog is ready / 格式目录未就绪时允许选择任意文件", () => {
        const windows = environment({
            userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
            platform: "Win32"
        });

        expect(buildFilePickerAccept([], windows)).toBeUndefined();
    });
});
