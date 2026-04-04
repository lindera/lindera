/**
 * OPFS (Origin Private File System) helper utilities for Lindera WASM.
 *
 * Provides functions to download, store, load, and manage Lindera dictionaries
 * using the browser's Origin Private File System for persistent caching.
 *
 * @module opfs
 */

/** Dictionary file names that make up a built Lindera dictionary. */
const DICTIONARY_FILES = [
  "metadata.json",
  "dict.da",
  "dict.vals",
  "dict.wordsidx",
  "dict.words",
  "matrix.mtx",
  "char_def.bin",
  "unk.bin",
];

/** Base directory path within OPFS for storing dictionaries. */
const OPFS_BASE_PATH = ["lindera", "dictionaries"];

/**
 * Gets or creates a nested directory handle within OPFS.
 *
 * @param {string[]} pathSegments - Array of directory names forming the path.
 * @returns {Promise<FileSystemDirectoryHandle>} The directory handle.
 */
async function getDirectoryHandle(pathSegments) {
  let dir = await navigator.storage.getDirectory();
  for (const segment of pathSegments) {
    dir = await dir.getDirectoryHandle(segment, { create: true });
  }
  return dir;
}

/**
 * Gets the OPFS directory handle for a specific dictionary.
 *
 * @param {string} name - The dictionary name (e.g., "ipadic").
 * @returns {Promise<FileSystemDirectoryHandle>} The dictionary directory handle.
 */
async function getDictionaryDir(name) {
  return getDirectoryHandle([...OPFS_BASE_PATH, name]);
}

/**
 * Extracts entries from a zip archive using DecompressionStream.
 *
 * This implementation parses the zip central directory and decompresses
 * entries using the Web Streams API (DecompressionStream), avoiding
 * external library dependencies.
 *
 * @param {ArrayBuffer} zipBuffer - The zip file contents.
 * @returns {Promise<Map<string, Uint8Array>>} Map of filename to file contents.
 */
async function extractZip(zipBuffer) {
  const view = new DataView(zipBuffer);
  const bytes = new Uint8Array(zipBuffer);
  const entries = new Map();

  // Find End of Central Directory record (search from end)
  let eocdOffset = -1;
  for (let i = bytes.length - 22; i >= 0; i--) {
    if (view.getUint32(i, true) === 0x06054b50) {
      eocdOffset = i;
      break;
    }
  }
  if (eocdOffset === -1) {
    throw new Error("Invalid zip file: End of Central Directory not found");
  }

  const cdOffset = view.getUint32(eocdOffset + 16, true);
  const cdEntries = view.getUint16(eocdOffset + 10, true);

  // Parse Central Directory entries
  let offset = cdOffset;
  for (let i = 0; i < cdEntries; i++) {
    if (view.getUint32(offset, true) !== 0x02014b50) {
      throw new Error(
        "Invalid zip file: bad Central Directory entry signature",
      );
    }

    const compressionMethod = view.getUint16(offset + 10, true);
    const compressedSize = view.getUint32(offset + 20, true);
    const uncompressedSize = view.getUint32(offset + 24, true);
    const fileNameLength = view.getUint16(offset + 28, true);
    const extraFieldLength = view.getUint16(offset + 30, true);
    const commentLength = view.getUint16(offset + 32, true);
    const localHeaderOffset = view.getUint32(offset + 42, true);

    const fileName = new TextDecoder().decode(
      bytes.subarray(offset + 46, offset + 46 + fileNameLength),
    );

    // Skip directories
    if (!fileName.endsWith("/")) {
      // Read from local file header to get actual data offset
      const localFileNameLength = view.getUint16(localHeaderOffset + 26, true);
      const localExtraLength = view.getUint16(localHeaderOffset + 28, true);
      const dataOffset =
        localHeaderOffset + 30 + localFileNameLength + localExtraLength;

      const compressedData = bytes.subarray(
        dataOffset,
        dataOffset + compressedSize,
      );

      let fileData;
      if (compressionMethod === 0) {
        // Stored (no compression)
        fileData = compressedData;
      } else if (compressionMethod === 8) {
        // Deflate - use DecompressionStream
        const ds = new DecompressionStream("deflate-raw");
        const writer = ds.writable.getWriter();
        const reader = ds.readable.getReader();

        // Write compressed data and close
        writer.write(compressedData).then(() => writer.close());

        // Read all decompressed chunks
        const chunks = [];
        let totalLength = 0;
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          chunks.push(value);
          totalLength += value.length;
        }

        // Concatenate chunks
        fileData = new Uint8Array(totalLength);
        let pos = 0;
        for (const chunk of chunks) {
          fileData.set(chunk, pos);
          pos += chunk.length;
        }

        // Verify decompressed size
        if (fileData.length !== uncompressedSize) {
          throw new Error(
            `Size mismatch for ${fileName}: expected ${uncompressedSize}, got ${fileData.length}`,
          );
        }
      } else {
        throw new Error(
          `Unsupported compression method ${compressionMethod} for ${fileName}`,
        );
      }

      entries.set(fileName, fileData);
    }

    // Move to next Central Directory entry
    offset += 46 + fileNameLength + extraFieldLength + commentLength;
  }

  return entries;
}

/**
 * Downloads a dictionary archive, extracts it, and stores the files in OPFS.
 *
 * The archive should be a zip file containing the 8 dictionary files
 * (metadata.json, dict.da, dict.vals, dict.wordsidx, dict.words,
 * matrix.mtx, char_def.bin, unk.bin), optionally nested in a subdirectory.
 *
 * @param {string} url - URL of the dictionary zip archive.
 * @param {string} name - Name to store the dictionary under (e.g., "ipadic").
 * @param {object} [options] - Optional settings.
 * @param {function} [options.onProgress] - Progress callback receiving
 *   `{ phase: string, loaded?: number, total?: number }`.
 * @returns {Promise<void>}
 * @throws {Error} If download fails, archive is invalid, or required files are missing.
 */
export async function downloadDictionary(url, name, options = {}) {
  const { onProgress } = options;

  // Download
  if (onProgress) onProgress({ phase: "downloading" });
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to download dictionary: HTTP ${response.status}`);
  }

  const contentLength = response.headers.get("content-length");
  const total = contentLength ? parseInt(contentLength, 10) : undefined;

  // Read response body with progress tracking
  let zipBuffer;
  if (onProgress && response.body) {
    const reader = response.body.getReader();
    const chunks = [];
    let loaded = 0;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      loaded += value.length;
      onProgress({ phase: "downloading", loaded, total });
    }

    zipBuffer = new Uint8Array(loaded);
    let pos = 0;
    for (const chunk of chunks) {
      zipBuffer.set(chunk, pos);
      pos += chunk.length;
    }
    zipBuffer = zipBuffer.buffer;
  } else {
    zipBuffer = await response.arrayBuffer();
  }

  // Extract
  if (onProgress) onProgress({ phase: "extracting" });
  const entries = await extractZip(zipBuffer);

  // Find dictionary files (may be nested in a subdirectory)
  const fileMap = new Map();
  for (const [path, data] of entries) {
    const baseName = path.split("/").pop();
    if (DICTIONARY_FILES.includes(baseName)) {
      fileMap.set(baseName, data);
    }
  }

  // Verify all required files are present
  const missing = DICTIONARY_FILES.filter((f) => !fileMap.has(f));
  if (missing.length > 0) {
    throw new Error(
      `Missing dictionary files in archive: ${missing.join(", ")}`,
    );
  }

  // Store in OPFS
  if (onProgress) onProgress({ phase: "storing" });
  const dir = await getDictionaryDir(name);
  for (const [fileName, data] of fileMap) {
    const fileHandle = await dir.getFileHandle(fileName, { create: true });
    const writable = await fileHandle.createWritable();
    await writable.write(data);
    await writable.close();
  }

  if (onProgress) onProgress({ phase: "complete" });
}

/**
 * Loads dictionary files from OPFS as an object of Uint8Arrays.
 *
 * The returned object has properties matching the file names expected
 * by `loadDictionaryFromBytes()`.
 *
 * @param {string} name - The dictionary name (e.g., "ipadic").
 * @returns {Promise<DictionaryFiles>} Object containing the dictionary file data.
 * @throws {Error} If the dictionary is not found in OPFS.
 */
export async function loadDictionaryFiles(name) {
  let dir;
  try {
    const root = await navigator.storage.getDirectory();
    let current = root;
    for (const segment of [...OPFS_BASE_PATH, name]) {
      current = await current.getDirectoryHandle(segment);
    }
    dir = current;
  } catch {
    throw new Error(
      `Dictionary "${name}" not found in OPFS. Call downloadDictionary() first.`,
    );
  }

  /** @param {string} fileName */
  async function readFile(fileName) {
    const fileHandle = await dir.getFileHandle(fileName);
    const file = await fileHandle.getFile();
    const buffer = await file.arrayBuffer();
    return new Uint8Array(buffer);
  }

  return {
    metadata: await readFile("metadata.json"),
    dictDa: await readFile("dict.da"),
    dictVals: await readFile("dict.vals"),
    dictWordsIdx: await readFile("dict.wordsidx"),
    dictWords: await readFile("dict.words"),
    matrixMtx: await readFile("matrix.mtx"),
    charDef: await readFile("char_def.bin"),
    unk: await readFile("unk.bin"),
  };
}

/**
 * Removes a dictionary from OPFS.
 *
 * @param {string} name - The dictionary name to remove.
 * @returns {Promise<void>}
 * @throws {Error} If the dictionary is not found.
 */
export async function removeDictionary(name) {
  const root = await navigator.storage.getDirectory();
  let current = root;
  for (const segment of OPFS_BASE_PATH) {
    current = await current.getDirectoryHandle(segment);
  }
  await current.removeEntry(name, { recursive: true });
}

/**
 * Lists all dictionaries stored in OPFS.
 *
 * @returns {Promise<string[]>} Array of dictionary names.
 */
export async function listDictionaries() {
  try {
    const root = await navigator.storage.getDirectory();
    let current = root;
    for (const segment of OPFS_BASE_PATH) {
      current = await current.getDirectoryHandle(segment);
    }

    const names = [];
    for await (const [name, handle] of current.entries()) {
      if (handle.kind === "directory") {
        names.push(name);
      }
    }
    return names;
  } catch {
    // Base directory doesn't exist yet - no dictionaries stored
    return [];
  }
}

/**
 * Checks if a dictionary exists in OPFS.
 *
 * @param {string} name - The dictionary name to check.
 * @returns {Promise<boolean>} True if the dictionary exists.
 */
export async function hasDictionary(name) {
  try {
    const root = await navigator.storage.getDirectory();
    let current = root;
    for (const segment of [...OPFS_BASE_PATH, name]) {
      current = await current.getDirectoryHandle(segment);
    }
    return true;
  } catch {
    return false;
  }
}
