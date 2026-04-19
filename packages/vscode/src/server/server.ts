import {
  CompletionItem,
  CompletionItemKind,
  Hover,
  InitializeParams,
  InitializeResult,
  InsertTextFormat,
  MarkupKind,
  ProposedFeatures,
  TextDocumentSyncKind,
  createConnection,
} from "vscode-languageserver/node";
import { TextDocuments } from "vscode-languageserver";
import { TextDocument } from "vscode-languageserver-textdocument";

import { collectFunctionSymbols, findFunctionDefinition, toDocumentSymbols, wordAtPosition } from "./analysis";
import { collectDiagnostics } from "./diagnostics";
import { COMPLETION_ITEMS, KEYWORD_DOCS } from "../shared/keywords";

const connection = createConnection(ProposedFeatures.all);
const documents = new TextDocuments(TextDocument);

connection.onInitialize((_params: InitializeParams): InitializeResult => ({
  capabilities: {
    textDocumentSync: TextDocumentSyncKind.Incremental,
    completionProvider: {},
    definitionProvider: true,
    documentSymbolProvider: true,
    hoverProvider: true,
  },
}));

documents.onDidOpen((event) => validateDocument(event.document));
documents.onDidChangeContent((event) => validateDocument(event.document));
documents.onDidClose((event) => connection.sendDiagnostics({ uri: event.document.uri, diagnostics: [] }));

connection.onCompletion((): CompletionItem[] =>
  COMPLETION_ITEMS.map((item) => ({
    label: item.label,
    detail: item.detail,
    kind: item.label.startsWith(".") ? CompletionItemKind.Method : CompletionItemKind.Keyword,
    insertText: item.insertText,
    insertTextFormat: InsertTextFormat.Snippet,
  })),
);

connection.onHover(({ textDocument, position }): Hover | null => {
  const document = documents.get(textDocument.uri);
  if (!document) {
    return null;
  }

  const word = wordAtPosition(document, position);
  if (!word) {
    return null;
  }

  const doc = KEYWORD_DOCS[word];
  if (!doc) {
    return null;
  }

  return {
    contents: {
      kind: MarkupKind.Markdown,
      value: `**${doc.detail}**\n\n${doc.documentation}`,
    },
  };
});

connection.onDefinition(({ textDocument, position }) => {
  const document = documents.get(textDocument.uri);
  if (!document) {
    return null;
  }

  const word = wordAtPosition(document, position);
  if (!word) {
    return null;
  }

  return findFunctionDefinition(word, documents.all());
});

connection.onDocumentSymbol(({ textDocument }) => {
  const document = documents.get(textDocument.uri);
  if (!document) {
    return [];
  }

  return toDocumentSymbols(collectFunctionSymbols(document));
});

documents.listen(connection);
connection.listen();

function validateDocument(document: TextDocument): void {
  connection.sendDiagnostics({
    uri: document.uri,
    diagnostics: collectDiagnostics(document),
  });
}
