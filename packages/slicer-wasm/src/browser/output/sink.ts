/**
 * @zh 多态 slicer 输出目标；只负责追加写入、完成和中止，不包含 header patch 能力。
 * @en Polymorphic slicer output target; it only owns append writes, completion, and aborting.
 */
export interface SlicerOutputSink {
    /**
     * @zh 在当前写入位置追加字节。
     * @en Appends bytes at the current write position.
     */
    write(data: Uint8Array): void | Promise<void>;
    /**
     * @zh 完成输出；内存 sink 返回 `Blob`，streaming sink 返回 `null`。
     * @en Finalizes output; returns `Blob` for in-memory sinks and `null` for streaming sinks.
     */
    finish(): Promise<Blob | null>;
    /**
     * @zh 出错时中止输出并尽力清理资源。
     * @en Aborts output on error and performs best-effort cleanup.
     */
    abort(reason?: string): Promise<void>;
    /**
     * @zh 是否直接流式写入外部目标。
     * @en Whether this sink streams directly to an external target.
     */
    readonly streaming: boolean;
}

/**
 * @zh 可在 layer size 已知后重写头部字节的 slicer 输出目标。
 * @en Slicer output target that can rewrite header bytes after layer sizes are known.
 */
export interface PatchableSlicerOutputSink extends SlicerOutputSink {
    /**
     * @zh 覆盖首次 `write()` 写入的初始头部。
     * @en Overwrites the initial header written by the first `write()` call.
     */
    patchHeader(data: Uint8Array): void | Promise<void>;
}

/**
 * @zh 缓冲 slicer 输出 sink 的配置。
 * @en Configuration for buffered slicer output sinks.
 */
export interface BufferedSlicerOutputSinkOptions {
    flushBytes?: number;
    now?: () => number;
}

/**
 * @zh 缓冲 slicer 输出 sink 的诊断统计。
 * @en Diagnostic statistics for buffered slicer output sinks.
 */
export interface BufferedSlicerOutputSinkStats {
    flushCount: number;
    flushBytes: number;
    flushWallMs: number;
    maxFlushWallMs: number;
    finishCount: number;
    finishWallMs: number;
    innerFinishWallMs: number;
}

class BlobOutputSink implements PatchableSlicerOutputSink {
    private parts: Uint8Array[] = [];
    private state: "open" | "finished" | "aborted" = "open";
    readonly streaming = false;

    write(data: Uint8Array): void {
        this.assertOpen();
        this.parts.push(data);
    }

    patchHeader(data: Uint8Array): void {
        this.assertOpen();
        this.parts[0] = data;
    }

    async finish(): Promise<Blob> {
        this.assertOpen();
        this.state = "finished";
        return new Blob(this.parts.map(toBlobPart), { type: "application/octet-stream" });
    }

    async abort(): Promise<void> {
        if (this.state === "finished") {
            return;
        }
        this.parts = [];
        this.state = "aborted";
    }

    private assertOpen(): void {
        if (this.state === "finished") {
            throw new Error("Blob output sink is already finished");
        }
        if (this.state === "aborted") {
            throw new Error("Blob output sink is aborted");
        }
    }
}

/**
 * @zh 为 patchable slicer 输出 sink 增加小块合并缓冲，并保留 flush/finish 诊断数据。
 * @en Adds small-write coalescing to a patchable slicer output sink while retaining flush/finish diagnostics.
 */
export class BufferedSlicerOutputSink implements PatchableSlicerOutputSink {
    private readonly flushBytes: number;
    private readonly now: () => number;
    private readonly stats: BufferedSlicerOutputSinkStats = {
        flushCount: 0,
        flushBytes: 0,
        flushWallMs: 0,
        maxFlushWallMs: 0,
        finishCount: 0,
        finishWallMs: 0,
        innerFinishWallMs: 0
    };
    private pending: Uint8Array[] = [];
    private pendingBytes = 0;
    private state: "open" | "finished" | "aborted" = "open";

    constructor(
        private readonly inner: PatchableSlicerOutputSink,
        options: BufferedSlicerOutputSinkOptions = {}
    ) {
        this.flushBytes = options.flushBytes ?? 4 * 1024 * 1024;
        this.now = options.now ?? (() => performance.now());
    }

    get streaming(): boolean {
        return this.inner.streaming;
    }

    async write(data: Uint8Array): Promise<void> {
        this.assertOpen();
        if (data.byteLength >= this.flushBytes) {
            await this.flush();
            await this.inner.write(data);
            return;
        }

        this.pending.push(data);
        this.pendingBytes += data.byteLength;
        if (this.pendingBytes >= this.flushBytes) {
            await this.flush();
        }
    }

    async patchHeader(data: Uint8Array): Promise<void> {
        this.assertOpen();
        await this.flush();
        await this.inner.patchHeader(data);
    }

    async finish(): Promise<Blob | null> {
        this.assertOpen();
        const finishStartedAt = this.now();
        await this.flush();
        const innerFinishStartedAt = this.now();
        try {
            const result = await this.inner.finish();
            this.state = "finished";
            return result;
        } finally {
            const finishedAt = this.now();
            this.stats.finishCount += 1;
            this.stats.finishWallMs += finishedAt - finishStartedAt;
            this.stats.innerFinishWallMs += finishedAt - innerFinishStartedAt;
        }
    }

    abort(reason?: string): Promise<void> {
        if (this.state !== "open") {
            return Promise.resolve();
        }
        this.pending = [];
        this.pendingBytes = 0;
        this.state = "aborted";
        return this.inner.abort(reason);
    }

    /**
     * @zh 返回当前缓冲 flush/finish 诊断快照。
     * @en Returns the current buffered flush/finish diagnostics snapshot.
     */
    snapshotStats(): BufferedSlicerOutputSinkStats {
        return { ...this.stats };
    }

    private async flush(): Promise<void> {
        if (this.pendingBytes === 0) {
            return;
        }

        const data = concatUint8Arrays(this.pending, this.pendingBytes);
        this.pending = [];
        this.pendingBytes = 0;
        const startedAt = this.now();
        try {
            await this.inner.write(data);
        } finally {
            const elapsed = this.now() - startedAt;
            this.stats.flushCount += 1;
            this.stats.flushBytes += data.byteLength;
            this.stats.flushWallMs += elapsed;
            this.stats.maxFlushWallMs = Math.max(this.stats.maxFlushWallMs, elapsed);
        }
    }

    private assertOpen(): void {
        if (this.state === "finished") {
            throw new Error("Buffered slicer output sink is already finished");
        }
        if (this.state === "aborted") {
            throw new Error("Buffered slicer output sink is aborted");
        }
    }
}

/**
 * @zh 创建内存 Blob 输出 sink。
 * @en Creates an in-memory Blob output sink.
 */
export function createBlobOutputSink(): PatchableSlicerOutputSink {
    return new BlobOutputSink();
}

/**
 * @zh 为 slicer 输出 sink 增加批量缓冲，减少小块写入次数。
 * @en Adds batching to a slicer output sink to reduce small write calls.
 */
export function createBufferedOutputSink(
    inner: PatchableSlicerOutputSink,
    options: BufferedSlicerOutputSinkOptions = {}
): BufferedSlicerOutputSink {
    return new BufferedSlicerOutputSink(inner, options);
}

/**
 * @zh 如果 sink 是缓冲 sink，则返回当前诊断统计快照。
 * @en Returns the current diagnostics snapshot when the sink is buffered.
 */
export function snapshotBufferedOutputSinkStats(
    sink: SlicerOutputSink
): BufferedSlicerOutputSinkStats | undefined {
    if (sink instanceof BufferedSlicerOutputSink) {
        return sink.snapshotStats();
    }
    return undefined;
}

function toBlobPart(data: Uint8Array): BlobPart {
    return data as unknown as BlobPart;
}

function concatUint8Arrays(parts: Uint8Array[], totalLength: number): Uint8Array {
    if (parts.length === 1) {
        return parts[0];
    }

    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const part of parts) {
        result.set(part, offset);
        offset += part.byteLength;
    }
    return result;
}
