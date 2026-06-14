#!/usr/bin/env node
"use strict";

const assert = require("assert");
const fs = require("fs");
const http = require("http");
const os = require("os");
const path = require("path");
const { spawn } = require("child_process");

const runtime = path.join(__dirname, "server.js");
const resourceRoot = fs.mkdtempSync(path.join(os.tmpdir(), "weft-js-runtime-"));
const weftDir = path.join(resourceRoot, ".weft");
const promptsDir = path.join(weftDir, "prompts");
const skillsDir = path.join(weftDir, "skills");
const extensionsDir = path.join(weftDir, "extensions");
const promptDir = path.join(promptsDir, "daily");
const skillDir = path.join(skillsDir, "writer");
const tsExtensionDir = path.join(extensionsDir, "typed");
const unsupportedTsExtensionDir = path.join(extensionsDir, "unsupported-typed");
const blockedExtensionDir = path.join(extensionsDir, "blocked");
fs.mkdirSync(promptDir, { recursive: true });
fs.mkdirSync(skillDir, { recursive: true });
fs.mkdirSync(tsExtensionDir, { recursive: true });
fs.mkdirSync(unsupportedTsExtensionDir, { recursive: true });
fs.mkdirSync(blockedExtensionDir, { recursive: true });
fs.writeFileSync(path.join(promptDir, "PROMPT.md"), `---
name: Daily Prompt
description: Writes a daily summary
arguments: [topic, audience]
variables: [tone]
---
Write a {{tone}} summary about {{topic}} for {{audience}}. First arg: $1. All args: $@.
`, "utf8");
fs.writeFileSync(path.join(skillDir, "SKILL.md"), `---
name: Writer Skill
description: Helps draft content
arguments: [brief]
---
Use this skill for {{task}}.
`, "utf8");

fs.writeFileSync(path.join(tsExtensionDir, "index.ts"), `module.exports.handle = function(payload: { name?: string }, context: { extensionRelativePath: string }) {
  const name: string = payload.name || "world";
  return { greeting: "hello " + name, extension: context.extensionRelativePath };
};
`, "utf8");
fs.writeFileSync(path.join(unsupportedTsExtensionDir, "index.ts"), `interface Payload { name?: string }
module.exports.handle = function(payload: Payload) {
  return payload.name || "world";
};
`, "utf8");
fs.writeFileSync(path.join(blockedExtensionDir, "index.js"), `module.exports.handle = function() {
  return "blocked should not execute";
};
`, "utf8");

function localTypeScriptAvailable() {
  let current = __dirname;
  while (true) {
    if (fs.existsSync(path.join(current, "node_modules", "typescript", "package.json"))) {
      return true;
    }
    const parent = path.dirname(current);
    if (parent === current) return false;
    current = parent;
  }
}
function request(port, method, pathname, body) {
  return new Promise((resolve, reject) => {
    const data = body ? JSON.stringify(body) : undefined;
    const req = http.request({
      hostname: "127.0.0.1",
      port,
      path: pathname,
      method,
      headers: data ? {
        "content-type": "application/json",
        "content-length": Buffer.byteLength(data),
      } : {},
    }, (res) => {
      let raw = "";
      res.setEncoding("utf8");
      res.on("data", (chunk) => { raw += chunk; });
      res.on("end", () => {
        try {
          resolve({ status: res.statusCode, body: JSON.parse(raw) });
        } catch (error) {
          reject(error);
        }
      });
    });
    req.on("error", reject);
    if (data) req.write(data);
    req.end();
  });
}

async function waitForHealth(port) {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    try {
      const response = await request(port, "GET", "/health");
      if (response.status === 200 && response.body.ok) return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw new Error("server did not become healthy");
}

(async () => {
  const port = 45000 + Math.floor(Math.random() * 1000);
  const child = spawn(process.execPath, [runtime], {
    env: { ...process.env, WEFT_RESOURCE_ROOT: resourceRoot, WEFT_JS_EXTENSION_PORT: String(port) },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stderr = "";
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  try {
    await waitForHealth(port);

    const metadata = await request(port, "GET", "/metadata/extensions");
    assert.strictEqual(metadata.status, 200);
    const typedMetadata = metadata.body.extensions.find((entry) => entry.id === "typed");
    assert(typedMetadata, "typed extension metadata should exist");
    assert.match(typedMetadata.audit.source_path, /\.weft\/extensions\/typed\/index\.ts$/);
    assert.strictEqual(typedMetadata.audit.extension_id, "typed");
    assert.strictEqual(typedMetadata.audit.runtime_type, "typescript");
    assert.strictEqual(typedMetadata.audit.timeout_ms, 30000);
    assert.match(typedMetadata.audit.hash_sha256, /^[a-f0-9]{64}$/);
    assert.strictEqual(typeof typedMetadata.audit.mtime_ms, "number");
    assert.deepStrictEqual(typedMetadata.audit.permissions_assumed, ["filesystem", "network", "process", "environment"]);

    const prompts = await request(port, "GET", "/prompts");
    assert.strictEqual(prompts.status, 200);
    assert.strictEqual(prompts.body.prompts.length, 1);
    assert.strictEqual(prompts.body.prompts[0].description, "Writes a daily summary");
    assert.deepStrictEqual(prompts.body.prompts[0].variables, ["audience", "tone", "topic"]);
    assert.deepStrictEqual(prompts.body.prompts[0].arg_placeholders, ["$1", "$@"]);
    assert.deepStrictEqual(prompts.body.prompts[0].argument_hints, ["topic", "audience"]);

    const rendered = await request(port, "POST", "/render-prompt", {
      id: "daily",
      variables: { tone: "concise", topic: "runtime", audience: "maintainers" },
      args: ["smoke", "test"],
    });
    assert.strictEqual(rendered.status, 200);
    assert.match(rendered.body.content, /concise summary about runtime for maintainers/);
    assert.match(rendered.body.content, /First arg: smoke/);
    assert.deepStrictEqual(rendered.body.variables, ["audience", "tone", "topic"]);
    assert.deepStrictEqual(rendered.body.arg_placeholders, ["$1", "$@"]);

    const skill = await request(port, "POST", "/render-skill", { id: "writer" });
    assert.strictEqual(skill.status, 200);
    assert.strictEqual(skill.body.skill.description, "Helps draft content");
    assert.deepStrictEqual(skill.body.skill.argument_hints, ["brief"]);
    assert.strictEqual(skill.body.skill.frontmatter.name, "Writer Skill");
    assert(!skill.body.skill.content.startsWith("---"), "skill content should not include frontmatter");

    const typed = await request(port, "POST", "/execute", { id: "typed", payload: { name: "runtime" } });
    assert.strictEqual(typed.status, 200);
    assert.deepStrictEqual(typed.body.result, { greeting: "hello runtime", extension: "typed/index.ts" });
    assert.match(typed.body.audit.source_path, /\.weft\/extensions\/typed\/index\.ts$/);
    assert.strictEqual(typed.body.audit.extension_id, "typed");
    assert.strictEqual(typed.body.audit.runtime_type, "typescript");
    assert.strictEqual(typed.body.audit.timeout_ms, 30000);
    assert.match(typed.body.audit.hash_sha256, /^[a-f0-9]{64}$/);
    assert.strictEqual(typeof typed.body.audit.mtime_ms, "number");
    assert.deepStrictEqual(typed.body.audit.permissions_assumed, ["filesystem", "network", "process", "environment"]);

    const unsupportedTyped = await request(port, "POST", "/execute", { id: "unsupported-typed", payload: { name: "runtime" } });
    if (localTypeScriptAvailable()) {
      assert.strictEqual(unsupportedTyped.status, 200);
      assert.strictEqual(unsupportedTyped.body.result, "runtime");
    } else {
      assert.strictEqual(unsupportedTyped.status, 415);
      assert.strictEqual(unsupportedTyped.body.error.code, "typescript_compiler_unavailable");
      assert.match(unsupportedTyped.body.error.message, /requires a local TypeScript compiler/);
    }

    child.kill("SIGTERM");
    await new Promise((resolve) => child.once("exit", resolve));

    const allowlistPort = port + 1000;
    const allowlistChild = spawn(process.execPath, [runtime], {
      env: { ...process.env, WEFT_RESOURCE_ROOT: resourceRoot, WEFT_JS_EXTENSION_PORT: String(allowlistPort), WEFT_JS_EXTENSION_ALLOWLIST: "typed" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    allowlistChild.stderr.on("data", (chunk) => { stderr += chunk; });
    try {
      await waitForHealth(allowlistPort);
      const blocked = await request(allowlistPort, "POST", "/execute", { id: "blocked" });
      assert.strictEqual(blocked.status, 403);
      assert.strictEqual(blocked.body.ok, false);
      assert.strictEqual(blocked.body.error.code, "extension_not_allowlisted");
      assert.strictEqual(blocked.body.error.details.extension_id, "blocked");
      assert.match(blocked.body.error.details.source_path, /\.weft\/extensions\/blocked\/index\.js$/);
      assert.strictEqual(blocked.body.error.details.runtime_type, "javascript");
      assert.deepStrictEqual(blocked.body.error.details.permissions_assumed, ["filesystem", "network", "process", "environment"]);
      const allowed = await request(allowlistPort, "POST", "/execute", { id: "typed", payload: { name: "allowlist" } });
      assert.strictEqual(allowed.status, 200);
      assert.strictEqual(allowed.body.audit.extension_id, "typed");
      assert.deepStrictEqual(allowed.body.result, { greeting: "hello allowlist", extension: "typed/index.ts" });
    } finally {
      allowlistChild.kill("SIGTERM");
      await new Promise((resolve) => allowlistChild.once("exit", resolve));
    }
  } finally {
    if (!child.killed && child.exitCode === null) {
      child.kill("SIGTERM");
      await new Promise((resolve) => child.once("exit", resolve));
    }
    fs.rmSync(resourceRoot, { recursive: true, force: true });
  }
  if (stderr) {
    process.stderr.write(stderr);
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
