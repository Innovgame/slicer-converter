/**
 * @zh 从 Rust 目录 facts 枚举可导出 profile、版本和 archive 扩展名。
 * @en Enumerates export profiles, versions, and archive extensions from Rust catalog facts.
 */
export function buildFormatValidationCases(formatInfos) {
    return formatInfos
        .filter((info) => info.capabilities.can_export)
        .flatMap((info) => {
            const versions = info.specifics?.supported_versions ?? [null];
            const extensions = info.is_zip ? [...new Set(info.extensions)] : [info.default_extension];
            return versions.flatMap((version) =>
                extensions.map((extension) => ({
                    key: `${info.name}-${version ?? "default"}-${extension}`,
                    format: info.name,
                    profileId: info.profile_id,
                    version,
                    headerVersion: ["ctb", "gktwo-ctb", "cbddlp", "photon", "encrypted-ctb"].includes(
                        info.specifics?.kind
                    )
                        ? info.specifics.ctb_version
                        : null,
                    versionOffset: info.specifics?.kind === "encrypted-ctb" ? 16 : version !== null ? 12 : 4,
                    codecFamily: info.specifics?.family ?? null,
                    extension
                }))
            );
        });
}

/**
 * @zh 统计真实解码像素差异；大小不一致必须由调用方显式处理重采样。
 * @en Measures decoded pixel differences; callers must explicitly handle resampling when sizes differ.
 */
export function comparePixels(before, after) {
    if (before.length !== after.length) throw new Error(`Pixel count mismatch: ${before.length} != ${after.length}`);
    let changedPixels = 0;
    let maxAbsoluteError = 0;
    let absoluteError = 0;
    for (let index = 0; index < before.length; index++) {
        const error = Math.abs(before[index] - after[index]);
        if (error !== 0) changedPixels++;
        maxAbsoluteError = Math.max(maxAbsoluteError, error);
        absoluteError += error;
    }
    return {
        pixels: before.length,
        changedPixels,
        maxAbsoluteError,
        meanAbsoluteError: before.length ? absoluteError / before.length : 0
    };
}

/**
 * @zh 格式允许的独立灰度投影，用于验证有损编码的实际输出。
 * @en Independent grayscale projection allowed by a format, used to verify lossy encoded output.
 */
export function quantizePixel(value, mode) {
    if (mode === "lossless") return value;
    if (mode === "gray4") return (value >> 4) * 17;
    if (mode === "binary") return value >= 128 ? 255 : 0;
    if (mode === "phz" && value >= 252) return 255;
    if (mode === "cxdlp") {
        const gray = value >> 1;
        return gray === 0 ? 0 : (gray << 1) | 1;
    }
    if (mode === "phz") {
        const gray = value >> 1;
        return (gray << 1) | (gray & 1);
    }
    if (mode === "gray7") {
        const gray = value >> 1;
        return (gray << 1) | (gray >> 6);
    }
    throw new Error(`Unknown grayscale mode: ${mode}`);
}

/**
 * @zh 以独立 nearest-neighbor 规则投影尺寸和灰度。
 * @en Projects image dimensions and grayscale using independent nearest-neighbor rules.
 */
export function resizeAndQuantizePixels(pixels, sourceWidth, sourceHeight, targetWidth, targetHeight, mode) {
    if (pixels.length !== sourceWidth * sourceHeight) throw new Error("Invalid source pixel count");
    const output = new Uint8Array(targetWidth * targetHeight);
    for (let y = 0; y < targetHeight; y++) {
        const sourceY = Math.floor((y * sourceHeight) / targetHeight);
        for (let x = 0; x < targetWidth; x++) {
            const sourceX = Math.floor((x * sourceWidth) / targetWidth);
            output[y * targetWidth + x] = quantizePixel(pixels[sourceY * sourceWidth + sourceX], mode);
        }
    }
    return output;
}

/**
 * @zh 独立校验产物的格式身份和物理 header 版本。
 * @en Independently verifies artifact format identity and physical header version.
 */
export function verifyArtifactIdentity(row, parsedFormat, bytes) {
    if (parsedFormat !== row.format) throw new Error(`Artifact format mismatch: ${parsedFormat} != ${row.format}`);
    const expected = row.version ?? row.headerVersion;
    if (expected == null) return null;
    const offset = row.versionOffset ?? (row.version != null ? 12 : 4);
    if (bytes.length < offset + 4) throw new Error("Truncated artifact version field");
    const actual = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset, true);
    if (actual !== expected) throw new Error(`Artifact version mismatch: ${actual} != ${expected}`);
    return actual;
}
