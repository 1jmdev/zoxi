import {
  DocumentSymbol,
  Location,
  Position,
  Range,
  SymbolKind,
} from "vscode-languageserver/node";
import { TextDocument } from "vscode-languageserver-textdocument";

const FUNCTION_PATTERN = /(^|\n)\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g;
const WORD_PATTERN = /[A-Za-z_][A-Za-z0-9_]*/;

export type FunctionSymbol = {
  readonly name: string;
  readonly range: Range;
  readonly selectionRange: Range;
  readonly location: Location;
};

export function collectFunctionSymbols(document: TextDocument): FunctionSymbol[] {
  const text = document.getText();
  const symbols: FunctionSymbol[] = [];

  for (const match of text.matchAll(FUNCTION_PATTERN)) {
    const fullMatch = match[0];
    const name = match[2];
    if (!name) {
      continue;
    }

    const prefixLength = fullMatch.length - name.length - 1;
    const nameOffset = (match.index ?? 0) + prefixLength;
    const selectionRange = rangeForWord(document, nameOffset, name.length);
    symbols.push({
      name,
      range: selectionRange,
      selectionRange,
      location: Location.create(document.uri, selectionRange),
    });
  }

  return symbols;
}

export function toDocumentSymbols(symbols: FunctionSymbol[]): DocumentSymbol[] {
  return symbols.map((symbol) =>
    DocumentSymbol.create(symbol.name, "function", SymbolKind.Function, symbol.range, symbol.selectionRange),
  );
}

export function findFunctionDefinition(name: string, documents: readonly TextDocument[]): Location | null {
  for (const document of documents) {
    const match = collectFunctionSymbols(document).find((symbol) => symbol.name === name);
    if (match) {
      return match.location;
    }
  }

  return null;
}

export function wordAtPosition(document: TextDocument, position: Position): string | null {
  const text = document.getText();
  const offset = document.offsetAt(position);
  const start = findWordBoundary(text, offset, -1);
  const end = findWordBoundary(text, offset, 1);
  if (start === end) {
    return null;
  }

  const candidate = text.slice(start, end);
  return WORD_PATTERN.test(candidate) ? candidate : null;
}

function rangeForWord(document: TextDocument, offset: number, length: number): Range {
  return Range.create(document.positionAt(offset), document.positionAt(offset + length));
}

function findWordBoundary(text: string, offset: number, step: -1 | 1): number {
  let cursor = offset;
  if (step < 0) {
    cursor -= 1;
  }

  while (cursor >= 0 && cursor < text.length && isWordCharacter(text[cursor])) {
    cursor += step;
  }

  return step < 0 ? cursor + 1 : cursor;
}

function isWordCharacter(character: string | undefined): boolean {
  return character !== undefined && /[A-Za-z0-9_]/.test(character);
}
