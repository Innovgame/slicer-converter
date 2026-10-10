import type { SlicerWorkerRequest } from "./protocol.js";

/**
 * @zh 带有 `busy` 标志的转换 Worker。
 * @en Conversion Worker extended with a `busy` flag.
 */
export interface ManagedSlicerWorker extends Worker {
    busy: boolean;
}

/**
 * @zh 将发往 Worker 的图层转换任务包。
 * @en Layer conversion task payload dispatched to a Worker.
 */
export interface SlicerWorkerDispatchPayload {
    request: SlicerWorkerRequest;
    transfer: ArrayBuffer;
}
