#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";

const args = process.argv.slice(2);
const version = args.find((arg) => !arg.startsWith("-"));
const shouldPush = args.includes("--push");

if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error("Usage: pnpm release <version> [--push]");
  console.error("Example: pnpm release 0.2.1 --push");
  process.exit(1);
}

const root = process.cwd();
const pkgPath = path.join(root, "package.json");
const tauriPath = path.join(root, "src-tauri/tauri.conf.json");
const cargoPath = path.join(root, "src-tauri/Cargo.toml");

const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
tauri.version = version;
fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n");

const cargo = fs.readFileSync(cargoPath, "utf8");
const nextCargo = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
fs.writeFileSync(cargoPath, nextCargo);

console.log(`Updated version to ${version}`);

if (shouldPush) {
  const tag = `v${version}`;
  execSync("git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml", {
    stdio: "inherit"
  });
  execSync(`git commit -m "chore(release): v${version}"`, { stdio: "inherit" });
  execSync(`git tag ${tag}`, { stdio: "inherit" });
  execSync("git push origin main --follow-tags", { stdio: "inherit" });
}
