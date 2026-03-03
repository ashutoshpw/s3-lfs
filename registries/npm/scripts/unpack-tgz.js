#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const pkgDir = path.resolve(__dirname, "..");
const outDir = path.join(pkgDir, "temp");
const npmCmd = process.platform === "win32" ? "npm.cmd" : "npm";

function run(cmd, args, options = {}) {
  const result = spawnSync(cmd, args, {
    cwd: pkgDir,
    encoding: "utf8",
    ...options
  });

  if (result.error) {
    console.error(`[s3-lfs] failed to run ${cmd} ${args.join(" ")}: ${result.error.message}`);
    process.exit(1);
  }

  return result;
}

function resolveTarballPath() {
  const explicit = process.argv[2];
  if (explicit) {
    const absolute = path.isAbsolute(explicit) ? explicit : path.join(pkgDir, explicit);
    if (!fs.existsSync(absolute)) {
      console.error(`[s3-lfs] tarball not found: ${absolute}`);
      process.exit(1);
    }
    return absolute;
  }

  const tgzFiles = fs
    .readdirSync(pkgDir)
    .filter((name) => name.endsWith(".tgz"))
    .map((name) => ({
      name,
      abs: path.join(pkgDir, name),
      mtimeMs: fs.statSync(path.join(pkgDir, name)).mtimeMs
    }))
    .sort((a, b) => b.mtimeMs - a.mtimeMs);

  if (tgzFiles.length > 0) {
    return tgzFiles[0].abs;
  }

  const pack = run(npmCmd, ["pack", "--json"], { stdio: ["ignore", "pipe", "inherit"] });
  if (pack.status !== 0) {
    process.exit(pack.status ?? 1);
  }

  let filename = "";
  try {
    const parsed = JSON.parse(pack.stdout.trim());
    filename = parsed?.[0]?.filename || "";
  } catch (error) {
    console.error(`[s3-lfs] unable to parse npm pack output: ${error.message}`);
    process.exit(1);
  }

  if (!filename) {
    console.error("[s3-lfs] npm pack did not return a tarball filename");
    process.exit(1);
  }

  return path.join(pkgDir, filename);
}

const tarballPath = resolveTarballPath();

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

const extract = run("tar", ["-xzf", tarballPath, "-C", outDir], { stdio: "inherit" });
if (extract.status !== 0) {
  process.exit(extract.status ?? 1);
}

console.log(`[s3-lfs] extracted ${path.basename(tarballPath)} to ${outDir}`);
