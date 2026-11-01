import { createHash, randomUUID } from "node:crypto";
import { existsSync, lstatSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";

const maximumJsonBytes = 24 * 1024 * 1024;

export function hash(value) {
  return createHash("sha256").update(value).digest("hex").toUpperCase();
}

export function assertDigest(value, label = "SHA-256") {
  if (typeof value !== "string" || !/^[A-Fa-f0-9]{64}$/.test(value)) {
    throw new Error(`Invalid ${label}`);
  }
  return value.toUpperCase();
}

export function boundedFile(file, maximumBytes = maximumJsonBytes) {
  const location = resolve(file);
  const entry = lstatSync(location);
  if (!entry.isFile() || entry.isSymbolicLink() || entry.size > maximumBytes || entry.size < 0) {
    throw new Error("Unsafe, symlinked, or oversized file: " + location);
  }
  return readFileSync(location);
}

export function jsonFile(file, maximumBytes = maximumJsonBytes) {
  const content = boundedFile(file, maximumBytes).toString("utf8").replace(/^\uFEFF/, "");
  const parsed = JSON.parse(content);
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("Expected a JSON document object");
  }
  return parsed;
}

export function safeName(name) {
  if (typeof name !== "string" || name.length > 240 || !name || name.startsWith("/") ||
      name.includes("\\") || name.includes(":") || name.split("/").some((part) => !part || part === "." || part === "..")) {
    throw new Error("Unsafe relative file name");
  }
  return name;
}

export function within(root, child) {
  const parent = resolve(root);
  const target = resolve(child);
  const rel = relative(parent, target);
  if (rel === "" || (!rel.startsWith(".." + sep) && rel !== ".." && !isAbsolute(rel))) return target;
  throw new Error("Path is outside the approved project directory");
}

export function writeNewJson(file, value) {
  writeFileSync(file, JSON.stringify(value, null, 2) + "\n", { flag: "wx", encoding: "utf8" });
}

export function newOutput(project, category) {
  const root = resolve(project);
  if (!existsSync(join(root, "Cargo.toml")) || !existsSync(join(root, "tools", "cofhe", "phase12", "verify.mjs"))) {
    throw new Error("Not an installed FHEC V2 project with Phase 12");
  }
  const target = join(root, "target");
  if (existsSync(target) && (lstatSync(target).isSymbolicLink() || !lstatSync(target).isDirectory())) {
    throw new Error("Project target folder must be a real directory");
  }
  mkdirSync(target, { recursive: true });
  const dir = join(target, `cofhe-phase13-${category}-${new Date().toISOString().slice(0, 10)}-${randomUUID().slice(0, 8)}`);
  mkdirSync(dir);
  return dir;
}

export function checkedUrl(value) {
  if (typeof value !== "string" || value.length > 8192) throw new Error("Invalid RPC endpoint");
  let url;
  try { url = new URL(value); } catch { throw new Error("RPC endpoint must be an absolute HTTPS URL"); }
  if (url.protocol !== "https:" || url.username || url.password || url.hash ||
      url.hostname === "localhost" || url.hostname === "127.0.0.1" || url.hostname === "::1") {
    throw new Error("Only public-host HTTPS RPC endpoints without embedded basic authentication are permitted");
  }
  return url;
}

export function redactError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return message.replace(/https?:\/\/[^\s"']+/g, "[redacted endpoint]").replace(/0x[a-fA-F0-9]{64}/g, "[redacted hex]").slice(0, 700);
}
