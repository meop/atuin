# winget WiX/.msi installer: notes (fork-only)

Goal: a real per-machine `.msi` beside the portable zip, as Starship and
PowerShell ship, for parity across the four shell-integrated tools (Atuin,
fzf, Yazi, zoxide).

## The problem

winget's portable install puts a symlink in `%LOCALAPPDATA%\Microsoft\WinGet\Links`,
created by whoever runs winget, normally a non-elevated terminal. Windows will
not let an elevated process follow a link that a less-privileged process
created (error 448, "The path cannot be traversed because it contains an
untrusted mount point"). An administrator's SSH session is elevated, so
`atuin` and its shell hook fail there; a standard user is unaffected.

## The change (commit "feat(dist): add a Windows MSI installer for atuin")

Redone on current main (2026-10-05) with cargo-dist 0.31.0, the pinned
version: `"msi"` added to `installers` in `dist-workspace.toml`;
`dist generate` created `crates/atuin/wix/main.wxs` and the
`[package.metadata.wix]` GUIDs. `release.yml` is unchanged. `atuin-server`
overrides `installers` to keep what it ships today (it has no shell
integration).

## Verified

- Fork smoke run (branch `winget-wix-smoke`, which also carries the
  `dist/windows-arm64-native` commit): https://github.com/meop/atuin/actions/runs/37311895415.
  `dist build` on windows-latest (x64) and natively on windows-11-arm
  (ARM64); each MSI installs `atuin.exe` of the right architecture, adds the
  machine PATH entry, runs locally and over SSH, uninstalls cleanly.
- Real machine (glass, Windows 11 26H2 x64), zoxide fork's
  `windows-msi-test/repro-ssh.ps1`: winget (`Atuinsh.Atuin`) portable install
  from the desktop session fails from an elevated SSH session (error 448);
  the MSI works. 2026-10-05.

## With the Windows ARM64 change

`windows-11-arm` has no WiX v3 (windows-latest has 3.14.1), and cargo-dist
does not install it, so the ARM64 MSI fails to build there
(https://github.com/meop/atuin/actions/runs/37308604531). Installing WiX
3.14.1's binaries (wixtoolset/wix3 release wix3141rtm, wix314-binaries.zip,
sha256 6ac824e1642d6f7277d0ed7ea09411a508f6116ba6fae0aa5f2c7daa2ff43d31) into
`<dir>\bin` and setting `WIX=<dir>\` fixes it; WiX 3's tools run on Windows
on Arm. In the real release, that step belongs in cargo-dist's
`github-build-setup`, as part of whichever of the two changes lands second.

## Before upstreaming

Atuin asks contributors to write PR descriptions themselves.
