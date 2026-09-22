# flint

A command-line linter for spreadsheet formulas. It reads a plain text
export of formulas (one per line) and reports problems by line number,
the way a code linter would.

## Why

Spreadsheets don't get code review. A formula with an extra `(`, an
`=` left empty by a bad copy-paste, or a stray `INDIRECT()` that makes
the whole sheet recalculate on every keystroke will sit there silently
until someone notices the sheet is slow or the numbers are wrong.
Most of that is mechanically checkable before it ships.

`flint` doesn't open `.xlsx` files directly. It expects formulas
already extracted to text, one per line, which is what you get from
exporting a sheet or dumping a column with a script. That keeps the
tool small and lets it work on formulas pulled from anywhere: Excel,
Google Sheets, LibreOffice, or a database column that happens to hold
formula strings.

## Usage

```
$ cat formulas.txt
=SUM(A1:A10
=IF(B2>0, "ok", "bad")
=
=NOW()+1
=VLOOKUP(A1, Sheet2!A:B, 2, FALSE)

$ flint formulas.txt
1: [unbalanced-parens] missing 1 closing ')'
3: [empty-formula] formula has no expression after '='
4: [volatile-function] uses NOW( which recalculates on every sheet edit
5: [cross-sheet-reference] hardcoded reference to sheet 'Sheet2'; renaming or reordering that sheet will silently break this formula
```

It also reads from stdin, so it fits in a pipeline:

```
$ ./export_formulas.sh mysheet.xlsx | flint -
```

Exit code is `0` if nothing was flagged, `1` if there were findings,
and `2` on a usage or I/O error.

## How it handles large input

Formula exports from a big workbook can be hundreds of thousands of
lines. `flint` reads the input line by line with a buffered reader and
checks each line as it arrives, so memory use stays flat whether the
file is 10 lines or 10 million — it never reads the whole thing into
memory first.

## Current checks

- `unbalanced-parens` — a formula whose parentheses don't close
  (ignores parentheses inside quoted string literals)
- `empty-formula` — a line that is just `=` with nothing after it
- `volatile-function` — use of `NOW`, `TODAY`, `RAND`, `RANDBETWEEN`,
  `OFFSET`, or `INDIRECT`, all of which force recalculation on every
  edit to the sheet, not just when their inputs change
- `cross-sheet-reference` — a hardcoded reference to another sheet,
  either bare (`Sheet2!A1`) or quoted (`'Q3 Actuals'!B2`); renaming
  or reordering the referenced sheet breaks these silently, so
  they're worth a second look during a sheet reorganization

Lines starting with `#` are treated as comments and skipped.

## Building

Standard library only, no dependencies:

```
cargo build --release
```

## License

MIT, see [LICENSE](LICENSE).
