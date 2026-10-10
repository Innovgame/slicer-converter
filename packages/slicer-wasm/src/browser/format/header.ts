import { parse_header as parseHeaderRaw } from "../../../lib/slicer_converter.js";

import { toWasmError } from "../runtime/wasm.js";

import type { FileMetadata, ParsedHeader } from "../types.js";

/**
 * @zh 解析切片文件头部并返回通用的 {@link ParsedHeader}。
 * @en Parses a slicer file header and returns the generic {@link ParsedHeader}.
 *
 * @zh 内部调用 Rust header parser，自动通过魔数字节或文件扩展名检测格式。
 * @en Calls the Rust header parser internally; format is detected automatically
 * via magic bytes or the provided file extension hint.
 *
 * @param data - @zh 完整文件的字节内容 @en Full file bytes
 * @param extension - @zh 文件扩展名提示（如 `"ctb"`），用于魔数字节无法区分格式时的备选提示
 *   @en File extension hint (e.g. `"ctb"`), used as a fallback when magic bytes
 *   are ambiguous
 */
export function parseBrowserFileHeader(data: Uint8Array, extension: string): ParsedHeader {
    try {
        return JSON.parse(parseHeaderRaw(data, extension)) as ParsedHeader;
    } catch (error: unknown) {
        throw toWasmError(error);
    }
}

/**
 * @zh 从 {@link FileMetadata} 构建层编码所需的 JSON 字符串。
 * @en Builds the layer-metadata JSON string required for layer encoding.
 *
 * @zh 对未提供的字段使用内置合理默认值（与大多数常用切片机设置水平一致）。
 * @en Applies sensible defaults for fields that are not provided (aligned with
 * typical slicer machine settings).
 *
 * @param metadata - @zh 来自已解析头部的文件元数据 @en File metadata from a parsed header
 * @returns @zh 序列化后的 JSON 字符串，可用于 layer encode 与 document assembly helper
 *   @en Serialized JSON string; pass as the `layer_metadata_json` parameter of
 *   layer encode and document assembly helpers
 */
export function buildLayerMetadataJson(metadata: FileMetadata): string {
    return JSON.stringify({
        /**
         * @zh 打印机规格字段。
         * @en Printer specification fields.
         */
        resolution_x: metadata.resolution_x ?? 0,
        resolution_y: metadata.resolution_y ?? 0,
        display_width_mm: metadata.display_width_mm ?? 0,
        display_height_mm: metadata.display_height_mm ?? 0,
        machine_z_mm: metadata.machine_z_mm ?? 0,
        machine_name: metadata.machine_name ?? "",
        mirror_x: metadata.mirror_x ?? false,
        mirror_y: metadata.mirror_y ?? false,
        /**
         * @zh 切片规格字段。
         * @en Slicing specification fields.
         */
        layer_height_mm: metadata.layer_height_mm ?? 0.05,
        layer_count: metadata.layer_count ?? 0,
        bottom_layer_count: metadata.bottom_layer_count ?? 0,
        transition_layer_count: metadata.transition_layer_count ?? 0,
        anti_alias_level: metadata.anti_alias_level ?? 1,
        encryption_key: metadata.encryption_key ?? 0,
        /**
         * @zh 曝光计划字段。
         * @en Exposure plan fields.
         */
        exposure_time: metadata.exposure_time ?? 2.0,
        bottom_exposure_time: metadata.bottom_exposure_time ?? metadata.exposure_time ?? 2.0,
        light_off_delay: metadata.light_off_delay ?? 0,
        bottom_light_off_delay: metadata.bottom_light_off_delay ?? metadata.light_off_delay ?? 0,
        light_pwm: metadata.light_pwm ?? 255,
        bottom_light_pwm: metadata.bottom_light_pwm ?? metadata.light_pwm ?? 255,
        /**
         * @zh 抬升和回抽运动计划字段。
         * @en Lift and retract motion plan fields.
         */
        lift_height: metadata.lift_height ?? 5.0,
        lift_speed: metadata.lift_speed ?? 65.0,
        lift_height2: metadata.lift_height2 ?? 0,
        lift_speed2: metadata.lift_speed2 ?? 0,
        retract_speed: metadata.retract_speed ?? 150.0,
        retract_height2: metadata.retract_height2 ?? 0,
        retract_speed2: metadata.retract_speed2 ?? 0,
        bottom_lift_height: metadata.bottom_lift_height ?? metadata.lift_height ?? 5.0,
        bottom_lift_speed: metadata.bottom_lift_speed ?? metadata.lift_speed ?? 65.0,
        /**
         * @zh 固化前后等待时间字段。
         * @en Wait-time fields around curing and lifting.
         */
        wait_time_before_cure: metadata.wait_time_before_cure ?? 0,
        wait_time_after_cure: metadata.wait_time_after_cure ?? 0,
        wait_time_after_lift: metadata.wait_time_after_lift ?? 0
    });
}

/**
 * @zh 从完整的 {@link FileMetadata} 构建可安全传入 Rust 的 JSON 字符串。
 * @en Builds a JSON string from the full {@link FileMetadata} that is safe to
 * pass back into Rust.
 *
 * @zh 会把 `Uint8Array` 缩略图像素规范化为普通数组，避免 `JSON.stringify`
 * 把 typed array 编成对象映射。
 * @en Normalizes thumbnail `Uint8Array` buffers into plain arrays so
 * `JSON.stringify` does not encode them as object maps.
 */
export function buildFileMetadataJson(metadata: FileMetadata): string {
    return JSON.stringify({
        ...metadata,
        thumbnails: metadata.thumbnails?.map((thumbnail) => ({
            ...thumbnail,
            pixels_rgba: Array.from(thumbnail.pixels_rgba)
        }))
    });
}
