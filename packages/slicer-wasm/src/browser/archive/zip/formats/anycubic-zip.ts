import type { FileMetadata, Thumbnail } from "../../../types.js";
import type { ZipEntryInfo } from "../directory.js";
import { readZipEntryText, type ZipSource } from "../source.js";
import {
    findZipEntry,
    readNumber,
    readNumberWithAliases,
    readRecord,
    readRecordArray,
    readRoundedNonNegativeNumber,
    readString,
    tryReadPngThumbnail,
    type ZipFormatParser
} from "./shared.js";

export const anycubicZipFormat: ZipFormatParser = {
    format: "anycubic-zip",
    readLayerIndex,
    parseMetadata
};

function readLayerIndex(name: string): number | undefined {
    const lowerName = name.toLowerCase();
    const prefix = "layer_images/layer_";
    if (!lowerName.startsWith(prefix)) {
        return undefined;
    }

    const fileName = lowerName.slice(prefix.length);
    const match = /^(\d+)\.(pw0img|pwszimg)$/u.exec(fileName);
    return match ? Number(match[1]) : undefined;
}

async function parseMetadata(
    source: ZipSource,
    entries: readonly ZipEntryInfo[],
    layerCount: number
): Promise<FileMetadata> {
    const settingsEntry = findZipEntry(entries, "anycubic_photon_resins.pwsp");
    if (!settingsEntry) {
        throw new Error("anycubic_photon_resins.pwsp not found");
    }

    const layersEntry = findZipEntry(entries, "layers_controller.conf");
    if (!layersEntry) {
        throw new Error("layers_controller.conf not found");
    }

    const settings = readRecord(JSON.parse(await readZipEntryText(source, settingsEntry)));
    const layers = readRecord(JSON.parse(await readZipEntryText(source, layersEntry)));
    const machine = readRecord(settings.machine_type);
    const layerRecords = readRecordArray(layers.paras);
    const firstLayer = layerRecords[0] ?? {};
    const printInfoEntry = findZipEntry(entries, "print_info.json");
    const printInfo = printInfoEntry
        ? readRecord(JSON.parse(await readZipEntryText(source, printInfoEntry)))
        : {};

    const metadata: FileMetadata = {
        machine_name: readString(machine, "name"),
        resolution_x: readNumber(machine, "res_x"),
        resolution_y: readNumber(machine, "res_y"),
        display_width_mm: readNumber(machine, "print_xsize"),
        display_height_mm: readNumber(machine, "print_ysize"),
        machine_z_mm: readNumber(machine, "print_zsize"),
        layer_height_mm: readNumber(firstLayer, "layer_thickness") ?? 0.05,
        layer_count: layerRecords.length || readNumber(layers, "count") || layerCount,
        anti_alias_level: Math.max(readNumber(machine, "raster_antialiasing") ?? 1, 1),
        exposure_time: readNumber(firstLayer, "exposure_time") ?? 2,
        bottom_exposure_time: readNumber(firstLayer, "exposure_time") ?? 2,
        lift_height: readNumber(firstLayer, "zup_height") ?? 5,
        lift_speed: readNumber(firstLayer, "zup_speed") ?? 60,
        print_time_sec: readRoundedNonNegativeNumber(printInfo, "print_time") ?? 0,
        volume_ml: readNumberWithAliases(printInfo, ["volume", "material_milliliters"]) ?? 0,
        weight_g: readNumberWithAliases(printInfo, ["weight", "material_grams"]) ?? 0,
        cost: readNumber(printInfo, "cost") ?? 0
    };

    const thumbnails = await readThumbnails(source, entries);
    if (thumbnails.length > 0) {
        metadata.thumbnails = thumbnails;
    }

    return metadata;
}

async function readThumbnails(source: ZipSource, entries: readonly ZipEntryInfo[]): Promise<Thumbnail[]> {
    const thumbnails: Thumbnail[] = [];
    for (const entry of entries) {
        const name = entry.name.toLowerCase();
        if (!name.startsWith("preview_images/") || !name.endsWith(".png")) {
            continue;
        }

        const thumbnail = await tryReadPngThumbnail(source, entry);
        if (thumbnail) {
            thumbnails.push(thumbnail);
        }
    }

    return thumbnails;
}
