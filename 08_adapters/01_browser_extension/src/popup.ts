import { toBookmarkCandidate, toPageCandidate } from "./capture.js";
import {
  checkPairing,
  normalizeBaseUrl,
  selectCandidates,
  startSession,
} from "./transport.js";
import type {
  BookmarkCapturePayload,
  CandidateEnvelope,
  CandidateSummary,
  DomCapturePayload,
  LocalApiConfig,
} from "./types.js";

const MAX_PAGE_TEXT = 450_000;
const MAX_BOOKMARKS = 200;

const elements = {
  connectionStatus: byId("connection-status"),
  settings: byId("settings"),
  settingsToggle: button("settings-toggle"),
  apiUrl: input("api-url"),
  apiToken: input("api-token"),
  pair: button("pair"),
  pageMode: button("page-mode"),
  bookmarkMode: button("bookmark-mode"),
  pagePanel: byId("page-panel"),
  bookmarkPanel: byId("bookmark-panel"),
  bookmarkFolder: select("bookmark-folder"),
  preparePage: button("prepare-page"),
  prepareSelection: button("prepare-selection"),
  prepareBookmarks: button("prepare-bookmarks"),
  candidatePanel: byId("candidate-panel"),
  candidateList: byId("candidate-list"),
  candidateCount: byId("candidate-count"),
  collect: button("collect"),
  result: byId("result"),
};

let currentSession: { id: string; scope: string } | undefined;
let currentCandidates: CandidateSummary[] = [];
let folderPaths = new Map<string, string>();

void initialize();

async function initialize(): Promise<void> {
  bindEvents();
  const stored = await chrome.storage.local.get([
    "apiUrl",
    "apiToken",
    "installationId",
  ]);
  elements.apiUrl.value =
    typeof stored.apiUrl === "string"
      ? stored.apiUrl
      : "http://127.0.0.1:43873";
  elements.apiToken.value =
    typeof stored.apiToken === "string" ? stored.apiToken : "";
  if (elements.apiToken.value) {
    await pair(false);
    if (elements.connectionStatus.dataset.state !== "ready") {
      elements.settings.hidden = false;
    }
  } else {
    elements.settings.hidden = false;
  }
}

function bindEvents(): void {
  elements.settingsToggle.addEventListener("click", () => {
    elements.settings.hidden = !elements.settings.hidden;
  });
  elements.pair.addEventListener("click", () => void pair(true));
  elements.pageMode.addEventListener("click", () => switchMode("page"));
  elements.bookmarkMode.addEventListener("click", () => void switchMode("bookmarks"));
  elements.preparePage.addEventListener("click", () => void preparePage("page"));
  elements.prepareSelection.addEventListener("click", () =>
    void preparePage("selection"),
  );
  elements.prepareBookmarks.addEventListener("click", () =>
    void prepareBookmarks(),
  );
  elements.collect.addEventListener("click", () => void collectSelected());
  elements.candidateList.addEventListener("change", updateCollectState);
}

async function pair(persist: boolean): Promise<void> {
  await run(async () => {
    const config = getConfig();
    await checkPairing(config);
    if (persist) {
      await chrome.storage.local.set({
        apiUrl: normalizeBaseUrl(config.baseUrl),
        apiToken: config.token,
      });
    }
    setConnection("Paired", "ready");
    setResult("Pairing verified.", "ready");
    elements.settings.hidden = true;
  });
}

function switchMode(mode: "page" | "bookmarks"): void | Promise<void> {
  const bookmarks = mode === "bookmarks";
  elements.pagePanel.hidden = bookmarks;
  elements.bookmarkPanel.hidden = !bookmarks;
  elements.pageMode.classList.toggle("active", !bookmarks);
  elements.bookmarkMode.classList.toggle("active", bookmarks);
  elements.pageMode.setAttribute("aria-pressed", String(!bookmarks));
  elements.bookmarkMode.setAttribute("aria-pressed", String(bookmarks));
  resetCandidates();
  if (bookmarks) {
    return loadBookmarkFolders();
  }
}

async function preparePage(kind: "page" | "selection"): Promise<void> {
  await run(async () => {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab.id === undefined || !tab.url || !/^https?:/.test(tab.url)) {
      throw new Error("The active tab is not a collectable web page.");
    }
    const [execution] = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      args: [kind, MAX_PAGE_TEXT],
      func: (captureKind: "page" | "selection", maxLength: number) => {
        const selectedText = globalThis.getSelection()?.toString() ?? "";
        const text =
          captureKind === "selection"
            ? selectedText
            : document.body?.innerText ?? document.title;
        return {
          url: location.href,
          title: document.title || location.href,
          text: text.slice(0, maxLength),
          selectedText: selectedText.slice(0, maxLength),
        } satisfies DomCapturePayload;
      },
    });
    if (!execution?.result) {
      throw new Error("Chrome returned no readable page content.");
    }
    const candidate = await toPageCandidate(execution.result, kind);
    const scope = kind === "selection" ? "current visible selection" : "current visible page";
    await prepareSession("source.browser_pages", scope, [candidate], true);
  });
}

async function loadBookmarkFolders(): Promise<void> {
  await run(async () => {
    const granted = await chrome.permissions.request({ permissions: ["bookmarks"] });
    if (!granted) {
      throw new Error("Bookmarks permission was not granted.");
    }
    const tree = await chrome.bookmarks.getTree();
    folderPaths = new Map();
    const folders: Array<{ id: string; path: string }> = [];
    walkBookmarkFolders(tree, [], folders);
    elements.bookmarkFolder.replaceChildren(
      ...folders.map((folder) => {
        folderPaths.set(folder.id, folder.path);
        const option = document.createElement("option");
        option.value = folder.id;
        option.textContent = folder.path;
        return option;
      }),
    );
  });
}

async function prepareBookmarks(): Promise<void> {
  await run(async () => {
    const folderId = elements.bookmarkFolder.value;
    const folderPath = folderPaths.get(folderId);
    if (!folderId || !folderPath) {
      throw new Error("Select a bookmark folder first.");
    }
    const roots = await chrome.bookmarks.getSubTree(folderId);
    const bookmarks: BookmarkCapturePayload[] = [];
    walkBookmarks(roots[0]?.children ?? [], folderPath.split(" / "), bookmarks);
    if (bookmarks.length === 0) {
      throw new Error("The selected folder contains no bookmarks.");
    }
    if (bookmarks.length > MAX_BOOKMARKS) {
      throw new Error(`The selected folder exceeds the ${MAX_BOOKMARKS}-bookmark limit.`);
    }
    const candidates = await Promise.all(bookmarks.map(toBookmarkCandidate));
    await prepareSession(
      "source.browser_bookmarks",
      `bookmark folder ${folderPath} (${bookmarks.length} links)`,
      candidates,
      false,
    );
  });
}

async function prepareSession(
  routeId: "source.browser_pages" | "source.browser_bookmarks",
  scopeDescription: string,
  candidates: CandidateEnvelope[],
  preselectSingle: boolean,
): Promise<void> {
  const installationId = await getInstallationId();
  const started = await startSession(getConfig(), {
    routeId,
    sourceReference: `submitted:${crypto.randomUUID()}`,
    scopeDescription,
    installationId,
    candidates,
  });
  currentSession = { id: started.session.session_id, scope: scopeDescription };
  currentCandidates = started.candidates;
  renderCandidates(preselectSingle);
  setResult(`${started.candidates.length} candidate${started.candidates.length === 1 ? "" : "s"} ready.`, "ready");
}

function renderCandidates(preselectSingle: boolean): void {
  elements.candidateList.replaceChildren(
    ...currentCandidates.map((candidate) => {
      const label = document.createElement("label");
      label.className = "candidate";
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.value = candidate.candidate_id;
      checkbox.checked = preselectSingle && currentCandidates.length === 1;
      const text = document.createElement("span");
      const title = document.createElement("strong");
      title.textContent = candidate.title || candidate.source_location || "Untitled";
      const location = document.createElement("small");
      location.textContent = candidate.hierarchy.join(" / ");
      text.append(title, location);
      label.append(checkbox, text);
      return label;
    }),
  );
  elements.candidateCount.textContent = String(currentCandidates.length);
  elements.candidatePanel.hidden = false;
  updateCollectState();
}

async function collectSelected(): Promise<void> {
  await run(async () => {
    if (!currentSession) {
      throw new Error("Prepare candidates before collecting.");
    }
    const selected = Array.from(
      elements.candidateList.querySelectorAll<HTMLInputElement>(
        'input[type="checkbox"]:checked',
      ),
      (input) => input.value,
    );
    if (selected.length === 0) {
      throw new Error("Select at least one candidate.");
    }
    const items = await selectCandidates(getConfig(), {
      sessionId: currentSession.id,
      candidateIds: selected,
      scopeDescription: currentSession.scope,
      confirmed: true,
    });
    const saved = items.filter((item) => item.state === "saved").length;
    const failed = items.filter((item) => item.state === "failed").length;
    setResult(`${saved} saved${failed ? `, ${failed} failed` : ""}.`, failed ? "error" : "ready");
    elements.collect.disabled = true;
  });
}

function walkBookmarkFolders(
  nodes: chrome.bookmarks.BookmarkTreeNode[],
  parents: string[],
  output: Array<{ id: string; path: string }>,
): void {
  for (const node of nodes) {
    if (node.url) continue;
    const next = node.title ? [...parents, node.title] : parents;
    if (node.title) output.push({ id: node.id, path: next.join(" / ") });
    if (node.children) walkBookmarkFolders(node.children, next, output);
  }
}

function walkBookmarks(
  nodes: chrome.bookmarks.BookmarkTreeNode[],
  parents: string[],
  output: BookmarkCapturePayload[],
): void {
  for (const node of nodes) {
    if (node.url) {
      output.push({
        id: node.id,
        title: node.title || node.url,
        url: node.url,
        folderPath: parents.join(" / "),
      });
      continue;
    }
    const next = node.title ? [...parents, node.title] : parents;
    if (node.children) walkBookmarks(node.children, next, output);
  }
}

async function getInstallationId(): Promise<string> {
  const stored = await chrome.storage.local.get("installationId");
  if (typeof stored.installationId === "string") return stored.installationId;
  const installationId = crypto.randomUUID();
  await chrome.storage.local.set({ installationId });
  return installationId;
}

function getConfig(): LocalApiConfig {
  return {
    baseUrl: elements.apiUrl.value,
    token: elements.apiToken.value,
  };
}

function resetCandidates(): void {
  currentSession = undefined;
  currentCandidates = [];
  elements.candidatePanel.hidden = true;
  elements.candidateList.replaceChildren();
  elements.collect.disabled = true;
  setResult("");
}

function updateCollectState(): void {
  elements.collect.disabled =
    elements.candidateList.querySelector('input[type="checkbox"]:checked') === null;
}

async function run(action: () => Promise<void>): Promise<void> {
  setResult("Working...");
  try {
    await action();
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unexpected browser error.";
    setResult(message, "error");
    if (message.toLowerCase().includes("pair") || message.toLowerCase().includes("api")) {
      setConnection("Not paired", "error");
    }
  }
}

function setConnection(text: string, state: "idle" | "ready" | "error"): void {
  elements.connectionStatus.textContent = text;
  elements.connectionStatus.dataset.state = state;
}

function setResult(text: string, state: "idle" | "ready" | "error" = "idle"): void {
  elements.result.textContent = text;
  elements.result.dataset.state = state;
}

function byId(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing extension element: ${id}`);
  return element;
}

function button(id: string): HTMLButtonElement {
  return byId(id) as HTMLButtonElement;
}

function input(id: string): HTMLInputElement {
  return byId(id) as HTMLInputElement;
}

function select(id: string): HTMLSelectElement {
  return byId(id) as HTMLSelectElement;
}
