# Mobile Workbench Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this approved design inline. Track steps below. User has authorized implementation; continue without another approval gate.

**Goal:** Implement the approved mobile Preview / Settings interface with a full viewport canvas and overlay drawers.

**Architecture:** AppShell owns a local responsive interaction session. The route continues to own document and conversion state and renders its existing slots once. PreviewPanel consumes the shell session for mobile overlays; CSS preserves the canvas geometry when pages/drawers change.

**Tech Stack:** React 19, TypeScript, existing Lucide and i18next, CSS, Vitest / Testing Library.

**Spec:** `docs/superpowers/specs/2026-10-12-mobile-workbench-design.md`

## Global Constraints

- Mobile breakpoint: max-width 767px; desktop and tablet behavior retained.
- No new dependencies or changes to codecs, export matrices, metadata or conversion pipeline.
- Mounted source, preview and export slots preserve user and conversion state.
- Chinese/English translations and bilingual TypeScript JSDoc/test names.
- Work in the user's shared checkout; leave changes reviewable and uncommitted.

## Review Focus

- A hidden preview must not lose zoom/pan when its canvas reports zero bounds.
- Touch drag, pinch and keyboard changes must commit the intended layer.
- Settings navigation during conversion must preserve progress and cancellation.
- Drawers and pure canvas mode must leave a working keyboard/visible exit.
- Narrow/landscape phones must keep controls and long names reachable.

## Task 1: Responsive shell and settings

**Files:** Create `model/use-mobile-workbench.ts`, `ui/settings-panel.tsx`, `tests/unit/app/app-shell.test.tsx` under the converter/app paths. Modify `ui/app-shell.tsx`, `ui/file-summary-card.tsx`, `app/routes/converter-page.tsx`, locales and `styles.css`.

**Interfaces:** `MobileWorkbenchSession` exposes `isMobile`, `tab`, `drawer`, `immersive`, page/drawer/immersive actions. `AppShell.viewer` also accepts `(session: MobileWorkbenchSession) => ReactNode`. Source input accepts an optional shared ref. Settings consumes the existing help entry, theme provider and i18next.

- [x] Write behavior tests: mobile tab switching preserves canvas and form identity/value; drawers retain drafts; immersive has a restore/Escape exit; desktop keeps its slots and no mobile navigation; actual theme/language selectors update providers.
- [x] Run `pnpm --filter @slicer-converter/slicer-app test:unit tests/unit/app/app-shell.test.tsx` and observe missing mobile navigation failure (3 failed, desktop passed).
- [x] Implement session, persistent shell slots, standalone settings page, shared file input and localized labels.
- [x] Run the focused tests. Expected: PASS.

## Task 2: Preview controls, touch gestures and visual verification

**Files:** Modify `ui/preview-panel.tsx`, `ui/layer-viewport.ts`, `styles.css`, related app tests. Add a mobile browser regression in the existing e2e suite.

**Interfaces:** PreviewPanel accepts optional mobile session, source browse action and export summary. Mobile layer input uses the existing frame-coalesced interaction callback. LayerViewport preserves its displayed frame and anchor while pinching or visiting a hidden page.

- [x] Write failing tests for mobile horizontal layer commits and two-pointer pinch zoom/pan.
- [x] Run focused tests and confirm missing mobile slider / pinch behavior failure.
- [x] Implement the approved graphite preview, toolbar, compact controls and export/source overlays. Keep desktop controls and stable IDs.
- [x] Run app unit suite, typecheck, lint/format checks and production build. Expected: PASS.
- [x] Attempt browser validation and record the environment block. Mobile browser rendering remains unverified.
- [x] Review the final diff and update this plan with verification results.

## Implementation notes

- Export summary is derived in the existing converter facade, preserving the app's reviewed WASM import boundary.
- The existing tour now navigates to its mobile page/drawer before highlighting its target.
- Independent read-only review found a focused-canvas Escape propagation issue and truncated detail filenames. Escape capture/focus restoration has a RED→GREEN real-viewport regression test; detail names now wrap on phones.
- Browser validation attempt: Playwright could not start its development server (`EPERM`, localhost:8080). A direct localhost-only server escalation also failed because automatic approval review returned an unsupported-model 404. No browser verification is claimed; no network/approval bypass was attempted.

## Final verification

- `pnpm test:app`: 46 files, 153 tests passed.
- `pnpm check`: formatting, ESLint and TypeScript passed.
- `pnpm build:app`: production build passed; existing Vite browser-external and Node deprecation notices remain.
- `git diff --check`: passed.
- Added browser regressions for state preservation, narrow settings/localization and updated the mobile guide scenario; execution blocked before browser launch by the local-server permission issue above.
- All actionable review findings resolved. Changes remain uncommitted for review.
