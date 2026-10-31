const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { describe, it } = require("node:test");

const lindera = require("../index.js");

// The source of a dictionary with a 4x4 connection matrix, whose
// metadata.json enables connection_id_mapping and whose one entry has the
// context IDs 2, with a histogram that counts 1, 3 and 2 for IDs 1, 2 and 3.
const FIXTURE = path.join(
  __dirname,
  "..",
  "..",
  "resources",
  "connection_id_mapping",
);

// Builds the fixture into a new temporary directory with the extra
// arguments `rest`, and returns the `context_id_map` of the built
// metadata.json; the directory is removed afterwards.
function builtContextIdMap(...rest) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "lindera-build-"));
  try {
    const metadata = lindera.Metadata.fromJsonFile(
      path.join(FIXTURE, "metadata.json"),
    );
    lindera.buildDictionary(
      path.join(FIXTURE, "source"),
      dir,
      metadata,
      ...rest,
    );
    const built = JSON.parse(
      fs.readFileSync(path.join(dir, "metadata.json"), "utf8"),
    );
    return built.context_id_map;
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

describe("buildDictionary", () => {
  it("keeps connection_id_mapping of the metadata", () => {
    // Without a histogram, the IDs other than 0 are ranked by the count of
    // dictionary entries, ties by ID: 2 (the entry's), then 1 and 3.
    assert.deepStrictEqual(builtContextIdMap(), {
      left: [0, 2, 1, 3],
      right: [0, 2, 1, 3],
    });
  });

  it("ranks context IDs by the histogram", () => {
    const histogram = path.join(FIXTURE, "context_id_freq.txt");
    assert.deepStrictEqual(builtContextIdMap(histogram), {
      left: [0, 3, 1, 2],
      right: [0, 3, 1, 2],
    });
  });

  it("rejects a histogram that is not a string", () => {
    assert.throws(() => builtContextIdMap(0), { code: "StringExpected" });
  });
});
