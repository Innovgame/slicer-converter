import type {
    PatchableSlicerOutputSink,
    ZipAssemblyTimings,
    ZipCentralDescriptor,
    ZipPreparedEntry
} from "../../../src/index.js";
import {
    buildCentralHeader,
    buildEocd,
    buildLocalHeader,
    encodeZipEntryName,
    ZipStreamAssembler
} from "../../../src/index.js";

interface TestZipArchiveBuildResult {
    blob: Blob | null;
    timings: ZipAssemblyTimings;
}

/**
 * @zh 测试用 ZIP archive reference builder；保持独立于流式 assembler。
 * @en ZIP archive reference builder for tests; remains independent of the streaming assembler.
 */
export async function assembleZipArchiveForTest(
    entries: ZipPreparedEntry[],
    sink: PatchableSlicerOutputSink
): Promise<TestZipArchiveBuildResult> {
    const centralDescriptors: ZipCentralDescriptor[] = [];
    let localOffset = 0;
    let centralSize = 0;
    const startedAt = performance.now();

    for (const entry of entries) {
        const nameBytes = encodeZipEntryName(entry.name);
        const localHeader = buildLocalHeader(entry, nameBytes);

        await sink.write(localHeader);
        await sink.write(entry.compressedData);

        centralDescriptors.push({
            nameBytes,
            compressionMethod: entry.compressionMethod,
            compressedSize: entry.compressedData.length,
            uncompressedSize: entry.uncompressedSize,
            crc32: entry.crc32,
            localOffset
        });
        centralSize += 46 + nameBytes.length;
        localOffset += localHeader.length + entry.compressedData.length;
    }

    for (const descriptor of centralDescriptors) {
        await sink.write(buildCentralHeader(descriptor));
    }
    await sink.write(buildEocd(entries.length, centralSize, localOffset));

    const blob = await sink.finish();
    const totalWallMs = performance.now() - startedAt;
    return {
        blob,
        timings: {
            prepareWallMs: 0,
            finalizeOnlyWallMs: totalWallMs,
            totalWallMs
        }
    };
}

/**
 * @zh 测试用 `ZipStreamAssembler` factory；固定从 package root surface 构造。
 * @en Test `ZipStreamAssembler` factory; pins construction to the package root surface.
 */
export function createZipStreamAssemblerForTest(sink: PatchableSlicerOutputSink): ZipStreamAssembler {
    return new ZipStreamAssembler(sink);
}
