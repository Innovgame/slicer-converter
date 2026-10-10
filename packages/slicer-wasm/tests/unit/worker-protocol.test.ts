import { readFile } from "node:fs/promises";

import { describe, expect, it } from "vitest";

import {
    SLICER_WORKER_REQUEST_TYPES,
    SLICER_WORKER_RESPONSE_TYPES,
    type SlicerWorkerRequest,
    type SlicerWorkerResponse
} from "../../src/browser/worker/protocol.js";

describe("slicer worker protocol / slicer worker 协议", () => {
    it("keeps request and response sentinels in sync with literal unions / request 与 response sentinels 同步 literal unions", async () => {
        const source = await readFile("src/browser/worker/protocol.ts", "utf8");

        expect([...SLICER_WORKER_REQUEST_TYPES]).toEqual(extractUnionMemberTypeLiterals(source, "SlicerWorkerRequest"));
        expect([...SLICER_WORKER_RESPONSE_TYPES]).toEqual(
            extractUnionMemberTypeLiterals(source, "SlicerWorkerResponse")
        );
    });

    it("keeps package protocol free of app names / package protocol 不包含 app names", async () => {
        const source = await readFile("src/browser/worker/protocol.ts", "utf8");

        expect(source).not.toContain("AppError");
        expect(source).not.toContain("export type WorkerRequest");
        expect(source).not.toContain("export type WorkerResponse");
        expect(source).not.toContain("apps/slicer-app");
    });

    it("preserves conversion preview and cancel message variants / 保留 conversion preview 和 cancel message variants", () => {
        expect(SLICER_WORKER_REQUEST_TYPES).toEqual([
            "decode-encode",
            "decode-encode-prepare",
            "decode-preview",
            "zip-decode-encode",
            "zip-decode-encode-prepare",
            "cancel"
        ]);
        expect(SLICER_WORKER_RESPONSE_TYPES).toEqual([
            "ready",
            "layer-done",
            "preview-done",
            "zip-layer-prepared",
            "error"
        ]);

        const request = { type: "cancel" } satisfies SlicerWorkerRequest;
        const response = { type: "ready" } satisfies SlicerWorkerResponse;
        expect(request.type).toBe("cancel");
        expect(response.type).toBe("ready");
    });
});

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
