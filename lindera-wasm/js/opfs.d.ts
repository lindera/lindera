/**
 * OPFS (Origin Private File System) helper utilities for Lindera WASM.
 *
 * @module opfs
 */

/** Dictionary binary file data loaded from OPFS. */
export interface DictionaryFiles {
  /** Contents of metadata.json */
  metadata: Uint8Array;
  /** Contents of dict.da (Double-Array Trie) */
  dictDa: Uint8Array;
  /** Contents of dict.vals (word value data) */
  dictVals: Uint8Array;
  /** Contents of dict.wordsidx (word details index) */
  dictWordsIdx: Uint8Array;
  /** Contents of dict.words (word details) */
  dictWords: Uint8Array;
  /** Contents of matrix.mtx (connection cost matrix) */
  matrixMtx: Uint8Array;
  /** Contents of char_def.bin (character definitions) */
  charDef: Uint8Array;
  /** Contents of unk.bin (unknown word dictionary) */
  unk: Uint8Array;
}

/** Progress information passed to the onProgress callback. */
export interface DownloadProgress {
  /** Current phase of the operation. */
  phase: "downloading" | "extracting" | "storing" | "complete";
  /** Bytes downloaded so far (only during "downloading" phase). */
  loaded?: number;
  /** Total bytes to download, if known (only during "downloading" phase). */
  total?: number;
}

/** Options for downloadDictionary(). */
export interface DownloadDictionaryOptions {
  /** Progress callback. */
  onProgress?: (progress: DownloadProgress) => void;
}

/**
 * Downloads a dictionary archive, extracts it, and stores the files in OPFS.
 *
 * @param url - URL of the dictionary zip archive.
 * @param name - Name to store the dictionary under (e.g., "ipadic").
 * @param options - Optional settings.
 */
export function downloadDictionary(
  url: string,
  name: string,
  options?: DownloadDictionaryOptions,
): Promise<void>;

/**
 * Loads dictionary files from OPFS as an object of Uint8Arrays.
 *
 * @param name - The dictionary name (e.g., "ipadic").
 * @returns Object containing the dictionary file data.
 */
export function loadDictionaryFiles(name: string): Promise<DictionaryFiles>;

/**
 * Removes a dictionary from OPFS.
 *
 * @param name - The dictionary name to remove.
 */
export function removeDictionary(name: string): Promise<void>;

/**
 * Lists all dictionaries stored in OPFS.
 *
 * @returns Array of dictionary names.
 */
export function listDictionaries(): Promise<string[]>;

/**
 * Checks if a dictionary exists in OPFS.
 *
 * @param name - The dictionary name to check.
 * @returns True if the dictionary exists.
 */
export function hasDictionary(name: string): Promise<boolean>;
