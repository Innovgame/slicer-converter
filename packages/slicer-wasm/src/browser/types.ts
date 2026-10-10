declare const slicerFormatIdBrand: unique symbol;

/**
 * @zh 由 Rust `FormatSpec` 注册表导出的切片文件格式标识符。
 * @en Slicer file format identifier exported by the Rust `FormatSpec` registry.
 */
export type SlicerFormatId = string & { readonly [slicerFormatIdBrand]?: "SlicerFormatId" };

/**
 * @zh Anycubic family 中 Rust registry 支持的机器标识。
 * @en Machine identifier supported by the Rust registry for the Anycubic family.
 */
export type AnycubicMachineId =
    | "PhotonS"
    | "PhotonZero"
    | "PhotonX"
    | "PhotonUltra"
    | "PhotonD2"
    | "PhotonMono"
    | "PhotonMonoSE"
    | "PhotonMono4"
    | "PhotonMono4K"
    | "PhotonMonoX"
    | "PhotonMonoX2"
    | "PhotonMonoSQ"
    | "PhotonM3"
    | "PhotonM3Max"
    | "PhotonM3Premium"
    | "PhotonMono2"
    | "PhotonMonoM5"
    | "PhotonMonoM5s"
    | "PhotonMonoM5sPro"
    | "PhotonMonoX6KM3Plus"
    | "PhotonMonoX6Ks";

/**
 * @zh `AnycubicVersion` 类型契约，使用 UVTools 兼容的数字版本。
 * @en Type contract for `AnycubicVersion`, using UVTools-compatible numeric versions.
 */
export type AnycubicVersion = 1 | 515 | 516 | 517 | 518;

/**
 * @zh Anycubic 文件表布局版本。
 * @en Anycubic file table layout version.
 */
export type AnycubicTableLayout = "V1" | "V515" | "V516" | "V517" | "V518";

/**
 * @zh 格式族的 typed specifics，来自 Rust `FormatSpec` registry。
 * @en Typed format-family specifics exported from the Rust `FormatSpec` registry.
 */
export type SupportedFormatSpecifics =
    | { family: "none" }
    | {
          family: "chitubox";
          kind: string;
          ctb_version: number | null;
          machine_profile: string | null;
          allowed_magics: number[];
          allowed_versions: number[];
          build_version: number | null;
          layer_table_entry_size: number | null;
      }
    | {
          family: "anycubic";
          machine_id: AnycubicMachineId;
          display_name: string;
          extension: SlicerFormatId;
          default_version: AnycubicVersion;
          supported_versions: AnycubicVersion[];
          table_layout: AnycubicTableLayout;
          resolution_x: number;
          resolution_y: number;
          display_width_mm: number;
          display_height_mm: number;
          machine_z_mm: number;
      }
    | {
          family: "anycubic-archive";
          settings_manifest_entry: string;
          layers_manifest_entry: string;
          layer_entry_prefix: string;
          default_layer_image_extension: string;
          layer_image_extensions: string[];
          required_manifest_entries: string[];
      }
    | {
          family: "creality";
          variant: string;
          version: number;
          extension: SlicerFormatId;
          magic: string;
          checksum: string;
          layer_definition_size: number;
          layer_parameter_size: number;
          preview_count: number;
          supports_layer_encryption: boolean;
      };

/**
 * @zh Archive 输出计划中的 metadata entry，保留 Rust ABI 的 snake_case 字段。
 * @en Metadata entry in an archive output plan, preserving Rust ABI snake_case fields.
 */
export interface ArchiveMetadataEntryPlan {
    name: string;
    data: number[];
    compression_method: 0 | 8;
}

/**
 * @zh Archive 输出计划中的 layer entry 描述，保留 Rust ABI 的 snake_case 字段。
 * @en Layer entry descriptor in an archive output plan, preserving Rust ABI snake_case fields.
 */
export interface ArchiveLayerEntryPlan {
    index: number;
    name: string;
    compression_method: 0 | 8;
}

/**
 * @zh 切片文件中单个图层的字节偏移量与大小。
 * @en Byte offset and size of a single layer within the slicer file.
 *
 * @zh JS 端使用这些值调用 `File.slice(offset, offset + size)` 读取单层数据。
 * @en The JS side uses these values to call `File.slice(offset, offset + size)`
 * to read per-layer data on demand.
 */
export interface LayerOffset {
    offset: number;
    size: number;
}

/**
 * @zh 嵌入文件中的缩略图图像。
 * @en Thumbnail image embedded in the slicer file.
 */
export interface Thumbnail {
    width: number;
    height: number;
    pixels_rgba: Uint8Array;
}

/**
 * @zh 由 Rust 注册表导出的格式元数据。
 * @en Format metadata exported from the Rust registry.
 */
export interface SupportedFormatInfo {
    name: string;
    display_name: string;
    family: string;
    profile_id: string;
    aliases: string[];
    extensions: string[];
    default_extension: string;
    target_names: string[];
    is_zip: boolean;
    capabilities: SupportedFormatCapabilities;
    specifics: SupportedFormatSpecifics;
}

/**
 * @zh Rust `FormatSpec` 导出的格式能力。
 * @en Format capabilities exported from the Rust `FormatSpec` registry.
 */
export interface SupportedFormatCapabilities {
    can_import: boolean;
    can_export: boolean;
    is_archive: boolean;
    supports_preview: boolean;
    supports_anti_alias: boolean;
    supports_per_layer_settings: boolean;
    supports_encryption: boolean;
    requires_full_document_parse: boolean;
}

/**
 * @zh 预览专用图层解码结果，像素已在 WASM 侧按目标尺寸降采样。
 * @en Preview-oriented layer decode result with pixels already downsampled in WASM.
 */
export interface DecodedPreviewLayer {
    pixels: Uint8Array;
    width: number;
    height: number;
}

/**
 * @zh 导出诊断报告；warning 不会阻止转换。
 * @en Export diagnostic report; warnings do not block conversion.
 */
export interface ExportDiagnosticReport {
    warnings: ExportDiagnosticWarning[];
}

/**
 * @zh 非致命导出 warning，由 Rust format registry 生成。
 * @en Non-fatal export warning generated by the Rust format registry.
 */
export interface ExportDiagnosticWarning {
    code: string;
    severity: "warning";
    message: string;
    source: ExportMachineSnapshot;
    target: ExportMachineSnapshot;
}

/**
 * @zh 诊断中用于展示和测试的机器几何快照。
 * @en Machine geometry snapshot used by diagnostics for display and tests.
 */
export interface ExportMachineSnapshot {
    format: SlicerFormatId | null;
    display_name: string | null;
    extension: SlicerFormatId | null;
    resolution_x: number;
    resolution_y: number;
    display_width_mm: number;
    display_height_mm: number;
}

/**
 * @zh 跨格式通用的切片文件元数据，是 CTB / SL1 / GOO 等格式元数据字段的最小公共子集。
 * @en Universal slicer file metadata — the common denominator across CTB, SL1,
 * GOO, and other supported formats. All fields are optional since different
 * formats expose different subsets.
 */
export interface FileMetadata {
    resolution_x?: number;
    resolution_y?: number;
    display_width_mm?: number;
    display_height_mm?: number;
    layer_count?: number;
    machine_name?: string;
    machine_z_mm?: number;
    layer_height_mm?: number;
    bottom_layer_count?: number;
    transition_layer_count?: number;
    exposure_time?: number;
    bottom_exposure_time?: number;
    light_off_delay?: number;
    bottom_light_off_delay?: number;
    light_pwm?: number;
    bottom_light_pwm?: number;
    mirror_x?: boolean;
    mirror_y?: boolean;
    lift_height?: number;
    lift_speed?: number;
    lift_height2?: number;
    lift_speed2?: number;
    retract_speed?: number;
    retract_height2?: number;
    retract_speed2?: number;
    bottom_lift_height?: number;
    bottom_lift_speed?: number;
    wait_time_before_cure?: number;
    wait_time_after_cure?: number;
    wait_time_after_lift?: number;
    anti_alias_level?: number;
    encryption_key?: number;
    print_time_sec?: number;
    volume_ml?: number;
    weight_g?: number;
    cost?: number;
    thumbnails?: Thumbnail[];
}

/**
 * @zh `parseBrowserFileHeader` 的解析结果，包含格式标识、通用元数据以及所有图层的偏移信息。
 * @en Result returned by `parseBrowserFileHeader`. Contains the detected format
 * identifier, universal metadata, and the byte-offset table for every layer.
 */
export interface ParsedHeader {
    format: SlicerFormatId;
    metadata: FileMetadata;
    layers: LayerOffset[];
}
