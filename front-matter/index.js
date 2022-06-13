/**
# The MIT License (MIT)
Copyright (c) Jason Campbell ("Author")
Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
import { CORE_SCHEMA, load } from "js-yaml";
const optionalByteOrderMark = "\\ufeff?";
const platform = typeof process !== "undefined" ? process.platform : "";
const pattern =
  "^(" +
  optionalByteOrderMark +
  "(= yaml =|---)" +
  "$([\\s\\S]*?)" +
  "^(?:\\2|\\.\\.\\.)\\s*" +
  "$" +
  (platform === "win32" ? "\\r?" : "") +
  "(?:\\n)?)";
// NOTE: If this pattern uses the 'g' flag the `regex` variable definition will
// need to be moved down into the functions that use it.
const regex = new RegExp(pattern, "m");

export default extractor;
const _test = test;
export { _test as test };

function extractor(str) {
  str = str || "";
  const lines = str.split(/(\r?\n)/);
  if (lines[0] && /= yaml =|---/.test(lines[0])) {
    return parse(str);
  } else {
    return {
      attributes: {},
      body: str,
      bodyBegin: 1,
    };
  }
}

function computeLocation(match, body) {
  let line = 1;
  let pos = body.indexOf("\n");
  const offset = match.index + match[0].length;

  while (pos !== -1) {
    if (pos >= offset) {
      return line;
    }
    line++;
    pos = body.indexOf("\n", pos + 1);
  }

  return line;
}

function parse(str, opts) {
  const match = regex.exec(str);
  if (!match) {
    return {
      attributes: {},
      body: str,
      bodyBegin: 1,
    };
  }

  var defaultOpts = { schema: CORE_SCHEMA };

  const yaml = match[match.length - 1].replace(/^\s+|\s+$/g, "");
  const attributes = load(yaml, opts || defaultOpts) || {};
  const body = str.replace(match[0], "");
  const bodyBegin = computeLocation(match, str);

  return {
    attributes,
    body,
    bodyBegin,
    frontmatter: yaml,
  };
}

function test(str) {
  str = str || "";

  return regex.test(str);
}
