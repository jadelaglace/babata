import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { argsOf, atomicJson, dependency, emitError, externalDirectory, identity, isMain, lock, readJson, requireValue, sha, verifyFiles, within } from "./common.mjs";

const schema = "babata.png-render/v1";
const origin = "https://babata.invalid";
const profile = "babata-reading/v1";
const pagedCss = 'main{padding:0;max-width:none;background:transparent}.page{padding:48px;width:100%}.page>:first-child{margin-top:0}.page>:last-child{margin-bottom:0}';
const css = '*{box-sizing:border-box}html{background:#eef1f5;color:#192534}body{margin:0;font-family:"Microsoft YaHei","Noto Sans CJK SC",sans-serif;font-size:20px;line-height:1.75}main{margin:0 auto;background:white;padding:48px;max-width:1120px;overflow-wrap:anywhere}h1{font-size:36px;line-height:1.35;border-bottom:3px solid #287b8e;padding-bottom:20px}h2{font-size:27px;color:#165a70;margin-top:32px}h3{font-size:23px}p,ul,ol{margin:16px 0}table{border-collapse:collapse;width:100%;font-size:18px}td,th{border:1px solid #ccd5df;padding:10px;text-align:left}th{background:#e8f0f4}pre{white-space:pre-wrap;background:#f2f5f8;padding:18px}code{font-size:.88em}img{max-width:100%;height:auto}a{color:#165a70}blockquote{border-left:4px solid #287b8e;margin:20px 0;padding:8px 22px;background:#f0f6f7}.page{background:white;break-inside:avoid}.page+.page{margin-top:24px}*{animation:none!important;transition:none!important}';
const mime = { ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".gif": "image/gif", ".webp": "image/webp", ".css": "text/css", ".woff": "font/woff", ".woff2": "font/woff2", ".ttf": "font/ttf", ".otf": "font/otf" };
export function pngDimensions(bytes) {
  requireValue(bytes.length >= 33 && bytes.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10])) && bytes.toString("ascii",12,16) === "IHDR", "renderer did not produce PNG");
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}
export async function probe() {
  const { chromium } = await dependency("playwright"); await dependency("marked");
  requireValue(fs.existsSync(process.env.BABATA_CHROMIUM_EXECUTABLE || chromium.executablePath()), "Chromium unavailable; install Playwright Chromium or configure BABATA_CHROMIUM_EXECUTABLE");
  return { state: "available", renderer: "babata-playwright/v1" };
}
function verify(directory) {
  const manifest = readJson(path.join(directory, "manifest.json"));
  requireValue(manifest.schema_version === schema && manifest.state === "succeeded", "render is not succeeded");
  requireValue(Array.isArray(manifest.files) && Array.isArray(manifest.outputs) && manifest.outputs.length > 0, "invalid render manifest");
  verifyFiles(directory, manifest.files);
  requireValue(new Set(manifest.files.map(f => f.path)).size === manifest.files.length && new Set(manifest.outputs.map(f => f.path)).size === manifest.outputs.length, "duplicate render entries");
  requireValue(identity(manifest.request) === manifest.request_sha256, "render request identity mismatch");
  requireValue(manifest.files.filter(f => f.role === "input").length === 1 && manifest.files.filter(f => f.role === "rendered-html").length === 1 && Array.isArray(manifest.source_assets), "render snapshots incomplete");
  const frozenInput = manifest.files.find(f => f.role === "input");
  requireValue(manifest.files.every(f => ["input", "rendered-html", "asset", "png"].includes(f.role)), "unknown render file role");
  const pngFiles = manifest.files.filter(f => f.role === "png");
  requireValue(pngFiles.length === manifest.outputs.length && manifest.outputs.every(o => pngFiles.some(f => f.path === o.path && f.sha256 === o.sha256)), "render output/file mismatch");
  const assetFiles = manifest.files.filter(f => f.role === "asset");
  requireValue(assetFiles.length === manifest.source_assets.length && manifest.source_assets.every(a => assetFiles.some(f => f.path === a.snapshot && f.sha256 === a.sha256)), "asset snapshot mismatch");
  requireValue(frozenInput && frozenInput.sha256 === manifest.request.input_sha256, "input snapshot does not match request");
  requireValue(manifest.checks?.assets === "pass" && manifest.checks?.structure === "pass" && manifest.checks?.dimensions === "pass", "render QC is incomplete");
  for (const output of manifest.outputs) {
    const bytes = fs.readFileSync(within(directory, path.join(directory, output.path)));
    requireValue(sha(bytes) === output.sha256, "PNG hash mismatch");
    requireValue(JSON.stringify(pngDimensions(bytes)) === JSON.stringify({width: output.width, height: output.height}), "PNG dimensions mismatch");
  }
  return { ...manifest, integrity: "verified" };
}
function validateOptions(options) {
  const mode = options.mode ?? "long"; const width = Number(options.width ?? 1120);
  requireValue(["long", "paged"].includes(mode), "mode must be long or paged");
  requireValue(Number.isInteger(width) && width >= 480 && width <= 2400, "width must be 480..2400 CSS pixels");
  requireValue(options.input && path.isAbsolute(options.input), "absolute input path required");
  const input = fs.realpathSync(options.input); const extension = path.extname(input).toLowerCase();
  requireValue([".md", ".html", ".htm"].includes(extension), "input must be Markdown or HTML");
  const bytes = fs.readFileSync(input);
  requireValue(bytes.length > 0 && bytes.length <= 8 * 1024 * 1024, "input must contain 1 byte..8 MiB");
  return { input, extension, bytes, mode, width };
}
async function renderFresh(directory, spec) {
  const { chromium } = await dependency("playwright");
  let html = spec.bytes.toString("utf8").replace(/^\uFEFF/, "");
  if (spec.extension === ".md") { const { marked } = await dependency("marked"); html = marked.parse(html, { gfm: true }); }
  requireValue(!/<(?:script|iframe|object|embed|base|video|audio|canvas)\b/i.test(html), "unsafe/unsupported HTML element");
  const pageCss = spec.mode === "paged" ? pagedCss : '';
  const document = '<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><style>' + css + pageCss + '</style></head><body><main>' + html + '</main></body></html>';
  const stage = path.join(directory, ".staging-" + crypto.randomUUID()); fs.mkdirSync(stage);
  const assets = new Map(); const failures = [];
  const browser = await chromium.launch({ headless: true, ...(process.env.BABATA_CHROMIUM_EXECUTABLE ? { executablePath: process.env.BABATA_CHROMIUM_EXECUTABLE } : {}) });
  try {
    const context = await browser.newContext({ javaScriptEnabled: false, serviceWorkers: "block", viewport: { width: spec.width, height: 900 }, deviceScaleFactor: 1, reducedMotion: "reduce" });
    const page = await context.newPage(); page.setDefaultTimeout(15000);
    page.on("pageerror", error => failures.push(error.message));
    page.on("console", message => { if (message.type() === "error") failures.push(message.text()); });
    await context.route("**/*", async route => {
      try {
        const url = new URL(route.request().url());
        requireValue(url.origin === origin, "remote resource denied: " + url.origin);
        if (url.pathname === "/__document.html") {
          await route.fulfill({ contentType: "text/html; charset=utf-8", body: document, headers: { "Content-Security-Policy": "default-src 'none'; script-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; base-uri 'none'; frame-src 'none'; connect-src 'none'" } }); return;
        }
        const name = decodeURIComponent(url.pathname).replace(/^\//, "");
        const file = within(path.dirname(spec.input), path.resolve(path.dirname(spec.input), name));
        const contentType = mime[path.extname(file).toLowerCase()]; requireValue(contentType, "unsupported asset format");
        const bytes = fs.readFileSync(file); requireValue(bytes.length <= 24 * 1024 * 1024, "asset exceeds 24 MiB");
        if (contentType === "text/css") requireValue(!/@import|(?:https?:|file:|\/\/)/i.test(bytes.toString("utf8")), "CSS imports/remote references unsupported; freeze assets locally");
        assets.set(file, { bytes, sha256: sha(bytes) });
        await route.fulfill({ contentType, body: bytes });
      } catch (error) { failures.push(error.message); await route.abort(); }
    });
    await page.goto(origin + "/__document.html", { waitUntil: "networkidle", timeout: 20000 });
    await page.evaluate(() => document.fonts.ready);
    if (spec.mode === "long") await page.setViewportSize({ width: spec.width, height: 1 });
    const structure = await page.evaluate(() => {
      const forbidden = [...document.querySelectorAll("script,iframe,object,embed,base,meta[http-equiv],video,audio,canvas")].map(e => e.tagName);
      for (const element of document.querySelectorAll("*")) for (const attr of element.attributes) {
        if (/^on/i.test(attr.name) || /^(?:javascript|file):/i.test(attr.value.trim())) forbidden.push(attr.name);
      }
      const missingImages = [...document.images].filter(e => !e.complete || !e.naturalWidth).map(e => e.getAttribute("src"));
      const failedFonts = [...document.fonts].filter(font => font.status === "error").map(font => font.family);
      const content = [...document.querySelectorAll("html,body,main,main *")].filter(e => !["STYLE", "LINK", "META", "TITLE"].includes(e.tagName));
      const invisible = content.filter(e => { const s=getComputedStyle(e); return (e.textContent.trim() || e.tagName === "IMG") && (s.display === "none" || s.visibility !== "visible" || Number(s.opacity) === 0 || s.contentVisibility === "hidden" || ["fixed", "absolute"].includes(s.position) || s.transform !== "none" || s.clipPath !== "none"); }).map(e => e.tagName);
      const overflow = content.filter(e => { const r=e.getBoundingClientRect(); const s=getComputedStyle(e); return r.left < -1 || r.right > innerWidth + 1 || (s.display !== "inline" && (e.scrollWidth > e.clientWidth + 2 || (e.tagName !== "HTML" && e.scrollHeight > e.clientHeight + 2))); }).map(e => e.tagName);
      const pages = [...document.querySelectorAll("main > .page")];
      const text = document.querySelector("main").innerText.trim();
      return { forbidden, missingImages, failedFonts, invisible, overflow, text, height: Math.ceil(document.documentElement.scrollHeight), pages: pages.length, otherContent: [...document.querySelector("main").childNodes].some(n => n.nodeType === 1 ? !n.classList.contains("page") : n.textContent.trim()) };
    });
    requireValue(!failures.length, "asset/render errors: " + failures.join("; "));
    requireValue(!structure.forbidden.length, "unsafe/unsupported HTML: " + structure.forbidden.join(", "));
    requireValue(!structure.missingImages.length, "missing images: " + structure.missingImages.join(", "));
    requireValue(!structure.failedFonts.length, "failed fonts: " + structure.failedFonts.join(", "));
    requireValue(!structure.invisible.length, "hidden/positioned content unsupported: " + structure.invisible.join(", "));
    requireValue(!structure.overflow.length, "content overflow: " + structure.overflow.join(", "));
    requireValue(structure.text.length > 0 || await page.locator("img").count() > 0, "render contains no readable content");
    const outputs = [];
    if (spec.mode === "long") {
      requireValue(structure.height <= 30000, "content exceeds 30000px; use explicit .page structure in paged mode");
      await page.screenshot({ path: path.join(stage, "output.png"), fullPage: true, animations: "disabled" }); outputs.push("output.png");
    } else {
      requireValue(structure.pages > 0 && !structure.otherContent, "paged mode requires all content in explicit main-level .page elements");
      const pages = page.locator("main > .page");
      for (let i = 0; i < structure.pages; i++) {
        const element = pages.nth(i); const box = await element.boundingBox();
        requireValue(box && box.height >= 1 && box.height <= 30000, "page has invalid height");
        const name = "page-" + String(i + 1).padStart(3, "0") + ".png";
        await element.screenshot({ path: path.join(stage, name), animations: "disabled" }); outputs.push(name);
      }
    }
    requireValue(sha(fs.readFileSync(spec.input)) === sha(spec.bytes), "input changed during render");
    for (const [file, asset] of assets) requireValue(sha(fs.readFileSync(file)) === asset.sha256, "asset changed during render");
    const files = [];
    const save = (name, bytes, role) => { fs.writeFileSync(path.join(stage, name), bytes, { flag: "wx" }); files.push({ path: name, sha256: sha(bytes), role }); };
    save("input" + spec.extension, spec.bytes, "input"); save("document.html", Buffer.from(document), "rendered-html");
    const sourceAssets = []; let index = 0;
    for (const [source, asset] of assets) { const name = "asset-" + (++index) + path.extname(source); save(name, asset.bytes, "asset"); sourceAssets.push({ source, snapshot: name, sha256: asset.sha256 }); }
    const measured = outputs.map(name => { const bytes = fs.readFileSync(path.join(stage,name)); const dimensions = pngDimensions(bytes); requireValue(dimensions.width > 0 && dimensions.height > 0, "empty PNG dimensions"); files.push({path: name, sha256: sha(bytes), role: "png"}); return { path: name, sha256: sha(bytes), ...dimensions }; });
    if (spec.mode === "long") requireValue(measured[0].width === spec.width && measured[0].height === structure.height, "PNG dimensions differ from measured content");
    const request = renderRequest(spec);
    const manifest = { schema_version: schema, state: "succeeded", request, request_sha256: identity(request), source_input: spec.input, source_assets: sourceAssets, created_at: new Date().toISOString(), browser: browser.version(), outputs: measured, files, checks: { assets: "pass", structure: "pass", dimensions: "pass", network: "blocked", scripts: "disabled", visual_review: "not-run", glyphs: "requires-visual-review", blank_bands: "requires-visual-review" } };
    for (const file of files) fs.renameSync(path.join(stage,file.path), path.join(directory,file.path));
    atomicJson(path.join(directory,"manifest.json"), manifest); fs.rmdirSync(stage); return verify(directory);
  } finally { await browser.close(); }
}
function renderRequest(spec) {
  return { input_sha256: sha(spec.bytes), mode: spec.mode, width: spec.width, profile, template_sha256: sha(css + (spec.mode === "paged" ? pagedCss : "")), renderer: "babata-playwright/v1" };
}
export async function render(command, options) {
  const directory = externalDirectory(options.out); const release = lock(directory);
  try {
    if (command === "verify") return verify(directory);
    requireValue(command === "render", "unknown render command: " + command);
    const spec = validateOptions(options);
    if (fs.existsSync(path.join(directory, "manifest.json"))) {
      const previous = verify(directory); const request = renderRequest(spec);
      requireValue(previous.source_input === spec.input, "same render run with different input location; local asset identity may differ; use a new output directory");
      requireValue(previous.request_sha256 === identity(request), "same render run with different inputs/options; use a new output directory");
      for (const asset of previous.source_assets) requireValue(sha(fs.readFileSync(asset.source)) === asset.sha256, "source asset changed; use new run");
      return { ...previous, replayed: true };
    }
    requireValue(fs.readdirSync(directory).every(x => x === ".operation.lock"), "output contains unrelated/interrupted files; reconcile or use a new directory");
    return await renderFresh(directory, spec);
  } finally { release(); }
}
if (isMain(import.meta)) {
  try { const { command, options } = argsOf(process.argv.slice(2)); process.stdout.write(JSON.stringify(command === "probe" ? await probe() : await render(command, options)) + "\n"); }
  catch (error) { emitError(error); }
}
