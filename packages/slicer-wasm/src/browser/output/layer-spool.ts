/**
 * @zh 临时保存已编码 layer bytes，并在 header/final layout 计算完成后按原 layer 边界重放。
 * @en Temporarily stores encoded layer bytes and replays them with original layer boundaries after header/final layout calculation.
 */
export interface LayerSpool {
    write(layer: Uint8Array): Promise<void>;
    closeForReplay(): Promise<void>;
    replay(consumer: (chunk: Uint8Array) => void | Promise<void>): Promise<void>;
    cleanup(): Promise<void>;
}

interface FileSystemDirectoryHandle {
    getFileHandle(name: string, options?: { create?: boolean }): Promise<FileSystemFileHandle>;
    removeEntry(name: string): Promise<void>;
}

interface FileSystemFileHandle {
    createWritable(): Promise<FileSystemWritableFileStream>;
    getFile(): Promise<Blob>;
}

interface FileSystemWritableFileStream {
    write(data: Uint8Array): Promise<void>;
    close(): Promise<void>;
    abort(reason?: unknown): Promise<void>;
}

interface OptionalOpfsStorage {
    storage?: {
        getDirectory?: () => Promise<FileSystemDirectoryHandle>;
    };
}

class MemoryLayerSpool implements LayerSpool {
    private readonly chunks: Uint8Array[] = [];
    private closed = false;

    async write(layer: Uint8Array): Promise<void> {
        if (this.closed) {
            throw new Error("Layer spool is already closed");
        }
        this.chunks.push(layer);
    }

    async closeForReplay(): Promise<void> {
        this.closed = true;
    }

    async replay(consumer: (chunk: Uint8Array) => void | Promise<void>): Promise<void> {
        assertClosedForReplay(this.closed);
        for (const chunk of this.chunks) {
            await consumer(chunk);
        }
    }

    async cleanup(): Promise<void> {
        this.chunks.length = 0;
    }
}

class OpfsLayerSpool implements LayerSpool {
    private closed = false;
    private readonly layerSizes: number[] = [];

    constructor(
        private readonly root: FileSystemDirectoryHandle,
        private readonly name: string,
        private readonly writable: FileSystemWritableFileStream
    ) {}

    async write(layer: Uint8Array): Promise<void> {
        if (this.closed) {
            throw new Error("Layer spool is already closed");
        }
        this.layerSizes.push(layer.byteLength);
        await this.writable.write(layer);
    }

    async closeForReplay(): Promise<void> {
        if (this.closed) {
            return;
        }
        await this.writable.close();
        this.closed = true;
    }

    async replay(consumer: (chunk: Uint8Array) => void | Promise<void>): Promise<void> {
        assertClosedForReplay(this.closed);
        const handle = await this.root.getFileHandle(this.name);
        const file = await handle.getFile();
        if (typeof file.stream !== "function") {
            await replayLayerBoundaries(new Uint8Array(await file.arrayBuffer()), this.layerSizes, consumer);
            return;
        }
        const streamReader = file.stream().getReader();
        try {
            let layerIndex = 0;
            let layerOffset = 0;
            let layerBuffer = this.newReplayLayerBuffer(layerIndex);
            while (true) {
                const { done, value } = await streamReader.read();
                if (done) {
                    break;
                }
                let chunkOffset = 0;
                while (chunkOffset < value.byteLength) {
                    if (layerIndex >= this.layerSizes.length) {
                        throw new Error("OPFS layer spool replay produced more bytes than expected");
                    }
                    const layerSize = this.layerSizes[layerIndex];
                    const needed = layerSize - layerOffset;
                    const take = Math.min(needed, value.byteLength - chunkOffset);
                    layerBuffer.set(value.subarray(chunkOffset, chunkOffset + take), layerOffset);
                    chunkOffset += take;
                    layerOffset += take;
                    if (layerOffset === layerSize) {
                        await consumer(layerBuffer);
                        layerIndex += 1;
                        layerOffset = 0;
                        layerBuffer = this.newReplayLayerBuffer(layerIndex);
                    }
                }
            }
            if (layerIndex !== this.layerSizes.length || layerOffset !== 0) {
                throw new Error("OPFS layer spool replay ended before all layers were read");
            }
        } finally {
            streamReader.releaseLock();
        }
    }

    async cleanup(): Promise<void> {
        if (!this.closed) {
            try {
                await this.writable.abort("cleanup");
            } catch {
                // Cleanup continues even when the writable is already closed.
            }
        }
        try {
            await this.root.removeEntry(this.name);
        } catch {
            // Temporary spool files may already have been removed by the browser.
        }
    }

    private newReplayLayerBuffer(layerIndex: number): Uint8Array {
        const size = this.layerSizes[layerIndex];
        return new Uint8Array(size ?? 0);
    }
}

async function replayLayerBoundaries(
    bytes: Uint8Array,
    layerSizes: readonly number[],
    consumer: (chunk: Uint8Array) => void | Promise<void>
): Promise<void> {
    let offset = 0;
    for (const size of layerSizes) {
        const end = offset + size;
        if (end > bytes.byteLength) {
            throw new Error("OPFS layer spool replay ended before all layers were read");
        }
        await consumer(bytes.subarray(offset, end));
        offset = end;
    }
    if (offset !== bytes.byteLength) {
        throw new Error("OPFS layer spool replay produced more bytes than expected");
    }
}

function assertClosedForReplay(closed: boolean): void {
    if (!closed) {
        throw new Error("Layer spool must be closed before replay");
    }
}

/**
 * @zh 创建 layer spool；支持 OPFS 时使用临时文件，否则退回内存。
 * @en Creates a layer spool; uses a temporary OPFS file when available and falls back to memory otherwise.
 */
export async function createLayerSpool(): Promise<LayerSpool> {
    const storage = (navigator as unknown as OptionalOpfsStorage).storage;
    if (typeof storage?.getDirectory !== "function") {
        return new MemoryLayerSpool();
    }

    let root: FileSystemDirectoryHandle | undefined;
    let name: string | undefined;
    let createdTempFile = false;

    try {
        root = await storage.getDirectory();
        name = `slicer-layer-spool-${Date.now()}-${Math.random().toString(36).slice(2)}.bin`;
        const handle = await root.getFileHandle(name, { create: true });
        createdTempFile = true;
        const writable = await handle.createWritable();
        return new OpfsLayerSpool(root, name, writable);
    } catch {
        if (root !== undefined && name !== undefined && createdTempFile) {
            try {
                await root.removeEntry(name);
            } catch {
                // Temporary spool cleanup is best-effort before falling back to memory.
            }
        }
        return new MemoryLayerSpool();
    }
}
