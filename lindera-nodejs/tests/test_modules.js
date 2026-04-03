const assert = require("assert");
const { describe, it } = require("node:test");

const lindera = require("../index.js");

describe("Module structure", () => {
  it("should export core classes", () => {
    assert.strictEqual(typeof lindera.TokenizerBuilder, "function");
    assert.strictEqual(typeof lindera.Tokenizer, "function");
    assert.strictEqual(typeof lindera.Schema, "function");
    assert.strictEqual(typeof lindera.Metadata, "function");
  });

  it("should export core functions", () => {
    assert.strictEqual(typeof lindera.loadDictionary, "function");
    assert.strictEqual(typeof lindera.loadUserDictionary, "function");
    assert.strictEqual(typeof lindera.buildDictionary, "function");
    assert.strictEqual(typeof lindera.buildUserDictionary, "function");
    assert.strictEqual(typeof lindera.version, "function");
  });

  it("should return a version string", () => {
    const ver = lindera.version();
    assert.strictEqual(typeof ver, "string");
    assert.ok(ver.length > 0);
    // Version should match semver pattern
    assert.match(ver, /^\d+\.\d+\.\d+/);
  });
});

describe("Schema", () => {
  it("should create a default schema with 13 fields", () => {
    const schema = lindera.Schema.createDefault();
    assert.strictEqual(schema.fieldCount(), 13);
    assert.strictEqual(schema.getFieldName(0), "surface");
    assert.strictEqual(schema.getFieldName(3), "cost");
  });

  it("should create a custom schema", () => {
    const schema = new lindera.Schema(["surface", "pos", "reading"]);
    assert.strictEqual(schema.fieldCount(), 3);
    assert.deepStrictEqual(schema.fields, ["surface", "pos", "reading"]);
  });

  it("should look up field index", () => {
    const schema = lindera.Schema.createDefault();
    assert.strictEqual(schema.getFieldIndex("surface"), 0);
    assert.strictEqual(schema.getFieldIndex("cost"), 3);
    assert.strictEqual(schema.getFieldIndex("nonexistent"), null);
  });

  it("should get field by name", () => {
    const schema = lindera.Schema.createDefault();
    const field = schema.getFieldByName("surface");
    assert.ok(field);
    assert.strictEqual(field.index, 0);
    assert.strictEqual(field.name, "surface");
  });

  it("should validate records", () => {
    const schema = new lindera.Schema(["a", "b", "c"]);
    // Valid record
    assert.doesNotThrow(() => schema.validateRecord(["1", "2", "3"]));
    // Too few fields
    assert.throws(() => schema.validateRecord(["1", "2"]));
  });
});

describe("Metadata", () => {
  it("should create default metadata", () => {
    const metadata = new lindera.Metadata();
    assert.strictEqual(metadata.name, "default");
    assert.strictEqual(metadata.encoding, "UTF-8");
    assert.strictEqual(metadata.defaultWordCost, -10000);
  });

  it("should create metadata with options", () => {
    const metadata = new lindera.Metadata({
      name: "custom",
      encoding: "EUC-JP",
    });
    assert.strictEqual(metadata.name, "custom");
    assert.strictEqual(metadata.encoding, "EUC-JP");
  });

  it("should support property setters", () => {
    const metadata = new lindera.Metadata();
    metadata.name = "updated";
    assert.strictEqual(metadata.name, "updated");
  });

  it("should convert to object", () => {
    const metadata = new lindera.Metadata({ name: "test" });
    const obj = metadata.toObject();
    assert.strictEqual(obj.name, "test");
    assert.strictEqual(typeof obj.encoding, "string");
  });

  it("should create default via factory", () => {
    const metadata = lindera.Metadata.createDefault();
    assert.strictEqual(metadata.name, "default");
  });
});
