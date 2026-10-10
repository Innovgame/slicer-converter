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
