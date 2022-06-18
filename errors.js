const chalk = require("chalk");

function checkAttributes(attributes, requiredAttributes, filePath) {
  const missingAttributes = requiredAttributes.filter(
    (attr) => !attributes.includes(attr)
  );

  if (missingAttributes.length > 0) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} missing attributes in ${process.cwd().replace(/\\/g, "/")}/${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );

    return 1;
  }
  return 0;
}

function indentationError(indentation, filePath) {
  const indents = indentation.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} lines cannot be indented more than 2 spaces from the previous line.\n${indents}`
  );
}

function spaceBeforeColonError(spacesBeforeColon, filePath) {
  const spaces = spacesBeforeColon.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no whitespace before colons.\n${spaces}`
  );
}

function blankLinesError(blankLines, filePath) {
  const blankLinesStr = blankLines.reduce(
    (acc, curr) => `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr}.\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no empty lines.\n${blankLinesStr}`
  );
}

function quotesError(quotes, filePath) {
  const quotesStr = quotes.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no quotes in the front matter.\n${quotesStr}`
  );
}

function trailingSpacesError(trailingSpaces, filePath) {
  const trailingSpacesStr = trailingSpaces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no trailing spaces.\n${trailingSpacesStr}`
  );
}

function bracketsError(brackets, filePath) {
  const bracketsStr = brackets.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no brackets. Please use hyphen "-" symbols followed by a space to list items on separate lines.\n${bracketsStr}`
  );
}

function curlyBracesError(curlyBraces, filePath) {
  const curlyBracesStr = curlyBraces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no curly braces. Please list key-value pairs on separate lines indented with 2 spaces.\n${curlyBracesStr}`
  );
}

function repeatingSpacesWarning(repeatingSpaces, filePath) {
  const repeatingSpacesStr = repeatingSpaces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.yellow(
      "YAMLException:"
    )} found possibly unintended whitespace.\n${repeatingSpacesStr}`
  );
}

function warnCommasWarning(warnCommas, filePath) {
  const warnCommasStr = warnCommas.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.yellow(
      "YAMLException:"
    )} found possibly unintended commas.\n${warnCommasStr}`
  );
}

function trailingCommasError(trailingCommas, filePath) {
  const trailingCommasStr = trailingCommas.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd().replace(/\\/g, "/")}/${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no trailing commas.\n${trailingCommasStr}`
  );
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
}