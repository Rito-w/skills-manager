#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const [bundleDirArg, target, outputDirArg] = process.argv.slice(2);

if (!bundleDirArg || !target || !outputDirArg) {
  console.error(
    "Usage: node .github/scripts/stage-release-assets.js <bundle-dir> <target-triple> <output-dir>"
  );
  process.exit(1);
}

const bundleDir = path.resolve(bundleDirArg);
const outputDir = path.resolve(outputDirArg);

function listFiles(dir) {
  if (!fs.existsSync(dir)) return [];

  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...listFiles(fullPath));
    } else if (entry.isFile()) {
      files.push(fullPath);
    }
  }

  return files;
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

// Installer suffixes that should be included even without .sig
const installerSuffixes = [".dmg", ".msi", ".exe", ".AppImage", ".deb", ".rpm"];

const files = listFiles(bundleDir);
const signatureFiles = files.filter((filePath) => filePath.endsWith(".sig"));
const stagedAssets = [];
const filePrefix = `${target}__`;
const stagedBasenames = new Set();

// 1. Collect signed assets (for updater)
for (const sigPath of signatureFiles) {
  const assetPath = sigPath.slice(0, -4);
  if (!fs.existsSync(assetPath) || !fs.statSync(assetPath).isFile()) {
    continue;
  }

  const assetName = `${filePrefix}${path.basename(assetPath)}`;
  const sigName = `${assetName}.sig`;

  stagedAssets.push({ assetPath, assetName, sigPath, sigName });
  stagedBasenames.add(path.basename(assetPath));
}

// 2. Collect installer packages without .sig (e.g. .dmg)
for (const filePath of files) {
  const basename = path.basename(filePath);
  if (stagedBasenames.has(basename)) continue;
  if (filePath.endsWith(".sig")) continue;

  const isInstaller = installerSuffixes.some((suffix) => filePath.endsWith(suffix));
  if (!isInstaller) continue;

  const assetName = `${filePrefix}${basename}`;
  stagedAssets.push({ assetPath: filePath, assetName, sigPath: null, sigName: null });
}

if (stagedAssets.length === 0) {
  console.error(`No release assets found under ${bundleDir}`);
  process.exit(1);
}

ensureDir(outputDir);

for (const asset of stagedAssets) {
  fs.copyFileSync(asset.assetPath, path.join(outputDir, asset.assetName));
  if (asset.sigPath) {
    fs.copyFileSync(asset.sigPath, path.join(outputDir, asset.sigName));
  }
}

console.log(
  `Staged ${stagedAssets.length} asset(s) for ${target} into ${path.relative(process.cwd(), outputDir)}`
);
