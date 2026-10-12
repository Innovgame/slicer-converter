import { useEffect, useRef, useState } from "react";

import { openSlicerDocumentFile, parseBrowserFileHeader, type SlicerDocument } from "@innovgame/slicer-converter";
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
    const selectionGeneration = useRef(0);
    const mounted = useRef(true);
    const currentDocument = useRef<OpenedDocument | null>(null);

    useEffect(() => {
        mounted.current = true;
        return () => {
            mounted.current = false;
            selectionGeneration.current++;
            const previous = currentDocument.current;
            currentDocument.current = null;
            if (previous) void closeDocument(previous);
        };
    }, []);

    async function onFileSelect(file: File): Promise<void> {
        const generation = ++selectionGeneration.current;
        const isCurrent = (): boolean => mounted.current && generation === selectionGeneration.current;
        const previous = currentDocument.current;
        currentDocument.current = null;
        if (previous) void closeDocument(previous);
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

            if (!isCurrent()) {
                await closeDocument(opened);
                return;
            }
            currentDocument.current = opened;
            setDocument(opened);
            const metadata = opened.parsedHeader.metadata;
            input.setStatus({
                type: "success",
                message: `Parsed ${opened.parsedHeader.format.toUpperCase()}: ${opened.parsedHeader.layers.length} layers, ${metadata.resolution_x}x${metadata.resolution_y}`
            });
        } catch (error: unknown) {
            if (!isCurrent()) return;
            setDocument(null);
            input.setStatus({ type: "error", message: `Parse error: ${getErrorMessage(error)}` });
        }
    }

    return { document, sourceFile, onFileSelect };
}

async function closeDocument(document: OpenedDocument): Promise<void> {
    try {
        await document.close();
    } catch (error: unknown) {
        console.warn("[document-controller] document close failed", error);
    }
}
