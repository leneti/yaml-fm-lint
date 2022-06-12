#! /usr/bin/env node
const fs = require("fs");
const fm = require("front-matter");
const yamlLint = require("yaml-lint");

const args = getArguments();
const defaultConfig = JSON.parse(
  fs.readFileSync(`${__dirname}\\config\\default.json`)
);
let config = args.config
  ? { ...defaultConfig, ...JSON.parse(fs.readFileSync(args.config)) }
  : { ...defaultConfig };
try {
  const mConfig = JSON.parse(
    fs.readFileSync(`${process.cwd()}\\.yaml-fm-lint.json`)
  );
  config = { ...config, ...mConfig };
} catch (_) {}
const allExcludedDirs = [...config.excludeDirs, ...config.extraExcludeDirs];

/**
 * Retrieves arguments from the command line
 * @returns {Object} - arguments object
 */
function getArguments() {
  const args = process.argv.slice(2);
  let pathRead = false;
  const argv = args.reduce((acc, curr) => {
    let [key, value] = curr.split("=");
    if (key.startsWith("-")) {
      key = key.replace("-", "");
    } else if (!key.startsWith("--")) {
      if (!pathRead) {
        value = key;
        key = "path";
        pathRead = true;
      } else {
        console.error(
          `Invalid argument: \"${curr}\". Only one path argument is allowed.`
        );
        process.exit(1);
      }
    } else {
      key = key.replace(/^--/, "");
    }
    acc[key] = value ?? true;
    return acc;
  }, {});

  if (!pathRead) {
    console.error(
      `Invalid arguments: No path argument found. Please specify a path.`
    );
    process.exit(1);
  }

  return argv;
}

/**
 * Checks if the front matter exists.
 * @param {string} fm - front matter string
 */
function checkFrontMatterExists(fm, filePath) {
  if (!fm) {
    console.log(
      `YAMLException: front matter not found in ${process.cwd()}\\${filePath}. Make sure front matter is at the beginning of the file.`
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
      `YAMLException: missing attributes in ${process.cwd()}\\${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
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
    let match;
    while ((match = quoteRegexp.exec(lines[i])) !== null) {
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
      return `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${
        curr.col
      }.\n\n${curr.snippet}\n`;
    }, "");
    console.log(
      `YAMLException: there should be no redundant quotes.\n${quotes}`
    );
    process.exitCode = 1;
  }
}

/**
 * Checks if there are any incorrectly spaced colons in the front matter and shows a warning.
 * @param {string} fm - front matter string
 * @param {string} filePath - path to file
 */
function checkNoSpacesBeforeColon(fm, filePath) {
  const lines = fm.split("\n");
  const spacesBeforeColon = [];
  const spaceBeforeColonRegexp = /\s+:/g;

  for (let i = 0; i < lines.length; i++) {
    let match;
    while ((match = spaceBeforeColonRegexp.exec(lines[i])) !== null) {
      const row = i + 2;
      const col = match.index + match[0].search(/\s+/) + 2;
      spacesBeforeColon.push({
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

  if (spacesBeforeColon.length > 0) {
    const spaces = spacesBeforeColon.reduce((acc, curr) => {
      return `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${
        curr.col
      }.\n\n${curr.snippet}\n`;
    }, "");
    console.log(
      `YAMLException: there should be no spaces before colons.\n${spaces}`
    );
    process.exitCode = 1;
  }
}

function lintNonRecursively(path) {
  if (fs.lstatSync(path).isDirectory()) {
    fs.readdir(path, "utf8", function (err, files) {
      if (err) throw err;
      files.forEach((file) => {
        if (file.endsWith(".md"))
          lintFile(`${path === "." ? "" : `${path}\\`}${file}`);
      });
    });
  } else {
    if (path.endsWith(".md")) lintFile(path);
  }
}

/**
 * Lints all files in the given directory
 * @param {string} path - path to file or directory
 */
function lintRecursively(path) {
  if (fs.lstatSync(path).isDirectory()) {
    if (
      allExcludedDirs.some((ignoredDirectory) =>
        path.includes(ignoredDirectory)
      ) &&
      !config.includeDirs.some((includedDirectory) =>
        path.includes(includedDirectory)
      )
    )
      return;

    fs.readdir(path, "utf8", function (err, files) {
      if (err) throw err;
      files.forEach((file) => {
        lintRecursively(`${path === "." ? "" : `${path}\\`}${file}`);
      });
    });
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
        checkNoSpacesBeforeColon(content.frontmatter, filePath);
      })
      .catch(console.error);
  });
}

exports.lintFM = () => {
  console.time("Linting took");
  if (!args.recursive && !args.r) {
    lintNonRecursively(args.path);
  } else {
    lintRecursively(args.path);
  }
  console.timeEnd("Linting took");
}
