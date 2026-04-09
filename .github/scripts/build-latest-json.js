#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const [tag, repoSlug, artifactsDirArg, outputPathArg] = process.argv.slice(2);

if (!tag || !repoSlug || !artifactsDirArg || !outputPathArg) {
  console.error(
    "Usage: node .github/scripts/build-latest-json.js <tag> <repo> <artifacts-dir> <output-file>"
  );
  process.exit(1);
}

const artifactsDir = path.resolve(artifactsDirArg);
const outputPath = path.resolve(outputPathArg);

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

function normalizeArch(value) {
  const input = value.toLowerCase();
  if (input.includes("aarch64") || input.includes("arm64")) return "aarch64";
  if (input.includes("x86_64") || input.includes("amd64") || input.includes("x64")) return "x86_64";
  if (input.includes("i686") || input.includes("x86")) return "i686";
  if (input.includes("armv7")) return "armv7";
  return input;
}

function inferPlatformFromFile(filePath) {
  const normalized = filePath.replaceAll(path.sep, "/").toLowerCase();
  let osPart = null;

  if (normalized.includes("/macos/") || normalized.endsWith(".app.tar.gz")) {
    osPart = "darwin";
  } else if (
    normalized.includes("/nsis/") ||
    normalized.includes("/msi/") ||
    normalized.endsWith(".exe") ||
    normalized.endsWith(".msi") ||
    normalized.endsWith(".msi.zip")
  ) {
    osPart = "windows";
  } else if (
    normalized.includes("/appimage/") ||
    normalized.includes("/deb/") ||
    normalized.includes("/rpm/") ||
    normalized.endsWith(".appimage") ||
    normalized.endsWith(".appimage.tar.gz") ||
    normalized.endsWith(".deb") ||
    normalized.endsWith(".rpm")
  ) {
    osPart = "linux";
  }

  if (!osPart) return null;

  const archMatch =
    normalized.match(/(aarch64|arm64|x86_64|amd64|x64|i686|armv7)/) ??
    normalized.match(
      /(aarch64-apple-darwin|x86_64-apple-darwin|x86_64-unknown-linux-gnu|x86_64-pc-windows-msvc)/
    );

  const archSource = archMatch?.[1] ?? filePath;
  return `${osPart}-${normalizeArch(archSource)}`;
}

function artifactPriority(filePath) {
  const lower = filePath.toLowerCase();
  if (lower.endsWith(".app.tar.gz")) return 100;
  if (lower.endsWith(".appimage.tar.gz")) return 95;
  if (lower.endsWith(".appimage")) return 90;
  if (lower.endsWith(".exe")) return 85;
  if (lower.endsWith(".msi.zip")) return 80;
  if (lower.endsWith(".msi")) return 75;
  if (lower.endsWith(".deb")) return 70;
  if (lower.endsWith(".rpm")) return 65;
  return 10;
}

const candidates = new Map();

for (const sigPath of listFiles(artifactsDir)) {
  if (!sigPath.endsWith(".sig")) continue;

  const assetPath = sigPath.slice(0, -4);
  if (!fs.existsSync(assetPath) || !fs.statSync(assetPath).isFile()) continue;

  const platform = inferPlatformFromFile(assetPath);
  if (!platform) continue;

  const current = {
    platform,
    assetName: path.basename(assetPath),
    signature: fs.readFileSync(sigPath, "utf8").trim(),
    path: assetPath
  };

  const existing = candidates.get(platform);
  if (!existing || artifactPriority(current.path) > artifactPriority(existing.path)) {
    candidates.set(platform, current);
  }
}

if (candidates.size === 0) {
  console.error(`No updater artifacts found under ${artifactsDir}`);
  process.exit(1);
}

const version = tag.startsWith("v") ? tag.slice(1) : tag;
const latestJson = {
  version,
  notes: "",
  pub_date: new Date().toISOString(),
  platforms: Object.fromEntries(
    Array.from(candidates.values()).map((artifact) => [
      artifact.platform,
      {
        signature: artifact.signature,
        url: `https://github.com/${repoSlug}/releases/download/${tag}/${artifact.assetName}`
      }
    ])
  )
};

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(latestJson, null, 2) + "\n");

console.log(
  `Generated latest.json for ${candidates.size} platform(s) at ${path.relative(process.cwd(), outputPath)}`
);
