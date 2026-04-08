import __wbg_init, {
  TokenizerBuilder,
  loadDictionaryFromBytes,
  getVersion,
} from "../../pkg/lindera_wasm.js";
import {
  downloadDictionary,
  loadDictionaryFiles,
  hasDictionary,
  listDictionaries,
  removeDictionary,
} from "../../pkg/opfs.js";

/** Default dictionary to load on first visit. */
const DEFAULT_DICT_NAME = "ipadic";

/**
 * Catalog of hosted dictionaries.
 * Keys match the `<select>` option values and OPFS storage names.
 */
const DICTIONARIES = {
  ipadic: { prefix: "lindera-ipadic" },
  unidic: { prefix: "lindera-unidic" },
  "ko-dic": { prefix: "lindera-ko-dic" },
  "cc-cedict": { prefix: "lindera-cc-cedict" },
  jieba: { prefix: "lindera-jieba" },
};

/** Cached manifest promise (loaded once). */
let _manifestPromise = null;

/**
 * Fetches the dictionary manifest from the same origin.
 *
 * The manifest maps dictionary names to their zip filenames and is
 * generated during the GitHub Pages deployment. Returns `null` if the
 * manifest is unavailable (e.g. on localhost).
 *
 * @returns {Promise<Object|null>} Parsed manifest or null.
 */
function fetchManifest() {
  if (!_manifestPromise) {
    const url = new URL("dict/manifest.json", window.location.href).href;
    _manifestPromise = fetch(url)
      .then((r) => (r.ok ? r.json() : null))
      .catch(() => null);
  }
  return _manifestPromise;
}

/**
 * Returns whether the app is running on localhost (webpack dev server).
 *
 * @returns {boolean} True on localhost or 127.0.0.1.
 */
function isLocalhost() {
  return (
    window.location.hostname === "localhost" ||
    window.location.hostname === "127.0.0.1"
  );
}

/**
 * Resolves the download URL for a named dictionary from the catalog.
 *
 * On GitHub Pages the dictionary zips are hosted alongside the demo under
 * `./dict/`. On localhost the webpack dev server proxy is used to fetch
 * from GitHub Releases.
 *
 * @param {string} name - Dictionary key from DICTIONARIES.
 * @param {string} version - The lindera-wasm version (fallback for filename).
 * @returns {Promise<{url: string, fetchInit?: RequestInit}>}
 */
async function getDictUrl(name, version) {
  const dict = DICTIONARIES[name];
  if (!dict) {
    throw new Error(`Unknown dictionary: ${name}`);
  }

  if (isLocalhost()) {
    const filename = `${dict.prefix}-${version}.zip`;
    return {
      url: `/github-releases/lindera/lindera/releases/download/v${version}/${filename}`,
    };
  }

  // On GitHub Pages: look up filename from manifest
  const manifest = await fetchManifest();
  if (manifest && manifest[name]) {
    return { url: new URL(`dict/${manifest[name]}`, window.location.href).href };
  }

  // Fallback: guess filename using WASM version
  const filename = `${dict.prefix}-${version}.zip`;
  return { url: new URL(`dict/${filename}`, window.location.href).href };
}

/**
 * Resolves a custom (user-entered) URL for downloading.
 *
 * On localhost, GitHub URLs are rewritten to use the webpack dev server
 * proxy. On other hosts the URL is returned as-is.
 *
 * @param {string} url - The user-entered URL.
 * @returns {{url: string}} Resolved URL.
 */
function resolveCustomUrl(url) {
  if (isLocalhost()) {
    const GITHUB_PREFIX = "https://github.com/";
    if (url.startsWith(GITHUB_PREFIX)) {
      return { url: "/github-releases/" + url.slice(GITHUB_PREFIX.length) };
    }
  }
  return { url };
}

let tokenizer = null;

// UI elements
const titleEl = document.getElementById("title");
const inputTextEl = document.getElementById("inputText");
const runButtonEl = document.getElementById("runButton");
const resultListEl = document.getElementById("resultList");
const dictStatusEl = document.getElementById("dictStatus");
const dictSelectEl = document.getElementById("dictSelect");
const dictUrlEl = document.getElementById("dictUrl");
const dictNameEl = document.getElementById("dictName");
const downloadButtonEl = document.getElementById("downloadButton");
const deleteButtonEl = document.getElementById("deleteButton");
const dictListEl = document.getElementById("dictList");
const progressEl = document.getElementById("progress");

/**
 * Updates the dictionary list display.
 */
async function refreshDictList() {
  const names = await listDictionaries();
  if (names.length === 0) {
    dictListEl.textContent = "No dictionaries stored";
  } else {
    dictListEl.textContent = names.join(", ");
  }
}

/**
 * Sets the status message.
 *
 * @param {string} message - Status text.
 * @param {boolean} [isError] - When true the message is shown in red.
 */
function setStatus(message, isError = false) {
  dictStatusEl.textContent = message;
  dictStatusEl.style.color = isError ? "#e53e3e" : "#38a169";
}

/**
 * Loads a dictionary from OPFS and initializes the tokenizer.
 *
 * @param {string} dictName - OPFS dictionary name.
 */
async function loadTokenizer(dictName) {
  try {
    setStatus(`Loading "${dictName}" from OPFS...`);

    const files = await loadDictionaryFiles(dictName);
    const dict = loadDictionaryFromBytes(
      files.metadata,
      files.dictDa,
      files.dictVals,
      files.dictWordsIdx,
      files.dictWords,
      files.matrixMtx,
      files.charDef,
      files.unk,
    );

    const builder = new TokenizerBuilder();
    builder.setDictionaryInstance(dict);
    builder.setMode("normal");
    tokenizer = builder.build();

    setStatus(`Tokenizer ready (dictionary: ${dictName})`);
    runButtonEl.disabled = false;
    console.log(
      `Tokenizer initialized with "${dictName}" dictionary from OPFS.`,
    );
  } catch (e) {
    setStatus(`Failed to load dictionary: ${e.message}`, true);
    console.error("Failed to load tokenizer:", e);
  }
}

// Show/hide custom URL fields based on selector
dictSelectEl.addEventListener("change", () => {
  const isCustom = dictSelectEl.value === "custom";
  dictUrlEl.classList.toggle("visible", isCustom);
  dictNameEl.classList.toggle("visible", isCustom);
});

// Initialize WASM module
__wbg_init()
  .then(async () => {
    try {
      const version = getVersion();
      document.title = `Lindera WASM v${version}`;
      titleEl.textContent = `Lindera WASM v${version}`;
    } catch (e) {
      console.error("Failed to get version:", e);
    }

    await refreshDictList();

    // Check if default dictionary is already in OPFS
    const exists = await hasDictionary(DEFAULT_DICT_NAME);
    if (exists) {
      await loadTokenizer(DEFAULT_DICT_NAME);
    } else {
      setStatus("No dictionary loaded. Select one and click Download.");
    }
  })
  .catch((e) => {
    console.error("Failed to initialize WASM module:", e);
    setStatus(`WASM initialization failed: ${e.message}`, true);
  });

// Download button handler
downloadButtonEl.addEventListener("click", async () => {
  const isCustom = dictSelectEl.value === "custom";
  let name;
  let resolved;

  if (isCustom) {
    const url = dictUrlEl.value.trim();
    name = dictNameEl.value.trim();
    if (!url || !name) {
      setStatus("Please enter both URL and dictionary name.", true);
      return;
    }
    resolved = resolveCustomUrl(url);
  } else {
    name = dictSelectEl.value;
    try {
      const version = getVersion();
      resolved = await getDictUrl(name, version);
    } catch (e) {
      setStatus(`Failed to resolve dictionary URL: ${e.message}`, true);
      return;
    }
  }

  downloadButtonEl.disabled = true;
  runButtonEl.disabled = true;
  progressEl.style.display = "block";

  try {
    await downloadDictionary(resolved.url, name, {
      fetchInit: resolved.fetchInit,
      onProgress: ({ phase, loaded, total }) => {
        switch (phase) {
          case "downloading":
            if (total) {
              const pct = ((loaded / total) * 100).toFixed(1);
              progressEl.textContent = `Downloading... ${pct}% (${(loaded / 1024 / 1024).toFixed(1)} MB / ${(total / 1024 / 1024).toFixed(1)} MB)`;
            } else if (loaded) {
              progressEl.textContent = `Downloading... ${(loaded / 1024 / 1024).toFixed(1)} MB`;
            } else {
              progressEl.textContent = "Downloading...";
            }
            break;
          case "extracting":
            progressEl.textContent = "Extracting zip archive...";
            break;
          case "storing":
            progressEl.textContent = "Storing to OPFS...";
            break;
          case "complete":
            progressEl.textContent = "Done!";
            break;
        }
      },
    });

    await refreshDictList();
    await loadTokenizer(name);
  } catch (e) {
    setStatus(`Download failed: ${e.message}`, true);
    console.error("Download failed:", e);
  } finally {
    downloadButtonEl.disabled = false;
    setTimeout(() => {
      progressEl.style.display = "none";
    }, 2000);
  }
});

// Delete button handler
deleteButtonEl.addEventListener("click", async () => {
  const isCustom = dictSelectEl.value === "custom";
  const name = isCustom ? dictNameEl.value.trim() : dictSelectEl.value;
  if (!name) {
    setStatus("Please enter a dictionary name to delete.", true);
    return;
  }

  try {
    await removeDictionary(name);
    setStatus(`Dictionary "${name}" removed.`);
    tokenizer = null;
    runButtonEl.disabled = true;
    await refreshDictList();
  } catch (e) {
    setStatus(`Failed to remove: ${e.message}`, true);
  }
});

// Tokenize button handler
runButtonEl.addEventListener("click", () => {
  if (!tokenizer) {
    setStatus(
      "Tokenizer is not initialized. Download a dictionary first.",
      true,
    );
    return;
  }

  const inputText = inputTextEl.value;
  const tokens = tokenizer.tokenize(inputText);

  resultListEl.innerHTML = "";

  const table = document.createElement("table");
  table.className = "token-table";
  table.innerHTML = `
        <thead>
            <tr>
                <th>Surface</th>
                <th>Position</th>
                <th>Details</th>
            </tr>
        </thead>
        <tbody></tbody>
    `;
  const tbody = table.querySelector("tbody");

  tokens.forEach((token) => {
    const tr = document.createElement("tr");
    tr.innerHTML = `
            <td><strong>${token.surface}</strong></td>
            <td>${token.position}</td>
            <td><small>${token.details.join(", ")}</small></td>
        `;
    tbody.appendChild(tr);
  });

  resultListEl.appendChild(table);
});
