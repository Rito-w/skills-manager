import fs from "node:fs";
import path from "node:path";

const changelogPath = path.join(process.cwd(), "CHANGELOG.md");

try {
  const content = fs.readFileSync(changelogPath, "utf8");
  const lines = content.split("\n");

  let capture = false;
  const log = [];

  const headerRegex = /^#+ \[?\d+\.\d+\.\d+/;

  for (const line of lines) {
    if (headerRegex.test(line)) {
      if (capture) {
        break;
      }
      capture = true;
      continue;
    }

    if (capture) {
      log.push(line);
    }
  }

  console.log(log.join("\n").trim());
} catch (error) {
  console.error("Error reading CHANGELOG.md:", error);
  process.exit(1);
}
