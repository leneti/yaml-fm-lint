class DomainError extends Error {
  /**
   *
   * @param {string} message - error message
   */
  constructor(message) {
    super(message);
    // Ensure the name of this error is the same as the class name
    this.name = this.constructor.name;
    // This clips the constructor invocation from the stack trace.
    // It's not absolutely essential, but it does make the stack trace a little nicer.
    Error.captureStackTrace(this, this.constructor);
  }
}

class FrontMatterNotFound extends DomainError {
  /**
   *
   * @param {string} filePath - file path
   */
  constructor(filePath) {
    super(
      `Front matter not found in ${filePath}.\nMake sure front matter is at the beginning of the file.`
    );
    this.filePath = filePath;
  }
}

class RedundandQuotes extends DomainError {
  /**
   *
   * @param {string} filePath - file path
   * @param {string} fm - front matter
   * @param {[{row: number, col: number}]} quotePos - array of redundant quotes
   */
  constructor(filePath, fm, quotePos) {
    const redundantQuotes = quotePos.reduce((acc, curr) => {
      return `${acc}\n  at ${__dirname}\\${filePath}:${curr.row}:${curr.col}.`;
    }, "");

    super(redundantQuotes);
    this.parsedString = fm;
  }
}

module.exports = {
  FrontMatterNotFoundError: FrontMatterNotFound,
  RedundandQuoteError: RedundandQuotes,
};
