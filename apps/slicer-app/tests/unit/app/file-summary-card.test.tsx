import React from "react";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { syncFormatFacts } from "@innovgame/slicer-converter";

import { buildFilePickerAccept } from "../../../src/features/converter/ui/file-picker-accept";
import { FileSummaryCard } from "../../../src/features/converter/ui/file-summary-card";
import { ThemeProvider } from "../../../src/app/providers";
import { formatFact } from "../support/format-facts";

vi.mock("../../../src/features/preview", () => ({
    renderThumbnail: vi.fn()
}));

describe("FileSummaryCard / 文件摘要卡片", () => {
    it("advertises importable slicer extensions from wasm facts on the file input / advertises importable slicer extensions 来自 wasm facts on the 文件 input", () => {
        syncFormatFacts([
            formatFact({
                name: "ctb",
                display_name: "CTB",
                aliases: ["ctb"],
                extensions: ["ctb"],
                is_zip: false,
                capabilities: {
                    can_import: true,
                    can_export: true,
                    is_archive: false,
                    supports_preview: true,
                    supports_anti_alias: false,
                    supports_per_layer_settings: false,
                    requires_full_document_parse: false,
                    supports_encryption: false
                }
            }),
            formatFact({
                name: "futurefmt",
                display_name: "Future Format",
                aliases: ["futurefmt"],
                extensions: ["futurefmt"],
                is_zip: false,
                capabilities: {
                    can_import: true,
                    can_export: false,
                    is_archive: false,
                    supports_preview: true,
                    supports_anti_alias: false,
                    supports_per_layer_settings: false,
                    requires_full_document_parse: false,
                    supports_encryption: false
                }
            })
        ]);

        render(
            React.createElement(
                ThemeProvider,
                null,
                React.createElement(FileSummaryCard, {
                    sourceFile: null,
                    parsedHeader: null,
                    onFileSelect: vi.fn()
                })
            )
        );

        expect(screen.getByLabelText("Drop slicer file or browse")).toBeInTheDocument();
        expect(screen.getByText("Supports .ctb, .futurefmt or click to browse")).toBeInTheDocument();
        expect(document.getElementById("fileInput")).toHaveAttribute(
            "accept",
            buildFilePickerAccept([".ctb", ".futurefmt"])
        );
    });

    it("does not put dotted catalog extensions on the native file picker / 带点目录扩展名不进入原生文件选择器", () => {
        syncFormatFacts([
            formatFact({
                name: "ctb",
                aliases: ["ctb"],
                extensions: ["ctb", "v1.ctb", "gktwo.ctb"],
                is_zip: false
            }),
            formatFact({
                name: "goo",
                aliases: ["goo"],
                extensions: ["goo"],
                is_zip: false
            })
        ]);

        render(
            React.createElement(
                ThemeProvider,
                null,
                React.createElement(FileSummaryCard, {
                    sourceFile: null,
                    parsedHeader: null,
                    onFileSelect: vi.fn()
                })
            )
        );

        const accept = document.getElementById("fileInput")?.getAttribute("accept") ?? "";
        expect(accept).not.toContain("v1.ctb");
        expect(accept).not.toContain("gktwo.ctb");
        expect(screen.getByText(/\.v1\.ctb/)).toBeInTheDocument();
        expect(screen.getByText(/\.goo/)).toBeInTheDocument();
    });

    it("updates the drop zone prompt while dragging / updates the drop zone prompt 同时 dragging", () => {
        const onFileSelect = vi.fn();

        const view = render(
            React.createElement(
                ThemeProvider,
                null,
                React.createElement(FileSummaryCard, {
                    sourceFile: null,
                    parsedHeader: null,
                    onFileSelect
                })
            )
        );

        const scope = within(view.container);
        const [dropZone] = scope.getAllByLabelText("Drop slicer file or browse");
        const file = new File([new Uint8Array([1, 2, 3])], "sample.goo", { type: "application/octet-stream" });

        fireEvent.dragEnter(dropZone, { dataTransfer: { files: [file] } });
        expect(scope.getByText("Release to upload your slicer file")).toBeInTheDocument();

        fireEvent.dragLeave(dropZone);
        expect(scope.getByText("Drag and drop a slicer file here")).toBeInTheDocument();
        expect(onFileSelect).not.toHaveBeenCalled();
    });

    it("forwards browsed files to onFileSelect / forwards browsed 文件 to onFileSelect", () => {
        const onFileSelect = vi.fn();

        render(
            React.createElement(
                ThemeProvider,
                null,
                React.createElement(FileSummaryCard, {
                    sourceFile: null,
                    parsedHeader: null,
                    onFileSelect
                })
            )
        );

        const inputs = Array.from(document.querySelectorAll("input[type='file']")) as HTMLInputElement[];
        const input = inputs.at(-1);
        const file = new File([new Uint8Array([1, 2, 3])], "sample.goo", { type: "application/octet-stream" });

        expect(input).toBeDefined();
        fireEvent.change(input!, { target: { files: [file] } });

        expect(onFileSelect).toHaveBeenCalledWith(file);
    });
});
