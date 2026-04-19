import { Diagnostic, DiagnosticSeverity, Range } from "vscode-languageserver/node";
import { TextDocument } from "vscode-languageserver-textdocument";

type StackEntry = {
  readonly expected: string;
  readonly actual: string;
  readonly offset: number;
};

const BRACKET_PAIRS: Record<string, string> = {
  "(": ")",
  "[": "]",
  "{": "}",
};

export function collectDiagnostics(document: TextDocument): Diagnostic[] {
  const text = document.getText();
  const diagnostics: Diagnostic[] = [];
  const stack: StackEntry[] = [];
  let inString = false;
  let inLineComment = false;
  let inBlockComment = false;

  for (let index = 0; index < text.length; index += 1) {
    const current = text[index];
    const next = text[index + 1];
    if (!current) {
      continue;
    }

    if (inLineComment) {
      if (current === "\n") {
        inLineComment = false;
      }
      continue;
    }

    if (inBlockComment) {
      if (current === "*" && next === "/") {
        inBlockComment = false;
        index += 1;
      }
      continue;
    }

    if (inString) {
      if (current === "\\") {
        index += 1;
        continue;
      }
      if (current === '"') {
        inString = false;
      }
      continue;
    }

    if (current === "/" && next === "/") {
      inLineComment = true;
      index += 1;
      continue;
    }

    if (current === "/" && next === "*") {
      inBlockComment = true;
      index += 1;
      continue;
    }

    if (current === '"') {
      inString = true;
      continue;
    }

    if (current in BRACKET_PAIRS) {
      const expected = BRACKET_PAIRS[current];
      if (expected) {
        stack.push({ expected, actual: current, offset: index });
      }
      continue;
    }

    if (current === ")" || current === "]" || current === "}") {
      const last = stack.pop();
      if (!last || last.expected !== current) {
        diagnostics.push(createDiagnostic(document, index, `Unexpected closing bracket '${current}'.`));
      }
    }
  }

  if (inString) {
    diagnostics.push(createDiagnostic(document, safeOffset(text.length), "Unterminated string literal."));
  }

  if (inBlockComment) {
    diagnostics.push(createDiagnostic(document, safeOffset(text.length), "Unterminated block comment."));
  }

  diagnostics.push(
    ...stack.map((entry) => createDiagnostic(document, entry.offset, `Missing closing bracket '${entry.expected}'.`)),
  );

  return diagnostics;
}

function createDiagnostic(document: TextDocument, offset: number, message: string): Diagnostic {
  const start = document.positionAt(offset);
  const end = document.positionAt(offset + 1);
  return {
    severity: DiagnosticSeverity.Error,
    range: Range.create(start, end),
    message,
    source: "zoxi-lsp",
  };
}

function safeOffset(length: number): number {
  return Math.max(0, length - 1);
}
