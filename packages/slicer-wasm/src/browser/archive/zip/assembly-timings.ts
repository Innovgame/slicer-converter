/**
 * @zh ZIP 组装过程的耗时诊断。
 * @en Timing diagnostics for ZIP assembly.
 */
export interface ZipAssemblyTimings {
    prepareWallMs: number;
    /**
     * @zh 仅衡量 stream assembler finalize 被调用到 sink 完成的墙上时间。
     * @en Wall-clock time spent inside stream assembler finalize until the sink is finished.
     */
    finalizeOnlyWallMs: number;
    totalWallMs: number;
    centralDirectoryWallMs?: number;
    centralDirectoryBytes?: number;
    centralDirectoryEntryCount?: number;
    sinkFinishWallMs?: number;
}
