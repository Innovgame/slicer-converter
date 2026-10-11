import wasmPackage from "../../../packages/slicer-wasm/package.json" with { type: "json" };

/**
 * @zh 当前产品版本。构建时从发布包读入，与仓库 tag 和 README 同源。
 * @en Current product version. Read at build time from the published package, the same source as repository tags and the README.
 */
export const PRODUCT_VERSION: string = wasmPackage.version;
