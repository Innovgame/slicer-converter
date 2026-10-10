import type { FileMetadata, Thumbnail } from "../../../types.js";
import type { ZipEntryInfo } from "../directory.js";
import { readZipEntryBytes, type ZipSource } from "../source.js";

/**
 * @internal
 * @zh ZIP archive 中单个格式的 browser-side 解析行为。
 * @en Browser-side parsing behavior for one ZIP archive format.
 */
export interface ZipFormatParser {
    readonly format: "anycubic-zip" | "chitubox-zip" | "sl1" | "vdt";
    readLayerIndex(name: string): number | undefined;
    parseMetadata(source: ZipSource, entries: readonly ZipEntryInfo[], layerCount: number): Promise<FileMetadata>;
}

export function getZipFileName(name: string): string {
    return name.split("/").at(-1) ?? name;
}

export function findZipEntry(entries: readonly ZipEntryInfo[], name: string): ZipEntryInfo | undefined {
    return entries.find((entry) => entry.name === name);
}

export function parseIniKeyValues(text: string): Record<string, string> {
    const values: Record<string, string> = {};

    for (const rawLine of text.split(/\r?\n/u)) {
        const line = rawLine.trim();
        if (!line || line.startsWith("#") || line.startsWith(";")) {
            continue;
        }

        const separator = line.indexOf("=");
        if (separator === -1) {
            continue;
        }

        const key = line.slice(0, separator).trim();
        const value = line.slice(separator + 1).trim();
        values[key] = value;
    }

    return values;
}

export function readRecord(value: unknown): Record<string, unknown> {
    return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

export function readRecordArray(value: unknown): Record<string, unknown>[] {
    return Array.isArray(value) ? value.map(readRecord) : [];
}

export function readNumber(record: Record<string, unknown> | Record<string, string>, key: string): number | undefined {
    const value = record[key];
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : undefined;
    }

    if (typeof value === "string" && value.length > 0) {
        const parsed = Number(value);
        return Number.isFinite(parsed) ? parsed : undefined;
    }

    return undefined;
}

export function readNumberWithAliases(
    record: Record<string, unknown> | Record<string, string>,
    keys: string[]
): number | undefined {
    for (const key of keys) {
        const value = readNumber(record, key);
        if (value !== undefined) {
            return value;
        }
    }

    return undefined;
}

export function readRoundedNonNegativeNumber(
    record: Record<string, unknown> | Record<string, string>,
    key: string
): number | undefined {
    const value = readNumber(record, key);
    return value !== undefined && value >= 0 ? Math.round(value) : undefined;
}

export function readString(record: Record<string, unknown> | Record<string, string>, key: string): string | undefined {
    const value = record[key];
    return typeof value === "string" && value.length > 0 ? value : undefined;
}

export function readBoolean(
    record: Record<string, unknown> | Record<string, string>,
    key: string
): boolean | undefined {
    const value = record[key];
    if (typeof value === "boolean") {
        return value;
    }

    if (typeof value === "string") {
        const normalized = value.trim().toLowerCase();
        if (normalized === "1" || normalized === "true") {
            return true;
        }
        if (normalized === "0" || normalized === "false") {
            return false;
        }
    }

    return undefined;
}

export async function tryReadPngThumbnail(source: ZipSource, entry: ZipEntryInfo): Promise<Thumbnail | undefined> {
    try {
        return await decodePngThumbnail(await readZipEntryBytes(source, entry));
    } catch {
        return undefined;
    }
}

async function decodePngThumbnail(pngBytes: Uint8Array): Promise<Thumbnail> {
    if (typeof createImageBitmap !== "function" || typeof document === "undefined") {
        throw new Error("Browser PNG thumbnail decoding is unavailable");
    }

    const bitmap = await createImageBitmap(new Blob([toArrayBufferBlobPart(pngBytes)], { type: "image/png" }));
    try {
        const canvas = document.createElement("canvas");
        canvas.width = bitmap.width;
        canvas.height = bitmap.height;
        const context = canvas.getContext("2d");
        if (!context) {
            throw new Error("Canvas 2D context is unavailable");
        }

        context.drawImage(bitmap, 0, 0);
        const imageData = context.getImageData(0, 0, bitmap.width, bitmap.height);
        return {
            width: bitmap.width,
            height: bitmap.height,
            pixels_rgba: new Uint8Array(imageData.data)
        };
    } finally {
        bitmap.close?.();
    }
}

function toArrayBufferBlobPart(data: Uint8Array): ArrayBuffer {
    const buffer = new ArrayBuffer(data.byteLength);
    new Uint8Array(buffer).set(data);
    return buffer;
}
