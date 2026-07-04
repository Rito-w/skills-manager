#!/usr/bin/env node
/**
 * Batch translate skill summaries to Chinese using Qwen API.
 * Usage: DASHSCOPE_API_KEY=sk-xxx node scripts/translate-skills.mjs
 */
import fs from "node:fs";
import https from "node:https";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const INDEX_PATH = path.resolve(__dirname, "../src-tauri/data/skills-index.json");
const API_URL = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
const MODEL = "qwen-turbo";
const BATCH_SIZE = 20;

const API_KEY = process.env.DASHSCOPE_API_KEY;
if (!API_KEY) {
  console.error("Error: DASHSCOPE_API_KEY environment variable is required");
  process.exit(1);
}

function post(url, body, headers) {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const data = JSON.stringify(body);
    const req = https.request(
      {
        hostname: parsed.hostname,
        path: parsed.pathname,
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Content-Length": Buffer.byteLength(data),
          ...headers,
        },
        timeout: 60000,
      },
      (res) => {
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => {
          const raw = Buffer.concat(chunks).toString();
          if (res.statusCode !== 200) {
            reject(new Error(`HTTP ${res.statusCode}: ${raw.slice(0, 200)}`));
          } else {
            resolve(JSON.parse(raw));
          }
        });
        res.on("error", reject);
      }
    );
    req.on("error", reject);
    req.on("timeout", () => { req.destroy(); reject(new Error("request timeout")); });
    req.write(data);
    req.end();
  });
}

async function translateBatch(summaries) {
  const body = {
    model: MODEL,
    messages: [
      {
        role: "system",
        content:
          "你是翻译助手。将下面 JSON 数组中的英文技术描述逐条翻译为简洁的中文。要求：1) 输出一个等长的 JSON 字符串数组；2) 保持顺序一一对应；3) 只输出 JSON 数组，不加任何解释。",
      },
      {
        role: "user",
        content: JSON.stringify(summaries),
      },
    ],
    temperature: 0.1,
  };

  const resp = await post(API_URL, body, {
    Authorization: `Bearer ${API_KEY}`,
  });

  const content = resp.choices?.[0]?.message?.content?.trim();
  if (!content) throw new Error("Empty response from API");

  // Extract JSON array from response (handle markdown code blocks)
  const jsonStr = content.replace(/^```json?\s*/, "").replace(/\s*```$/, "");
  const parsed = JSON.parse(jsonStr);

  if (!Array.isArray(parsed) || parsed.length !== summaries.length) {
    throw new Error(
      `Length mismatch: expected ${summaries.length}, got ${Array.isArray(parsed) ? parsed.length : "non-array"}`
    );
  }

  return parsed;
}

async function main() {
  const index = JSON.parse(fs.readFileSync(INDEX_PATH, "utf8"));
  const skills = index.skills;
  const total = skills.length;

  // Count how many need translation
  const needTranslation = skills.filter((s) => !s.summary_zh).length;
  console.log(`Total skills: ${total}, need translation: ${needTranslation}`);

  if (needTranslation === 0) {
    console.log("All skills already translated. Done.");
    return;
  }

  let translated = 0;
  let batchNum = 0;

  for (let i = 0; i < total; i += BATCH_SIZE) {
    const batch = skills.slice(i, i + BATCH_SIZE);
    const needWork = batch.some((s) => !s.summary_zh);
    if (!needWork) continue;

    // Collect summaries that need translation in this batch
    const summaries = batch.map((s) => s.summary || s.name || "");
    
    batchNum++;
    try {
      const translations = await translateBatch(summaries);

      // Write translations back
      for (let j = 0; j < batch.length; j++) {
        if (!batch[j].summary_zh) {
          skills[i + j].summary_zh = translations[j];
          translated++;
        }
      }

      // Save after each batch (crash-safe)
      index.skills = skills;
      fs.writeFileSync(INDEX_PATH, JSON.stringify(index, null, 2) + "\n");

      console.log(
        `  Batch ${batchNum}: translated ${batch.length} items (${translated}/${needTranslation} done)`
      );
    } catch (err) {
      console.error(`  Batch ${batchNum} failed: ${err.message}`);
      console.error("  Saving progress and pausing 5s...");
      index.skills = skills;
      fs.writeFileSync(INDEX_PATH, JSON.stringify(index, null, 2) + "\n");
      await new Promise((r) => setTimeout(r, 5000));
    }

    // Rate limit: small delay between batches
    await new Promise((r) => setTimeout(r, 500));
  }

  console.log(`\nDone! Translated ${translated} skills.`);
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
