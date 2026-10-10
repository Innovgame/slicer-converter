import type { SlicerWorkerError, SlicerWorkerErrorMessage, SlicerWorkerRequest } from "./protocol.js";

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
}

/**
 * @zh 判断值是否为 package 结构化 worker 错误。
 * @en Checks whether a value is a package structured worker error.
 */
export function isSlicerWorkerError(value: unknown): value is SlicerWorkerError {
    return (
        isRecord(value) &&
        typeof value.code === "string" &&
        typeof value.message === "string" &&
        typeof value.stage === "string" &&
        typeof value.retryable === "boolean" &&
        typeof value.timestamp === "number"
    );
}

/**
 * @zh 提取可展示的 worker 错误消息。
 * @en Extracts a displayable worker error message.
 */
export function getWorkerErrorMessage(error: unknown): string {
    if (isSlicerWorkerError(error)) {
        return error.message;
    }

    if (error instanceof Error) {
        return error.message;
    }

    if (isRecord(error) && typeof error.message === "string") {
        return error.message;
    }

    return String(error);
}

/**
 * @zh 将 unknown 错误规范化为结构化 worker 错误。
 * @en Normalizes an unknown error into a structured worker error.
 */
export function toSlicerWorkerError(
    error: unknown,
    fallback: Omit<SlicerWorkerError, "message" | "timestamp"> & { timestamp?: number; message?: string }
): SlicerWorkerError {
    const record = isRecord(error) ? error : null;

    if (isSlicerWorkerError(error)) {
        return error;
    }

    return {
        code: typeof record?.code === "string" ? record.code : fallback.code,
        stage: typeof record?.stage === "string" ? record.stage : fallback.stage,
        retryable: typeof record?.retryable === "boolean" ? record.retryable : fallback.retryable,
        timestamp: typeof record?.timestamp === "number" ? record.timestamp : (fallback.timestamp ?? Date.now()),
        message:
            typeof record?.message === "string" ? record.message : (fallback.message ?? getWorkerErrorMessage(error)),
        format: typeof record?.format === "string" ? record.format : fallback.format,
        layerIndex: typeof record?.layerIndex === "number" ? record.layerIndex : fallback.layerIndex,
        detail: typeof record?.detail === "string" ? record.detail : fallback.detail
    };
}

/**
 * @zh 构建 worker error response。
 * @en Builds a worker error response.
 */
export function toSlicerWorkerErrorMessage(input: {
    error: unknown;
    code: string;
    stage: string;
    retryable: boolean;
    timestamp?: number;
    format?: string;
    layerIndex?: number;
    detail?: string;
    index?: number;
}): SlicerWorkerErrorMessage {
    return {
        type: "error",
        index: input.index,
        error: toSlicerWorkerError(input.error, {
            code: input.code,
            stage: input.stage,
            retryable: input.retryable,
            timestamp: input.timestamp,
            format: input.format,
            layerIndex: input.layerIndex,
            detail: input.detail
        })
    };
}

/**
 * @zh 判断错误是否可重试。
 * @en Checks whether an error is retryable.
 */
export function isRetryableWorkerError(error: unknown): boolean {
    return isSlicerWorkerError(error) && error.retryable;
}

/**
 * @zh 从 request 提取 worker 错误上下文。
 * @en Extracts worker error context from a request.
 */
export function getSlicerWorkerRequestErrorContext(
    request: SlicerWorkerRequest
): Pick<SlicerWorkerErrorMessage, "index"> {
    return {
        index: "index" in request ? request.index : undefined
    };
}
