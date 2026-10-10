import type { FileMetadata, Thumbnail } from "../../../types.js";
import type { ZipEntryInfo } from "../directory.js";
import { readZipEntryText, type ZipSource } from "../source.js";
import {
    findZipEntry,
    getZipFileName,
    readNumber,
    readNumberWithAliases,
    readRoundedNonNegativeNumber,
    readString,
    tryReadPngThumbnail,
    type ZipFormatParser
} from "./shared.js";

export const chituboxZipFormat: ZipFormatParser = {
    format: "chitubox-zip",
    readLayerIndex,
    parseMetadata
};

function readLayerIndex(name: string): number | undefined {
    const fileName = getZipFileName(name).toLowerCase();
    if (!fileName.endsWith(".png") || fileName.includes("preview")) {
        return undefined;
    }

    const stem = fileName.slice(0, -".png".length);
    if (!/^\d+$/u.test(stem)) {
        return undefined;
    }

    const layerNumber = Number(stem);
    return layerNumber > 0 ? layerNumber - 1 : undefined;
}

async function parseMetadata(
    source: ZipSource,
    entries: readonly ZipEntryInfo[],
    layerCount: number
): Promise<FileMetadata> {
    const gcodeEntry = findZipEntry(entries, "run.gcode");
    if (!gcodeEntry) {
        throw new Error("run.gcode not found");
    }

    const metadata = parseChituboxGcodeMetadata(await readZipEntryText(source, gcodeEntry), layerCount);
    const thumbnails = await readThumbnails(source, entries);
    if (thumbnails.length > 0) {
        metadata.thumbnails = thumbnails;
    }

    return metadata;
}

function parseChituboxGcodeMetadata(gcode: string, layerCount: number): FileMetadata {
    const values: Record<string, string> = {};
    for (const rawLine of gcode.split(/\r?\n/u)) {
        const line = rawLine.trim();
        if (!line.startsWith(";")) {
            continue;
        }
        const separator = line.indexOf(":");
        if (separator === -1) {
            continue;
        }

        const key = line.slice(1, separator).trim();
        const value = line.slice(separator + 1).trim();
        values[key] = value;
    }

    const metadata: FileMetadata = {
        machine_name: readString(values, "machineType") ?? "Default",
        layer_height_mm: readNumber(values, "layerHeight") ?? 0.05,
        layer_count: readNumber(values, "totalLayer") ?? layerCount,
        bottom_layer_count: readNumberWithAliases(values, ["bottomLayerCount", "bottomLayCount"]) ?? 4,
        anti_alias_level: readNumber(values, "antiAliasLevel") ?? 1,
        exposure_time: readNumber(values, "normalExposureTime") ?? 7,
        bottom_exposure_time: readNumberWithAliases(values, ["bottomLayerExposureTime", "bottomLayExposureTime"]) ?? 35,
        light_off_delay: readNumber(values, "lightOffTime") ?? 0,
        bottom_light_off_delay: readNumber(values, "bottomLightOffTime") ?? 0,
        light_pwm: readNumber(values, "PWMLight") ?? 255,
        bottom_light_pwm: readNumber(values, "bottomPWMLight") ?? 255,
        lift_height: readNumber(values, "normalLayerLiftHeight") ?? 5,
        lift_speed: readNumber(values, "normalLayerLiftSpeed") ?? 60,
        retract_speed: readNumber(values, "normalDropSpeed") ?? 150,
        bottom_lift_height: readNumber(values, "bottomLayerLiftHeight") ?? 5,
        bottom_lift_speed: readNumber(values, "bottomLayerLiftSpeed") ?? 60,
        print_time_sec: readRoundedNonNegativeNumber(values, "estimatedPrintTime") ?? 0,
        volume_ml: readNumber(values, "volume") ?? 0,
        weight_g: readNumber(values, "weight") ?? 0,
        cost: readNumber(values, "price") ?? 0
    };

    metadata.resolution_x = readNumber(values, "resolutionX");
    metadata.resolution_y = readNumber(values, "resolutionY");
    metadata.display_width_mm = readNumber(values, "machineX");
    metadata.display_height_mm = readNumber(values, "machineY");
    metadata.machine_z_mm = readNumber(values, "machineZ");
    metadata.mirror_x = readChituboxMirrorX(values);

    return metadata;
}

async function readThumbnails(source: ZipSource, entries: readonly ZipEntryInfo[]): Promise<Thumbnail[]> {
    const thumbnails: Thumbnail[] = [];
    for (const name of ["preview.png", "preview_cropping.png"]) {
        const entry = findZipEntry(entries, name);
        if (!entry) {
            continue;
        }

        const thumbnail = await tryReadPngThumbnail(source, entry);
        if (thumbnail) {
            thumbnails.push(thumbnail);
        }
    }

    return thumbnails;
}

function readChituboxMirrorX(record: Record<string, string>): boolean | undefined {
    const mirror = readNumber(record, "mirror");
    if (mirror !== undefined) {
        return mirror !== 0;
    }

    const projectType = readString(record, "projectType");
    return projectType ? projectType.toLowerCase() === "lcd_mirror" : undefined;
}
