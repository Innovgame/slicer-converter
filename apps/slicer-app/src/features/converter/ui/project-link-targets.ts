/**
 * @zh 测试页顶栏外链。不进入文档状态。
 * @en External links in the test-page header. They do not enter document state.
 */
export const PROJECT_LINKS = [
    {
        id: "github",
        href: "https://github.com/Innovgame/slicer-converter",
        label: "GitHub",
        shortLabel: "GitHub",
        ariaLabel: "Open the slicer-converter GitHub repository"
    },
    {
        id: "bug",
        href: "https://github.com/Innovgame/slicer-converter/issues/new?template=bug_report.md",
        label: "Report a bug",
        shortLabel: "Bug",
        ariaLabel: "Report a bug"
    },
    {
        id: "idea",
        href: "https://github.com/Innovgame/slicer-converter/issues/new?template=feature_request.md",
        label: "Request a feature",
        shortLabel: "Idea",
        ariaLabel: "Request a feature"
    }
] as const;
