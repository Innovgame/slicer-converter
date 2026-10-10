/**
 * @zh 按图层索引顺序消费输出字节的队列契约。
 * @en Queue contract for consuming output bytes in layer-index order.
 */
export interface OrderedLayerQueue {
    push(index: number, bytes: Uint8Array): void;
    fail(error: Error): void;
    consume(): AsyncGenerator<Uint8Array, void, void>;
}

/**
 * @zh 创建一个有序图层队列，允许并发不按序推入的图层按正确顺序消费。
 * @en Creates an ordered layer queue that allows concurrently pushed out-of-order layers to be consumed in order.
 *
 * @param totalLayers - @zh 文件总图层数 @en Total number of layers in the file
 */
export function createOrderedLayerQueue(totalLayers: number): OrderedLayerQueue {
    const pending = new Array<Uint8Array | null>(totalLayers).fill(null);
    const consumed = new Array<boolean>(totalLayers).fill(false);
    let nextIndex = 0;
    let notify: (() => void) | null = null;
    let failure: Error | null = null;

    function waitForNextChunk(): Promise<void> {
        return new Promise((resolve) => {
            notify = resolve;
        });
    }

    function failQueue(error: Error): void {
        failure = error;
        notify?.();
        notify = null;
    }

    function assertCanPush(index: number): void {
        if (failure) {
            throw failure;
        }
        if (!Number.isInteger(index) || index < 0 || index >= totalLayers) {
            const error = new Error(`Invalid layer index ${index}; expected an integer from 0 to ${totalLayers - 1}`);
            failQueue(error);
            throw error;
        }
        if (consumed[index]) {
            const error = new Error(`Layer index ${index} was already consumed`);
            failQueue(error);
            throw error;
        }
        if (pending[index] !== null) {
            const error = new Error(`Layer index ${index} is already pending`);
            failQueue(error);
            throw error;
        }
    }

    return {
        push(index: number, bytes: Uint8Array) {
            assertCanPush(index);
            pending[index] = bytes;
            notify?.();
            notify = null;
        },
        fail(error: Error) {
            failQueue(error);
        },
        async *consume() {
            while (nextIndex < totalLayers) {
                if (failure) {
                    throw failure;
                }

                const chunk = pending[nextIndex];
                if (!chunk) {
                    await waitForNextChunk();
                    continue;
                }

                pending[nextIndex] = null;
                consumed[nextIndex] = true;
                nextIndex += 1;
                yield chunk;
            }
        }
    };
}
