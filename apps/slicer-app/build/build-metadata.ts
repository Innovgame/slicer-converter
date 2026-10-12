import { execFileSync } from "node:child_process";

/**
 * @internal
 * @zh 构建提交解析输入；Git 读取可替换以验证无仓库和 CI 构建。
 * @en Build commit resolution inputs; Git reading can be replaced to verify archive and CI builds.
 */
export interface BuildCommitOptions {
    projectRoot?: string;
    env?: Readonly<Record<string, string | undefined>>;
    readGitCommit?: () => string;
}

/**
 * @internal
 * @zh 解析构建时的提交标识，不在浏览器运行。
 * @en Resolves the build commit identifier; runs outside the browser.
 */
export function resolveBuildCommit(options: BuildCommitOptions): string {
    const env = options.env ?? process.env;
    const explicit = env.VITE_BUILD_COMMIT?.trim();
    if (explicit) {
        const commit = normalizeCommit(explicit);
        if (!commit) throw new Error("VITE_BUILD_COMMIT must contain 8 to 64 hexadecimal characters");
        return commit;
    }
    const readGitCommit =
        options.readGitCommit ??
        (() =>
            execFileSync("git", ["rev-parse", "HEAD"], {
                cwd: options.projectRoot,
                encoding: "utf8",
                stdio: ["ignore", "pipe", "ignore"]
            }));
    let gitCommit = "";
    try {
        gitCommit = normalizeCommit(readGitCommit());
    } catch {
        /* Source archives can omit Git metadata. */
    }
    return gitCommit || normalizeCommit(env.GITHUB_SHA ?? "");
}

function normalizeCommit(value: string): string {
    const commit = value.trim();
    return /^[\da-f]{8,64}$/i.test(commit) ? commit.toLowerCase().slice(0, 8) : "";
}
