import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";

export const sha = bytes => crypto.createHash("sha256").update(bytes).digest("hex");
export const jsonBytes = value => Buffer.from(JSON.stringify(value, null, 2) + "\n");
export function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map(k => [k, canonical(value[k])]));
  return value;
}
export const identity = value => sha(jsonBytes(canonical(value)));
export function requireValue(ok, message) { if (!ok) throw new Error(message); }
export function argsOf(argv) {
  const [command, ...args] = argv; const options = {};
  for (let i = 0; i < args.length; i += 2) {
    requireValue(args[i].startsWith("--") && args[i + 1] && !args[i + 1].startsWith("--"), `expected --option value: ${args[i]}`);
    requireValue(!Object.hasOwn(options, args[i].slice(2)), "duplicate option");
    options[args[i].slice(2)] = args[i + 1];
  }
  return { command, options };
}
export function readJson(file) { return JSON.parse(fs.readFileSync(file, "utf8").replace(/^\uFEFF/, "")); }
export function externalDirectory(directory) {
  requireValue(directory && path.isAbsolute(directory), "output/run directory must be absolute");
  let existing = path.resolve(directory);
  while (!fs.existsSync(existing)) existing = path.dirname(existing);
  const resolved = fs.realpathSync(existing);
  let probe = resolved;
  for (;;) {
    requireValue(!fs.existsSync(path.join(probe, ".git")), "runtime artifacts must stay outside Git");
    const parent = path.dirname(probe); if (parent === probe) break; probe = parent;
  }
  fs.mkdirSync(directory, { recursive: true });
  return fs.realpathSync(directory);
}
export function within(root, file) {
  const resolved = fs.realpathSync(file);
  const relative = path.relative(fs.realpathSync(root), resolved);
  requireValue(relative && !relative.startsWith("..") && !path.isAbsolute(relative), `path escaped bundle: ${file}`);
  return resolved;
}
export function atomicJson(file, value) {
  const temporary = `${file}.${crypto.randomUUID()}.tmp`;
  try { fs.writeFileSync(temporary, jsonBytes(value), { flag: "wx" }); fs.renameSync(temporary, file); }
  finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
}
export function lock(directory) {
  const file = path.join(directory, ".operation.lock");
  const fd = fs.openSync(file, "wx");
  fs.writeFileSync(fd, jsonBytes({ pid: process.pid, started: new Date().toISOString() }));
  return () => { fs.closeSync(fd); fs.unlinkSync(file); };
}
export function verifyFiles(directory, files) {
  for (const file of files) {
    requireValue(typeof file.path === "string" && !path.isAbsolute(file.path), "invalid relative artifact path");
    const actual = within(directory, path.join(directory, file.path));
    requireValue(sha(fs.readFileSync(actual)) === file.sha256, `artifact hash mismatch: ${file.path}`);
  }
}
export async function dependency(name) {
  const require = createRequire(import.meta.url);
  let location;
  try { location = require.resolve(name); }
  catch {
    requireValue(process.env.BABATA_NODE_MODULES, `${name} unavailable; install atoms package dependencies or set BABATA_NODE_MODULES`);
    location = require.resolve(name, { paths: [process.env.BABATA_NODE_MODULES] });
  }
  const loaded = await import(pathToFileURL(location).href);
  return loaded.default ?? loaded;
}
export function isMain(meta) { return process.argv[1] && meta.url === pathToFileURL(path.resolve(process.argv[1])).href; }
export function emitError(error) { process.stderr.write(JSON.stringify({ status: "failed", reason: error.message }) + "\n"); process.exitCode = 1; }
