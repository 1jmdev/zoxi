import * as path from "node:path";
import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions, Trace, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const outputChannel = vscode.window.createOutputChannel("Zoxi Language Server");
  context.subscriptions.push(outputChannel);

  const restartCommand = vscode.commands.registerCommand("zoxi.restartLanguageServer", async () => {
    await stopClient();
    client = await startClient(context, outputChannel);
  });
  context.subscriptions.push(restartCommand);

  client = await startClient(context, outputChannel);
}

export async function deactivate(): Promise<void> {
  await stopClient();
}

async function startClient(
  context: vscode.ExtensionContext,
  outputChannel: vscode.OutputChannel,
): Promise<LanguageClient> {
  const serverModule = context.asAbsolutePath(path.join("dist", "server", "server.js"));
  const serverOptions: ServerOptions = {
    run: { module: serverModule, transport: TransportKind.ipc },
    debug: { module: serverModule, transport: TransportKind.ipc },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "zoxi" },
      { scheme: "untitled", language: "zoxi" },
    ],
    outputChannel,
  };

  const traceSetting = vscode.workspace.getConfiguration("zoxi").get<string>("trace.server", "off");
  const nextClient = new LanguageClient("zoxi", "Zoxi Language Server", serverOptions, clientOptions);
  if (traceSetting !== "off") {
    await nextClient.setTrace(traceSetting === "verbose" ? Trace.Verbose : Trace.Messages);
  }

  await nextClient.start();
  return nextClient;
}

async function stopClient(): Promise<void> {
  if (!client) {
    return;
  }

  const activeClient = client;
  client = undefined;
  await activeClient.stop();
}
