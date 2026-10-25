#!/usr/bin/env node
// Build the dummy game runner for the current platform and copy it next to the
// Tauri resources, which is where the app expects it at runtime.
import { spawnSync } from "node:child_process";

const platform = process.platform;

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit", shell: false });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

if (platform === "win32") {
  run("pnpm", ["run", "build:runner:win"]);
  run("pnpm", ["run", "copy:runner:win"]);
} else if (platform === "darwin") {
  run("pnpm", ["run", "build:runner:darwin"]);
  run("pnpm", ["run", "copy:runner:darwin"]);
} else if (platform === "linux") {
  // build.sh prefers cargo and falls back to cc, so this works on machines
  // without a full Rust toolchain too.
  run("sh", ["./src-linux/build.sh", "--copy"]);
} else {
  console.error(`Unsupported platform for runner sync: ${platform}`);
  process.exit(1);
}
