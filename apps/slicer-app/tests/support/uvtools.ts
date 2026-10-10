import { execFile } from "node:child_process";
import { access } from "node:fs/promises";
import { promisify } from "node:util";

import { expect } from "@playwright/test";

const execFileAsync = promisify(execFile);
const UVTOOLS_FAILURE_PATTERN =
    /Unhandled exception|EndOfStreamException|Corrupted RLE|FileLoadException|Not a valid/i;
const UVTOOLS_WARNING_PATTERN = /incorrect image ratio|UsingSquarePixels:\s*False/i;
const UVTOOLS_SUCCESS_PATTERN = /Opening file .*Done/i;
const UVTOOLS_CAN_DECODE_PATTERN = /CanDecode:\s*True/i;

interface UvtoolsRunResult {
    output: string;
    exitCode: number;
}

export interface UvtoolsCompatibilitySummary {
    exitCode: number;
    opened: boolean;
    issue: string | null;
    warnings: string[];
    canDecode: boolean | null;
    canEncode: boolean | null;
    usingSquarePixels: boolean | null;
    fileExtension: string | null;
    resolutionX: number | null;
    resolutionY: number | null;
    displayWidth: number | null;
    displayHeight: number | null;
    layerCount: number | null;
    machineName: string | null;
}

let uvtoolsCommandPromise: Promise<string> | undefined;

export async function expectUvtoolsCanOpen(filePath: string): Promise<UvtoolsCompatibilitySummary> {
    const uvtoolsCommand = await resolveUvtoolsCommand();
    const result = await runUvtoolsPrintProperties(uvtoolsCommand, filePath);
    const summary = parseUvtoolsCompatibilitySummary(result);
    const excerpt = excerptUvtoolsOutput(result.output);

    expect(summary.issue, excerpt).toBeNull();
    return summary;
}

export function getUvtoolsCompatibilityIssue(result: UvtoolsRunResult): string | null {
    if (!UVTOOLS_SUCCESS_PATTERN.test(result.output)) {
        return "UVTools did not report a successful open";
    }

    const failure = result.output.match(UVTOOLS_FAILURE_PATTERN);
    if (failure) {
        return `UVTools compatibility issue: ${failure[0]}`;
    }

    if (
        result.exitCode !== 0 &&
        !UVTOOLS_CAN_DECODE_PATTERN.test(result.output) &&
        getUvtoolsCompatibilityWarnings(result).length === 0
    ) {
        return `UVTools exited with exit code ${result.exitCode}`;
    }

    return null;
}

export function getUvtoolsCompatibilityWarnings(result: UvtoolsRunResult): string[] {
    return Array.from(result.output.matchAll(new RegExp(UVTOOLS_WARNING_PATTERN, "gi"))).map((match) => match[0]);
}

export function parseUvtoolsCompatibilitySummary(result: UvtoolsRunResult): UvtoolsCompatibilitySummary {
    return {
        exitCode: result.exitCode,
        opened: UVTOOLS_SUCCESS_PATTERN.test(result.output),
        issue: getUvtoolsCompatibilityIssue(result),
        warnings: getUvtoolsCompatibilityWarnings(result),
        canDecode: readBooleanProperty(result.output, "CanDecode"),
        canEncode: readBooleanProperty(result.output, "CanEncode"),
        usingSquarePixels: readBooleanProperty(result.output, "UsingSquarePixels"),
        fileExtension: readStringProperty(result.output, "FileExtension"),
        resolutionX: readNumberProperty(result.output, "ResolutionX"),
        resolutionY: readNumberProperty(result.output, "ResolutionY"),
        displayWidth: readNumberProperty(result.output, "DisplayWidth"),
        displayHeight: readNumberProperty(result.output, "DisplayHeight"),
        layerCount: readNumberProperty(result.output, "LayerCount"),
        machineName: readStringProperty(result.output, "MachineName")
    };
}

export function resolveUvtoolsCommand(): Promise<string> {
    uvtoolsCommandPromise ??= findUvtoolsCommand();
    return uvtoolsCommandPromise;
}

async function findUvtoolsCommand(): Promise<string> {
    const candidates = [
        process.env.UVTOOLS_CMD?.trim(),
        process.platform === "win32" ? "C:\\Program Files\\UVtools\\UVtoolsCmd.exe" : undefined
    ].filter((candidate): candidate is string => Boolean(candidate));

    for (const candidate of candidates) {
        try {
            await access(candidate);
            return candidate;
        } catch {
            // Try the next configured/default location.
        }
    }

    throw new Error(
        [
            "UVTools compatibility check requires UVtoolsCmd.",
            "Set UVTOOLS_CMD to the UVtoolsCmd executable path, for example:",
            'UVTOOLS_CMD="C:\\Program Files\\UVtools\\UVtoolsCmd.exe"'
        ].join(" ")
    );
}

async function runUvtoolsPrintProperties(uvtoolsCommand: string, filePath: string): Promise<UvtoolsRunResult> {
    try {
        const result = await execFileAsync(uvtoolsCommand, ["--no-progress", "print-properties", filePath], {
            timeout: 120_000,
            windowsHide: true,
            maxBuffer: 8 * 1024 * 1024
        });
        return {
            output: joinProcessOutput(result.stdout, result.stderr),
            exitCode: 0
        };
    } catch (error) {
        return outputFromExecFileError(error);
    }
}

function outputFromExecFileError(error: unknown): UvtoolsRunResult {
    if (typeof error === "object" && error !== null) {
        const maybeOutput = error as { stdout?: unknown; stderr?: unknown; message?: unknown; code?: unknown };
        return {
            output: joinProcessOutput(maybeOutput.stdout, maybeOutput.stderr, maybeOutput.message),
            exitCode: typeof maybeOutput.code === "number" ? maybeOutput.code : 1
        };
    }
    return {
        output: String(error),
        exitCode: 1
    };
}

function joinProcessOutput(...parts: unknown[]): string {
    return parts
        .filter((part) => part !== undefined && part !== null)
        .map((part) => (Buffer.isBuffer(part) ? part.toString("utf8") : String(part)))
        .join("\n");
}

function excerptUvtoolsOutput(output: string): string {
    return output.split(/\r?\n/u).slice(0, 80).join("\n");
}

function readStringProperty(output: string, propertyName: string): string | null {
    const match = new RegExp(`^${escapeRegExp(propertyName)}:\\s*(.*)$`, "imu").exec(output);
    return match?.[1]?.trim() ?? null;
}

function readBooleanProperty(output: string, propertyName: string): boolean | null {
    const value = readStringProperty(output, propertyName);
    if (value === null) {
        return null;
    }
    if (/^true$/iu.test(value)) {
        return true;
    }
    if (/^false$/iu.test(value)) {
        return false;
    }
    return null;
}

function readNumberProperty(output: string, propertyName: string): number | null {
    const value = readStringProperty(output, propertyName);
    if (value === null) {
        return null;
    }
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
}

function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}
