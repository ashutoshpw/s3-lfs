#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const pkgDir = path.resolve(__dirname, "..");
const npmCmd = process.platform === "win32" ? "npm.cmd" : "npm";
const publishArgs = process.argv.slice(2);

function runNpm(args, options = {}) {
  const result = spawnSync(npmCmd, args, {
    cwd: pkgDir,
    encoding: "utf8",
    ...options
  });

  if (result.error) {
    console.error(`[s3-lfs] failed to run npm ${args.join(" ")}: ${result.error.message}`);
    process.exit(1);
  }

  return result;
}

const packResult = runNpm(["pack", "--json"], { stdio: ["ignore", "pipe", "inherit"] });
if (packResult.status !== 0) {
  process.exit(packResult.status ?? 1);
}

let tarballName = "";
try {
  const parsed = JSON.parse(packResult.stdout.trim());
  tarballName = parsed?.[0]?.filename || "";
} catch (error) {
  console.error(`[s3-lfs] unable to parse npm pack output: ${error.message}`);
  process.exit(1);
}

if (!tarballName) {
  console.error("[s3-lfs] npm pack did not return a tarball filename");
  process.exit(1);
}

const tarballPath = path.join(pkgDir, tarballName);
console.log(`[s3-lfs] publishing tarball ${tarballPath}`);

const args = ["publish", tarballPath, ...publishArgs];
const publishResult = runNpm(args, { stdio: "inherit" });
process.exit(publishResult.status ?? 1);
