import fs from "node:fs";
import path from "node:path";
import { argsOf, atomicJson, canonical, emitError, externalDirectory, identity, isMain, jsonBytes, lock, readJson, requireValue, sha, verifyFiles, within } from "./common.mjs";

const schema = "babata.research-run/v1";
const commandInputs = { init: "request", record: "event", finish: "conclusion", status: null, verify: null };
const usage = `Recoverable research (Node built-ins only; no repository required)
  node <skill>/scripts/research.mjs <init|record|finish|status|verify> <location> [input]
  location: --run <absolute-directory>
         or --workspace <absolute-directory> --run-id <stable-id>
         or --run-id <stable-id> (uses BABATA_ASSISTANT_HOME/solutions/research/workspaces)
  input: init --request <json>; record --event <json>; finish --conclusion <json>
  status/verify require an existing run. No configuration is changed.
`;
function text(value) { return typeof value === "string" && value.trim().length > 0; }
export function resolveResearchRun(options, env = process.env) {
  if (Object.hasOwn(options, "run")) {
    requireValue(!Object.hasOwn(options, "workspace") && !Object.hasOwn(options, "run-id"), "--run cannot be combined with --workspace or --run-id");
    requireValue(text(options.run) && path.isAbsolute(options.run), "--run must be an absolute directory");
    return path.resolve(options.run);
  }
  const id = options["run-id"];
  requireValue(text(id) && /^[A-Za-z0-9][A-Za-z0-9_-]{0,79}$/.test(id) && !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(id), "--run-id must be a stable portable ID (1-80 letters/digits/_/-, starting with a letter/digit; no reserved device names)");
  let workspace;
  if (Object.hasOwn(options, "workspace")) {
    workspace = options.workspace;
    requireValue(text(workspace) && path.isAbsolute(workspace), "--workspace must be an absolute directory");
  } else {
    const assistant = env.BABATA_ASSISTANT_HOME;
    requireValue(text(assistant) && path.isAbsolute(assistant), "Assistant default unavailable; pass --workspace <absolute-dir> --run-id <id> or --run <absolute-dir>");
    workspace = path.join(assistant, "solutions", "research", "workspaces");
  }
  return path.resolve(workspace, id);
}
function validateRequest(request) {
  requireValue(request && text(request.question) && text(request.producer), "question and producer required");
  requireValue(["sources", "research"].includes(request.depth), "depth must be sources or research");
  requireValue(text(request.scope), "explicit scope required");
}
function validateEvent(event) {
  requireValue(text(event.id) && /^[a-zA-Z0-9_-]{1,80}$/.test(event.id), "invalid event id");
  requireValue(["search", "body"].includes(event.kind), "event kind must be search or body");
  requireValue(text(event.provider) && text(event.observed_at) && Number.isFinite(Date.parse(event.observed_at)), "provider and actual observation timestamp required");
  requireValue(["succeeded", "failed", "inaccessible"].includes(event.status), "invalid observation status");
  if (event.status !== "succeeded") requireValue(text(event.reason), "failed observation requires reason");
  if (event.kind === "search") {
    requireValue(text(event.query) && Array.isArray(event.results), "query and search results required");
    for (const result of event.results) {
      requireValue(text(result.url) && /^https?:\/\//.test(result.url) && text(result.title), "search result requires HTTP URL and title");
    }
  } else {
    requireValue(text(event.url) && /^https?:\/\//.test(event.url), "body event requires HTTP URL");
    requireValue(["full", "relevant-excerpt", "none"].includes(event.coverage), "body coverage required");
    if (event.status === "succeeded") requireValue(text(event.body) && event.coverage !== "none", "successful body event requires acquired text and coverage");
    else requireValue(!text(event.body) && event.coverage === "none", "unreadable body cannot claim acquired text");
  }
}
function validateConclusion(conclusion, events, request) {
  requireValue(text(conclusion.markdown) && text(conclusion.producer) && Array.isArray(conclusion.claims) && Array.isArray(conclusion.limitations), "conclusion requires markdown, producer, claims and limitations");
  const bodyIds = new Set(events.filter(e => e.kind === "body" && e.status === "succeeded").map(e => e.id));
  for (const claim of conclusion.claims) {
    requireValue(text(claim.text) && ["evidence", "inference", "assumption", "recommendation", "unresolved"].includes(claim.kind) && Array.isArray(claim.sources), "invalid claim attribution");
    requireValue(claim.sources.every(id => bodyIds.has(id)), "claim references must name acquired body event IDs");
    if (["evidence", "inference"].includes(claim.kind)) requireValue(claim.sources.length > 0, "evidence/inference requires acquired body event IDs");
  }
  requireValue(events.length > 0, "cannot finish without actual observations");
  if (request.depth === "research") requireValue(bodyIds.size > 0 && conclusion.claims.length > 0, "research requires acquired bodies and attributed claims; use sources depth for locator-only discovery");
}
function manifestAt(directory) {
  const manifest = readJson(path.join(directory, "manifest.json"));
  requireValue(manifest.schema_version === schema && Array.isArray(manifest.files), "invalid research run");
  verifyFiles(directory, manifest.files);
  requireValue(["working", "finalized"].includes(manifest.state) && Array.isArray(manifest.events), "invalid research lifecycle");
  requireValue(new Set(manifest.files.map(f => f.path)).size === manifest.files.length && new Set(manifest.events).size === manifest.events.length, "duplicate research entries");
  const expected = new Map([["request.json", "request"], ...manifest.events.map(id => [`event-${id}.json`, "observation"])]);
  if (manifest.state === "finalized") { expected.set("conclusion.json", "conclusion"); expected.set("report.md", "draft"); }
  requireValue(expected.size === manifest.files.length && manifest.files.every(f => expected.get(f.path) === f.role), "research manifest file roles/completeness mismatch");
  const request = readJson(path.join(directory, "request.json")); validateRequest(request);
  requireValue(identity(request) === manifest.request_sha256, "request identity mismatch");
  const events = manifest.events.map(id => { const event = readJson(within(directory, path.join(directory, `event-${id}.json`))); validateEvent(event); requireValue(event.id === id, "observation identity mismatch"); return event; });
  if (manifest.state === "finalized") {
    const conclusion = readJson(within(directory, path.join(directory, "conclusion.json")));
    validateConclusion(conclusion, events, request);
    requireValue(fs.readFileSync(within(directory, path.join(directory, "report.md")), "utf8") === conclusion.markdown, "report differs from conclusion");
  }
  return manifest;
}
function addFile(directory, manifest, name, bytes, role) {
  const existing = manifest.files.find(f => f.path === name);
  if (existing) { requireValue(existing.sha256 === sha(bytes), `conflicting replay: ${name}`); return; }
  const destination = path.join(directory, name);
  if (fs.existsSync(destination)) requireValue(sha(fs.readFileSync(within(directory, destination))) === sha(bytes), `uncommitted conflicting file: ${name}`);
  else fs.writeFileSync(destination, bytes, { flag: "wx" });
  manifest.files.push({ path: name, sha256: sha(bytes), role });
}
export function research(command, options) {
  requireValue(Object.hasOwn(commandInputs, command), `unknown research command: ${command}`);
  const input = commandInputs[command];
  const allowed = new Set(["run", "workspace", "run-id", ...(input ? [input] : [])]);
  for (const key of Object.keys(options)) requireValue(allowed.has(key), `unknown option for ${command}: --${key}`);
  if (input) requireValue(text(options[input]), `--${input} required for ${command}`);
  const selected = resolveResearchRun(options);
  if (command !== "init") requireValue(fs.existsSync(selected) && fs.statSync(selected).isDirectory(), "research run does not exist; init an explicit run first");
  const directory = externalDirectory(selected);
  return { ...executeResearch(command, options, directory), run_directory: directory };
}
function executeResearch(command, options, directory) {
  const release = lock(directory);
  try {
    const manifestPath = path.join(directory, "manifest.json");
    if (command === "init") {
      const request = readJson(options.request); validateRequest(request);
      if (fs.existsSync(manifestPath)) { const prior = manifestAt(directory); requireValue(prior.request_sha256 === identity(request), "same run with different request; use a new run for refresh"); return { ...prior, replayed: true }; }
      requireValue(fs.readdirSync(directory).every(x => x === ".operation.lock"), "new run directory is not empty; reconcile interrupted init first");
      fs.writeFileSync(path.join(directory, "request.json"), jsonBytes(canonical(request)), { flag: "wx" });
      const manifest = { schema_version: schema, request_sha256: identity(request), created_at: new Date().toISOString(), state: "working", claims_status: "not-checked", events: [], files: [{ path: "request.json", sha256: sha(fs.readFileSync(path.join(directory, "request.json"))), role: "request" }] };
      atomicJson(manifestPath, manifest); return manifest;
    }
    const manifest = manifestAt(directory);
    if (command === "verify" || command === "status") return { ...manifest, integrity: "verified", semantic_review: "host-owned; integrity is not factual certification" };
    if (command === "record") {
      const event = readJson(options.event); validateEvent(event);
      const name = `event-${event.id}.json`; const existing = manifest.files.find(f => f.path === name);
      requireValue(manifest.state === "working" || existing, "run finalized; use a new run for new observations");
      addFile(directory, manifest, name, jsonBytes(canonical(event)), "observation");
      if (!manifest.events.includes(event.id)) manifest.events.push(event.id);
      atomicJson(manifestPath, manifest); return { state: manifest.state, event: event.id, replayed: !!existing };
    }
    if (command === "finish") {
      const conclusion = readJson(options.conclusion);
      const request = readJson(path.join(directory, "request.json"));
      validateConclusion(conclusion, manifest.events.map(id => readJson(within(directory, path.join(directory, `event-${id}.json`)))), request);
      for (const [name, bytes] of [["conclusion.json", jsonBytes(canonical(conclusion))], ["report.md", Buffer.from(conclusion.markdown)]]) {
        if (fs.existsSync(path.join(directory, name))) requireValue(sha(fs.readFileSync(within(directory, path.join(directory, name)))) === sha(bytes), `conflicting replay: ${name}`);
      }
      addFile(directory, manifest, "conclusion.json", jsonBytes(canonical(conclusion)), "conclusion");
      addFile(directory, manifest, "report.md", Buffer.from(conclusion.markdown), "draft");
      manifest.state = "finalized"; manifest.claims_status = "host-reviewed-machine-conclusions";
      atomicJson(manifestPath, manifest); return manifestAt(directory);
    }
    throw new Error(`unknown research command: ${command}`);
  } finally { release(); }
}
if (isMain(import.meta)) {
  try {
    const argv = process.argv.slice(2);
    if (argv.length === 1 && ["help", "--help"].includes(argv[0])) process.stdout.write(usage);
    else { const { command, options } = argsOf(argv); process.stdout.write(JSON.stringify(research(command, options)) + "\n"); }
  }
  catch (error) { emitError(error); }
}
