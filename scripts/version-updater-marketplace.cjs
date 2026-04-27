// Phase 19 US-1901 — commit-and-tag-version updater for .claude-plugin/marketplace.json.
//
// marketplace.json has nested version: plugins[0].version. Default JSON updater
// only handles root-level "version". This module reads + writes the nested key.
//
// Contract: module.exports = { readVersion(contents), writeVersion(contents, version) }.
// Both receive the file contents as a string and return string.

module.exports = {
  readVersion(contents) {
    const json = JSON.parse(contents);
    if (!json.plugins || !json.plugins[0] || !json.plugins[0].version) {
      throw new Error("marketplace.json missing plugins[0].version");
    }
    return json.plugins[0].version;
  },
  writeVersion(contents, version) {
    const json = JSON.parse(contents);
    if (!json.plugins || !json.plugins[0]) {
      throw new Error("marketplace.json missing plugins[0]");
    }
    json.plugins[0].version = version;
    return JSON.stringify(json, null, 2) + "\n";
  },
};
