import { createHash } from "node:crypto";
import { readFileSync, lstatSync, realpathSync } from "node:fs";
import { isAbsolute, relative, resolve, sep } from "node:path";

export function fail(message) {
  throw new Error(message);
}

export function object(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  return value;
}

export function integer(value, label, min = 0, max = Number.MAX_SAFE_INTEGER) {
  if (!Number.isSafeInteger(value) || value < min || value > max) fail(`${label} must be an integer between ${min} and ${max}`);
  return value;
}

export function exact(value, expected, label) {
  if (value !== expected) fail(`${label} must equal ${expected}`);
  return value;
}

export function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

export function digest(path) {
  if (!lstatSync(path).isFile()) fail(`Not a regular file: ${path}`);
  return sha256(readFileSync(path));
}

export function hex(value, label, bytes) {
  if (typeof value !== "string" || !new RegExp(`^0x[0-9a-fA-F]{${2 * bytes}}$`).test(value)) {
    fail(`${label} must be a ${bytes}-byte 0x-prefixed hex string`);
  }
  return value.toLowerCase();
}

export function quantity(value, label) {
  if (typeof value !== "string" || !/^0x(?:0|[1-9a-fA-F][0-9a-fA-F]*)$/.test(value)) {
    fail(`${label} must be a canonical JSON-RPC hex quantity`);
  }
  return BigInt(value);
}

export function jsonFile(path, label = "JSON file") {
  if (!lstatSync(path).isFile()) fail(`${label} is not a regular file`);
  const size = lstatSync(path).size;
  if (size > 20 * 1024 * 1024) fail(`${label} exceeds the 20 MiB safety bound`);
  try {
    const raw = readFileSync(path, "utf8");
    const content = raw.startsWith("\uFEFF") ? raw.slice(1) : raw;
    return object(JSON.parse(content), label);
  } catch (error) {
    fail(`Invalid ${label}: ${error.message}`);
  }
}

export function within(root, path) {
  const canonicalRoot = realpathSync(root);
  const canonicalPath = realpathSync(path);
  const subpath = relative(canonicalRoot, canonicalPath);
  if (subpath === "" || (!subpath.startsWith(`..${sep}`) && subpath !== ".." && !isAbsolute(subpath))) {
    return canonicalPath;
  }
  fail(`Path escapes expected root: ${path}`);
}

export function options(args, spec) {
  const out = {};
  for (let i = 0; i < args.length; i += 2) {
    const name = args[i];
    if (!Object.hasOwn(spec, name) || i + 1 >= args.length || Object.hasOwn(out, spec[name])) {
      fail(`Unknown, duplicate or incomplete option: ${name ?? "(missing)"}`);
    }
    out[spec[name]] = args[i + 1];
  }
  for (const required of Object.values(spec)) {
    if (!Object.hasOwn(out, required)) fail(`Missing required option: ${required}`);
  }
  return out;
}

export function requireNewPath(path) {
  const target = resolve(path);
  if (target.length > 4096) fail("Output path is too long");
  return target;
}
