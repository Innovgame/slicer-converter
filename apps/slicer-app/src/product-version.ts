import wasmPackage from "../../../packages/slicer-wasm/package.json" with { type: "json" };

/**
 * @zh 当前产品版本。构建时从发布包读入，与仓库 tag 和 README 同源。
 * @en Current product version. Read at build time from the published package, the same source as repository tags and the README.
 */
export const PRODUCT_VERSION: string = wasmPackage.version;

/**
 * @zh 构建时检出的 Git 提交前 8 位；无 Git 元数据时为空。
 * @en First eight characters of the checked-out Git commit at build time; empty without Git metadata.
 */
export const PRODUCT_BUILD_COMMIT: string = import.meta.env.VITE_BUILD_COMMIT;
