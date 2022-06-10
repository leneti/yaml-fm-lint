#! /usr/bin/env node
const fs = require("fs");
const fm = require("front-matter");
const yamlLint = require("yaml-lint");
const { FrontMatterNotFoundError, RedundantQuoteError } = require("./errors");

const filePath = process.argv[2];

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
    console.log(`Redundant quotes found in ${filePath}:\n${quotes}`);
    process.exitCode = 1;
  }
}

fs.readFile(filePath, "utf8", function (err, data) {
  if (err) throw err;

  const content = fm(data);

  if (!content.frontmatter) {
    throw new FrontMatterNotFoundError(filePath);
  }

  yamlLint
    .lint(content.frontmatter)
    .then(() => {
      checkQuotes(content.frontmatter)
    })
    .catch(console.error);
});
