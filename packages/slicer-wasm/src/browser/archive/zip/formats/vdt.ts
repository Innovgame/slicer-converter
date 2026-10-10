import type { FileMetadata } from "../../../types.js";
import type { ZipEntryInfo } from "../directory.js";
import { readZipEntryText, type ZipSource } from "../source.js";
import {
    findZipEntry,
    getZipFileName,
    readBoolean,
    readNumber,
    readRecord,
    readString,
    type ZipFormatParser
} from "./shared.js";

export const vdtZipFormat: ZipFormatParser = {
    format: "vdt",
    readLayerIndex,
    parseMetadata
};

function readLayerIndex(name: string): number | undefined {
    const fileName = getZipFileName(name).toLowerCase();
    if (!fileName.endsWith(".png") || fileName.includes("preview")) {
        return undefined;
    }

    const stem = fileName.slice(0, -".png".length);
    if (/^\d+$/u.test(stem)) {
        return Number(stem);
    }

    const legacyMatch = /^layer_(\d+)$/u.exec(stem);
    return legacyMatch ? Number(legacyMatch[1]) : undefined;
}

async function parseMetadata(
    source: ZipSource,
    entries: readonly ZipEntryInfo[],
    layerCount: number
): Promise<FileMetadata> {
    const manifestEntry = findZipEntry(entries, "manifest.json");
    if (!manifestEntry) {
        throw new Error("manifest.json not found");
    }

    const manifest = JSON.parse(await readZipEntryText(source, manifestEntry)) as Record<string, unknown>;
    const machine = readRecord(manifest.machine);
    const advanced = readRecord(manifest.advanced_parameters);
    const print = readRecord(manifest.print);

    return {
        resolution_x: readNumber(machine, "resolution_x"),
        resolution_y: readNumber(machine, "resolution_y"),
        display_width_mm: readNumber(machine, "lcd_width"),
        display_height_mm: readNumber(machine, "lcd_height"),
        machine_z_mm: readNumber(machine, "z_height"),
        machine_name: readString(machine, "name"),
        mirror_x: readBoolean(machine, "x_mirror"),
        mirror_y: readBoolean(machine, "y_mirror"),
        layer_height_mm: readNumber(print, "layer_thickness") ?? 0.05,
        layer_count: layerCount,
        bottom_layer_count: readNumber(print, "bottom_layers") ?? 0,
        transition_layer_count: readNumber(print, "transition_layers") ?? 0,
        anti_alias_level: Math.max(readNumber(advanced, "antialasing_level") ?? 1, 1),
        encryption_key: 0,
        exposure_time: readNumber(print, "exposure_time") ?? 2,
        bottom_exposure_time: readNumber(print, "bottom_exposure_time") ?? 30,
        light_off_delay: readNumber(print, "light_off_delay") ?? 0,
        bottom_light_off_delay: readNumber(print, "bottom_light_off_delay") ?? 0,
        light_pwm: readNumber(advanced, "light_pwm") ?? 255,
        bottom_light_pwm: readNumber(advanced, "bottom_light_pwm") ?? 255,
        lift_height: readNumber(print, "lift_distance") ?? 5,
        lift_speed: readNumber(print, "lift_speed") ?? 60,
        lift_height2: readNumber(print, "lift_distance2") ?? 0,
        lift_speed2: readNumber(print, "lift_speed2") ?? 0,
        retract_speed: readNumber(print, "retract_speed") ?? 150,
        retract_height2: readNumber(print, "retract_distance2") ?? 0,
        retract_speed2: readNumber(print, "retract_speed2") ?? 0,
        bottom_lift_height: readNumber(print, "bottom_lift_distance") ?? 5,
        bottom_lift_speed: readNumber(print, "bottom_lift_speed") ?? 60,
        wait_time_before_cure: readNumber(print, "wait_time_before_cure") ?? 0,
        wait_time_after_cure: readNumber(print, "wait_time_after_cure") ?? 0,
        wait_time_after_lift: readNumber(print, "wait_time_after_lift") ?? 0,
        print_time_sec: readNumber(print, "print_time") ?? 0,
        volume_ml: readNumber(print, "volume_ml") ?? 0,
        weight_g: readNumber(print, "weight_g") ?? 0,
        cost: readNumber(print, "cost") ?? 0
    };
}
