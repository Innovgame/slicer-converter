# Mobile workbench design

Approved in conversation: `tabs-preview-redesign.png` and its interactive prototype.

At widths up to 767px, the app has Preview and Settings bottom tabs. Preview contains a full viewport canvas, a lightweight filename header, a touch toolbar, horizontal layer controls, and an export drawer. Settings is a separate page with real theme/language preferences, the existing guide, project link, and feedback links. File details open from the preview header; choosing a file uses the existing file input.

The source, preview canvas, and export form remain mounted when tabs and drawers change. Switching pages retains layer selection, zoom/pan, output choices, and an in-flight conversion. Drawers overlay the canvas without resizing it. Pure canvas mode hides the header, toolbar, layer controls, status and tabs; a visible restore button and Escape exit it. A new document exits pure canvas mode. Desktop and tablet keep their existing workbench layouts.

Preview uses the approved graphite canvas/panels and blue actions independently of the settings-page theme. Settings follows the real saved application theme. Labels use bundled Chinese/English translations. File names truncate in the header and remain readable in file details. Controls support touch, keyboard focus, safe area insets, narrow phones, and landscape. Errors and conversion status stay accessible, including after changing tabs.

No codec, export matrix, metadata, or conversion pipeline changes. No new dependencies. New TypeScript contracts use bilingual JSDoc; tests use English / Chinese names.

Validation: behavior tests for state retention, drawer/immersive navigation, real preferences, layer interaction and pinch zoom; existing app tests, TypeScript, lint/format, production build, and mobile browser inspection where environment permits.
