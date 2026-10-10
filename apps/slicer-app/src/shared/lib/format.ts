/**
 * @zh 将毫秒数格式化为可读的时间字符串。
 * @en Formats a millisecond duration into a human-readable string.
 */
export function formatDuration(milliseconds: number): string {
    if (milliseconds < 1000) {
        return `${milliseconds.toFixed(1)}ms`;
    }

    return `${(milliseconds / 1000).toFixed(2)}s`;
}

/**
 * @zh 将字节数格式化为带单位后缀的可读字符串。
 * @en Formats a byte count into a human-readable string with a SI suffix.
 */
export function formatBytes(bytes: number): string {
    if (bytes < 1024) {
        return `${bytes} B`;
    }
    if (bytes < 1048576) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    }
    return `${(bytes / 1048576).toFixed(1)} MB`;
}
