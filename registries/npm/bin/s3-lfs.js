#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const isWindows = process.platform === "win32";
const binaryName = isWindows ? "s3-lfs.exe" : "s3-lfs";
const binaryPath = path.join(__dirname, "..", "vendor", binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error("s3-lfs binary is not installed.");
  console.error("Try reinstalling the package or run: npm rebuild s3-lfs");
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit"
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
