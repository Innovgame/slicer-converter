import type { FileMetadata } from "@slicer-converter/wasm";

/**
 * @zh 将元数据中第一张缩略图渲染到抛入的 canvas 上。
 * @en Renders the first thumbnail from the metadata onto the provided canvas.
 *
 * @zh 若元数据中没有缩略图，和/container 将被隐藏。
 * @en Hides `container` when no thumbnails are present in the metadata.
 *
 * @param metadata - @zh 含缩略图的文件元数据 @en File metadata containing optional thumbnails
 * @param container - @zh 包装 canvas 的容器元素 @en Wrapper element for the canvas
 * @param canvas - @zh 要绘制缩略图的 canvas @en Canvas to draw the thumbnail on
 */
export function renderThumbnail(
    metadata: FileMetadata,
    container: HTMLElement | null,
    canvas: HTMLCanvasElement | null
): void {
    if (!container || !canvas) {
        return;
    }

    const thumbs = metadata.thumbnails;
    if (!thumbs || thumbs.length === 0) {
        container.classList.add("hidden");
        return;
    }

    const thumb = thumbs.reduce((a, b) => (a.width * a.height >= b.width * b.height ? a : b));
    canvas.width = thumb.width;
    canvas.height = thumb.height;

    const thumbContext = canvas.getContext("2d");
    if (!thumbContext) {
        return;
    }

    const imageData = thumbContext.createImageData(thumb.width, thumb.height);
    const rgba = thumb.pixels_rgba;
    imageData.data.set(rgba.length <= imageData.data.length ? rgba : rgba.subarray(0, imageData.data.length));
    thumbContext.putImageData(imageData, 0, 0);
    container.classList.remove("hidden");
}
