export type KeywordDoc = {
  readonly detail: string;
  readonly documentation: string;
};

export const KEYWORD_DOCS: Record<string, KeywordDoc> = {
  async: {
    detail: "async",
    documentation: "Marks a function or block as asynchronous. Zoxi keeps async semantics aligned with Rust.",
  },
  await: {
    detail: "await",
    documentation: "Waits for an async value to resolve.",
  },
  fn: {
    detail: "fn name(args): Type",
    documentation: "Declares a function. In generated Rust, the return annotation is emitted with `->`.",
  },
  let: {
    detail: "let",
    documentation: "Introduces a local binding.",
  },
  match: {
    detail: "match",
    documentation: "Pattern matching expression, same general shape as Rust.",
  },
  string: {
    detail: "string",
    documentation: "Zoxi alias for `String` in transpiled Rust.",
  },
  view: {
    detail: ".view()",
    documentation: "Zoxi string view helper that transpiles to `.as_str()`.",
  },
  Ok: {
    detail: "Ok(value)",
    documentation: "Success variant for `Result`. Zoxi can auto-wrap tail expressions in result-returning functions.",
  },
  Err: {
    detail: "Err(error)",
    documentation: "Error variant for `Result`.",
  },
};

export const COMPLETION_ITEMS = [
  { label: "fn", insertText: "fn ${1:name}(${2}): ${3:Type} {\n    ${0}\n}", detail: KEYWORD_DOCS.fn!.detail },
  { label: "let", insertText: "let ${1:name} = ${0};", detail: KEYWORD_DOCS.let!.detail },
  { label: "if", insertText: "if ${1:condition} {\n    ${0}\n}", detail: "if condition" },
  { label: "match", insertText: "match ${1:value} {\n    ${2:pattern} => ${0},\n}", detail: KEYWORD_DOCS.match!.detail },
  { label: "async", insertText: "async", detail: KEYWORD_DOCS.async!.detail },
  { label: "await", insertText: "await", detail: KEYWORD_DOCS.await!.detail },
  { label: "string", insertText: "string", detail: KEYWORD_DOCS.string!.detail },
  { label: ".view()", insertText: ".view()", detail: KEYWORD_DOCS.view!.detail },
];
