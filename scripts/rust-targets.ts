export interface RustTargetSpec {
  alias: string;
  target: string;
  assetName: string;
  platform: NodeJS.Platform;
  arch: NodeJS.Architecture;
  exe: boolean;
}

export const RUST_TARGETS: RustTargetSpec[] = [
  {
    alias: "darwin-arm64",
    target: "aarch64-apple-darwin",
    assetName: "axhub-helpers-darwin-arm64",
    platform: "darwin",
    arch: "arm64",
    exe: false,
  },
  {
    alias: "darwin-amd64",
    target: "x86_64-apple-darwin",
    assetName: "axhub-helpers-darwin-amd64",
    platform: "darwin",
    arch: "x64",
    exe: false,
  },
  {
    alias: "linux-arm64",
    target: "aarch64-unknown-linux-gnu",
    assetName: "axhub-helpers-linux-arm64",
    platform: "linux",
    arch: "arm64",
    exe: false,
  },
  {
    alias: "linux-amd64",
    target: "x86_64-unknown-linux-gnu",
    assetName: "axhub-helpers-linux-amd64",
    platform: "linux",
    arch: "x64",
    exe: false,
  },
  {
    alias: "windows-amd64",
    target: "x86_64-pc-windows-msvc",
    assetName: "axhub-helpers-windows-amd64.exe",
    platform: "win32",
    arch: "x64",
    exe: true,
  },
];

export const RELEASE_ASSET_NAMES = RUST_TARGETS.map((target) => target.assetName);

export const rustTargetByAlias = (alias: string): RustTargetSpec | undefined =>
  RUST_TARGETS.find((target) => target.alias === alias);

export const rustTargetByTriple = (triple: string): RustTargetSpec | undefined =>
  RUST_TARGETS.find((target) => target.target === triple);

export const hostRustTarget = (): RustTargetSpec | undefined =>
  RUST_TARGETS.find((target) => target.platform === process.platform && target.arch === process.arch);

export const hostPrimaryBinaryName = (platform: NodeJS.Platform = process.platform): string =>
  platform === "win32" ? "axhub-helpers.exe" : "axhub-helpers";

export const cargoBinaryName = (target: Pick<RustTargetSpec, "exe">): string =>
  `axhub-helpers${target.exe ? ".exe" : ""}`;
