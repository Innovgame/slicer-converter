import type { FileMetadata } from "../../../types.js";
import type { ZipEntryInfo } from "../directory.js";
import { readZipEntryText, type ZipSource } from "../source.js";
import {
    findZipEntry,
    getZipFileName,
    parseIniKeyValues,
    readBoolean,
    readNumber,
    readString,
    type ZipFormatParser
} from "./shared.js";

export const sl1ZipFormat: ZipFormatParser = {
    format: "sl1",
    readLayerIndex,
    parseMetadata
};

function readLayerIndex(name: string): number | undefined {
    const fileName = getZipFileName(name).toLowerCase();
    if (!fileName.endsWith(".png") || fileName.includes("thumbnail")) {
        return undefined;
    }

    const stem = fileName.slice(0, -".png".length);
    const suffix = stem.slice(-5);
    return /^\d{5}$/u.test(suffix) ? Number(suffix) : undefined;
}

async function parseMetadata(
    source: ZipSource,
    entries: readonly ZipEntryInfo[],
    layerCount: number
): Promise<FileMetadata> {
    const configEntry = findZipEntry(entries, "config.ini");
    if (!configEntry) {
        throw new Error("config.ini not found");
    }

    const config = parseIniKeyValues(await readZipEntryText(source, configEntry));
    const metadata: FileMetadata = {
        layer_height_mm: readNumber(config, "layer_height") ?? 0.05,
        layer_count: layerCount,
        bottom_layer_count: readNumber(config, "num_fade") ?? 0,
        transition_layer_count: readNumber(config, "num_slow") ?? 0,
        exposure_time: readNumber(config, "exp_time") ?? 2,
        bottom_exposure_time: readNumber(config, "exp_time_first") ?? 30,
        print_time_sec: readNumber(config, "print_time") ?? 0,
        volume_ml: readNumber(config, "used_material") ?? 0,
        machine_name: readString(config, "printer_model")
    };

    const printerEntry = findZipEntry(entries, "prusaslicer.ini");
    if (printerEntry) {
        const printerConfig = parseIniKeyValues(await readZipEntryText(source, printerEntry));
        metadata.display_width_mm = readNumber(printerConfig, "display_width");
        metadata.display_height_mm = readNumber(printerConfig, "display_height");
        metadata.resolution_x = readNumber(printerConfig, "display_pixels_x");
        metadata.resolution_y = readNumber(printerConfig, "display_pixels_y");
        metadata.machine_z_mm = readNumber(printerConfig, "max_print_height");
        metadata.mirror_x = readBoolean(printerConfig, "display_mirror_x");
        metadata.mirror_y = readBoolean(printerConfig, "display_mirror_y");
    }

    return metadata;
}
