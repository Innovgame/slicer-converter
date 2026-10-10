import { describe, expect, it } from "vitest";
import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";

describe("worker protocol type boundary / worker 协议类型边界", () => {
    it("keeps worker protocol types owned by slicer-wasm / 保持 worker protocol 类型由 slicer-wasm 拥有", () => {
        expect(existsSync(path.join(process.cwd(), "src/worker-protocol.ts"))).toBe(false);
    });

    it("keeps worker messaging types out of the generic app types barrel / 保持 worker messaging types out of the 通用 app types barrel", async () => {
        const typeBarrelPath = path.join(process.cwd(), "src/types.ts");
        const protocolSource = await readFile(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/protocol.ts"),
            "utf8"
        );
        const workerTypeNames = ["SlicerWorkerRequest", "SlicerWorkerResponse", "SlicerWorkerErrorMessage", "SlicerWorkerError"];

        expect(existsSync(typeBarrelPath)).toBe(false);
        for (const typeName of workerTypeNames) {
            expect(protocolSource).toContain(typeName);
        }
    });

    it("keeps output pipeline types out of the worker message protocol / output pipeline types 不放在 worker message protocol 中", async () => {
        const protocolSource = await readFile(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/protocol.ts"),
            "utf8"
        );

        expect(protocolSource).not.toContain("OrderedLayerQueue");
        expect(protocolSource).not.toContain("ZipAssemblyTimings");
    });

    it("exposes runtime worker protocol sentinels from the package / 从 package 暴露 runtime worker protocol sentinels", async () => {
        const protocol = await import("@slicer-converter/wasm");

        expect(protocol.SLICER_WORKER_REQUEST_TYPES).toContain("decode-encode");
        expect(protocol.SLICER_WORKER_REQUEST_TYPES).toContain("decode-preview");
        expect(protocol.SLICER_WORKER_REQUEST_TYPES).not.toContain("zip-decode-preview");
        expect(protocol.SLICER_WORKER_REQUEST_TYPES).toContain("cancel");
        expect(protocol.SLICER_WORKER_RESPONSE_TYPES).toContain("layer-done");
        expect(protocol.SLICER_WORKER_RESPONSE_TYPES).toContain("error");
    });

    it("keeps protocol sentinels in sync with message type literals / protocol sentinels 与 message type literals 保持同步", async () => {
        const protocol = await import("@slicer-converter/wasm");
        const protocolSource = await readFile(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/protocol.ts"),
            "utf8"
        );

        expect([...protocol.SLICER_WORKER_REQUEST_TYPES]).toEqual(
            extractUnionMemberTypeLiterals(protocolSource, "SlicerWorkerRequest")
        );
        expect([...protocol.SLICER_WORKER_RESPONSE_TYPES]).toEqual(
            extractUnionMemberTypeLiterals(protocolSource, "SlicerWorkerResponse")
        );
    });

    it("does not keep legacy ZIP timing aliases / 不 keep legacy ZIP timing aliases", async () => {
        const protocolSource = await readFile(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/protocol.ts"),
            "utf8"
        );

        expect(protocolSource).not.toContain("finalizeWallMs");
    });

    it("does not leak raw WASM codec names into the app worker protocol / 不把 raw WASM codec 名称泄漏到 app worker protocol", async () => {
        const protocolSource = await readFile(
            path.join(process.cwd(), "../../packages/slicer-wasm/src/browser/worker/protocol.ts"),
            "utf8"
        );

        expect(protocolSource).not.toContain("decode_layer_preview");
    });

    it("does not keep an unconnected worker transcode stats sidecar / 不保留 unconnected worker transcode stats sidecar", () => {
        expect(existsSync(path.join(process.cwd(), "src/worker/transcode-stats.ts"))).toBe(false);
        expect(existsSync(path.join(process.cwd(), "src/worker/transcode-stats-summary.ts"))).toBe(false);
    });

    it("keeps conversion worker scheduling contracts out of the conversion controller / 保持 转换 worker scheduling contracts out of the 转换 controller", () => {
        const controllerPath = path.normalize(
            path.join(process.cwd(), "src/features/conversion/model/conversion-controller.ts")
        );
        const offenders = listSourceFiles(path.join(process.cwd(), "src"))
            .filter((file) => path.normalize(file) !== controllerPath)
            .filter((file) => {
                const text = readFileSync(file, "utf8");
                return text.includes('from "../conversion-controller"') && text.includes("WorkerDispatchPayload");
            })
            .map((file) => path.relative(process.cwd(), file).replaceAll("\\", "/"));

        expect(offenders).toEqual([]);
    });
});

function listSourceFiles(root: string): string[] {
    const result: string[] = [];
    collectSourceFiles(root, result);
    return result;
}

function collectSourceFiles(dir: string, result: string[]): void {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const fullPath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            collectSourceFiles(fullPath, result);
            continue;
        }
        if (/\.(ts|tsx)$/.test(entry.name)) {
            result.push(fullPath);
        }
    }
}

function extractUnionMemberTypeLiterals(source: string, unionName: string): string[] {
    const unionBody = source.match(new RegExp(`export type ${unionName} =([\\s\\S]*?);`))?.[1];
    if (!unionBody) {
        throw new Error(`Missing ${unionName} union`);
    }

    return unionBody
        .split("|")
        .map((memberName) => memberName.trim())
        .filter(Boolean)
        .map((memberName) => {
            const typeLiteral = source.match(
                new RegExp(`export interface ${memberName}[\\s\\S]*?type: "([^"]+)";`)
            )?.[1];
            if (!typeLiteral) {
                throw new Error(`Missing type literal for ${memberName}`);
            }
            return typeLiteral;
        });
}
