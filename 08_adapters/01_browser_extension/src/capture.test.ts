import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { sha256Hex, toBookmarkCandidate, toPageCandidate } from "./capture.js";
import { normalizeBaseUrl } from "./transport.js";

test("page and selection candidates keep their concrete browser scope", async () => {
  const page = await toPageCandidate(
    {
      url: "https://example.test/page",
      title: "Example",
      text: "Full visible page",
      selectedText: "Chosen text",
    },
    "page",
  );
  const selection = await toPageCandidate(
    {
      url: "https://example.test/page",
      title: "Example",
      text: "Full visible page",
      selectedText: "Chosen text",
    },
    "selection",
  );
  assert.equal(page.payload.text, "Full visible page");
  assert.equal(selection.payload.text, "Chosen text");
  assert.equal(page.payloadSha256, await sha256Hex("Full visible page"));
  assert.equal(selection.payloadSha256, await sha256Hex("Chosen text"));
});

test("bookmark candidates remain locator-only with folder hierarchy", async () => {
  const candidate = await toBookmarkCandidate({
    id: "42",
    title: "Reference",
    url: "https://example.test/reference",
    folderPath: "Bookmarks / Research",
  });
  assert.equal(candidate.routeId, "source.browser_bookmarks");
  assert.equal(candidate.nativeId, "42");
  assert.equal(candidate.metadata.bookmarkFolder, "Bookmarks / Research");
  assert.equal(candidate.metadata.locatorOnly, true);
});

test("local API address is restricted to explicit loopback HTTP", () => {
  assert.equal(
    normalizeBaseUrl("http://127.0.0.1:43873/path"),
    "http://127.0.0.1:43873",
  );
  assert.throws(() => normalizeBaseUrl("https://example.test:43873"));
  assert.throws(() => normalizeBaseUrl("http://localhost:43873"));
  assert.throws(() => normalizeBaseUrl("http://127.0.0.1"));
});

test("manifest keeps page access temporary and bookmarks optional", () => {
  const manifest = JSON.parse(
    readFileSync(new URL("../manifest.json", import.meta.url), "utf8"),
  ) as {
    permissions: string[];
    optional_permissions: string[];
    host_permissions: string[];
  };
  assert.deepEqual(manifest.permissions, ["activeTab", "scripting", "storage"]);
  assert.deepEqual(manifest.optional_permissions, ["bookmarks"]);
  assert.deepEqual(manifest.host_permissions, ["http://127.0.0.1/*"]);
  assert.equal(JSON.stringify(manifest).includes("<all_urls>"), false);
});
