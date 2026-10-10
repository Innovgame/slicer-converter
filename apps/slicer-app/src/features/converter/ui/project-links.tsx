import { BugIcon, GitHubIcon, IdeaIcon } from "./icons";
import { PROJECT_LINKS } from "./project-link-targets";

const LINK_ICONS = {
    github: GitHubIcon,
    bug: BugIcon,
    idea: IdeaIcon
} as const;

/**
 * @zh 顶栏项目与反馈链接。收在一条工具槽里，新标签页打开，不改变当前转换。
 * @en Header links for the project and feedback. They sit in one tool well, open in a new tab, and do not change the current conversion.
 */
export function ProjectLinks() {
    return (
        <nav className="sc-project-links" aria-label="Project">
            {PROJECT_LINKS.map((link) => {
                const LinkIcon = LINK_ICONS[link.id];
                return (
                    <a
                        key={link.href}
                        className="sc-project-link"
                        href={link.href}
                        target="_blank"
                        rel="noreferrer noopener"
                        aria-label={link.ariaLabel}
                        title={link.label}
                    >
                        <LinkIcon />
                        <span className="sc-project-link-label">{link.label}</span>
                        <span className="sc-project-link-short">{link.shortLabel}</span>
                    </a>
                );
            })}
        </nav>
    );
}
