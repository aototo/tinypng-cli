#!/usr/bin/env node
// postinstall: download the matching prebuilt tinypng binary from GitHub Releases
// and place it into bin/ next to this package.
//
// Skipped automatically when running from a source checkout (e.g. during
// `cargo` development) — detected by the presence of ../Cargo.toml.

const fs = require("fs");
const os = require("os");
const path = require("path");
const https = require("https");
const { execSync } = require("child_process");

const VERSION = require("../package.json").version;
const REPO = "aototo/tinypng-cli";
const NAME = "tinypng";

// Dev-mode escape hatch: don't try to download when we're inside the source
// repo (Cargo.toml is a sibling of package.json).
const repoRoot = path.resolve(__dirname, "..");
if (fs.existsSync(path.join(repoRoot, "Cargo.toml"))) {
  console.log(
    "tinypng-cli: source checkout detected, skipping binary download."
  );
  process.exit(0);
}

// Also allow an explicit opt-out.
if (process.env.TINYPNG_CLI_SKIP_DOWNLOAD) {
  console.log(
    "tinypng-cli: TINYPNG_CLI_SKIP_DOWNLOAD set, skipping binary download."
  );
  process.exit(0);
}

const PLATFORM_TARGETS = {
  "darwin-arm64": {
    target: "aarch64-apple-darwin",
    ext: "tar.gz",
  },
  "darwin-x64": {
    target: "x86_64-apple-darwin",
    ext: "tar.gz",
  },
  "linux-x64": {
    target: "x86_64-unknown-linux-gnu",
    ext: "tar.gz",
  },
  "linux-arm64": {
    target: "aarch64-unknown-linux-gnu",
    ext: "tar.gz",
  },
  "win32-x64": {
    target: "x86_64-pc-windows-msvc",
    ext: "zip",
  },
};

const key = `${process.platform}-${process.arch}`;
const info = PLATFORM_TARGETS[key];

if (!info) {
  console.error(
    `tinypng-cli: unsupported platform ${key}. Supported: ${Object.keys(
      PLATFORM_TARGETS
    ).join(", ")}.`
  );
  console.error(
    "Please open an issue at https://github.com/aototo/tinypng-cli/issues"
  );
  process.exit(1);
}

const isWindows = process.platform === "win32";
const binaryName = NAME + (isWindows ? ".exe" : "");
const archiveName = `tinypng-cli-v${VERSION}-${info.target}.${info.ext}`;
const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${archiveName}`;
const binDir = path.join(repoRoot, "bin");
const dest = path.join(binDir, binaryName);

fs.mkdirSync(binDir, { recursive: true });

function download(url, destPath, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 10) return reject(new Error("Too many redirects"));
    https
      .get(
        url,
        {
          headers: { "User-Agent": "tinypng-cli-installer" },
        },
        (res) => {
          if (
            [301, 302, 303, 307, 308].includes(res.statusCode) &&
            res.headers.location
          ) {
            return download(res.headers.location, destPath, redirects + 1).then(
              resolve,
              reject
            );
          }
          if (res.statusCode !== 200) {
            return reject(
              new Error(`Download failed (HTTP ${res.statusCode}): ${url}`)
            );
          }
          const file = fs.createWriteStream(destPath);
          res.pipe(file);
          file.on("finish", () => file.close(() => resolve()));
          file.on("error", reject);
        }
      )
      .on("error", reject);
  });
}

async function main() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "tinypng-cli-"));
  const archivePath = path.join(tmpDir, archiveName);

  try {
    console.log(`tinypng-cli: downloading ${archiveName}...`);
    await download(url, archivePath);

    if (info.ext === "zip") {
      execSync(
        `powershell -NoProfile -Command "Expand-Archive -Force -Path '${archivePath}' -DestinationPath '${tmpDir}'"`,
        { stdio: "ignore" }
      );
    } else {
      execSync(`tar -xzf "${archivePath}" -C "${tmpDir}"`, {
        stdio: "ignore",
      });
    }

    const extracted = path.join(tmpDir, binaryName);
    if (!fs.existsSync(extracted)) {
      throw new Error(
        `Expected ${binaryName} inside archive but did not find it`
      );
    }
    fs.copyFileSync(extracted, dest);
    fs.chmodSync(dest, 0o755);
    console.log(`tinypng-cli: installed ${binaryName} (v${VERSION})`);
  } catch (err) {
    console.error(`tinypng-cli: install failed: ${err.message}`);
    console.error(
      "You can download the binary manually from https://github.com/aototo/tinypng-cli/releases"
    );
    process.exit(1);
  } finally {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {}
  }
}

main();
