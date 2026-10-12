/**
 * @zh 读取文件选择器策略时使用的浏览器环境。
 * @en Browser environment used to choose the file-picker accept policy.
 */
export interface FilePickerEnvironment {
    readonly userAgent: string;
    readonly platform: string;
    readonly maxTouchPoints: number;
    readonly userAgentDataPlatform?: string;
}

const APPLE_PICKER_ACCEPT = "application/octet-stream";

/**
 * @zh 生成 `<input type="file">` 的 accept。
 * @en Builds the accept value for `<input type="file">`.
 *
 * @zh iOS、iPadOS 和 macOS 的系统文件框按 UTI 过滤。`.goo`、`.ctb` 没有系统 UTI，
 * 目录里的 `.v1.ctb` 又不是合法的文件扩展名；这两类 token 会让目标文件显示为不可选。
 * Apple 平台因此只接受 `application/octet-stream`（`public.data`）。其它平台仍按单段扩展名过滤，
 * 并丢掉带点扩展名，避免同一解析问题。不支持的文件仍会在解析阶段失败。
 * @en iOS, iPadOS, and macOS file dialogs filter by UTI. `.goo` and `.ctb` have no system UTI,
 * and catalog tokens such as `.v1.ctb` are not valid filename extensions; either token disables
 * those files. Apple platforms therefore accept only `application/octet-stream` (`public.data`).
 * Other platforms keep a single-segment extension filter and drop dotted extensions so the same
 * parser bug cannot apply. Unsupported files still fail during parse.
 */
export function buildFilePickerAccept(
    extensions: readonly string[],
    environment: FilePickerEnvironment = readFilePickerEnvironment()
): string | undefined {
    if (isAppleFilePicker(environment)) {
        return APPLE_PICKER_ACCEPT;
    }

    const safeExtensions = listPickerSafeExtensions(extensions);
    return safeExtensions.length > 0 ? safeExtensions.join(",") : undefined;
}

/**
 * @zh 读取当前浏览器的文件选择器环境；无 `navigator` 时返回空环境。
 * @en Reads the current browser file-picker environment, or an empty environment when `navigator` is missing.
 */
export function readFilePickerEnvironment(): FilePickerEnvironment {
    if (typeof navigator === "undefined") {
        return { userAgent: "", platform: "", maxTouchPoints: 0 };
    }

    const userAgentData = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData;
    return {
        userAgent: navigator.userAgent,
        platform: navigator.platform,
        maxTouchPoints: navigator.maxTouchPoints ?? 0,
        userAgentDataPlatform: userAgentData?.platform
    };
}

function isAppleFilePicker(environment: FilePickerEnvironment): boolean {
    if (/iPad|iPhone|iPod/.test(environment.userAgent)) {
        return true;
    }
    if (/^mac/i.test(environment.userAgentDataPlatform ?? "")) {
        return true;
    }
    if (/Mac/.test(environment.platform)) {
        return true;
    }
    return /Macintosh|Mac OS X/.test(environment.userAgent);
}

function listPickerSafeExtensions(extensions: readonly string[]): string[] {
    const safe = new Set<string>();
    for (const raw of extensions) {
        const token = raw.trim().toLowerCase();
        if (!token) {
            continue;
        }

        const dotted = token.startsWith(".") ? token : `.${token}`;
        const body = dotted.slice(1);
        if (!body || /[/*]/.test(body)) {
            continue;
        }

        const extension = body.includes(".") ? body.slice(body.lastIndexOf(".") + 1) : body;
        if (extension) {
            safe.add(`.${extension}`);
        }
    }

    return Array.from(safe).sort();
}
