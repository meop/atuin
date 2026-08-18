# winget WiX/.msi installer — WIP notes

Goal: give atuin's winget manifest a real per-machine `.msi` installer
alongside the existing portable/zip one, matching starship's setup.
Motivation: winget's portable installers land as a symlink in the WinGet
Links folder, and Windows blocks following symlinks over SSH by default —
breaking `atuin` for anyone using it non-interactively, since its shell hook
(`atuin init <shell>`) runs on every session start and drives history search.

Reference implementation studied: starship/starship (`cargo wix` +
`install/windows/main.wxs`).

## What's done (commit 2c3b61d8, branch `winget-wix-installer`)
Key finding: atuin doesn't hand-roll its Windows release pipeline like
starship does — it already uses `cargo-dist` (`dist-workspace.toml`, pinned
`cargo-dist-version = "0.31.0"`), which has native msi installer support.
So instead of a custom `main.wxs` + workflow steps, this just turned that on:
- `dist-workspace.toml` — added `"msi"` to `installers`.
- `crates/atuin/Cargo.toml`, `crates/atuin-server/Cargo.toml` — added
  `[package.metadata.wix]` with fresh, dist-generated GUIDs (not reused from
  starship).
- `crates/atuin/wix/main.wxs`, `crates/atuin-server/wix/main.wxs` — new,
  tool-generated WiX templates (via `dist generate`, not hand-written).
- `.github/workflows/release.yml` — **not** modified; `dist generate`
  reported it already up to date, since the workflow reads installer config
  from `dist-workspace.toml` at runtime.

## Needs verification (none of this ran on a real Windows box)
- [ ] `dist generate` scaffolded an msi for **both** `atuin` (the CLI — the
      actual shell-integration concern) and `atuin-server` (daemon), since
      both are workspace members with a `[[bin]]`. The server almost
      certainly doesn't need one — check cargo-dist's docs for a per-package
      installer override to drop it, rather than hand-editing generated
      files (CI checks generated files stay in sync via `dist plan`).
- [ ] `license = false, eula = false` in both `[package.metadata.wix]`
      blocks is cargo-dist's default when it can't find a configured license
      path — atuin does have a root `LICENSE`; consider wiring it in.
- [ ] Cut a real release through CI (or run `dist build` on an actual
      Windows box) to confirm the wix build succeeds end to end and the
      resulting `.msi` actually installs and lands `atuin` on PATH.
- [ ] Confirm winget prefers the `.msi` over the `.zip` by default once both
      exist in a generated manifest (expected, per winget-cli's
      `InstallerTypeEnum` ranking Wix/Msi above Zip/Portable — but worth
      confirming against a real manifest).

## Not in scope here
Submitting the actual winget-pkgs manifest update — separate PR to
`microsoft/winget-pkgs` once the `.msi` is a real release asset.
