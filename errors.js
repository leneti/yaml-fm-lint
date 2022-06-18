const chalk = require("chalk");
const cwd = process.cwd().replace(/\\/g, "/");

function getSnippets(arr, filePath) {
  return arr.reduce(
    (acc, curr) =>
      `${acc}\n  at ${cwd}/${filePath}:${curr.row}:${curr.col}.\n\n${curr.snippet}\n`,
    ""
  );
}

function showError(message, filePath, errors) {
  const snippets = getSnippets(errors, filePath)
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} ${message}.\n${snippets}`
  );
}

function checkAttributes(attributes, requiredAttributes, filePath) {
  const missingAttributes = requiredAttributes.filter(
    (attr) => !attributes.includes(attr)
  );

  if (missingAttributes.length > 0) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} missing attributes in ${cwd}/${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );

    return 1;
  }
  return 0;
}

function indentationError(indentation, filePath) {
  const message = "lines cannot be indented more than 2 spaces from the previous line";
  showError(message, filePath, indentation);
}

function spaceBeforeColonError(spacesBeforeColon, filePath) {
  const message = "there should be no whitespace before colons";
  showError(message, filePath, spacesBeforeColon);
}

function blankLinesError(blankLines, filePath) {
  const blankLinesStr = blankLines.reduce(
    (acc, curr) => `${acc}\n  at ${cwd}/${filePath}:${curr}.\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no empty lines.\n${blankLinesStr}`
  );
}

function quotesError(quotes, filePath) {
  const message = "there should be no quotes in the front matter";
  showError(message, filePath, quotes);
}

function trailingSpacesError(trailingSpaces, filePath) {
  const message = "there should be no trailing spaces";
  showError(message, filePath, trailingSpaces);
}

function bracketsError(brackets, filePath) {
  const message = "there should be no brackets. Please use hyphen "-" symbols followed by a space to list items on separate lines";
  showError(message, filePath, brackets);
}

function curlyBracesError(curlyBraces, filePath) {
  const message = "there should be no curly braces. Please list key-value pairs on separate lines indented with 2 spaces";
  showError(message, filePath, curlyBraces);
}

function repeatingSpacesWarning(repeatingSpaces, filePath) {
  const repeatingSpacesStr = getSnippets(repeatingSpaces, filePath);
  console.log(
    `${chalk.yellow(
      "YAMLException:"
    )} found possibly unintended whitespace.\n${repeatingSpacesStr}`
  );
}

function warnCommasWarning(warnCommas, filePath) {
  const warnCommasStr = getSnippets(warnCommas, filePath);
  console.log(
    `${chalk.yellow(
      "YAMLException:"
    )} found possibly unintended commas.\n${warnCommasStr}`
  );
}

function trailingCommasError(trailingCommas, filePath) {
  const message = "there should be no trailing commas";
  showError(message, filePath, trailingCommas);
}

module.exports = {
  checkAttributes,
  indentationError,
  spaceBeforeColonError,
  blankLinesError,
  quotesError,
  trailingSpacesError,
  bracketsError,
  curlyBracesError,
  repeatingSpacesWarning,
  warnCommasWarning,
  trailingCommasError,
};
