const mockArgs = {
  path: "examples",
  fix: false,
  config: undefined,
  recursive: false,
  mandatory: true,
  quiet: false,
  oneline: false,
  colored: false,
};

const mockConfig = {
  excludeDirs: ["__mocks__", "__tests__", ".git", "coverage", "node_modules"],
  extraExcludeDirs: [],
  extensions: [".md"],
  includeDirs: [],
  mandatory: true,
  requiredAttributes: ["test"],
};

describe("yaml-fm-lint", () => {
  describe("example file tests: ", () => {
    beforeEach(() => {
      jest.resetModules();
      console.log = jest.fn();
    });

    afterEach(jest.clearAllMocks);

    it("testPassing.md should return no errors/warnings", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testPassing.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(errorNumber).toBe(0);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testMissingAttributes.md should return 'missing attributes' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testMissingAttributes.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/missing attributes.*test/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testBlankLines.md should return 'no empty lines' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testBlankLines.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no empty lines/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testTrailingSpaces.md should return 'no trailing spaces' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testTrailingSpaces.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no trailing spaces/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testSpaceBeforeColon.md should return 'no whitespace before colons' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testSpaceBeforeColon.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no whitespace before colons/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testQuotes.md should return 'no quotes' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testQuotes.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no quotes/)
          );
          expect(errorNumber).toBe(2);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testBrackets.md should return 'no brackets' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testBrackets.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no brackets/)
          );
          expect(errorNumber).toBe(2);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testCurlyBraces.md should return 'no curly braces' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testCurlyBraces.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no curly braces/)
          );
          expect(errorNumber).toBe(2);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testIndentation.md should return 'cannot be indented more than 2 spaces' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testIndentation.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/cannot be indented more than 2 spaces/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testTrailingCommas.md should return 'no trailing commas' error", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testTrailingCommas.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no trailing commas/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testCommas.md should return 'unintended commas' warning", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testCommas.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/unintended commas/)
          );
          expect(errorNumber).toBe(0);
          expect(warningNumber).toBe(1);
        })
        .catch(console.log)
        .finally(done);
    });

    it("testWhitespace.md should return 'unintended whitespace' warning", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/testWhitespace.md" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/unintended whitespace/)
          );
          expect(errorNumber).toBe(0);
          expect(warningNumber).toBe(2);
        })
        .catch(console.log)
        .finally(done);
    });
  });

  describe("args tests: ", () => {
    beforeEach(() => {
      jest.resetModules();
      console.log = jest.fn();
    });

    afterEach(jest.clearAllMocks);

    it("should lint deeply nested files with recursive flag", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, recursive: true, path: "examples/deeply" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/there should be no empty lines/)
          );
          expect(errorNumber).toBe(1);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    });

    it("should not find or lint any nested files without the recursive flag", (done) => {
      const { main } = require("../index");
      const args = { ...mockArgs, path: "examples/deeply" };
      main(args, mockConfig)
        .then(({ errorNumber, warningNumber }) => {
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/No markdown files found/)
          );
          expect(errorNumber).toBe(0);
          expect(warningNumber).toBe(0);
        })
        .catch(console.log)
        .finally(done);
    })

    it("should lint files with custom config", (done) => {
      const argv = process.argv;
      console.time = jest.fn();
      console.timeEnd = jest.fn();
      process.argv = [
        "node",
        "index.js",
        "examples/testPassing.md",
        "--config=examples/customConfig.json",
      ];
      const { run } = require("../index");
      run()
        .then(() => {
          expect(console.time).toHaveBeenCalled();
          expect(console.log).toHaveBeenCalledWith(
            expect.stringMatching(/valid front matter/)
          );
          expect(console.timeEnd).toHaveBeenCalled();
          process.argv = argv;
        })
        .finally(done);
    });
  });
});
