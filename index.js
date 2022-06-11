#! /usr/bin/env node
const fs = require("fs");
const fm = require("front-matter");
const yamlLint = require("yaml-lint");

const args = getArguments();
const config = JSON.parse(
  fs.readFileSync(args.config ?? "config/default.json")
);
const ignoredDirectories = [
  "node_modules",
  ".git",
  ".idea",
  ".vscode",
  ".expo",
  ".expo-shared",
];

(() => {
  console.time("Linting took")
  lintRecursively(args.path);
  console.timeEnd("Linting took")
})()


// console.log("arguments:", args);

/**
 * Retrieves arguments from the command line
 * @returns {Object} - arguments object
 */
function getArguments() {
  const args = process.argv.slice(2);
  const argv = args.reduce((acc, curr) => {
    let [key, value] = curr.split("=");
    if (!key.startsWith("--")) {
      value = key;
      key = "path";
    } else {
      key = key.replace(/^--/, "");
    }
    acc[key] = value;
    return acc;
  }, {});

  return argv;
}

/**
 * Checks if the front matter exists.
 * @param {string} fm - front matter string
 */
function checkFrontMatterExists(fm, filePath) {
  if (!fm) {
    console.log(
      `YAMLException: Front matter not found in ${__dirname}\\${filePath}. Make sure front matter is at the beginning of the file.`
    );
    process.exitCode = 1;
  }
}

/**
 * Checks if there are any quotes in the front matter and warns against using them.
 * @param {string} fm - (front matter) string to check
 */
function checkQuotes(fm, filePath) {
  const lines = fm.split("\n");
  const redundantQuotes = [];
  const quoteRegexp =
    /(^[\"\'])|([\"\']\s*\r)|([\"\']\s*:)|(:\s+[\"\'])|([\"\']$)/g;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let match;
    while ((match = quoteRegexp.exec(line)) !== null) {
      quoteRegexp.lastIndex = match.index + 1;
      const row = i + 2;
      const col = match.index + match[0].search(/[\"\']/) + 2;
      redundantQuotes.push({
        row,
        col,
        snippet: `${i > 0 ? `${i + 1} | ${lines[i - 1]}\n` : ""}${i + 2} | ${
          lines[i]
        }\n${"----^".padStart(col + 3, "-")}\n${
          i + 1 < lines.length ? `${i + 3} | ${lines[i + 1]}\n` : ""
        }`,
      });
    }
  }

  if (redundantQuotes.length > 0) {
    const quotes = redundantQuotes.reduce((acc, curr) => {
      return `${acc}\n  at ${__dirname}\\${filePath}:${curr.row}:${curr.col}.\n\n${curr.snippet}\n`;
    }, "");
    console.log(
      `YAMLException: Redundant quotes found in ${__dirname}\\${filePath}:\n${quotes}`
    );
    process.exitCode = 1;
  }
}

/**
 * Checks if all required attributes are present in the front matter.
 * @param {Object} attributes front matter attributes
 */
function checkAttributes(attributes, filePath) {
  const missingAttributes =
    config.requiredAttributes?.filter(
      (attribute) => !attributes.hasOwnProperty(attribute)
    ) ?? [];

  if (missingAttributes.length > 0) {
    console.log(
      `YAMLException: Missing attributes in ${__dirname}\\${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );
    process.exitCode = 1;
  }
}

/**
 * Lints all files in the given directory
 * @param {string} path - path to file or directory
 */
 function lintRecursively(path) {
  if (fs.lstatSync(path).isDirectory()) {
    if (
      ignoredDirectories.some((ignoredDirectory) =>
        path.includes(ignoredDirectory)
      )
    )
      return;

    fs.readdir(path, "utf8", function (err, files) {
      if (err) throw err;
      files.forEach((file) => {
        lintRecursively(`${path === "." ? "" : `${path}\\`}${file}`);
      });
    })
  } else {
    if (path.endsWith(".md")) lintFile(path);
  }
}

function lintFile(filePath) {
  fs.readFile(filePath, "utf8", function (err, data) {
    if (err) throw err;

    const content = fm(data);

    checkFrontMatterExists(content.frontmatter, filePath);

    yamlLint
      .lint(content.frontmatter)
      .then(() => {
        checkAttributes(content.attributes, filePath);
        checkQuotes(content.frontmatter, filePath);
      })
      .catch(console.error);
  });
}
