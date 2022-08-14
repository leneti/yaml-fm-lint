/**
 * @param {{attributes: Object, fmLines: string[], lintLog: (type: "Error" | "Warning", message: string, affected: string[] | number[] | { row: number, col: number, colStart?: number, colEnd?: number }[] | undefined) => void}} props
 * @returns {{errors: number, warnings: number}}
 */
 function lowercaseTags({ fmLines, lintLog }) {
  const tagsRegExp = /^tags.*:/g;

  const tagsLineIndex = fmLines.findIndex((line) => tagsRegExp.test(line));
  if (tagsLineIndex < 0) return { errors: 0, warnings: 0 };
  
  const eachTagRegExp = /^(\s*-\s+)(.+)$/;
  const locations = [];
  let errors = 0;

  for (let i = tagsLineIndex + 1; i < fmLines.length; i++) {
    const line = fmLines[i];
    if (!eachTagRegExp.test(line)) break;
    const match = line.match(eachTagRegExp);
    const tag = match[2];
    if (tag.toLowerCase() !== tag) {
      locations.push({
        row: i,
        col: match[1].length + 2,
        colStart: match[1].length,
        colEnd: match[1].length + tag.length,
      })
      errors++;
    }
  }

  lintLog("Error", "tags must be lowercase", locations);

  return { errors, warnings: 0 };
}

module.exports = {
  extraLintFns: [lowercaseTags],
  requiredAttributes: ["tags"],
};
