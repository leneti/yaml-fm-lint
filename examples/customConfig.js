/**
 * @param {{frontMatter: Object, showOneline: (type: "Error" | "Warning", message: string, affected: string | {row: number, col: number, snippet?: string}[]) => void, rawFm: string}} param0
 * @returns {{errors: number, warnings: number}}
 */
function lowercaseTags({ frontMatter, showOneline }) {
  const tags = frontMatter.tags;
  let errors = 0;

  tags.forEach((tag) => {
    if (tag.toLowerCase() !== tag) {
      showOneline("Error", "tags must be lowercase", tag);
      errors++;
    }
  });

  return { errors, warnings: 0 };
}

module.exports = {
  extraLintFns: [lowercaseTags],
  requiredAttributes: ["tags"],
};
