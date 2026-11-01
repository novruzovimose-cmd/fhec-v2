import { createHash } from "node:crypto";
import { lstatSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

export function jsonBytes(value) {
  return Buffer.from(JSON.stringify(value, null, 2) + "\n", "utf8");
}

export function readJson(path) {
  const content = readFileSync(path, "utf8").replace(/^\uFEFF/, "");
  return JSON.parse(content);
}

export function writeJsonNew(path, value) {
  writeFileSync(path, jsonBytes(value), { flag: "wx" });
}

export function assertRegularFile(path, maxBytes = 8 * 1024 * 1024) {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error("Expected a regular, non-symlink file: " + path);
  }
  if (stat.size > maxBytes || stat.size < 0) {
    throw new Error("File exceeds a safety size bound: " + path);
  }
  return stat;
}

export function safeJoin(root, name) {
  if (typeof name !== "string" || !name || isAbsolute(name) || name.includes("\\") ||
      name.split("/").some((part) => !part || part === "." || part === "..")) {
    throw new Error("Unsafe relative file name");
  }
  const full = resolve(root, ...name.split("/"));
  if (relative(resolve(root), full).startsWith(".." + sep) || full === resolve(root)) {
    throw new Error("Path escapes the intended directory");
  }
  return full;
}

export function fileEntry(root, name) {
  const path = safeJoin(root, name);
  const stat = assertRegularFile(path);
  return { file: name, bytes: stat.size, sha256: sha256(readFileSync(path)) };
}

export function identical(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function requireSame(left, right, message) {
  if (!identical(left, right)) throw new Error(message);
}

export function allUnique(values) {
  return new Set(values).size === values.length;
}

export function rootInside(root, path) {
  const relativePath = relative(resolve(root), resolve(path));
  return relativePath && relativePath !== ".." && !relativePath.startsWith(".." + sep) && !isAbsolute(relativePath);
}

export function relativeFile(root, name) {
  return join(root, ...name.split("/"));
}
