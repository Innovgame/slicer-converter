import type { PatchableSlicerOutputSink } from "./sink.js";

interface FileSystemFileHandle {
    createWritable(): Promise<FileSystemWritableFileStream>;
}

interface FileSystemWritableFileStream {
    write(data: Uint8Array | Blob | string): Promise<void>;
    seek(position: number): Promise<void>;
    close(): Promise<void>;
    abort(reason?: unknown): Promise<void>;
}

interface ShowSaveFilePickerOptions {
    suggestedName?: string;
    types?: Array<{
        description?: string;
        accept: Record<string, string[]>;
    }>;
}

interface FileSystemAccessGlobal {
    showSaveFilePicker?: (opts?: ShowSaveFilePickerOptions) => Promise<FileSystemFileHandle>;
}

/**
 * @zh File System Access sink 的诊断事件。
 * @en Diagnostic event emitted while opening or finishing a File System Access sink.
 */
export type FileSystemOutputSinkDiagnostic =
    | { readonly type: "file-system-picker"; readonly suggestedName: string; readonly pickerWallMs: number }
    | {
          readonly type: "file-system-create-writable";
          readonly suggestedName: string;
          readonly createWritableWallMs: number;
      }
    | { readonly type: "file-system-close"; readonly closeWallMs: number };

/**
 * @zh 打开 File System Access sink 的可选配置。
 * @en Optional configuration for opening a File System Access sink.
 */
export interface OpenFileSystemSinkOptions {
    readonly description?: string;
    readonly now?: () => number;
    readonly onDiagnostic?: (diagnostic: FileSystemOutputSinkDiagnostic) => void;
}

class FileSystemOutputSink implements PatchableSlicerOutputSink {
    readonly streaming = true;
    private finished = false;
    private aborted = false;

    constructor(
        private readonly writable: FileSystemWritableFileStream,
        private readonly options: Required<Pick<OpenFileSystemSinkOptions, "now">> &
            Pick<OpenFileSystemSinkOptions, "onDiagnostic">
    ) {}

    async write(data: Uint8Array): Promise<void> {
        this.ensureActive();
        await this.writable.write(data);
    }

    async patchHeader(data: Uint8Array): Promise<void> {
        this.ensureActive();
        await this.writable.seek(0);
        await this.writable.write(data);
    }

    async finish(): Promise<null> {
        this.ensureActive();
        const startedAt = this.options.now();
        await this.writable.close();
        this.options.onDiagnostic?.({
            type: "file-system-close",
            closeWallMs: this.options.now() - startedAt
        });
        this.finished = true;
        return null;
    }

    async abort(reason?: string): Promise<void> {
        if (this.finished || this.aborted) {
            return;
        }
        this.aborted = true;
        try {
            await this.writable.abort(reason);
        } catch {
            // Abort is best-effort; callers already treat this sink as closed.
        }
    }

    private ensureActive(): void {
        if (this.finished) {
            throw new Error("FileSystemOutputSink: already finished");
        }
        if (this.aborted) {
            throw new Error("FileSystemOutputSink: already aborted");
        }
    }
}

/**
 * @zh 当前运行环境是否支持 File System Access 写入。
 * @en Whether the current runtime supports File System Access writes.
 */
export function isFileSystemWriteSupported(): boolean {
    return typeof getFileSystemAccessGlobal().showSaveFilePicker === "function";
}

/**
 * @zh 打开保存文件选择器并返回 streaming output sink；不支持或用户取消时返回 `null`。
 * @en Opens a save-file picker and returns a streaming output sink; returns `null` when unsupported or cancelled.
 */
export async function openFileSystemSink(
    suggestedName: string,
    mimeType: string,
    extensions: string[],
    options: OpenFileSystemSinkOptions = {}
): Promise<PatchableSlicerOutputSink | null> {
    const showSaveFilePicker = getFileSystemAccessGlobal().showSaveFilePicker;
    if (typeof showSaveFilePicker !== "function") {
        return null;
    }

    const now = options.now ?? (() => performance.now());
    try {
        const pickerStartedAt = now();
        const handle = await showSaveFilePicker({
            suggestedName,
            types: [{ description: options.description ?? "Slicer file", accept: { [mimeType]: extensions } }]
        });
        options.onDiagnostic?.({
            type: "file-system-picker",
            suggestedName,
            pickerWallMs: now() - pickerStartedAt
        });

        const writableStartedAt = now();
        const writable = await handle.createWritable();
        options.onDiagnostic?.({
            type: "file-system-create-writable",
            suggestedName,
            createWritableWallMs: now() - writableStartedAt
        });

        return new FileSystemOutputSink(writable, { now, onDiagnostic: options.onDiagnostic });
    } catch {
        return null;
    }
}

function getFileSystemAccessGlobal(): typeof globalThis & FileSystemAccessGlobal {
    return globalThis as typeof globalThis & FileSystemAccessGlobal;
}
