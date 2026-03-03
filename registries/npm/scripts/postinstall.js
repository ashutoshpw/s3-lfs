#!/usr/bin/env node

const fs = require("node:fs");
const path = require("node:path");
const https = require("node:https");

const repo = process.env.LFS_S3_NPM_REPO || "ashutoshpw/s3-lfs";
const explicitTag = process.env.LFS_S3_NPM_TAG || "";
const flavor = (process.env.LFS_S3_NPM_FLAVOR || "rs").toLowerCase();
const skipDownload = process.env.LFS_S3_NPM_SKIP_DOWNLOAD === "1";

if (skipDownload) {
  console.log("[s3-lfs] skipping download (LFS_S3_NPM_SKIP_DOWNLOAD=1)");
  process.exit(0);
}

if (!["rs", "go"].includes(flavor)) {
  console.error(`[s3-lfs] unsupported flavor: ${flavor} (expected "rs" or "go")`);
  process.exit(1);
}

const platformName = (() => {
  switch (process.platform) {
    case "linux":
      return "linux";
    case "darwin":
      return "macos";
    case "win32":
      return "windows";
    default:
      return null;
  }
})();

if (!platformName) {
  console.error(`[s3-lfs] unsupported platform: ${process.platform}`);
  process.exit(1);
}

const archName = (() => {
  switch (process.arch) {
    case "x64":
      return "amd64";
    case "arm64":
      return "arm64";
    default:
      return null;
  }
})();

if (!archName) {
  console.error(
    `[s3-lfs] unsupported architecture: ${process.arch}; available release assets are amd64 and arm64`
  );
  process.exit(1);
}

const ext = process.platform === "win32" ? ".exe" : "";
const platformAsset = `s3-lfs-${flavor}-${platformName}-${archName}${ext}`;

const installDir = path.join(__dirname, "..", "vendor");
const installName = process.platform === "win32" ? "s3-lfs.exe" : "s3-lfs";
const installPath = path.join(installDir, installName);

const downloadUrl = explicitTag
  ? `https://github.com/${repo}/releases/download/${explicitTag}/${platformAsset}`
  : `https://github.com/${repo}/releases/latest/download/${platformAsset}`;

function download(url, destination) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (
          res.statusCode &&
          [301, 302, 303, 307, 308].includes(res.statusCode) &&
          res.headers.location
        ) {
          download(res.headers.location, destination).then(resolve).catch(reject);
          res.resume();
          return;
        }

        if (res.statusCode !== 200) {
          reject(new Error(`download failed with status ${res.statusCode}`));
          res.resume();
          return;
        }

        fs.mkdirSync(path.dirname(destination), { recursive: true });
        const tmpPath = `${destination}.tmp`;
        const file = fs.createWriteStream(tmpPath, { mode: 0o755 });

        res.pipe(file);
        file.on("finish", () => {
          file.close((closeErr) => {
            if (closeErr) {
              reject(closeErr);
              return;
            }
            fs.renameSync(tmpPath, destination);
            if (process.platform !== "win32") {
              fs.chmodSync(destination, 0o755);
            }
            resolve();
          });
        });

        file.on("error", (err) => {
          try {
            fs.unlinkSync(tmpPath);
          } catch (_e) {
            // ignore cleanup error
          }
          reject(err);
        });
      })
      .on("error", reject);
  });
}

(async () => {
  try {
    console.log(`[s3-lfs] downloading ${platformAsset} from ${downloadUrl}`);
    await download(downloadUrl, installPath);
    console.log(`[s3-lfs] installed binary at ${installPath}`);
  } catch (error) {
    console.error(`[s3-lfs] install failed: ${error.message}`);
    process.exit(1);
  }
})();
