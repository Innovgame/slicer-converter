import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * @zh 合并 Tailwind class，后写的覆盖先写的。
 * @en Merges Tailwind classes, with later values winning.
 */
export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}
