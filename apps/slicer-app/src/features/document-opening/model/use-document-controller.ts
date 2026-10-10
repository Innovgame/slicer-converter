import { useState } from "react";

import { openSlicerDocumentFile, parseBrowserFileHeader, type SlicerDocument } from "@slicer-converter/wasm";
import { getErrorMessage } from "../../../shared/lib/errors";
import type { StatusState } from "../../converter/model/converter-view-state";

type OpenedDocument = SlicerDocument;

/**
 * @zh 管理 source file 的读取和 header 解析状态。
 * @en Manages source file loading and header parsing state.
 */
export function useDocumentController(input: { wasmReady: boolean; setStatus: (status: StatusState | null) => void }) {
    const [document, setDocument] = useState<OpenedDocument | null>(null);
    const [sourceFile, setSourceFile] = useState<File | null>(null);

    async function onFileSelect(file: File): Promise<void> {
        setSourceFile(file);
        setDocument(null);

        if (!input.wasmReady) {
            input.setStatus({ type: "error", message: "WASM not loaded - cannot parse file." });
            return;
        }

        try {
            const opened = await openSlicerDocumentFile({
                file,
                parseHeader: parseBrowserFileHeader
            });

            setDocument(opened);
            const metadata = opened.parsedHeader.metadata;
            input.setStatus({
                type: "success",
                message: `Parsed ${opened.parsedHeader.format.toUpperCase()}: ${opened.parsedHeader.layers.length} layers, ${metadata.resolution_x}x${metadata.resolution_y}`
            });
        } catch (error: unknown) {
            setDocument(null);
            input.setStatus({ type: "error", message: `Parse error: ${getErrorMessage(error)}` });
        }
    }

    return { document, sourceFile, onFileSelect };
}
