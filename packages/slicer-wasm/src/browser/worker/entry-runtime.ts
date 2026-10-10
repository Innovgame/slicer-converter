import { getSlicerWorkerRequestErrorContext, getWorkerErrorMessage, toSlicerWorkerErrorMessage } from "./errors.js";
import type { SlicerWorkerErrorMessage, SlicerWorkerRequest, SlicerWorkerResponse } from "./protocol.js";
import type { SlicerWorkerRuntime } from "./runtime.js";

/**
 * @zh Worker 全局对象所需的最小消息接口。
 * @en Minimal messaging interface required from the Worker global scope.
 */
export interface SlicerWorkerMessageScope {
    postMessage(message: SlicerWorkerResponse, transfer?: Transferable[]): void;
    onmessage: ((event: MessageEvent<SlicerWorkerRequest>) => void) | null;
}

/**
 * @zh 启动 Worker 入口所需的依赖。
 * @en Dependencies required to bootstrap the Worker entry runtime.
 */
export interface SlicerWorkerEntryRuntimeDeps {
    scope: SlicerWorkerMessageScope;
    runtime: SlicerWorkerRuntime;
    initWasm(): Promise<unknown>;
}

/**
 * @zh 绑定 Worker 初始化流程与消息分发。
 * @en Binds Worker initialisation and message dispatch.
 */
export function startSlicerWorkerEntryRuntime({ scope, runtime, initWasm }: SlicerWorkerEntryRuntimeDeps): void {
    let ready = false;

    void initWasm()
        .then(() => {
            ready = true;
            scope.postMessage({ type: "ready" } satisfies SlicerWorkerResponse);
        })
        .catch((error: unknown) => {
            const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                error: new Error(`WASM init failed: ${getWorkerErrorMessage(error)}`),
                code: "E_WORKER_INIT",
                stage: "worker-init",
                retryable: true
            });
            scope.postMessage(response);
        });

    scope.onmessage = (event: MessageEvent<SlicerWorkerRequest>): void => {
        const message = event.data;

        if (message.type === "cancel") {
            return;
        }

        if (!ready) {
            const context = getSlicerWorkerRequestErrorContext(message);
            const response: SlicerWorkerErrorMessage = toSlicerWorkerErrorMessage({
                error: new Error("Worker not ready"),
                code: "E_WORKER_NOT_READY",
                stage: "worker-init",
                retryable: true,
                index: context.index
            });
            scope.postMessage(response);
            return;
        }

        const handler = runtime.handlers[message.type] as (request: typeof message) => void;
        handler(message);
    };
}
