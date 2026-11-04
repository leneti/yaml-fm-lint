// Records what yaml-fm-lint 1.7.1 reports for one fixture.
//
// Usage: node record-1x.cjs <pkgDir> <fixtureAbsPath> <fixtureRel>
//
// Prints exactly one JSON document to stdout:
//   {"fixture", "version", "errors", "warnings", "crashed"}
// The absolute file path is left out on purpose, so the output does not
// depend on the machine it was recorded on.

const fs = require("fs");
const path = require("path");

const [pkgDir, fixtureAbs, fixtureRel] = process.argv.slice(2);

// 1.7.1 prints to stdout on YAML errors, so keep our own writer and silence the rest.
const write = process.stdout.write.bind(process.stdout);
console.log = () => {};
console.error = () => {};
console.warn = () => {};
console.time = () => {};
console.timeEnd = () => {};

function output(errors, warnings, crashed) {
  const result = { fixture: fixtureRel, version: "1.7.1", errors, warnings, crashed };
  write(JSON.stringify(result, null, 2) + "\n");
}

async function main() {
  const pkgRoot = path.join(pkgDir, "node_modules", "yaml-fm-lint");
  const text = fs.readFileSync(fixtureAbs, "utf8");
  if (text === "") {
    // 1.7.1 ignores the passed config for empty text, so it cannot be recorded.
    output({}, {}, "empty file");
    return;
  }
  try {
    const { lintFile } = require(path.join(pkgRoot, "index.js"));
    const defaultConfig = JSON.parse(
      fs.readFileSync(path.join(pkgRoot, "config", "default.json"), "utf8"),
    );
    const result = await lintFile(fixtureAbs, text, { quiet: true }, defaultConfig);
    output(result.errors || {}, result.warnings || {}, null);
  } catch (err) {
    output({}, {}, String(err));
  }
}

main().then(
  () => process.exit(0),
  (err) => {
    output({}, {}, String(err));
    process.exit(0);
  },
);
