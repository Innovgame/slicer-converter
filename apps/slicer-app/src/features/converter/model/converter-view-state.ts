/**
 * @zh Converter UI 顶层状态的消息类型。
 * @en Message type for top-level converter UI status.
 */
export type StatusKind = "error" | "info" | "success" | "warning";

/**
 * @zh 展示在 banner 或 action 区域中的用户可见状态。
 * @en User-visible status shown in the banner or action area.
 */
export interface StatusState {
    type: StatusKind;
    message: string;
}

/**
 * @zh 当前 layer preview 的浏览器位图状态。
 * @en Browser bitmap state for the current layer preview.
 */
export interface PreviewImageState {
    bitmap: ImageBitmap;
    layerIndex: number;
    width: number;
    height: number;
}

/**
 * @zh 长耗时转换任务的 UI 进度。
 * @en UI progress for long-running conversion work.
 */
export interface ProgressState {
    label: string;
    percent: number;
}
