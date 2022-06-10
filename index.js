#! /usr/bin/env node
const fs = require("fs");
const fm = require("front-matter");
const { FrontMatterNotFoundError, RedundandQuoteError } = require("./errors");

const filePath = process.argv[2];

/**
 *
 * @param {string} str - string to check
 * @returns {[{row: number, col: number}]} - array of objects with exact positions of redundant quotes
 */
function checkQuotes(str) {
  const lines = str.split("\n");
  const redundantQuotes = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const quoteRegexp = /(^[\"\'])|([\"\']\s*\r)|([\"\']\s*:)|(:\s+[\"\'])|([\"\']$)/g;
    let match;
    while ((match = quoteRegexp.exec(line)) !== null) {
      quoteRegexp.lastIndex = match.index + 1;
      redundantQuotes.push({
        row: i + 2,
        col: match.index + match[0].search(/[\"\']/) + 2,
      });
    }
  }
  return redundantQuotes;
}

fs.readFile(filePath, "utf8", function (err, data) {
  if (err) throw err;

  const content = fm(data);

  if (!content.frontmatter) {
    throw new FrontMatterNotFoundError(filePath);
  }

  const mQuotePos = checkQuotes(content.frontmatter);
  if (mQuotePos.length > 0) {
    throw new RedundandQuoteError(filePath, content.frontmatter, mQuotePos);
  }

  console.log(content);
  console.log(content.frontmatter);
});