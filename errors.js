import chalk from "chalk";

export function checkAttributes(attributes, requiredAttributes, filePath) {
  const missingAttributes = requiredAttributes.filter(
    (attr) => !attributes.includes(attr)
  );

  if (missingAttributes.length > 0) {
    console.log(
      `${chalk.red(
        "YAMLException:"
      )} missing attributes in ${process.cwd()}\\${filePath}: ${missingAttributes.join(
        ", "
      )}\n`
    );
    process.exitCode = 1;
    return 1;
  }
  return 0;
}

export function indentationError(indentation, filePath) {
  const indents = indentation.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} lines cannot be indented more than 2 spaces from the previous line.\n${indents}`
  );
  process.exitCode = 1;
}

export function spaceBeforeColonError(spacesBeforeColon, filePath) {
  const spaces = spacesBeforeColon.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no whitespace before colons.\n${spaces}`
  );
  process.exitCode = 1;
}

export function blankLinesError(blankLines, filePath) {
  const blankLinesStr = blankLines.reduce(
    (acc, curr) => `${acc}\n  at ${process.cwd()}\\${filePath}:${curr}.\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no empty lines.\n${blankLinesStr}`
  );
  process.exitCode = 1;
}

export function quotesError(quotes, filePath) {
  const quotesStr = quotes.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no quotes in the front matter.\n${quotesStr}`
  );
  process.exitCode = 1;
}

export function trailingSpacesError(trailingSpaces, filePath) {
  const trailingSpacesStr = trailingSpaces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no trailing spaces.\n${trailingSpacesStr}`
  );
  process.exitCode = 1;
}

export function bracketsError(brackets, filePath) {
  const bracketsStr = brackets.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no brackets. Please use hyphen "-" symbols followed by a space to list items on separate lines.\n${bracketsStr}`
  );
  process.exitCode = 1;
}

export function curlyBracesError(curlyBraces, filePath) {
  const curlyBracesStr = curlyBraces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
        curr.snippet
      }\n`,
    ""
  );
  console.log(
    `${chalk.red(
      "YAMLException:"
    )} there should be no curly braces. Please list key-value pairs on separate lines indented with 2 spaces.\n${curlyBracesStr}`
  );
  process.exitCode = 1;
}

export function repeatingSpacesError(repeatingSpaces, filePath) {
  const repeatingSpacesStr = repeatingSpaces.reduce(
    (acc, curr) =>
      `${acc}\n  at ${process.cwd()}\\${filePath}:${curr.row}:${curr.col}.\n\n${
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
