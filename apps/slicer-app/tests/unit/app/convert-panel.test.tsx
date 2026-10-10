import React from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { ParsedHeader } from "@innovgame/slicer-converter";
import { ConvertPanel } from "../../../src/features/converter/ui/convert-panel";
import { syncFormatFacts } from "@innovgame/slicer-converter";
import { ThemeProvider } from "../../../src/app/providers";
import { anycubicFormatFact, chituboxFormatFact, formatFact } from "../support/format-facts";

syncFormatFacts([
    chituboxFormatFact({
        name: "ctb",
        display_name: "Chitubox CTB v2",
        aliases: ["ctb", "ctb-v2"],
        extensions: ["ctb", "v2.ctb"],
        target_names: ["ctb", "ctb-v2", "v2.ctb"],
        ctb_version: 2
    }),
    anycubicFormatFact({
        machine_id: "PhotonZero",
        display_name: "Photon Zero",
        extension: "pw0",
        default_version: 1,
        supported_versions: [1],
        table_layout: "V1",
        resolution_x: 2560,
        resolution_y: 1440,
        display_width_mm: 68.04,
        display_height_mm: 120.96,
        machine_z_mm: 150
    })
]);

const parsedHeader = {
    format: "goo",
    metadata: {
        resolution_x: 1440,
        resolution_y: 2560,
        anti_alias_level: 1,
        encryption_key: 0
    },
    layers: [{ offset: 0, size: 10 }]
} satisfies ParsedHeader;

function renderConvertPanel(props: Partial<React.ComponentProps<typeof ConvertPanel>> = {}) {
    return render(
        React.createElement(
            ThemeProvider,
            null,
            React.createElement(ConvertPanel, {
                wasmReady: true,
                parsedHeader,
                status: { type: "success", message: "ok" },
                outputFileName: "sample",
                slicerConverterFormatId: "chitubox-manual",
                slicerConverterSuffixId: "ctb",
                slicerConverterVersionId: "ctb",
                converting: false,
                progress: null,
                onOutputFileNameChange: vi.fn(),
                onSelectSlicerConverterFormat: vi.fn(),
                onSelectSlicerConverterSuffix: vi.fn(),
                onSelectSlicerConverterVersion: vi.fn(),
                onConvert: vi.fn(),
                onCancel: vi.fn(),
                ...props
            })
        )
    );
}

describe("ConvertPanel / 转换面板", () => {
    it("renders unified filename and slicer-converter target selectors / 渲染统一文件名和 slicer-converter 目标选择", () => {
        const onOutputFileNameChange = vi.fn();
        renderConvertPanel({
            outputFileName: "plate",
            slicerConverterFormatId: "anycubic-manual",
            slicerConverterSuffixId: "pw0",
            slicerConverterVersionId: "1",
            onOutputFileNameChange
        });

        expect(screen.getByLabelText("File name")).toHaveAttribute("id", "slicerConverterFileName");
        expect(screen.getByLabelText("Format")).toHaveAttribute("id", "slicerConverterFormat");
        expect(screen.getByLabelText("Suffix")).toHaveAttribute("id", "slicerConverterSuffix");
        expect(screen.getByLabelText("Version")).toHaveAttribute("id", "slicerConverterVersion");
        expect(screen.getByText(".pw0")).toBeInTheDocument();
        expect(
            screen.getByLabelText("File name").compareDocumentPosition(screen.getByLabelText("Format")) &
                Node.DOCUMENT_POSITION_FOLLOWING
        ).not.toBe(0);

        fireEvent.change(screen.getByLabelText("File name"), { target: { value: "plate-copy" } });
        expect(onOutputFileNameChange).toHaveBeenCalledWith("plate-copy");
    });

    it("does not show Anycubic Auto in the target format list / 不 show Anycubic Auto in the target format list", () => {
        renderConvertPanel();

        expect(screen.queryByRole("option", { name: "Anycubic Auto" })).toBeNull();
        expect(screen.getByRole("option", { name: "Chitubox" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Anycubic" })).toBeInTheDocument();
    });

    it("hides version selector for targets without real versions / 无真实版本目标隐藏版本选择", () => {
        syncFormatFacts([
            formatFact({
                name: "goo",
                display_name: "Goo",
                aliases: ["goo"],
                extensions: ["goo"],
                default_extension: "goo",
                is_zip: false
            }),
            formatFact({
                name: "vdt",
                display_name: "VDT",
                aliases: ["vdt"],
                extensions: ["vdt"],
                default_extension: "vdt",
                is_zip: true
            })
        ]);

        renderConvertPanel({
            slicerConverterFormatId: "vdt",
            slicerConverterSuffixId: "vdt",
            slicerConverterVersionId: "default"
        });

        expect(screen.getByLabelText("Format")).toBeInTheDocument();
        expect(screen.getByLabelText("Suffix")).toBeInTheDocument();
        expect(screen.queryByLabelText("Version")).toBeNull();
        expect(screen.queryByRole("option", { name: "默认" })).toBeNull();
        expect(screen.queryByRole("option", { name: "default" })).toBeNull();
    });

    it("shows a geometry warning before convert / 转换前显示几何 warning", () => {
        renderConvertPanel({
            exportWarning:
                "Warning: Source machine ELEGOO Mars 5 Ultra geometry differs from target machine Photon Zero (PW0)."
        });

        expect(document.getElementById("exportGeometryWarning")).toHaveTextContent("Photon Zero");
        expect(screen.getByRole("button", { name: "Convert" })).toBeEnabled();
    });

    it("shows retry button when status is conversion error / shows retry button 当 status is 转换 错误", () => {
        renderConvertPanel({ status: { type: "error", message: "Convert error: boom" } });

        const retry = screen.getByRole("button", { name: "Retry failed conversion" });
        expect(retry).toBeVisible();
    });

    it("hides retry button when status is not error / hides retry button 当 status is not 错误", () => {
        renderConvertPanel();

        expect(screen.queryByRole("button", { name: "Retry failed conversion" })).toBeNull();
    });

    it("defaults Anycubic matrix suffix to the first format / Anycubic matrix suffix 默认使用 first format", () => {
        const onSelectSlicerConverterSuffix = vi.fn();
        syncFormatFacts([
            anycubicFormatFact({
                machine_id: "PhotonMonoX",
                display_name: "Photon Mono X",
                extension: "pwmx",
                default_version: 516,
                supported_versions: [1, 515, 516],
                table_layout: "V516",
                resolution_x: 3840,
                resolution_y: 2400,
                display_width_mm: 192,
                display_height_mm: 120,
                machine_z_mm: 245
            }),
            anycubicFormatFact({
                machine_id: "PhotonM3Premium",
                display_name: "Photon M3 Premium",
                extension: "pm3r",
                default_version: 517,
                supported_versions: [515, 516, 517],
                table_layout: "V517",
                resolution_x: 7680,
                resolution_y: 4320,
                display_width_mm: 218.88,
                display_height_mm: 123.12,
                machine_z_mm: 250
            })
        ]);

        renderConvertPanel({
            slicerConverterFormatId: "anycubic-manual",
            slicerConverterSuffixId: "",
            slicerConverterVersionId: "",
            onSelectSlicerConverterSuffix
        });

        const suffix = screen.getByLabelText("Suffix") as HTMLSelectElement;
        const version = screen.getByLabelText("Version") as HTMLSelectElement;
        expect(suffix).toBeInTheDocument();
        expect(suffix.value).toBe("pwmx");
        expect(version.value).toBe("516");
        expect(screen.queryByRole("option", { name: "Select an Anycubic machine" })).toBeNull();
        expect(screen.getByRole("option", { name: "pwmx" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "pm3r" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "515" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "516" })).toBeInTheDocument();
        expect(screen.queryByRole("option", { name: "Photon Mono M5 (PM5)" })).toBeNull();

        fireEvent.change(suffix, { target: { value: "pm3r" } });
        expect(onSelectSlicerConverterSuffix).toHaveBeenCalledWith("pm3r");
    });

    it("defaults Chitubox matrix selectors to the first format / Chitubox matrix selectors 默认使用 first format", () => {
        const onSelectSlicerConverterSuffix = vi.fn();
        const onSelectSlicerConverterVersion = vi.fn();
        syncFormatFacts([
            chituboxFormatFact({
                name: "ctb-v1",
                display_name: "Chitubox CTB v1",
                aliases: ["ctb-v1", "v1.ctb"],
                extensions: ["ctb", "v1.ctb"],
                target_names: ["ctb-v1", "v1.ctb"],
                ctb_version: 1,
                allowed_versions: [1],
                build_version: 1,
                layer_table_entry_size: 36
            }),
            chituboxFormatFact({
                name: "ctb",
                display_name: "Chitubox CTB v2",
                aliases: ["ctb", "ctb-v2"],
                extensions: ["ctb", "v2.ctb"],
                target_names: ["ctb", "ctb-v2", "v2.ctb"],
                ctb_version: 2,
                allowed_versions: [1, 2],
                build_version: 2,
                layer_table_entry_size: 36
            }),
            chituboxFormatFact({
                name: "ctb-v4",
                display_name: "Chitubox CTB v4",
                aliases: ["ctb-v4", "v4.ctb"],
                extensions: ["ctb", "v4.ctb"],
                target_names: ["ctb-v4", "v4.ctb"],
                ctb_version: 4,
                allowed_versions: [4],
                build_version: 4,
                layer_table_entry_size: 84
            }),
            chituboxFormatFact({
                name: "cbddlp-v1",
                display_name: "Chitubox CBDDLP v1",
                aliases: ["cbddlp-v1"],
                extensions: ["cbddlp"],
                target_names: ["cbddlp-v1"],
                kind: "cbddlp",
                ctb_version: 1,
                allowed_versions: [1],
                build_version: 1,
                layer_table_entry_size: 36
            }),
            chituboxFormatFact({
                name: "cbddlp",
                display_name: "Chitubox CBDDLP v2",
                aliases: ["cbddlp", "cbddlp-v2"],
                extensions: ["cbddlp"],
                target_names: ["cbddlp", "cbddlp-v2"],
                kind: "cbddlp",
                ctb_version: 2,
                allowed_versions: [2],
                build_version: 2,
                layer_table_entry_size: 36
            }),
            chituboxFormatFact({
                name: "gktwo-ctb-v1",
                display_name: "GKtwo CTB v1",
                aliases: ["gktwo-ctb-v1", "gktwo-v1.ctb"],
                extensions: ["gktwo-v1.ctb"],
                target_names: ["gktwo-ctb-v1", "gktwo-v1.ctb"],
                kind: "gktwo-ctb",
                ctb_version: 1,
                machine_profile: "gktwo",
                allowed_versions: [1],
                build_version: 1,
                layer_table_entry_size: 36
            }),
            chituboxFormatFact({
                name: "gktwo-ctb",
                display_name: "GKtwo CTB v4",
                aliases: ["gktwo-ctb", "gktwo.ctb"],
                extensions: ["ctb", "gktwo.ctb"],
                target_names: ["gktwo-ctb", "gktwo.ctb"],
                kind: "gktwo-ctb",
                ctb_version: 4,
                machine_profile: "gktwo",
                allowed_versions: [4],
                build_version: 4,
                layer_table_entry_size: 84
            }),
            chituboxFormatFact({
                name: "gktwo-ctb-v5",
                display_name: "GKtwo CTB v5",
                aliases: ["gktwo-ctb-v5", "gktwo-v5.ctb"],
                extensions: ["gktwo-v5.ctb"],
                target_names: ["gktwo-ctb-v5", "gktwo-v5.ctb"],
                kind: "gktwo-ctb",
                ctb_version: 5,
                machine_profile: "gktwo",
                allowed_versions: [5],
                build_version: 5,
                layer_table_entry_size: 84
            }),
            chituboxFormatFact({
                name: "chitubox-zip",
                display_name: "Chitubox Zip archive",
                aliases: ["chitubox-zip", "chituzip"],
                extensions: ["zip"],
                target_names: ["chitubox-zip"],
                kind: "zip"
            }),
            chituboxFormatFact({
                name: "phz",
                display_name: "Chitubox PHZ",
                aliases: ["phz"],
                extensions: ["phz"],
                target_names: ["phz"],
                kind: "phz",
                ctb_version: 2
            }),
            chituboxFormatFact({
                name: "fdg",
                display_name: "Voxelab FDG",
                aliases: ["fdg"],
                extensions: ["fdg"],
                target_names: ["fdg"],
                kind: "fdg",
                ctb_version: 2,
                machine_profile: "voxelab"
            }),
            chituboxFormatFact({
                name: "photons",
                display_name: "Anycubic PhotonS",
                aliases: ["photons", "photon-s"],
                extensions: ["photons"],
                target_names: ["photons", "photon-s"],
                kind: "photons",
                machine_profile: "photon-s"
            }),
            chituboxFormatFact({
                name: "encrypted-ctb-v4",
                display_name: "Chitubox CTB (Encrypted) v4",
                aliases: ["encrypted-ctb-v4", "encrypted-v4.ctb"],
                extensions: ["ctb", "encrypted-v4.ctb"],
                target_names: ["encrypted-ctb-v4", "encrypted-v4.ctb"],
                kind: "encrypted-ctb",
                ctb_version: 4
            }),
            chituboxFormatFact({
                name: "encrypted-ctb",
                display_name: "Chitubox CTB (Encrypted) v5",
                aliases: ["encrypted-ctb", "encrypted-ctb-v5", "encrypted.ctb", "encrypted-v5.ctb"],
                extensions: ["encrypted.ctb", "encrypted-v5.ctb", "ctb"],
                target_names: ["encrypted-ctb", "encrypted-ctb-v5", "encrypted.ctb", "encrypted-v5.ctb"],
                kind: "encrypted-ctb",
                ctb_version: 5
            })
        ]);

        renderConvertPanel({
            slicerConverterFormatId: "chitubox-manual",
            slicerConverterSuffixId: "",
            slicerConverterVersionId: "",
            onSelectSlicerConverterSuffix,
            onSelectSlicerConverterVersion
        });

        const suffix = screen.getByLabelText("Suffix") as HTMLSelectElement;
        const version = screen.getByLabelText("Version") as HTMLSelectElement;
        expect(suffix).toBeInTheDocument();
        expect(suffix.value).toBe("ctb");
        expect(version.value).toBe("ctb-v1");
        expect(screen.queryByRole("option", { name: "Select a Chitubox target" })).toBeNull();
        expect(screen.getByRole("option", { name: "Chitubox CTB" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Chitubox CBDDLP" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "GKtwo CTB" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Chitubox Zip archive" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Chitubox PHZ" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Voxelab FDG" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Anycubic PhotonS" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "Chitubox CTB (Encrypted)" })).toBeInTheDocument();
        expect(screen.queryByRole("option", { name: "Chitubox Zip archive catalog" })).toBeNull();
        expect(screen.getByRole("option", { name: "1" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "2" })).toBeInTheDocument();
        expect(screen.getByRole("option", { name: "4" })).toBeInTheDocument();
        expect(screen.queryByRole("option", { name: "v1" })).toBeNull();

        fireEvent.change(version, { target: { value: "ctb-v4" } });
        expect(onSelectSlicerConverterVersion).toHaveBeenCalledWith("ctb-v4");

        fireEvent.change(suffix, { target: { value: "cbddlp" } });
        expect(onSelectSlicerConverterSuffix).toHaveBeenCalledWith("cbddlp");

        fireEvent.change(suffix, { target: { value: "gktwo-ctb" } });
        expect(onSelectSlicerConverterSuffix).toHaveBeenCalledWith("gktwo-ctb");

        fireEvent.change(suffix, { target: { value: "zip" } });
        expect(onSelectSlicerConverterSuffix).toHaveBeenCalledWith("zip");
    });
});
