export const ANYCUBIC_BINARY_EXTENSIONS = [
    "pw0",
    "pws",
    "pwx",
    "dlp",
    "pwmo",
    "pwms",
    "pwma",
    "pwmx",
    "pmsq",
    "pm3",
    "pm3m",
    "dl2p",
    "pmx2",
    "pwmb",
    "pm3r",
    "pm3n",
    "pm4n",
    "px6s",
    "pm5",
    "pm5s",
    "m5sp"
] as const;

export const ANYCUBIC_ARCHIVE_EXTENSIONS = ["pm4u", "pm7", "pm7m", "pwsz", "pp1", "pp1m"] as const;

export const ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS = ANYCUBIC_BINARY_EXTENSIONS.filter(
    (extension) => extension !== "pm4n"
);

export const ANYCUBIC_UVTOOLS_STRICT_EXTENSION_SET: ReadonlySet<string> = new Set(
    ANYCUBIC_UVTOOLS_STRICT_EXTENSIONS
);
