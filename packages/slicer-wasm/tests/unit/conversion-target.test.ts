import { describe, expect, it } from "vitest";

import {
    type SlicerFormatId,
    asSlicerFormatId,
    buildSlicerOutputFilename,
    findSlicerExportTargetMatrixSelection,
    listAnycubicManualTargetOptions,
    listSlicerExportTargetMatrix,
    listExportTargetOptions,
    prepareSlicerExportTargetFromMatrix,
    resolveSlicerExportTarget,
    selectDefaultExportTarget,
    syncSupportedFormatInfosForValidation
} from "../../src/index.js";
import { IMPORT_ONLY_CAPABILITIES, formatFact } from "./support/format-facts.js";

describe("conversion target policy / 转换目标策略", () => {
    it("lists export-capable concrete targets and disables source / 列出可导出目标并禁用源格式", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", display_name: "VDT", aliases: ["vdt"], extensions: ["vdt"], is_zip: true }),
            formatFact({
                name: "fdg",
                display_name: "FDG",
                aliases: ["fdg"],
                extensions: ["fdg"],
                is_zip: false,
                capabilities: IMPORT_ONLY_CAPABILITIES
            })
        ]);

        expect(listExportTargetOptions(asSlicerFormatId("goo"))).toEqual([
            { value: "goo", label: "Goo", disabled: true },
            { value: "vdt", label: "VDT", disabled: false }
        ]);
    });

    it("rejects import-only concrete targets / 拒绝仅导入格式", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "fdg",
                display_name: "FDG",
                aliases: ["fdg"],
                extensions: ["fdg"],
                is_zip: false,
                capabilities: IMPORT_ONLY_CAPABILITIES
            })
        ]);

        expect(() =>
            resolveSlicerExportTarget({
                sourceFormat: asSlicerFormatId("goo"),
                selectedTarget: "fdg"
            })
        ).toThrow("Format fdg is not export-capable");
    });

    it("selects the first non-source export target / 选择第一个非源导出目标", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({ name: "vdt", display_name: "VDT", aliases: ["vdt"], extensions: ["vdt"], is_zip: true })
        ]);

        expect(selectDefaultExportTarget(asSlicerFormatId("goo"))).toBe("vdt");
    });

    it("builds output file names from resolved extension / 使用解析后的扩展名构建输出名", () => {
        expect(
            buildSlicerOutputFilename({
                sourceFileName: "plate.goo",
                requestedTargetFormat: "anycubic-manual",
                resolvedExtension: "pm3"
            })
        ).toBe("plate.pm3");
    });
});

describe("export target matrix / 导出目标矩阵", () => {
    it("groups direct export targets by format, default suffix, and default version / 按格式 默认后缀 默认版本分组直接导出目标", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "vdt",
                display_name: "VDT",
                aliases: ["vdt"],
                extensions: ["vdt"],
                default_extension: "vdt",
                is_zip: true
            }),
            formatFact({
                name: "fdg",
                display_name: "FDG",
                aliases: ["fdg"],
                extensions: ["fdg"],
                is_zip: false,
                capabilities: IMPORT_ONLY_CAPABILITIES
            })
        ]);

        expect(listSlicerExportTargetMatrix(asSlicerFormatId("goo"))).toEqual([
            {
                id: "vdt",
                name: "VDT",
                suffixes: [
                    {
                        id: "vdt",
                        name: "vdt",
                        versions: [
                            {
                                id: "default",
                                name: "默认",
                                target: {
                                    requestedTargetFormat: "vdt",
                                    resolvedTargetFormat: "vdt",
                                    resolvedExtension: undefined,
                                    resolvedAnycubicVersion: undefined,
                                    statusMessage: null
                                }
                            }
                        ]
                    }
                ]
            }
        ]);
    });

    it("groups Chitubox manual targets by suffix and version / 按后缀和版本分组 Chitubox 手动目标", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "ctb-v2",
                display_name: "Chitubox CTB v2",
                aliases: ["ctb-v2"],
                extensions: ["ctb"],
                default_extension: "ctb",
                is_zip: false,
                specifics: {
                    family: "chitubox",
                    kind: "ctb",
                    ctb_version: 2,
                    machine_profile: null,
                    allowed_magics: [],
                    allowed_versions: [],
                    build_version: null,
                    layer_table_entry_size: null
                }
            }),
            formatFact({
                name: "ctb-v3",
                display_name: "Chitubox CTB v3",
                aliases: ["ctb-v3"],
                extensions: ["ctb"],
                default_extension: "ctb",
                is_zip: false,
                specifics: {
                    family: "chitubox",
                    kind: "ctb",
                    ctb_version: 3,
                    machine_profile: null,
                    allowed_magics: [],
                    allowed_versions: [],
                    build_version: null,
                    layer_table_entry_size: null
                }
            })
        ]);

        const chitubox = listSlicerExportTargetMatrix(asSlicerFormatId("goo")).find(
            (option) => option.id === "chitubox-manual"
        );

        expect(chitubox).toEqual({
            id: "chitubox-manual",
            name: "Chitubox",
            suffixes: [
                {
                    id: "ctb",
                    name: "Chitubox CTB",
                    versions: [
                        {
                            id: "ctb-v2",
                            name: "2",
                            target: {
                                requestedTargetFormat: "chitubox-manual",
                                resolvedTargetFormat: "ctb-v2",
                                resolvedExtension: "ctb",
                                resolvedAnycubicVersion: undefined,
                                statusMessage: "Using manual Chitubox target: Chitubox CTB v2 (ctb)"
                            }
                        },
                        {
                            id: "ctb-v3",
                            name: "3",
                            target: {
                                requestedTargetFormat: "chitubox-manual",
                                resolvedTargetFormat: "ctb-v3",
                                resolvedExtension: "ctb",
                                resolvedAnycubicVersion: undefined,
                                statusMessage: "Using manual Chitubox target: Chitubox CTB v3 (ctb)"
                            }
                        }
                    ]
                }
            ]
        });
    });

    it("groups Anycubic manual targets by suffix / 按后缀分组 Anycubic 手动目标", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "pm3",
                display_name: "Photon Mono",
                aliases: ["pm3"],
                extensions: ["pm3"],
                default_extension: "pm3",
                is_zip: false,
                specifics: {
                    family: "anycubic",
                    machine_id: "PhotonMono",
                    display_name: "Photon Mono",
                    extension: "pm3" as SlicerFormatId,
                    default_version: 1,
                    supported_versions: [1],
                    table_layout: "V1",
                    resolution_x: 1620,
                    resolution_y: 2560,
                    display_width_mm: 68.04,
                    display_height_mm: 120.96,
                    machine_z_mm: 165
                }
            }),
            formatFact({
                name: "pmx2",
                display_name: "Photon Mono X2",
                aliases: ["pmx2"],
                extensions: ["pmx2"],
                default_extension: "pmx2",
                is_zip: false,
                specifics: {
                    family: "anycubic",
                    machine_id: "PhotonMonoX2",
                    display_name: "Photon Mono X2",
                    extension: "pmx2" as SlicerFormatId,
                    default_version: 515,
                    supported_versions: [515],
                    table_layout: "V515",
                    resolution_x: 4096,
                    resolution_y: 2560,
                    display_width_mm: 197.12,
                    display_height_mm: 122.88,
                    machine_z_mm: 200
                }
            })
        ]);

        const anycubic = listSlicerExportTargetMatrix(asSlicerFormatId("goo")).find(
            (option) => option.id === "anycubic-manual"
        );

        expect(anycubic).toEqual({
            id: "anycubic-manual",
            name: "Anycubic",
            suffixes: [
                {
                    id: "pm3",
                    name: "pm3",
                    defaultVersionId: "1",
                    versions: [
                        {
                            id: "1",
                            name: "1",
                            target: {
                                requestedTargetFormat: "anycubic-manual",
                                resolvedTargetFormat: "pm3",
                                resolvedExtension: "pm3",
                                resolvedAnycubicVersion: 1,
                                statusMessage: "Using manual Anycubic target: Photon Mono (PM3)"
                            }
                        }
                    ]
                },
                {
                    id: "pmx2",
                    name: "pmx2",
                    defaultVersionId: "515",
                    versions: [
                        {
                            id: "515",
                            name: "515",
                            target: {
                                requestedTargetFormat: "anycubic-manual",
                                resolvedTargetFormat: "pmx2",
                                resolvedExtension: "pmx2",
                                resolvedAnycubicVersion: 515,
                                statusMessage: "Using manual Anycubic target: Photon Mono X2 (PMX2)"
                            }
                        }
                    ]
                }
            ]
        });
    });

    it("expands Anycubic binary supported versions into matrix rows / 将 Anycubic binary supported versions 展开为矩阵版本行", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "pwms",
                display_name: "Photon Mono SE",
                aliases: ["pwms"],
                extensions: ["pwms"],
                default_extension: "pwms",
                is_zip: false,
                specifics: {
                    family: "anycubic",
                    machine_id: "PhotonMonoSE",
                    display_name: "Photon Mono SE",
                    extension: "pwms" as SlicerFormatId,
                    default_version: 516,
                    supported_versions: [1, 515, 516],
                    table_layout: "V516",
                    resolution_x: 2560,
                    resolution_y: 1620,
                    display_width_mm: 120.96,
                    display_height_mm: 68.04,
                    machine_z_mm: 160
                }
            })
        ]);

        const anycubic = listSlicerExportTargetMatrix(asSlicerFormatId("goo")).find(
            (option) => option.id === "anycubic-manual"
        );
        const pwms = anycubic?.suffixes.find((suffix) => suffix.id === "pwms");

        expect(pwms?.versions.map((version) => version.id)).toEqual(["1", "515", "516"]);
        expect(pwms?.versions.map((version) => version.name)).toEqual(["1", "515", "516"]);
        expect(pwms?.versions.map((version) => version.target.resolvedAnycubicVersion)).toEqual([1, 515, 516]);
        expect(pwms?.versions.map((version) => version.target.resolvedTargetFormat)).toEqual(["pwms", "pwms", "pwms"]);
        expect(pwms?.versions.map((version) => version.target.resolvedExtension)).toEqual(["pwms", "pwms", "pwms"]);

        expect(
            findSlicerExportTargetMatrixSelection({
                sourceFormat: asSlicerFormatId("goo"),
                formatId: "anycubic-manual",
                suffixId: "pwms",
                versionId: "515"
            })?.version
        ).toEqual(expect.objectContaining({ id: "515", target: expect.objectContaining({ resolvedAnycubicVersion: 515 }) }));
        expect(
            findSlicerExportTargetMatrixSelection({
                sourceFormat: asSlicerFormatId("goo"),
                formatId: "anycubic-manual",
                suffixId: "pwms"
            })?.version
        ).toEqual(expect.objectContaining({ id: "516", target: expect.objectContaining({ resolvedAnycubicVersion: 516 }) }));
    });

    it("deduplicates Anycubic archive aliases resolving to the same suffix and target / 去重解析到相同后缀和目标的 Anycubic archive aliases", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "pm5-archive",
                display_name: "Anycubic PM5 Archive",
                aliases: ["pm5"],
                extensions: ["pm5"],
                default_extension: "pm5",
                target_names: ["pm5-archive", "pm5"],
                is_zip: true,
                specifics: {
                    family: "anycubic-archive",
                    settings_manifest_entry: "settings.json",
                    layers_manifest_entry: "layers.json",
                    layer_entry_prefix: "layers/",
                    default_layer_image_extension: "png",
                    layer_image_extensions: ["png"],
                    required_manifest_entries: ["settings.json", "layers.json"]
                }
            })
        ]);

        const anycubic = listSlicerExportTargetMatrix(asSlicerFormatId("goo")).find(
            (option) => option.id === "anycubic-manual"
        );

        expect(anycubic?.suffixes).toEqual([
            {
                id: "pm5",
                name: "pm5",
                versions: [
                    {
                        id: "pm5-archive",
                        name: "默认",
                        target: {
                            requestedTargetFormat: "anycubic-manual",
                            resolvedTargetFormat: "pm5-archive",
                            resolvedExtension: "pm5",
                            resolvedAnycubicVersion: undefined,
                            statusMessage: "Using manual Anycubic target: Anycubic PM5 Archive (PM5)"
                        }
                    }
                ]
            }
        ]);
    });

    it("hides Anycubic archive identity from UI suffixes while preserving default extension / UI 后缀隐藏 Anycubic archive identity 但保留默认扩展名", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "anycubic-zip",
                display_name: "Anycubic ZIP",
                aliases: ["anycubic-zip", "pm4u", "pm7"],
                extensions: ["pm4u", "pm7"],
                default_extension: "pm4u",
                target_names: ["anycubic-zip", "pm4u", "pm7"],
                is_zip: true,
                specifics: {
                    family: "anycubic-archive",
                    settings_manifest_entry: "settings.json",
                    layers_manifest_entry: "layers.json",
                    layer_entry_prefix: "layers/",
                    default_layer_image_extension: "pw0Img",
                    layer_image_extensions: ["pw0Img"],
                    required_manifest_entries: ["settings.json", "layers.json"]
                }
            })
        ]);

        const anycubic = listSlicerExportTargetMatrix(asSlicerFormatId("goo")).find(
            (option) => option.id === "anycubic-manual"
        );

        expect(anycubic?.suffixes.map((suffix) => suffix.id)).toEqual(["pm4u", "pm7"]);
        expect(anycubic?.suffixes.flatMap((suffix) => suffix.versions).map((version) => version.target.resolvedExtension)).toEqual([
            "pm4u",
            "pm7"
        ]);
    });

    it("does not expose Anycubic archive identity as a manual UI option / 不把 Anycubic archive identity 暴露为手动 UI 选项", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "anycubic-zip",
                display_name: "Anycubic ZIP",
                aliases: ["anycubic-zip", "pm4u", "pm7"],
                extensions: ["pm4u", "pm7"],
                default_extension: "pm4u",
                target_names: ["anycubic-zip", "pm4u", "pm7"],
                is_zip: true,
                specifics: {
                    family: "anycubic-archive",
                    settings_manifest_entry: "settings.json",
                    layers_manifest_entry: "layers.json",
                    layer_entry_prefix: "layers/",
                    default_layer_image_extension: "pw0Img",
                    layer_image_extensions: ["pw0Img"],
                    required_manifest_entries: ["settings.json", "layers.json"]
                }
            })
        ]);

        expect(listAnycubicManualTargetOptions().map((option) => option.value)).toEqual(["pm4u", "pm7"]);
    });

    it("keeps Anycubic archive identity as a resolver compatibility alias / resolver 兼容 Anycubic archive identity 别名", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "anycubic-zip",
                display_name: "Anycubic ZIP",
                aliases: ["anycubic-zip", "pm4u", "pm7"],
                extensions: ["pm4u", "pm7"],
                default_extension: "pm4u",
                target_names: ["anycubic-zip", "pm4u", "pm7"],
                is_zip: true,
                specifics: {
                    family: "anycubic-archive",
                    settings_manifest_entry: "settings.json",
                    layers_manifest_entry: "layers.json",
                    layer_entry_prefix: "layers/",
                    default_layer_image_extension: "pw0Img",
                    layer_image_extensions: ["pw0Img"],
                    required_manifest_entries: ["settings.json", "layers.json"]
                }
            })
        ]);

        expect(
            resolveSlicerExportTarget({
                sourceFormat: asSlicerFormatId("goo"),
                selectedTarget: "anycubic-manual",
                anycubicManualTarget: "anycubic-zip"
            })
        ).toEqual({
            requestedTargetFormat: "anycubic-manual",
            resolvedTargetFormat: "anycubic-zip",
            resolvedExtension: "pm4u",
            resolvedAnycubicVersion: undefined,
            statusMessage: "Using manual Anycubic target: Anycubic ZIP (PM4U)"
        });
    });

    it("falls back to the first available matrix selection / 回退到第一个可用矩阵选择", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({
                name: "ctb-v2",
                display_name: "Chitubox CTB v2",
                aliases: ["ctb-v2"],
                extensions: ["ctb"],
                default_extension: "ctb",
                is_zip: false,
                specifics: {
                    family: "chitubox",
                    kind: "ctb",
                    ctb_version: 2,
                    machine_profile: null,
                    allowed_magics: [],
                    allowed_versions: [],
                    build_version: null,
                    layer_table_entry_size: null
                }
            }),
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false })
        ]);

        expect(
            findSlicerExportTargetMatrixSelection({
                sourceFormat: null,
                formatId: "missing-format",
                suffixId: "missing-suffix",
                versionId: "missing-version"
            })
        ).toEqual({
            format: expect.objectContaining({ id: "chitubox-manual" }),
            suffix: expect.objectContaining({ id: "ctb" }),
            version: expect.objectContaining({ id: "ctb-v2" })
        });

        expect(
            findSlicerExportTargetMatrixSelection({
                sourceFormat: asSlicerFormatId("goo"),
                formatId: "goo"
            })
        ).toEqual({
            format: expect.objectContaining({ id: "chitubox-manual" }),
            suffix: expect.objectContaining({ id: "ctb" }),
            version: expect.objectContaining({ id: "ctb-v2" })
        });
    });

    it("prepares external app export target details from matrix ids / 通过矩阵 id 准备外部 app 导出目标详情", () => {
        syncSupportedFormatInfosForValidation([
            formatFact({ name: "goo", display_name: "Goo", aliases: ["goo"], extensions: ["goo"], is_zip: false }),
            formatFact({
                name: "ctb-v3",
                display_name: "Chitubox CTB v3",
                aliases: ["ctb-v3"],
                extensions: ["ctb"],
                default_extension: "ctb",
                is_zip: false,
                specifics: {
                    family: "chitubox",
                    kind: "ctb",
                    ctb_version: 3,
                    machine_profile: null,
                    allowed_magics: [],
                    allowed_versions: [],
                    build_version: null,
                    layer_table_entry_size: null
                }
            })
        ]);

        const prepared = prepareSlicerExportTargetFromMatrix({
            sourceFormat: asSlicerFormatId("goo"),
            fileNameStem: "plate",
            formatId: "chitubox-manual",
            suffixId: "ctb",
            versionId: "ctb-v3"
        });

        expect(prepared).toEqual({
            selection: {
                format: expect.objectContaining({ id: "chitubox-manual" }),
                suffix: expect.objectContaining({ id: "ctb" }),
                version: expect.objectContaining({ id: "ctb-v3" })
            },
            target: {
                requestedTargetFormat: "chitubox-manual",
                resolvedTargetFormat: "ctb-v3",
                resolvedExtension: "ctb",
                resolvedAnycubicVersion: undefined,
                statusMessage: "Using manual Chitubox target: Chitubox CTB v3 (ctb)"
            },
            extension: "ctb",
            fileName: "plate.ctb"
        });
        expect(
            prepareSlicerExportTargetFromMatrix({
                sourceFormat: asSlicerFormatId("goo"),
                fileNameStem: "plate",
                formatId: "missing",
                suffixId: "missing",
                versionId: "missing"
            })?.fileName
        ).toBe("plate.ctb");
    });
});
