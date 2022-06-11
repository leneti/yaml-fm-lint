#! /usr/bin/env node
const fs = require("fs");
const fm = require("front-matter");
const yamlLint = require("yaml-lint");
const config = require('config');

const filePath = process.argv[2];

/**
 * Checks if the front matter exists.
 * @param {string} fm - front matter string
 */
function checkFrontMatterExists(fm) {
  if (!fm) {
    console.log(`YAMLException: Front matter not found in ${__dirname}\\${filePath}. Make sure front matter is at the beginning of the file.`);
    process.exit(1)
  }
}

/**
 * Checks if there are any quotes in the front matter and warns against using them.
 * @param {string} str - string to check
 */
function checkQuotes(str) {
  const lines = str.split("\n");
  const redundantQuotes = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const quoteRegexp =
      /(^[\"\'])|([\"\']\s*\r)|([\"\']\s*:)|(:\s+[\"\'])|([\"\']$)/g;
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
        }\n${"----^".padStart(col+3, "-")}\n${i + 1 < lines.length ? `${i + 3} | ${lines[i + 1]}\n` : ""}`,
      });
    }
  }

  if (redundantQuotes.length > 0) {
    const quotes = redundantQuotes.reduce((acc, curr) => {
      return `${acc}\n  at ${__dirname}\\${filePath}:${curr.row}:${curr.col}.\n\n${curr.snippet}\n`;
    }, "");
    console.log(`YAMLException: Redundant quotes found in ${filePath}:\n${quotes}`);
    process.exitCode = 1;
  }
}

/**
 * Checks if all required attributes are present in the front matter.
 * @param {Object} attributes front matter attributes
 */
function checkAttributes(attributes) {
  const requiredAttributes = config.get("requiredAttributes");
  const missingAttributes = requiredAttributes.filter(
    (attribute) => !attributes.hasOwnProperty(attribute)
  );

  if (missingAttributes.length > 0) {
    console.log(
      `YAMLException: Missing attributes in ${__dirname}\\${filePath}: ${missingAttributes.join(", ")}\n`
    );
    process.exitCode = 1;
  }
}

fs.readFile(filePath, "utf8", function (err, data) {
  if (err) throw err;

  const content = fm(data);

  checkFrontMatterExists(content.frontmatter)

  yamlLint
    .lint(content.frontmatter)
    .then(() => {
      checkAttributes(content.attributes);
      checkQuotes(content.frontmatter)
    })
    .catch(console.error);
});
