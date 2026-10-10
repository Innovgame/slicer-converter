import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

function Icon(props: IconProps) {
    return (
        <svg
            aria-hidden="true"
            fill="none"
            height="16"
            stroke="currentColor"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="2"
            viewBox="0 0 24 24"
            width="16"
            {...props}
        />
    );
}

/** @zh 打开文件图标。 @en Open-file icon. */
export function UploadIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M12 16V4" />
            <path d="m7 9 5-5 5 5" />
            <path d="M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
        </Icon>
    );
}

/** @zh 源文件栏图标。 @en Source-column icon. */
export function FileIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8Z" />
            <path d="M14 3v5h5" />
        </Icon>
    );
}

/** @zh 导出栏图标。 @en Export-column icon. */
export function ExportIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M12 15V3" />
            <path d="m7 8 5-5 5 5" />
            <path d="M4 15v3a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-3" />
        </Icon>
    );
}

/** @zh 目标格式分组图标。 @en Target-group icon. */
export function SlidersIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h10M18 18h2" />
            <circle cx="16" cy="6" r="2" />
            <circle cx="10" cy="12" r="2" />
            <circle cx="16" cy="18" r="2" />
        </Icon>
    );
}

/** @zh 转换动作图标。 @en Convert-action icon. */
export function ConvertIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M3 8h14l-3.5-3.5" />
            <path d="M21 16H7l3.5 3.5" />
        </Icon>
    );
}

/** @zh 层叠图标。 @en Stacked-layers icon. */
export function LayersIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M12 2 2 7l10 5 10-5-10-5Z" />
            <path d="m2 17 10 5 10-5" />
            <path d="m2 12 10 5 10-5" />
        </Icon>
    );
}

/** @zh 浅色主题图标。 @en Light-theme icon. */
export function SunIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <circle cx="12" cy="12" r="4" />
            <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
        </Icon>
    );
}

/** @zh 深色主题图标。 @en Dark-theme icon. */
export function MoonIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M12 3a7 7 0 1 0 9 9 9 9 0 1 1-9-9Z" />
        </Icon>
    );
}

/** @zh 适应窗口图标。 @en Fit-window icon. */
export function MaximizeIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M8 3H5a2 2 0 0 0-2 2v3M21 8V5a2 2 0 0 0-2-2h-3M3 16v3a2 2 0 0 0 2 2h3M16 21h3a2 2 0 0 0 2-2v-3" />
        </Icon>
    );
}

/** @zh 1:1 像素图标。 @en 1:1 pixel icon. */
export function ScanIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M3 7V5a2 2 0 0 1 2-2h2M17 3h2a2 2 0 0 1 2 2v2M21 17v2a2 2 0 0 1-2 2h-2M7 21H5a2 2 0 0 1-2-2v-2" />
            <path d="M7 12h10" />
        </Icon>
    );
}

/** @zh 放大图标。 @en Zoom-in icon. */
export function ZoomInIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5M11 8v6M8 11h6" />
        </Icon>
    );
}

/** @zh 缩小图标。 @en Zoom-out icon. */
export function ZoomOutIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5M8 11h6" />
        </Icon>
    );
}

/** @zh 像素网格图标。 @en Pixel-grid icon. */
export function GridIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <rect height="18" rx="2" width="18" x="3" y="3" />
            <path d="M3 9h18M3 15h18M9 3v18M15 3v18" />
        </Icon>
    );
}

/** @zh 向上微调图标。 @en Increment icon. */
export function ChevronUpIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="m6 14 6-6 6 6" />
        </Icon>
    );
}

/** @zh 向下微调图标。 @en Decrement icon. */
export function ChevronDownIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="m6 10 6 6 6-6" />
        </Icon>
    );
}

/** @zh 回到首层图标。 @en Reset-to-first-layer icon. */
export function RotateCcwIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
            <path d="M3 3v5h5" />
        </Icon>
    );
}

/** @zh 项目仓库图标。 @en Project repository icon. */
export function GitHubIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
            <path d="M9 18c-4.51 2-5-2-7-2" />
        </Icon>
    );
}

/** @zh 缺陷反馈图标。 @en Bug-report icon. */
export function BugIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M9 7.5 7.5 5" />
            <path d="M15 7.5 16.5 5" />
            <path d="M8 13h8" />
            <path d="M7 12.5a5 5 0 0 1 10 0V16a5 5 0 0 1-10 0Z" />
            <path d="M7 12H4" />
            <path d="M20 12h-3" />
            <path d="m7.2 16-2.2 2" />
            <path d="m16.8 16 2.2 2" />
        </Icon>
    );
}

/** @zh 功能建议图标。 @en Feature-request icon. */
export function IdeaIcon(props: IconProps) {
    return (
        <Icon {...props}>
            <path d="M9 18h6" />
            <path d="M10 21h4" />
            <path d="M12 3a6 6 0 0 0-3.4 10.9c.4.4.7 1 .8 1.6h5.2c.1-.6.4-1.2.8-1.6A6 6 0 0 0 12 3Z" />
        </Icon>
    );
}
