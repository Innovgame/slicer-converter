import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

interface TypeContract {
    rustFile: string;
    rustStruct: string;
    tsInterface: string;
    tsTypeOverrides?: Record<string, readonly string[]>;
}

interface FormatSpecificsContract {
    rustFile: string;
    rustStruct: string;
    tsFamily: string;
    tsTypeOverrides?: Record<string, readonly string[]>;
}

const testDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(testDir, "../..");
const workspaceRoot = path.resolve(packageRoot, "../..");

const rustWasmApiSource = readFileSync(path.join(workspaceRoot, "rust/crates/wasm/src/wasm_api.rs"), "utf8");
const rustCatalogSource = readFileSync(path.join(workspaceRoot, "rust/crates/core/src/formats/catalog.rs"), "utf8");
const rustCapabilitiesSource = readFileSync(
    path.join(workspaceRoot, "rust/crates/core/src/formats/common/capabilities.rs"),
    "utf8"
);
const rustAnycubicSpecificsSource = readFileSync(
    path.join(workspaceRoot, "rust/crates/core/src/formats/anycubic/specifics.rs"),
    "utf8"
);
const rustChituboxSpecificsSource = readFileSync(
    path.join(workspaceRoot, "rust/crates/core/src/formats/chitubox/specifics.rs"),
    "utf8"
);
const rustCrealitySpecificsSource = readFileSync(
    path.join(workspaceRoot, "rust/crates/core/src/formats/creality/specifics.rs"),
    "utf8"
);
const tsTypesSource = readFileSync(path.join(packageRoot, "src/browser/types.ts"), "utf8");

const rustSources: Record<string, string> = {
    "formats/catalog.rs": rustCatalogSource,
    "formats/anycubic/specifics.rs": rustAnycubicSpecificsSource,
    "formats/chitubox/specifics.rs": rustChituboxSpecificsSource,
    "formats/creality/specifics.rs": rustCrealitySpecificsSource,
    "formats/common/capabilities.rs": rustCapabilitiesSource,
    "wasm_api.rs": rustWasmApiSource
};

const contracts: TypeContract[] = [
    {
        rustFile: "formats/catalog.rs",
        rustStruct: "SupportedFormatInfo",
        tsInterface: "SupportedFormatInfo"
    },
    {
        rustFile: "formats/common/capabilities.rs",
        rustStruct: "FormatCapabilities",
        tsInterface: "SupportedFormatCapabilities"
    },
    {
        rustFile: "wasm_api.rs",
        rustStruct: "ArchiveMetadataEntryInfo",
        tsInterface: "ArchiveMetadataEntryPlan",
        tsTypeOverrides: {
            compression_method: ["0 | 8"]
        }
    },
    {
        rustFile: "wasm_api.rs",
        rustStruct: "ArchiveLayerEntryInfo",
        tsInterface: "ArchiveLayerEntryPlan",
        tsTypeOverrides: {
            compression_method: ["0 | 8"]
        }
    }
];

const specificsContracts: FormatSpecificsContract[] = [
    {
        rustFile: "formats/catalog.rs",
        rustStruct: "SupportedFormatSpecifics::Anycubic",
        tsFamily: "anycubic",
        tsTypeOverrides: {
            extension: ["string", "SlicerFormatId"],
            table_layout: ["string", "AnycubicTableLayout"]
        }
    },
    {
        rustFile: "formats/chitubox/specifics.rs",
        rustStruct: "ChituboxSpec",
        tsFamily: "chitubox",
        tsTypeOverrides: {
            kind: ["string"],
            allowed_magics: ["number[]"],
            allowed_versions: ["number[]"],
            build_version: ["number | null"],
            layer_table_entry_size: ["number | null"]
        }
    },
    {
        rustFile: "formats/creality/specifics.rs",
        rustStruct: "CrealityCxdLpSpec",
        tsFamily: "creality",
        tsTypeOverrides: {
            extension: ["string", "SlicerFormatId"]
        }
    }
];

function extractRustStructFields(source: string, structName: string): Map<string, string> {
    if (structName.includes("::")) {
        return extractRustEnumStructVariantFields(source, structName);
    }

    const match = new RegExp(`(?:pub(?:\\([^)]*\\))?\\s+)?struct\\s+${structName}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(
        source
    );

    if (!match) {
        throw new Error(`Rust struct ${structName} was not found`);
    }

    return new Map(
        match[1]
            .split("\n")
            .map((line) => line.match(/^\s*(?:pub(?:\([^)]*\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,]+),?\s*$/))
            .filter((field): field is RegExpMatchArray => field !== null)
            .map((field) => [field[1], normalizeRustType(field[2])])
    );
}

function extractRustEnumStructVariantFields(source: string, enumVariantName: string): Map<string, string> {
    const [enumName, variantName] = enumVariantName.split("::");
    const enumMatch = new RegExp(`pub\\s+enum\\s+${enumName}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(source);

    if (!enumMatch) {
        throw new Error(`Rust enum ${enumName} was not found`);
    }

    const variantMatch = new RegExp(`\\b${variantName}\\s*\\{([\\s\\S]*?)\\n\\s*\\},`).exec(enumMatch[1]);

    if (!variantMatch) {
        throw new Error(`Rust enum variant ${enumVariantName} was not found`);
    }

    return new Map(
        variantMatch[1]
            .split("\n")
            .map((line) => line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,]+),?\s*$/))
            .filter((field): field is RegExpMatchArray => field !== null)
            .map((field) => [field[1], normalizeRustType(field[2])])
    );
}

function extractTsInterfaceFields(source: string, interfaceName: string): Map<string, string> {
    const match = new RegExp(`export\\s+interface\\s+${interfaceName}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(source);

    if (!match) {
        throw new Error(`TypeScript interface ${interfaceName} was not found`);
    }

    return new Map(
        match[1]
            .split("\n")
            .map((line) => line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\?)?\s*:\s*(.+);\s*$/))
            .filter((field): field is RegExpMatchArray => field !== null)
            .map((field) => [field[1], normalizeTsType(field[2])])
    );
}

function extractTsFormatSpecificsFields(source: string, family: string): Map<string, string> {
    const branch = [...source.matchAll(/\|\s*\{([\s\S]*?)\n\s*\}/g)]
        .map((match) => match[1])
        .find((body) => new RegExp(`family:\\s*"${family}"`).test(body));

    if (!branch) {
        throw new Error(`TypeScript SupportedFormatSpecifics family ${family} was not found`);
    }

    const fields = new Map(
        branch
            .split("\n")
            .map((line) => line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\?)?\s*:\s*(.+);\s*$/))
            .filter((field): field is RegExpMatchArray => field !== null)
            .map((field) => [field[1], normalizeTsType(field[2])])
    );
    fields.delete("family");

    return fields;
}

function normalizeRustType(typeName: string): string {
    return typeName.replace(/\s+/g, "");
}

function normalizeTsType(typeName: string): string {
    return typeName.replace(/\s+/g, " ").trim();
}

function compatibleTsTypesFor(
    rustType: string,
    contract: Pick<TypeContract, "rustStruct" | "tsTypeOverrides">,
    fieldName: string
): readonly string[] {
    const override = contract.tsTypeOverrides?.[fieldName];
    if (override) {
        return override.map(normalizeTsType);
    }

    if (rustType === "String") {
        return ["string"];
    }

    if (rustType === "bool") {
        return ["boolean"];
    }

    if (rustType === "Vec<String>") {
        return ["string[]"];
    }

    if (rustType === "&'static[u32]") {
        return ["number[]"];
    }

    if (rustType === "Vec<u8>") {
        return ["number[]"];
    }

    if (/^(?:u|i)(?:8|16|32|64|128|size)$/.test(rustType) || /^(?:f32|f64)$/.test(rustType)) {
        return ["number"];
    }

    if (rustType === "formats::common::FormatCapabilities" || rustType === "common::FormatCapabilities") {
        return ["SupportedFormatCapabilities"];
    }

    if (rustType === "&'staticstr") {
        return ["string"];
    }

    if (rustType === "AnycubicMachineId" || rustType === "super::anycubic::AnycubicMachineId") {
        return ["AnycubicMachineId"];
    }

    if (rustType === "AnycubicVersion" || rustType === "super::anycubic::AnycubicVersion") {
        return ["AnycubicVersion"];
    }

    if (rustType === "Vec<AnycubicVersion>" || rustType === "Vec<super::anycubic::AnycubicVersion>") {
        return ["AnycubicVersion[]"];
    }

    if (rustType === "AnycubicTableLayout") {
        return ["AnycubicTableLayout"];
    }

    if (rustType === "Option<&'staticstr>") {
        return ["string | null"];
    }

    if (rustType === "Option<u32>") {
        return ["number | null"];
    }

    if (rustType === "SupportedFormatSpecifics") {
        return ["SupportedFormatSpecifics"];
    }

    throw new Error(`No TypeScript serialization mapping for ${contract.rustStruct}.${fieldName}: ${rustType}`);
}

function sortedFieldNames(fields: Map<string, string>): string[] {
    return [...fields.keys()].sort();
}

function extractRustEnumVariants(source: string, enumName: string): string[] {
    const match = new RegExp(`pub\\s+enum\\s+${enumName}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(source);
    if (!match) {
        throw new Error(`Rust enum ${enumName} was not found`);
    }

    return match[1]
        .split("\n")
        .map((line) => line.replace(/\/\/.*$/, "").trim())
        .filter((line) => /^[A-Za-z_][A-Za-z0-9_]*,?$/.test(line))
        .map((line) => line.replace(/,$/, ""))
        .map((variant) => serializedRustEnumVariant(enumName, variant));
}

function extractTsUnionValues(source: string, typeName: string): string[] {
    const match = new RegExp(`export\\s+type\\s+${typeName}\\s*=([\\s\\S]*?);`).exec(source);
    if (!match) {
        throw new Error(`TypeScript union ${typeName} was not found`);
    }

    if (typeName === "AnycubicVersion") {
        return [...match[1].matchAll(/\b\d+\b/g)].map((value) => value[0]);
    }

    return [...match[1].matchAll(/"([^"]+)"/g)].map((value) => value[1]);
}

function serializedRustEnumVariant(enumName: string, variant: string): string {
    if (enumName === "AnycubicVersion") {
        return variant === "V1" ? "1" : variant.slice(1);
    }

    return variant;
}

describe("Rust-to-TypeScript WASM type contracts / Rust 到 TypeScript 的 WASM 类型契约", () => {
    it.each(contracts)(
        "keeps $tsInterface fields in sync with Rust $rustStruct",
        ({ rustFile, rustStruct, tsInterface }) => {
            const rustFields = extractRustStructFields(rustSources[rustFile], rustStruct);
            const tsFields = extractTsInterfaceFields(tsTypesSource, tsInterface);

            expect(sortedFieldNames(tsFields)).toEqual(sortedFieldNames(rustFields));
        }
    );

    it.each(contracts)("keeps $tsInterface serialized field types compatible with Rust $rustStruct", (contract) => {
        const rustFields = extractRustStructFields(rustSources[contract.rustFile], contract.rustStruct);
        const tsFields = extractTsInterfaceFields(tsTypesSource, contract.tsInterface);

        for (const [fieldName, rustType] of rustFields) {
            const tsType = tsFields.get(fieldName);
            const compatibleTsTypes = compatibleTsTypesFor(rustType, contract, fieldName);

            expect(tsType, `${contract.tsInterface}.${fieldName}`).toBeDefined();
            expect(compatibleTsTypes, `${contract.tsInterface}.${fieldName}`).toContain(tsType);
        }
    });

    it.each(specificsContracts)(
        "keeps SupportedFormatSpecifics $tsFamily fields in sync with Rust $rustStruct",
        ({ rustFile, rustStruct, tsFamily }) => {
            const rustFields = extractRustStructFields(rustSources[rustFile], rustStruct);
            const tsFields = extractTsFormatSpecificsFields(tsTypesSource, tsFamily);

            expect(sortedFieldNames(tsFields)).toEqual(sortedFieldNames(rustFields));
        }
    );

    it.each(specificsContracts)(
        "keeps SupportedFormatSpecifics $tsFamily field types compatible with Rust $rustStruct",
        (contract) => {
            const rustFields = extractRustStructFields(rustSources[contract.rustFile], contract.rustStruct);
            const tsFields = extractTsFormatSpecificsFields(tsTypesSource, contract.tsFamily);

            for (const [fieldName, rustType] of rustFields) {
                const tsType = tsFields.get(fieldName);
                const compatibleTsTypes = compatibleTsTypesFor(rustType, contract, fieldName);

                expect(tsType, `SupportedFormatSpecifics.${contract.tsFamily}.${fieldName}`).toBeDefined();
                expect(compatibleTsTypes, `SupportedFormatSpecifics.${contract.tsFamily}.${fieldName}`).toContain(
                    tsType
                );
            }
        }
    );

    it("models typed format specifics on SupportedFormatInfo / models typed format specifics on SupportedFormatInfo 用例", () => {
        const supportedFormatFields = extractTsInterfaceFields(tsTypesSource, "SupportedFormatInfo");

        expect(supportedFormatFields.get("family")).toBe("string");
        expect(supportedFormatFields.get("profile_id")).toBe("string");
        expect(supportedFormatFields.get("default_extension")).toBe("string");
        expect(supportedFormatFields.get("target_names")).toBe("string[]");
        expect(supportedFormatFields.get("specifics")).toBe("SupportedFormatSpecifics");
        expect(tsTypesSource).toContain('family: "anycubic"');
        expect(tsTypesSource).toContain('family: "anycubic-archive"');
        expect(tsTypesSource).toContain('family: "chitubox"');
        expect(tsTypesSource).toContain('family: "creality"');
        expect(tsTypesSource).toContain('family: "none"');
    });

    it("models Anycubic archive specifics separately from machine specifics / models Anycubic archive specifics separately 来自 machine specifics", () => {
        expect(tsTypesSource).toContain('family: "anycubic-archive"');
        expect(tsTypesSource).toContain("settings_manifest_entry: string");
        expect(tsTypesSource).toContain("layers_manifest_entry: string");
        expect(tsTypesSource).toContain("layer_entry_prefix: string");
        expect(tsTypesSource).toContain("default_layer_image_extension: string");
        expect(tsTypesSource).toContain("layer_image_extensions: string[]");
        expect(tsTypesSource).toContain("required_manifest_entries: string[]");
    });

    it.each([
        ["AnycubicMachineId", "AnycubicMachineId"],
        ["AnycubicVersion", "AnycubicVersion"],
        ["AnycubicTableLayout", "AnycubicTableLayout"]
    ])("keeps TS %s union values in sync with Rust %s", (tsTypeName, rustEnumName) => {
        expect(extractTsUnionValues(tsTypesSource, tsTypeName)).toEqual(
            extractRustEnumVariants(rustAnycubicSpecificsSource, rustEnumName)
        );
    });
});
