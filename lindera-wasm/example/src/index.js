import __wbg_init, { TokenizerBuilder, loadDictionaryFromBytes, getVersion } from '../../pkg/lindera_wasm.js';
import { downloadDictionary, loadDictionaryFiles, hasDictionary, listDictionaries, removeDictionary } from '../../pkg/opfs.js';

const DEFAULT_DICT_NAME = 'ipadic';

/**
 * Builds the default dictionary URL using the current package version.
 */
function defaultDictUrl(version) {
    return `https://github.com/lindera/lindera/releases/download/v${version}/lindera-ipadic-${version}.zip`;
}

/**
 * Rewrites a GitHub URL to use the dev server proxy to avoid CORS issues.
 */
function proxyUrl(url) {
    const GITHUB_PREFIX = 'https://github.com/';
    if (url.startsWith(GITHUB_PREFIX)) {
        return '/github-releases/' + url.slice(GITHUB_PREFIX.length);
    }
    return url;
}

let tokenizer = null;

// UI elements
const titleEl = document.getElementById('title');
const inputTextEl = document.getElementById('inputText');
const runButtonEl = document.getElementById('runButton');
const resultListEl = document.getElementById('resultList');
const dictStatusEl = document.getElementById('dictStatus');
const dictUrlEl = document.getElementById('dictUrl');
const dictNameEl = document.getElementById('dictName');
const downloadButtonEl = document.getElementById('downloadButton');
const deleteButtonEl = document.getElementById('deleteButton');
const dictListEl = document.getElementById('dictList');
const progressEl = document.getElementById('progress');

/**
 * Updates the dictionary list display.
 */
async function refreshDictList() {
    const names = await listDictionaries();
    if (names.length === 0) {
        dictListEl.textContent = 'No dictionaries stored';
    } else {
        dictListEl.textContent = names.join(', ');
    }
}

/**
 * Sets the status message.
 */
function setStatus(message, isError = false) {
    dictStatusEl.textContent = message;
    dictStatusEl.style.color = isError ? '#e53e3e' : '#38a169';
}

/**
 * Loads a dictionary from OPFS and initializes the tokenizer.
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
        console.log(`Tokenizer initialized with "${dictName}" dictionary from OPFS.`);
    } catch (e) {
        setStatus(`Failed to load dictionary: ${e.message}`, true);
        console.error("Failed to load tokenizer:", e);
    }
}

// Initialize WASM module
__wbg_init().then(async () => {
    // Show version and set default dictionary URL
    try {
        const version = getVersion();
        document.title = `Lindera WASM v${version} (OPFS)`;
        titleEl.textContent = `Lindera WASM v${version}`;
        dictUrlEl.value = defaultDictUrl(version);
    } catch (e) {
        console.error("Failed to get version:", e);
    }

    await refreshDictList();

    // Check if default dictionary is already in OPFS
    const exists = await hasDictionary(DEFAULT_DICT_NAME);
    if (exists) {
        await loadTokenizer(DEFAULT_DICT_NAME);
    } else {
        setStatus('No dictionary loaded. Download one to get started.');
    }
}).catch(e => {
    console.error("Failed to initialize WASM module:", e);
    setStatus(`WASM initialization failed: ${e.message}`, true);
});

// Download button handler
downloadButtonEl.addEventListener('click', async () => {
    const url = dictUrlEl.value.trim();
    const name = dictNameEl.value.trim();

    if (!url || !name) {
        setStatus('Please enter both URL and dictionary name.', true);
        return;
    }

    downloadButtonEl.disabled = true;
    runButtonEl.disabled = true;
    progressEl.style.display = 'block';

    try {
        await downloadDictionary(proxyUrl(url), name, {
            onProgress: ({ phase, loaded, total }) => {
                switch (phase) {
                    case 'downloading':
                        if (total) {
                            const pct = ((loaded / total) * 100).toFixed(1);
                            progressEl.textContent = `Downloading... ${pct}% (${(loaded / 1024 / 1024).toFixed(1)} MB / ${(total / 1024 / 1024).toFixed(1)} MB)`;
                        } else if (loaded) {
                            progressEl.textContent = `Downloading... ${(loaded / 1024 / 1024).toFixed(1)} MB`;
                        } else {
                            progressEl.textContent = 'Downloading...';
                        }
                        break;
                    case 'extracting':
                        progressEl.textContent = 'Extracting zip archive...';
                        break;
                    case 'storing':
                        progressEl.textContent = 'Storing to OPFS...';
                        break;
                    case 'complete':
                        progressEl.textContent = 'Done!';
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
        setTimeout(() => { progressEl.style.display = 'none'; }, 2000);
    }
});

// Delete button handler
deleteButtonEl.addEventListener('click', async () => {
    const name = dictNameEl.value.trim();
    if (!name) {
        setStatus('Please enter a dictionary name to delete.', true);
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
runButtonEl.addEventListener('click', () => {
    if (!tokenizer) {
        setStatus('Tokenizer is not initialized. Download a dictionary first.', true);
        return;
    }

    const inputText = inputTextEl.value;
    const tokens = tokenizer.tokenize(inputText);

    resultListEl.innerHTML = '';

    const table = document.createElement('table');
    table.className = 'token-table';
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
    const tbody = table.querySelector('tbody');

    tokens.forEach((token) => {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><strong>${token.surface}</strong></td>
            <td>${token.position}</td>
            <td><small>${token.details.join(', ')}</small></td>
        `;
        tbody.appendChild(tr);
    });

    resultListEl.appendChild(table);
});
