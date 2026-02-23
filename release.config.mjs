/**
 * @type {import('semantic-release').GlobalConfig}
 */
export default {
  branches: ["main"],
  plugins: [
    "@semantic-release/commit-analyzer",
    "@semantic-release/release-notes-generator",
    [
      "@semantic-release/github",
      {
        "assets": [
          { "path": "target/release/libhermes.dylib", "label": "MacOS" },
          { "path": "target/release/libhermes.so", "label": "Linux" },
          // TODO: Figure out how
          // { "path": "target/release/hermes.lld", "label": "Windows" },
        ]
      }
    ]
  ]
}
