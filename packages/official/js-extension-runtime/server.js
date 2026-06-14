#!/usr/bin/env node
"use strict";

const crypto = require("crypto");
const fs = require("fs");
const http = require("http");
const path = require("path");
const { pathToFileURL } = require("url");

const HOST = process.env.WEFT_JS_EXTENSION_HOST || "127.0.0.1";
const PORT = Number(process.env.WEFT_JS_EXTENSION_PORT || process.env.PORT || 43134);
const ROOT = path.resolve(process.env.WEFT_WORKSPACE_ROOT || process.cwd());
const RESOURCE_ROOT = path.resolve(process.env.WEFT_RESOURCE_ROOT || ROOT);
const WEFT_DIR = process.env.WEFT_RESOURCE_ROOT && path.basename(RESOURCE_ROOT) !== ".weft"
  ? path.join(RESOURCE_ROOT, ".weft")
  : path.resolve(process.env.WEFT_RESOURCE_ROOT || path.join(ROOT, ".weft"));
const RESOURCE_DIRS = {
  extensions: path.join(WEFT_DIR, "extensions"),
  skills: path.join(WEFT_DIR, "skills"),
  prompts: path.join(WEFT_DIR, "prompts"),
};
const EXECUTABLE_EXTENSION_EXTENSIONS = new Set([".js", ".mjs", ".cjs", ".ts"]);
const JAVASCRIPT_EXTENSION_EXTENSIONS = new Set([".js", ".mjs", ".cjs"]);
const MAX_JSON_BODY_BYTES = Number(process.env.WEFT_JS_EXTENSION_MAX_BODY_BYTES || 1024 * 1024);
const DEFAULT_EXECUTION_TIMEOUT_MS = Number(process.env.WEFT_JS_EXTENSION_TIMEOUT_MS || 30000);
const ALLOWLIST_RAW = process.env.WEFT_JS_EXTENSION_ALLOWLIST || "";
const ASSUMED_PERMISSIONS = Object.freeze(["filesystem", "network", "process", "environment"]);
const TS_CACHE_DIR = path.join(WEFT_DIR, ".cache", "js-extension-runtime", "typescript");
const UNSUPPORTED_TS_MESSAGE = "TypeScript extension execution requires a local TypeScript compiler. Add typescript to the extension workspace or use only simple dependency-free TS syntax supported by the fallback transpiler.";

function exists(filePath) {
  try {
    fs.accessSync(filePath, fs.constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function safeReadJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function safeReadFirstLine(filePath) {
  try {
    const text = fs.readFileSync(filePath, "utf8");
    return parseMarkdownFrontmatter(text).content.split(/\r?\n/).find((line) => line.trim().length > 0) || "";
  } catch {
    return "";
  }
}

function parseFrontmatterValue(value) {
  const trimmed = value.trim();
  if (trimmed === "") {
    return "";
  }
  if ((trimmed.startsWith('"') && trimmed.endsWith('"')) || (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
    return trimmed.slice(1, -1);
  }
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    return trimmed.slice(1, -1).split(",").map((item) => parseFrontmatterValue(item)).filter((item) => item !== "");
  }
  if (trimmed === "true") {
    return true;
  }
  if (trimmed === "false") {
    return false;
  }
  return trimmed;
}

function parseMarkdownFrontmatter(text) {
  if (typeof text !== "string" || !text.startsWith("---")) {
    return { attributes: {}, content: text || "" };
  }
  const lines = text.split(/\r?\n/);
  if (lines[0].trim() !== "---") {
    return { attributes: {}, content: text };
  }
  const endIndex = lines.findIndex((line, index) => index > 0 && line.trim() === "---");
  if (endIndex === -1) {
    return { attributes: {}, content: text };
  }
  const attributes = {};
  for (const line of lines.slice(1, endIndex)) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith("#")) {
      continue;
    }
    const separator = trimmed.indexOf(":");
    if (separator <= 0) {
      continue;
    }
    const key = trimmed.slice(0, separator).trim();
    const value = trimmed.slice(separator + 1);
    if (/^[A-Za-z0-9_.-]+$/.test(key)) {
      attributes[key] = parseFrontmatterValue(value);
    }
  }
  return { attributes, content: lines.slice(endIndex + 1).join("\n") };
}

function readMarkdownFileWithFrontmatter(filePath) {
  const raw = fs.readFileSync(filePath, "utf8");
  return { raw, ...parseMarkdownFrontmatter(raw) };
}

function stringList(value) {
  if (Array.isArray(value)) {
    return value.map((item) => String(item).trim()).filter(Boolean);
  }
  if (typeof value === "string") {
    return value.split(",").map((item) => item.trim()).filter(Boolean);
  }
  return [];
}

function frontmatterArgumentHints(attributes) {
  return stringList(attributes.arguments || attributes.args || attributes.argument_hints || attributes.argumentHints);
}

function frontmatterVariableHints(attributes) {
  return stringList(attributes.variables || attributes.vars);
}

function discoverTemplateVariables(content) {
  const variables = new Set();
  const regex = /\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g;
  let match;
  while ((match = regex.exec(content || "")) !== null) {
    variables.add(match[1]);
  }
  return [...variables].sort((a, b) => a.localeCompare(b));
}

function discoverArgPlaceholders(content) {
  const placeholders = new Set();
  const regex = /\$(\d+|@)/g;
  let match;
  while ((match = regex.exec(content || "")) !== null) {
    placeholders.add(`$${match[1]}`);
  }
  return [...placeholders].sort((a, b) => {
    if (a === "$@") return 1;
    if (b === "$@") return -1;
    return Number(a.slice(1)) - Number(b.slice(1));
  });
}

function primaryContentFile(type, dir) {
  const names = type === "skills" ? ["SKILL.md", "skill.md"] : ["PROMPT.md", "prompt.md", "template.md"];
  for (const name of names) {
    const candidate = path.join(dir, name);
    if (exists(candidate) && fs.statSync(candidate).isFile()) {
      return candidate;
    }
  }
  return null;
}

function findMetadataFile(dir) {
  for (const name of ["weft.json", "extension.json", "skill.json", "prompt.json", "package.json"]) {
    const candidate = path.join(dir, name);
    if (exists(candidate)) {
      return candidate;
    }
  }
  return null;
}

function normalizeRelative(filePath) {
  return path.relative(ROOT, filePath).replace(/\\/g, "/");
}

function fileSha256(filePath) {
  try {
    return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
  } catch {
    return null;
  }
}

function auditMetadataForExtension(filePath, extensionId, timeoutMs = DEFAULT_EXECUTION_TIMEOUT_MS) {
  const stat = filePath && exists(filePath) ? fs.statSync(filePath) : null;
  return {
    source_path: filePath ? normalizeRelative(filePath) : null,
    extension_id: extensionId || null,
    mtime_ms: stat ? stat.mtimeMs : null,
    hash_sha256: filePath ? fileSha256(filePath) : null,
    runtime_type: extensionKindForFile(filePath),
    timeout_ms: timeoutMs,
    permissions_assumed: [...ASSUMED_PERMISSIONS],
  };
}

function parseExtensionAllowlist(raw = ALLOWLIST_RAW) {
  return String(raw || "")
    .split(/[;,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function normalizeAllowlistToken(token) {
  const value = String(token || "").trim();
  if (!value) {
    return "";
  }
  if (path.isAbsolute(value)) {
    return path.resolve(value).toLowerCase();
  }
  return value.replace(/\\/g, "/").replace(/^\.\//, "").toLowerCase();
}

function isExtensionAllowlisted(extensionId, filePath) {
  const entries = parseExtensionAllowlist();
  if (entries.length === 0) {
    return true;
  }

  const normalizedId = String(extensionId || "").toLowerCase();
  const absoluteFile = path.resolve(filePath).toLowerCase();
  const relativeToExtensions = path.relative(RESOURCE_DIRS.extensions, filePath).replace(/\\/g, "/").toLowerCase();
  const relativeToRoot = normalizeRelative(filePath).toLowerCase();

  return entries.some((entry) => {
    const normalized = normalizeAllowlistToken(entry);
    if (!normalized) {
      return false;
    }
    if (normalized === normalizedId || normalized === relativeToExtensions || normalized === relativeToRoot || normalized === absoluteFile) {
      return true;
    }
    return relativeToExtensions.startsWith(`${normalized.replace(/\/+$/, "")}/`);
  });
}

function extensionKindForFile(filePath) {
  const extension = path.extname(filePath || "").toLowerCase();
  if (extension === ".mjs") {
    return "module";
  }
  if (extension === ".cjs") {
    return "commonjs";
  }
  if (extension === ".ts") {
    return "typescript";
  }
  if (extension === ".js") {
    return "javascript";
  }
  return extension ? extension.slice(1) : "unknown";
}

function discoverStaticExports(filePath) {
  if (!filePath || !exists(filePath) || !fs.statSync(filePath).isFile()) {
    return null;
  }

  const extension = path.extname(filePath).toLowerCase();
  if (!JAVASCRIPT_EXTENSION_EXTENSIONS.has(extension) && extension !== ".ts") {
    return null;
  }

  let source;
  try {
    source = fs.readFileSync(filePath, "utf8");
  } catch {
    return null;
  }

  const exports = new Set();
  const patterns = [
    /export\s+default\b/g,
    /export\s+(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/g,
    /export\s+(?:const|let|var|class)\s+([A-Za-z_$][\w$]*)/g,
    /export\s*\{([^}]+)\}/g,
    /module\.exports\s*=\s*/g,
    /exports\.([A-Za-z_$][\w$]*)\s*=/g,
    /module\.exports\.([A-Za-z_$][\w$]*)\s*=/g,
  ];

  let match;
  while ((match = patterns[0].exec(source))) {
    exports.add("default");
  }
  for (const pattern of patterns.slice(1, 4)) {
    while ((match = pattern.exec(source))) {
      if (pattern === patterns[3]) {
        for (const item of match[1].split(",")) {
          const exportedName = item.trim().split(/\s+as\s+/i).pop().trim();
          if (/^[A-Za-z_$][\w$]*$/.test(exportedName)) {
            exports.add(exportedName);
          }
        }
      } else if (match[1]) {
        exports.add(match[1]);
      }
    }
  }
  while ((match = patterns[4].exec(source))) {
    exports.add("module.exports");
  }
  for (const pattern of patterns.slice(5)) {
    while ((match = pattern.exec(source))) {
      exports.add(match[1]);
    }
  }

  return Array.from(exports).sort();
}

function readResourceMetadata(type, dir) {
  const metadataFile = findMetadataFile(dir);
  const json = metadataFile ? safeReadJson(metadataFile) : null;
  const readme = path.join(dir, "README.md");
  const fallbackName = path.basename(dir);
  const contentFile = type === "skills" || type === "prompts" ? primaryContentFile(type, dir) : null;
  const parsedContent = contentFile ? readMarkdownFileWithFrontmatter(contentFile) : { attributes: {}, content: "" };
  const frontmatter = parsedContent.attributes;
  const metadata = { ...frontmatter, ...(json || {}) };
  const extensionEntry = type === "extensions" ? findExtensionEntry(dir, metadata) : null;
  const extension = extensionEntry ? path.extname(extensionEntry).toLowerCase() : null;
  const supported = type !== "extensions" || Boolean(extensionEntry && EXECUTABLE_EXTENSION_EXTENSIONS.has(extension));
  const executable = type === "extensions" && supported;
  const resource = {
    id: metadata.id || metadata.name || fallbackName,
    name: metadata.name || metadata.displayName || metadata.title || metadata.id || fallbackName,
    type,
    kind: type === "extensions" ? extensionKindForFile(extensionEntry) : type.replace(/s$/, ""),
    version: metadata.version || null,
    description: metadata.description || (exists(readme) ? safeReadFirstLine(readme).replace(/^#\s*/, "") : null),
    path: normalizeRelative(dir),
    absolute_path: dir,
    supported,
    executable,
    entry: extensionEntry ? normalizeRelative(extensionEntry) : null,
    exports: type === "extensions" ? discoverStaticExports(extensionEntry) : null,
    metadata_file: metadataFile ? normalizeRelative(metadataFile) : null,
  };

  if (type === "extensions") {
    resource.audit = auditMetadataForExtension(extensionEntry, resource.id);
  }

  if (contentFile) {
    resource.content_file = normalizeRelative(contentFile);
    const argumentHints = frontmatterArgumentHints(frontmatter);
    if (argumentHints.length > 0) {
      resource.argument_hints = argumentHints;
    }
  }
  if (type === "prompts" && contentFile) {
    resource.variables = [...new Set([...frontmatterVariableHints(frontmatter), ...discoverTemplateVariables(parsedContent.content)])].sort((a, b) => a.localeCompare(b));
    resource.arg_placeholders = discoverArgPlaceholders(parsedContent.content);
  }
  return resource;
}

function readPromptFileMetadata(filePath) {
  const parsed = readMarkdownFileWithFrontmatter(filePath);
  const frontmatter = parsed.attributes;
  const fallbackName = path.basename(filePath, path.extname(filePath));
  const resource = {
    id: frontmatter.id || fallbackName,
    name: frontmatter.name || frontmatter.title || fallbackName,
    type: "prompts",
    kind: "prompt",
    version: frontmatter.version || null,
    description: frontmatter.description || null,
    path: normalizeRelative(filePath),
    absolute_path: filePath,
    supported: true,
    executable: false,
    entry: null,
    exports: null,
    metadata_file: null,
    content_file: normalizeRelative(filePath),
    variables: [...new Set([...frontmatterVariableHints(frontmatter), ...discoverTemplateVariables(parsed.content)])].sort((a, b) => a.localeCompare(b)),
    arg_placeholders: discoverArgPlaceholders(parsed.content),
  };
  const argumentHints = frontmatterArgumentHints(frontmatter);
  if (argumentHints.length > 0) {
    resource.argument_hints = argumentHints;
  }
  return resource;
}

function scanResourceType(type) {
  const base = RESOURCE_DIRS[type];
  if (!base || !exists(base)) {
    return [];
  }

  return fs.readdirSync(base, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() || (type === "prompts" && entry.isFile() && [".md", ".txt"].includes(path.extname(entry.name).toLowerCase())))
    .map((entry) => {
      const entryPath = path.join(base, entry.name);
      return entry.isDirectory() ? readResourceMetadata(type, entryPath) : readPromptFileMetadata(entryPath);
    })
    .sort((a, b) => a.id.localeCompare(b.id));
}

function scanResources() {
  return {
    extensions: scanResourceType("extensions"),
    skills: scanResourceType("skills"),
    prompts: scanResourceType("prompts"),
  };
}

function isInside(parent, child) {
  const relative = path.relative(parent, child);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function safeResolveUnder(base, relativePath) {
  if (typeof relativePath !== "string" || relativePath.trim() === "") {
    return null;
  }

  const normalized = relativePath.replace(/\\/g, "/");
  if (normalized.includes("\0") || path.isAbsolute(normalized)) {
    return null;
  }

  const resolved = path.resolve(base, normalized);
  return isInside(base, resolved) ? resolved : null;
}

function readRequestJson(req, callback) {
  let body = "";
  let rejected = false;
  const declaredLength = Number(req.headers["content-length"] || 0);
  if (declaredLength > MAX_JSON_BODY_BYTES) {
    callback(Object.assign(new Error("request body too large"), { statusCode: 413, code: "payload_too_large" }));
    req.destroy();
    return;
  }
  req.setEncoding("utf8");
  req.on("data", (chunk) => {
    if (rejected) {
      return;
    }
    body += chunk;
    if (Buffer.byteLength(body, "utf8") > MAX_JSON_BODY_BYTES) {
      rejected = true;
      callback(Object.assign(new Error("request body too large"), { statusCode: 413, code: "payload_too_large" }));
      req.destroy();
    }
  });
  req.on("end", () => {
    if (rejected) {
      return;
    }
    if (body.trim() === "") {
      return callback(null, {});
    }
    try {
      callback(null, JSON.parse(body));
    } catch (error) {
      callback(Object.assign(error, { statusCode: 400, code: "invalid_json" }));
    }
  });
  req.on("error", (error) => {
    if (!rejected) {
      callback(error);
    }
  });
}

function readPrompt(id) {
  const base = RESOURCE_DIRS.prompts;
  const requested = safeResolveUnder(base, id);
  if (!requested) {
    return null;
  }

  const candidates = [];
  if (exists(requested)) {
    const stat = fs.statSync(requested);
    if (stat.isFile()) {
      candidates.push(requested);
    } else if (stat.isDirectory()) {
      candidates.push(path.join(requested, "PROMPT.md"), path.join(requested, "prompt.md"), path.join(requested, "template.md"));
    }
  } else if (!path.extname(requested)) {
    candidates.push(`${requested}.md`, `${requested}.txt`);
  }

  for (const candidate of candidates) {
    const resolved = path.resolve(candidate);
    if (isInside(base, resolved) && exists(resolved) && fs.statSync(resolved).isFile()) {
      const parsed = readMarkdownFileWithFrontmatter(resolved);
      return {
        id,
        path: normalizeRelative(resolved),
        content: parsed.content,
        frontmatter: parsed.attributes,
        variables: [...new Set([...frontmatterVariableHints(parsed.attributes), ...discoverTemplateVariables(parsed.content)])].sort((a, b) => a.localeCompare(b)),
        arg_placeholders: discoverArgPlaceholders(parsed.content),
        argument_hints: frontmatterArgumentHints(parsed.attributes),
      };
    }
  }

  return null;
}

function renderPromptTemplate(template, variables, args) {
  const values = variables && typeof variables === "object" && !Array.isArray(variables) ? variables : {};
  const argv = Array.isArray(args) ? args.map((value) => String(value)) : [];

  return template
    .replace(/\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g, (match, name) => {
      return Object.prototype.hasOwnProperty.call(values, name) ? String(values[name]) : match;
    })
    .replace(/\$(\d+|@)/g, (match, token) => {
      if (token === "@") {
        return argv.join(" ");
      }
      const index = Number(token) - 1;
      return index >= 0 && index < argv.length ? argv[index] : "";
    });
}

function readSkill(id) {
  const base = RESOURCE_DIRS.skills;
  const dir = safeResolveUnder(base, id);
  if (!dir || !exists(dir) || !fs.statSync(dir).isDirectory()) {
    return null;
  }

  const skillFile = path.resolve(dir, "SKILL.md");
  if (!isInside(base, skillFile) || !exists(skillFile) || !fs.statSync(skillFile).isFile()) {
    return null;
  }

  const parsed = readMarkdownFileWithFrontmatter(skillFile);
  return {
    ...readResourceMetadata("skills", dir),
    id,
    content: parsed.content,
    frontmatter: parsed.attributes,
    argument_hints: frontmatterArgumentHints(parsed.attributes),
    skill_file: normalizeRelative(skillFile),
  };
}

function findExtensionEntry(dir, metadata) {
  const configuredEntry = metadata && (metadata.main || metadata.module || metadata.entry || metadata.handler);
  if (typeof configuredEntry === "string" && configuredEntry.trim() !== "") {
    const resolved = safeResolveUnder(dir, configuredEntry);
    if (resolved && exists(resolved) && fs.statSync(resolved).isFile()) {
      return resolved;
    }
  }

  for (const fileName of ["index.js", "index.mjs", "index.cjs", "main.js", "main.mjs", "main.cjs", "extension.js", "extension.mjs", "extension.cjs"]) {
    const candidate = path.join(dir, fileName);
    if (exists(candidate) && fs.statSync(candidate).isFile()) {
      return candidate;
    }
  }

  const files = fs.readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && EXECUTABLE_EXTENSION_EXTENSIONS.has(path.extname(entry.name).toLowerCase()))
    .map((entry) => path.join(dir, entry.name));

  if (files.length === 1) {
    return files[0];
  }

  for (const fileName of ["index.ts", "main.ts", "extension.ts"]) {
    const candidate = path.join(dir, fileName);
    if (exists(candidate) && fs.statSync(candidate).isFile()) {
      return candidate;
    }
  }

  const tsFiles = fs.readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && path.extname(entry.name).toLowerCase() === ".ts")
    .map((entry) => path.join(dir, entry.name));

  return tsFiles.length === 1 ? tsFiles[0] : null;
}

function inferExtensionIdForFile(file) {
  const relative = path.relative(RESOURCE_DIRS.extensions, file).replace(/\\/g, "/");
  const firstSegment = relative.split("/")[0];
  if (firstSegment && firstSegment !== relative) {
    const dir = path.join(RESOURCE_DIRS.extensions, firstSegment);
    const metadataFile = exists(dir) && fs.statSync(dir).isDirectory() ? findMetadataFile(dir) : null;
    const metadata = metadataFile ? safeReadJson(metadataFile) : null;
    return metadata && (metadata.id || metadata.name) || firstSegment;
  }
  return path.basename(file, path.extname(file));
}

function resolveExtensionModule(body) {
  const base = RESOURCE_DIRS.extensions;
  if (!exists(base) || !fs.statSync(base).isDirectory()) {
    return { error: "extensions_not_found", statusCode: 404, message: "Extensions directory does not exist." };
  }

  const requestedFile = body.file || body.path || body.module;
  if (requestedFile) {
    const file = safeResolveUnder(base, requestedFile);
    if (!file || !exists(file) || !fs.statSync(file).isFile()) {
      return { error: "extension_not_found", statusCode: 404, message: "Extension file was not found under the extensions directory." };
    }
    if (path.extname(file).toLowerCase() === ".ts") {
      return { file, id: inferExtensionIdForFile(file) };
    }
    if (!EXECUTABLE_EXTENSION_EXTENSIONS.has(path.extname(file).toLowerCase())) {
      return { error: "unsupported_module_type", statusCode: 415, message: "Unsupported extension module type. Use .js, .mjs, .cjs, or .ts." };
    }
    return { file, id: inferExtensionIdForFile(file) };
  }

  const id = body.id || body.extension || body.name;
  if (typeof id !== "string" || id.trim() === "") {
    return { error: "missing_extension", statusCode: 400, message: "POST /execute requires an extension id/name or a relative extension file path." };
  }

  for (const entry of fs.readdirSync(base, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }

    const dir = path.join(base, entry.name);
    const metadataFile = findMetadataFile(dir);
    const metadata = metadataFile ? safeReadJson(metadataFile) : null;
    const metadataId = metadata && (metadata.id || metadata.name);
    if (entry.name !== id && metadataId !== id) {
      continue;
    }

    const file = findExtensionEntry(dir, metadata);
    if (!file) {
      return { error: "extension_entry_not_found", statusCode: 404, message: "No supported JS entry file found for extension." };
    }
    if (!isInside(base, path.resolve(file))) {
      return { error: "extension_outside_root", statusCode: 403, message: "Extension entry must stay inside the extensions directory." };
    }
    return { file, id: metadataId || entry.name };
  }

  return { error: "extension_not_found", statusCode: 404, message: "Extension id/name was not found." };
}

function findLocalTypeScript(startDir) {
  let current = path.resolve(startDir);
  while (isInside(ROOT, current) || isInside(WEFT_DIR, current)) {
    const candidate = path.join(current, "node_modules", "typescript");
    const packageJson = path.join(candidate, "package.json");
    if (exists(packageJson)) {
      try {
        return require(candidate);
      } catch {
        return null;
      }
    }
    const parent = path.dirname(current);
    if (parent === current) {
      break;
    }
    current = parent;
  }
  return null;
}

function hasLikelyUnsupportedTypeScript(source) {
  const unsupportedPatterns = [
    /\benum\s+[A-Za-z_$][\w$]*\s*\{/,
    /\bnamespace\s+[A-Za-z_$][\w$]*\s*\{/,
    /\binterface\s+[A-Za-z_$][\w$]*/,
    /\btype\s+[A-Za-z_$][\w$]*(?:<[^>]+>)?\s*=/,
    /\b(?:public|private|protected|readonly)\s+[A-Za-z_$][\w$]*/,
    /\bimplements\s+[A-Za-z_$][\w$]*/,
    /\bdeclare\s+/,
    /\babstract\s+class\s+/,
    /\bimport\s+type\b/,
    /\bexport\s+type\b/,
    /\bexport\s+interface\b/,
  ];
  return unsupportedPatterns.some((pattern) => pattern.test(source));
}

function stripSimpleTypeScript(source) {
  if (hasLikelyUnsupportedTypeScript(source)) {
    const error = new Error(`${UNSUPPORTED_TS_MESSAGE} The fallback transpiler only supports simple parameter, variable, return, and assertion type annotations.`);
    error.code = "typescript_compiler_unavailable";
    error.statusCode = 415;
    throw error;
  }

  return source
    .replace(/:\s*\{[^{}]*\}\s*(?=,|\)|=)/g, "")
    .replace(/:\s*[A-Za-z_$][\w$.[\]<>|&?,\s]*(?=\s*[,)=;{])/g, "")
    .replace(/\)\s*:\s*[A-Za-z_$][\w$.[\]<>|&?,\s]*(?=\s*=>|\s*\{)/g, ")")
    .replace(/\b(as|satisfies)\s+[A-Za-z_$][\w$.[\]<>|&?,\s]*(?=\s*[,);\]}]|$)/g, "")
    .replace(/!\s*(?=[,);\].])/g, "");
}

function transpileTypeScriptSource(file, source) {
  const ts = findLocalTypeScript(path.dirname(file));
  if (ts && typeof ts.transpileModule === "function") {
    const transpiled = ts.transpileModule(source, {
      fileName: file,
      compilerOptions: {
        module: ts.ModuleKind && ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget && ts.ScriptTarget.ES2020,
        esModuleInterop: true,
        sourceMap: false,
        inlineSourceMap: false,
        inlineSources: false,
      },
      reportDiagnostics: true,
    });
    const diagnostics = (transpiled.diagnostics || []).filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error);
    if (diagnostics.length > 0) {
      const message = diagnostics.map((diagnostic) => ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")).join("\n");
      const error = new Error(message || "TypeScript transpilation failed.");
      error.code = "typescript_transpile_failed";
      error.statusCode = 400;
      throw error;
    }
    return transpiled.outputText;
  }
  return stripSimpleTypeScript(source);
}

function transpileTypeScriptToCache(file) {
  if (!isInside(RESOURCE_DIRS.extensions, path.resolve(file))) {
    const error = new Error("Extension entry must stay inside the extensions directory.");
    error.code = "extension_outside_root";
    error.statusCode = 403;
    throw error;
  }

  const source = fs.readFileSync(file, "utf8");
  const stat = fs.statSync(file);
  const hash = crypto.createHash("sha256").update(source).digest("hex");
  const cacheKey = crypto.createHash("sha256").update(path.resolve(file)).digest("hex");
  const cachedFile = path.join(TS_CACHE_DIR, `${cacheKey}-${Math.floor(stat.mtimeMs)}-${hash.slice(0, 16)}.cjs`);
  if (exists(cachedFile)) {
    return cachedFile;
  }

  fs.mkdirSync(TS_CACHE_DIR, { recursive: true });
  const js = transpileTypeScriptSource(file, source);
  fs.writeFileSync(cachedFile, `${js}\n//# sourceURL=${pathToFileURL(file).href}\n`, "utf8");
  return cachedFile;
}

async function loadExtensionModule(file) {
  const extension = path.extname(file).toLowerCase();
  if (extension === ".ts") {
    return loadExtensionModule(transpileTypeScriptToCache(file));
  }
  if (!JAVASCRIPT_EXTENSION_EXTENSIONS.has(extension)) {
    const error = new Error("Unsupported extension module type. Use .js, .mjs, .cjs, or .ts.");
    error.code = "unsupported_module_type";
    error.statusCode = 415;
    throw error;
  }

  if (extension === ".mjs") {
    return import(`${pathToFileURL(file).href}?mtime=${fs.statSync(file).mtimeMs}`);
  }

  delete require.cache[require.resolve(file)];
  try {
    return require(file);
  } catch (error) {
    if (extension === ".js" && (error.code === "ERR_REQUIRE_ESM" || /Cannot use import statement/.test(error.message))) {
      return import(`${pathToFileURL(file).href}?mtime=${fs.statSync(file).mtimeMs}`);
    }
    throw error;
  }
}

function resolveHandler(moduleExports) {
  if (typeof moduleExports === "function") {
    return moduleExports;
  }
  if (moduleExports && typeof moduleExports.default === "function") {
    return moduleExports.default;
  }
  if (moduleExports && typeof moduleExports.handle === "function") {
    return moduleExports.handle;
  }
  return null;
}

async function executeBuiltinTool(body) {
  const payload = Object.prototype.hasOwnProperty.call(body || {}, "payload") ? body.payload : body;
  const tool = String(payload.tool || payload.name || payload.toolName || "").trim();
  const args = payload.args && typeof payload.args === "object" ? payload.args : payload;
  const normalizedTool = tool === "websearch" ? "web_search" : tool;
  if (["web_search", "search_web"].includes(normalizedTool)) {
    try {
      return { statusCode: 200, body: { ok: true, extension: "builtin/tool-executor", result: await builtinSearchWeb(args) } };
    } catch (error) {
      return { statusCode: 200, body: { ok: true, extension: "builtin/tool-executor", result: { status: "ok", data: emptySearchResult(args && args.query, error) } } };
    }
  }
  if (["fetch_url", "web_fetch"].includes(normalizedTool)) {
    try {
      return { statusCode: 200, body: { ok: true, extension: "builtin/tool-executor", result: await builtinFetchUrl(args) } };
    } catch (error) {
      return { statusCode: 400, body: errorBody(error && error.code ? error.code : "tool_execution_failed", error instanceof Error ? error.message : String(error)) };
    }
  }
  return { statusCode: 404, body: errorBody("builtin_tool_not_found", `No JS builtin tool handler for '${tool}'.`) };
}

function isBuiltinToolExecution(body) {
  const action = String((body && (body.action || body.functionName)) || "").trim();
  const id = String((body && (body.extension || body.name)) || "").trim();
  const payload = body && Object.prototype.hasOwnProperty.call(body, "payload") ? body.payload : body;
  const tool = String((payload && (payload.tool || payload.toolName)) || "").trim();
  return id === "weft-tool-executor" || action === "execute_tool" || action === "executeTool" || tool.length > 0;
}

function decodeTransportBase64Text(raw) {
  const value = String(raw || "").trim();
  if (!value) return "";
  try {
    return Buffer.from(value, "base64").toString("utf8");
  } catch {
    return "";
  }
}

function resolveSearchQuery(args) {
  return String(
    decodeTransportBase64Text(args && (args.query_b64 || args.q_b64))
    || (args && (args.query || args.q))
    || ""
  ).trim();
}

function normalizeSearchLimit(args) {
  const raw = Number(args && (args.max_results || args.limit || args.count));
  if (!Number.isFinite(raw)) return 5;
  return Math.max(1, Math.min(10, Math.floor(raw)));
}

function emptySearchResult(query, error) {
  const normalizedQuery = String(query || "").trim();
  const message = error instanceof Error && error.message ? error.message : String(error || "no results");
  return {
    query: normalizedQuery,
    provider: "js-runtime-fallback",
    heading: "",
    results: [],
    links: [],
    summary: `No concise web result found for '${normalizedQuery}'. (${message})`,
    source: "JS runtime web search fallback",
    diagnostics: [message],
  };
}

function publicHttpUrl(raw) {
  const url = new URL(String(raw || "").trim());
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("Only http(s) URLs are allowed.");
  }
  const hostname = url.hostname.toLowerCase();
  if (hostname === "localhost" || hostname.endsWith(".local") || hostname.endsWith(".internal") || /^(127\.|10\.|0\.|169\.254\.|192\.168\.)/.test(hostname) || /^172\.(1[6-9]|2\d|3[0-1])\./.test(hostname)) {
    throw new Error("Local/private network URLs are blocked.");
  }
  return url.toString();
}

function htmlToText(html) {
  return String(html || "")
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/g, " ")
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\s+/g, " ")
    .trim();
}

async function builtinFetchUrl(args) {
  const url = publicHttpUrl(args && args.url);
  const response = await fetch(url, {
    method: String((args && args.method) || "GET").toUpperCase(),
    headers: { "user-agent": "Mozilla/5.0 WEFT JS Tool Runtime", accept: "text/html,application/xhtml+xml,application/json,text/plain,*/*" },
    body: args && args.body ? String(args.body) : undefined,
    signal: AbortSignal.timeout(15000),
  });
  const contentType = response.headers.get("content-type") || "";
  const raw = await response.text();
  const content = contentType.includes("html") ? htmlToText(raw) : raw.trim();
  const clipped = content.length > 80000 ? `${content.slice(0, 80000)}\n\n[内容已截断]` : content;
  const title = String((args && args.title) || url);
  return { status: "ok", data: { ok: true, url, status: response.status, content_type: contentType, content: clipped, body: raw, summary: `[网页内容：${title}]\n\n${clipped.slice(0, 8000)}` } };
}

function resolveSearchProvider(raw) {
  const value = String(raw || process.env.WEFT_WEB_SEARCH_PROVIDER || process.env.WEB_SEARCH_PROVIDER || "").trim().toLowerCase();
  if (["exa", "exa-search", "exa_search"].includes(value)) return { provider: "exa", fallback: false };
  if (["brave", "brave-search", "brave_search"].includes(value)) return { provider: "brave", fallback: false };
  if (["searxng", "searx", "searx-ng", "searx_ng"].includes(value)) return { provider: "searxng", fallback: false };
  if (["tavily", "tavily-search", "tavily_search"].includes(value)) return { provider: "tavily", fallback: false };
  if (["", "default", "duckduckgo", "ddg", "duck-duck-go", "duck_duck_go"].includes(value)) return { provider: "auto", fallback: false };
  return { provider: "auto", fallback: true, unknown: value };
}

function asSearchData(query, provider, source, results, diagnostics = [], summary = "") {
  const links = results.map((entry) => ({ title: entry.title, url: entry.url, snippet: entry.text }));
  return {
    query,
    provider,
    heading: "",
    results,
    links,
    summary: summary || (results.length ? results.map((entry) => `${entry.title}: ${entry.text}`).join("\n") : `No concise web result found for '${query}'.`),
    source,
    diagnostics,
  };
}

function validSearchResults(items, limit) {
  return items
    .map((entry) => ({
      title: htmlToText(entry.title || entry.name || ""),
      url: String(entry.url || entry.link || "").trim(),
      text: htmlToText(entry.text || entry.snippet || entry.description || entry.content || ""),
      score: Number(entry.score || 0),
    }))
    .filter((entry) => /^https?:\/\//i.test(entry.url))
    .slice(0, limit);
}

async function searchTavily(query, limit) {
  const key = process.env.TAVILY_API_KEY || process.env.WEFT_TAVILY_API_KEY;
  if (!key) throw new Error("Tavily API key not configured");
  const response = await fetch("https://api.tavily.com/search", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ api_key: key, query, max_results: limit, search_depth: "basic", include_answer: false }),
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Tavily search failed with status: ${response.status}`);
  const parsed = await response.json();
  const results = validSearchResults(Array.isArray(parsed.results) ? parsed.results.map((entry) => ({ title: entry.title, url: entry.url, text: entry.content, score: entry.score })) : [], limit);
  if (results.length === 0) throw new Error("Tavily returned no usable results");
  return asSearchData(query, "tavily-js", "Tavily Search API via JS runtime", results, [], String(parsed.answer || ""));
}

async function searchBrave(query, limit) {
  const key = process.env.BRAVE_API_KEY || process.env.WEFT_BRAVE_API_KEY || process.env.BRAVE_SEARCH_API_KEY;
  if (!key) throw new Error("Brave API key not configured");
  const url = `https://api.search.brave.com/res/v1/web/search?q=${encodeURIComponent(query)}&count=${limit}`;
  const response = await fetch(url, {
    headers: { accept: "application/json", "x-subscription-token": key },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Brave search failed with status: ${response.status}`);
  const parsed = await response.json();
  if (!parsed.web || !Array.isArray(parsed.web.results)) throw new Error("Invalid Brave API response");
  const results = validSearchResults(parsed.web.results.map((entry) => ({ title: entry.title, url: entry.url, text: entry.description })), limit);
  if (results.length === 0) throw new Error("Brave returned no usable results");
  return asSearchData(query, "brave-js", "Brave Search API via JS runtime", results);
}

async function searchSearxng(query, limit) {
  const base = String(process.env.SEARXNG_INSTANCE_URL || process.env.WEFT_SEARXNG_INSTANCE_URL || "").trim().replace(/\/+$/, "");
  if (!base) throw new Error("SearXNG instance URL not configured");
  const response = await fetch(`${base}/search?q=${encodeURIComponent(query)}&format=json&pageno=1`, {
    headers: { accept: "application/json", "user-agent": "WEFT JS Tool Runtime" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`SearXNG search failed with status: ${response.status}`);
  const parsed = await response.json();
  if (!Array.isArray(parsed.results)) throw new Error("Invalid SearXNG API response");
  const results = validSearchResults(parsed.results.map((entry) => ({ title: entry.title, url: entry.url, text: entry.content, score: entry.score })), limit);
  if (results.length === 0) throw new Error("SearXNG returned no usable results");
  return asSearchData(query, "searxng-js", "SearXNG JSON API via JS runtime", results);
}

function unwrapDuckDuckGoRedirect(rawUrl) {
  let url = String(rawUrl || "").replace(/&amp;/g, "&").trim();
  try {
    const parsed = new URL(url.startsWith("//") ? `https:${url}` : url);
    const uddg = parsed.searchParams.get("uddg");
    if (uddg) url = decodeURIComponent(uddg);
  } catch {
    const match = url.match(/[?&]uddg=([^&]+)/);
    if (match) url = decodeURIComponent(match[1]);
  }
  return url;
}

function isOrganicSearchResultUrl(url) {
  if (!/^https?:\/\//i.test(url)) return false;
  try {
    const parsed = new URL(url);
    if (/duckduckgo\.com$/i.test(parsed.hostname) && parsed.pathname.includes("/y.js")) return false;
    if (/bing\.com$/i.test(parsed.hostname) && parsed.pathname.includes("/aclick")) return false;
  } catch {
    return false;
  }
  return true;
}

function parseDuckDuckGo(html, limit) {
  const results = [];
  const bodyRegex = /<div[^>]*class="[^"]*result__body[^"]*"[^>]*>([\s\S]*?)(?=<div[^>]*class="[^"]*result__body[^"]*"|<div[^>]*class="[^"]*nav-link|<\/body>|$)/gi;
  let bodyMatch;
  while ((bodyMatch = bodyRegex.exec(html)) && results.length < limit) {
    const block = bodyMatch[1];
    const linkMatch = block.match(/<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/i);
    if (!linkMatch) continue;
    const title = htmlToText(linkMatch[2]);
    const url = unwrapDuckDuckGoRedirect(linkMatch[1]);
    const snippetMatch = block.match(/<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>([\s\S]*?)<\/a>/i)
      || block.match(/<div[^>]*class="[^"]*result__snippet[^"]*"[^>]*>([\s\S]*?)<\/div>/i)
      || block.match(/<span[^>]*class="[^"]*result__snippet[^"]*"[^>]*>([\s\S]*?)<\/span>/i);
    const text = snippetMatch ? htmlToText(snippetMatch[1]) : "";
    if (title && isOrganicSearchResultUrl(url)) results.push({ title, url, text, score: 0 });
  }
  return results;
}

async function searchDuckDuckGo(query, limit, diagnostics = []) {
  const url = `https://html.duckduckgo.com/html/?q=${encodeURIComponent(query)}`;
  const response = await fetch(url, {
    headers: {
      "user-agent": "Mozilla/5.0 (compatible; WEFT JS WebSearch/1.0)",
      accept: "text/html,application/xhtml+xml",
    },
    signal: AbortSignal.timeout(12000),
  });
  if (!response.ok) throw new Error(`DuckDuckGo search failed with status: ${response.status}`);
  const html = await response.text();
  if (/Unfortunately, bots use DuckDuckGo too|anomaly\.js\?|anomalyDetectionBlock|id="challenge-form"|name="challenge"|g-recaptcha/i.test(html)) {
    throw new Error("DuckDuckGo returned a bot-detection challenge");
  }
  const results = parseDuckDuckGo(html, limit);
  if (results.length > 0) return asSearchData(query, "duckduckgo-js", "DuckDuckGo HTML via JS runtime", results, diagnostics);
  throw new Error("DuckDuckGo returned no parseable results");
}

async function searchBingHtml(query, limit, diagnostics = []) {
  const response = await fetch(`https://www.bing.com/search?q=${encodeURIComponent(query)}&count=${limit}`, {
    headers: {
      "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
      accept: "text/html,application/xhtml+xml",
      "accept-language": "zh-CN,zh;q=0.9,en;q=0.8",
    },
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) throw new Error(`Bing search failed with status: ${response.status}`);
  const html = await response.text();
  const results = [];
  const regex = /<li[^>]*class="[^"]*b_algo[^"]*"[^>]*>([\s\S]*?)(?=<li[^>]*class="[^"]*b_algo[^"]*"|<\/ol>|$)/gi;
  let match;
  while ((match = regex.exec(html)) && results.length < limit) {
    const block = match[1];
    const linkMatch = block.match(/<h2[^>]*>[\s\S]*?<a[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]*?<\/h2>/i)
      || block.match(/<a[^>]*href="(https?:\/\/[^"]+)"[^>]*>([\s\S]*?)<\/a>/i);
    if (!linkMatch) continue;
    const url = linkMatch[1].replace(/&amp;/g, "&");
    const title = htmlToText(linkMatch[2]);
    const snippetMatch = block.match(/<p[^>]*>([\s\S]*?)<\/p>/i)
      || block.match(/<div[^>]*class="[^"]*b_caption[^"]*"[^>]*>[\s\S]*?<p[^>]*>([\s\S]*?)<\/p>/i);
    const text = snippetMatch ? htmlToText(snippetMatch[1]) : "";
    if (title && isOrganicSearchResultUrl(url)) results.push({ title, url, text, score: 0 });
  }
  if (results.length === 0) throw new Error("Bing returned no parseable results");
  return asSearchData(query, "bing-html-js", "Bing HTML via JS runtime", results, diagnostics);
}

async function searchMojeek(query, limit, diagnostics = []) {
  const response = await fetch(`https://www.mojeek.com/search?q=${encodeURIComponent(query)}`, {
    headers: { "user-agent": "Mozilla/5.0 WEFT JS Tool Runtime", accept: "text/html,application/xhtml+xml" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Mojeek search failed with status: ${response.status}`);
  const html = await response.text();
  const results = [];
  const regex = /<a[^>]+href="(https?:\/\/[^"]+)"[^>]*>([\s\S]*?)<\/a>[\s\S]{0,600}?(?:<p[^>]*>|<span[^>]*class="[^"]*(?:s|desc|snippet)[^"]*"[^>]*>)([\s\S]*?)(?:<\/p>|<\/span>)/gi;
  let match;
  while ((match = regex.exec(html)) && results.length < limit) {
    if (/mojeek\.com/i.test(match[1])) continue;
    results.push({ title: htmlToText(match[2]), url: match[1], text: htmlToText(match[3]), score: 0 });
  }
  if (results.length === 0) throw new Error("Mojeek returned no parseable results");
  return asSearchData(query, "mojeek-js", "Mojeek HTML via JS runtime", results, diagnostics);
}

async function searchWikipedia(query, limit, diagnostics = []) {
  const response = await fetch(`https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(query)}&format=json&srlimit=${limit}&origin=*`, {
    headers: { "user-agent": "WEFT JS Tool Runtime" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Wikipedia search failed with status: ${response.status}`);
  const parsed = await response.json();
  const results = validSearchResults(Array.isArray(parsed.query && parsed.query.search) ? parsed.query.search.map((entry) => ({ title: entry.title, url: `https://en.wikipedia.org/wiki/${encodeURIComponent(String(entry.title || "").replace(/ /g, "_"))}`, text: entry.snippet })) : [], limit);
  if (results.length === 0) throw new Error("Wikipedia returned no usable results");
  return asSearchData(query, "wikipedia-js", "Wikipedia Search API via JS runtime", results, diagnostics);
}

async function searchHackerNewsAlgolia(query, limit, diagnostics = []) {
  const response = await fetch(`https://hn.algolia.com/api/v1/search?query=${encodeURIComponent(query)}&hitsPerPage=${limit}`, {
    headers: { accept: "application/json", "user-agent": "WEFT JS Tool Runtime" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`HN Algolia search failed with status: ${response.status}`);
  const parsed = await response.json();
  const results = validSearchResults(Array.isArray(parsed.hits) ? parsed.hits.map((entry) => ({ title: entry.title || entry.story_title, url: entry.url || `https://news.ycombinator.com/item?id=${entry.objectID}`, text: entry.comment_text || entry.story_text || "" })) : [], limit);
  if (results.length === 0) throw new Error("HN Algolia returned no usable results");
  return asSearchData(query, "hn-algolia-js", "Hacker News Algolia API via JS runtime", results, diagnostics);
}

async function searchFactoryDocs(query, limit, diagnostics = []) {
  if (!/factory|droid|weft|byok|computer|workspace|mission|mcp|skill|hook/i.test(query)) throw new Error("Factory docs fallback is not relevant to this query");
  const response = await fetch("https://docs.factory.ai/llms.txt", {
    headers: { "user-agent": "WEFT JS Tool Runtime", accept: "text/plain,*/*" },
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Factory docs search failed with status: ${response.status}`);
  const text = await response.text();
  const lower = query.toLowerCase().split(/\s+/).filter((part) => part.length > 2);
  const lines = text.split(/\r?\n/);
  const results = [];
  for (let index = 0; index < lines.length && results.length < limit; index += 1) {
    const line = lines[index];
    const url = (line.match(/https:\/\/docs\.factory\.ai\/[^\s)]+/) || [])[0];
    if (!url) continue;
    const haystack = `${line} ${lines[index - 1] || ""} ${lines[index + 1] || ""}`.toLowerCase();
    if (lower.length > 0 && !lower.some((term) => haystack.includes(term))) continue;
    results.push({ title: htmlToText((lines[index - 1] || "Factory documentation").replace(/^#+\s*/, "")) || "Factory documentation", url, text: htmlToText(line), score: 0 });
  }
  if (results.length === 0) {
    for (const url of text.match(/https:\/\/docs\.factory\.ai\/[^\s)]+/g) || []) {
      if (results.length >= limit) break;
      results.push({ title: "Factory documentation", url, text: url, score: 0 });
    }
  }
  if (results.length === 0) throw new Error("Factory docs returned no links");
  return asSearchData(query, "factory-docs-js", "Factory docs llms.txt via JS runtime", results, diagnostics);
}

async function searchExaApi(query, limit, diagnostics = []) {
  const key = process.env.EXA_API_KEY || process.env.WEFT_EXA_API_KEY;
  if (!key) throw new Error("Exa API key not configured");
  const response = await fetch("https://api.exa.ai/search", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      accept: "application/json",
      "x-api-key": key,
    },
    body: JSON.stringify({
      query,
      numResults: limit,
      type: "auto",
      contents: {
        text: { maxCharacters: 1200 },
        highlights: { numSentences: 2, highlightsPerUrl: 2 },
      },
    }),
    signal: AbortSignal.timeout(20000),
  });
  if (!response.ok) throw new Error(`Exa search failed with status: ${response.status}: ${(await response.text()).slice(0, 300)}`);
  const parsed = await response.json();
  const results = validSearchResults(Array.isArray(parsed.results) ? parsed.results.map((entry) => ({
    title: entry.title,
    url: entry.url,
    text: entry.text || entry.summary || (Array.isArray(entry.highlights) ? entry.highlights.join(" ") : ""),
    score: entry.score,
  })) : [], limit);
  if (results.length === 0) throw new Error("Exa returned no usable results");
  return asSearchData(query, "exa-js", "Exa Search API via JS runtime", results, diagnostics);
}

async function searchExaMcp(query, limit, diagnostics = []) {
  const key = process.env.EXA_API_KEY || process.env.WEFT_EXA_API_KEY;
  if (!key) throw new Error("Exa API key not configured");
  const response = await fetch("https://mcp.exa.ai/mcp", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json, text/event-stream", authorization: `Bearer ${key}` },
    body: JSON.stringify({ jsonrpc: "2.0", id: "weft-web-search", method: "tools/call", params: { name: "web_search_exa", arguments: { query, type: "auto", numResults: limit, livecrawl: "fallback", contextMaxCharacters: 10000 } } }),
    signal: AbortSignal.timeout(25000),
  });
  if (!response.ok) throw new Error(`Exa MCP search failed with status: ${response.status}`);
  const raw = await response.text();
  const lines = raw.split(/\r?\n/).filter((line) => line.startsWith("data: "));
  for (const line of lines) {
    const parsed = JSON.parse(line.slice(6));
    const text = parsed && parsed.result && Array.isArray(parsed.result.content) ? String(parsed.result.content[0] && parsed.result.content[0].text || "") : "";
    if (text.trim()) return asSearchData(query, "exa-mcp-js", "Exa MCP web_search_exa via JS runtime", [{ title: `Web search: ${query}`, url: "https://mcp.exa.ai/mcp", text, score: 1 }], diagnostics, text);
  }
  throw new Error("Exa MCP returned no text content");
}

async function builtinSearchWeb(args) {
  const query = resolveSearchQuery(args);
  if (!query) return { status: "error", error: "missing query" };
  const wrap = (data) => data && (data.status === "ok" || data.status === "error") ? data : { status: "ok", data };
  const limit = normalizeSearchLimit(args || {});
  const diagnostics = [];
  const preferred = resolveSearchProvider(args && args.provider);
  if (preferred.fallback) diagnostics.push(`Unknown provider '${preferred.unknown}', using automatic fallback chain.`);
  const chain = [];
  if (preferred.provider !== "auto") chain.push(preferred.provider);
  if ((args && args.use_exa) || process.env.EXA_API_KEY || process.env.WEFT_EXA_API_KEY) chain.push("exa");
  if (process.env.TAVILY_API_KEY || process.env.WEFT_TAVILY_API_KEY) chain.push("tavily");
  if (process.env.BRAVE_API_KEY || process.env.WEFT_BRAVE_API_KEY || process.env.BRAVE_SEARCH_API_KEY) chain.push("brave");
  if (process.env.SEARXNG_INSTANCE_URL || process.env.WEFT_SEARXNG_INSTANCE_URL) chain.push("searxng");
  chain.push("duckduckgo");
  chain.push("bing");
  chain.push("wikipedia");
  chain.push("hn");
  chain.push("mojeek");
  for (const provider of [...new Set(chain)]) {
    try {
      if (provider === "exa") {
        try {
          return wrap(await searchExaApi(query, limit, diagnostics));
        } catch (apiError) {
          diagnostics.push(`exa-api: ${apiError instanceof Error ? apiError.message : String(apiError)}`);
          return wrap(await searchExaMcp(query, limit, diagnostics));
        }
      }
      if (provider === "tavily") return wrap(await searchTavily(query, limit));
      if (provider === "brave") return wrap(await searchBrave(query, limit));
      if (provider === "searxng") return wrap(await searchSearxng(query, limit));
      if (provider === "duckduckgo") return wrap(await searchDuckDuckGo(query, limit, diagnostics));
      if (provider === "bing") return wrap(await searchBingHtml(query, limit, diagnostics));
      if (provider === "wikipedia") return wrap(await searchWikipedia(query, limit, diagnostics));
      if (provider === "hn") return wrap(await searchHackerNewsAlgolia(query, limit, diagnostics));
      if (provider === "mojeek") return wrap(await searchMojeek(query, limit, diagnostics));
      return wrap(await searchDuckDuckGo(query, limit, diagnostics));
    } catch (error) {
      diagnostics.push(`${provider}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return { status: "ok", data: emptySearchResult(query, new Error(diagnostics.join("; "))) };
}

async function executeExtension(body) {
  if (isBuiltinToolExecution(body || {})) {
    return executeBuiltinTool(body || {});
  }

  const resolved = resolveExtensionModule(body || {});
  if (resolved.error) {
    return { statusCode: resolved.statusCode, body: errorBody(resolved.error, resolved.message) };
  }

  const file = resolved.file;
  const extensionId = resolved.id || inferExtensionIdForFile(file);
  const timeoutMs = Math.max(1, Number(body.timeoutMs || body.timeout || DEFAULT_EXECUTION_TIMEOUT_MS));
  const audit = auditMetadataForExtension(file, extensionId, timeoutMs);
  if (!isExtensionAllowlisted(extensionId, file)) {
    return {
      statusCode: 403,
      body: errorBody("extension_not_allowlisted", "Extension execution is blocked by WEFT_JS_EXTENSION_ALLOWLIST.", audit),
    };
  }

  const extensionRelativePath = path.relative(RESOURCE_DIRS.extensions, file).replace(/\\/g, "/");
  const executionFile = path.extname(file).toLowerCase() === ".ts" ? transpileTypeScriptToCache(file) : file;
  const moduleExports = await loadExtensionModule(executionFile);
  const handler = resolveHandler(moduleExports);
  if (!handler) {
    return {
      statusCode: 400,
      body: errorBody("handler_not_found", "Extension module must export a default function or handle function."),
    };
  }

  const payload = Object.prototype.hasOwnProperty.call(body, "payload") ? body.payload : body;
  const context = Object.freeze({
    service: "js-extension-runtime",
    workspaceRoot: ROOT,
    weftDir: WEFT_DIR,
    extensionsDir: RESOURCE_DIRS.extensions,
    extensionPath: file,
    extensionRelativePath,
  });

  let timeoutHandle;
  const timeoutPromise = new Promise((_, reject) => {
    timeoutHandle = setTimeout(() => {
      const error = new Error(`Extension execution timed out after ${timeoutMs}ms.`);
      error.code = "execution_timeout";
      error.statusCode = 504;
      reject(error);
    }, timeoutMs);
  });

  try {
    const result = await Promise.race([handler(payload, context), timeoutPromise]);
    return { statusCode: 200, body: { ok: true, extension: extensionRelativePath, audit, result } };
  } finally {
    clearTimeout(timeoutHandle);
  }
}

function errorBody(code, message, details) {
  const body = {
    ok: false,
    error: {
      code,
      message,
    },
  };
  if (details !== undefined) {
    body.error.details = details;
  }
  return body;
}

function sendJson(res, statusCode, body) {
  const data = JSON.stringify(body, null, 2);
  res.writeHead(statusCode, {
    "content-type": "application/json; charset=utf-8",
    "content-length": Buffer.byteLength(data),
  });
  res.end(data);
}

function route(req, res) {
  const url = new URL(req.url, `http://${req.headers.host || `${HOST}:${PORT}`}`);

  if (req.method === "POST" && url.pathname === "/execute") {
    return readRequestJson(req, (error, body = {}) => {
      if (error) {
        return sendJson(res, error.statusCode || 400, errorBody(error.code || "invalid_json", error.message));
      }

      executeExtension(body)
        .then((response) => sendJson(res, response.statusCode, response.body))
        .catch((executionError) => sendJson(res, executionError.statusCode || 500,
          errorBody(
            executionError.code || "execution_failed",
            executionError.statusCode ? executionError.message : "Extension execution failed.",
            executionError.statusCode ? undefined : executionError.message
          )));
    });
  }

  if (req.method === "POST" && url.pathname === "/reload") {
    return sendJson(res, 200, scanResources());
  }

  if (req.method === "POST" && url.pathname === "/render-prompt") {
    return readRequestJson(req, (error, body = {}) => {
      if (error) {
        return sendJson(res, 400, { error: "invalid_json" });
      }

      const prompt = readPrompt(body.id);
      if (!prompt) {
        return sendJson(res, 404, { error: "prompt_not_found" });
      }

      return sendJson(res, 200, {
        id: prompt.id,
        path: prompt.path,
        content: renderPromptTemplate(prompt.content, body.variables, body.args),
        variables: prompt.variables,
        arg_placeholders: prompt.arg_placeholders,
        argument_hints: prompt.argument_hints,
      });
    });
  }

  if (req.method === "POST" && url.pathname === "/render-skill") {
    return readRequestJson(req, (error, body = {}) => {
      if (error) {
        return sendJson(res, 400, { error: "invalid_json" });
      }

      const skill = readSkill(body.id);
      if (!skill) {
        return sendJson(res, 404, { error: "skill_not_found" });
      }

      return sendJson(res, 200, { skill });
    });
  }

  if (req.method === "GET" && url.pathname.startsWith("/skill/")) {
    const id = decodeURIComponent(url.pathname.slice("/skill/".length));
    const skill = readSkill(id);
    if (!skill) {
      return sendJson(res, 404, { error: "skill_not_found" });
    }

    return sendJson(res, 200, { skill });
  }

  if (req.method !== "GET") {
    return sendJson(res, 405, { error: "method_not_allowed" });
  }

  if (url.pathname === "/health") {
    return sendJson(res, 200, { ok: true, service: "js-extension-runtime" });
  }

  if (url.pathname === "/metadata" || url.pathname === "/resources") {
    return sendJson(res, 200, scanResources());
  }

  if (url.pathname === "/metadata/extensions" || url.pathname === "/extensions") {
    return sendJson(res, 200, { extensions: scanResourceType("extensions") });
  }

  if (url.pathname === "/metadata/skills" || url.pathname === "/skills") {
    return sendJson(res, 200, { skills: scanResourceType("skills") });
  }

  if (url.pathname === "/metadata/prompts" || url.pathname === "/prompts") {
    return sendJson(res, 200, { prompts: scanResourceType("prompts") });
  }

  return sendJson(res, 404, { error: "not_found" });
}

const server = http.createServer(route);
server.listen(PORT, HOST, () => {
  console.log(`js-extension-runtime listening on http://${HOST}:${PORT}`);
});

function shutdown() {
  server.close(() => process.exit(0));
}

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

