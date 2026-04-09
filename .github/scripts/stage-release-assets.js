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

const files = listFiles(bundleDir);
const signatureFiles = files.filter((filePath) => filePath.endsWith(".sig"));
const stagedPairs = [];
const filePrefix = `${target}__`;

for (const sigPath of signatureFiles) {
  const assetPath = sigPath.slice(0, -4);
  if (!fs.existsSync(assetPath) || !fs.statSync(assetPath).isFile()) {
    continue;
  }

  const assetName = `${filePrefix}${path.basename(assetPath)}`;
  const sigName = `${assetName}.sig`;

  stagedPairs.push({
    assetPath,
    sigPath,
    assetName,
    sigName
  });
}

if (stagedPairs.length === 0) {
  console.error(`No signed release assets found under ${bundleDir}`);
  process.exit(1);
}

ensureDir(outputDir);

for (const pair of stagedPairs) {
  fs.copyFileSync(pair.assetPath, path.join(outputDir, pair.assetName));
  fs.copyFileSync(pair.sigPath, path.join(outputDir, pair.sigName));
}

console.log(
  `Staged ${stagedPairs.length} signed asset(s) for ${target} into ${path.relative(process.cwd(), outputDir)}`
);
