import { describe, expect, it, vi } from "vitest";

import { resolveBuildCommit } from "../../../build/build-metadata";

const sha = "1234567890abcdef1234567890abcdef12345678";

describe("build commit metadata / 构建提交元数据", () => {
    it("uses an explicit validated build commit before Git / 显式构建提交校验后优先于 Git", () => {
        const readGitCommit = vi.fn(() => sha);
        expect(resolveBuildCommit({ env: { VITE_BUILD_COMMIT: "ABCDEF123456" }, readGitCommit })).toBe("abcdef12");
        expect(readGitCommit).not.toHaveBeenCalled();
    });

    it("uses the checked-out commit before the CI trigger commit / 检出的提交优先于 CI 触发提交", () => {
        expect(resolveBuildCommit({ env: { GITHUB_SHA: "abcdef1234567890" }, readGitCommit: () => sha })).toBe(
            "12345678"
        );
    });

    it("uses CI metadata when Git is unavailable / Git 不可用时使用 CI 元数据", () => {
        expect(
            resolveBuildCommit({
                env: { GITHUB_SHA: sha },
                readGitCommit: () => {
                    throw new Error("no Git");
                }
            })
        ).toBe("12345678");
    });

    it("omits commit metadata in source archives / 源码压缩包缺少提交信息时省略", () => {
        expect(
            resolveBuildCommit({
                env: {},
                readGitCommit: () => {
                    throw new Error("no Git");
                }
            })
        ).toBe("");
    });

    it.each(["invalid", "1234567", "abcdef12-dirty"])(
        "rejects an invalid explicit commit %s / 拒绝无效显式提交",
        (value) => {
            expect(() => resolveBuildCommit({ env: { VITE_BUILD_COMMIT: value }, readGitCommit: () => sha })).toThrow(
                /VITE_BUILD_COMMIT/
            );
        }
    );
});
