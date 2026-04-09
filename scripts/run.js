#!/usr/bin/env node
// npm bin entry — delegates to the native tinypng binary placed in bin/ by
// scripts/install.js. Pass through argv, stdio, and exit code.

const path = require("path");
const { spawnSync } = require("child_process");

const isWindows = process.platform === "win32";
const binaryName = "tinypng" + (isWindows ? ".exe" : "");
const binary = path.resolve(__dirname, "..", "bin", binaryName);

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  if (result.error.code === "ENOENT") {
    console.error(
      `tinypng-cli: binary not found at ${binary}.\n` +
        "The postinstall step may have failed. Try reinstalling:\n" +
        "  npm install -g tinypng-cli\n" +
        "Or download manually from https://github.com/aototo/tinypng-cli/releases"
    );
  } else {
    console.error(`tinypng-cli: failed to launch binary: ${result.error.message}`);
  }
  process.exit(1);
}

process.exit(result.status ?? 0);
