import { build_archive_metadata_entries, describe_archive_layer_entries } from "../../../lib/slicer_converter.js";

import type { ArchiveLayerEntryPlan, ArchiveMetadataEntryPlan, FileMetadata } from "../types.js";
import { buildFileMetadataJson } from "../format/header.js";

/**
 * @zh ZIP archive entry 的压缩方式。
 * @en Compression method for a ZIP archive entry.
 */
export type ArchiveCompressionMethod = 0 | 8;

/**
 * @zh Rust archive plan 输出的 metadata entry，已转换为浏览器可写字节。
 * @en Metadata entry produced from the Rust archive plan, converted to browser-writable bytes.
 */
export interface ArchiveMetadataOutputEntry {
    name: string;
    data: Uint8Array;
    compressionMethod: ArchiveCompressionMethod;
}

/**
 * @zh Rust archive plan 输出的 layer entry 描述。
 * @en Layer entry descriptor produced from the Rust archive plan.
 */
export interface ArchiveLayerOutputEntry {
    index: number;
    name: string;
    compressionMethod: ArchiveCompressionMethod;
}

/**
 * @zh 构建 archive metadata entries 所需的输入。
 * @en Input required to build archive metadata entries.
 */
export interface ArchiveMetadataEntriesInput {
    metadata: FileMetadata;
    targetFormat: string;
    jobName: string;
    totalLayers: number;
}

/**
 * @zh 描述一段连续 layer entries 所需的输入。
 * @en Input required to describe a contiguous range of layer entries.
 */
export interface ArchiveLayerEntriesInput {
    targetFormat: string;
    jobName: string;
    startIndex: number;
    count: number;
}

/**
 * @zh 单个 archive layer entry 描述。
 * @en Descriptor for one archive layer entry.
 */
export type ArchiveLayerEntryDescriptor = ArchiveLayerOutputEntry;

/**
 * @zh 调用 WASM archive planner 并返回浏览器侧 metadata entries。
 * @en Calls the WASM archive planner and returns browser-side metadata entries.
 */
export function buildArchiveMetadataEntries(input: ArchiveMetadataEntriesInput): ArchiveMetadataOutputEntry[] {
    const entries = parsePlanArray<ArchiveMetadataEntryPlan>(
        build_archive_metadata_entries(
            buildFileMetadataJson(input.metadata),
            input.targetFormat,
            input.jobName,
            input.totalLayers
        ),
        "archive metadata entries"
    );

    return entries.map((entry) => ({
        name: entry.name,
        data: Uint8Array.from(entry.data),
        compressionMethod: toArchiveCompressionMethod(entry.compression_method)
    }));
}

/**
 * @zh 调用 WASM archive planner 描述指定范围内的 layer entries。
 * @en Calls the WASM archive planner to describe layer entries in a requested range.
 */
export function describeArchiveLayerEntries(input: ArchiveLayerEntriesInput): ArchiveLayerOutputEntry[] {
    const entries = parsePlanArray<ArchiveLayerEntryPlan>(
        describe_archive_layer_entries(input.targetFormat, input.jobName, input.startIndex, input.count),
        "archive layer entries"
    );

    return entries.map((entry) => ({
        index: entry.index,
        name: entry.name,
        compressionMethod: toArchiveCompressionMethod(entry.compression_method)
    }));
}

/**
 * @zh 描述单个 archive layer entry。
 * @en Describes one archive layer entry.
 */
export function describeArchiveLayerEntry(
    input: Omit<ArchiveLayerEntriesInput, "startIndex" | "count"> & { index: number }
): ArchiveLayerEntryDescriptor {
    const [entry] = describeArchiveLayerEntries({
        targetFormat: input.targetFormat,
        jobName: input.jobName,
        startIndex: input.index,
        count: 1
    });
    if (!entry) {
        throw new Error(`Missing archive layer entry descriptor for ${input.targetFormat} layer ${input.index}`);
    }
    return entry;
}

/**
 * @zh 创建按 layer index 查找 archive entry descriptor 的 provider。
 * @en Creates a provider that resolves archive entry descriptors by layer index.
 */
export function createArchiveLayerDescriptorProvider(
    input: ArchiveLayerEntriesInput
): (index: number) => ArchiveLayerEntryDescriptor {
    const descriptors = describeArchiveLayerEntries(input);
    const descriptorsByIndex = new Map(descriptors.map((descriptor) => [descriptor.index, descriptor]));

    return (index) => {
        const descriptor = descriptorsByIndex.get(index);
        if (!descriptor) {
            throw new Error(`Missing ZIP layer descriptor for target format ${input.targetFormat} layer ${index}`);
        }
        return descriptor;
    };
}

function parsePlanArray<T>(json: string, label: string): T[] {
    const value = JSON.parse(json) as unknown;
    if (!Array.isArray(value)) {
        throw new Error(`WASM ${label} plan must be an array`);
    }
    return value as T[];
}

function toArchiveCompressionMethod(value: number): ArchiveCompressionMethod {
    if (value === 0 || value === 8) {
        return value;
    }
    throw new Error(`Unsupported ZIP compression method from WASM archive plan: ${value}`);
}
