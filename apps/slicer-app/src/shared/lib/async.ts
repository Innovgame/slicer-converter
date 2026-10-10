/**
 * @zh 限制并发度的 Promise 映射工具。
 * @en Promise map with concurrency limit.
 *
 * @zh 对数组中的每个元素执行异步操作，但同时最多只运行 `concurrency` 个操作。
 * @en Applies an async transformation to each array element, but limits concurrent executions.
 */
export async function mapWithConcurrency<TInput, TOutput>(
    items: TInput[],
    concurrency: number,
    mapper: (item: TInput, index: number) => Promise<TOutput>
): Promise<TOutput[]> {
    if (items.length === 0) {
        return [];
    }

    const limit = Math.max(1, concurrency);
    const results = new Array<TOutput>(items.length);
    let nextIndex = 0;

    const runWorker = async (): Promise<void> => {
        while (nextIndex < items.length) {
            const currentIndex = nextIndex;
            nextIndex += 1;
            results[currentIndex] = await mapper(items[currentIndex], currentIndex);
        }
    };

    await Promise.all(Array.from({ length: Math.min(limit, items.length) }, () => runWorker()));
    return results;
}
