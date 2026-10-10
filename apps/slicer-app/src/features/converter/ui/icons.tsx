import {
    ArrowRightLeft,
    Bug,
    ChevronDown,
    ChevronUp,
    File,
    Grid2X2,
    Layers,
    Lightbulb,
    Maximize,
    Moon,
    RotateCcw,
    Scan,
    SlidersHorizontal,
    Sun,
    Upload,
    ZoomIn,
    ZoomOut,
    type LucideIcon,
    type LucideProps
} from "lucide-react";

function renderIcon(Component: LucideIcon, props: LucideProps) {
    return <Component aria-hidden="true" size={16} strokeWidth={1.5} {...props} />;
}

/** @zh 打开文件图标。 @en Open-file icon. */
export function UploadIcon(props: LucideProps) {
    return renderIcon(Upload, props);
}

/** @zh 源文件栏图标。 @en Source-column icon. */
export function FileIcon(props: LucideProps) {
    return renderIcon(File, props);
}

/** @zh 导出栏图标。 @en Export-column icon. */
export function ExportIcon(props: LucideProps) {
    return renderIcon(Upload, props);
}

/** @zh 目标格式分组图标。 @en Target-group icon. */
export function SlidersIcon(props: LucideProps) {
    return renderIcon(SlidersHorizontal, props);
}

/** @zh 转换动作图标。 @en Convert-action icon. */
export function ConvertIcon(props: LucideProps) {
    return renderIcon(ArrowRightLeft, props);
}

/** @zh 层叠图标。 @en Stacked-layers icon. */
export function LayersIcon(props: LucideProps) {
    return renderIcon(Layers, props);
}

/** @zh 浅色主题图标。 @en Light-theme icon. */
export function SunIcon(props: LucideProps) {
    return renderIcon(Sun, props);
}

/** @zh 深色主题图标。 @en Dark-theme icon. */
export function MoonIcon(props: LucideProps) {
    return renderIcon(Moon, props);
}

/** @zh 适应窗口图标。 @en Fit-window icon. */
export function MaximizeIcon(props: LucideProps) {
    return renderIcon(Maximize, props);
}

/** @zh 1:1 像素图标。 @en 1:1 pixel icon. */
export function ScanIcon(props: LucideProps) {
    return renderIcon(Scan, props);
}

/** @zh 放大图标。 @en Zoom-in icon. */
export function ZoomInIcon(props: LucideProps) {
    return renderIcon(ZoomIn, props);
}

/** @zh 缩小图标。 @en Zoom-out icon. */
export function ZoomOutIcon(props: LucideProps) {
    return renderIcon(ZoomOut, props);
}

/** @zh 像素网格图标。 @en Pixel-grid icon. */
export function GridIcon(props: LucideProps) {
    return renderIcon(Grid2X2, props);
}

/** @zh 向上微调图标。 @en Increment icon. */
export function ChevronUpIcon(props: LucideProps) {
    return renderIcon(ChevronUp, props);
}

/** @zh 向下微调图标。 @en Decrement icon. */
export function ChevronDownIcon(props: LucideProps) {
    return renderIcon(ChevronDown, props);
}

/** @zh 回到首层图标。 @en Reset-to-first-layer icon. */
export function RotateCcwIcon(props: LucideProps) {
    return renderIcon(RotateCcw, props);
}

/** @zh 项目仓库图标。 @en Project repository icon. */
export function GitHubIcon(props: LucideProps) {
    return (
        <svg
            aria-hidden="true"
            fill="none"
            height="16"
            stroke="currentColor"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="1.5"
            viewBox="0 0 24 24"
            width="16"
            {...props}
        >
            <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
            <path d="M9 18c-4.51 2-5-2-7-2" />
        </svg>
    );
}

/** @zh 缺陷反馈图标。 @en Bug-report icon. */
export function BugIcon(props: LucideProps) {
    return renderIcon(Bug, props);
}

/** @zh 功能建议图标。 @en Feature-request icon. */
export function IdeaIcon(props: LucideProps) {
    return renderIcon(Lightbulb, props);
}
