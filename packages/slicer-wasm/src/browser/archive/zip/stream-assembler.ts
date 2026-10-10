/**
 * @zh 流式 ZIP 组装器：按 `index` 升序写入 layer 条目，并允许 metadata 条目立即写入。
 * @en Streaming ZIP assembler that writes layer entries in ascending `index` order while metadata entries flush immediately.
 */

import type { ZipAssemblyTimings } from "./assembly-timings.js";
import type { SlicerOutputSink } from "../../output/sink.js";
import {
    buildCentralHeader,
    buildEocd,
    buildLocalHeader,
    encodeZipEntryName,
    type ZipCentralDescriptor,
    type ZipPreparedEntry
} from "./archive.js";

/**
 * @zh ZIP 流式组装完成后的结果；内存 sink 返回 `Blob`，外部 streaming sink 返回 `null`。
 * @en Result returned after ZIP streaming assembly finishes; in-memory sinks return a `Blob`, external streaming sinks return `null`.
 */
export interface ZipStreamAssemblerResult {
    blob: Blob | null;
    timings: ZipAssemblyTimings;
}

/**
 * @zh 使用 prepared ZIP entries 组装 archive 的浏览器侧流式 writer。
 * @en Browser-side streaming writer that assembles an archive from prepared ZIP entries.
 */
export class ZipStreamAssembler {
    private readonly sink: SlicerOutputSink;
    private readonly centralDescriptors: ZipCentralDescriptor[] = [];
    private localOffset = 0;
    private centralSize = 0;
    private entryCount = 0;
    private readonly pending = new Map<number, ZipPreparedEntry>();
    private nextLayerFlushIndex = 0;
    private finalized = false;
    private readonly startedAt = performance.now();
    /**
     * @zh 串行化所有 sink 写入，避免并发 push 时 header/payload 字节交错。
     * @en Serializes all sink writes so concurrent pushes cannot interleave header and payload bytes.
     */
    private writeChain: Promise<void> = Promise.resolve();

    constructor(sink: SlicerOutputSink) {
        this.sink = sink;
    }

    private runExclusive<T>(task: () => Promise<T>): Promise<T> {
        const run = this.writeChain.then(task, task);
        this.writeChain = run.then(
            () => undefined,
            () => undefined
        );
        return run;
    }

    /**
     * @zh 立即写入一个 prepared 条目，通常用于 archive metadata，不参与 layer 排序窗口。
     * @en Writes one prepared entry immediately, usually archive metadata, outside the layer ordering window.
     */
    writePreparedEntry(entry: ZipPreparedEntry): Promise<void> {
        return this.runExclusive(async () => {
            this.ensureNotFinalized();
            await this.flushEntry(entry);
        });
    }

    /**
     * @zh 入队按 layer index 排序的条目，并在连续前缀补齐后按升序 flush。
     * @en Enqueues a layer-indexed entry and flushes the contiguous prefix in ascending order.
     */
    pushLayerEntry(index: number, entry: ZipPreparedEntry): Promise<void> {
        return this.runExclusive(async () => {
            this.ensureNotFinalized();
            await this.enqueueLayerEntry(index, entry);
        });
    }

    /**
     * @zh 当前已到达但尚未 flush 的 layer 数，用于测试和诊断。
     * @en Count of arrived but not yet flushed layers, for tests and diagnostics.
     */
    get pendingCount(): number {
        return this.pending.size;
    }

    /**
     * @zh 写入 Central Directory 与 EOCD 并完成 sink；仍有 pending layer 时抛错。
     * @en Writes the Central Directory and EOCD, then finishes the sink; pending layers are treated as errors.
     */
    finalize(): Promise<ZipStreamAssemblerResult> {
        return this.runExclusive(async () => {
            const finalizeStartedAt = performance.now();
            this.ensureNotFinalized();
            if (this.pending.size > 0) {
                const missing = Array.from(this.pending.keys())
                    .sort((a, b) => a - b)
                    .join(",");
                throw new Error(`ZipStreamAssembler.finalize: ${this.pending.size} layer(s) still pending: ${missing}`);
            }
            this.finalized = true;

            const centralDirectoryStartedAt = performance.now();
            const centralDirectory = this.buildCentralDirectory();
            const centralDirectoryBytes = centralDirectory.byteLength;
            await this.sink.write(centralDirectory);
            const centralDirectoryWallMs = performance.now() - centralDirectoryStartedAt;

            const sinkFinishStartedAt = performance.now();
            const blob = await this.sink.finish();
            const sinkFinishWallMs = performance.now() - sinkFinishStartedAt;

            const finalizeEndedAt = performance.now();
            const totalWallMs = finalizeEndedAt - this.startedAt;
            return {
                blob,
                timings: {
                    prepareWallMs: 0,
                    finalizeOnlyWallMs: finalizeEndedAt - finalizeStartedAt,
                    totalWallMs,
                    centralDirectoryWallMs,
                    centralDirectoryBytes,
                    centralDirectoryEntryCount: this.centralDescriptors.length,
                    sinkFinishWallMs
                }
            };
        });
    }

    private buildCentralDirectory(): Uint8Array {
        const centralHeaders = this.centralDescriptors.map((descriptor) => buildCentralHeader(descriptor));
        const eocd = buildEocd(this.entryCount, this.centralSize, this.localOffset);
        const totalLength = centralHeaders.reduce((sum, header) => sum + header.byteLength, eocd.byteLength);
        const result = new Uint8Array(totalLength);
        let offset = 0;
        for (const header of centralHeaders) {
            result.set(header, offset);
            offset += header.byteLength;
        }
        result.set(eocd, offset);
        return result;
    }

    private ensureNotFinalized(): void {
        if (this.finalized) {
            throw new Error("ZipStreamAssembler: already finalized");
        }
    }

    private async enqueueLayerEntry(index: number, entry: ZipPreparedEntry): Promise<void> {
        if (index < this.nextLayerFlushIndex) {
            throw new Error(`ZipStreamAssembler: layer ${index} already flushed (next = ${this.nextLayerFlushIndex})`);
        }
        if (this.pending.has(index)) {
            throw new Error(`ZipStreamAssembler: duplicate layer index ${index}`);
        }

        if (index !== this.nextLayerFlushIndex) {
            this.pending.set(index, entry);
            return;
        }

        await this.flushEntry(entry);
        this.nextLayerFlushIndex += 1;
        while (this.pending.has(this.nextLayerFlushIndex)) {
            const next = this.pending.get(this.nextLayerFlushIndex)!;
            this.pending.delete(this.nextLayerFlushIndex);
            await this.flushEntry(next);
            this.nextLayerFlushIndex += 1;
        }
    }

    private async flushEntry(entry: ZipPreparedEntry): Promise<void> {
        const nameBytes = encodeZipEntryName(entry.name);
        const localHeader = buildLocalHeader(entry, nameBytes);

        await this.sink.write(localHeader);
        await this.sink.write(entry.compressedData);

        this.centralDescriptors.push({
            nameBytes,
            compressionMethod: entry.compressionMethod,
            compressedSize: entry.compressedData.length,
            uncompressedSize: entry.uncompressedSize,
            crc32: entry.crc32,
            localOffset: this.localOffset
        });
        this.centralSize += 46 + nameBytes.length;
        this.localOffset += localHeader.length + entry.compressedData.length;
        this.entryCount += 1;
    }
}
