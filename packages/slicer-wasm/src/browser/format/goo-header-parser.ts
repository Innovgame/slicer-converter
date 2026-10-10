import type { FileMetadata, LayerOffset, ParsedHeader, Thumbnail } from "../types.js";

const SMALL_PREVIEW_W = 116;
const SMALL_PREVIEW_H = 116;
const SMALL_PREVIEW_BYTES = SMALL_PREVIEW_W * SMALL_PREVIEW_H * 2;
const BIG_PREVIEW_W = 290;
const BIG_PREVIEW_H = 290;
const BIG_PREVIEW_BYTES = BIG_PREVIEW_W * BIG_PREVIEW_H * 2;
const GOO_MAGIC = [0x07, 0x00, 0x00, 0x00, 0x44, 0x4c, 0x50, 0x00];
const FIXED_HEADER_BYTES = 195_477;
const LAYER_DEF_SIZE = 70;
const DATA_LENGTH_OFFSET = 66;

/**
 * @zh Streaming GOO header parser，仅读取固定头部和 70-byte layer definition。
 * @en Streaming GOO header parser; reads only the fixed header and 70-byte layer definitions.
 */
export async function parseGooHeaderStreaming(file: File): Promise<ParsedHeader> {
    const headerSize = Math.min(file.size, FIXED_HEADER_BYTES);
    const headerBuf = new Uint8Array(await readSlice(file, 0, headerSize));

    if (headerBuf.length < 400) {
        throw new Error(`GOO file too small (${headerBuf.length} bytes)`);
    }

    for (let i = 0; i < GOO_MAGIC.length; i += 1) {
        if (headerBuf[4 + i] !== GOO_MAGIC[i]) {
            throw new Error("Not a valid GOO file (magic mismatch)");
        }
    }

    const view = new DataView(headerBuf.buffer, headerBuf.byteOffset, headerBuf.byteLength);
    const metadata = parseFixedMetadata(headerBuf, view);
    const layers = await walkLayerDefs(file, metadata._layerDefAddress, metadata.layer_count ?? 0);
    const { _layerDefAddress: _, ...cleanMetadata } = metadata;

    return {
        format: "goo" as never,
        metadata: cleanMetadata,
        layers
    };
}

type MetadataWithInternal = FileMetadata & { _layerDefAddress: number };

function parseFixedMetadata(buf: Uint8Array, view: DataView): MetadataWithInternal {
    let pos = 0;
    pos += 4 + 8;
    pos += 32 + 24 + 24;
    const machineName = decodeNullTerminated(buf, pos, 32);
    pos += 32;
    pos += 32 + 32;
    pos += 2 + 2 + 2;

    const smallPreview = buf.slice(pos, pos + SMALL_PREVIEW_BYTES);
    pos += SMALL_PREVIEW_BYTES + 2;
    const bigPreview = buf.slice(pos, pos + BIG_PREVIEW_BYTES);
    pos += BIG_PREVIEW_BYTES + 2;

    const layerCount = view.getUint32(pos, false);
    pos += 4;
    const resolutionX = view.getUint16(pos, false);
    pos += 2;
    const resolutionY = view.getUint16(pos, false);
    pos += 2;
    const mirrorX = buf[pos] !== 0;
    pos += 1;
    const mirrorY = buf[pos] !== 0;
    pos += 1;
    const displayWidthMm = view.getFloat32(pos, false);
    pos += 4;
    const displayHeightMm = view.getFloat32(pos, false);
    pos += 4;
    const machineZMm = view.getFloat32(pos, false);
    pos += 4;
    const layerHeightMm = view.getFloat32(pos, false);
    pos += 4;
    const exposureTime = view.getFloat32(pos, false);
    pos += 4;
    pos += 1;
    const lightOffDelay = view.getFloat32(pos, false);
    pos += 4;
    pos += 4 + 4 + 4;
    const waitTimeAfterCure = view.getFloat32(pos, false);
    pos += 4;
    const waitTimeAfterLift = view.getFloat32(pos, false);
    pos += 4;
    const waitTimeBeforeCure = view.getFloat32(pos, false);
    pos += 4;
    const bottomExposureTime = view.getFloat32(pos, false);
    pos += 4;
    const bottomLayerCount = view.getUint32(pos, false);
    pos += 4;
    const bottomLiftHeight = view.getFloat32(pos, false);
    pos += 4;
    const bottomLiftSpeed = view.getFloat32(pos, false);
    pos += 4;
    const liftHeight = view.getFloat32(pos, false);
    pos += 4;
    const liftSpeed = view.getFloat32(pos, false);
    pos += 4;
    pos += 4 + 4;
    pos += 4;
    const retractSpeed = view.getFloat32(pos, false);
    pos += 4;
    pos += 4 + 4;
    const liftHeight2 = view.getFloat32(pos, false);
    pos += 4;
    const liftSpeed2 = view.getFloat32(pos, false);
    pos += 4;
    pos += 4 + 4;
    const retractHeight2 = view.getFloat32(pos, false);
    pos += 4;
    const retractSpeed2 = view.getFloat32(pos, false);
    pos += 4;
    const bottomLightPwm = view.getUint16(pos, false);
    pos += 2;
    const lightPwm = view.getUint16(pos, false);
    pos += 2;
    pos += 1;
    const printTimeSec = view.getUint32(pos, false);
    pos += 4;
    const volumeMm3 = view.getFloat32(pos, false);
    pos += 4;
    const materialGrams = view.getFloat32(pos, false);
    pos += 4;
    const materialCost = view.getFloat32(pos, false);
    pos += 4;
    pos += 8;
    const layerDefAddress = view.getUint32(pos, false);
    pos += 4;
    pos += 1;
    const transitionLayerCount = view.getUint16(pos, false);

    const thumbnails: Thumbnail[] = [];
    const smallRgba = decodeRgb565BE(smallPreview, SMALL_PREVIEW_W, SMALL_PREVIEW_H);
    if (smallRgba) {
        thumbnails.push({ width: SMALL_PREVIEW_W, height: SMALL_PREVIEW_H, pixels_rgba: smallRgba });
    }
    const bigRgba = decodeRgb565BE(bigPreview, BIG_PREVIEW_W, BIG_PREVIEW_H);
    if (bigRgba) {
        thumbnails.push({ width: BIG_PREVIEW_W, height: BIG_PREVIEW_H, pixels_rgba: bigRgba });
    }

    return {
        resolution_x: resolutionX,
        resolution_y: resolutionY,
        display_width_mm: displayWidthMm,
        display_height_mm: displayHeightMm,
        machine_z_mm: machineZMm,
        machine_name: machineName,
        mirror_x: mirrorX,
        mirror_y: mirrorY,
        layer_height_mm: layerHeightMm,
        layer_count: layerCount,
        bottom_layer_count: bottomLayerCount,
        transition_layer_count: transitionLayerCount,
        exposure_time: exposureTime,
        bottom_exposure_time: bottomExposureTime,
        light_off_delay: lightOffDelay,
        bottom_light_off_delay: lightOffDelay,
        light_pwm: lightPwm,
        bottom_light_pwm: bottomLightPwm,
        lift_height: liftHeight,
        lift_speed: liftSpeed,
        lift_height2: liftHeight2,
        lift_speed2: liftSpeed2,
        retract_speed: retractSpeed,
        retract_height2: retractHeight2,
        retract_speed2: retractSpeed2,
        bottom_lift_height: bottomLiftHeight,
        bottom_lift_speed: bottomLiftSpeed,
        wait_time_before_cure: waitTimeBeforeCure,
        wait_time_after_cure: waitTimeAfterCure,
        wait_time_after_lift: waitTimeAfterLift,
        anti_alias_level: 1,
        encryption_key: 0,
        print_time_sec: printTimeSec,
        volume_ml: volumeMm3 / 1000,
        weight_g: materialGrams,
        cost: materialCost,
        thumbnails,
        _layerDefAddress: layerDefAddress
    };
}

async function walkLayerDefs(file: File, startOffset: number, layerCount: number): Promise<LayerOffset[]> {
    const layers: LayerOffset[] = [];
    let pos = startOffset;

    for (let i = 0; i < layerCount; i += 1) {
        if (pos + LAYER_DEF_SIZE > file.size) {
            throw new Error(
                `GOO layer table truncated at layer ${i}: need ${pos + LAYER_DEF_SIZE} bytes, file is ${file.size}`
            );
        }

        const defBuf = new Uint8Array(await readSlice(file, pos, LAYER_DEF_SIZE));
        const dataLength = new DataView(defBuf.buffer, defBuf.byteOffset, defBuf.byteLength).getUint32(
            DATA_LENGTH_OFFSET,
            false
        );
        const dataStart = pos + LAYER_DEF_SIZE;
        layers.push({ offset: dataStart, size: dataLength });
        pos = dataStart + dataLength + 2;
    }

    return layers;
}

function readSlice(file: File, offset: number, size: number): Promise<ArrayBuffer> {
    return file.slice(offset, offset + size).arrayBuffer();
}

function decodeNullTerminated(buf: Uint8Array, offset: number, maxLen: number): string {
    let end = offset;
    const limit = offset + maxLen;
    while (end < limit && buf[end] !== 0) {
        end += 1;
    }
    return new TextDecoder().decode(buf.subarray(offset, end));
}

function decodeRgb565BE(data: Uint8Array, w: number, h: number): Uint8Array | null {
    const expected = w * h * 2;
    if (data.length < expected) {
        return null;
    }

    const rgba = new Uint8Array(w * h * 4);
    for (let i = 0; i < w * h; i += 1) {
        const hi = data[i * 2];
        const lo = data[i * 2 + 1];
        const value = (hi << 8) | lo;
        const offset = i * 4;
        rgba[offset] = ((value >> 11) & 0x1f) << 3;
        rgba[offset + 1] = ((value >> 5) & 0x3f) << 2;
        rgba[offset + 2] = (value & 0x1f) << 3;
        rgba[offset + 3] = 255;
    }

    return rgba;
}
