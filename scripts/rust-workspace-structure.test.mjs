import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const rustRoot = join(repoRoot, "rust");
const rustManifestPath = join(rustRoot, "Cargo.toml");

const expectedCrates = new Map([
    ["slicer-converter-core", "rust/crates/core/Cargo.toml"],
    ["slicer-converter-wasm", "rust/crates/wasm/Cargo.toml"],
    ["slicer-converter-cli", "rust/crates/cli/Cargo.toml"]
]);

function toRepoPath(path) {
    return relative(repoRoot, path).replaceAll("\\", "/");
}

function readWorkspaceMetadata() {
    return JSON.parse(
        execFileSync("cargo", ["metadata", "--manifest-path", rustManifestPath, "--format-version", "1", "--no-deps"], {
            cwd: repoRoot,
            encoding: "utf8"
        })
    );
}

function collectTextFiles(rootPath, results = []) {
    const ignoredNames = new Set([
        ".git",
        "node_modules",
        "target",
        "dist",
        "lib",
        "test-results",
        "playwright-report",
        "coverage"
    ]);
    const entry = statSync(rootPath);

    if (entry.isDirectory()) {
        for (const name of readdirSync(rootPath)) {
            if (!ignoredNames.has(name)) {
                collectTextFiles(join(rootPath, name), results);
            }
        }

        return results;
    }

    const extension = extname(rootPath);
    if ([".json", ".md", ".mjs", ".ts", ".tsx", ".toml", ".ps1"].includes(extension)) {
        results.push(rootPath);
    }

    return results;
}

test("keeps Rust Cargo workspace split into core, wasm, and cli crates / 保持 Rust Cargo workspace 拆分为 core、wasm、cli", () => {
    const rootManifest = readFileSync(rustManifestPath, "utf8");
    const metadata = readWorkspaceMetadata();
    const packagesByName = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]));

    assert.match(rootManifest, /^\[workspace\]/m);
    assert.doesNotMatch(rootManifest, /^\[package\]/m);
    assert.equal(packagesByName.has("slicer-converter"), false);

    for (const [packageName, manifestPath] of expectedCrates) {
        const pkg = packagesByName.get(packageName);

        assert.ok(pkg, `Missing workspace package ${packageName}`);
        assert.equal(toRepoPath(pkg.manifest_path), manifestPath);
        assert.ok(existsSync(join(repoRoot, manifestPath)));
    }
});

test("keeps repository scripts on workspace crate paths / 保持仓库脚本指向 workspace crate 路径", () => {
    const packageJson = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
    const wasmPackageJson = JSON.parse(readFileSync(join(repoRoot, "packages/slicer-wasm/package.json"), "utf8"));

    assert.match(packageJson.scripts["test:rust"], /--workspace\b/);
    assert.match(packageJson.scripts["test:wasm"], /\brust\/crates\/wasm\b/);
    assert.match(packageJson.scripts["benchmark:rust"], /-p slicer-converter-core\b/);
    assert.match(packageJson.scripts.fixtures, /-p slicer-converter-core\b/);
    assert.match(wasmPackageJson.scripts.build, /\.\.\/\.\.\/rust\/crates\/wasm\b/);
});

test("keeps workspace-aware files free of legacy Rust root crate paths / 保持 workspace 相关文件不引用旧 Rust 根 crate 路径", () => {
    const filesToScan = [
        join(repoRoot, "package.json"),
        join(repoRoot, "README.md"),
        ...collectTextFiles(join(repoRoot, "scripts")),
        ...collectTextFiles(join(repoRoot, "apps")),
        ...collectTextFiles(join(repoRoot, "packages"))
    ];
    const legacyPatterns = [/rust\/src\b/, /rust\\src\b/, /rust\/tests\b/, /rust\\tests\b/];
    const offenders = [];

    for (const file of filesToScan) {
        const text = readFileSync(file, "utf8");

        for (const pattern of legacyPatterns) {
            if (pattern.test(text)) {
                offenders.push(`${toRepoPath(file)} matches ${pattern}`);
            }
        }
    }

    assert.deepEqual(offenders, []);
});
