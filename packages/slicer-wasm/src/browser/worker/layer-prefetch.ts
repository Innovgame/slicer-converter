import type { SlicerWorkerDispatchPayload } from "./dispatch.js";

/**
 * @zh 自适应预读窗口的配置项。
 * @en Configuration for the adaptive prefetch window.
 */
export interface SlicerLayerPrefetchOptions {
    /**
     * @zh 初始预读跨度（图层数）。
     * @en Initial prefetch window size, in layer count.
     */
    initialAhead: number;
    /**
     * @zh 最小跨度，默认与 `initialAhead` 相同。
     * @en Minimum window size; defaults to `initialAhead`.
     */
    minAhead?: number;
    /**
     * @zh 最大跨度，默认与 `initialAhead` 相同。
     * @en Maximum window size; defaults to `initialAhead`.
     */
    maxAhead?: number;
}

/**
 * @zh 创建具有自适应窗口的图层预读队列。
 * @en Creates a layer prefetch queue with an adaptive window.
 *
 * @zh 在 worker 忙碌时提前读入 N 个图层，将文件 I/O（`buildPayload` → `File.slice()`）
 * 与 worker 可用性解耦，这样当下一个 worker 变得空闲时数据已在内存中就绪。
 * @en Reads ahead N layers while workers are busy, decoupling file I/O
 * (`buildPayload` → `File.slice()`) from worker availability so data is
 * already in memory when the next worker becomes free.
 *
 * @zh 窗口大小在运行时自动调整：当消费方不得不等待（I/O 瓶颈）时扩大，适当提前时缩小。
 * @en Window size auto-adjusts at runtime: expands when the consumer has to
 * await (I/O bottleneck), shrinks when consistently ahead of schedule.
 *
 * @param totalLayers - @zh 文件总图层数 @en Total number of layers in the file
 * @param buildPayload - @zh 按索引从文件加载图层数据的工厂函数 @en Factory that loads layer data by index from the file
 * @param prefetch - @zh 预读配置：数字或 {@link SlicerLayerPrefetchOptions} @en Prefetch config: a number or {@link SlicerLayerPrefetchOptions}
 * @param signal - @zh 用于取消预读的 `AbortSignal` @en `AbortSignal` for cancellation
 */
export function createSlicerLayerPrefetch(
    totalLayers: number,
    buildPayload: (layerIndex: number) => Promise<SlicerWorkerDispatchPayload>,
    prefetch: number | SlicerLayerPrefetchOptions,
    signal: AbortSignal
): { next(): Promise<SlicerWorkerDispatchPayload>; dispose(): void } {
    interface PrefetchTask {
        promise: Promise<SlicerWorkerDispatchPayload>;
        ready: boolean;
    }

    const queue: PrefetchTask[] = [];
    let nextFetch = 0;
    let disposed = false;
    const { initialAhead, minAhead, maxAhead } = normalizeConfig(prefetch, totalLayers);
    let windowAhead = initialAhead;
    let readyBacklogStreak = 0;

    function enqueue(layerIndex: number): void {
        const task: PrefetchTask = {
            ready: false,
            promise: Promise.resolve(buildPayload(layerIndex)).then((payload) => {
                task.ready = true;
                return payload;
            })
        };
        void task.promise.catch(() => undefined);
        queue.push(task);
    }

    function adaptWindow(consumed: PrefetchTask): void {
        if (!consumed.ready && queue.length === 0) {
            /**
             * @zh 消费方等待了 payload I/O，扩大窗口以隐藏读取延迟。
             * @en The consumer had to await payload I/O, so grow the window to hide read latency.
             */
            if (windowAhead < maxAhead) {
                windowAhead += 1;
            }
            readyBacklogStreak = 0;
            return;
        }

        /**
         * @zh 如果 queue 连续保持接近满载，说明可能预读过多，需要谨慎缩小窗口。
         * @en If the queue repeatedly stays nearly full while consuming, we are likely over-prefetching; shrink cautiously.
         */
        if (queue.length >= Math.max(1, windowAhead - 1) && windowAhead > minAhead) {
            readyBacklogStreak += 1;
            if (readyBacklogStreak >= 3) {
                windowAhead -= 1;
                readyBacklogStreak = 0;
            }
            return;
        }

        readyBacklogStreak = 0;
    }

    function fill(): void {
        while (!disposed && !signal.aborted && queue.length < windowAhead && nextFetch < totalLayers) {
            enqueue(nextFetch);
            nextFetch += 1;
        }
    }

    fill();

    return {
        dispose() {
            disposed = true;
            queue.length = 0;
        },
        async next(): Promise<SlicerWorkerDispatchPayload> {
            const task = queue.shift();
            if (task) {
                adaptWindow(task);
            }
            fill();
            if (!task) {
                throw new Error("Prefetch queue underflow: next() called more than totalLayers times");
            }
            return task.promise;
        }
    };
}

function normalizeConfig(
    prefetch: number | SlicerLayerPrefetchOptions,
    totalLayers: number
): {
    initialAhead: number;
    minAhead: number;
    maxAhead: number;
} {
    const parsed =
        typeof prefetch === "number"
            ? {
                  initialAhead: prefetch,
                  minAhead: 1,
                  maxAhead: Math.max(prefetch, prefetch * 4)
              }
            : {
                  initialAhead: prefetch.initialAhead,
                  minAhead: prefetch.minAhead ?? 1,
                  maxAhead: prefetch.maxAhead ?? Math.max(prefetch.initialAhead, prefetch.initialAhead * 4)
              };

    const maxByLayers = Math.max(1, totalLayers);
    const initialAhead = clamp(Math.trunc(parsed.initialAhead), 1, maxByLayers);
    const minAhead = clamp(Math.trunc(parsed.minAhead), 1, initialAhead);
    const maxAhead = clamp(Math.trunc(parsed.maxAhead), initialAhead, maxByLayers);

    return { initialAhead, minAhead, maxAhead };
}

function clamp(value: number, min: number, max: number): number {
    if (!Number.isFinite(value)) {
        return min;
    }
    return Math.min(Math.max(value, min), max);
}
